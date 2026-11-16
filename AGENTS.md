# notecal — notes for coding agents

A calculator notepad: Tauri 2 shell (Rust), Svelte 5 + CodeMirror 6 UI, and a
calculation engine built on fend. Linux (Wayland/Hyprland) is the main target;
macOS work is deferred.

## Layout

| Path | What |
|---|---|
| `crates/notecal-core/` | Engine: preprocessing, evaluation, currency, formatting. No UI or I/O. |
| `crates/notecal-core/tests/golden/*.txt` | Golden cases: `input => display [|| copy]`, `---` between notes, `@locale` / `@precision` directives |
| `src-tauri/src/` | App shell: notes storage (`notes.rs`), commands, settings, rates, theme, tray/window (`shell.rs`) |
| `ui/src/` | Svelte UI; editor extensions in `ui/src/editor/`, note session in `ui/src/notes/` |
| `docs/notecal.prd.md` | Product requirements and budgets (typing p95, cold start, RAM, package size) |
| `.claude/plans/*.plan.md` | Milestone plans with task lists; mark a task ✅ with a short Done note when finished |

## Check your work

Run before saying a change is done:

```sh
scripts/check.sh
```

It runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`,
and in `ui/`: `bun run check`, `bun run test`. A change is done only when it passes.
Add or update tests with the change: golden cases for engine behaviour, Rust unit
tests next to the code, vitest files beside the UI module (`*.test.ts`).

## Rules

- Notes are the user's data. Writes stay atomic (temp file, `sync_all`, rename) and
  never overwrite a note that changed on disk; that case makes a conflict copy.
- Don't block the main thread with disk or network work; use async commands.
- Keep the typing path cheap: the editor re-tokenises only edited lines.
- Match the surrounding code's style; no new dependencies without a reason stated
  in your summary.
- Git: do not commit, push, tag or rewrite history unless the user asks. Commit
  messages follow the existing style (`feat:`, `fix(core):`, …) with no
  attribution or co-author lines.
- Report what you verified and what you did not, plainly.
