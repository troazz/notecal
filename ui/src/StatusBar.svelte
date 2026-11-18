<script lang="ts">
  import { t, type Language } from "./i18n.svelte";

  let {
    noteName,
    language,
    locale,
    ratesDate,
    usesCurrency,
    usesTime,
    ontogglelanguage,
    ontogglelocale,
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
  } = $props();
</script>

<footer class="status-bar">
  <span class="note" title={noteName ?? ""}>{noteName ?? ""}</span>
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
  button:hover,
  button:focus-visible {
    color: var(--text-color);
    background: color-mix(in srgb, var(--text-color) 10%, transparent);
    outline: none;
  }
</style>
