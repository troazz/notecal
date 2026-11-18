//! Calculation engine for notecal.
//!
//! [`Engine::evaluate`] takes the full text of a note and returns one
//! [`LineResult`] per line. The engine caches each line's result and the
//! evaluation state after it, so a later call only re-evaluates from the
//! first changed line downward. [`evaluate_note`] is a one-shot shortcut.
//! The `_at` variants take the current time, for dates (`today`, `now`); the
//! others read the system clock.
//!
//! Variables live in the engine's evaluation state, which belongs to one
//! note: use one `Engine` per open note.

#![forbid(unsafe_code)]

pub mod currency;
mod datetime;
mod format;
mod preprocess;

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use fend_core::{
    Context, CustomUnitAttribute, DecimalSeparatorStyle, ExchangeRateFnV2, ExchangeRateFnV2Options,
    Interrupt, evaluate_with_interrupt,
};
use jiff::ZonedDateTime;

use datetime::{DateState, Outcome};
use preprocess::LineKind;

/// How long one line may take before it is interrupted (PRD C13).
pub const LINE_TIME_LIMIT: Duration = Duration::from_millis(50);

const TOO_LONG: &str = "This calculation took too long";
/// Hover message for a currency line before any rates were fetched (PRD C9).
pub const NO_RATES: &str = "No exchange rates yet";

/// Number formatting convention for input and output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Locale {
    /// `1,234.5`: comma groups thousands, dot is the decimal separator.
    #[default]
    En,
    /// `1.234,5`: dot groups thousands, comma is the decimal separator.
    Id,
}

impl Locale {
    fn group_separator(self) -> char {
        match self {
            Locale::En => ',',
            Locale::Id => '.',
        }
    }

    fn decimal_separator(self) -> char {
        match self {
            Locale::En => '.',
            Locale::Id => ',',
        }
    }
}

/// Language of words in results: weekday and month names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    En,
    Id,
}

/// Evaluation settings that come from the user's preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Separators for numbers in input and output.
    pub locale: Locale,
    /// Maximum decimals shown for non-currency results.
    pub precision: u8,
    /// Weekday and month names in date results.
    pub language: Language,
    /// Pixels per inch for `px` (CSS uses 96).
    pub css_ppi: u16,
    /// Pixels per `em` and `rem` (the CSS default font size is 16).
    pub css_em_px: u16,
    /// The user's definitions file: `name = value` and `unit name = value`
    /// lines evaluated before every note (see [`check_definitions`]).
    pub definitions: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: Locale::En,
            precision: 6,
            language: Language::En,
            css_ppi: 96,
            css_em_px: 16,
            definitions: String::new(),
        }
    }
}

/// A line of the definitions file that could not be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionError {
    /// 1-based line number in the definitions file.
    pub line: usize,
    pub message: String,
}

/// Checks the definitions in `settings`, as the engine will load them.
///
/// Lines that only fail for lack of exchange rates are not reported: they
/// work once rates are fetched.
#[must_use]
pub fn check_definitions(settings: &Settings, rates: Option<&Rates>) -> Vec<DefinitionError> {
    build_base(settings, rates, &ZonedDateTime::now(), &mut HashMap::new())
        .1
        .into_iter()
        .filter(|e| e.message != NO_RATES)
        .collect()
}

/// Exchange rates relative to USD, keyed by ISO currency code (e.g. `"IDR"`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rates {
    per_usd: HashMap<String, f64>,
}

impl Rates {
    /// Builds a rate table where each value is units of that currency per 1 USD.
    ///
    /// Entries that are not finite and positive are dropped: they cannot
    /// convert anything, and a NaN would make two equal tables compare
    /// unequal, which defeats the engine's cache.
    #[must_use]
    pub fn from_per_usd(mut per_usd: HashMap<String, f64>) -> Self {
        per_usd.retain(|_, rate| rate.is_finite() && *rate > 0.0);
        Self { per_usd }
    }

    /// Units of `code` per 1 USD, if known.
    #[must_use]
    pub fn get(&self, code: &str) -> Option<f64> {
        self.per_usd.get(code).copied()
    }
}

