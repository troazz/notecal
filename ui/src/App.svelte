<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Editor, { type EditorController } from "./Editor.svelte";
  import Switcher from "./notes/Switcher.svelte";
  import Settings from "./settings/Settings.svelte";
  import StatusBar from "./StatusBar.svelte";
  import { notesApi } from "./notes/api";
  import { settingsApi, type AppSettings, type SettingsView } from "./settings/api";
  import { NoteSession } from "./notes/session";
  import { KeystrokeTimer } from "./editor/perf";
  import type { VimStatus } from "./editor/vimstate";
  import type { CheatSection } from "./notes/cheatsearch";
  import { WELCOME_NOTE, welcomeNote } from "./onboarding/steps";
  import {
    currentLanguage,
    setLanguage,
    t,
    translateMessage,
    type Key,
    type Language,
  } from "./i18n.svelte";

  const timer = new KeystrokeTimer();
  let showStats = $state(false);
  let stats = $state(timer.stats());
  let showSwitcher = $state(false);
  let showSettings = $state(false);
  /** A8: the first-run onboarding (also reopened from Settings and the cheatsheet). */
  let showOnboarding = $state(false);
  let noteName = $state<string | null>(null);
  let notice = $state<string | null>(null);
  /** PRD C9 status line: shown only when the note uses a currency. */
  let usesCurrency = $state(false);
  let ratesDate = $state<string | null>(null);
  /** The saved settings, for the status bar's switches. */
  let appSettings = $state<AppSettings | null>(null);
  /** Mirrors the i18n language so the status bar re-renders on change. */
  let shownLanguage = $state<Language>("en");
  /** C11: the note shows `now`/`today`, so results refresh on the minute. */
  let usesTime = $state(false);
  /** E10: the vim mode shown in the status bar; null while vim mode is off. */
  let vimStatus = $state<VimStatus | null>(null);
  /** A4: false where the desktop can't keep a window on top (Wayland other than Hyprland). */
  let pinSupported = $state(true);
  /** Whether vim keys are on in the editor (applied, not just saved). */
  let vimApplied = false;

  let editor: EditorController | undefined;
  let session: NoteSession | undefined;

  // While a result depends on the clock, re-evaluate just after each minute
  // starts, skipping ticks while the window is hidden (it refreshes on focus).
  $effect(() => {
    if (!usesTime) return;
    let timeout: ReturnType<typeof setTimeout>;
    const schedule = () => {
      timeout = setTimeout(() => void tick(), 60_000 - (Date.now() % 60_000) + 50);
    };
    const tick = async () => {
      const visible = await getCurrentWindow()
        .isVisible()
        .catch(() => true);
      if (visible) editor?.reevaluate();
      schedule();
    };
    schedule();
    return () => clearTimeout(timeout);
  });

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
    notice = translateMessage(message);
  }

  /** M9: the interface language, from the setting or the system. */
  function applyLanguage() {
    invoke<Language>("ui_language")
      .then((language) => {
        setLanguage(language);
        shownLanguage = language;
        document.documentElement.lang = language;
        // The "Copied" flash is drawn by CSS (editor/results.ts).
        document.documentElement.style.setProperty(
          "--copied-label",
          JSON.stringify(t("results.copied")),
        );
        // Error hover text is translated when results are drawn.
        editor?.reevaluate();
      })
      .catch((error: unknown) => console.error("ui_language failed", error));
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
    applyVim();
    void start();
  }

  /** Turns vim keys on or off to match the settings (E10). */
  function applyVim() {
    const on = appSettings?.vim_mode ?? false;
    if (!editor || on === vimApplied) return;
    vimApplied = on;
    if (!on) vimStatus = null;
    editor
      .setVim(
        on
          ? {
              onstatus: (status) => (vimStatus = status),
              onwrite: () => void persist(),
              onquit: () => void persist().then(() => getCurrentWindow().hide()),
            }
          : null,
      )
      .catch((error: unknown) => report(String(error)));
  }

  /** Reopens the last note at its cursor (N4), or the most recent note. */
  /** One start at a time (a folder change can arrive while one runs). */
  let starting: Promise<void> | null = null;
  function start(): Promise<void> {
    starting ??= startOnce().finally(() => (starting = null));
    return starting;
  }

  async function startOnce() {
    try {
      const [state, notes] = await Promise.all([notesApi.loadState(), notesApi.list()]);
      if (state.onboarding === "pending") showOnboarding = true;
      const last = notes.find((n) => n.name === state.lastNote)?.name;
      const name = last ?? notes[0]?.name ?? (await createScratch());
      await openNote(name, last ? state.cursor : 0);
    } catch (error) {
      report(String(error));
    }
  }

  async function openNote(name: string, cursor = 0) {
    if (!session || !editor) return;
    try {
      await saveState();
      await session.open(name);
      editor.setReadOnly(false);
      editor.setCursor(cursor);
      noteName = name;
      showSwitcher = false;
      editor.focus();
      await saveState();
    } catch (error) {
      report(error instanceof Error ? error.message : String(error));
    }
  }

  /** The onboarding was finished or skipped; it isn't shown at startup again. */
  async function finishOnboarding(withWelcome: boolean) {
    showOnboarding = false;
    try {
      await notesApi.finishOnboarding();
      if (withWelcome) await openWelcomeNote();
    } catch (error) {
      report(String(error));
    }
    editor?.focus();
  }

  /** Opens the Welcome note of examples, creating it the first time. */
  async function openWelcomeNote() {
    const exists = (await notesApi.list()).some((n) => n.name === WELCOME_NOTE);
    if (exists) return openNote(WELCOME_NOTE);
    const sheet = await invoke<CheatSection[]>("cheatsheet");
    const text = welcomeNote(sheet, {
      title: t("onboarding.title"),
      intro: t("onboarding.noteIntro"),
      section: (id) => t(`cheatsheet.${id}` as Key),
      explain: (id) => t(`cheat.${id}` as Key),
    });
    await notesApi.create(WELCOME_NOTE);
    await openNote(WELCOME_NOTE);
    if (session?.name !== WELCOME_NOTE) return;
    // An ordinary edit of the open note: the session saves it.
    editor?.setText(text, 0);
    session.edited();
  }

  /** There is always a note open, so typing is never lost: recreate Scratch when none exist. */
  async function createScratch(): Promise<string> {
    await notesApi.create("Scratch");
    return "Scratch";
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
    editor?.setReadOnly(true);
    try {
      const next = (await notesApi.list())[0]?.name ?? (await createScratch());
      await openNote(next);
    } catch (error) {
      report(String(error));
    }
  }

  /** Applies settings that live in the UI, and reports file problems. */
  function applySettings(view: SettingsView) {
    appSettings = view.settings;
    applyLanguage();
    if (view.problems.length) report(`settings.toml: ${view.problems.map(translateMessage).join(" ")}`);
    applyFont(view.settings);
    applyVim();
    // Locale and precision are applied by the backend; results need a refresh.
    editor?.reevaluate();
  }

  /** E7: `auto` is the system monospace font, which follows Omarchy's font. */
  function applyFont(settings: AppSettings) {
    const family =
      settings.font_family === "auto"
        ? "monospace"
        : `"${settings.font_family.replaceAll('"', "")}", monospace`;
    const root = document.documentElement.style;
    root.setProperty("--editor-font", family);
    root.setProperty("--editor-font-size", `${settings.font_size}px`);
  }

  /** Settings screen saves: pending note edits go first (a notes-folder change switches notes). */
  /** Status bar: switch between English and Indonesian (saved like Settings). */
  function toggleLanguage() {
    if (!appSettings) return;
    const language = currentLanguage() === "id" ? "en" : "id";
    saveSettings({ ...appSettings, language }).catch((error: unknown) => report(String(error)));
  }

  /** Status bar: switch the number format between 1,234.5 and 1.234,5. */
  function toggleLocale() {
    if (!appSettings) return;
    const locale = appSettings.locale === "id" ? "en" : "id";
    saveSettings({ ...appSettings, locale }).catch((error: unknown) => report(String(error)));
  }

  async function saveSettings(settings: AppSettings) {
    await session?.flush();
    await settingsApi.save(settings);
  }

  /** The notes folder changed: reopen from the new folder. */
  /**
   * The notes folder changed. Pending edits are saved first: the open note's
   * name does not exist in the new folder, so the save creates it there
   * rather than losing the text.
   */
  async function onNotesDirChanged() {
    await session?.flush();
    session?.closed();
    noteName = null;
    // Until a note opens, typing would go nowhere.
    editor?.setReadOnly(true);
    await start();
  }

  onMount(() => {
    const appWindow = getCurrentWindow();
    applyLanguage();
    invoke<"native" | "hyprland" | "none">("pin_support")
      .then((support) => (pinSupported = support !== "none"))
      .catch((error: unknown) => console.error("pin_support failed", error));
    invoke<{ date: string | null }>("rates_status")
      .then((status) => (ratesDate = status.date))
      .catch((error: unknown) => console.error("rates_status failed", error));
    settingsApi
      .get()
      .then((view) => {
        appSettings = view.settings;
        if (view.problems.length) {
          report(`settings.toml: ${view.problems.map(translateMessage).join(" ")}`);
        }
        applyFont(view.settings);
        applyVim();
      })
      .catch((error: unknown) => report(String(error)));
    const stops = [
      settingsApi.onChanged(applySettings),
      listen("notes-dir-changed", () => void onNotesDirChanged()),
      listen<{ date: string | null }>("rates-updated", (event) => {
        ratesDate = event.payload.date;
        editor?.reevaluate();
      }),
      notesApi.onDiskChange((event) => void session?.diskChanged(event)),
      appWindow.onFocusChanged(({ payload: focused }) => {
        if (!focused) void persist();
        else if (usesTime) editor?.reevaluate();
      }),
      appWindow.onCloseRequested(async (event) => {
        await persist();
        if (session?.hasUnsavedEdits) {
          // The save failed (the notice says why); closing would lose the edits.
          event.preventDefault();
          return;
        }
        // A3: with "background" on, closing hides; Quit is in the tray.
        // Hiding needs a way back: the tray icon (or a compositor binding).
        const [background, tray] = await Promise.all([
          settingsApi
            .get()
            .then((view) => view.settings.background)
            .catch(() => false),
          invoke<boolean>("tray_available").catch(() => false),
        ]);
        if (background && tray) {
          event.preventDefault();
          await appWindow.hide();
        }
      }),
      listen("quit-requested", async () => {
        await persist();
        if (session?.hasUnsavedEdits) {
          // Stay open so the edits are not lost; the notice says why.
          await invoke("cancel_quit").catch(() => {});
          return;
        }
        await invoke("quit").catch((error: unknown) => report(String(error)));
      }),
    ];
    return () =>
      stops.forEach((stop) =>
        stop.then((unlisten) => unlisten()).catch((error: unknown) => console.error(error)),
      );
  });

  /** Set before vim sees Esc: true when that Esc is vim's (leave insert mode, cancel `d`). */
  let escapeForVim = false;

  /**
   * Runs before the editor's own keys. With vim on, app shortcuts must win over
   * vim's (Ctrl+P is `k` in normal mode), and the Esc decision must be made
   * before vim clears a pending operator.
   */
  function onKeydownCapture(event: KeyboardEvent) {
    const target = event.target instanceof Element ? event.target : null;
    if (event.key === "Escape") {
      // Esc in a panel (search, vim's `:` prompt) is the panel's; see inEditorPanel.
      escapeForVim = target?.closest(".cm-content") ? (editor?.vimBusy() ?? false) : false;
      return;
    }
    // The whole editor, panels included: vim's `:` and `/` prompts swallow keys too.
    if (!appSettings?.vim_mode || !target?.closest(".cm-editor")) return;
    const action = shortcutAction(event);
    if (!action) return;
    event.preventDefault();
    event.stopPropagation();
    if (target.closest(".cm-panels") && target instanceof HTMLElement) {
      // Vim closes its prompt on a timer after blur and then focuses the editor;
      // run the shortcut after that, so the switcher or settings keep the focus.
      target.blur();
      setTimeout(action);
    } else {
      action();
    }
  }

  /** The app's Ctrl shortcuts; null for any other key. */
  function shortcutAction(event: KeyboardEvent): (() => void) | null {
    // The onboarding is modal: no switcher or settings on top of it.
    if (showOnboarding) return null;
    const mod = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();
    if (!mod) return null;
    if (event.shiftKey && key === "m") {
      return () => {
        stats = timer.stats();
        showStats = !showStats;
      };
    }
    // A4: always on top, saved as a setting.
    if (event.shiftKey && key === "t") return () => void toggleAlwaysOnTop();
    if (event.key === ",") {
      return () => {
        showSwitcher = false;
        showSettings = !showSettings;
        if (!showSettings) editor?.focus();
      };
    }
    if (!event.shiftKey && key === "p") {
      return () => {
        void session?.flush();
        showSwitcher = !showSwitcher;
        if (!showSwitcher) editor?.focus();
      };
    }
    return null;
  }

  function onKeydown(event: KeyboardEvent) {
    const action = shortcutAction(event);
    if (action) {
      event.preventDefault();
      action();
    } else if (
      event.key === "Escape" &&
      !showSwitcher &&
      !showSettings &&
      !showOnboarding &&
      !event.defaultPrevented &&
      !inEditorPanel(event) &&
      !escapeForVim
    ) {
      // Key scenario 1: Esc hides the window; edits are saved first.
      event.preventDefault();
      void persist().then(() => getCurrentWindow().hide());
    }
  }

  async function toggleAlwaysOnTop() {
    try {
      const { settings } = await settingsApi.get();
      await settingsApi.save({ ...settings, always_on_top: !settings.always_on_top });
      report(settings.always_on_top ? t("app.alwaysOnTopOff") : t("app.alwaysOnTopOn"));
    } catch (error) {
      report(String(error));
    }
  }

  /** Esc inside the editor's search panel closes the panel, not the window. */
  function inEditorPanel(event: KeyboardEvent): boolean {
    return event.target instanceof Element && event.target.closest(".cm-panels") !== null;
  }

  function statsLine(): string {
    const ms = (n: number) => n.toFixed(1);
    return `keystrokes=${stats.count} p50=${ms(stats.p50)}ms p95=${ms(stats.p95)}ms max=${ms(stats.max)}ms`;
  }
