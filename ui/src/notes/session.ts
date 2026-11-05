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
  /** Saves and reloads run one at a time, in order; this never rejects. */
  private queue: Promise<void> = Promise.resolve();
  /** Counts edits, to notice typing that happens while a save or load is awaited. */
  private generation = 0;

  constructor(private readonly deps: SessionDeps) {}

  get name(): string | null {
    return this.current;
  }

  get hasUnsavedEdits(): boolean {
    return this.dirty;
  }

  /**
   * Saves the current note, then shows `name`. Throws, without switching,
   * if the current note's edits could not be saved.
   */
  async open(name: string): Promise<void> {
    let seen: number;
    do {
      seen = this.generation;
      await this.flush();
    } while (this.generation !== seen);
    if (this.dirty) {
      throw new Error(`“${this.current}” has unsaved edits that could not be saved.`);
    }
    const text = await this.deps.open(name);
    this.current = name;
    this.dirty = false;
    this.replace(text);
  }

  /** Call after every document change. */
  edited(): void {
    if (this.replacing || this.current === null) return;
    this.generation += 1;
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
    return this.enqueue(() => this.saveIfDirty());
  }

  private enqueue(task: () => Promise<void>): Promise<void> {
    this.queue = this.queue.then(task).catch((error: unknown) => {
      this.deps.notify(`Something went wrong while saving: ${String(error)}`);
    });
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
  diskChanged(event: DiskChangeEvent): Promise<void> {
    // Deleted: keep the buffer; the next save writes the file again.
    if (event.kind === "deleted") return Promise.resolve();
    // Queued behind any save in flight, so the file is read after it lands.
    return this.enqueue(async () => {
      if (event.name !== this.current) return;
      if (this.dirty) {
        // Rust sees the disk differ and keeps both versions.
        this.cancelTimer();
        await this.saveIfDirty();
        return;
      }
      const seen = this.generation;
      const text = await this.deps.open(event.name);
      if (!this.dirty && this.generation === seen && this.current === event.name) this.replace(text);
    });
  }

  private async saveIfDirty(): Promise<void> {
    const name = this.current;
    if (!this.dirty || name === null) return;
    const text = this.deps.getText();
    const seen = this.generation;
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
      // Keystrokes typed while the save was awaited are newer than the copy.
      const latest = this.generation === seen ? null : this.deps.getText();
      this.replace(outcome.diskText);
      this.dirty = false;
      if (latest !== null) await this.deps.save(outcome.copyName, latest, conflictStamp(this.deps.now()));
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
