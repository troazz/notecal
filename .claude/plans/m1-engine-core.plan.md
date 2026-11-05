# Plan: M1 — Engine core (`notecal-core`)

**Source PRD**: `docs/notecal.prd.md`
**Selected Milestone**: 1 — Engine core (~2.5 w)
**Complexity**: Large

## Summary

Build `crates/notecal-core`: a pure Rust library that takes a whole note and returns one result per line. It wraps fend-core 1.5.9 and adds a small preprocessor for Numi syntax: percentages, `prev`/`sum`/`total`/`avg`, comments and headers, locale separators, and display formatting. It covers C1–C8, C10, C12 and C13 plus the PRD's Language rules. The milestone is done when ≥ 150 golden cases pass, a property test shows no input panics or hangs, and criterion benches meet the engine budgets (≤ 40 µs per line median, ≤ 8 ms per 200-line note).

## Requirements restated

| Item | Done means |
|---|---|
| C1, C2, C3, C10 | Delegated to fend; golden cases prove each listed operator, unit family and function |
| C4 percentages | The five rows of the PRD percent table pass exactly |
| C5 variables | Per-note scope; reassignment works; names that fend already knows (units, functions, constants) or our keywords are rejected with a hover reason |
| C6 references | `prev`, `sum`/`total`, `avg` follow the PRD block rules, including unit conversion to the first counted line and "no result" on incompatible dimensions |
| C7 comments/errors | `#` headers and `//` comments give no result; failures give `display: None` and `error: Some(reason)` |
| C8 number formats | `0xff`, `0b101`, `0o17`, `1e3`, and `in hex/binary/octal` |
| C12 separators | `en` and `id` locales for input and output, per PRD rules |
| C13 time limit | Each line is interrupted after 50 ms; the rest of the note still evaluates |
| Display | Currencies 2 dp; others up to `precision` (default 6), trailing zeros trimmed; group separators |
| Robustness | A panic in one line affects only that line |

Out of scope for M1: fetching currency rates (M4), IPC and threading (M2), and a per-line result cache. The cache is only added if Task 9's benches miss the budget.

## Key facts from fend-core 1.5.9 (checked in source)

- `evaluate_with_interrupt(input, &mut Context, &impl Interrupt)`, where `Interrupt::should_interrupt(&self) -> bool`. This is how C13 is enforced.
- `Context` implements `Clone`, and fend stores assignments (`a = 5`) itself. Per-note variables come free.
- `Context::set_decimal_separator_style(Dot | Comma)` exists. It covers part or all of C12; Task 2 confirms how much.
- `set_exchange_rate_handler_v2` takes rates from the host. M1 feeds it a fixed in-memory table so currency rules are testable without the network.
- The crate uses edition 2024, so Rust ≥ 1.85 is required.

## Design

Public API (the whole surface):

```rust
pub enum Locale { En, Id }
pub struct Settings { pub locale: Locale, pub precision: u8 }      // Default: En, 6
pub struct Rates { /* base USD, code -> rate */ }
pub struct LineResult { pub display: Option<String>, pub copy: Option<String>, pub error: Option<String> }

pub fn evaluate_note(text: &str, settings: &Settings, rates: Option<&Rates>) -> Vec<LineResult>;
```

It is one stateless function: it creates a fresh fend `Context` per call, so variables are scoped to the note. If the cache is ever needed, it stays an internal detail behind the same function.

Per line, top to bottom:

1. **Classify.** blank → block break; `#…` → header (block break, no result); `//…` → comment (no result).
2. **Rewrite** (preprocessor, token-based with paren depth, not regex over raw text):
   - Group separators per locale, using the PRD comma rule (unless fend already handles it; see Task 2).
   - Percentages: `A + B%` → `A * (1 + B/100)`, `A - B%` → `A * (1 - B/100)`, `B% of A` → `(B/100) * A`, `A as % of B` → `(A / B) to %`.
   - References: every line with a result is stored in a hidden fend variable `_r<n>`. `prev` → `_r<k>` (nearest line above with a result). `sum`/`total` → `(_r<a> + _r<b> + …)` over the counted lines of the current block. `avg` → `(sum) / count`. fend's `+` converts to the left operand's unit, so it converts to the first line's unit for free. Incompatible units make fend error, which becomes "no result".
   - Assignment `name = expr`: reject if `name` is a keyword (`prev sum total avg in to of as`), starts with `_`, or already evaluates in a fresh `Context` (a fend built-in). Otherwise evaluate `name = (expr)` and set `_r<n> = name`.
3. **Evaluate** with a deadline interrupt (50 ms) inside `catch_unwind`. On a panic, restore the context snapshot taken before the line.
4. **Format.** Take fend's main result string and apply precision, trimming and group separators. `copy` holds the bare number with no unit and no separators.

## Patterns to Mirror

