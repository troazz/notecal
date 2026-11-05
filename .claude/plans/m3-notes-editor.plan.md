# Plan: M3 — Notes and editor

**Source PRD**: `docs/notecal.prd.md`
**Selected Milestone**: 3 — Notes and editor (~2 w)
**Complexity**: Large

## Summary

Turn the M2 spike into a real notepad on Linux: notes are `.txt` files in the notes directory, saved automatically and safely even when Syncthing or another editor changes them, with a Ctrl+P switcher to create, open, rename and delete notes. The editor gains highlighting, undo/search/multi-cursor, error hover, copy shortcuts and the PRD's result-copy rules. The engine's unit-division bug is fixed first, because notes will expose it.

## Requirements restated

| ID | Done means |
|---|---|
| N1 | Each note is `<name>.txt` in the notes directory (`~/.local/share/notecal/notes/` on Linux); the list ignores hidden and temp files |
| N2 | Edits save 500 ms after the last keystroke, through temp file + rename; no save button |
| N3 | Ctrl+P opens a switcher: type to filter, Enter opens; a name with no match creates it; rename and delete (with confirm) from the switcher |
| N4 (P1) | Launch reopens the last note at its cursor position (`state.json`) |
| N5 | The storage rules: hash check before save, reload when only the disk changed, conflict copy when both changed, keep the buffer when the file is deleted; changes detected with a file watcher, not polling |
| E1 | Done in M2 |
| E2 | Highlighting for numbers, units/identifiers, assigned variables, operators, keywords (`in`, `to`, `of`, `prev`, `sum`, `total`, `avg`) and comments/headers |
| E3 | Click a result copies its **displayed** text; Mod+Shift+C copies the current line's result |
| E4 | Undo/redo, multi-cursor and search from CodeMirror |
| E6 | Light and dark colours meet WCAG 2.2 AA for text and results (checked, recorded) |
| Themes (added 2026-11-05) | All colours, syntax included, come from one palette in Omarchy's `colors.toml` format. Sources: Omarchy's current theme (followed live), user files in `~/.config/notecal/themes/*.toml`, built-in light/dark. M3 uses "auto": Omarchy if present, else built-in by system mode. Picker UI is M4 (A6) |
| E8 (P1) | Alt-click copies the bare number (`copy` field) |
| E9 (P1) | A shortcut announces the current line's result through an ARIA live region |
| Errors rule | Lines with an error show a gutter marker; hovering it shows the engine's reason; the result column stays empty |

Deferred to later milestones: E5 autocomplete and E7 font settings (P1, fold into M4 with the settings screen), locale switching (A6, M4), macOS (Gate 2).

## Key facts (checked 2026-11-05)

- Versions: `notify` 8.2.1 (stable; 9.0 is still RC), `notify-debouncer-full` 0.7.0, `tempfile` 3.27.0 (dev only), `@codemirror/language` 6.13.1, `@codemirror/commands` 6.11.1, `@codemirror/search` 6.7.2, `@lezer/highlight` 1.2.5.
- Tauri's `app_data_dir()` is named after the bundle identifier (`com.troazz.notecal`), but the PRD says `~/.local/share/notecal/`. Use `data_dir()` + `notecal` so the path matches the PRD and is easy to point Syncthing at.
- M2 currently copies the bare number on click. The PRD splits this: click copies the displayed text (E3), Alt-click copies the bare number (E8). Task 6 corrects it.
- `LineDto.error` already reaches the UI; nothing shows it yet.
- Known engine bug (M1 plan, "Known issues"): `8 km / 2 km` gives `4 km^2`.

## Design

**Storage (`src-tauri/src/notes.rs`, pure Rust, no Tauri types)**

