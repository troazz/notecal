# Plan: Dates and time zones (M6), with the M7–M10 roadmap

**Source PRD**: `docs/notecal.prd.md`
**Selected Milestone**: 6 — Dates and time zones (detailed below); M7–M10 outlined
**Complexity**: Large (roadmap); M6 Medium–Large

## Summary

Close the feature gaps found in the comparison with similar calculator apps (2026-11-18). The scope, settled with the author:
- In: dates and time zones (C11), CSS units, custom definitions, a command-line tool (A7), UI languages, and a Windows build.
- Out: crypto prices and Apple notarization.

1.0.0 is released once M6–M10 are all done (see Decisions).

## Roadmap

| # | Milestone | Outcome | Size |
|---|---|---|---|
| 6 | Dates and time zones | C11: `today`, `now`, date arithmetic with calendar units, differences between dates, time zone conversion by IANA name, city or abbreviation; time-dependent lines refresh | ~2 w |
| 7 | CSS units and custom definitions | `px`, `pt`, `em`, `rem` with PPI and em size in settings (CSS meaning wins over pint and radiation rem); `~/.config/notecal/definitions.txt` loaded before every note | ~1 w |
| 8 | Command-line tool | A7: `notecal-cli "5 km in miles"`, whole notes from stdin or by name, the same settings, rates cache and definitions as the app; an Omarchy menu entry and a Raycast/Alfred script; packaged in the PKGBUILD and the cask | ~1 w |
| 9 | UI languages | Interface strings in a catalog, English and Indonesian, `language = "auto"` from the system; engine messages translated where they are ours | ~1 w |
| 10 | Windows | Build, test and package for Windows (NSIS installer in the release workflow); Windows paths for notes, settings and rates; atomic saves that survive file locks; plus a documented iCloud Drive notes folder on macOS | ~2 w |

Order: dates first (the biggest gap and the most-used of the missing features). CSS units and definitions share the engine's base context, so they go next. The CLI then gets all engine features at once. Languages and Windows touch the whole app, so they come last, when strings and paths stop moving.

## Key facts (checked 2026-11-18)

- fend-core 1.5.9 parses `today` and `tomorrow`, but `Context::set_current_time_v1` is a no-op: its body is commented out, so `today` cannot evaluate. fend has no time zones. Dates need our own layer.
- fend's `month` is 1/12 of a year (~30.44 days) and `year` is 365.25 days. Calendar arithmetic (`today + 1 month` lands on the same day-of-month) has to be ours.
- fend `pt` is a pint and `rem` is a radiation dose. fend has no `px` or `em`.
- The engine caches each line with the fend `Context` after it (`CachedLine.context_after`). A line is re-evaluated when its text changes or when it timed out. Time-dependent lines need a third reason: the clock moved.
- The PRD already lists C11 (dates, P2) and A7 (CLI, P2). Its layout reserves `crates/notecal-cli/`.
- The settings and rates-cache code lives in `src-tauri/src/{settings,rates}.rs`. The CLI needs the read side of both without Tauri.

---

## M6 detail: Dates and time zones

### What users can write

| Input | Result (now = Wed 18 Nov 2026 10:00 in Asia/Jakarta) |
|---|---|
| `today` | `Wed, 18 Nov 2026` |
| `now` / `time` | `10:00` (`10:00 WIB` when a zone is involved) |
| `tomorrow`, `yesterday` | `Thu, 19 Nov 2026`, `Tue, 17 Nov 2026` |
| `today + 2 weeks`, `today - 3 days` | `Wed, 2 Dec 2026`, `Sun, 15 Nov 2026` |
| `31 Jan 2026 + 1 month` | `Sat, 28 Feb 2026` (calendar month, clamped) |
| `2026-12-25 - today` | `37 days` |
| `25 Dec - today in weeks` | `5.285714 weeks` |
| `now in Tokyo`, `time in PST` | `12:00 JST`, `19:00 PST (Tue)` |
| `3pm Jakarta in London` | `08:00 GMT` |
| `deadline = 2026-12-25` then `deadline - today` | `37 days` |
| `today + 10 days` with `@locale id` | `Sab, 28 Nov 2026` — day and month names follow the UI language once M9 lands; English until then |

Dates and times are their own values. They don't mix with `sum`/`total` (a date line in a block is skipped by totals, like a header). `prev` refers to them.

### Design

- **New module `crates/notecal-core/src/datetime.rs`**, built on the `jiff` crate.
  - jiff has calendar-aware spans and reads the IANA time zone database: the system copy on Linux and macOS, a bundled copy on Windows, which M10 needs.
  - It parses and evaluates a small grammar: `date-term ((+|-) duration)*`, `date - date [in unit]`, and `time-term [zone] in zone`.
  - Date terms: `today`, `now`, `time`, `tomorrow`, `yesterday`, ISO dates, `25 Dec [2026]`, `Dec 25`, `3pm`, `15:30`, and date variables.
  - Durations: `N years|months|weeks|days|hours|minutes|seconds`, combinable (`1 week 2 days`).