impl ExchangeRateFnV2 for Rates {
    fn relative_to_base_currency(
        &self,
        currency: &str,
        _options: &ExchangeRateFnV2Options,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync + 'static>> {
        self.get(currency)
            .ok_or_else(|| format!("No exchange rate for {currency}").into())
    }
}

/// The outcome of evaluating one line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineResult {
    /// Text for the result column; `None` shows nothing.
    pub display: Option<String>,
    /// Bare number for copying: no unit, no group separators.
    pub copy: Option<String>,
    /// Why the line has no result, for the hover message only.
    pub error: Option<String>,
    /// The result changes as time passes (`now`, `today`): re-evaluate the
    /// note when the minute changes.
    pub uses_time: bool,
}

impl LineResult {
    fn error(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
            ..Self::default()
        }
    }
}

/// Evaluates every line of `text` with a fresh engine.
#[must_use]
pub fn evaluate_note(text: &str, settings: &Settings, rates: Option<&Rates>) -> Vec<LineResult> {
    Engine::default().evaluate(text, settings, rates)
}

/// [`evaluate_note`] at a given current time.
#[must_use]
pub fn evaluate_note_at(
    text: &str,
    settings: &Settings,
    rates: Option<&Rates>,
    now: &ZonedDateTime,
) -> Vec<LineResult> {
    Engine::default().evaluate_at(text, settings, rates, now)
}

/// What `prev` and totals can refer to at a given line.
#[derive(Debug, Clone, Copy)]
struct References {
    /// Some earlier line has a result, held in the hidden `_prev` variable.
    has_prev: bool,
    /// Lines counted by `sum`/`total`/`avg` in the current block; their
    /// running total is held in the hidden `_sum` variable.
    block_len: usize,
    /// False once a counted line could not be added to the running total.
    sum_valid: bool,
}

impl Default for References {
    fn default() -> Self {
        Self {
            has_prev: false,
            block_len: 0,
            sum_valid: true,
        }
    }
}

struct CachedLine {
    text: String,
    /// Whether this line's result was added to the block's running total.
    feeds_total: bool,
    result: LineResult,
    context_after: Context,
    references_after: References,
    dates_after: DateState,
}

/// Evaluates a note and remembers enough to re-evaluate it cheaply.
#[derive(Default)]
pub struct Engine {
    config: Option<(Settings, Option<Rates>)>,
    /// The state every note starts from: CSS units and the user's definitions.
    base: Option<Base>,
    lines: Vec<CachedLine>,
    /// Whether a name already means something to fend (unit, function, …).
    builtin_names: HashMap<String, bool>,
    /// The minute and time zone of the previous call: lines that use the
    /// clock are re-evaluated when either changes.
    clock: Option<(i64, jiff::tz::TimeZone)>,
}

impl Engine {
    /// [`Engine::evaluate_at`] with the system clock and time zone.
    #[must_use]
    pub fn evaluate(
        &mut self,
        text: &str,
        settings: &Settings,
        rates: Option<&Rates>,
    ) -> Vec<LineResult> {
        self.evaluate_at(text, settings, rates, &ZonedDateTime::now())
    }