```rust
pub struct NoteStore { dir: PathBuf, known: HashMap<String, Known> }
enum Known { Content(String), Missing }   // what we last read or wrote

pub fn list(&self) -> io::Result<Vec<NoteInfo>>          // name, modified
pub fn open(&mut self, name) -> io::Result<String>
pub fn save(&mut self, name, text) -> io::Result<SaveOutcome>
pub fn create / rename / delete
pub fn disk_changed(&mut self, name) -> io::Result<DiskChange>  // for watcher events

enum SaveOutcome { Saved, Conflict { copy_name: String, disk_text: String } }
enum DiskChange  { Unchanged, Changed(String), Deleted }
```

- "Content hash" from the PRD is implemented as a comparison with the last known content. Notes are small text files, so keeping the text is cheaper and simpler than adding a hash crate, and it is exact.
- `save`: read disk. Same as known, or file missing → atomic write, remember text, `Saved`. Different → write local text to `<name> (conflict YYYY-MM-DD HHMM).txt` (atomically, with a numeric suffix if that exists), remember the disk text, return `Conflict`.
- Atomic write: `.notecal-tmp-<random>` in the same directory, write, `sync_all`, `rename`. Temp files are excluded from `list`.
- Names are validated: non-empty, no `/`, `\0`, or leading `.`, at most 120 bytes; `create`/`rename` refuse an existing name. Errors are full sentences for the UI.
- Tested with `tempfile::TempDir`, including every storage-rule branch.

**Tauri layer (`src-tauri/src/main.rs`, new `commands.rs`)**

- `Mutex<NoteStore>` in state. Commands: `list_notes`, `open_note`, `save_note`, `create_note`, `rename_note`, `delete_note`, `load_state`, `save_state`.
- Watcher: `notify-debouncer-full` on the notes directory (non-recursive, 100 ms debounce). For the open note, call `disk_changed`; on `Changed`/`Deleted` emit `note-changed-on-disk`. Our own writes compare equal to `known`, so they never echo. Any directory change emits `notes-list-changed` for the switcher.
- `state.json` in the config directory: `{ last_note, cursor }`, written atomically with the same helper.

**UI**

- `ui/src/notes/session.ts` (framework-free, Vitest-tested with a fake invoke and fake timers): current note, dirty flag, 500 ms debounced save, flush on note switch and on window blur/close. On `note-changed-on-disk`: if not dirty → reload the buffer keeping the cursor where possible; if dirty → save now (Rust turns it into a conflict copy) then load the disk text and show a notice naming the conflict file.
- `ui/src/notes/Switcher.svelte`: Ctrl+P overlay. Filter as you type, arrow keys + Enter to open, "Create ‘name’" row when nothing matches, F2 to rename and Delete to delete (with confirm), Esc to close.
- `ui/src/editor/language.ts`: `StreamLanguage` tokenizer for E2, plus a `HighlightStyle` using the theme CSS variables.
- `ui/src/editor/errors.ts`: gutter marker for lines whose `error` is set, `hoverTooltip` on the marker showing the reason.
- `ui/src/editor/results.ts`: click copies `display`; Alt-click copies `copy`; Mod+Shift+C copies the current line's `display`; a visually hidden `aria-live="polite"` region and Mod+Shift+R announce the current line's result (E9).
- Replace `minimalSetup` with an explicit list: `history`, `drawSelection`, `allowMultipleSelections`, `rectangularSelection`, `highlightSelectionMatches`, `search` keymap, default and history keymaps.
- The first launch with an empty notes directory creates `Scratch.txt` with the PRD example note. The Gate 1 fixture stays in the repo for re-measurement and stops being loaded at start.

## Patterns to Mirror

| Category | Source | Pattern |
|---|---|---|
| Rust state + tests | `src-tauri/src/evaluator.rs:61`, `:111` | Plain struct owned by Tauri state, logic testable without a Tauri runtime, unit tests in the same file |
| Error recovery | `src-tauri/src/evaluator.rs:101` (`lock_engine`) | A poisoned lock recovers instead of wedging the app |
| User-facing errors | `crates/notecal-core/src/preprocess.rs:122` (`check_no_assignment`) | `Result<_, String>` with a full-sentence message |
| UI logic | `ui/src/editor/scheduler.ts:20`, `scheduler.test.ts:38` | Framework-free class, injected `invoke` and timers, Vitest with fakes |
| Editor extensions | `ui/src/editor/results.ts` | One module per concern exporting an `Extension`; state through `StateField` + `StateEffect` |
| Golden tests | `crates/notecal-core/tests/golden/*.txt` | `input => expected` cases per language rule |

