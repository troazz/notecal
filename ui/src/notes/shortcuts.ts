import type { Key } from "../i18n.svelte";

/** A row of the cheatsheet's keyboard-shortcuts section. */
export interface Shortcut {
  keys: string;
  label: Key;
  /** Shown only while vim mode is on. */
  vim?: true;
  /** Enter runs it instead of doing nothing. */
  action?: "welcome";
}

/** The app's keys, for the cheatsheet (M12). Keep in step with App.svelte and editor/results.ts. */
export const SHORTCUTS: readonly Shortcut[] = [
  { keys: "Ctrl+P", label: "keys.notes" },
  { keys: "Ctrl+P ?", label: "keys.cheatsheet" },
  { keys: "Ctrl+,", label: "keys.settings" },
  { keys: "Alt+Space", label: "keys.hide" },
  { keys: "Ctrl+Shift+T", label: "keys.pin" },
  { keys: "Ctrl+Shift+C", label: "keys.copy" },
  { keys: "Ctrl+Shift+B", label: "keys.copyBare" },
  { keys: "Click", label: "keys.clickCopy" },
  { keys: "Ctrl+Shift+A", label: "keys.announce" },
  { keys: "Ctrl+F", label: "keys.find" },
  { keys: "Ctrl+Z / Ctrl+Shift+Z", label: "keys.undo" },
  { keys: "Ctrl+Shift+M", label: "keys.stats" },
  { keys: "Enter", label: "keys.welcome", action: "welcome" },
  { keys: ":w", label: "keys.vimWrite", vim: true },
  { keys: ":q  :x  :wq", label: "keys.vimQuit", vim: true },
  { keys: '"+y  "+p', label: "keys.vimClipboard", vim: true },
];
