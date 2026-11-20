# Plan: Vim mode, always-on-top toggle, searchable cheatsheet help (M12)

**Source PRD**: `docs/notecal.prd.md` (new requirement E10, A4 extended, milestone 12)
**Selected Milestone**: 12 — Vim mode, always-on-top toggle, searchable help
**Complexity**: Medium (~1 w)

## Summary
An opt-in vim mode for the editor: normal, insert, visual and visual-line modes,
motions, operators, counts, registers, `.` repeat, `u`/`Ctrl+R`, `/` search and a
few ex commands, from `@replit/codemirror-vim`. It is off by default, toggled by a
`vim_mode` setting (settings file, Settings dialog), and the status bar shows the
current mode. App shortcuts (Ctrl+P, Ctrl+,, Ctrl+Shift+M/T) keep working, and Esc
leaves insert mode before it ever hides the window.

Part B extends always-on-top (A4): it exists today (Ctrl+Shift+T and a Settings
checkbox), but only on X11 and macOS, because Tauri's `set_always_on_top` does nothing
on Wayland. M12 adds a pin button to the status bar and makes the toggle work on
Hyprland.

Part C makes the cheatsheet (M11) a searchable help. Today the filter matches only a
section title, a section id or the example's own text, so `?discount` finds nothing.
Each example gets a one-line explanation in both languages, and a "Keyboard shortcuts"
section lists the app's keys, including the vim commands and the pin. The filter
searches all of it.

## Requirements
- **E10 (P2):** a vim keybinding mode, off by default.
- Toggle: `vim_mode = true|false` in `settings.toml`, and a checkbox in Settings.
  The change applies live, without restarting and without losing undo history.
- Status bar: `NORMAL` / `INSERT` / `VISUAL` / `V-LINE` / `REPLACE` while vim mode
  is on; nothing extra when it's off. Pending keys (`2d`, `"a`) are shown too.
- **Esc:** in insert or visual mode it returns to normal mode. In normal mode with no
  pending keys it hides the window, as it does today (key scenario 1).
- Ex commands: `:w` saves now, `:q` / `:x` / `:wq` save and hide the window. Others
  that the library handles (`:s`, `:noh`, `:<line>`) stay as they are.
- Yank and put use the system clipboard (`"+` and the unnamed register), so a
  copied result can be pasted with `p`.
