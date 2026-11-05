# Plan: M2 — Shell spike + Gate 1

**Source PRD**: `docs/notecal.prd.md`
**Selected Milestone**: 2 — Shell spike + Gate 1 (~1.5 w)
**Complexity**: Medium

## Summary

Put `notecal-core` behind a minimal Tauri 2 + Svelte 5 + CodeMirror 6 window: one editor, live right-aligned results, evaluation off the UI thread with versioned responses. Then measure Gate 1 on the Arch/Hyprland machine and the macOS machine: keystroke-to-results p95 on a 200-line note, cold start, and idle RAM. Gate 1 decides whether M3 continues on WebKitGTK or moves to an iced frontend. Notes, saving, highlighting, hotkeys and settings are out of scope (M3/M4); the spike opens a fixed 200-line fixture note.

## Requirements restated

| Item | Done means |
|---|---|
| E1 (spike level) | Each line's result is drawn right-aligned on that line, in a muted colour, updating as you type |
| Evaluation flow (PRD Technical architecture) | Edits coalesced to one request per animation frame; each request carries a document version; evaluation runs off the UI thread; superseded requests are skipped; the UI drops responses older than the current version |
| IPC contract | `evaluate({ version, text, locale }) → { version, lines: [{ display, copy, error }] } \| null` (`null` = superseded) |
| E6 (basic) | Light and dark colours follow the system setting |
| Gate 1 | Measurements recorded in `docs/gate1.md` for both machines, with a pass/fail call against the PRD rules |

Out of scope: N1–N5, E2–E5, E7–E9, A1–A7, packaging, size budgets.

## Key facts (checked 2026-11-05)