    /// Evaluates every line of `text`, top to bottom, at time `now`.
    ///
    /// Returns exactly one entry per line (`text.split('\n')`). Lines above
    /// the first line that changed since the previous call are reused,
    /// except lines that hit the time limit, which are always retried, and
    /// lines that use the clock once the minute or time zone has changed.
    #[must_use]
    pub fn evaluate_at(
        &mut self,
        text: &str,
        settings: &Settings,
        rates: Option<&Rates>,
        now: &ZonedDateTime,
    ) -> Vec<LineResult> {
        let same_config = self
            .config
            .as_ref()
            .is_some_and(|(s, r)| s == settings && r.as_ref() == rates);
        if !same_config {
            self.lines.clear();
            self.builtin_names.clear();
            self.base = Some(build_base(settings, rates, now, &mut self.builtin_names).0);
            // Names were checked against the context before the definitions;
            // now that they are defined, notes must not reassign them.
            self.builtin_names.clear();
            self.config = Some((settings.clone(), rates.cloned()));
        }
        let base = self.base.as_ref().expect("set above");
        let clock = (
            now.timestamp().as_second().div_euclid(60),
            now.time_zone().clone(),
        );
        let clock_moved = self.clock.as_ref() != Some(&clock);
        self.clock = Some(clock);

        let lines: Vec<&str> = text.split('\n').collect();
        let feeds_total = preprocess::feeds_total(&lines);
        // A line is reusable when its text is the same and so is whether a
        // total below needs it (typing `total` re-evaluates the block). A
        // timed-out line is not: whether it finishes depends on machine load.
        let unchanged = self
            .lines
            .iter()
            .zip(lines.iter().zip(&feeds_total))
            .take_while(|(cached, (line, feeds))| {
                cached.text == **line
                    && cached.feeds_total == **feeds
                    && cached.result.error.as_deref() != Some(TOO_LONG)
                    && !(clock_moved && cached.result.uses_time)
            })
            .count();
        self.lines.truncate(unchanged);

        let restore = |lines: &[CachedLine]| match lines.last() {
            Some(cached) => (
                cached.context_after.clone(),
                cached.references_after,
                cached.dates_after.clone(),
            ),
            None => (
                base.context.clone(),
                References::default(),
                base.dates.clone(),
            ),
        };
        let (mut context, mut references, mut dates) = restore(&self.lines);
        for (line, &feeds) in lines.iter().zip(&feeds_total).skip(unchanged) {
            let builtin_names = &mut self.builtin_names;
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let mut line_eval = LineEval {
                    base: &base.context,
                    builtin_names,
                    settings,
                    rates,
                    now,
                };
                line_eval.run(line, feeds, &mut context, &mut references, &mut dates)
            }));
            let result = outcome.unwrap_or_else(|_| {
                // A panic may leave fend's context half-updated: restore the
                // state after the previous line.
                (context, references, dates) = restore(&self.lines);
                LineResult::error("Internal error while evaluating this line")
            });
            self.lines.push(CachedLine {
                text: (*line).to_string(),
                feeds_total: feeds,
                result,
                context_after: context.clone(),
                references_after: references,
                dates_after: dates.clone(),
            });
        }
        self.lines.iter().map(|c| c.result.clone()).collect()
    }
}

/// Evaluation state shared by every line of a note before its first line.
struct Base {
    context: Context,
    dates: DateState,
}

fn new_context(settings: &Settings, rates: Option<&Rates>) -> Context {
    let mut context = Context::new();
    if settings.locale == Locale::Id {
        context.set_decimal_separator_style(DecimalSeparatorStyle::Comma);
    }
    if let Some(rates) = rates {
        context.set_exchange_rate_handler_v2(rates.clone());
    }
    // CSS units. `pt` and `rem` replace fend's pint and radiation dose,
    // which stay available as `pint` and their full names.
    let ppi = settings.css_ppi.max(1);
    let em = settings.css_em_px.max(1);
    let none = CustomUnitAttribute::None;
    context.define_custom_unit_v1("px", "px", &format!("1/{ppi} inch"), &none);
    context.define_custom_unit_v1("pt", "pt", "1/72 inch", &none);
    context.define_custom_unit_v1("em", "em", &format!("{em} px"), &none);
    context.define_custom_unit_v1("rem", "rem", &format!("{em} px"), &none);
    context
}

/// The base context with the user's definitions evaluated into it, and the
/// definitions that failed.
fn build_base(
    settings: &Settings,
    rates: Option<&Rates>,
    now: &ZonedDateTime,
    builtin_names: &mut HashMap<String, bool>,
) -> (Base, Vec<DefinitionError>) {
    let mut context = new_context(settings, rates);
    let plain = context.clone();
    let mut dates = DateState::default();
    let mut errors = Vec::new();
    for (i, raw) in settings.definitions.split('\n').enumerate() {
        let line = raw.trim();
        if preprocess::classify(line) != LineKind::Expr {
            continue;
        }
        let line = preprocess::strip_comment(line);
        let result = if let Some(rest) = line.strip_prefix("unit ") {
            define_unit(&mut context, rest.trim())
        } else if preprocess::split_assignment(line).is_some() {
            let mut line_eval = LineEval {
                base: &plain,
                builtin_names,
                settings,
                rates,
                now,
            };
            let mut references = References::default();
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                line_eval.run(line, false, &mut context, &mut references, &mut dates)
            }));
            match outcome {
                Ok(result) => result.error.map_or(Ok(()), Err),
                Err(_) => Err("Internal error while evaluating this line".to_string()),
            }
        } else {
            Err("A definition is 'name = value' or 'unit name = value'".to_string())
        };
        if let Err(message) = result {
            errors.push(DefinitionError {
                line: i + 1,
                message,
            });
        }
    }
    (Base { context, dates }, errors)
}

