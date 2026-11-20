// Vim mode (PRD E10). Loaded with import() only when `vim_mode` is on, so
// users without it don't pay for the library.
import { Vim, getCM, vim } from "@replit/codemirror-vim";
import type { Extension } from "@codemirror/state";
import { ViewPlugin, type EditorView } from "@codemirror/view";
import { describeVim, type VimInputState, type VimStatus } from "./vimstate";

export interface VimHandlers {
  /** The mode or the pending keys changed. */
  onstatus: (status: VimStatus) => void;
  /** `:w`: save now. */
  onwrite: () => void;
  /** `:q`, `:x`, `:wq`: save and hide the window. */
  onquit: () => void;
}

// Ex commands are global in the library; they call whichever editor set them last
// (notecal has one).
let handlers: VimHandlers | null = null;
Vim.defineEx("write", "w", () => handlers?.onwrite());
Vim.defineEx("quit", "q", () => handlers?.onquit());
Vim.defineEx("xit", "x", () => handlers?.onquit());
Vim.defineEx("wq", "wq", () => handlers?.onquit());

interface VimCM {
  state: { vim?: VimInputState | null; dialog?: unknown };
  on: (type: string, f: (event?: { mode: string; subMode?: string }) => void) => void;
  off: (type: string, f: (event?: { mode: string; subMode?: string }) => void) => void;
}

/** Reports the mode to the status bar. Runs after the vim plugin is built. */
const statusReporter = ViewPlugin.fromClass(
  class {
    private cm: VimCM | null;
    private mode = "normal";
    private onMode = (event?: { mode: string; subMode?: string }) => {
      if (event) this.mode = event.subMode ? `${event.mode} ${event.subMode}` : event.mode;
      this.report();
    };
    private onKey = () => this.report();

    constructor(view: EditorView) {
      this.cm = getCM(view) as unknown as VimCM | null;
      this.cm?.on("vim-mode-change", this.onMode);
      this.cm?.on("vim-keypress", this.onKey);
      this.cm?.on("vim-command-done", this.onKey);
      this.report();
    }

    report() {
      const vimState = this.cm?.state.vim;
      handlers?.onstatus(describeVim(this.mode, vimState ?? null));
    }

    destroy() {
      this.cm?.off("vim-mode-change", this.onMode);
      this.cm?.off("vim-keypress", this.onKey);
      this.cm?.off("vim-command-done", this.onKey);
    }
  },
);

/** The extension: vim itself, then the reporter. */
export function vimExtension(next: VimHandlers): Extension {
  handlers = next;
  return [vim(), statusReporter];
}
