/** Mirrors `SaveOutcome` in src-tauri/src/notes.rs. */
export type SaveOutcome =
  | { kind: "saved" }
  | { kind: "conflict"; copyName: string; diskText: string };

/** Mirrors `DiskChangeEvent` in src-tauri/src/commands.rs. */
export interface DiskChangeEvent {
  name: string;
  kind: "changed" | "deleted";
}

export interface SessionDeps {
  open: (name: string) => Promise<string>;
  save: (name: string, text: string, stamp: string) => Promise<SaveOutcome>;
  getText: () => string;
  /** Replaces the editor's text, keeping the cursor where possible. */
  setText: (text: string) => void;
  /** Shows a message to the user (conflicts and save errors). */
  notify: (message: string) => void;
  setTimer: (callback: () => void, ms: number) => unknown;
  clearTimer: (handle: unknown) => void;
  now: () => Date;
}

/** PRD N2: save this long after the last edit. */
export const SAVE_DELAY_MS = 500;

/** `YYYY-MM-DD HHMM` in local time, for conflict copy names. */
export function conflictStamp(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ` +
    `${pad(date.getHours())}${pad(date.getMinutes())}`
  );
}

/**
 * The open note: debounced autosave, and the UI half of the PRD storage
 * rules. The Rust side decides whether a save is safe; this side reloads
 * when only the disk changed and shows the conflict notice.
 */
export class NoteSession {
  private current: string | null = null;
  private dirty = false;
  private timer: unknown = null;
  /** Set while the session itself replaces the text, so that is not an edit. */
  private replacing = false;
  /** Saves run one at a time, in order. */
  private queue: Promise<void> = Promise.resolve();

  constructor(private readonly deps: SessionDeps) {}

  get name(): string | null {
    return this.current;
  }

  get hasUnsavedEdits(): boolean {
    return this.dirty;
  }

  /** Saves the current note if needed, then shows `name`. */
  async open(name: string): Promise<void> {
    await this.flush();
    const text = await this.deps.open(name);
    this.current = name;
    this.dirty = false;
    this.replace(text);
  }

  /** Call after every document change. */
  edited(): void {
    if (this.replacing || this.current === null) return;
    this.dirty = true;
    this.cancelTimer();
    this.timer = this.deps.setTimer(() => {
      this.timer = null;
      void this.flush();
    }, SAVE_DELAY_MS);
  }

  /** Saves now if there are unsaved edits (note switch, blur, close). */
  flush(): Promise<void> {
    this.cancelTimer();
    this.queue = this.queue.then(() => this.saveIfDirty());
    return this.queue;
  }

  /** The open note was renamed in the switcher. */
  renamed(from: string, to: string): void {
    if (this.current === from) this.current = to;
  }

  /** The open note was deleted in the switcher; nothing is open until the next `open`. */
  closed(): void {
    this.cancelTimer();
    this.current = null;
    this.dirty = false;
  }

  /** Handles `note-changed-on-disk` from the watcher. */
  async diskChanged(event: DiskChangeEvent): Promise<void> {
    if (event.name !== this.current) return;
    // Deleted: keep the buffer; the next save writes the file again.
    if (event.kind === "deleted") return;
    if (this.dirty) {
      // Rust sees the disk differ and keeps both versions.
      await this.flush();
      return;
    }
    const text = await this.deps.open(event.name);
    if (!this.dirty && this.current === event.name) this.replace(text);
  }

  private async saveIfDirty(): Promise<void> {
    const name = this.current;
    if (!this.dirty || name === null) return;
    const text = this.deps.getText();
    this.dirty = false;
    let outcome: SaveOutcome;
    try {
      outcome = await this.deps.save(name, text, conflictStamp(this.deps.now()));
    } catch (error) {
      this.dirty = true;
      this.deps.notify(`Could not save “${name}”: ${String(error)}`);
      return;
    }
    if (outcome.kind === "conflict" && this.current === name) {
      this.replace(outcome.diskText);
      this.deps.notify(
        `“${name}” was changed outside notecal. Your version was saved as “${outcome.copyName}”.`,
      );
    }
  }

  private replace(text: string): void {
    this.replacing = true;
    try {
      this.deps.setText(text);
    } finally {
      this.replacing = false;
    }
  }

  private cancelTimer(): void {
    if (this.timer !== null) {
      this.deps.clearTimer(this.timer);
      this.timer = null;
    }
  }
}