</script>

<svelte:window onkeydown={onKeydown} onkeydowncapture={onKeydownCapture} />

<main>
  <div class="editor-area">
    <Editor
      {timer}
      onready={onEditorReady}
      onedit={() => session?.edited()}
      onresults={(response) => {
        usesCurrency = response.uses_currency;
        usesTime = response.uses_time;
      }}
    />
  </div>
  <StatusBar
    {noteName}
    language={shownLanguage}
    locale={appSettings?.locale ?? "en"}
    {ratesDate}
    {usesCurrency}
    {usesTime}
    ontogglelanguage={toggleLanguage}
    ontogglelocale={toggleLocale}
    {vimStatus}
    pinned={appSettings?.always_on_top ?? false}
    {pinSupported}
    ontogglepin={() => void toggleAlwaysOnTop()}
  />
  {#if showSwitcher}
    <Switcher
      current={noteName}
      beforechange={() => session?.flush() ?? Promise.resolve()}
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
      vimMode={appSettings?.vim_mode ?? false}
      onwelcome={() => {
        showSwitcher = false;
        showOnboarding = true;
      }}
      oninsert={(text) => {
        showSwitcher = false;
        editor?.insertBelow(text);
        editor?.focus();
      }}
    />
  {/if}
  {#if showSettings}
    <Settings
      onwelcome={() => {
        showSettings = false;
        showOnboarding = true;
      }}
      onsave={saveSettings}
      onclose={() => {
        showSettings = false;
        editor?.focus();
      }}
      onerror={report}
    />
  {/if}
  {#if showOnboarding}
    {#await import("./onboarding/Onboarding.svelte") then { default: Onboarding }}
      <Onboarding onsave={saveSettings} onfinish={(welcome) => void finishOnboarding(welcome)} onerror={report} />
    {/await}
  {/if}
  {#if notice}
    <div class="notice" role="alert">
      <span>{notice}</span>
      <button type="button" onclick={() => (notice = null)}>{t("app.dismiss")}</button>
    </div>
  {/if}
  {#if showStats}
    <aside class="stats" aria-live="polite">
      <span>{statsLine()}</span>
      <button type="button" onclick={() => navigator.clipboard.writeText(statsLine())}>{t("app.copy")}</button>
    </aside>
  {/if}
</main>

<style>
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
    display: flex;
    flex-direction: column;
  }
  .editor-area {
    flex: 1;
    min-height: 0;
  }
  .notice {
    position: absolute;
    right: 0.75rem;
    /* Above the status bar. */
    bottom: calc(22px + 0.75rem);
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
    bottom: calc(22px + 0.5rem);
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
