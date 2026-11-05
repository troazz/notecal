import { RangeSetBuilder, type Extension, type Text } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet, type ViewUpdate } from "@codemirror/view";

/** Words with a meaning in the calculation language (PRD language rules). */
const KEYWORDS = new Set(["in", "to", "as", "of", "prev", "sum", "total", "avg", "mod", "per"]);

export type TokenClass =
  | "heading"
  | "comment"
  | "number"
  | "keyword"
  | "definition"
  | "variable"
  | "function"
  | "unit"
  | "operator"
  | "paren";

export interface Token {
  from: number;
  to: number;
  cls: TokenClass;
}

const ASSIGNMENT = /^\s*([A-Za-z_][A-Za-z_0-9]*)\s*=(?!=)/;
const NUMBER = /^(0x[0-9a-f]+|0b[01]+|0o[0-7]+|\d[\d,]*(\.\d+)?(e[+-]?\d+)?)/i;
const WORD = /^[A-Za-z_][A-Za-z_0-9]*/;

/**
 * Splits one line into highlighted tokens. `isVariable` says whether a name
 * was assigned on an earlier line of the note.
 */
export function tokenizeLine(text: string, isVariable: (name: string) => boolean): Token[] {
  const tokens: Token[] = [];
  let pos = 0;
  let first = true;
  while (pos < text.length) {
    const rest = text.slice(pos);
    const space = /^\s+/.exec(rest);
    if (space) {
      pos += space[0].length;
      continue;
    }
    const start = pos;
    const push = (length: number, cls: TokenClass) => {
      tokens.push({ from: start, to: start + length, cls });
      pos += length;
    };
    // Headers (`# Trip`) and comments (`# note`, `// note`) run to the end.
    if (rest.startsWith("#") || rest.startsWith("//")) {
      push(rest.length, first ? "heading" : "comment");
      break;
    }
    const number = NUMBER.exec(rest);
    const word = WORD.exec(rest);
    if (number) {
      push(number[0].length, "number");
    } else if (word) {
      const name = word[0];
      const after = rest.slice(name.length);
      if (KEYWORDS.has(name.toLowerCase())) push(name.length, "keyword");
      else if (first && /^\s*=(?!=)/.test(after)) push(name.length, "definition");
      else if (isVariable(name)) push(name.length, "variable");
      else if (/^\s*\(/.test(after)) push(name.length, "function");
      else push(name.length, "unit");
    } else if (/^[-+*/^%=×÷]/.test(rest)) {
      push(1, "operator");
    } else if (/^[()]/.test(rest)) {
      push(1, "paren");
    } else {
      pos += 1;
    }
    first = false;
  }
  return tokens;
}

/** Line number of the first assignment of each name in the note. */
export function firstAssignments(doc: Text): Map<string, number> {
  const assigned = new Map<string, number>();
  let number = 0;
  for (const iter = doc.iterLines(); !iter.next().done; ) {
    number += 1;
    const match = ASSIGNMENT.exec(iter.value);
    if (match && !assigned.has(match[1]!)) assigned.set(match[1]!, number);
  }
  return assigned;
}

const marks = new Map<TokenClass, Decoration>();
function mark(cls: TokenClass): Decoration {
  let decoration = marks.get(cls);
  if (!decoration) {
    decoration = Decoration.mark({ class: `cm-tok-${cls}` });
    marks.set(cls, decoration);
  }
  return decoration;
}

function build(view: EditorView, assigned: Map<string, number>): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const doc = view.state.doc;
  for (const { from, to } of view.visibleRanges) {
    for (let pos = from; pos <= to; ) {
      const line = doc.lineAt(pos);
      const isVariable = (name: string) => (assigned.get(name) ?? Infinity) < line.number;
      for (const token of tokenizeLine(line.text, isVariable)) {
        builder.add(line.from + token.from, line.from + token.to, mark(token.cls));
      }
      pos = line.to + 1;
    }
  }
  return builder.finish();
}

function sameAssignments(a: Map<string, number>, b: Map<string, number>): boolean {
  if (a.size !== b.size) return false;
  for (const [name, line] of a) if (b.get(name) !== line) return false;
  return true;
}

/** Maps existing marks through the edit and re-tokenises only the touched lines. */
function rebuildChanged(update: ViewUpdate, old: DecorationSet, assigned: Map<string, number>): DecorationSet {
  const doc = update.state.doc;
  const touched: { from: number; to: number }[] = [];
  update.changes.iterChangedRanges((_fromA, _toA, fromB, toB) => {
    touched.push({ from: doc.lineAt(fromB).from, to: doc.lineAt(toB).to });
  });
  let decorations = old.map(update.changes);
  for (const { from, to } of touched) {
    decorations = decorations.update({ filterFrom: from, filterTo: to, filter: () => false });
    const added = [];
    for (let pos = from; pos <= to; ) {
      const line = doc.lineAt(pos);
      const isVariable = (name: string) => (assigned.get(name) ?? Infinity) < line.number;
      for (const token of tokenizeLine(line.text, isVariable)) {
        added.push(mark(token.cls).range(line.from + token.from, line.from + token.to));
      }
      pos = line.to + 1;
    }
    decorations = decorations.update({ add: added, sort: true });
  }
  return decorations;
}

/**
 * Highlights only the visible lines, and on an edit only the edited lines. A full parser
 * (CodeMirror's StreamLanguage) re-tokenised every line after an edit and
 * pushed typing p95 from ~47 ms to ~320 ms on WebKitGTK.
 */
const highlighter = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    private assigned: Map<string, number>;

    constructor(view: EditorView) {
      this.assigned = firstAssignments(view.state.doc);
      this.decorations = build(view, this.assigned);
    }

    update(update: ViewUpdate) {
      let rebuild = update.viewportChanged;
      if (update.docChanged) {
        const assigned = firstAssignments(update.state.doc);
        // A new or moved assignment can recolour any later line; otherwise
        // only the edited lines change.
        rebuild ||= !sameAssignments(assigned, this.assigned);
        this.assigned = assigned;
      }
      if (rebuild) {
        this.decorations = build(update.view, this.assigned);
      } else if (update.docChanged) {
        this.decorations = rebuildChanged(update, this.decorations, this.assigned);
      }
    }
  },
  { decorations: (plugin) => plugin.decorations },
);

/** Colours come from CSS variables, so themes (built-in, Omarchy) restyle them. */
const theme = EditorView.baseTheme({
  ".cm-tok-heading": { color: "var(--hl-heading)", fontWeight: "bold" },
  ".cm-tok-comment": { color: "var(--hl-comment)" },
  ".cm-tok-number": { color: "var(--hl-number)" },
  ".cm-tok-keyword": { color: "var(--hl-keyword)" },
  ".cm-tok-definition, .cm-tok-variable": { color: "var(--hl-variable)" },
  ".cm-tok-function": { color: "var(--hl-function)" },
  ".cm-tok-unit": { color: "var(--hl-unit)" },
  ".cm-tok-operator, .cm-tok-paren": { color: "var(--hl-operator)" },
});

/** Syntax highlighting for notes (PRD E2). */
export const notecalHighlighting: Extension = [highlighter, theme];
