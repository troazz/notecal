import { HighlightStyle, StreamLanguage, syntaxHighlighting, type StringStream } from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import { tags } from "@lezer/highlight";

/** Words with a meaning in the calculation language (PRD language rules). */
const KEYWORDS = new Set(["in", "to", "as", "of", "prev", "sum", "total", "avg", "mod", "per"]);

interface LineState {
  /** Variables assigned so far in the note, so later uses read as variables. */
  vars: string[];
  /** True until the first token of the current line. */
  atLineStart: boolean;
}

function token(stream: StringStream, state: LineState): string | null {
  if (stream.sol()) state.atLineStart = true;
  if (stream.eatSpace()) return null;
  const first = state.atLineStart;
  state.atLineStart = false;

  // Headers (`# Trip`) and comments (`# note`, `// note`) run to the end.
  if (stream.match("//") || stream.peek() === "#") {
    stream.skipToEnd();
    return first ? "heading" : "comment";
  }
  if (stream.match(/^0x[0-9a-f]+|^0b[01]+|^0o[0-7]+/i)) return "number";
  if (stream.match(/^\d[\d,]*(\.\d+)?(e[+-]?\d+)?/i)) return "number";

  const word = stream.match(/^[A-Za-z_][A-Za-z_0-9]*/) as RegExpMatchArray | null;
  if (word) {
    const name = word[0];
    if (KEYWORDS.has(name.toLowerCase())) return "keyword";
    if (first && stream.match(/^\s*=(?!=)/, false)) {
      if (!state.vars.includes(name)) state.vars.push(name);
      return "definition";
    }
    if (state.vars.includes(name)) return "variableName";
    if (stream.match(/^\s*\(/, false)) return "fn";
    return "unit";
  }
  if (stream.match(/^[-+*/^%=×÷]/)) return "operator";
  if (stream.match(/^[()]/)) return "paren";
  stream.next();
  return null;
}

export const notecalLanguage = StreamLanguage.define<LineState>({
  name: "notecal",
  startState: () => ({ vars: [], atLineStart: true }),
  copyState: (state) => ({ vars: state.vars.slice(), atLineStart: state.atLineStart }),
  token,
  tokenTable: {
    heading: tags.heading,
    comment: tags.comment,
    number: tags.number,
    keyword: tags.keyword,
    definition: tags.definition(tags.variableName),
    variableName: tags.variableName,
    fn: tags.function(tags.variableName),
    unit: tags.unit,
    operator: tags.operator,
    paren: tags.paren,
  },
});

/** Colours come from CSS variables, so themes (built-in, Omarchy, user) restyle them. */
const highlightStyle = HighlightStyle.define([
  { tag: tags.heading, color: "var(--hl-heading)", fontWeight: "bold" },
  { tag: tags.comment, color: "var(--hl-comment)" },
  { tag: tags.number, color: "var(--hl-number)" },
  { tag: tags.keyword, color: "var(--hl-keyword)" },
  { tag: [tags.variableName, tags.definition(tags.variableName)], color: "var(--hl-variable)" },
  { tag: tags.function(tags.variableName), color: "var(--hl-function)" },
  { tag: tags.unit, color: "var(--hl-unit)" },
  { tag: [tags.operator, tags.paren], color: "var(--hl-operator)" },
]);

/** Syntax highlighting for notes (PRD E2). */
export const notecalHighlighting: Extension = [notecalLanguage, syntaxHighlighting(highlightStyle)];