- **Engine hook.** `LineEval::evaluate_expr` first tries `datetime::evaluate(expr, &clock, &date_vars)`.
  - No match: it falls through to fend unchanged, so existing behaviour and the golden suite stay as they are.
  - A match: it returns a `LineResult`, and assignments store the value in a per-note date-variable map, kept in `CachedLine` like `references_after`.
- **Clock and the cache.**
  - `Engine::evaluate` takes `now` through a new `Clock { now, zone }` field on `Settings`. The engine stays pure: no system calls in `notecal-core`.
  - `LineResult` gains `uses_time`.
  - The cache treats a `uses_time` line as stale when the clock's minute differs from when it was evaluated, so lines above it are still reused.
- **Zones.**
  - IANA names directly (`Asia/Jakarta`).
  - A curated alias table, `datetime/zones.rs`, with about 150 major cities and countries, plus abbreviations that mean one thing (WIB, WITA, WIT, JST, KST, IST→Asia/Kolkata, CET, CEST, BST, GMT, UTC, PST/PDT, MST, CST, EST/EDT, AEST, …). Ambiguous abbreviations resolve to the most common meaning, and the documentation says so.
- **App.**
  - The evaluator fills the clock from the system: `jiff::Zoned::now()` and the system zone.
  - The UI re-evaluates the note once a minute, on the minute, only while the note has `uses_time` lines and the window is visible.
  - The response gains `uses_time`, mirroring `uses_currency`.
- **Highlighting and autocomplete.** Date keywords and zone aliases are highlighted as units and added to completion.
- **Golden harness.** New directives: `@now 2026-11-18T10:00:00` and `@zone Asia/Jakarta`. A new file, `tests/golden/datetime.txt`, holds at least 60 cases, covering:
  - month-end clamping and leap years;
  - DST transitions (America/New_York, Europe/London);
  - `id` locale output;
  - errors (`today + 5 kg`, an unknown zone).

### Tasks

#### Task 1: datetime module and grammar ✅
- **Action**: add `jiff` to notecal-core. Write `datetime.rs` with the parser, evaluator and formatter, and its unit tests: one test per grammar form, month-end, leap day, DST gap and overlap.
- **Mirror**: the hand-written tokenizer in `preprocess.rs` (`tokenize`, `Tok`); errors as `Result<_, String>` with user-facing messages; tests inline in `mod tests`.
- **Validate**: `cargo test -p notecal-core datetime`
- **Done**: `src/datetime.rs` (tokenizer, grammar, calendar arithmetic, formatting) on jiff 1.0.1; 12 unit tests. Weekday is shown for a time on another day than the local date (`now in New York` → `Tue 22:00 EST`).

#### Task 2: Engine integration and the clock ✅
- **Action**: add `Clock` to `Settings`, try dates before fend in `evaluate_expr`, and store date variables per line in `CachedLine`. Add `uses_time`, and make stale-minute lines re-evaluate. Totals skip date lines.
- **Mirror**: `References` and how `CachedLine` carries state forward; the `TOO_LONG` re-evaluation rule in `Engine::evaluate`.
- **Validate**: engine unit tests for cache reuse across a minute change; `cargo test -p notecal-core`.
- **Done**: `Engine::evaluate_at` and `evaluate_note_at` take `now`; `evaluate` uses the system clock. The clock is not in `Settings` (that would clear the whole cache every minute); the engine remembers the last minute and zone instead. `DateState` (date variables, date `prev`, names the note assigned) is carried in `CachedLine`. A user variable named `time`/`now` keeps its meaning.

#### Task 3: Zones and aliases ✅
- **Action**: write the `zones.rs` alias table, case-insensitive with multi-word names (`New York`, `Los Angeles`). Unknown names give "Unknown time zone 'X'".
- **Validate**: unit tests for every alias resolving to a zone jiff knows.
- **Done**: `src/datetime/zones.rs`, 200+ aliases, plus `UTC±H[:MM]` offsets; a test checks every alias resolves.

#### Task 4: Golden cases ✅
- **Action**: add the `@now`/`@zone` directives to `tests/golden.rs`, and `datetime.txt` with at least 60 cases. Raise the harness minimum.
- **Mirror**: the existing `@locale` / `@precision` directive handling.
- **Validate**: `cargo test -p notecal-core --test golden`
- **Done**: `datetime.txt`, 91 cases (suite 425); harness minimum 400. The 4 first-run failures were test mistakes (`a`/`b` are units in fend; London is on BST on 24 Oct).

#### Task 5: App wiring ✅
- **Action**: build the clock in `evaluator.rs`, add `uses_time` to the response, and add a minute ticker in the UI while visible and needed. Add highlighting tokens and completion entries.
- **Mirror**: how `uses_currency` flows from `evaluator.rs` to the UI and its re-evaluation on `rates-updated`.
- **Validate**: `scripts/check.sh`. By hand: `now` updates within a minute of the clock changing, and stops updating while the window is hidden.
- **Done**: `uses_time` in `EvalResponse`; the UI re-evaluates just after each minute while the window is visible and on focus. Highlighting needed no change (date words already show as units); date words added to completion. Not yet checked by hand in the running app.

