<script lang="ts">
  import { onMount, tick } from "svelte";
  import { notesApi, type NoteInfo } from "./api";

  let {
    current,
    beforechange,
    onopen,
    onrenamed,
    ondeleted,
    onclose,
    onerror,
  }: {
    current: string | null;
    /** Settles pending saves, so none lands on a renamed or deleted note. */
    beforechange: () => Promise<void>;
    onopen: (name: string) => void;
    onrenamed: (from: string, to: string) => void;
    ondeleted: (name: string) => void;
    onclose: () => void;
    onerror: (message: string) => void;
  } = $props();

  type Row = { kind: "note"; name: string } | { kind: "create"; name: string };

  let notes = $state<NoteInfo[]>([]);
  let query = $state("");
  let selected = $state(0);
  /** Row being renamed, and the name typed so far. */
  let renaming = $state<{ from: string; to: string } | null>(null);
  /** Note waiting for delete confirmation. */
  let confirmDelete = $state<string | null>(null);
  let input: HTMLInputElement;
  let renameInput = $state<HTMLInputElement>();

  const rows = $derived.by((): Row[] => {
    const q = query.trim();
    const lower = q.toLowerCase();
    const matches: Row[] = notes
      .filter((n) => n.name.toLowerCase().includes(lower))
      .map((n) => ({ kind: "note", name: n.name }));
    const exact = notes.some((n) => n.name.toLowerCase() === lower);
    return q && !exact ? [...matches, { kind: "create", name: q }] : matches;
  });

  $effect(() => {
    if (selected >= rows.length) selected = Math.max(0, rows.length - 1);
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
  <div class="switcher" role="dialog" aria-modal="true" aria-label="Notes" tabindex="-1" onkeydown={onKeydown}>
    <input
      bind:this={input}
      bind:value={query}
      oninput={() => (selected = 0)}
      placeholder="Find or create a note"
      aria-label="Find or create a note"
      role="combobox"
      aria-expanded={rows.length > 0}
      aria-controls="note-list"
      aria-activedescendant={rows.length ? `note-row-${selected}` : undefined}
      spellcheck="false"
      autocomplete="off"
    />
    <!-- Clicking a row must not take focus from the input. -->
    <ul id="note-list" role="listbox" aria-label="Notes" onmousedown={(e) => e.preventDefault()}>
      {#each rows as row, i (row.kind + row.name)}
        <li
          id="note-row-{i}"
          role="option"
          aria-selected={i === selected}
          class:selected={i === selected}
          onmousemove={() => (selected = i)}
          onclick={() => void choose(row)}
          onkeydown={() => {}}
        >
          {#if row.kind === "create"}
            <span class="create">Create “{row.name}”</span>
          {:else if renaming?.from === row.name}
            <input
              class="rename"
              bind:this={renameInput}
              bind:value={renaming.to}
              onblur={() => void commitRename()}
              onclick={(e) => e.stopPropagation()}
              aria-label="New name for {row.name}"
              spellcheck="false"
            />
          {:else if confirmDelete === row.name}
            <span class="confirm">Delete “{row.name}”? Enter to delete, any other key to keep</span>
          {:else}
            <span class="name">{row.name}</span>
            {#if row.name === current}<span class="tag">open</span>{/if}
            <span class="actions">
              <button
                type="button"
                title="Rename (F2)"
                onclick={(e) => {
                  e.stopPropagation();
                  void startRename(row.name);
                }}>Rename</button
              >
              <button
                type="button"
                title="Delete (Ctrl+Delete)"
                onclick={(e) => {
                  e.stopPropagation();
                  confirmDelete = row.name;
                  input.focus();
                }}>Delete</button
              >
            </span>
          {/if}
        </li>
      {:else}
        <li class="empty" role="option" aria-selected="false" aria-disabled="true">
          Type a name to create a note
        </li>
      {/each}
    </ul>
    <p class="sr-only" aria-live="assertive">
      {confirmDelete ? `Delete “${confirmDelete}”? Press Enter to delete, any other key to keep.` : ""}
    </p>
    <p class="hint">Enter open · F2 rename · Ctrl+Delete delete · Esc close</p>
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
</style>
