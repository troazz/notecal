<script lang="ts">
  // First-run onboarding (PRD A8). Every choice is saved at once through the
  // same path as the Settings screen, so it applies live behind this overlay.
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { settingsApi, type AppSettings } from "../settings/api";
  import { t, translateMessage, type Key } from "../i18n.svelte";
  import { resolvePalette, type Palette, type Theme } from "../theme";
  import type { CheatSection } from "../notes/cheatsearch";
  import { STEPS, FONT_SIZES, PRECISIONS, bindLines, type DesktopInfo, type StepId } from "./steps";

  let {
    onsave,
    onfinish,
    onerror,
  }: {
    /** Saves settings (the app flushes pending note edits first). */
    onsave: (settings: AppSettings) => Promise<void>;
    /** Finished or skipped; `welcome`: create the Welcome note. */
    onfinish: (welcome: boolean) => void;
    onerror: (message: string) => void;
  } = $props();

  let settings = $state<AppSettings | null>(null);
  let desktop = $state<DesktopInfo>({ wayland: false, hyprland: null, pin: "native" });
  let shortcutStatus = $state("");
  let themes = $state<string[]>([]);
  /** Preview palettes by theme setting value. */
  let palettes = $state<Record<string, Palette>>({});
  let sheet = $state<CheatSection[]>([]);
  let step = $state<StepId>("welcome");
  let visited = $state(new Set<StepId>(["welcome"]));
  let welcome = $state(true);
  let copied = $state(false);
  let dialog: HTMLDivElement;

  const index = $derived(STEPS.indexOf(step));
  const binds = $derived(bindLines(desktop));
  /** The welcome step's live demo: results from the engine, in the current format. */
  const demo = $derived(
    // No currency: a fresh install has no rates yet.
    ["percent-add", "assign", "length", "date-add"]
      .map((id) => sheet.flatMap((s) => s.examples).find((e) => e.id === id))
      .filter((e) => e !== undefined),
  );

  async function load() {
    try {
      const [view, info, status, userThemes, sections] = await Promise.all([
        settingsApi.get(),
        invoke<DesktopInfo>("desktop_info"),
        invoke<string>("shortcut_status"),
        invoke<string[]>("list_themes"),
        invoke<CheatSection[]>("cheatsheet"),
      ]);
      settings = view.settings;
      desktop = info;
      shortcutStatus = status;
      themes = userThemes;
      sheet = sections;
    } catch (error) {
      onerror(String(error));
    }
  }

  async function loadPalettes(names: string[]) {
    const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    for (const name of names) {
      try {
        const theme = await invoke<Theme | null>("preview_theme", { name });
        palettes[name] = resolvePalette(theme, dark).palette;
      } catch (error) {
        console.error("preview_theme failed", name, error);
      }
    }
  }

  onMount(() => {
    void load().then(() => {
      focusStep();
      void loadPalettes(["auto", "light", "dark", ...themes]);
    });
    // Number format and precision change the demo's results.
    const stop = settingsApi.onChanged((view) => {
      settings = view.settings;
      invoke<CheatSection[]>("cheatsheet")
        .then((sections) => (sheet = sections))
        .catch((error: unknown) => console.error("cheatsheet failed", error));
    });
    return () => void stop.then((unlisten) => unlisten());
  });

  /** Changes are saved one at a time, each on top of the previous one. */
  let pending: Promise<void> = Promise.resolve();

  function update(patch: Partial<AppSettings>, field?: HTMLInputElement): Promise<void> {
    pending = pending.then(async () => {
      if (!settings) return;
      const previous = settings;
      settings = { ...settings, ...patch };
      try {
        await onsave(settings);
      } catch (error) {
        onerror(translateMessage(String(error)));
        settings = previous;
        if (field) field.value = String(previous[field.dataset["key"] as keyof AppSettings] ?? "");
      }
    });
    return pending;
  }

  function go(next: StepId) {
    step = next;
    visited = new Set([...visited, next]);
    focusStep();
  }

  function move(delta: number) {
    const next = STEPS[index + delta];
    if (next) go(next);
  }

  /** Puts the focus on the step's first control (or Next), so keyboard users start there. */
  function focusStep() {
    requestAnimationFrame(() =>
      dialog
        ?.querySelector<HTMLElement>(".body input:checked, .body input, .body button, footer .primary")
        ?.focus(),
    );
  }

  async function finish(withWelcome: boolean) {
    if (document.activeElement instanceof HTMLElement && dialog.contains(document.activeElement)) {
      document.activeElement.blur(); // commits a half-typed field
    }
    await pending;
    onfinish(withWelcome);
  }

  async function copyBinds() {
    if (!binds) return;
    try {
      await navigator.clipboard.writeText(binds);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch (error) {
      onerror(String(error));
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      // Esc is Skip; the app's own Esc (hide) must not see it.
      event.preventDefault();
      event.stopPropagation();
      void finish(true);
    } else if (event.altKey && event.key === "ArrowRight") {
      event.preventDefault();
      move(1);
    } else if (event.altKey && event.key === "ArrowLeft") {
      event.preventDefault();
      move(-1);
    } else if (event.key === "Tab") {
      trapTab(event);
    }
  }

  /** Keeps Tab inside the dialog (it is modal). */
  function trapTab(event: KeyboardEvent) {
    const focusable = [...dialog.querySelectorAll<HTMLElement>("button, input, textarea")].filter(
      (el) => !el.hasAttribute("disabled") && !(el instanceof HTMLInputElement && el.type === "radio" && !el.checked),
    );
    const first = focusable[0];
    const last = focusable.at(-1);
    if (!first || !last) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  const themeChoices = $derived([
    { value: "auto", label: t("settings.themeAuto") },
    { value: "light", label: t("settings.themeLight") },
    { value: "dark", label: t("settings.themeDark") },
    ...themes.map((name) => ({ value: name, label: name })),
  ]);
</script>

<div class="backdrop">
  <div
    class="onboarding"
    role="dialog"
    aria-modal="true"
    aria-labelledby="onboarding-title"
    tabindex="-1"
    bind:this={dialog}
    onkeydown={onKeydown}
  >
    <header>
      <h2 id="onboarding-title">{t("onboarding.title")}</h2>
      <button type="button" class="link" onclick={() => void finish(true)}>{t("onboarding.skip")}</button>
    </header>

    <div class="layout">
      <nav aria-label={t("onboarding.steps")}>
        <ol>
          {#each STEPS as id, i (id)}
            <li>
              <button
                type="button"
                class:current={id === step}
                aria-current={id === step ? "step" : undefined}
                onclick={() => go(id)}
              >
                <span class="mark" aria-hidden="true">{visited.has(id) && i < index ? "✓" : i + 1}</span>
                {t(`onboarding.step.${id}` as Key)}
              </button>
            </li>
          {/each}
        </ol>
      </nav>

      <section class="body" aria-live="polite">
        {#if !settings}
          <p class="hint">…</p>
        {:else if step === "welcome"}
          <h3>{t("onboarding.title")}</h3>
          <p>{t("onboarding.lead")}</p>
          <div class="demo" aria-hidden="true">
            {#each demo as example (example.id)}
              {@const lines = example.text.split("\n")}
              {#each lines as line, i (i)}
                <div class="demo-line">
                  <code>{line}</code><span>{i === lines.length - 1 ? (example.result ?? "") : ""}</span>
                </div>
              {/each}
            {/each}
          </div>
          <p class="hint">{t("onboarding.later")}</p>
        {:else if step === "look"}
          <h3>{t("onboarding.lookTitle")}</h3>
          <fieldset>
            <legend>{t("settings.theme")}</legend>
            <div class="cards" role="radiogroup">
              {#each themeChoices as choice (choice.value)}
                {@const p = palettes[choice.value]}
                <label class="card" class:chosen={settings.theme === choice.value}>
                  <input
                    type="radio"
                    name="theme"
                    value={choice.value}
                    checked={settings.theme === choice.value}
                    onchange={() => void update({ theme: choice.value })}
                  />
                  <span
                    class="preview"
                    style:background={p?.background}
                    style:color={p?.foreground}
                    style:border-color={p ? `color-mix(in srgb, ${p.foreground} 18%, ${p.background})` : undefined}
                  >
                    <span class="pl"><span style:color={p?.yellow}>20</span>% of <span style:color={p?.yellow}>150</span><span style:color={p?.muted}>30</span></span>
                    <span class="pl"><span style:color={p?.yellow}>5</span> <span style:color={p?.cyan}>km</span><span style:color={p?.muted}>3.1 mi</span></span>
                  </span>
                  <span class="card-label">{choice.label}</span>
                </label>
              {/each}
            </div>
          </fieldset>
          <fieldset>
            <legend>{t("settings.fontSize")}</legend>
            <div class="chips" role="radiogroup">
              {#each FONT_SIZES as size (size)}
                <label class="chip" class:chosen={settings.font_size === size}>
                  <input
                    type="radio"
                    name="font-size"
                    checked={settings.font_size === size}
                    onchange={() => void update({ font_size: size })}
                  /><span style:font-size="{size}px">{size}</span>
                </label>
              {/each}
            </div>
          </fieldset>
        {:else if step === "language"}
          <h3>{t("onboarding.languageTitle")}</h3>
          <fieldset>
            <legend>{t("settings.language")}</legend>
            <div class="chips" role="radiogroup">
              {#each [{ v: "auto", l: t("settings.languageAuto") }, { v: "en", l: "English" }, { v: "id", l: "Bahasa Indonesia" }] as choice (choice.v)}
                <label class="chip" class:chosen={settings.language === choice.v}>
                  <input
                    type="radio"
                    name="language"
                    checked={settings.language === choice.v}
                    onchange={() => void update({ language: choice.v as AppSettings["language"] })}
                  />{choice.l}
                </label>
              {/each}
            </div>
          </fieldset>
          <fieldset>
            <legend>{t("settings.numberFormat")}</legend>
            <div class="cards" role="radiogroup">
              {#each [{ v: "en", sample: "1,234,567.89" }, { v: "id", sample: "1.234.567,89" }] as choice (choice.v)}
                <label class="card number" class:chosen={settings.locale === choice.v}>
                  <input
                    type="radio"
                    name="locale"
                    checked={settings.locale === choice.v}
                    onchange={() => void update({ locale: choice.v as AppSettings["locale"] })}
                  />
                  <span class="sample">{choice.sample}</span>
                </label>
              {/each}
            </div>
          </fieldset>
          <fieldset>
            <legend>{t("settings.precision")}</legend>
            <div class="chips" role="radiogroup">
              {#each PRECISIONS as places (places)}
                <label class="chip" class:chosen={settings.precision === places}>
                  <input
                    type="radio"
                    name="precision"
                    checked={settings.precision === places}
                    onchange={() => void update({ precision: places })}
                  /><span class="mono">{(1 / 3).toFixed(places)}</span>
                </label>
              {/each}
            </div>
          </fieldset>
        {:else if step === "keys"}
          <h3>{t("onboarding.keysTitle")}</h3>
          <fieldset>
            <legend>{t("onboarding.showHide")}</legend>
            {#if binds}
              <p class="hint">{desktop.hyprland ? t("onboarding.bindHyprland") : t("onboarding.bindOther")}</p>
              <pre class="binds">{binds}</pre>
              <button type="button" onclick={() => void copyBinds()}
                >{copied ? t("onboarding.copied") : t("onboarding.copy")}</button
              >
            {:else}
              <input
                aria-label={t("settings.shortcut")}
                data-key="shortcut"
                value={settings.shortcut}
                onchange={(e) =>
                  void update({ shortcut: e.currentTarget.value.trim() || "Alt+Space" }, e.currentTarget)}
              />
              <p class="hint">{t("onboarding.shortcutHint")} {translateMessage(shortcutStatus)}</p>
            {/if}
          </fieldset>
          <fieldset>
            <legend>{t("onboarding.editing")}</legend>
            <div class="cards" role="radiogroup">
              {#each [{ vim: false, title: t("onboarding.standard"), hint: t("onboarding.standardHint") }, { vim: true, title: t("onboarding.vim"), hint: t("onboarding.vimHint") }] as choice (choice.vim)}
                <label class="card text" class:chosen={settings.vim_mode === choice.vim}>
                  <input
                    type="radio"
                    name="editing"
                    checked={settings.vim_mode === choice.vim}
                    onchange={() => void update({ vim_mode: choice.vim })}
                  />
                  <strong>{choice.title}</strong>
                  <span class="hint">{choice.hint}</span>
                </label>
              {/each}
            </div>
          </fieldset>
          {#if desktop.pin !== "none"}
            <label class="check">
              <input
                type="checkbox"
                checked={settings.always_on_top}
                onchange={(e) => void update({ always_on_top: e.currentTarget.checked })}
              />
              {t("onboarding.pin")}
            </label>
          {/if}
        {:else if step === "data"}
          <h3>{t("onboarding.dataTitle")}</h3>
          <label class="check">
            <input
              type="checkbox"
              checked={settings.currency_refresh}
              onchange={(e) => void update({ currency_refresh: e.currentTarget.checked })}
            />
            {t("onboarding.rates")}
          </label>
          <p class="hint">{t("onboarding.ratesHint")}</p>
          <fieldset>
            <legend>{t("settings.notesDir")}</legend>
            <input
              data-key="notes_dir"
              value={settings.notes_dir}
              placeholder={t("settings.notesDirDefault")}
              aria-label={t("settings.notesDir")}
              spellcheck="false"
              onchange={(e) => void update({ notes_dir: e.currentTarget.value.trim() }, e.currentTarget)}
            />
            <p class="hint">{t("onboarding.notesDirHint")}</p>
          </fieldset>
        {:else}
          <h3>{t("onboarding.readyTitle")}</h3>
          <dl class="tips">
            <dt><kbd>Ctrl+P</kbd></dt>
            <dd>{t("onboarding.tipNotes")}</dd>
            <dt><kbd>Ctrl+P</kbd> <kbd>?</kbd></dt>
            <dd>{t("onboarding.tipCheat")}</dd>
            <dt><kbd>Esc</kbd></dt>
            <dd>{t("onboarding.tipHide")}</dd>
            <dt>{t("onboarding.clickResult")}</dt>
            <dd>{t("onboarding.tipCopy")}</dd>
          </dl>
          <label class="check">
            <input type="checkbox" bind:checked={welcome} />
            {t("onboarding.welcomeNote")}
          </label>
        {/if}
      </section>
    </div>

    <footer>
      {#if index > 0}
        <button type="button" onclick={() => move(-1)}>{t("onboarding.back")}</button>
      {/if}
      <span class="spacer"></span>
      {#if step === "welcome"}
        <button type="button" class="primary" onclick={() => move(1)}>{t("onboarding.start")}</button>
      {:else if step === "ready"}
        <button type="button" class="primary" onclick={() => void finish(welcome)}>{t("onboarding.finish")}</button>
      {:else}
        <button type="button" class="primary" onclick={() => move(1)}>{t("onboarding.next")}</button>
      {/if}
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 0.75rem;
    background: color-mix(in srgb, var(--bg-color) 55%, transparent);
    z-index: 20;
  }
  .onboarding {
    display: flex;
    flex-direction: column;
    width: min(44rem, 100%);
    max-height: 100%;
    background: var(--panel-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.3);
    font: 13px system-ui, sans-serif;
    outline: none;
    animation: fade 120ms ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .onboarding {
      animation: none;
    }
  }
  header,
  footer {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0.9rem;
  }
  header {
    justify-content: space-between;
    border-bottom: 1px solid var(--border-color);
  }
  footer {
    border-top: 1px solid var(--border-color);
  }
  .spacer {
    flex: 1;
  }
  h2 {
    margin: 0;
    font-size: 14px;
  }
  h3 {
    margin: 0 0 0.6rem;
    font-size: 17px;
  }
  .layout {
    display: flex;
    min-height: 0;
    flex: 1;
  }
  nav {
    flex: none;
    width: 9.5rem;
    padding: 0.6rem 0.4rem;
    border-right: 1px solid var(--border-color);
  }
  nav ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.35rem 0.5rem;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--muted-color);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  nav button.current {
    color: var(--text-color);
    background: color-mix(in srgb, var(--accent-color) 16%, transparent);
  }
  .mark {
    display: inline-grid;
    place-items: center;
    width: 1.2rem;
    height: 1.2rem;
    border: 1px solid currentColor;
    border-radius: 50%;
    font-size: 10px;
  }
  .body {
    flex: 1;
    min-width: 0;
    padding: 0.9rem 1rem;
    overflow-y: auto;
  }
  p {
    margin: 0 0 0.6rem;
    line-height: 1.45;
  }
  fieldset {
    margin: 0 0 0.9rem;
    padding: 0;
    border: none;
  }
  legend {
    margin-bottom: 0.4rem;
    padding: 0;
    color: var(--muted-color);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .cards,
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }
  .card,
  .chip {
    position: relative;
    border: 1px solid var(--border-color);
    border-radius: 7px;
    background: var(--bg-color);
    cursor: pointer;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    width: 8.5rem;
    padding: 0.45rem;
  }
  .card.text {
    width: 12.5rem;
  }
  .card.number {
    width: auto;
    padding: 0.6rem 0.9rem;
  }
  .chip {
    display: flex;
    align-items: center;
    padding: 0.35rem 0.7rem;
  }
  .card.chosen,
  .chip.chosen {
    border-color: var(--accent-color);
    box-shadow: 0 0 0 1px var(--accent-color);
  }
  .card:has(input:focus-visible),
  .chip:has(input:focus-visible) {
    outline: 2px solid var(--accent-color);
    outline-offset: 2px;
  }
  /* The radio stays in the page for keyboard and screen readers; the card shows the choice. */
  .card input,
  .chip input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.4rem 0.5rem;
    border: 1px solid transparent;
    border-radius: 4px;
    font: 11px var(--editor-font, monospace);
  }
  .pl {
    display: flex;
    gap: 0.25rem;
  }
  .pl > :last-child {
    margin-left: auto;
  }
  .card-label {
    font-size: 12px;
  }
  .sample,
  .mono,
  code {
    font-family: var(--editor-font, monospace);
  }
  .sample {
    font-size: 15px;
  }
  .demo {
    margin: 0.4rem 0 0.8rem;
    padding: 0.5rem 0.7rem;
    border-radius: 6px;
    background: var(--bg-color);
    font: 13px var(--editor-font, monospace);
  }
  .demo-line {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    line-height: 1.6;
  }
  .demo-line span {
    color: var(--result-color);
  }
  .binds {
    margin: 0 0 0.4rem;
    padding: 0.5rem 0.6rem;
    border-radius: 5px;
    background: var(--bg-color);
    font: 11px var(--editor-font, monospace);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin: 0 0 0.4rem;
  }
  .hint {
    color: var(--muted-color);
    font-size: 12px;
  }
  input:not([type]) {
    box-sizing: border-box;
    width: 100%;
    padding: 0.3rem 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-color);
    color: var(--text-color);
    font: inherit;
  }
  .tips {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.45rem 0.9rem;
    margin: 0 0 1rem;
  }
  .tips dt {
    font-family: var(--editor-font, monospace);
  }
  .tips dd {
    margin: 0;
  }
  kbd {
    padding: 0 0.3rem;
    border: 1px solid var(--border-color);
    border-radius: 3px;
    font: 12px var(--editor-font, monospace);
  }
  button {
    padding: 0.3rem 0.8rem;
    border: 1px solid var(--border-color);
    border-radius: 5px;
    background: var(--bg-color);
    color: var(--text-color);
    font: inherit;
    cursor: pointer;
  }
  button.primary {
    border-color: var(--accent-color);
    background: var(--accent-color);
    color: var(--bg-color);
  }
  button.link {
    border: none;
    background: none;
    color: var(--muted-color);
  }
  button:hover {
    filter: brightness(1.08);
  }
  button:focus-visible {
    outline: 2px solid var(--accent-color);
    outline-offset: 2px;
  }
  @media (max-width: 560px) {
    nav {
      display: none;
    }
  }
</style>