- Arch machine: `webkit2gtk-4.1` 2.54.1, `librsvg`, `base-devel` installed; session is Wayland on Hyprland. `libappindicator` is not installed (tray is M4, not needed now).
- Current versions: `tauri` 2.12.5, `tauri-build` 2.7.4, `@tauri-apps/cli` and `@tauri-apps/api` 2.12.4, `svelte` 5.57.3, `vite` 8.3.6, `@sveltejs/vite-plugin-svelte` 7.3.1, `codemirror` 6.0.2 (`@codemirror/view` 6.43.15, `@codemirror/state` 6.7.6), `vitest` 5.1.0, `svelte-check` 4.7.8. TypeScript: `svelte-check` supports `^5 || ^6`, so the project pins `typescript` 6.0.3 (7.0.2 is the latest release but unsupported by Svelte tooling). Package manager: bun 1.4.4 (user's choice).
- `notecal-core` exposes `Engine::evaluate(&mut self, text, &Settings, Option<&Rates>) -> Vec<LineResult>`; one `Engine` per open note keeps the prefix cache warm.

## Design

**Rust side (`src-tauri`)**

```rust
struct Evaluator { engine: Mutex<Engine>, latest: AtomicU64 }

#[tauri::command]
async fn evaluate(req: EvalRequest, state: State<'_, Evaluator>) -> Result<Option<EvalResponse>, String>
```

- The command records `req.version` in `latest` (`fetch_max`), then runs the evaluation on `tauri::async_runtime::spawn_blocking`.
- Inside the blocking task: lock the engine. If `latest` has moved past this request's version, return `None` without evaluating. That makes "newer supersedes older" a lock plus one atomic compare, with no queue or channel.
- `EvalRequest`/`EvalResponse` are serde types in `src-tauri`; `notecal-core` stays free of serde and Tauri.
- Panics are already caught per line in the engine. The command also maps a poisoned mutex to a fresh `Engine` so one bad state cannot wedge the app.
- `mark_ready` command: logs milliseconds from process start (captured first thing in `main`) to the UI's first editor focus, for the cold-start metric.
- Capabilities: only `core:default` plus the two commands. A strict CSP (`default-src 'self'`), no remote content.

**UI side (`ui/`)**

- Vite + Svelte 5 + TypeScript, no router. One component mounts CodeMirror.
- `results.ts`: a CodeMirror `StateField` holding the latest results, plus a `ViewPlugin` that draws them as end-of-line widget decorations. The widgets are absolutely positioned at the right edge of the line, so they never shift the text. Soft wrap is off (PRD risk table).
- `scheduler.ts`: framework-free module. On any doc change: bump `version`, schedule one `requestAnimationFrame`, and invoke `evaluate` with the current text. On response: apply only if `response.version === current version`. Unit-tested with Vitest using fake rAF and a fake invoke.
- Perf overlay (dev aid, toggle `Ctrl+Shift+M`): for each keystroke, time from the input transaction to the frame after the results decoration is applied; shows count, p50 and p95 over the last 200 keystrokes, with a "copy stats" button.
- Fixture: `ui/src/fixtures/note-200.txt`, a realistic 200-line note (mixed units, currency, percentages, totals, blank-line blocks), loaded at start.

**Workspace**

- Add `src-tauri` to the Cargo workspace members.
- Package manager: bun (`bun install`, `bun run`, lockfile `bun.lock`). Tauri's `beforeDevCommand`/`beforeBuildCommand` call `bun run`. CI uses `oven-sh/setup-bun`.
- CI: install the WebKitGTK build dependencies on the Ubuntu runner so `clippy`/`test` cover `src-tauri`; add a `ui` job (`bun install --frozen-lockfile`, `bun run check`, `bun run test`, `bun run build`).

## Patterns to Mirror

| Category | Source | Pattern |
|---|---|---|
| Naming | `crates/notecal-core/src/lib.rs` | `snake_case` modules, small private helpers, doc comments on public items |
| Errors | `crates/notecal-core/src/lib.rs` (`LineResult::error`, `run_fend`) | User-facing failures become data, never panics; messages are full sentences |
| Tests | `crates/notecal-core/src/lib.rs` (`incremental_matches_fresh_evaluation`), `tests/golden.rs` | Behaviour tests next to the code; equivalence and table-style cases |
| UI | — | No existing frontend code. Conventions set here become the pattern for M3. |

## Files to Change

| File | Action | Why |
|---|---|---|
| `Cargo.toml` | UPDATE | Add `src-tauri` to workspace members |
| `src-tauri/Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json`, `icons/` | CREATE | Tauri 2 app shell |
| `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` | CREATE | App setup, `evaluate` and `mark_ready` commands |
| `src-tauri/src/evaluator.rs` | CREATE | `Evaluator` with supersede logic, unit-tested without a Tauri runtime |
| `ui/package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `svelte.config.js` | CREATE | Frontend toolchain |
| `ui/src/main.ts`, `ui/src/App.svelte` | CREATE | Mount editor |
| `ui/src/editor/results.ts`, `ui/src/editor/scheduler.ts`, `ui/src/editor/perf.ts` | CREATE | Result decorations, request coalescing and versioning, perf overlay |
| `ui/src/editor/*.test.ts` | CREATE | Vitest for scheduler and perf stats |
| `ui/src/fixtures/note-200.txt` | CREATE | Gate 1 fixture |
| `scripts/measure-ram.sh` | CREATE | Sum PSS of the app and all WebKit child processes (Linux) |
| `.github/workflows/ci.yml` | UPDATE | WebKitGTK deps; `ui` job |
| `docs/gate1.md` | CREATE | Gate 1 method, measurements and decision |
| `docs/notecal.prd.md` | UPDATE | Milestone 2 row → in-progress (done by `/plan`) |

## Tasks

### Task 1: Scaffold the shell
- **Action**: Create `ui/` (Vite + Svelte 5 + TS) and `src-tauri/` (Tauri 2) by hand, pinned to the versions above. Add `src-tauri` to the workspace. Window: 720×520, titled "notecal". CSP and capabilities as in Design.
- **Validate**: `cd ui && bun run tauri dev` opens a window on Hyprland showing a placeholder; `cargo clippy --all-targets -- -D warnings` stays clean.

### Task 2: `evaluate` command with superseding
- **Action**: `Evaluator`, serde DTOs, `evaluate` and `mark_ready` commands. Confirm at compile time that `Engine` is `Send` (fend's `Context` must be, to live in Tauri state).
- **Validate**: Rust unit tests: in-order requests all evaluate; a request whose version is older than `latest` returns `None`; results equal `evaluate_note` for the same text.

### Task 3: Editor with live results
- **Action**: CodeMirror editor, `results.ts` decorations, `scheduler.ts` coalescing, system light/dark colours, fixture loaded at start.
- **Validate**: Vitest: many changes in one frame produce one request; a stale response is ignored; the latest response is applied. Manual: typing in the 200-line fixture updates results on the same line, right-aligned, with no text shift.

### Task 4: Measurement instruments
- **Action**: Perf overlay (`Ctrl+Shift+M`), `mark_ready` cold-start log, `scripts/measure-ram.sh`, and a short method section in `docs/gate1.md` (release build, run count, how p95 is taken).
- **Validate**: Overlay shows stable numbers after 200 keystrokes; `mark_ready` prints once per launch; the RAM script prints per-process PSS and a total for the running app.

### Task 5: CI
- **Action**: Install WebKitGTK dev packages in the Rust job; add the `ui` job.
- **Validate**: Workflow passes on the first push.

### Task 6: Gate 1 run and decision (needs you on both machines)
- **Prerequisites on the Mac**: Xcode command-line tools (`xcode-select --install`), rustup, bun, and a clone of the repo.
- **Action**: Build release (`bun run tauri build --no-bundle`). On each machine: 5 cold starts (median), 200+ keystrokes in the fixture (p95 from the overlay), idle RAM after 60 s untouched. On Arch, also note any rendering glitches; if typing p95 fails, repeat with `WEBKIT_DISABLE_DMABUF_RENDERER=1`. Record everything in `docs/gate1.md` and make the call: pass → set the Linux cold-start and RAM targets in the PRD; fail → the PRD's iced fallback for M3.
- **Validate**: `docs/gate1.md` has both machines' numbers and a decision; PRD Linux targets filled in (or the M3 plan switched).

## Validation

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cd ui && bun run check && bun run test && bun run build
cd ui && bun run tauri dev        # manual
```

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| fend's `Context` is not `Send`, so `Engine` cannot sit in Tauri state | Low | Checked at compile time in Task 2; fallback is a dedicated worker thread owning the engine, fed by a channel |
| Vitest under bun | Low | Run Vitest through `bun run test` (Vitest's own runner, not `bun test`) |
| Widget decorations reflow or flicker on WebKitGTK | Medium | Absolutely positioned widgets, no soft wrap; this is exactly what Gate 1 observes |
| Hyprland/WebKitGTK rendering glitches (DMABUF) | Medium | Gate 1 retries with `WEBKIT_DISABLE_DMABUF_RENDERER=1` before considering iced |
| Cold-start measurement is noisy | Medium | Release build, 5 runs, report median; measure from process start, not window creation |
| macOS build needs local toolchain | Low | Mac is available; Task 6 lists Xcode command-line tools, rustup and bun as prerequisites |

## Acceptance

- [x] All tasks complete (Task 6 on Arch only; macOS deferred to M5)
- [ ] Validation passes locally and in CI (local passes; CI runs on first push)
- [x] Typing in the 200-line fixture shows live, right-aligned results (Arch)
- [x] `docs/gate1.md` records cold start, keystroke p95 and idle RAM for Arch, and a pass decision
- [x] PRD updated with the Linux cold-start and RAM targets
- [x] Patterns mirrored, not reinvented