## Files to Change

| File | Action | Why |
|---|---|---|
| `crates/notecal-core/src/preprocess.rs`, `tests/golden/units.txt` | UPDATE | Unit-division fix and its golden cases |
| `.claude/plans/m1-engine-core.plan.md` | UPDATE | Mark the known issue fixed |
| `src-tauri/Cargo.toml` | UPDATE | `notify` 8.2.1, `notify-debouncer-full` 0.7.0, `toml` 1.2.0 (also needed by M4 settings); `tempfile` dev-dep |
| `src-tauri/src/notes.rs` | CREATE | `NoteStore`, atomic write, storage rules, tests |
| `src-tauri/src/state.rs` | CREATE | `state.json` load/save |
| `src-tauri/src/commands.rs` | CREATE | Note and state commands |
| `src-tauri/src/watch.rs` | CREATE | Debounced watcher → events |
| `src-tauri/src/theme.rs` | CREATE | Parse `colors.toml`-format palettes, resolve the auto source, watch Omarchy and user theme dirs |
| `ui/src/theme.ts` | CREATE | Apply a palette as CSS variables; built-in light/dark palettes |
| `src-tauri/src/main.rs` | UPDATE | Register state, commands, watcher |
| `ui/package.json`, `bun.lock` | UPDATE | CodeMirror language/commands/search, lezer highlight |
| `ui/src/notes/session.ts`, `session.test.ts` | CREATE | Autosave and external-change handling |
| `ui/src/notes/Switcher.svelte` | CREATE | N3 |
| `ui/src/editor/language.ts`, `language.test.ts` | CREATE | E2 tokenizer |
| `ui/src/editor/errors.ts` | CREATE | Error gutter + hover |
| `ui/src/editor/results.ts` | UPDATE | E3/E8/E9 copy rules and announcer |
| `ui/src/Editor.svelte`, `ui/src/App.svelte` | UPDATE | Wire notes, switcher, extensions; stop loading the fixture |
| `docs/notecal.prd.md` | UPDATE | M3 row → in-progress (done by `/plan`) |

## Tasks

### Task 1: Fix unit division in the engine ✅
- **Action**: In the preprocessor, wrap a `number unit` operand that follows `/` in parentheses, so `8 km / 2 km` reads as `8 km / (2 km)`. Golden cases: `8 km / 2 km => 4`, `1 GiB / 4 KiB => 262,144`, `100 km / 2 h => 50 km / h`. A bare number divisor (`10 km / 2`) is unchanged.
- **Mirror**: `rewrite_percentages` token handling in `preprocess.rs`; golden file format.
- **Validate**: `cargo test -p notecal-core`; benches still within budget (`cargo bench -p notecal-core -- --quick`).
- **Done**: grouping applies only when the dividend ends in a word (unit or variable), so `10 / 2 m` stays `5 m`; chains like `a / 2 b / 3 c` group each divisor. 218 golden cases pass. `note_200_uncached` unchanged (alternating runs: 14.6–14.8 ms before and after).

### Task 2: Note storage ✅
- **Action**: `notes.rs` with `NoteStore`, name validation, atomic write, `list/open/save/create/rename/delete/disk_changed`.
- **Mirror**: `evaluator.rs` structure and tests.
- **Validate**: Tests for every storage rule: plain save; external change + no local edit → `Changed`; external change + local edit → conflict file with the local text and disk text kept; deleted file → buffer saved again on next save; temp files never listed; rename to existing name refused; invalid names refused.

