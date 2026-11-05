# Plan: M4 — App shell and currency

**Source PRD**: `docs/notecal.prd.md`
**Selected Milestone**: 4 — App shell and currency (~1.5 w)
**Complexity**: Large

## Summary

Make notecal a background utility on Linux: one instance, shown and hidden with `notecal --toggle` from a Hyprland keybinding (the global-shortcut plugin cannot work on Wayland), hidden with Esc, remembered window geometry, optional always-on-top and tray. Add a settings file and screen (theme, font, precision, locale, notes folder, currency refresh) and live currency conversion from cached open.er-api.com rates that work offline. Also lands the items moved here from M3: the theme picker with user theme files, autocomplete, and font settings.

## Requirements restated

| ID | Done means |
|---|---|
| A2 (P0) | A second `notecal --toggle` shows the running window focused (or hides it if focused) and exits; a plain second `notecal` focuses the first. Hyprland binding documented |
| A1 (P0) | Configurable global shortcut (default Alt+Space) on macOS and X11; skipped on Wayland, where Settings points to the `--toggle` binding instead |
| A3 (P1) | Tray icon with Show/Hide and Quit; closing the window hides it while "Keep running in the background" is on |
| A4 (P1) | Always-on-top setting, plus a shortcut to toggle it |
| A5 (P1) | Window size and position restored on launch |
| A6 (P1) | `~/.config/notecal/settings.toml`, editable by hand (picked up live) or from a Settings screen (Ctrl+,): shortcut, theme, font family and size, precision, locale, notes folder, currency refresh, always-on-top, background mode |
| C9 (P1) | Currency conversion with rates cached in `~/.cache/notecal/rates.json`; refreshed at most every 24 h and only when a note uses a currency; works offline from the cache; "No exchange rates yet" on hover when there is no cache; status line shows the rates date; attribution in Settings |
| Scenario 1 | Hotkey → type → copy → Esc hides; round trip under 3 s; show ≤ 100 ms (PRD budget) |
| From M3 | Theme picker: auto (Omarchy, else system), light, dark, or a user theme file from `~/.config/notecal/themes/*.toml` (Omarchy `colors.toml` format). E5 autocomplete for variables, units and currencies. E7 font family/size |

Out of scope: A7 CLI evaluation (P2), C11 dates (P2), packaging (M5), macOS checks (M5).

## Key facts (checked 2026-11-05)

- Versions: `tauri-plugin-single-instance` 2.5.5, `tauri-plugin-global-shortcut` 2.4.1, `tauri-plugin-window-state` 2.5.0 (JS packages at the same versions), `ureq` 3.5.1 (blocking HTTP, rustls), `@codemirror/autocomplete` 6.20.4. `toml` 1.2.0 is already a dependency.
- open.er-api.com works today: 166 currencies including IDR, `time_last_update_utc` per response.
- `notecal-core` already takes `Option<&Rates>`; `Rates::from_per_usd` drops bad values. The Tauri evaluator currently passes `None` and takes the locale from each request.
- Session is Wayland (Hyprland, Lua config in `~/.config/hypr/bindings.lua`). `libayatana-appindicator` 0.6.1 is installed, so a tray icon can be created; whether a bar shows it depends on the Omarchy bar.
- Omarchy's monospace font is fontconfig's `monospace` (`omarchy-font-current` runs `fc-match monospace`), so a font setting of "auto" = CSS `monospace` follows Omarchy with no extra code.

## Design

**Settings (`src-tauri/src/settings.rs`)**

```toml
# ~/.config/notecal/settings.toml (every key optional)
shortcut = "Alt+Space"        # A1; ignored on Wayland
theme = "auto"                # auto | light | dark | <file name in themes/>
font_family = "auto"          # auto = system monospace (follows Omarchy)
font_size = 15
precision = 6                 # 0–20
locale = "en"                 # en | id
notes_dir = ""                # empty = ~/.local/share/notecal/notes
currency_refresh = true       # false = never fetch, cached rates only
always_on_top = false
background = true             # close hides to tray; Quit from tray
```