- App shortcuts win over vim: Ctrl+P (switcher), Ctrl+, (settings), Ctrl+Shift+M,
  Ctrl+Shift+T. Vim loses Ctrl+P in normal mode (it's just `k`).
- Read-only mode (conflict view) keeps working: motions and yank are allowed, edits are not.
- Translated: settings label and mode names in English and Indonesian (mode names
  stay in English; they're vim's terms).

### Part B: always-on-top toggle
- **Status bar button:** a pin button beside the language button. It shows the state
  (filled when on), and its tooltip names the shortcut. Clicking it does the same as
  Ctrl+Shift+T: it flips `always_on_top` and saves the setting.
- **Hyprland:** when `HYPRLAND_INSTANCE_SIGNATURE` is set, the toggle pins the window
  through `hyprctl`, because Tauri's call is a no-op there. In Hyprland, pin keeps a
  floating window above others and on every workspace. notecal floats through the
  Omarchy window rule, so the window can be pinned. The pin is applied:
  - when the setting changes;
  - every time the window is shown, because Hyprland drops the pin when the window
    is unmapped (hidden).
- **Other Wayland compositors:** there is no protocol for this, so the button is
  disabled, with a tooltip saying why. The setting is still saved.
- **Labels:** the Settings label drops "(X11/macOS)" and becomes
  "(X11, macOS, Hyprland)".

### Part C: searchable cheatsheet help
- **Explanations:** every example has a one-line explanation, such as "20% of 150 →
  a percentage of a number" or "persentase dari sebuah angka".
  - It's shown dimmed under the example.
  - It's translated in en and id.
  - The copy inserted on Enter is unchanged: the example only.
- **Search:**
  - Split the query into words. Every word must match the section title, the
    explanation, the example text or the section's keywords.
  - The match ignores case and accents.
  - English words still match while the UI is Indonesian, as section ids do today:
    `?discount` works in both languages.
- **Keyboard shortcuts section:**
  - Lists the app's keys: Ctrl+P, `?`, Ctrl+,, Esc, Ctrl+Shift+T/M, copy result, and
    search.
  - When vim mode is on, it also lists the vim notes: Esc, `:w`, `:q`, and `"+` for
    the clipboard.
  - Its rows are help only: Enter on them inserts nothing.
  - The list comes from one table, so the Settings labels and the cheatsheet can't
    drift apart.
- **Zero matches:** the empty state shows the query and a hint to try fewer words.

**Non-goals:** a `.vimrc` or user key mappings, macros persisted across sessions,
`:e`/`:split`/buffers, vim keys in the switcher or settings dialogs, a vim mode for
the CLI.

## Patterns to Mirror
| Category | Source | Pattern |
|---|---|---|
| Editor extensions | `ui/src/Editor.svelte:79-110` | `Compartment` (as `readOnlyMode`) reconfigured by `dispatch`; `setText` re-applies compartments after `setState` |
| Setting (Rust) | `crates/notecal-env/src/settings.rs:83-127, 255` | field on `AppSettings` with a doc comment, default, bool arm in `parse`, unit tests below |
| Setting (UI) | `ui/src/settings/api.ts:16`, `ui/src/settings/Settings.svelte:247-262` | typed field; checkbox with `data-key` calling `update({...})` |
| Shortcuts | `ui/src/App.svelte:301-327` | one `onKeydown` on `svelte:window`; Esc branch checks `event.defaultPrevented` and `inEditorPanel` |
| Status bar | `ui/src/StatusBar.svelte:4-20` | props from `App.svelte`, text from `t()` |
| Strings | `ui/src/i18n.svelte.ts` | `settings.*` / `status.*` keys in both tables; `i18n.test.ts` checks the tables match |
| Tests | `ui/src/editor/*.test.ts`, `settings.rs` `#[test]` | vitest beside the module; Rust unit tests next to the code |

## Files to Change
| File | Action | Why |
|---|---|---|
| `ui/package.json`, `ui/bun.lock` | UPDATE | add `@replit/codemirror-vim`, exact version (the standard CM6 vim; MIT; writing our own vim is far more code) |
| `ui/src/editor/vim.ts` | CREATE | builds the vim extension: clipboard register, `:w`/`:q`/`:x`/`:wq`, mode-change listener, `isPending()` for Esc |
| `ui/src/editor/vim.test.ts` | CREATE | mode changes, ex commands, Esc behaviour, read-only, setText keeps mode |
| `ui/src/Editor.svelte` | UPDATE | `vimMode` compartment placed first (vim keymap has to run before `defaultKeymap`); `setVim(on)` on the controller; `onmode` callback |
| `ui/src/App.svelte` | UPDATE | pass `vim_mode` to the editor; Esc hides only if vim isn't in insert/visual or pending; `:w`/`:q` hooks to `persist()` and hide |
| `ui/src/StatusBar.svelte` | UPDATE | mode indicator |
| `crates/notecal-env/src/settings.rs` | UPDATE | `vim_mode: bool`, default false, parse arm, tests |
| `ui/src/settings/api.ts`, `ui/src/settings/Settings.svelte` | UPDATE | field and checkbox |
| `ui/src/i18n.svelte.ts` | UPDATE | `settings.vimMode`, `status.vim*` keys (en, id) |
| `docs/notecal.prd.md` | UPDATE | E10 row; milestone 12 row |
| `README.md`, `docs/linux-setup.md`, `CHANGELOG.md` | UPDATE | the setting and the Esc rule |
| `src-tauri/src/shell.rs` | UPDATE | `apply_always_on_top`: Tauri call, or `hyprctl` pin on Hyprland; re-applied after `show` |
| `src-tauri/src/commands.rs` | UPDATE | `window_pin_support` command: `native` / `hyprland` / `none`, for the button state |
| `crates/notecal-core/src/cheatsheet.rs` | UPDATE | examples become `Example { id, text }` with stable ids, so the UI can translate an explanation per example; keyword list per section |
| `src-tauri/src/evaluator.rs` | UPDATE | `CheatExample` carries `id` |
| `ui/src/notes/cheatsearch.ts` | CREATE | pure `matchExample(query, row)`: words, case- and accent-folding, en + current-language text |
| `ui/src/notes/cheatsearch.test.ts` | CREATE | search cases in both languages |
| `ui/src/notes/shortcuts.ts` | CREATE | the shortcut table (keys, i18n key, vim-only flag) |
| `ui/src/notes/Switcher.svelte` | UPDATE | explanation line under each example; shortcut rows; use `matchExample` |

## Tasks
### Task 1: Setting ✅
- **Done**: `vim_mode` in `AppSettings` (default off, bool arm, round-trip test); Settings checkbox; en/id label.
- **Action**: add `vim_mode` to `AppSettings` (default false), the bool arm in `parse`,
  serialisation, the TS type and the Settings checkbox with en/id labels.
- **Mirror**: `always_on_top`.
- **Validate**: `cargo test -p notecal-env` (parse true/false/bad value → problem); `bun run check`.

### Task 2: Vim extension module ✅
- **Done**: `editor/vim.ts` (loaded by `import()`, 123 KB / 39 KB gzip chunk) defines `:w`, `:q`, `:x`, `:wq` and reports the mode. The library already maps `"+y`/`"+p` to the system clipboard, so no custom register: plain `y`/`p` stay vim's own, like vim. Pure parts in `editor/vimstate.ts` with tests: mode labels, pending keys built from the input state (the library's echo drops the count, and records the operator key only for multi-key operators), and the busy rule.
- **Action**: `ui/src/editor/vim.ts` exports `vimExtension({ onmode, onwrite, onquit })`.
  Uses `vim()` from the library, `Vim.defineEx` for `w`, `q`, `x`, `wq`, and the
  library's `vim-mode-change` event for `onmode`. Registers `+`/unnamed to the
  clipboard through `navigator.clipboard` (falls back to the internal register if the
  clipboard is denied). Exposes `vimBusy(view)`: true in insert/visual or with pending keys.
