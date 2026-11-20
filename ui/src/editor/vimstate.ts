// Pure helpers for vim mode, kept apart from the library so the status bar
// and tests don't load it.

/** The part of the library's per-editor state that notecal reads. */
export interface VimInputState {
  insertMode?: boolean;
  visualMode?: boolean;
  inputState?: {
    keyBuffer?: string[];
    operator?: string | null;
    operatorArgs?: { indentRight?: boolean } | null;
    /** The keys that chose the operator, set only when more than one (`gU`). */
    operatorShortcut?: string;
    prefixRepeat?: string[];
    motionRepeat?: string[];
    registerName?: string;
  };
}

/** What the status bar shows: `NORMAL`, `INSERT`, … and keys typed so far. */
export interface VimStatus {
  mode: string;
  pending: string;
}

const LABELS: Record<string, string> = {
  normal: "NORMAL",
  insert: "INSERT",
  replace: "REPLACE",
  visual: "VISUAL",
  "visual linewise": "V-LINE",
  "visual blockwise": "V-BLOCK",
};

/** Keys of the one-key operators in normal mode (multi-key ones have `operatorShortcut`). */
const OPERATOR_KEYS: Record<string, string> = { delete: "d", yank: "y", change: "c", indentAuto: "=" };

function operatorKeys(input: NonNullable<VimInputState["inputState"]>): string {
  if (!input.operator) return "";
  if (input.operatorShortcut) return input.operatorShortcut;
  if (input.operator === "indent") return input.operatorArgs?.indentRight ? ">" : "<";
  return OPERATOR_KEYS[input.operator] ?? "";
}

/** The pending command as typed: `"a2d3`. Built from the input state, since the
 *  library's own key echo drops the count once an operator follows it. */
function pendingKeys(vim: VimInputState): string {
  const input = vim.inputState;
  if (!input || vim.insertMode) return "";
  return [
    input.registerName ? `"${input.registerName}` : "",
    ...(input.prefixRepeat ?? []),
    operatorKeys(input),
    ...(input.motionRepeat ?? []),
    ...(input.keyBuffer ?? []),
  ].join("");
}

/** `mode` is the library's mode-change event (`visual linewise`, …). */
export function describeVim(mode: string, vim: VimInputState | null): VimStatus {
  const label = LABELS[mode] ?? mode.toUpperCase();
  return { mode: label, pending: vim ? pendingKeys(vim) : "" };
}
