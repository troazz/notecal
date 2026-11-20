<script lang="ts">
  import { t, type Language } from "./i18n.svelte";
  import type { VimStatus } from "./editor/vimstate";

  let {
    noteName,
    language,
    locale,
    ratesDate,
    usesCurrency,
    usesTime,
    ontogglelanguage,
    ontogglelocale,
    vimStatus,
    pinned,
    pinSupported,
    ontogglepin,
  }: {
    noteName: string | null;
    /** The language shown (the setting resolved against the system). */
    language: Language;
    /** Number format: separators in input and output. */
    locale: "en" | "id";
    ratesDate: string | null;
    usesCurrency: boolean;
    usesTime: boolean;
    ontogglelanguage: () => void;
    ontogglelocale: () => void;
    /** E10: null while vim mode is off. */
    vimStatus: VimStatus | null;
    /** A4: always on top. */
    pinned: boolean;
    /** False where the desktop can't keep a window on top (most Wayland compositors). */
    pinSupported: boolean;
    ontogglepin: () => void;
  } = $props();
</script>

<footer class="status-bar">
  <span class="left">
    {#if vimStatus}
      <span class="vim" title={t("status.vimTitle")}
        >{vimStatus.mode}{#if vimStatus.pending}<span class="pending">{vimStatus.pending}</span>{/if}</span
      >
    {/if}
    <span class="note" title={noteName ?? ""}>{noteName ?? ""}</span>
  </span>
  <span class="items">
    {#if usesCurrency}
      <span title={t("app.ratesTitle")}>
        {t("app.rates", { date: ratesDate ?? t("app.ratesNone") })}
      </span>
    {/if}
    {#if usesTime}
      <span title={t("status.liveTitle")}>{t("status.live")}</span>
    {/if}
    <button
      type="button"
      class="pin"
      class:on={pinned}
      title={pinSupported ? t(pinned ? "status.pinOn" : "status.pinOff") : t("status.pinUnsupported")}
      aria-label={pinSupported ? t(pinned ? "status.pinOn" : "status.pinOff") : t("status.pinUnsupported")}
      aria-pressed={pinned}
      disabled={!pinSupported}
      onclick={ontogglepin}
      ><svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"
        ><path
          d="M10.5 1.5 14.5 5.5 12.5 6.5 10 9 10.5 12 9 13.5 6 10.5 2.5 14 2 13.5 5.5 10 2.5 7 4 5.5 7 6 9.5 3.5Z"
          fill={pinned ? "currentColor" : "none"}
          stroke="currentColor"
          stroke-width="1.2"
          stroke-linejoin="round"
        /></svg
      ></button
    >
    <button
      type="button"
      title={t("status.localeTitle")}
      aria-label={t("status.localeTitle")}
      onclick={ontogglelocale}>{locale === "id" ? "1.234,5" : "1,234.5"}</button
    >
    <button
      type="button"
      title={t("status.languageTitle")}
      aria-label={t("status.languageTitle")}
      onclick={ontogglelanguage}>{language.toUpperCase()}</button
    >
  </span>
</footer>

<style>
  .status-bar {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    height: 22px;
    padding: 0 0.6rem;
    border-top: 1px solid var(--border-color);
    background: var(--panel-color);
    color: var(--muted-color);
    font: 11px system-ui, sans-serif;
    user-select: none;
  }
  .left {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .vim {
    flex: none;
    color: var(--accent-color);
    font-weight: 600;
    letter-spacing: 0.03em;
  }
  .pending {
    margin-left: 0.4rem;
    font-weight: 400;
    color: var(--text-color);
  }
  .note {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .items {
    flex: none;
    display: flex;
    align-items: center;
    gap: 0.9rem;
  }
  button {
    padding: 0 0.15rem;
    border: none;
    border-radius: 3px;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .pin {
    display: flex;
    align-items: center;
    height: 16px;
  }
  .pin.on {
    color: var(--accent-color);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  button:hover:not(:disabled),
  button:focus-visible {
    color: var(--text-color);
    background: color-mix(in srgb, var(--text-color) 10%, transparent);
    outline: none;
  }
</style>