- **Validate**: `vim.test.ts` in jsdom/happy-dom with a real `EditorView`: `i`, typing,
  Esc → normal; `dd` deletes a line; `:w` calls `onwrite`; `vimBusy` true after `d`.

### Task 3: Editor wiring ✅
- **Done**: `vimMode` compartment first in the extensions, re-applied by `setText`; `setVim(handlers|null)` and `vimBusy()` on the controller; later toggles win over a slow import. The vim block cursor takes the editor's blink rate (0), so it stays steady.
- **Action**: a `vimMode` `Compartment` at the head of `extensions`; controller gets
  `setVim(on: boolean)`; `setText` re-applies it like `readOnlyMode`. The vim cursor
  must not blink (Gate 2 CPU finding): pass the library's option or override with CSS
  `animation: none` on `.cm-fat-cursor`.
- **Validate**: toggling vim keeps undo history and cursor; Ctrl+Shift+M p95 unchanged
  by hand on the 200-line note.

### Task 4: App shortcuts and Esc ✅
- **Done**: Capture-phase handler: the Esc decision is taken before vim sees the key (vim lets Esc bubble after cancelling `d`, which would otherwise hide the window). App Ctrl shortcuts run before vim from anywhere in the editor, including vim's `:`/`/` prompts; from a prompt the shortcut runs after the prompt's own timed close, or vim takes the focus back from the switcher. Shortcut actions now live in `shortcutAction()`, shared by both phases. Checked in the app: Esc in insert → normal (window stays), `2d` then Esc cancels (stays), Esc in normal hides; Ctrl+P from normal mode and from the `/` prompt.
- **Action**: in `App.svelte` `onKeydown`, the Esc branch also requires
  `!editor.vimBusy()`. Make sure vim doesn't swallow Ctrl+P / Ctrl+, : the app
  shortcuts move to the capture phase (`addEventListener("keydown", …, true)`) so they
  run before CodeMirror, and still `preventDefault`.
- **Validate**: vitest for the Esc decision as a pure function
  (`shouldHideOnEscape({ vimOn, busy, panelOpen, ... })`); by hand: Esc in insert →
  normal, Esc again → hides; Ctrl+P opens the switcher from normal and insert.

### Task 5: Status bar ✅
- **Done**: Mode in the accent colour at the left of the status bar, pending keys after it (`NORMAL 2d`).
- **Action**: `StatusBar` gets `vimMode: string | null`; shows it at the left of the
  bar in the accent colour, with pending keys after it. `null` when vim is off.
- **Validate**: `bun run check`; screenshot.

