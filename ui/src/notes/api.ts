import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DiskChangeEvent, SaveOutcome } from "./session";

/** Mirrors `NoteInfo` in src-tauri/src/notes.rs. */
export interface NoteInfo {
  name: string;
  modifiedMs: number;
}

/** Mirrors `UiState` in src-tauri/src/state.rs. */
export interface UiState {
  lastNote: string | null;
  cursor: number;
  /** First-run onboarding (A8); the backend keeps it when the UI saves. */
  onboarding?: "pending" | "done";
}

/** Typed wrappers for the note commands in src-tauri/src/commands.rs. */
export const notesApi = {
  list: () => invoke<NoteInfo[]>("list_notes"),
  open: (name: string) => invoke<string>("open_note", { name }),
  save: (name: string, text: string, stamp: string) =>
    invoke<SaveOutcome>("save_note", { name, text, stamp }),
  create: (name: string) => invoke<void>("create_note", { name }),
  rename: (from: string, to: string) => invoke<void>("rename_note", { from, to }),
  delete: (name: string) => invoke<void>("delete_note", { name }),
  loadState: () => invoke<UiState>("load_state"),
  saveState: (uiState: UiState) => invoke<void>("save_state", { uiState }),
  finishOnboarding: () => invoke<void>("finish_onboarding"),
  onDiskChange: (handler: (event: DiskChangeEvent) => void): Promise<UnlistenFn> =>
    listen<DiskChangeEvent>("note-changed-on-disk", (e) => handler(e.payload)),
  onListChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen("notes-list-changed", () => handler()),
};