/// `unit sprint = 2 weeks`: a unit that converts both ways (`6 weeks in sprints`).
fn define_unit(context: &mut Context, rest: &str) -> Result<(), String> {
    let (name, definition) = preprocess::split_assignment(rest)
        .ok_or_else(|| "A unit is defined as 'unit name = value'".to_string())?;
    if name.starts_with('_') {
        return Err(format!("'{name}' is reserved and cannot be a unit"));
    }
    // fend reports a bad definition only when the unit is used: try it first.
    run_fend(definition, &mut context.clone())?;
    let plural = if name.ends_with('s') {
        name.to_string()
    } else {
        format!("{name}s")
    };
    context.define_custom_unit_v1(
        name,
        &plural,
        &format!("({definition})"),
        &CustomUnitAttribute::None,
    );
    run_fend(&format!("1 {name}"), &mut context.clone()).map(|_| ())
}

struct Deadline(Instant);

impl Interrupt for Deadline {
    fn should_interrupt(&self) -> bool {
        Instant::now() >= self.0
    }
}

fn run_fend(input: &str, context: &mut Context) -> Result<fend_core::FendResult, String> {
    let deadline = Deadline(Instant::now() + LINE_TIME_LIMIT);
    evaluate_with_interrupt(input, context, &deadline).map_err(|e| {
        if e == "interrupted" {
            TOO_LONG.to_string()
        } else {
            e
        }
    })
}

/// Everything one line's evaluation needs besides the mutable state.
struct LineEval<'a> {
    base: &'a Context,
    builtin_names: &'a mut HashMap<String, bool>,
    settings: &'a Settings,
    rates: Option<&'a Rates>,
    now: &'a ZonedDateTime,
}