### Task 6: Always-on-top on Hyprland ✅
- **Done**: `pin.rs`: Tauri's call on X11/macOS/Windows; on Hyprland `hyprctl clients -j` → our pid → pin only if the state differs, Lua dispatch first (`hl.dsp.window.pin`), classic `pin address:` as fallback; on its own thread, waits up to 0.5 s for the window to map; re-applied after every show. Unit tests for the client lookup. Checked on Hyprland 0.56: Ctrl+Shift+T pins, still pinned after hide/show, unpins.
- **Action**: in `shell.rs`, split `apply_always_on_top(window, on)` out of
  `apply_window_settings`.
  - **On Hyprland:** look up the window's address with `hyprctl clients -j`, matching
    our PID. Read its `pinned` state, and dispatch pin only when it differs, because
    pin is a toggle and a blind call would flip it the wrong way. Verify the dispatch
    syntax against the installed Hyprland: this machine uses the Lua config
    (`hl.dsp.*`).
  - **When to run it:** after the window is shown in the `--toggle` path and the tray
    path. Run it off the main thread (async, per AGENTS.md).
  - **Failures:** a missing `hyprctl` or a failed call is logged, as today.
- **Mirror**: `apply_window_settings`, `is_wayland`.
- **Validate**:
  - Unit test the parsing of `hyprctl clients -j` output and the compare-then-dispatch
    decision with a fixture.
  - By hand: pin, hide, show, and check the window is still pinned. Switch workspace
    and check the window follows.

### Task 7: Status bar pin button ✅
- **Done**: Pin button (filled and accent when on, `aria-pressed`), disabled with a reason where `pin_support` is `none`.
- **Action**:
  - Add a `pinned` prop and an `ontogglepin` callback to `StatusBar`. `ontogglepin`
    calls the existing `toggleAlwaysOnTop()` in `App.svelte`.
  - The icon is an inline SVG pin. Set `aria-pressed`, and a tooltip
    "Always on top (Ctrl+Shift+T)" in en and id.
  - When `window_pin_support` is `none`, disable the button with a tooltip explaining
    why.
- **Mirror**: the language and locale buttons in `StatusBar.svelte:38-48`.
- **Validate**: `bun run check`, `bun run test` (i18n tables match), plus a screenshot.

### Task 8: Example ids and explanations ✅
- **Done**: `Example { id, text }` via `ex(id, text)`; ids unique and kebab-case (test); `CheatExample.id`; 44 `cheat.<id>` explanations in en and id; i18n test fails on a missing one.
- **Action**:
  - In `cheatsheet.rs`, make each example `Example { id: "percent-of", text: "20% of 150" }`.
    Ids are kebab-case and unique; a Rust test checks that.
  - Pass `id` through `CheatExample`.
  - Add `cheat.<id>` strings to both i18n tables.
  - Extend `i18n.test.ts`, which already reads `cheatsheet.rs?raw`. It fails if an
    example id has no en or id explanation.
- **Mirror**: section ids and `cheatsheet.<section>` keys (M11).
- **Validate**: `cargo test -p notecal-core cheatsheet`, `bun run test`.

### Task 9: Search and shortcut rows ✅
- **Done**: `notes/cheatsearch.ts` + tests. Matching rule refined while testing: section titles and keywords match whole words only (`?pin` matched every shortcut through "Pintasan"), explanations by word start (`?pin` matched "typing"), the example or keys by any part (`?%`, `?0x`). `notes/shortcuts.ts` feeds a Keyboard shortcuts section; vim rows only with vim mode on; rows are not insertable. Not done: Settings labels don't read from the table (only the always-on-top label names a key).
- **Action**:
  - Add `cheatsearch.ts`. It normalises text with NFD, strips diacritics and
    lowercases. Every query word must be found in the joined haystack: title (current
    and en), explanation (current and en), example text and section keywords.
  - Add `shortcuts.ts`, a table of `{ keys, label: Key, vim?: true }`. The Switcher
    shows it as a "Keyboard shortcuts" section. Its rows are not insertable: Enter does
    nothing and the hint says so.
  - The Settings labels that mention shortcuts read the same keys.
- **Validate**: `cheatsearch.test.ts` cases:
  - `?discount` and `?diskon` → the percentages examples;
  - `?tz tokyo` → the dates example;
  - `?pin` → the shortcut row;
  - `?vim` → nothing when vim mode is off.
  - By hand: scrolling still follows the selection (M11 fix).