### Task 3: Commands, state file, watcher ✅
- **Action**: `commands.rs`, `state.rs`, `watch.rs`; notes dir `data_dir()/notecal/notes`, created on start; `Scratch.txt` seeded when empty.
- **Note**: the conflict-file timestamp (`YYYY-MM-DD HHMM`, local time) is passed in by the UI with each save, so Rust needs no date/time-zone dependency.
- **Validate**: Unit tests for `state.rs`. Manual: `echo 1+1 >> ~/.local/share/notecal/notes/Scratch.txt` while the app is open reloads the buffer; idle CPU still ~1% (`top`).

### Task 4: Autosave session in the UI ✅
- **Action**: `session.ts` + wiring in `Editor.svelte`/`App.svelte`; save on 500 ms debounce, on switch, on blur and before close.
- **Validate**: Vitest: rapid edits make one save after 500 ms; switching notes flushes first; an external change while clean reloads; while dirty, saves then shows the conflict notice.

### Task 5: Quick switcher ✅
- **Action**: `Switcher.svelte` with filter, create, rename, delete and keyboard navigation; reopen last note and cursor (N4).
- **Validate**: `bun run check`; manual: create, rename, delete, reopen after restart.
- **Done** (checked in the release build on Hyprland): autosave, external `echo >>` reloads the buffer, typing plus an external write within 500 ms produces `Scratch (conflict 2026-11-05 1841).txt` and a notice, Ctrl+P create/rename (F2)/delete (Ctrl+Delete, confirmed), close saves edits and cursor, relaunch reopens the last note. The window title shows the note name (needs `core:window:allow-set-title`; close needs `core:window:allow-destroy`).

### Task 6: Editor features ✅
- **Action**: Explicit extension list (E4), `language.ts` (E2), `errors.ts` (error gutter + hover), copy rules (E3 click = display, E8 Alt-click = bare, Mod+Shift+C), announcer (E9).
- **Validate**: Vitest for the tokenizer (numbers, keywords, comments, assignment names). Manual: highlight, search (Ctrl+F), multi-cursor (Ctrl+click), error hover on `m = 5`, each copy path.
- **Done**: shortcuts are Mod+Shift+C (copy result as shown), Mod+Shift+B (bare number, E8), Mod+Shift+A (announce, E9); click copies as shown, Alt-click the bare number. Error markers are hidden on the cursor's line so half-typed input does not flicker; the reason shows as the marker's tooltip. Checked in the release build: highlighting, error marker, Ctrl+F, both copy shortcuts.

### Task 7: Themes (Omarchy and user palettes) ✅ (Omarchy + built-in; user files move to M4)
- **Action**: `theme.rs`: parse a palette (`mode`, `background`, `foreground`, `accent`, `selection`, `muted`, `red`, `yellow`, `green`, `cyan`, `blue`, `magenta`; unknown keys ignored, missing keys fall back to the built-in palette of the same `mode`, invalid colours rejected). Auto source: `~/.local/state/omarchy/current/theme/colors.toml` if it exists, else built-in. Watch the Omarchy `current` directory and `~/.config/notecal/themes/` with the existing debouncer; emit `theme-changed` with the palette. Command `get_theme`. UI `theme.ts` sets CSS variables on `:root`; the syntax `HighlightStyle` and results use only those variables. Role map: number → yellow, unit → cyan, variable → foreground, keyword → magenta, operator → blue, comment/header → muted, result → muted, error → red, selection → selection.
- **Mirror**: `notes.rs` validation and tests; watcher from Task 3.
- **Validate**: Rust tests: Omarchy `colors.toml` sample parses; missing keys fall back; bad hex rejected; auto picks Omarchy when present. Manual: `omarchy theme set <other>` repaints notecal without restart; idle CPU unchanged.

- **Done**: `theme.rs` parses palettes (invalid colours dropped so they fall back; mode inferred from the background if missing) and follows Omarchy by watching `~/.local/state/omarchy/current` for the `theme` folder swap and `theme.name` write that `omarchy theme set` performs. `ui/src/theme.ts` applies the built-in palette before mount (no flash), then the followed theme; the system light/dark switch still applies when no theme is followed. Checked: the app starts in ethereal; a simulated swap to catppuccin-latte (isolated HOME) repaints live; idle CPU unchanged (~1%).
- **Moved to M4**: user theme files in `~/.config/notecal/themes/` need the theme picker (A6) to be selectable, so they ship with it. The parser they will use is the one here.

