<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Editor, { type EditorController } from "./Editor.svelte";
  import Switcher from "./notes/Switcher.svelte";
  import { notesApi } from "./notes/api";
  import { NoteSession } from "./notes/session";
  import { KeystrokeTimer } from "./editor/perf";

  const timer = new KeystrokeTimer();
  let showStats = $state(false);
  let stats = $state(timer.stats());
  let showSwitcher = $state(false);
  let noteName = $state<string | null>(null);
  let notice = $state<string | null>(null);

  let editor: EditorController | undefined;
  let session: NoteSession | undefined;

  $effect(() => {
    if (!showStats) return;
    const id = setInterval(() => (stats = timer.stats()), 500);
    return () => clearInterval(id);
  });

  $effect(() => {
    const title = noteName ? `${noteName} — notecal` : "notecal";
    getCurrentWindow()
      .setTitle(title)
      .catch((error: unknown) => console.error("setTitle failed", error));
  });

  function report(message: string) {
    notice = message;
  }

  function onEditorReady(controller: EditorController) {
    editor = controller;
    session = new NoteSession({
      open: notesApi.open,
      save: notesApi.save,
      getText: controller.getText,
      setText: (text) => controller.setText(text),
      notify: report,
      setTimer: (cb, ms) => setTimeout(cb, ms),
      clearTimer: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
      now: () => new Date(),
    });
    void start();
  }

  /** Reopens the last note at its cursor (N4), or the most recent note. */
  async function start() {
    try {
      const [state, notes] = await Promise.all([notesApi.loadState(), notesApi.list()]);
      const last = notes.find((n) => n.name === state.lastNote)?.name;
      const name = last ?? notes[0]?.name;
      if (name === undefined) {
        showSwitcher = true;
        return;
      }
      await openNote(name, last ? state.cursor : 0);
    } catch (error) {
      report(String(error));
    }
  }

  async function openNote(name: string, cursor = 0) {
    if (!session || !editor) return;
    await saveState();
    await session.open(name);
    editor.setCursor(cursor);
    noteName = name;
    showSwitcher = false;
    editor.focus();
    await saveState();
  }

  async function saveState() {
    if (!session?.name || !editor) return;
    try {
      await notesApi.saveState({ lastNote: session.name, cursor: editor.getCursor() });
    } catch (error) {
      report(String(error));
    }
  }

  /** Saves edits and the cursor; used on blur and close. */
  async function persist() {
    await session?.flush();
    await saveState();
  }

  async function onDeleted(name: string) {
    if (session?.name !== name) return;
    session.closed();
    noteName = null;
    editor?.setText("");
    const next = (await notesApi.list())[0]?.name;
    if (next) await openNote(next);
  }

  onMount(() => {
    const appWindow = getCurrentWindow();
    const stops = [
      notesApi.onDiskChange((event) => void session?.diskChanged(event)),
      appWindow.onFocusChanged(({ payload: focused }) => {
        if (!focused) void persist();
      }),
      appWindow.onCloseRequested(async () => {
        await persist();
      }),
    ];
    return () => stops.forEach((stop) => void stop.then((unlisten) => unlisten()));
  });

  function onKeydown(event: KeyboardEvent) {
    const mod = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();
    if (mod && event.shiftKey && key === "m") {
      event.preventDefault();
      stats = timer.stats();
      showStats = !showStats;
    } else if (mod && !event.shiftKey && key === "p") {
      event.preventDefault();
      void session?.flush();
      showSwitcher = !showSwitcher;
      if (!showSwitcher) editor?.focus();
    }
  }

  function statsLine(): string {
    const ms = (n: number) => n.toFixed(1);
    return `keystrokes=${stats.count} p50=${ms(stats.p50)}ms p95=${ms(stats.p95)}ms max=${ms(stats.max)}ms`;
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main>
  <Editor {timer} onready={onEditorReady} onedit={() => session?.edited()} />
  {#if showSwitcher}
    <Switcher
      current={noteName}
      onopen={(name) => void openNote(name)}
      onrenamed={(from, to) => {
        session?.renamed(from, to);
        if (noteName === from) noteName = to;
        void saveState();
      }}
      ondeleted={(name) => void onDeleted(name)}
      onclose={() => {
        showSwitcher = false;
        editor?.focus();
      }}
      onerror={report}
    />
  {/if}
  {#if notice}
    <div class="notice" role="status">
      <span>{notice}</span>
      <button type="button" onclick={() => (notice = null)} aria-label="Dismiss">Dismiss</button>
    </div>
  {/if}
  {#if showStats}
    <aside class="stats" aria-live="polite">
      <span>{statsLine()}</span>
      <button type="button" onclick={() => navigator.clipboard.writeText(statsLine())}>Copy</button>
    </aside>
  {/if}
</main>

<style>
  :global(:root) {
    color-scheme: light dark;
    --bg-color: #fbfbfa;
    --text-color: #1d1f23;
    --result-color: #5b6270;
    --panel-color: #ecedef;
    --border-color: #d3d5da;
    --selection-color: #d7e3fb;
    --muted-color: #5b6270;
    --accent-color: #2457c5;
    --error-color: #b42318;
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg-color: #1b1d22;
      --text-color: #e6e7ea;
      --result-color: #a0a7b4;
      --panel-color: #2a2d34;
      --border-color: #3b3f48;
      --selection-color: #2f3b55;
      --muted-color: #a0a7b4;
      --accent-color: #8fb3ff;
      --error-color: #ff8a80;
    }
  }
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: var(--bg-color);
    color: var(--text-color);
  }
  :global(#app) {
    height: 100%;
  }
  main {
    height: 100%;
    position: relative;
  }
  .notice {
    position: absolute;
    right: 0.75rem;
    bottom: 0.75rem;
    max-width: min(28rem, calc(100% - 1.5rem));
    display: flex;
    gap: 0.75rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border-color);
    border-radius: 6px;
    background: var(--panel-color);
    font: 13px system-ui, sans-serif;
    z-index: 20;
  }
  .notice button {
    font: inherit;
    padding: 0.15rem 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-color);
    color: var(--text-color);
    cursor: pointer;
  }
  .stats {
    position: absolute;
    bottom: 0.5rem;
    left: 0.5rem;
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    background: var(--panel-color);
    font: 12px ui-monospace, monospace;
  }
</style>