| Category | Source | Pattern |
|---|---|---|
| Naming | — | No existing code; greenfield repo. Use standard Rust naming (`snake_case` modules, `CamelCase` types). |
| Errors | — | None to mirror. Engine never returns `Err` to callers; failures become `LineResult { display: None, error: Some(..) }`. |
| Tests | — | None to mirror. Golden data files plus one harness test, described in Task 3. |

## Files to Change

| File | Action | Why |
|---|---|---|
| `Cargo.toml` | CREATE | Workspace root (`members = ["crates/*"]`), shared lint settings |
| `rust-toolchain.toml` | CREATE | Pin stable channel with `clippy` and `rustfmt` for local and CI parity |
| `crates/notecal-core/Cargo.toml` | CREATE | Deps: `fend-core = "1.5.9"`; dev-deps: `proptest`, `criterion` |
| `crates/notecal-core/src/lib.rs` | CREATE | Public API, `evaluate_note` loop |
| `crates/notecal-core/src/preprocess.rs` | CREATE | Line classification, tokenizer, percent/reference/separator rewrites |
| `crates/notecal-core/src/format.rs` | CREATE | Display and copy formatting |
| `crates/notecal-core/src/interrupt.rs` | CREATE | Deadline `Interrupt` implementation |
| `crates/notecal-core/tests/golden.rs` | CREATE | Golden harness |
| `crates/notecal-core/tests/golden/*.txt` | CREATE | Golden cases, one file per area (arith, units, percent, refs, vars, locale, formats, functions, errors) |
| `crates/notecal-core/tests/robustness.rs` | CREATE | Property tests: no panic, bounded time |
| `crates/notecal-core/benches/eval.rs` | CREATE | Criterion: single line, 200-line note |
| `.github/workflows/ci.yml` | CREATE | `fmt`, `clippy -D warnings`, `test` on push and PR |
| `docs/notecal.prd.md` | UPDATE | Milestone 1 row → in-progress (done by `/plan`) |

## Tasks

### Task 0: Toolchain
- **Action**: Install Rust via rustup (`sudo pacman -S rustup && rustup default stable`), or via mise if you prefer. Add `rust-toolchain.toml` (`channel = "stable"`, components `clippy`, `rustfmt`).
- **Validate**: `cargo --version` reports ≥ 1.85.

### Task 1: Workspace skeleton
- **Action**: Create the workspace and `notecal-core` with the public types and an `evaluate_note` that returns one empty result per line. Add the CI workflow.
- **Validate**: `cargo test -p notecal-core` passes; `cargo clippy --all-targets -- -D warnings` is clean.

### Task 2: fend behaviour spike (week-1 risk check)
- **Action**: Write throwaway tests that run raw fend on the risky inputs: `50 + 10%`, `20% of 50`, `5 as % of 20`, `4,500,000`, `4.500.000` (Comma style), `0xff in binary`, `5 km in miles`, `10^10^10` with a 50 ms interrupt, a 200-line note's timing, and user `a = 5` then `a * 2`. Record what fend does natively. Remove each preprocessor rewrite that fend already makes unnecessary.
- **Validate**: The findings are written as a short table at the top of `preprocess.rs`. Scope changes (rewrites dropped or added) are reported before Task 4.

### Task 3: Golden harness
- **Action**: File format: each line is `input => expected` (`=>` with nothing after it means no result). `---` starts a new note, and `@locale id` / `@precision 2` directives apply to the following note. The harness runs each note through `evaluate_note` and reports every mismatch with file:line.
- **Validate**: A seed file with 10 C1/C2 cases passes; a deliberately wrong case fails with a readable diff.

### Task 4: Comments, headers, blocks, errors (C7)
- **Action**: Line classification, the `_r<n>` result variables, and error capture into `LineResult.error`.
- **Validate**: Golden `errors.txt` and `comments.txt` pass.

### Task 5: Percentages (C4)
- **Action**: Tokenizer with paren depth, plus the four percent rewrites at top level and inside parentheses.
- **Validate**: Golden `percent.txt` (PRD table plus nested and unit cases such as `20% of 4,500,000 IDR`) passes.

### Task 6: Variables and references (C5, C6)
- **Action**: Reserved-name check, assignment rewrite, `prev`/`sum`/`total`/`avg` with block rules, and the fixed `Rates` table wired to fend's exchange-rate handler for currency cases.
- **Validate**: Golden `vars.txt` and `refs.txt` pass, including: the PRD example note, mixed km/m sum in km, an IDR+km sum with no result, `sum` lines not counted twice, reassignment, and `m = 5` rejected.

### Task 7: Locale and display formatting (C12)
- **Action**: Input separator handling per Task 2 findings; `format.rs` for precision, trimming, currency 2 dp, group separators, and the `copy` string.
- **Validate**: Golden `locale.txt` (en and id), `format.txt` (`1/3`, `2/3 IDR`, `0.1 + 0.2`, large numbers) pass.