### Task 8: Contrast check and wrap-up ✅

**Performance follow-up (2026-11-05).** The user's hand-typed p95 rose from 45 ms (M2) to 69 ms after Tasks 4–7. Findings: the engine is not the cause (replaying the typed keystrokes: p50 4.5 ms, max 11 ms per step); CodeMirror's `StreamLanguage` re-tokenised every following line per keystroke, and the error gutter added layout work per update. Fixes: a visible-lines highlighter that re-tokenises only edited lines; error markers drawn as line attributes (CSS `::before`), hidden on the cursor line via `highlightActiveLine`; disk-touching commands made `async` so `fsync` never runs on the main thread. Scripted typing (wtype, 90 ms/key, noisy ±40 ms): M2 63–77 ms, M3 before fixes 95–102 ms, after 89 ms. **Decision (user): accept the current latency for now; revisit before Gate 2 (M5).**

**Reviews (2026-11-05).** Rust review: fixed a case-only rename that could replace a different note on Linux, conflict copies for long names, sanitising the UI-supplied stamp, and theme state managed before its watcher starts. UI review: fixed undo carrying another note's text across a switch (editor state now replaced, not edited), lost typing with no note open (Scratch is recreated), reloads racing an in-flight save (queued), keystrokes typed during a conflicting save (written to the conflict copy), one failed save blocking all later ones, unhandled errors on note switch, rename/delete during a pending save, clipboard errors, and switcher focus/ARIA. Not changed: blocking disk work on async workers (small files, acceptable), parent-directory fsync, and the check-then-rename window on create/save (tiny, inherent). Built-in contrast is covered by `ui/src/theme.test.ts` (every text colour ≥ 4.5:1 on background and panel).
- **Action**: Measure text, result, comment and highlight colours against both backgrounds; adjust any below 4.5:1 (3:1 for the gutter marker). Record ratios in the plan. Run `/ecc:rust-review` and `/ecc:code-review`.
- **Validate**: Every colour pair ≥ 4.5:1; keystroke p95 on the 200-line fixture still ≤ 50 ms with highlighting on.

## Validation

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cd ui && bun run check && bun run test && bun run build
cd ui && bun run tauri build --no-bundle   # manual checks on target/release/notecal
```

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Watcher sees our own atomic rename and reloads the buffer mid-typing | Medium | Compare disk text with `known` before emitting; our writes match and are ignored |
| Syncthing writes its own temp files (`.syncthing.*.tmp`) into the notes dir | Medium | `list` ignores hidden files; the watcher only acts on `<name>.txt` |
| Reloading the buffer moves the cursor | Medium | Keep the cursor offset clamped to the new length; full reload only when the text differs |
| Highlighting pushes keystroke p95 over 50 ms | Low | `StreamLanguage` is incremental; re-measure in Task 8 |
| Omarchy changes its theme file layout | Low | Path and parser in one module; missing or unreadable file falls back to built-in |
| An Omarchy or user theme has poor contrast | Medium | Only built-in palettes are held to WCAG; user-chosen themes are applied as given |
| Unit-division rewrite changes other results | Medium | Full golden suite plus proptest equivalence run on every change |
| Data loss on crash between debounce and save | Low | 500 ms window by design (PRD N2); flush on blur and close |

## Acceptance

- [x] All tasks complete
- [x] Validation passes locally and in CI
- [x] Every storage rule has a passing test
- [x] Editing a note while Syncthing (or `echo >>`) changes it never loses text (manual `echo >>` and conflict checks, plus storage and session tests)
- [x] Contrast ratios recorded and passing (built-in palettes)
- [x] notecal follows `omarchy theme set` live
- [x] Patterns mirrored, not reinvented
