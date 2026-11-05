# PRD: notecal — a Numi-style calculator notepad (Tauri + Svelte + Rust)

Nov 5, 2026 · @Puas Citro · Revision 2 (see [Changes from revision 1](#changes-from-revision-1))

## Overview

notecal is a lightweight calculator notepad for macOS and Arch Linux. Each line holds natural math, and its result appears right-aligned on the same line as you type. It is inspired by Numi, which is macOS-only, and runs natively on both of the author's daily systems.

The stack is Tauri 2 for the shell, Svelte 5 for the UI, CodeMirror 6 for the editor, and a Rust engine (`notecal-core`) built on fend-core for evaluation. notecal is open source.

**Problem.** Quick everyday math with units, currencies, percentages and running totals is awkward in a regular calculator. Spreadsheets are too heavy for it. Numi solves this well but does not run on Linux. Numara is the main Electron alternative, and it ships 100 MB+ with a full Chromium runtime.

## Goals and non-goals

The v1.0 goal is a scratchpad that opens instantly and feels native. It should replace a calculator app for daily dev and life math on both machines.

**Goals**

- Instant results: results update as you type, with no Enter key and no submit step.
- Light and fast: a small package, low idle RAM, and near-instant cold start (see Non-functional requirements).
- Numi-like expressiveness: units, currency, percentages, variables, references to previous lines, and totals.
- Keyboard-first: a hotkey to summon or hide the app, and every action reachable without a mouse.
- Plain-text notes: notes stay readable and editable outside the app, including in synced folders.

**Non-goals (v1.0)**

- iOS, Android, or Windows builds.
- Cloud sync or accounts. Users can sync the notes folder themselves, for example with git or Syncthing.
- Plotting, graphing, matrices, or symbolic algebra.
- A plugin or extension system.
- Collaboration or sharing features.
- Variables shared across notes. Variables are scoped to one note.

## Target users and platforms

The primary user is a developer who works on both macOS and Arch Linux (Hyprland, Wayland) and wants the same scratchpad on each machine. The secondary audience is the open-source Linux community, which has no polished Numi equivalent.

| Platform | Target | Webview | Distribution |
| --- | --- | --- | --- |
| macOS | 13 Ventura+, Apple Silicon and Intel | WKWebView | Unsigned `.dmg` |
| Arch Linux | Rolling, Wayland and X11 | WebKitGTK 4.1 (system package) | AUR package (primary), AppImage |

Other Linux distros are best-effort through the AppImage. Windows is out of scope.

The macOS build ships unsigned for v1.0. Users open it once via right-click → Open (or `xattr -d com.apple.quarantine`). A Homebrew cask likely needs a signed, notarized build, so it is deferred until signing is revisited.

## Core user experience

The whole UI is one text editor. You write expressions on the left, and each line's result appears right-aligned in a muted color. Clicking a result copies it.

Example note (`→ …` marks what the result column shows; it is not typed):

```
# Monthly budget
rent = 4,500,000 IDR
electricity = 350,000 IDR
internet = 450,000 IDR
total                          → 5,300,000 IDR
20% of prev                    → 1,060,000 IDR

# Conversions
450 USD in IDR
5 km in miles
3 days 4 hours in minutes
0xff in binary
```

**Key scenarios**

1. Quick calculation: press the hotkey, type, read the result, copy it, and press Esc to hide the window. The whole round trip takes under 3 s.
2. Budget note: build up variables, reuse them across lines, and finish with a total. The note is saved automatically.
3. Unit and currency conversion: type something like `1.5 GB in MB` or `100 USD in IDR` and get an immediate answer, even offline with cached rates.
4. Dev math: convert between hex, binary, and decimal, and work out bytes, durations, and percentages for capacity planning.
5. Switching notes: open a quick switcher (Cmd/Ctrl+P) to jump between named notes.

## Functional requirements

Priority key: **P0** must be in v1.0, **P1** should be in v1.0, and **P2** comes after v1.0.

### Calculation language (evaluated by `notecal-core`)

| ID | Requirement | Priority |
| --- | --- | --- |
| C1 | Arithmetic: `+ - * / ^ mod`, parentheses, and implicit multiplication (`2(3+4)`) | P0 |
| C2 | Arbitrary-precision decimals, so `0.1 + 0.2` gives `0.3` | P0 |
| C3 | Units and conversion with `in`/`to`: length, mass, volume, time, temperature, data size, speed, area | P0 |
| C4 | Percentages with Numi semantics (see [Language rules](#language-rules)) | P0 |
| C5 | Variables: `name = expr`, usable on any later line of the same note | P0 |
| C6 | Line references: `prev`, `sum`/`total`, and `avg` (see [Language rules](#language-rules)) | P0 |
| C7 | Comments (`#`, `//`) and headers; a line that does not evaluate shows no result, never an error inline | P0 |
| C8 | Number formats: hex, binary, octal, scientific, with conversion through `in hex` / `in binary` / `in octal` | P1 |
| C9 | Currency conversion with daily cached rates from open.er-api.com (must include IDR); works offline using the last fetched rates | P1 |
| C10 | Math functions: `sqrt`, `round`, `floor`, `ceil`, `abs`, `log`, `ln`, trig, and constants `pi`, `e` | P0 |
| C11 | Dates and times: `today + 3 weeks`, `now in Asia/Jakarta`, time zone conversion | P2 |
| C12 | Thousands separators in input and output, following the locale setting (see [Language rules](#language-rules)) | P0 |
| C13 | Evaluation of one line is time-limited; a line that exceeds the limit shows no result and never blocks the editor | P0 |

### Language rules

These rules are normative. Each one gets golden test cases.

**Percentages (C4)** follow Numi, not fend's plain `% = /100` reading:

| Input | Result |
| --- | --- |
| `20% of 50` | `10` |
| `50 + 10%` | `55` |
| `50 - 10%` | `45` |
| `5 as % of 20` | `25%` |
| `10%` alone | `10%` |

**Line references (C6)**

- A *block* is the run of lines above the current line, up to the nearest blank line or header line (a line starting with `#`).
- `sum` and `total` add every result in the current block, including assignment lines. Lines without a result (comments, unparseable lines) are skipped. Lines that themselves contain `sum`, `total` or `avg` are not counted.
- `avg` is `sum` divided by the number of counted lines.
- All counted values must share a dimension. The result uses the unit of the first counted line; currencies convert to the first line's currency. If dimensions are incompatible (for example IDR and km), the line shows no result.
- `prev` is the result of the nearest line above that has a result, within the same note.

**Variables (C5)**

- Names are scoped to one note and become visible from the line after the assignment.
- Reassigning a name later in the note is allowed; later lines see the newest value.
- A name that collides with a built-in unit, function, constant or keyword (for example `m`, `e`, `in`, `sum`) cannot be assigned. The line shows no result, and the hover message explains why.

**Number input and display (C12, A6)**

- The locale setting decides separators. `en` (default): `,` groups thousands, `.` is decimal. `id`: `.` groups thousands, `,` is decimal.
- In `en`, a comma between digits followed by exactly three digits is a thousands separator: `4,500,000` is 4500000. Any other comma is an argument separator.
- Currencies display with 2 decimals. Other values display up to the precision setting (default 6) with trailing zeros trimmed. Exact repeating fractions are rounded to that precision.
- Results display with thousands separators in the current locale.

**Errors (C7)**

- The result column never shows error text. A line that fails shows nothing.
- The engine still returns the error reason, and the editor shows it on hover over the line's gutter marker.

### Editor

| ID | Requirement | Priority |
| --- | --- | --- |
| E1 | Results right-aligned per line and updated as you type | P0 |
| E2 | Syntax highlighting for numbers, units, variables, operators, and comments | P0 |
| E3 | Click a result to copy its displayed text; a shortcut copies the current line's result | P0 |
| E4 | Undo and redo, multi-cursor, and search, as provided by CodeMirror | P0 |
| E5 | Autocomplete for variable and unit names | P1 |
| E6 | Light and dark themes that follow the system setting, both meeting WCAG 2.2 AA contrast for text and results | P0 |
| E7 | Adjustable font and font size | P1 |
| E8 | Alt-click or a shortcut copies the bare number (no unit, no group separators) | P1 |
| E9 | A shortcut announces the current line's result to screen readers (ARIA live region) | P1 |

### Notes

| ID | Requirement | Priority |
| --- | --- | --- |
| N1 | Multiple notes, each a `.txt` file in the notes directory | P0 |
| N2 | Autosave with a 500 ms debounce, written atomically; no save button | P0 |
| N3 | Quick switcher (Cmd/Ctrl+P) to create, open, rename, and delete notes | P0 |
| N4 | Reopen the last note and cursor position on launch | P1 |
| N5 | Detect external changes to the open note (sync tools, other editors) without losing data (see [Storage rules](#storage-rules)) | P0 |

### App shell

| ID | Requirement | Priority |
| --- | --- | --- |
| A1 | Global hotkey to toggle the window on macOS and X11 (default Alt+Space, configurable) | P0 |
| A2 | `notecal --toggle` shows or hides the running instance (single-instance); this is the Wayland path, bound in the compositor config (Hyprland, Sway, GNOME) | P0 |
| A3 | Tray or menu bar icon that keeps the app running in the background | P1 |
| A4 | Optional always-on-top mode | P1 |
| A5 | Remember window size and position | P1 |
| A6 | Settings screen covering hotkey, theme, font, decimal precision, locale, notes directory, and currency refresh | P1 |
| A7 | Command-line evaluation, e.g. `notecal "5 km in miles"`, that prints the result and exits | P2 |

## Non-functional requirements

"Light and fast" is defined by the budgets below. CI checks the size budgets. Performance is measured manually on both machines at Gate 1 and before each release.

| Metric | Budget | How measured |
| --- | --- | --- |
| Package size | `.dmg` ≤ 15 MB; AUR package ≤ 15 MB. AppImage is tracked, not budgeted (it bundles WebKitGTK) | CI artifact size |
| Cold start to editable | ≤ 400 ms macOS; Linux target set at Gate 1 from measurement | Timestamp from process start to first editor focus |
| Hotkey show (app already running) | ≤ 100 ms | Screen capture |
| Idle RAM | ≤ 80 MB macOS; Linux target set at Gate 1 from measurement | Sum of PSS across the app and all webview processes (`smem` / Activity Monitor) |
| Keystroke to updated results | ≤ 50 ms p95 for a 200-line note, end to end | Frame timing in the webview |
| Full evaluation of a 200-line note, nothing cached | ≤ 15 ms | `criterion` benchmark |
| Re-evaluation after editing the last line of a 200-line note | ≤ 2 ms | `criterion` benchmark |
| Single-line evaluation | ≤ 200 µs median | `criterion` benchmark |
| Per-line time limit (C13) | 50 ms, then the line is interrupted | Engine test |
| Idle CPU | ~0% with no timers or polling | `top` |

Engine budgets are set from fend-core's measured speed (M1 spike: ~120 µs per line, ~11 ms per 200-line note). The cache re-evaluates from the first changed line downward, so typing near the end of a note costs well under 2 ms, and editing the top line costs at most a full evaluation; even an uncached 15 ms evaluation plus IPC and render fits inside the 50 ms end-to-end budget.

**Other requirements**

- Offline-first: everything except refreshing currency rates works without a network connection.
- Privacy: no telemetry. The only network call is the currency-rate fetch.
- Reliability: no input can crash, hang or freeze the app. Engine panics are caught at the command boundary, evaluation runs off the UI thread, and long-running lines are interrupted (C13). A fuzz or property test checks that the engine never panics.
- Data safety: no code path overwrites a note's on-disk content that the app has not read (see Storage rules).
- Accessibility: full keyboard navigation, WCAG 2.2 AA contrast in both themes, and screen-reader access to results (E9).

## Technical architecture

The Rust process owns all logic. The Svelte webview is a thin editor: it sends the note text and draws the results it gets back. This keeps the engine testable on its own, and a native frontend could replace the webview later if needed.

```
┌─────────────── Webview (Svelte 5 + CodeMirror 6) ───────────────┐
│ editor ── on change, coalesced to next frame ──► invoke evaluate │
│ result decorations ◄── results tagged with doc version ──────────┤
└──────────────────────────────┬───────────────────────────────────┘
                               │ Tauri IPC
┌──────────────────────────────▼───────────────────────────────────┐
│ src-tauri: commands, plugins, storage, rate fetcher              │
│   evaluate → worker thread → notecal-core (catch_unwind)         │
├──────────────────────────────────────────────────────────────────┤
│ notecal-core: preprocessor (Numi syntax, locale) → fend-core     │
│   per-line cache, variables, prev/sum/avg, interrupt (C13)       │
└──────────────────────────────┬───────────────────────────────────┘
                               │
            notes/*.txt · settings.toml · state.json · rates.json
```

**Evaluation flow.**

1. Edits are coalesced to at most one request per animation frame. There is no fixed debounce.
2. Each request carries the editor's document version. Evaluation runs on a worker thread. A newer request supersedes an older one, and the UI drops any response older than the current version.
3. The engine evaluates line by line, top to bottom, so variables, `prev` and `sum` resolve correctly.
4. The engine keeps each line's result and the evaluation state after it. On the next request, lines above the first changed line are reused and evaluation resumes from there. This is required, not optional: fend-core costs ~120 µs per line. (A finer, dependency-aware cache was rejected: fend does not expose its values, and the prefix cache already meets the budgets.)

**IPC contract.**

```
evaluate({ version, text, locale }) →
  { version, lines: [{ line, display: string | null, copy: string | null, error: string | null }] }
```

`display` is what the result column shows. `copy` is the bare number for E8. `error` is only for the hover message.

| Layer | Choice | Why |
| --- | --- | --- |
| Shell | Tauri 2 | System webview for a small package; plugins provide global shortcut, single instance, tray, and window state |
| UI | Svelte 5 + Vite, TypeScript | Small compiled output; no router or SvelteKit needed |
| Editor | CodeMirror 6 | Line decorations for results, plus a custom language mode for highlighting |
| Engine | Rust crate `notecal-core` wrapping fend-core | Arbitrary precision, units, an exchange-rate hook and an interrupt hook, with no dependency on the UI |
| Tests | `cargo test` golden cases, property/fuzz tests, `criterion` benches, Vitest | Locks in syntax behaviour, robustness and the speed budgets |
| CI/CD | GitHub Actions + tauri-action | Builds the `.dmg`, AppImage, and AUR inputs on every tag |

**Repo layout**

```
notecal/
├── crates/notecal-core/   # preprocessor + fend wrapper, no Tauri deps
├── crates/notecal-cli/    # CLI evaluation (A7, P2)
├── src-tauri/             # commands, plugins, storage, --toggle
├── ui/                    # Svelte + CodeMirror
└── docs/                  # this PRD
```

## Data, storage and settings

All data lives in plain files under the platform's standard directories, resolved through Tauri's path API. There is no database.

| Data | Location (Linux / macOS) | Format |
| --- | --- | --- |
| Notes | `~/.local/share/notecal/notes/` / `~/Library/Application Support/notecal/notes/` | One `.txt` per note |
| Settings | `~/.config/notecal/settings.toml` / `~/Library/Application Support/notecal/settings.toml` | TOML |
| Currency rates | `~/.cache/notecal/rates.json` / `~/Library/Caches/notecal/rates.json` | JSON with `fetched_at` timestamp |
| UI state | Same directory as settings, in `state.json` | Last note, cursor, window geometry |

The notes directory can be changed in settings, so users can point it at a git- or Syncthing-synced folder.

### Storage rules

- **Atomic writes.** Notes are written to a hidden temp file in the same directory (`.notecal-tmp-*`), then renamed. The note list ignores these temp files.
- **No blind overwrites.** The app remembers the content hash of the file as last read or written. Before each autosave it checks the on-disk file:
  - unchanged on disk → save;
  - changed on disk and no unsaved local edits → reload from disk;
  - changed on disk and unsaved local edits → write the local version to `<name> (conflict YYYY-MM-DD HHMM).txt`, then reload from disk, and tell the user.
- **Watching.** The notes directory is watched with a file watcher, not by polling, so the idle-CPU budget holds.
- **Deleted externally.** If the open note's file disappears, the app keeps the buffer and saves it again on the next edit.

### Currency rates

- Source: `https://open.er-api.com/v6/latest/USD` (free, no API key, includes IDR). The provider's attribution terms are honoured with a credit line in Settings/About.
- Rates refresh at most once every 24 h, and only when a note uses a currency.
- The status line shows the date of the rates in use.
- If no rates have ever been fetched, currency lines show no result, with the hover message "No exchange rates yet".
- The fetcher lives in one module in `src-tauri` and feeds rates into fend-core's exchange-rate hook.

## Milestones and release plan

v1.0 is planned as five milestones totalling about 9 weeks of part-time work. The engine comes first, so syntax and speed are proven before UI work starts.

> The milestone breakdown below was reconstructed in revision 2 because the original embedded milestone chart did not export. Adjust it if it differs from the original plan.

### Delivery Milestones
<!-- Status: pending | in-progress | complete. /plan turns each row into a plan. -->

| # | Milestone | Outcome | Status | Plan |
|---|---|---|---|---|
| 1 | Engine core (~2.5 w) | `notecal-core` evaluates C1–C8, C10, C12, C13 with the language rules; ≥ 150 golden cases, fuzz test, per-line cache, and benches meeting the engine budgets | complete | `.claude/plans/m1-engine-core.plan.md` |
| 2 | Shell spike + Gate 1 (~1.5 w) | Tauri + Svelte + CodeMirror window with live right-aligned results for one note on both machines; Gate 1 measurements recorded | in-progress | `.claude/plans/m2-shell-spike.plan.md` |
| 3 | Notes and editor (~2 w) | N1–N5, E1–E6 (E5, E7–E9 if time allows); notes are safe under Syncthing | pending | — |
| 4 | App shell and currency (~1.5 w) | A1, A2, A3–A6, C9 with cached offline rates | pending | — |
| 5 | Release + Gate 2 (~1.5 w) | CI builds `.dmg`, AppImage and AUR inputs with size checks; ≥ 300 golden cases; v1.0 tagged | pending | — |

**Gate 1 (end of M2): WebKitGTK on Arch.** This is the biggest risk, so it is checked before investing in features. Measure on the author's Arch/Hyprland machine and macOS machine: keystroke-to-results p95 on a 200-line note, cold start, and idle RAM.

- Pass: the end-to-end typing budget is met. The Linux cold-start and RAM targets are then fixed from the measurements.
- Fail: try the WebKitGTK workaround flags (for example `WEBKIT_DISABLE_DMABUF_RENDERER=1`). If they don't help, M3 onward moves to an iced frontend on top of the same `notecal-core` crate.

**Gate 2 (end of M5): release.** All budgets in Non-functional requirements are met on both platforms, and the golden suite passes in CI.

## Risks, open questions and success metrics

### Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| WebKitGTK is slow or glitchy on some Wayland/GPU setups | Typing lag on Arch | Gate 1 at end of M2; workaround flags; the engine stays UI-agnostic, so an iced frontend remains an option |
| WebKitGTK cold start and memory exceed the macOS-level budgets | Linux misses "light and fast" | Linux targets are set from Gate 1 measurements, not assumed |
| fend-core syntax differs from Numi's (`prev`, `sum`, `% of`, `50 + 10%`) | Expressions feel wrong | Preprocessing layer rewrites Numi syntax before fend; the percent and reference rules are golden-tested in the first week of M1 |
| fend-core is slow (~120 µs per line, measured in M1) | Typing lag on long notes | Per-line cache limits re-evaluation to changed lines and their dependants; budgets set from measurement |
| Pathological input (`10^10^10`, huge factorials) | CPU or memory spike, frozen results | Per-line interrupt (C13), evaluation off the UI thread, fuzz test |
| Global hotkeys do not work on Wayland | No system-wide hotkey on Hyprland | `notecal --toggle` (A2) is P0; docs show the Hyprland, Sway and GNOME bindings |
| Wayland compositors deny focus to the shown window | Hotkey show misses 100 ms or opens unfocused | Test at Gate 1; use xdg-activation where available |
| A sync tool changes a note while it is open | Lost edits | Storage rules: hash check before save, conflict copy instead of overwrite |
| The currency API changes or shuts down | C9 breaks | Cached rates are always served; the fetcher is isolated in one module |
| Results get misaligned on long or wrapped lines | Poor readability | No soft wrap by default; results column has a fixed minimum width |

### Decisions (from revision 1 open questions)

- App name: **notecal**.
- Currency source: **open.er-api.com**; it must include IDR.
- Variables: **scoped per note** only.
- Open source: **yes**.
- macOS signing: **unsigned first**; notarization revisited later.

### Open questions

- [ ] License: MIT or GPL-3.0?
- [ ] Linux cold-start and idle-RAM targets (set at Gate 1).

### Success metrics

- notecal becomes the default calculator on both of the author's machines within 2 weeks of v1.0.
- All budgets in Non-functional requirements are met on both platforms.
- The golden test suite has at least 300 expression cases, all passing in CI.

## Changes from revision 1

- Renamed Tally to notecal everywhere (crates, binary, paths); recorded the answered open questions as decisions.
- Fixed contradictory latency budgets: the old 1 ms per line could not fit 200 lines into 16 ms, and the 30 ms debounce exceeded the 16 ms keystroke budget. Replaced with per-line, per-note and end-to-end budgets that add up, and frame-coalesced requests instead of a debounce.
- Package-size budget is now per artifact; the AppImage bundles WebKitGTK and cannot meet 15 MB. Linux cold-start and RAM targets are set from Gate 1 measurements; RAM is measured as summed PSS across webview processes.
- Added C13 (per-line time limit), off-thread evaluation, versioned IPC responses, and a fuzz test, because catching panics does not stop runaway arbitrary-precision math.
- Defined percentage semantics, `sum`/`total`/`avg`/`prev` and block rules, variable shadowing, locale separators, display precision, and copy behaviour. Fixed the example note, which double-counted values in `sum`.
- Resolved the IPC `error?` vs C7 conflict: errors never show inline, only on hover.
- Raised C12 to P0, since the core example (`4,500,000 IDR`) depends on it.
- Split the hotkey: `notecal --toggle` is P0 because the global-shortcut plugin does not work on Wayland. The CLI evaluation requirement stays P2 as A7.
- Added N5 and storage rules so synced folders cannot cause data loss.
- Removed the "pluggable currency provider" (one provider; isolated module instead). Deferred the Homebrew cask pending signing.
- Restored the architecture diagram and reconstructed the milestone and gate definitions lost in export, in the table format ECC's `/plan` reads.
- Made accessibility concrete: WCAG 2.2 AA contrast and screen-reader access to results.
- (M1, after the fend spike) Engine budgets set from measurement: ≤ 200 µs per line, ≤ 15 ms uncached per 200-line note, ≤ 2 ms to re-evaluate after editing the last line; the prefix cache became required.