### Task 8: Units, functions, number formats (C1–C3, C8, C10)
- **Action**: Mostly golden coverage of fend behaviour; fix any formatting gaps it exposes. Bring the total golden count to ≥ 150.
- **Validate**: `cargo test -p notecal-core --test golden` passes and prints a case count ≥ 150.

### Task 9: Time limit, panic isolation, robustness (C13)
- **Action**: Deadline interrupt, `catch_unwind` with context restore, and proptest generators for random strings and grammar-shaped expressions (deep nesting, huge exponents, factorials).
- **Validate**: `tests/robustness.rs` passes: no panic escapes, every line returns within 100 ms, and a pathological line leaves neighbouring lines' results intact.

### Task 10: Benchmarks
- **Action**: Criterion benches for one typical line and a realistic 200-line note (mix of units, currency, percentages and references). If either misses its budget, profile first. Add the per-line cache only if the profile shows re-evaluation is the cost.
- **Validate**: `cargo bench -p notecal-core` reports single line ≤ 40 µs median and 200-line note ≤ 8 ms on the Arch machine.

## Validation

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test -p notecal-core
cargo test -p notecal-core --test golden -- --nocapture   # prints case count
cargo bench -p notecal-core                               # manual budget check
```

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| fend is slower than 40 µs per line (Context clone per line, unit lookups) | Medium | Task 2 measures in week 1; snapshot only before assignment lines; per-line cache as fallback (Task 10) |
| fend's interrupt is not checked inside some long operation, so the 50 ms limit is missed | Medium | Task 2 and Task 9 test huge exponents and factorials; if a gap exists, reject such literals in the preprocessor before evaluation |
| Percent rewrites misfire on edge cases (`10% * 3`, `50 + 10% of 20`) | Medium | Token-based rewriting with paren depth; every ambiguous form gets a golden case and a PRD clarification if needed |
| fend output strings are awkward to reformat (approximations, unit plurals) | Medium | Task 2 records the shapes; formatter works from fend's output plus a dp conversion if fend supports one |
| `_r<n>` hidden variables leak into completions or clash with user names | Low | Names starting with `_` are reserved and rejected for users |
| Rust toolchain not installed | Certain | Task 0 |

## Acceptance

- [ ] All tasks complete
- [ ] Validation passes in CI and locally
- [ ] ≥ 150 golden cases pass, including every row in the PRD Language rules
- [ ] Property test: no panic, no line over 100 ms
- [ ] Benches meet ≤ 40 µs per line and ≤ 8 ms per 200-line note
- [ ] Patterns mirrored, not reinvented (greenfield: conventions above become the patterns for M2+)

## Task 2 findings (fend-core 1.5.9, release build, Arch machine, 2026-11-05)

| Area | fend native behaviour | Preprocessor needed? |
|---|---|---|
| C1, C2, C10 | All correct (`0.1 + 0.2` → `0.3`, `2(3+4)` → `14`, `7 mod 3`, trig, `sqrt`, `round`…) | No |
| C3 units | All families correct; `in` and `to` both work; `20 C in F` → `68 °F` | No |
| C8 formats | `0xff in binary` → `11111111`, `255 in hex` → `ff`, `0b101`, `0o17`, `1e3` | No |
| `20% of 50`, `10%`, `10% * 3` | `10`, `10%`, `30%` | No |
| `50 + 10%` / `50 - 10%` | `50.1` / `49.9` (percent read as /100) | **Yes** |
| `5 as % of 20` | Parse error | **Yes** (`(5/20) to %` works natively) |
| Variables | `a = 5` then `a * 2` works | No |
| Reserved names | fend allows `m = 5` and `e = 2` (shadows unit/constant) | **Yes**, reject |
| `prev`, `sum`, `total`, `avg` | Unknown identifier | **Yes** |
| `#` lines | Empty result | No (classify anyway for blocks) |
| `//` lines | Error | **Yes** |
| C12 input, `en` | `4,500,000` → `4500000` natively | No |
| C12 input, `en`, bad group | `1,5` → `15` (silently wrong) | **Yes**, reject |
| C12 input, `id` (Comma style) | `4.500.000` and `1,5 + 1` → `2,5` natively | No |
| Output | No group separators; `approx. ` prefix; full precision (`3.1068559612`) | **Yes** (format.rs) |
| Huge output | `2^2^2^2^2` prints ~20k digits, 10 ms | **Yes**, cap length → scientific/no result |
| Currency | Handler returning units-per-USD gives correct direction (`100 USD in IDR` → `1600000 IDR`); `1 km + 1 IDR` errors | No |
| C13 interrupt | `10^10^10` and `1000000!` stop at 50.1 ms | No |
| Context clone | Clone per line adds ~5% (11.9 vs 11.3 ms per 200 lines) | Snapshot per line is fine |

**Speed (budget miss).** Single line: 8–60 µs for arithmetic, 85–200 µs for unit conversions; mean ~120 µs. A 200-line mixed note: ~11.3 ms. Both miss the PRD engine budgets (40 µs, 8 ms). The cost is inside fend's evaluation, not context setup.
