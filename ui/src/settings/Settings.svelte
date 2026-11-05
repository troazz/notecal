<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { settingsApi, type AppSettings } from "./api";

  let {
    onsave,
    onclose,
    onerror,
  }: {
    /** Saves settings (the app flushes pending note edits first). */
    onsave: (settings: AppSettings) => Promise<void>;
    onclose: () => void;
    onerror: (message: string) => void;
  } = $props();

  let settings = $state<AppSettings | null>(null);
  let problems = $state<string[]>([]);
  let themes = $state<string[]>([]);
  let shortcutStatus = $state("");
  let dialog: HTMLDivElement;

  async function load() {
    try {
      const [view, userThemes, status] = await Promise.all([
        settingsApi.get(),
        invoke<string[]>("list_themes"),
        invoke<string>("shortcut_status"),
      ]);
      settings = view.settings;
      problems = view.problems;
      themes = userThemes;
      shortcutStatus = status;
    } catch (error) {
      onerror(String(error));
    }
  }

  onMount(() => {
    void load();
    dialog.focus();
    const stop = settingsApi.onChanged(() => void load());
    return () => void stop.then((unlisten) => unlisten());
  });

  /** Saves one change; the backend validates and applies it. */
  async function update(patch: Partial<AppSettings>) {
    if (!settings) return;
    const next = { ...settings, ...patch };
    try {
      await onsave(next);
      settings = next;
    } catch (error) {
      onerror(String(error));
      await load();
    }
  }

  function numberFrom(event: Event): number {
    return Number((event.currentTarget as HTMLInputElement).value);
  }

  function textFrom(event: Event): string {
    return (event.currentTarget as HTMLInputElement | HTMLSelectElement).value;
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div
    class="settings"
    role="dialog"
    aria-modal="true"
    aria-labelledby="settings-title"
    tabindex="-1"
    bind:this={dialog}
    onkeydown={onKeydown}
  >
    <h2 id="settings-title">Settings</h2>
    {#if problems.length}
      <ul class="problems" role="alert">
        {#each problems as problem (problem)}<li>{problem}</li>{/each}
      </ul>
    {/if}
    {#if settings}
      <div class="grid">
        <label for="s-theme">Theme</label>
        <select id="s-theme" value={settings.theme} onchange={(e) => void update({ theme: textFrom(e) })}>
          <option value="auto">Auto (Omarchy, else system)</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
          {#each themes as name (name)}<option value={name}>{name}</option>{/each}
        </select>

        <label for="s-font">Font</label>
        <input
          id="s-font"
          value={settings.font_family}
          placeholder="auto"
          onchange={(e) => void update({ font_family: textFrom(e).trim() || "auto" })}
        />

        <label for="s-size">Font size</label>
        <input
          id="s-size"
          type="number"
          min="8"
          max="40"
          value={settings.font_size}
          onchange={(e) => void update({ font_size: numberFrom(e) })}
        />

        <label for="s-precision">Decimal places</label>
        <input
          id="s-precision"
          type="number"
          min="0"
          max="20"
          value={settings.precision}
          onchange={(e) => void update({ precision: numberFrom(e) })}
        />

        <label for="s-locale">Number format</label>
        <select
          id="s-locale"
          value={settings.locale}
          onchange={(e) => void update({ locale: textFrom(e) as AppSettings["locale"] })}
        >
          <option value="en">1,234.5 (en)</option>
          <option value="id">1.234,5 (id)</option>
        </select>

        <label for="s-notes">Notes folder</label>
        <input
          id="s-notes"
          value={settings.notes_dir}
          placeholder="Default (~/.local/share/notecal/notes)"
          onchange={(e) => void update({ notes_dir: textFrom(e).trim() })}
        />

        <label for="s-shortcut">Shortcut</label>
        <div>
          <input
            id="s-shortcut"
            value={settings.shortcut}
            onchange={(e) => void update({ shortcut: textFrom(e).trim() || "Alt+Space" })}
          />
          <p class="hint">{shortcutStatus}</p>
        </div>

        <span></span>
        <div class="checks">
          <label
            ><input
              type="checkbox"
              checked={settings.always_on_top}
              onchange={(e) => void update({ always_on_top: e.currentTarget.checked })}
            /> Always on top (Ctrl+Shift+T; X11/macOS)</label
          >
          <label
            ><input
              type="checkbox"
              checked={settings.background}
              onchange={(e) => void update({ background: e.currentTarget.checked })}
            /> Keep running in the background when closed</label
          >
          <label
            ><input
              type="checkbox"
              checked={settings.currency_refresh}
              onchange={(e) => void update({ currency_refresh: e.currentTarget.checked })}
            /> Update exchange rates daily</label
          >
        </div>
      </div>
    {/if}
    <p class="hint">
      Saved to ~/.config/notecal/settings.toml as you change them. Exchange rates by Exchange Rate
      API (www.exchangerate-api.com, open.er-api.com).
    </p>
    <p class="hint">Esc to close</p>
  </div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 6vh;
    background: color-mix(in srgb, var(--bg-color) 40%, transparent);
    z-index: 15;
  }
  .settings {
    width: min(34rem, calc(100% - 2rem));
    max-height: 86vh;
    overflow-y: auto;
    padding: 0.75rem 1rem;
    background: var(--panel-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.25);
    font: 13px system-ui, sans-serif;
    outline: none;
  }
  h2 {
    margin: 0 0 0.75rem;
    font-size: 15px;
  }
  .grid {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.5rem 0.75rem;
    align-items: center;
  }
  input,
  select {
    box-sizing: border-box;
    width: 100%;
    padding: 0.3rem 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-color);
    color: var(--text-color);
    font: inherit;
  }
  input[type="checkbox"] {
    width: auto;
    margin-right: 0.4rem;
  }
  .checks {
    display: grid;
    gap: 0.3rem;
  }
  .hint {
    margin: 0.4rem 0 0;
    color: var(--muted-color);
    font-size: 12px;
  }
  .problems {
    margin: 0 0 0.75rem;
    padding-left: 1.1rem;
    color: var(--error-color);
  }
</style>
