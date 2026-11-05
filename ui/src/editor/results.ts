import { RangeSetBuilder, StateEffect, StateField, type Extension, type Text } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, type DecorationSet } from "@codemirror/view";
import type { LineResult } from "./types";

/** Replaces the displayed results; one entry per document line. */
export const setResults = StateEffect.define<LineResult[]>();

function build(doc: Text, lines: readonly LineResult[]): DecorationSet {
  const builder = new RangeSetBuilder<Decoration>();
  const count = Math.min(doc.lines, lines.length);
  for (let i = 0; i < count; i++) {
    const result = lines[i];
    if (!result?.display) continue;
    // A line attribute drawn by CSS, not a widget: an inline widget at the
    // line end is what the editor measures the end-of-line cursor against,
    // which put the cursor next to the result instead of after the text.
    const line = doc.line(i + 1);
    builder.add(
      line.from,
      line.from,
      Decoration.line({
        attributes: { "data-result": result.display, "data-copy": result.copy ?? result.display },
      }),
    );
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

/** Marks the line whose result was just copied (PRD E1: click to copy). */
const flashCopied = StateEffect.define<number | null>();

const copiedField = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(decorations, tr) {
    for (const effect of tr.effects) {
      if (effect.is(flashCopied)) {
        return effect.value === null
          ? Decoration.none
          : Decoration.set(Decoration.line({ class: "cm-result-copied" }).range(effect.value));
      }
    }
    return tr.docChanged ? decorations.map(tr.changes) : decorations;
  },
  provide: (field) => EditorView.decorations.from(field),
});

const COPIED_FLASH_MS = 900;
let measureCanvas: HTMLCanvasElement | undefined;

function textWidth(text: string, font: string): number {
  measureCanvas ??= document.createElement("canvas");
  const context = measureCanvas.getContext("2d");
  if (!context) return 0;
  context.font = font;
  return context.measureText(text).width;
}

/**
 * The result is a CSS pseudo-element, which has no DOM node to receive
 * events, so a click is matched against where the result is drawn: right
 * edge of the visible area minus the inset, back by the text's width.
 */
function resultUnderPointer(event: MouseEvent, view: EditorView): { line: HTMLElement; copy: string } | null {
  const line = (event.target as Element | null)?.closest<HTMLElement>(".cm-line[data-result]");
  const text = line?.dataset["result"];
  const copy = line?.dataset["copy"];
  if (!line || !text || copy === undefined) return null;
  const rootFontSize = parseFloat(getComputedStyle(document.documentElement).fontSize);
  const scroller = view.scrollDOM;
  const right = scroller.getBoundingClientRect().left + scroller.clientWidth - 0.75 * rootFontSize;
  const left = right - textWidth(text, getComputedStyle(line).font);
  return event.clientX >= left && event.clientX <= right ? { line, copy } : null;
}

const copyOnClick = EditorView.domEventHandlers({
  mousedown(event, view) {
    if (event.button !== 0) return false;
    const hit = resultUnderPointer(event, view);
    if (!hit) return false;
    event.preventDefault();
    const from = view.posAtDOM(hit.line);
    void navigator.clipboard.writeText(hit.copy).then(() => {
      view.dispatch({ effects: flashCopied.of(from) });
      setTimeout(() => view.dispatch({ effects: flashCopied.of(null) }), COPIED_FLASH_MS);
    });
    return true;
  },
  mousemove(event, view) {
    view.contentDOM.classList.toggle("cm-over-result", resultUnderPointer(event, view) !== null);
    return false;
  },
});

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
  ".cm-line.cm-result-copied::after": { content: '"Copied"' },
  ".cm-content.cm-over-result": { cursor: "pointer" },
});

/** Draws each line's result right-aligned on that line; a click copies it (PRD E1). */
export const results: Extension = [resultsField, copiedField, pinToViewport, copyOnClick, resultsTheme];