#### Task 6: Docs and performance ✅
- **Action**: README "What it does" and an examples section, CHANGELOG, and PRD C11 marked done. Re-run the engine benches.
- **Validate**: `note_200_uncached` stays within 10% of its M5 number, and keystroke cost is unchanged for notes without dates.
- **Done**: benches unchanged or better than M5 (`note_200_uncached` 12.5 ms vs 12.9, `single_line` 124 µs vs 135, last-line edit 124 µs vs 130). README, CHANGELOG, PRD row updated.

### Validation

```bash
scripts/check.sh
cargo bench -p notecal-core -- --quick
```

### Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| The date grammar captures lines meant for fend (`5 min`, `may`, `march 3`) | Medium | Dates only match whole-line forms starting with a date term or ending in a zone conversion. Every existing golden case must keep its result. |
| Ambiguous abbreviations (CST, IST, BST) | High | Pick one documented meaning each; IANA names and cities are always unambiguous |
| Per-minute re-evaluation costs battery | Low | Only lines marked `uses_time`, only while visible, aligned to the minute |
| jiff adds size | Low | ~0.3–0.5 MB; system tzdb on Linux and macOS. Check the `.dmg` stays well under the 15 MB budget. |
| Date order in `id` (`25/12`) vs `en` | Medium | Accept ISO and month-name forms only, and no numeric `dd/mm` vs `mm/dd`, which avoids the ambiguity |

### Acceptance (M6)

- [ ] All six tasks complete; `scripts/check.sh` passes
- [ ] At least 60 date golden cases; existing golden cases unchanged
- [ ] Benches within budget; `.dmg` still ≤ 15 MB
- [ ] PRD C11 marked done; README and CHANGELOG updated

---

## M7–M10 outlines (each gets its own detailed plan when started)

**M7 — CSS units and custom definitions**
- **Spike first:** check that the base context can define `px`, `pt`, `em`, `rem` as units that convert (`12pt in px`, `2rem in px`) and that override fend's `pt` and `rem`. If fend variables can't act as units, the rewriting goes in the preprocessor.
- **Settings:** `css_ppi` (default 96) and `css_em_px` (default 16).
- **Definitions file:** `definitions.txt` uses note syntax (variables, `f = x: …` functions, units built from existing units).
  - It's evaluated into the base context when settings load and when the file changes.
  - Errors are listed in Settings like other settings problems.
  - Names defined there can't be reassigned in notes.
- **PRD change:** the non-goal "variables shared across notes" becomes "only through the definitions file".

**M8 — Command-line tool**
- **Shared crate:** move the read side of settings and the rates cache, plus their paths, from `src-tauri` into a small `notecal-env` crate used by the app and the CLI.
- **The `crates/notecal-cli` binary `notecal-cli` (no GUI dependencies):**
  - `notecal-cli EXPR…` evaluates an expression.
  - `notecal-cli -` or a pipe evaluates a whole note and prints `line  =>  result`.
  - `notecal-cli --note NAME` evaluates a saved note.
  - `--copy` prints the bare number.
  - It uses the cached rates and never fetches them.
- **Packaging:** the PKGBUILD installs it; the cask adds a `binary` stanza.
- **Launchers:** an Omarchy menu extension example (copies the result) and a Raycast script command.

**M9 — UI languages**
- **Catalog:** `ui/src/i18n/{en,id}.ts` with typed keys, and every UI string moved into it. The tray menu labels in `shell.rs` get a matching Rust table.
- **Setting:** `language = "auto" | "en" | "id"`, where auto follows `LANG` or the macOS preference.
- **Engine messages:** notecal's own messages are translated. fend's errors stay English, and that's documented.
- **Contributors:** a README note on adding a language.

**M10 — Windows (and iCloud Drive)**
- **CI:** add `windows-latest` to `ci.yml` (Rust and UI tests) and to `release.yml` (NSIS installer, size check). A winget/Scoop manifest comes later.
- **Code:**
  - Windows paths come from the `dirs`-based locations, so they should mostly work already.
  - Atomic save on Windows retries a rename that a sync client or antivirus is holding.
  - The global shortcut and tray are used as-is.
- **Testing:** a manual pass on a Windows machine or VM is the author's step.
- **Unsigned installer:** SmartScreen will warn, and the README covers "More info → Run anyway".
- **iCloud Drive on macOS:** document a notes folder in iCloud Drive, and handle evicted `.name.txt.icloud` placeholders: list them as notes and download on open (`brctl download`), or show "not downloaded".

## Decisions (2026-11-18)

- **Release:** 1.0.0 ships after M6–M10 are done. The early `v1.0.0` tag and its draft release were deleted. The rc tags stay as build checks. Milestones are not released one by one.
- **Weekday and month names:** English by default. M9 can localize them.
