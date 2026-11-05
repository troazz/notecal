import { RangeSetBuilder, StateEffect, StateField, type Extension, type Text } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet } from "@codemirror/view";
import type { LineResult } from "./types";

/** Replaces the displayed results; one entry per document line. */
export const setResults = StateEffect.define<LineResult[]>();

function build(doc: Text, lines: readonly LineResult[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const count = Math.min(doc.lines, lines.length);
  for (let i = 0; i < count; i++) {
    const display = lines[i]?.display;
    if (!display) continue;
    // A line attribute drawn by CSS, not a widget: an inline widget at the
    // line end is what the editor measures the end-of-line cursor against,
    // which put the cursor next to the result instead of after the text.
    const line = doc.line(i + 1);
    builder.add(line.from, line.from, Decoration.line({ attributes: { "data-result": display } }));
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

/**
 * Without soft wrap every line is as wide as the longest one, so a result
 * aligned to its line's right edge can sit outside the visible area. This
 * keeps the results at the right edge of what is on screen by publishing how
 * far the lines extend past it.
 */
const pinToViewport = ViewPlugin.fromClass(
  class {
    constructor(readonly view: EditorView) {
      this.measure();
    }

    update(): void {
      this.measure();
    }

    measure(): void {
      this.view.requestMeasure({
        read: (view) => {
          const scroller = view.scrollDOM;
          const content = view.contentDOM;
          const lineRight = content.offsetLeft + content.offsetWidth;
          const visibleRight = scroller.scrollLeft + scroller.clientWidth;
          return Math.max(0, lineRight - visibleRight);
        },
        write: (overflow, view) => {
          view.contentDOM.style.setProperty("--result-overflow", `${overflow}px`);
        },
      });
    }
  },
  {
    eventHandlers: {
      scroll(_event, view) {
        this.measure();
        return false;
      },
    },
  },
);

const resultsTheme = EditorView.baseTheme({
  ".cm-line": { position: "relative" },
  // Absolutely positioned so a result never shifts the text it follows. The
  // background hides any text the result overlaps on a long line.
  ".cm-line[data-result]::after": {
    content: "attr(data-result)",
    position: "absolute",
    right: "calc(var(--result-overflow, 0px) + 0.75rem)",
    paddingLeft: "0.75rem",
    color: "var(--result-color)",
    background: "var(--bg-color)",
    whiteSpace: "pre",
    pointerEvents: "none",
  },
});

/** Draws each line's result right-aligned on that line (PRD E1). */
export const results: Extension = [resultsField, pinToViewport, resultsTheme];
