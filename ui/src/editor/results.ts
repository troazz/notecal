import { RangeSetBuilder, StateEffect, StateField, type Extension, type Text } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, keymap, type DecorationSet } from "@codemirror/view";
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
function resultUnderPointer(
  event: MouseEvent,
  view: EditorView,
): { line: HTMLElement; display: string; copy: string } | null {
  const line = (event.target as Element | null)?.closest<HTMLElement>(".cm-line[data-result]");
  const display = line?.dataset["result"];
  const copy = line?.dataset["copy"];
  if (!line || !display || copy === undefined) return null;
  const rootFontSize = parseFloat(getComputedStyle(document.documentElement).fontSize);
  const scroller = view.scrollDOM;
  const right = scroller.getBoundingClientRect().left + scroller.clientWidth - 0.75 * rootFontSize;
  const left = right - textWidth(display, getComputedStyle(line).font);
  return event.clientX >= left && event.clientX <= right ? { line, display, copy } : null;
}

/** Copies `text` and briefly shows "Copied" in place of the line's result. */
function copyResult(view: EditorView, lineStart: number, text: string): void {
  void navigator.clipboard.writeText(text).then(() => {
    view.dispatch({ effects: flashCopied.of(lineStart) });
    setTimeout(() => view.dispatch({ effects: flashCopied.of(null) }), COPIED_FLASH_MS);
  });
}

/** The result shown on the line holding the main cursor, if any. */
function currentResult(view: EditorView): { lineStart: number; display: string; copy: string } | null {
  const line = view.state.doc.lineAt(view.state.selection.main.head);
  let found: { display: string; copy: string } | null = null;
  view.state.field(resultsField).between(line.from, line.from, (_from, _to, decoration) => {
    const attributes = decoration.spec["attributes"] as Record<string, string> | undefined;
    const display = attributes?.["data-result"];
    if (display) found = { display, copy: attributes?.["data-copy"] ?? display };
  });
  return found ? { lineStart: line.from, ...(found as { display: string; copy: string }) } : null;
}

/** A visually hidden live region for screen-reader announcements (PRD E9). */
class Announcer {
  private readonly region: HTMLElement;

  constructor(view: EditorView) {
    this.region = document.createElement("div");
    this.region.className = "cm-announcer";
    this.region.setAttribute("aria-live", "polite");
    this.region.setAttribute("role", "status");
    view.dom.appendChild(this.region);
  }

  say(text: string): void {
    // Clearing first makes a repeated message announce again.
    this.region.textContent = "";
    requestAnimationFrame(() => (this.region.textContent = text));
  }

  destroy(): void {
    this.region.remove();
  }
}

const announcer = ViewPlugin.fromClass(Announcer);

const resultKeys = keymap.of([
  {
    // PRD E3: copy the current line's result as shown.
    key: "Mod-Shift-c",
    run: (view) => {
      const result = currentResult(view);
      if (result) copyResult(view, result.lineStart, result.display);
      return result !== null;
    },
  },
  {
    // PRD E8: copy the bare number (no unit, no group separators).
    key: "Mod-Shift-b",
    run: (view) => {
      const result = currentResult(view);
      if (result) copyResult(view, result.lineStart, result.copy);
      return result !== null;
    },
  },
  {
    // PRD E9: read the current line's result aloud.
    key: "Mod-Shift-a",
    run: (view) => {
      const result = currentResult(view);
      view.plugin(announcer)?.say(result ? result.display : "No result on this line");
      return true;
    },
  },
]);

const copyOnClick = EditorView.domEventHandlers({
  mousedown(event, view) {
    if (event.button !== 0) return false;
    const hit = resultUnderPointer(event, view);
    if (!hit) return false;
    event.preventDefault();
    // PRD E3: a click copies the result as shown; E8: Alt-click the bare number.
    copyResult(view, view.posAtDOM(hit.line), event.altKey ? hit.copy : hit.display);
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
  ".cm-announcer": {
    position: "absolute",
    width: "1px",
    height: "1px",
    overflow: "hidden",
    clipPath: "inset(50%)",
    whiteSpace: "nowrap",
  },
});

/**
 * Draws each line's result right-aligned on that line (PRD E1). Click copies
 * it as shown and Alt-click the bare number (E3, E8); Mod-Shift-C / Mod-Shift-B
 * do the same for the cursor's line, and Mod-Shift-A announces it (E9).
 */
export const results: Extension = [
  resultsField,
  copiedField,
  pinToViewport,
  copyOnClick,
  announcer,
  resultKeys,
  resultsTheme,
];
