# Plan: M5 — Release + Gate 2

**Source PRD**: `docs/notecal.prd.md`
**Selected Milestone**: 5 — Release + Gate 2 (~1.5 w)
**Complexity**: Large

## Summary

Ship notecal v1.0: CI builds the macOS `.dmg` and the Linux AppImage on every `v*` tag with size checks, the repo carries an AUR `PKGBUILD` that builds from the tagged source, the golden suite grows to at least 300 cases, and Gate 2 checks every non-functional budget on both machines. This milestone also settles the items carried forward: typing latency (accepted over budget in M3), the macOS run deferred at Gate 1, and the X11/macOS code paths that could not be tested on the Wayland-only machine.

## Requirements restated

| Item | Done means |
|---|---|
| Release CI | A `v*` tag builds `.dmg` (macOS, Apple Silicon and Intel) and AppImage (Linux) with tauri-action and attaches them to a draft GitHub Release; a step fails the job if the `.dmg` is over 15 MB and records the AppImage size |
| AUR | `packaging/aur/PKGBUILD` and `.SRCINFO` build `notecal` from the tagged source against system WebKitGTK; installs the binary, a `.desktop` file, the icon and the license; package ≤ 15 MB; builds cleanly with `makepkg` on Arch |
| Golden suite | ≥ 300 cases, covering every language rule, both locales, currency with fixed rates, units including the M3 division fix, and errors; passes in CI |
| Gate 2 | Every PRD budget measured on Arch (release package) and on the Mac (the CI `.dmg`), recorded in `docs/gate2.md` with a pass/fail per budget |
| v1.0 | Version 1.0.0 everywhere, README with install and usage, CHANGELOG, tag `v1.0.0` (pushed only with your go-ahead) |

## Key facts (checked 2026-11-05)

- Release binary today: 21.1 MB, 14.6 MB stripped, about 3.9 MB compressed. The workspace has no `[profile.release]` yet, so `strip`, LTO and `codegen-units = 1` should bring it well under 15 MB.
- Golden suite: 176 `=>` cases in 13 files (the harness counts 218 including no-result lines). It needs about 85–125 more.
- Icons are still generated from a placeholder `source.png`.
- `tauri-action` is at `action-v1.0.0`. CI's Rust job is currently slow because the M4 dependencies are compiling uncached.
- `makepkg` is installed; `namcap` is not (optional, for linting the package).
- There is no README in the repo yet.
- The X11 and macOS paths for the global shortcut, window state and always-on-top have never run. Typing p95 was 69 ms by hand (budget 50 ms), accepted at M3 for revisiting here.

## Design

- **Release profile** (workspace `Cargo.toml`): `strip = true`, `lto = "fat"`, `codegen-units = 1`, `opt-level = 3`. `panic` stays `unwind`, because the engine catches panics per line. Engine benches are re-checked after the change.
- **Release workflow** (`.github/workflows/release.yml`, triggered on `v*` tags):
  - Matrix: `macos-latest` (`--target universal-apple-darwin`) and `ubuntu-22.04` (older glibc so the AppImage runs on more distros).
  - Each runner sets up bun and Rust, then runs tauri-action with `tagName` and a draft release.
  - A size-check step reads the produced bundle sizes and fails if the `.dmg` is over 15 MB.
  - The CI workflow keeps doing checks only.
- **AUR** (`packaging/aur/`):
  - `PKGBUILD` named `notecal`, with `source = git+https://github.com/troazz/notecal.git#tag=v$pkgver`.
  - `makedepends`: `rust`, `bun`, `cargo-tauri` or `bun x tauri`.
  - `depends`: `webkit2gtk-4.1`, `gtk3`, `libayatana-appindicator`.
  - `build()` runs `bun install --frozen-lockfile` and `bun run tauri build --no-bundle`. `package()` installs the binary, `notecal.desktop`, the icons and `LICENSE`.
  - `.SRCINFO` is generated with `makepkg --printsrcinfo`.
  - Publishing to the AUR needs your AUR account (SSH key), so you run the push. The plan provides the exact commands.