### Task 10: Docs ✅
- **Done**: README keys and a Vim mode paragraph, `docs/linux-setup.md` (settings, Hyprland pin), CHANGELOG under 1.0.0.
- **Action**: PRD E10 and milestone 12; README settings table and a "Vim mode" line;
  `docs/linux-setup.md` settings list; CHANGELOG under 1.0.0 (unreleased) or 1.1.0,
  as the author decides.
- **Validate**: `scripts/check.sh`.

## Validation
```bash
scripts/check.sh
cd ui && bun run tauri build --no-bundle   # then by hand on the 200-line note
```
By hand: the Esc rule, `:w`/`:q`, `yy` then `p` across notes, `p` of a result
copied by click, Ctrl+P/Ctrl+, from every mode, conflict (read-only) view, toggling
in Settings while in insert mode, Indonesian UI.

## Risks
| Risk | Likelihood | Mitigation |
|---|---|---|
| Esc meaning collides with "Esc hides the window" | High | the busy rule above; tested as a pure function |
| Vim swallows app shortcuts (Ctrl+P, Ctrl+,) | Medium | app shortcuts in the capture phase |
| Vim's fat cursor blinks and brings back idle CPU | Medium | no-blink option or CSS; re-measure idle CPU focused |
| Typing p95 rises (extra keymap on every key) | Low | measure with Ctrl+Shift+M; the library's handler is a map lookup |
| Library compatibility with our pinned `@codemirror/*` versions | Low | pin an exact version that peers with `@codemirror/view` 6.43; `bun run check` |
| Clipboard permission in WebKitGTK | Low | fallback to the internal register; Tauri clipboard plugin only if needed |
| Hyprland drops the pin on hide, or `pin` needs a floating window | Medium | re-apply on every show; document that the Omarchy float rule is required (it's in `docs/linux-setup.md`) |
| `hyprctl` dispatch syntax differs between Hyprland's Lua and classic configs | Medium | try the classic `dispatch pin address:…`, verify on this machine, keep the call in one function |
| New dependency (~100 KB min) in the bundle | Low | lazy `import()` only when `vim_mode` is on, so users without vim pay nothing |

## Acceptance
- [ ] `vim_mode` off: behaviour identical to today (all existing tests pass)
- [ ] `vim_mode` on: modes, motions, operators, counts, `.`, undo/redo, `/` search work
- [ ] Mode shown in the status bar; toggles live from Settings and `settings.toml`
- [ ] Esc: insert/visual → normal; normal → hide window
- [ ] `:w` saves; `:q`/`:x`/`:wq` save and hide
- [ ] App shortcuts work from every vim mode
- [ ] Pin button in the status bar toggles always-on-top and saves it; Ctrl+Shift+T still works
- [ ] On Hyprland the window stays pinned across hide/show; disabled with a reason on other Wayland compositors
- [ ] Cheatsheet examples show a translated explanation; search matches explanations, keywords and English terms in either language
- [ ] Keyboard shortcuts section in the cheatsheet, searchable, not insertable; vim rows only when vim mode is on
- [ ] Idle CPU focused and typing p95 not worse than Gate 2
- [ ] `scripts/check.sh` passes; docs updated

## Results (2026-11-20)

- `scripts/check.sh` passes: Rust (settings, pin, cheatsheet ids), 100 UI tests.
- Tried in the release build on Arch/Hyprland. Bugs found and fixed in the app: Esc after `d` hid the window; the pending count was missing (`d` for `2d`); Ctrl+P from vim's `/` prompt was swallowed, then left the focus in the prompt.
- **Unexplained:** during testing, the line `17 mod 5` (a cheatsheet example) appeared in the test note once, while no Enter had been sent. It happened while the focus bug above could leave the switcher open with the focus elsewhere. It wasn't reproduced after the fix. The line was removed.
- **Idle CPU, focused, vim on:** one 30 s sample gave 3.2%, and a sample with vim off gave a noisy 9.8%. Neither is a reliable reading, so re-measure at Gate 2 (`top`, 30 s, quiet machine).
- **Not checked by hand:** typing p95 with vim on, the Settings checkbox (only the file was used), the Indonesian UI with vim, and macOS/X11/Windows.
