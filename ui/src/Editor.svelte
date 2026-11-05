<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { EditorView, minimalSetup } from "codemirror";
  import { results, setResults } from "./editor/results";
  import { Scheduler } from "./editor/scheduler";
  import type { KeystrokeTimer } from "./editor/perf";
  import type { EvalResponse } from "./editor/types";

  let { initialText, timer }: { initialText: string; timer: KeystrokeTimer } = $props();

  let host: HTMLDivElement;

  onMount(() => {
    let view: EditorView | undefined;

    const scheduler = new Scheduler({
      invoke: (request) => invoke<EvalResponse | null>("evaluate", { request }),
      apply: (response) => {
        view?.dispatch({ effects: setResults.of(response.lines) });
        // rAF runs before the paint; the timeout after it runs once the
        // frame with the new results is on screen.
        requestAnimationFrame(() =>
          setTimeout(() => timer.painted(response.version, performance.now()), 0),
        );
      },
      getText: () => view?.state.doc.toString() ?? "",
      getLocale: () => "en",
      requestFrame: (callback) => requestAnimationFrame(callback),
      onError: (error) => console.error("evaluate failed", error),
    });

    view = new EditorView({
      parent: host,
      doc: initialText,
      extensions: [
        minimalSetup,
        results,
        EditorView.updateListener.of((update) => {
          if (!update.docChanged) return;
          scheduler.changed();
          const typed = update.transactions.some(
            (tr) => tr.isUserEvent("input") || tr.isUserEvent("delete"),
          );
          if (typed) timer.keystroke(scheduler.currentVersion, performance.now());
        }),
      ],
    });

    scheduler.changed();
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
    font-family: ui-monospace, "JetBrains Mono", Menlo, monospace;
    font-size: 15px;
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
</style>
