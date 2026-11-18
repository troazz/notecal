<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { notesApi, type NoteInfo } from "./api";
  import { t, type Key } from "../i18n.svelte";

  let {
    current,
    beforechange,
    onopen,
    onrenamed,
    ondeleted,
    onclose,
    onerror,
    oninsert,
  }: {
    current: string | null;
    /** Settles pending saves, so none lands on a renamed or deleted note. */
    beforechange: () => Promise<void>;
    onopen: (name: string) => void;
    onrenamed: (from: string, to: string) => void;
    ondeleted: (name: string) => void;
    onclose: () => void;
    onerror: (message: string) => void;
    /** Inserts a cheatsheet example into the note (M11). */
    oninsert: (text: string) => void;
  } = $props();

  /** Mirrors `CheatSection` in src-tauri/src/evaluator.rs. */
  type CheatSection = {
    id: string;
    examples: { text: string; result: string | null }[];
  };

  type Row =
    | { kind: "note"; name: string }
    | { kind: "create"; name: string }
    | { kind: "cheatsheet" }
    | { kind: "example"; title: string; text: string; result: string | null; first: boolean };

  function sectionTitle(id: string): string {
    return t(`cheatsheet.${id}` as Key);
  }

  function exampleRows(filter: string): Row[] {
    const rows: Row[] = [];
    for (const section of sheet ?? []) {
      const title = sectionTitle(section.id);
      // The English id too, so `?date` works whatever the interface language.
      const inTitle = title.toLowerCase().includes(filter) || section.id.includes(filter);
      let first = true;
      for (const example of section.examples) {
        if (!inTitle && !example.text.toLowerCase().includes(filter)) continue;
        rows.push({ kind: "example", title, ...example, first });
        first = false;
      }
    }
    return rows;
  }

  let notes = $state<NoteInfo[]>([]);
  let query = $state("");
  let selected = $state(0);
  /** Row being renamed, and the name typed so far. */
  let renaming = $state<{ from: string; to: string } | null>(null);
  /** Note waiting for delete confirmation. */
  let confirmDelete = $state<string | null>(null);
  let input: HTMLInputElement;
  let renameInput = $state<HTMLInputElement>();

  /** M11: typing `?` turns the switcher into the cheatsheet. */
  const cheatMode = $derived(query.trimStart().startsWith("?"));
  let sheet = $state<CheatSection[] | null>(null);

  $effect(() => {
    if (cheatMode && sheet === null) {
      sheet = [];
      invoke<CheatSection[]>("cheatsheet")
        .then((sections) => (sheet = sections))
        .catch((error: unknown) => onerror(String(error)));
    }
  });


  const rows = $derived.by((): Row[] => {
    if (cheatMode) return exampleRows(query.trimStart().slice(1).trim().toLowerCase());
    const q = query.trim();
    const lower = q.toLowerCase();
    const matches: Row[] = notes
      .filter((n) => n.name.toLowerCase().includes(lower))
      .map((n) => ({ kind: "note", name: n.name }));
    const exact = notes.some((n) => n.name.toLowerCase() === lower);
    if (!q) return [...matches, { kind: "cheatsheet" }];
    return exact ? matches : [...matches, { kind: "create", name: q }];
  });

  $effect(() => {
    if (selected >= rows.length) selected = Math.max(0, rows.length - 1);
  });

  // Keep the selected row in view when moving with the arrow keys (long
  // lists such as the cheatsheet scroll).
  $effect(() => {
    const id = `note-row-${selected}`;
    void rows.length;
    tick().then(() => document.getElementById(id)?.scrollIntoView({ block: "nearest" }));
  });

  async function refresh() {
    try {
      notes = await notesApi.list();
    } catch (error) {
      onerror(String(error));
    }
  }

  onMount(() => {
    void refresh();
    input.focus();
    const stop = notesApi.onListChanged(() => void refresh());
    return () => void stop.then((unlisten) => unlisten());
  });

  async function choose(row: Row | undefined) {
    if (!row) return;
    if (row.kind === "cheatsheet") {
      query = "?";
      selected = 0;
      input.focus();
      return;
    }
    if (row.kind === "example") {
      oninsert(row.text);
      return;
    }
    if (row.kind === "create") {
      try {
        await notesApi.create(row.name);
      } catch (error) {
        onerror(String(error));
        return;
      }
    }
    onopen(row.name);
  }

  async function startRename(name: string) {
    renaming = { from: name, to: name };
    await tick();
    renameInput?.select();
  }

  async function commitRename() {
    if (!renaming) return;
    const { from, to } = renaming;
    const target = to.trim();
    renaming = null;
    if (target && target !== from) {
      try {
        await beforechange();
        await notesApi.rename(from, target);
        onrenamed(from, target);
        await refresh();
      } catch (error) {
        onerror(String(error));
      }
    }
    input.focus();
  }

  async function commitDelete() {
    const name = confirmDelete;
    confirmDelete = null;
    if (!name) return;
    try {
      await beforechange();
      await notesApi.delete(name);
      ondeleted(name);
      await refresh();
    } catch (error) {
      onerror(String(error));
    }
    input.focus();
  }

  function onKeydown(event: KeyboardEvent) {
    if (renaming) {
      if (event.key === "Enter") void commitRename();
      if (event.key === "Escape") {
        renaming = null;
        input.focus();
      }
      if (event.key === "Enter" || event.key === "Escape") event.preventDefault();
      return;
    }
    if (confirmDelete) {
      event.preventDefault();
      if (event.key === "Enter" || event.key.toLowerCase() === "y") void commitDelete();
      else confirmDelete = null;
      return;
    }
    const row = rows[selected];
    switch (event.key) {
      case "Tab":
        // Keep focus inside the dialog; the input is its only stop.
        input.focus();
        break;
      case "ArrowDown":
        selected = Math.min(selected + 1, rows.length - 1);
        break;
      case "ArrowUp":
        selected = Math.max(selected - 1, 0);
        break;
      case "Enter":
        void choose(row);
        break;
      case "Escape":
        onclose();
        break;
      case "F2":
        if (row?.kind === "note") void startRename(row.name);
        break;
      case "Delete":
        if (!(event.ctrlKey || event.metaKey) || row?.kind !== "note") return;
        confirmDelete = row.name;
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="switcher" role="dialog" aria-modal="true" aria-label={t("switcher.notes")} tabindex="-1" onkeydown={onKeydown}>
    <input
      bind:this={input}
      bind:value={query}
      oninput={() => (selected = 0)}
      placeholder={t("switcher.placeholder")}
      aria-label={t("switcher.placeholder")}
      role="combobox"
      aria-expanded={rows.length > 0}
      aria-controls="note-list"
      aria-activedescendant={rows.length ? `note-row-${selected}` : undefined}
      spellcheck="false"
      autocomplete="off"
    />
    <!-- Clicking a row must not take focus from the input. -->
    <ul id="note-list" role="listbox" aria-label={t("switcher.notes")} onmousedown={(e) => e.preventDefault()}>
      {#each rows as row, i (row.kind === "example" ? row.title + row.text : row.kind === "cheatsheet" ? "cheatsheet" : row.kind + row.name)}
        <li
          id="note-row-{i}"
          role="option"
          aria-selected={i === selected}
          class:selected={i === selected}
          onmousemove={() => (selected = i)}
          onclick={() => void choose(row)}
          onkeydown={() => {}}
        >
          {#if row.kind === "cheatsheet"}
            <span class="cheat-link">{t("cheatsheet.open")}</span>
            <span class="tag">?</span>
          {:else if row.kind === "example"}
            <span class="example">
              {#if row.first}<span class="section">{row.title}</span>{/if}
              <span class="example-line">
                <code>{row.text.replaceAll("\n", "  ⏎  ")}</code>
                <span class="result">{row.result ?? ""}</span>
              </span>
            </span>
          {:else if row.kind === "create"}
            <span class="create">{t("switcher.create", { name: row.name })}</span>
          {:else if renaming?.from === row.name}
            <input
              class="rename"
              bind:this={renameInput}
              bind:value={renaming.to}
              onblur={() => void commitRename()}
              onclick={(e) => e.stopPropagation()}
              aria-label={t("switcher.newName", { name: row.name })}
              spellcheck="false"
            />
          {:else if confirmDelete === row.name}
            <span class="confirm">{t("switcher.confirmDeleteShort", { name: row.name })}</span>
          {:else}
            <span class="name">{row.name}</span>
            {#if row.name === current}<span class="tag">{t("switcher.open")}</span>{/if}
            <span class="actions">
              <button
                type="button"
                title={t("switcher.renameTitle")}
                onclick={(e) => {
                  e.stopPropagation();
                  void startRename(row.name);
                }}>{t("switcher.rename")}</button
              >
              <button
                type="button"
                title={t("switcher.deleteTitle")}
                onclick={(e) => {
                  e.stopPropagation();
                  confirmDelete = row.name;
                  input.focus();
                }}>{t("switcher.delete")}</button
              >
            </span>
          {/if}
        </li>
      {:else}
        <li class="empty" role="option" aria-selected="false" aria-disabled="true">
          {cheatMode ? t("cheatsheet.none") : t("switcher.empty")}
        </li>
      {/each}
    </ul>
    <p class="sr-only" aria-live="assertive">
      {confirmDelete ? t("switcher.confirmDelete", { name: confirmDelete }) : ""}
    </p>
    {#if cheatMode}
      <p class="hint">{t("cheatsheet.help")}</p>
      <p class="hint">{t("cheatsheet.footer")}</p>
    {:else}
      <p class="hint">{t("switcher.help")}</p>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: color-mix(in srgb, var(--bg-color) 40%, transparent);
    z-index: 10;
  }
  .switcher {
    width: min(32rem, calc(100% - 2rem));
    background: var(--panel-color);
    color: var(--text-color);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.25);
    font: 14px system-ui, sans-serif;
    overflow: hidden;
  }
  input {
    box-sizing: border-box;
    width: 100%;
    padding: 0.6rem 0.75rem;
    border: 0;
    border-bottom: 1px solid var(--border-color);
    background: transparent;
    color: inherit;
    font: inherit;
    outline: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0.25rem 0;
    max-height: 50vh;
    overflow-y: auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.75rem;
    cursor: pointer;
    min-height: 1.6rem;
  }
  li.selected {
    background: var(--selection-color);
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag,
  .hint,
  .empty {
    color: var(--muted-color);
    font-size: 12px;
  }
  .create {
    color: var(--accent-color);
  }
  .confirm {
    color: var(--error-color);
  }
  .actions {
    display: none;
    gap: 0.25rem;
  }
  li.selected .actions {
    display: flex;
  }
  button {
    font: 12px system-ui, sans-serif;
    padding: 0.1rem 0.4rem;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    background: var(--bg-color);
    color: var(--text-color);
    cursor: pointer;
  }
  .rename {
    padding: 0.1rem 0.3rem;
    border: 1px solid var(--accent-color);
    border-radius: 4px;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .hint {
    margin: 0;
    padding: 0.4rem 0.75rem;
    border-top: 1px solid var(--border-color);
  }
  .cheat-link {
    flex: 1;
    color: var(--muted-color);
  }
  .example {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .section {
    margin-top: 0.2rem;
    color: var(--muted-color);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .example-line {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    min-width: 0;
  }
  .example-line code {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--editor-font, monospace);
  }
  .example-line .result {
    flex: none;
    color: var(--muted-color);
    font-family: var(--editor-font, monospace);
  }
</style>