- **Golden cases**: new files by area (`currency.txt`, `locale_id.txt`, `division.txt`, `edge.txt`, more percent and reference cases). Any case that exposes a bug gets fixed in the engine with the case kept.
- **Icon**: a simple original SVG (a notepad page with a `=` result line), rendered to `source.png` and regenerated with `bun run tauri icon`.

## Patterns to Mirror

| Category | Source | Pattern |
|---|---|---|
| Golden tests | `crates/notecal-core/tests/golden/*.txt`, `tests/golden.rs` | `input => display [|| copy]`, `---` between notes, `@locale` / `@precision` directives |
| CI | `.github/workflows/ci.yml` | apt deps for WebKitGTK, `oven-sh/setup-bun`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache` |
| Measurements | `docs/gate1.md`, `scripts/measure-start.sh`, `scripts/measure-ram.sh` | Same method and table layout for Gate 2 |
| Docs | `docs/linux-setup.md` | User-facing setup in plain steps |

## Files to Change

| File | Action | Why |
|---|---|---|
| `Cargo.toml` | UPDATE | Release profile |
| `crates/notecal-core/tests/golden/*.txt`, `tests/golden.rs` | CREATE/UPDATE | ≥ 300 cases; raise the harness minimum to 300 |
| `crates/notecal-core/src/*` | UPDATE (if cases find bugs) | Engine fixes |
| `src-tauri/icons/*`, `assets/icon.svg` | UPDATE/CREATE | Real icon |
| `.github/workflows/release.yml` | CREATE | Tagged builds and size checks |
| `packaging/aur/PKGBUILD`, `.SRCINFO`, `notecal.desktop` | CREATE | AUR package |
| `README.md`, `CHANGELOG.md` | CREATE | Install, usage, shortcuts, files, credits |
| `docs/gate2.md` | CREATE | Gate 2 measurements and decision |
| `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `crates/notecal-core/Cargo.toml`, `ui/package.json` | UPDATE | Version 1.0.0 |
| `docs/notecal.prd.md` | UPDATE | M5 row, Gate 2 result, typing budget decision |

## Tasks

### Task 1: Release profile and size ✅
- **Action**: Add `[profile.release]` and rebuild. Compare binary size and engine benches before and after.
- **Validate**: Stripped binary well under 15 MB. `cargo bench -p notecal-core -- --quick` stays within the engine budgets. The app still starts.
- **Done**: binary 21.1 MB → 11.2 MB (strip + fat LTO + one codegen unit). Engine got faster too: `note_200_uncached` 12.9 ms (was ~14.7), `single_line` 135 µs, last-line edit 130 µs. Local release build now takes ~3.5 min. App starts normally.

### Task 2: Golden suite to ≥ 300 ✅
- **Action**: Write cases per language rule and locale, plus currency (fixed rates), division, errors and edge cases. Fix any engine bug the cases expose. Raise the harness minimum to 300.
- **Validate**: `cargo test -p notecal-core` reports ≥ 300 cases, all passing.
- **Done**: 334 cases (5 new files: currency, locale_id, division, blocks, edge). No engine bugs found; the 18 first-run mismatches were test mistakes (unannotated block lines, a total converting to the first line's currency as the PRD says, sub-cent amounts rounding to 0). Harness minimum raised to 300.

### Task 3: App icon ✅
- **Action**: Draw an original SVG icon, render `source.png`, regenerate the icon set, and check it in the tray, window and `.desktop`.
- **Validate**: You approve the icon. `tauri icon` output is committed.
- **Done**: `assets/icon.svg` (approved), rendered to `src-tauri/icons/source.png`; icon set regenerated (Android/iOS/Windows outputs removed).

### Task 4: Release workflow ✅ (written; first run needs a tag)
- **Action**: Add `release.yml` (matrix macOS universal and ubuntu-22.04, draft release, size check). Test it with a pre-release tag such as `v0.9.0-rc.1` on a branch, only with your go-ahead, since it creates a GitHub release.
- **Validate**: The workflow produces a `.dmg` of 15 MB or less and an AppImage, both attached to a draft release.

### Task 5: AUR package ✅ (built locally; publishing is yours)
- **Action**: Write `PKGBUILD`, `notecal.desktop` and `.SRCINFO`. Build locally with `makepkg` from a local tag or path source.
- **Validate**: `makepkg` succeeds on Arch, `pacman -Qip` shows an installed size of 15 MB or less, and the installed `notecal --toggle` works from `PATH`. Publishing to the AUR is your step, with the commands in the README.
- **Done**: built from this repo's `main` with `makepkg -f -d` (toolchain via rustup, so dependency checks skipped locally): package 4.3 MB, installed 10.47 MiB; `check()` ran all tests. Fixes found on the way: `makedepends` uses `cargo` (provided by `rust` and `rustup`), and `options=('!lto')` because makepkg's GCC LTO turned `ring`'s C code into bitcode the Rust linker could not read. Not yet checked: installing it (`sudo pacman -U`) and `notecal --toggle` from `PATH` — your step.

### Task 6: README, CHANGELOG, version 1.0.0 ✅
- **Action**: README covering what notecal is, a screenshot, install (AUR, AppImage, unsigned `.dmg` with the right-click → Open note), keybindings and Hyprland setup, files, settings, and credits (fend, open.er-api). Add a CHANGELOG and bump the version to 1.0.0.
- **Validate**: The docs read correctly and the version string shows everywhere.

### Task 7: Gate 2 measurements (needs you on both machines)
- **Action**: On Arch, with the release package, measure:
  - cold start (5 runs, median);
  - idle RAM and idle CPU;
  - hotkey show time;
  - keystroke p95 typed by hand.

  On the Mac, with the CI `.dmg`, measure the same, and check the global shortcut, window-state and always-on-top paths. Record everything in `docs/gate2.md`.
- **Decision for you**: typing p95. Either make it faster, for example by debouncing evaluation to the next idle frame or by moving highlighting off the critical path, or amend the budget with the measured number.
- **Validate**: `docs/gate2.md` has both machines and a pass/fail per budget. The PRD records the decision.

### Task 8: Tag v1.0.0
- **Action**: After Gate 2 passes, tag `v1.0.0` (with your go-ahead), let the release workflow build, review the draft release, publish it, and push the AUR package.
- **Validate**: The release has the artifacts, and the AUR page shows notecal 1.0.0.

## Validation

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
cd ui && bun run check && bun run test && bun run build
cd packaging/aur && makepkg -f    # local package build
```

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Fat LTO slows CI builds a lot | Medium | Use it only in the release profile; CI checks build in debug and test mode |
| AppImage built on ubuntu-22.04 misbehaves on Arch (bundled WebKitGTK) | Medium | The AUR package is the primary Linux path; the AppImage is best-effort per the PRD |
| Unsigned universal `.dmg` is blocked by Gatekeeper | Expected | Document right-click → Open and `xattr -d com.apple.quarantine` (PRD decision) |
| New golden cases expose engine bugs late | Medium | Done in Task 2, early, so fixes land before Gate 2 |
| Typing p95 still over 50 ms at Gate 2 | High | Your decision in Task 7: optimise or amend the budget |
| A test tag creates a public release | Low | Draft releases only; tags pushed only with your go-ahead |

## Acceptance

- [ ] All tasks complete
- [ ] Validation passes locally and in CI
- [ ] `.dmg` ≤ 15 MB and AUR package ≤ 15 MB, checked in CI or `makepkg`
- [ ] ≥ 300 golden cases
- [ ] `docs/gate2.md` records both machines with a decision
- [ ] v1.0.0 tagged and released
