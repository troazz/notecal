import { RangeSetBuilder, StateEffect, StateField, type Extension, type Text } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import type { LineResult } from "./types";

/** Replaces the displayed results; one entry per document line. */
export const setResults = StateEffect.define<LineResult[]>();

class ResultWidget extends WidgetType {
  constructor(readonly text: string) {
    super();
  }

  override eq(other: ResultWidget): boolean {
    return other.text === this.text;
  }

  toDOM(): HTMLElement {
    const span = document.createElement("span");
    span.className = "cm-result";
    span.textContent = this.text;
    return span;
  }
}

function build(doc: Text, lines: readonly LineResult[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const count = Math.min(doc.lines, lines.length);
  for (let i = 0; i < count; i++) {
    const display = lines[i]?.display;
    if (!display) continue;
    const line = doc.line(i + 1);
    builder.add(line.to, line.to, Decoration.widget({ widget: new ResultWidget(display), side: 1 }));
  }
  return builder.finish();
}

const resultsField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(decorations, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setResults)) return build(tr.state.doc, effect.value);
    }
    // Keep showing the previous results, moved with the text, until the
    // response for the new version arrives.
    return tr.docChanged ? decorations.map(tr.changes) : decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});

const resultsTheme = EditorView.baseTheme({
  ".cm-line": { position: "relative" },
  // Absolutely positioned so a result never shifts the text it follows.
  ".cm-result": {
    position: "absolute",
    right: "0.75rem",
    color: "var(--result-color)",
    whiteSpace: "pre",
  },
});

/** Draws each line's result right-aligned on that line (PRD E1). */
export const results: Extension = [resultsField, resultsTheme];