- Parsed with defaults for missing keys; invalid values are reported once and replaced by the default, never fatal. Saved atomically (`notes::write_atomic`). Saving from the Settings screen rewrites the file from the struct, so hand-written comments are dropped; the file starts with a header saying so.
- Watched (debouncer from `watch.rs`); a change emits `settings-changed`. The evaluator reads locale and precision from settings, so `EvalRequest` drops `locale`.
- Changing `notes_dir` re-opens the `NoteStore` and its watch on the new folder (existing notes are not moved; the switcher shows the new folder's notes).

**Window behaviour (`src-tauri/src/shell.rs`)**

- `single-instance` plugin: the callback gets the second process's args; `--toggle` → hide if visible and focused, else show + unminimize + focus. Registered first, as the plugin requires.
- Esc in the editor (when no panel is open) hides the window.
- `global-shortcut` registered from settings unless `WAYLAND_DISPLAY` is set; failures are reported in Settings, not fatal.
- `window-state` plugin restores size/position; always-on-top applied from settings and toggled by Mod+Shift+T.
- Tray (`tauri` `tray-icon` feature): Show/Hide, Always on top, Settings, Quit. Window close → hide when `background = true`, else quit.

**Currency (`src-tauri/src/rates.rs`)**

- Cache file `{ fetched_at, source_date, rates: {code: per_usd} }`, loaded at start.
- After each evaluation, if the text uses a currency code (3 uppercase letters that is a known code from the cache, or a fixed list of ISO codes when there is no cache) and the cache is missing or older than 24 h and `currency_refresh` is on, fetch once in a background thread (`ureq`, 10 s timeout). On success: save the cache atomically, swap the evaluator's rates, emit `rates-updated` (the UI re-evaluates). On failure: keep the old cache, retry no sooner than 1 h.
- Engine: when rates are `None` and a line names a currency, the error is "No exchange rates yet" (golden cases).
- Status line (bottom-right, muted), shown only when the note uses a currency: "Rates: 5 Nov 2026", or "Rates: none yet" before the first fetch.

**UI**

- `ui/src/settings/Settings.svelte` (Ctrl+,): form bound to the settings object, saved on change through `save_settings`; shows the Wayland binding hint with a copy button; credit line "Rates by Exchange Rate API (open.er-api.com)".
- `theme.ts`: chosen theme from settings; `auto` keeps the M3 behaviour; user theme files come from Rust (`list_themes`, parsed by the existing `theme::parse`).
- Font: CSS variables `--editor-font` / `--editor-font-size` from settings.
- `ui/src/editor/complete.ts`: `@codemirror/autocomplete` source with the note's variables (from `firstAssignments`), common units, and currency codes from the rates cache; activates on 2+ letters, never on numbers.

## Patterns to Mirror

| Category | Source | Pattern |
|---|---|---|
| Rust module + tests | `src-tauri/src/notes.rs`, `theme.rs` | Plain Rust module, no Tauri types, `TempDir` tests in the same file; Tauri glue in `commands.rs` |
| File I/O | `src-tauri/src/notes.rs` (`write_atomic`) | Atomic writes for every file the app owns |
| Watching | `src-tauri/src/watch.rs` (`watch_dir`), `commands.rs` (`ThemeSource::on_paths_changed`) | Debounced watch → state method → emit event |
| Errors | `notes.rs` (`Result<_, String>` full sentences) | User-facing sentences; non-fatal problems logged with `eprintln!("notecal: …")` |
| UI logic | `ui/src/notes/session.ts` + tests | Framework-free class with injected deps, Vitest fakes |
| UI events | `ui/src/notes/api.ts` | Typed wrappers for commands and events |

## Files to Change

| File | Action | Why |
|---|---|---|
| `src-tauri/Cargo.toml`, `ui/package.json` | UPDATE | Plugins, `ureq`, `tray-icon` feature, `@codemirror/autocomplete` |
| `src-tauri/src/settings.rs` | CREATE | Settings model, parse/save, defaults, tests |
| `src-tauri/src/shell.rs` | CREATE | Toggle, shortcut, tray, always-on-top, close behaviour |
| `src-tauri/src/rates.rs` | CREATE | Cache, refresh policy, fetch, tests (fetch behind a trait so tests never hit the network) |
| `src-tauri/src/evaluator.rs` | UPDATE | Settings and rates from state; request drops `locale` |
| `src-tauri/src/commands.rs`, `main.rs`, `theme.rs` | UPDATE | Settings/rates/theme commands, plugin setup, user themes, notes-dir switch |
| `src-tauri/capabilities/default.json` | UPDATE | Permissions for window show/hide/always-on-top used from JS |
| `crates/notecal-core/src/lib.rs`, `tests/golden/currency.txt` | UPDATE/CREATE | "No exchange rates yet" rule and golden cases |
| `ui/src/settings/Settings.svelte`, `ui/src/settings/api.ts` | CREATE | A6 screen |
| `ui/src/editor/complete.ts` + test | CREATE | E5 |
| `ui/src/theme.ts`, `App.svelte`, `Editor.svelte`, `editor/scheduler.ts` | UPDATE | Theme choice, fonts, status line, Esc hide, request without locale |
| `docs/linux-setup.md` | CREATE | Hyprland / Sway / GNOME `notecal --toggle` bindings |

## Tasks

### Task 1: Settings model and file ✅
- **Action**: `settings.rs` (defaults, parse with per-key fallback, save atomically, validation of ranges and enums), commands `get_settings` / `save_settings`, watch → `settings-changed`. Evaluator takes locale and precision from settings; scheduler stops sending locale.
- **Validate**: Rust tests: missing file → defaults; partial file; invalid values fall back with a message; round trip. Manual: editing `settings.toml` by hand changes precision live.
- **Done**: checked in the release build — writing `precision = 2` by hand changed `260.975901 miles` to `260.98 miles` without a restart. Problems found in the file are shown in the notice. `notes_dir` is read at startup; switching it live is Task 6.

### Task 2: Single instance and `--toggle` (A2) ✅
- **Action**: single-instance plugin, toggle logic, Esc hides, `docs/linux-setup.md` with the Hyprland Lua binding (`notecal --toggle` on Alt+Space) plus Sway and GNOME.
- **Validate**: Manual on Hyprland: `notecal --toggle` from a terminal shows the window focused and typing goes to it; again hides it; show time measured from the command to the first frame (target ≤ 100 ms).
- **Done**: on Hyprland, toggle hides the focused window and shows it focused (no focus-stealing rule needed); one instance stays running. Command → focused window: 87–111 ms, including starting the second process and `hyprctl` polling. Esc hides (after saving); Esc in the search panel only closes the panel.

### Task 3: Window state, always-on-top, global shortcut (A5, A4, A1)
- **Action**: window-state plugin; always-on-top setting + Mod+Shift+T; global shortcut registration (skipped on Wayland, with the reason shown in Settings).
- **Validate**: Manual: size/position restored after restart; always-on-top toggles; on Wayland the shortcut is not registered and Settings explains why.

### Task 4: Tray and background mode (A3)
- **Action**: tray icon and menu; close → hide when `background` is on.
- **Validate**: Manual: tray menu works where the bar shows tray icons; with no tray host, closing still behaves (if `background` is on and no tray is visible, `--toggle` still shows the window). Idle CPU unchanged.

### Task 5: Currency rates (C9)
- **Action**: `rates.rs` with an injectable fetcher; cache load/save; refresh policy; evaluator wiring; `rates-updated` event; engine "No exchange rates yet"; status line; attribution.
- **Validate**: Rust tests with a fake fetcher: no cache + currency → one fetch; fresh cache → no fetch; failure keeps the old cache and backs off; no currency → no fetch; corrupt cache ignored. Golden cases for the no-rates message. Manual: `100 USD in IDR` works online, then offline after a restart.

### Task 6: Settings screen and fonts (A6, E7)
- **Action**: `Settings.svelte` (Ctrl+,), font variables, notes-folder switch, Wayland hint, credits.
- **Validate**: `bun run check`; manual: every control writes `settings.toml` and takes effect without restart.

### Task 7: Theme picker and user themes (from M3)
- **Action**: `theme` setting; `list_themes` reads `~/.config/notecal/themes/*.toml` with the existing parser; watch that folder; picker in Settings.
- **Validate**: Rust tests for listing and loading; manual: copy an Omarchy `colors.toml` into the folder, pick it, edit it, see it repaint.

### Task 8: Autocomplete (E5)
- **Action**: `complete.ts` source: variables, common units, currency codes.
- **Validate**: Vitest on the source; manual: typing `elec` offers `electricity`; numbers never trigger it. Typing p95 checked by hand once (no scripted runs).

### Task 9: Wrap-up
- **Action**: `/ecc:rust-review` and `/ecc:code-review`, fix real findings, update PRD/plan.
- **Validate**: full validation below; CI green.

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
| Hyprland does not give focus to the shown window (focus stealing prevention) | Medium | Test in Task 2; `set_focus` after `show`; if denied, document a Hyprland window rule (`focusonactivate`) in `linux-setup.md` |
| No tray host in the Omarchy bar | Medium | Tray is P1; `--toggle` stays the primary path; closing never strands the app without a way back |
| Network fetch adds size/TLS dependencies | Low | `ureq` with rustls only; fetch code isolated in `rates.rs` |
| Settings file edited into an invalid state | Medium | Per-key fallback with a message; never crash or overwrite the user's file on read |
| Notes-folder switch while edits are pending | Medium | Flush the session before switching; same path as note switch |
| Autocomplete adds typing latency | Low | Explicit-only for numbers, 2-letter trigger; hand-checked p95 |

## Acceptance

- [ ] All tasks complete
- [ ] Validation passes locally and in CI
- [ ] `notecal --toggle` shows/hides a focused window on Hyprland
- [ ] Currency conversion works online and offline from the cache
- [ ] Every setting works from the file and the screen
- [ ] Patterns mirrored, not reinvented
