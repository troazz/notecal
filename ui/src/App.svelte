<script lang="ts">
  import Editor from "./Editor.svelte";
  import { KeystrokeTimer } from "./editor/perf";
  import fixture from "./fixtures/note-200.txt?raw";

  const timer = new KeystrokeTimer();
  let showStats = $state(false);
  let stats = $state(timer.stats());

  $effect(() => {
    if (!showStats) return;
    const id = setInterval(() => (stats = timer.stats()), 500);
    return () => clearInterval(id);
  });

  function onKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === "m") {
      event.preventDefault();
      stats = timer.stats();
      showStats = !showStats;
    }
  }

  function statsLine(): string {
    const ms = (n: number) => n.toFixed(1);
    return `keystrokes=${stats.count} p50=${ms(stats.p50)}ms p95=${ms(stats.p95)}ms max=${ms(stats.max)}ms`;
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main>
  <Editor initialText={fixture} {timer} />
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
  }
  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg-color: #1b1d22;
      --text-color: #e6e7ea;
      --result-color: #a0a7b4;
      --panel-color: #2a2d34;
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
