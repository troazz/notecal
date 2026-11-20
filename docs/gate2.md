# Gate 2 — release budgets (M5)

Gate 2 checks every PRD budget before v1.0.0, on both of the author's machines, with the release build. Method as in [Gate 1](gate1.md): a release build (`cd ui && bun run tauri build --no-bundle`) with the 200-line fixture note (`ui/src/fixtures/note-200.txt`) open.

## Arch Linux (Hyprland, Wayland) — 2026-11-18

WebKitGTK 2.54.1, release build of `main` after M6–M11 (dates, CSS units, definitions, CLI, languages, status bar, cheatsheet).

| Metric | Gate 1 (M2) | Gate 2 | PRD budget | |
|---|---|---|---|---|
| Cold start (median of 5, `scripts/measure-start.sh`) | 321 ms | **298 ms** (292–311) | ≤ 400 ms | pass |
| Hotkey show: `notecal --toggle` until Hyprland maps the window (median of 5) | — | **92 ms** (84–101) | ≤ 100 ms | pass |
| Idle RAM, PSS of app + WebKit processes after 60 s (`scripts/measure-ram.sh`) | 163 MiB | **174 MiB** | ≤ 200 MiB | pass |
| Idle CPU, window hidden or unfocused (`top`, 30 s) | ~1% | **1.1–1.2%** | ~0% | pass, as at Gate 1 |
| Idle CPU, window focused (`top`, 30 s) | — | **2.4%** | ~0% | see below |
| Arch package (M8 build: app, `notecal-cli`, `notecal-calc`) | 4.3 MB | **5.4 MB** (12.95 MiB installed) | ≤ 15 MB | pass |
| `.dmg` (CI, release test `v1.0.0-rc.4`) | — | **9.6 MB** | ≤ 15 MB | pass |
| Keystroke → results p95, typed by hand | 45 ms | _pending: the author types it_ | ≤ 50 ms | |
| Engine benches (`cargo bench -p notecal-core --bench eval`) | — | 200-line note 12.5 ms; single line 124 µs; last-line edit 124 µs | ≤ 15 ms; ≤ 200 µs; ≤ 2 ms | pass |

**Idle CPU fixes made during this gate** (commit `cf9185f`):
- **Starting point:** measured idle CPU was 4.3% focused and 2.2% hidden.
- **The file watchers:** five debounced watches (notes, settings, themes, Omarchy theme) each woke up every 25 ms, the debouncer's default of a quarter of the 100 ms window. They now tick every 200 ms, with a 200 ms window, so an external change still shows within about half a second. Hidden: 2.2% → 1.2%.
- **The blinking cursor:** it made WebKit repaint about twice a second, roughly 2% while the window was focused. The cursor is steady now. Focused: 3.4% → 2.4%.
- **Remaining:** while focused, the cost is WebKit's own (about 1.4% in the web process). It wasn't investigated further.

**Typing:** a scripted run with `wtype` (124 keystrokes at top, middle and end, with 95-line jumps between them) gave p50 27 ms but p95 213 ms (max 245 ms). Scripted typing was noisy at earlier gates too, and the Gate 1 number is hand-typed. So the budget is decided by a hand-typed run on this build. If that also shows a slow tail, it gets investigated before 1.0.0.

## macOS

_Pending: the author measures the CI `.dmg` on the Mac._

| Metric | Result | PRD |
|---|---|---|
| Cold start (median of 5) | | ≤ 400 ms |
| Hotkey show (Alt+Space) | | ≤ 100 ms |
| Idle RAM (Activity Monitor: notecal + WebContent + Networking) | | ≤ 80 MB |
| Idle CPU | | ~0% |
| Keystroke → results p95 | | ≤ 50 ms |

## Decision

2026-11-20: the author released v1.0.0 with the Arch measurements above. Two measurements were still open:
- the hand-typed keystroke p95 on Arch (the scripted p95 was 213 ms);
- the macOS numbers.

They are follow-ups for 1.0.x, not release blockers.