impl LineEval<'_> {
    fn run(
        &mut self,
        line: &str,
        feeds_total: bool,
        context: &mut Context,
        references: &mut References,
        dates: &mut DateState,
    ) -> LineResult {
        let line = line.trim();
        match preprocess::classify(line) {
            LineKind::Blank | LineKind::Header => {
                references.block_len = 0;
                references.sum_valid = true;
                return LineResult::default();
            }
            LineKind::Comment => return LineResult::default(),
            LineKind::Expr => {}
        }
        let line = preprocess::strip_comment(line);
        match self.evaluate_expr(line, feeds_total, context, references, dates) {
            Ok(Some(result)) => result,
            Ok(None) => LineResult::default(),
            Err(_) if self.rates.is_none() && currency::mentions_currency(line) => {
                LineResult::error(NO_RATES)
            }
            Err(message) => LineResult::error(message),
        }
    }

    /// Returns the formatted result, or `None` when fend produced no output.
    /// Updates `references` when the line has a result.
    fn evaluate_expr(
        &mut self,
        line: &str,
        feeds_total: bool,
        context: &mut Context,
        references: &mut References,
        dates: &mut DateState,
    ) -> Result<Option<LineResult>, String> {
        let assignment = preprocess::split_assignment(line);
        let mut expr = assignment.map_or(line, |(_, rhs)| rhs);
        let name = assignment.map(|(name, _)| name);
        if let Some(name) = name {
            self.check_name(name)?;
        }

        // Dates are evaluated before fend (see `datetime`). A date difference
        // comes back as a duration for fend to evaluate like any quantity.
        let fend_duration;
        let mut uses_time = false;
        if let Some(evaluated) = datetime::evaluate(expr, self.now, dates) {
            let evaluated = evaluated?;
            uses_time = evaluated.uses_time;
            match evaluated.outcome {
                Outcome::Value(value) => {
                    dates.set_date(name, &value);
                    let (display, copy) =
                        datetime::format(&value, self.now, self.settings.language);
                    return Ok(Some(LineResult {
                        display: Some(display),
                        copy: Some(copy),
                        error: None,
                        uses_time,
                    }));
                }
                Outcome::Fend(duration) => {
                    fend_duration = duration;
                    expr = &fend_duration;
                }
            }
        }
        let result = self.evaluate_fend(expr, name, feeds_total, context, references)?;
        if result.is_some() {
            dates.set_other(name);
        }
        Ok(result.map(|r| LineResult { uses_time, ..r }))
    }

    /// Evaluates a non-date expression with fend; `name` receives the result.
    fn evaluate_fend(
        &mut self,
        expr: &str,
        name: Option<&str>,
        feeds_total: bool,
        context: &mut Context,
        references: &mut References,
    ) -> Result<Option<LineResult>, String> {
        let locale = self.settings.locale;
        preprocess::check_group_separators(
            expr,
            locale.group_separator(),
            locale.decimal_separator(),
        )?;
        preprocess::check_parens(expr)?;
        preprocess::check_no_assignment(expr)?;
        let rewritten = preprocess::rewrite_references(
            expr,
            references.has_prev,
            references.block_len,
            references.sum_valid,
        )?;
        let grouped = preprocess::group_unit_divisors(&rewritten.expr);
        let fend_expr = preprocess::rewrite_percentages(&grouped);

        // `_prev = name = (expr)` stores the result for `prev` and in the
        // user's variable. Counted lines with a total below them also add it to
        // the block's running total in the same evaluation.
        let mut store = format!("{} = ", preprocess::PREV_VAR);
        if let Some(name) = name {
            store.push_str(name);
            store.push_str(" = ");
        }
        store.push('(');
        store.push_str(&fend_expr);
        store.push(')');
        let sum = preprocess::SUM_VAR;
        let prev = preprocess::PREV_VAR;
        let counted = !rewritten.aggregates;
        let output = if !counted || !feeds_total || !references.sum_valid {
            run_fend(&store, context)?
        } else if references.block_len == 0 {
            run_fend(&format!("{sum} = ({store}); {prev}"), context)?
        } else {
            match run_fend(&format!("{sum} = {sum} + ({store}); {prev}"), context) {
                Ok(output) => output,
                Err(e) if e == TOO_LONG => return Err(e),
                Err(_) => {
                    // Either the line itself failed, or its units cannot be
                    // added to the total. Evaluate it alone to tell which.
                    // Safe to repeat: if the failed run got as far as
                    // `_prev = name = (…)`, running it again stores the same
                    // values; `_sum` was not changed because its addition
                    // is what failed.
                    let output = run_fend(&store, context)?;
                    references.sum_valid = false;
                    output
                }
            }
        };
        if output.output_is_empty() {
            return Ok(None);
        }
        references.has_prev = true;
        if counted {
            references.block_len += 1;
        }

        let formatted = format::format_result(
            output.get_main_result(),
            self.settings,
            self.rates,
            asks_for_base(expr),
        );
        let result = LineResult {
            display: Some(formatted.display),
            copy: Some(formatted.copy),
            error: None,
            uses_time: false,
        };
        Ok(Some(result))
    }

    fn check_name(&mut self, name: &str) -> Result<(), String> {
        if name.starts_with('_') || preprocess::KEYWORDS.contains(&name) {
            return Err(format!("'{name}' is reserved and cannot be a variable"));
        }
        let builtin = *self
            .builtin_names
            .entry(name.to_string())
            .or_insert_with(|| run_fend(name, &mut self.base.clone()).is_ok());
        if builtin {
            return Err(format!(
                "'{name}' is already a unit, function or constant; pick another name"
            ));
        }
        Ok(())
    }
}

