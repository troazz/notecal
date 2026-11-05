# Gate 1 — WebKitGTK on Arch (M2)

Gate 1 decides whether M3 continues on the Tauri/WebKit frontend or moves to an iced frontend on the same `notecal-core` crate (PRD "Milestones and release plan").

## Method

All numbers come from a **release build** (`cd ui && bun run tauri build --no-bundle`) opening the 200-line fixture note (`ui/src/fixtures/note-200.txt`).

| Metric | How | Script / tool |
|---|---|---|
| Cold start to editable | Launch → editor's first focus (`mark_ready` logs wall-clock ms). 5 runs, median. Measured from the launcher's clock, so it includes process exec and library loading. | `scripts/measure-start.sh [binary] [runs]` |
| Keystroke to updated results | Type ≥ 200 characters spread over the note (edits at top, middle and end). Press `Ctrl+Shift+M` (`Cmd+Shift+M` on macOS) and read p95. Each sample runs from the edit to the frame after the matching results are applied. | In-app overlay (`ui/src/editor/perf.ts`) |
| Idle RAM | Leave the window untouched 60 s, then sum PSS of the app and every WebKit child process. | Linux: `scripts/measure-ram.sh`. macOS: Activity Monitor "Memory" for notecal plus its WebContent and Networking processes. |
| Idle CPU | Same idle window, `top` over 9 s for the app and its children. | `top -p <pids>` |

## Arch Linux (Hyprland, Wayland) — 2026-11-05

WebKitGTK 2.54.1.

| Metric | Default renderer | `WEBKIT_DISABLE_DMABUF_RENDERER=1` | PRD |
|---|---|---|---|
| Cold start (median of 5) | 435 ms | 321 ms | Linux target set here |
| Idle RAM (PSS, all processes) | 202 MiB | 163 MiB | Linux target set here |
| Idle CPU | ~11% (app 8.5%, web process 3%) | ~1% | ~0% |
| Keystroke → results p95 | not measured | 43 ms (p50 27 ms, max 48 ms; 110 keystrokes, rerun with ≥ 200 pending) | ≤ 50 ms |
| Rendering glitches | none seen | results drawn off-screen when one line is wider than the window (fixed, see below) | — |

**Change made:** the app now sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` at startup on Linux unless the variable is already set (`src-tauri/src/main.rs`). With it, idle CPU meets the budget and cold start and RAM improve. Setting the variable to `0` restores the DMABUF renderer.

**Bug found in the typing run:** with soft wrap off, every line is as wide as the longest one, so results aligned to the line's right edge were pushed out of view once any line was wider than the window. Results now stay pinned to the visible right edge (`ui/src/editor/results.ts`, `pinToViewport`) and cover any text they overlap.

## macOS

_Pending: run on the Mac (prerequisites in the M2 plan, Task 6)._

| Metric | Result | PRD |
|---|---|---|
| Cold start (median of 5) | | ≤ 400 ms |
| Idle RAM | | ≤ 80 MB |
| Idle CPU | | ~0% |
| Keystroke → results p95 | | ≤ 50 ms |

## Decision

_Pending the keystroke p95 on Arch and the macOS numbers._
