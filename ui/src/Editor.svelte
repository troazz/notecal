<script lang="ts" module>
  /** What the app can do with the editor once it exists. */
  export interface EditorController {
    getText: () => string;
    /** Replaces the whole text; the cursor moves to `cursor`, or stays put (clamped). */
    setText: (text: string, cursor?: number) => void;
    getCursor: () => number;
    setCursor: (cursor: number) => void;
    /** Evaluates again, e.g. after locale, precision or rates changed. */
    reevaluate: () => void;
    focus: () => void;
  }
</script>

<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
  import { EditorState } from "@codemirror/state";
  import {
    EditorView,
    crosshairCursor,
    drawSelection,
    highlightActiveLine,
    highlightSpecialChars,
    keymap,
    rectangularSelection,
  } from "@codemirror/view";
  import { noteCompletion } from "./editor/complete";
  import { notecalHighlighting } from "./editor/language";
  import { results, setResults } from "./editor/results";
  import { Scheduler } from "./editor/scheduler";
  import type { KeystrokeTimer } from "./editor/perf";
  import type { EvalResponse } from "./editor/types";

  let {
    timer,
    onready,
    onedit,
    onresults,
  }: {
    timer: KeystrokeTimer;
    onready: (controller: EditorController) => void;
    onedit: () => void;
    onresults: (response: EvalResponse) => void;
  } = $props();

  let host: HTMLDivElement;

  onMount(() => {
    let view: EditorView | undefined;
    /** Currency codes for autocomplete; loaded once, empty until then. */
    let currencies: readonly string[] = [];
    invoke<string[]>("currency_codes")
      .then((codes) => (currencies = codes))
      .catch((error: unknown) => console.error("currency_codes failed", error));

    const scheduler = new Scheduler({
      invoke: (request) => invoke<EvalResponse | null>("evaluate", { request }),
      apply: (response) => {
        view?.dispatch({ effects: setResults.of(response.lines) });
        onresults(response);
        // rAF runs before the paint; the timeout after it runs once the
        // frame with the new results is on screen.
        requestAnimationFrame(() =>
          setTimeout(() => timer.painted(response.version, performance.now()), 0),
        );
      },
      getText: () => view?.state.doc.toString() ?? "",
      requestFrame: (callback) => requestAnimationFrame(callback),
      onError: (error) => console.error("evaluate failed", error),
    });

    const extensions = [
        // PRD E4: undo/redo, multiple cursors (Ctrl+click, Alt+drag), search.
        highlightSpecialChars(),
        history(),
        drawSelection(),
        EditorState.allowMultipleSelections.of(true),
        rectangularSelection(),
        crosshairCursor(),
        highlightSelectionMatches(),
        search({ top: true }),
        keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
        notecalHighlighting,
        noteCompletion(() => currencies),
        // Marks the cursor line so its (half-typed) error marker stays hidden.
        highlightActiveLine(),
        results,
        EditorView.updateListener.of((update) => {
          if (!update.docChanged) return;
          scheduler.changed();
          onedit();
          const typed = update.transactions.some(
            (tr) => tr.isUserEvent("input") || tr.isUserEvent("delete"),
          );
          if (typed) timer.keystroke(scheduler.currentVersion, performance.now());
        }),
    ];
    view = new EditorView({ parent: host, extensions });

    const editor = view;
    onready({
      getText: () => editor.state.doc.toString(),
      setText: (text, cursor) => {
        const anchor = Math.min(cursor ?? editor.state.selection.main.head, text.length);
        // A fresh state, not a change: undo history must never carry text
        // from another note (or a replaced version) into this one.
        editor.setState(EditorState.create({ doc: text, selection: { anchor }, extensions }));
        editor.dispatch({ scrollIntoView: true });
        scheduler.changed();
      },
      getCursor: () => editor.state.selection.main.head,
      setCursor: (cursor) =>
        editor.dispatch({
          selection: { anchor: Math.min(cursor, editor.state.doc.length) },
          scrollIntoView: true,
        }),
      focus: () => editor.focus(),
      reevaluate: () => scheduler.changed(),
    });
    view.focus();
    requestAnimationFrame(() => void invoke("mark_ready"));

    return () => view?.destroy();
  });
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
  }
  .editor :global(.cm-editor) {
    height: 100%;
    font-family: var(--editor-font, monospace);
    font-size: var(--editor-font-size, 15px);
  }
  .editor :global(.cm-editor.cm-focused) {
    outline: none;
  }
  .editor :global(.cm-scroller) {
    padding: 0.5rem 0;
  }
  .editor :global(.cm-cursor) {
    border-left-color: var(--text-color);
  }
  .editor :global(.cm-selectionBackground),
  .editor :global(.cm-focused .cm-selectionBackground) {
    background: var(--selection-color) !important;
  }
  .editor :global(.cm-activeLine) {
    background: transparent;
  }
  .editor :global(.cm-selectionMatch) {
    background: color-mix(in srgb, var(--selection-color) 60%, transparent);
  }
  .editor :global(.cm-panels) {
    background: var(--panel-color);
    color: var(--text-color);
    border-color: var(--border-color);
    font: 13px system-ui, sans-serif;
  }
  .editor :global(.cm-panels input),
  .editor :global(.cm-panels button) {
    font: inherit;
    color: var(--text-color);
    background: var(--bg-color);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background-image: none;
  }
  .editor :global(.cm-tooltip-autocomplete) {
    background: var(--panel-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    font: 13px var(--editor-font, monospace);
  }
  .editor :global(.cm-tooltip-autocomplete li[aria-selected]) {
    background: var(--selection-color);
    color: var(--text-color);
  }
  .editor :global(.cm-searchMatch) {
    background: color-mix(in srgb, var(--accent-color) 30%, transparent);
  }
</style>