/// True when the expression converts to a number base, whose digits the
/// formatter must leave alone.
fn asks_for_base(expr: &str) -> bool {
    let words: Vec<String> = expr
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.windows(2).any(|pair| {
        matches!(pair[0].as_str(), "in" | "to")
            && matches!(
                pair[1].as_str(),
                "hex" | "hexadecimal" | "binary" | "bin" | "octal" | "oct" | "base"
            )
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn currency_without_rates_says_so() {
        for line in ["100 USD in IDR", "5 EUR", "rent = 4,500,000 IDR"] {
            let result = &evaluate_note(line, &Settings::default(), None)[0];
            assert_eq!(result.display, None, "{line}");
            assert_eq!(result.error.as_deref(), Some(NO_RATES), "{line}");
        }
        // Lines without a currency are unaffected.
        let plain = &evaluate_note("5 km in miles", &Settings::default(), None)[0];
        assert!(plain.display.is_some());
    }

    use super::*;

    #[test]
    fn returns_one_result_per_line() {
        let results = evaluate_note("1\n\n2", &Settings::default(), None);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn empty_note_has_one_line() {
        assert_eq!(evaluate_note("", &Settings::default(), None).len(), 1);
    }

    #[test]
    fn incremental_matches_fresh_evaluation() {
        let settings = Settings::default();
        let edits = [
            "a = 2\nb = a * 3\nsum\n\nprev + 1",
            "a = 5\nb = a * 3\nsum\n\nprev + 1",
            "a = 5\nb = a * 3\nsum\n\nprev + 1\navg",
            "a = 5\nb = a * 3\n\nsum\nprev + 1",
            "a = 5",
            "",
            "x = 1\ny = x + 1\ny * 10",
            // typing a total below must re-evaluate the lines it adds up
            "1\n2\n3",
            "1\n2\n3\ntotal",
            "1\n2\n3\ntotal\n4",
            "1\n2\n3\ntotal\n4\navg",
            "1\n2\n\n3\ntotal",
        ];
        let mut engine = Engine::default();
        for text in edits {
            assert_eq!(
                engine.evaluate(text, &settings, None),
                evaluate_note(text, &settings, None),
                "after editing to {text:?}"
            );
        }
    }

    #[test]
    fn settings_change_invalidates_cache() {
        let mut engine = Engine::default();
        let en = Settings::default();
        let id = Settings {
            locale: Locale::Id,
            ..Settings::default()
        };
        let text = "1/4";
        assert_eq!(
            engine.evaluate(text, &en, None)[0].display.as_deref(),
            Some("0.25")
        );
        assert_eq!(
            engine.evaluate(text, &id, None)[0].display.as_deref(),
            Some("0,25")
        );
    }

    #[test]
    fn clock_lines_refresh_when_the_minute_changes() {
        let zone = jiff::tz::TimeZone::get("Asia/Jakarta").unwrap();
        let at = |minute| {
            jiff::civil::Date::new(2026, 11, 18)
                .unwrap()
                .at(10, minute, 0, 0)
                .to_zoned_datetime(zone.clone())
                .unwrap()
        };
        let settings = Settings::default();
        let text = "x = 2\nnow\nx * 3";
        let mut engine = Engine::default();

        let first = engine.evaluate_at(text, &settings, None, &at(0));
        assert_eq!(first[1].display.as_deref(), Some("10:00"));
        assert!(first[1].uses_time && !first[0].uses_time && !first[2].uses_time);

        let later = engine.evaluate_at(text, &settings, None, &at(1));
        assert_eq!(later[1].display.as_deref(), Some("10:01"));
        assert_eq!(later[2].display.as_deref(), Some("6"));
        assert_eq!(engine.evaluate_at(text, &settings, None, &at(1)), later);
    }

    #[test]
    fn check_definitions_reports_bad_lines_by_number() {
        let settings = Settings {
            definitions: "# header\nvat = 11%\n1 + 1\nunit = 5\nunit week2 = 2 parsecs/\nfx = 100 EUR in USD\n"
                .to_string(),
            ..Settings::default()
        };
        let errors = check_definitions(&settings, None);
        let lines: Vec<usize> = errors.iter().map(|e| e.line).collect();
        // Line 6 needs exchange rates, which is not reported as an error.
        assert_eq!(lines, [3, 4, 5], "{errors:?}");
    }
}
