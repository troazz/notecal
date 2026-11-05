//! Calculation engine for notecal.
//!
//! [`Engine::evaluate`] takes the full text of a note and returns one
//! [`LineResult`] per line. The engine caches each line's result and the
//! evaluation state after it, so a later call only re-evaluates from the
//! first changed line downward. [`evaluate_note`] is a one-shot shortcut.
//!
//! Variables live in the engine's evaluation state, which belongs to one
//! note: use one `Engine` per open note.

#![forbid(unsafe_code)]

mod format;
mod preprocess;

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use fend_core::{
    Context, DecimalSeparatorStyle, ExchangeRateFnV2, ExchangeRateFnV2Options, Interrupt,
    evaluate_with_interrupt,
};

use preprocess::LineKind;

/// How long one line may take before it is interrupted (PRD C13).
pub const LINE_TIME_LIMIT: Duration = Duration::from_millis(50);

const TOO_LONG: &str = "This calculation took too long";

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

/// Evaluation settings that come from the user's preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// Separators for numbers in input and output.
    pub locale: Locale,
    /// Maximum decimals shown for non-currency results.
    pub precision: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: Locale::En,
            precision: 6,
        }
    }
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
}

/// Evaluates a note and remembers enough to re-evaluate it cheaply.
#[derive(Default)]
pub struct Engine {
    config: Option<(Settings, Option<Rates>)>,
    base: Option<Context>,
    lines: Vec<CachedLine>,
    /// Whether a name already means something to fend (unit, function, …).
    builtin_names: HashMap<String, bool>,
}

impl Engine {
    /// Evaluates every line of `text`, top to bottom.
    ///
    /// Returns exactly one entry per line (`text.split('\n')`). Lines above
    /// the first line that changed since the previous call are reused,
    /// except lines that hit the time limit, which are always retried.
    #[must_use]
    pub fn evaluate(
        &mut self,
        text: &str,
        settings: &Settings,
        rates: Option<&Rates>,
    ) -> Vec<LineResult> {
        let same_config = self
            .config
            .as_ref()
            .is_some_and(|(s, r)| s == settings && r.as_ref() == rates);
        if !same_config {
            self.lines.clear();
            self.builtin_names.clear();
            self.base = Some(new_context(settings, rates));
            self.config = Some((settings.clone(), rates.cloned()));
        }
        let base = self.base.as_ref().expect("set above");

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
            })
            .count();
        self.lines.truncate(unchanged);

        let (mut context, mut references) = match self.lines.last() {
            Some(cached) => (cached.context_after.clone(), cached.references_after),
            None => (base.clone(), References::default()),
        };
        for (line, &feeds) in lines.iter().zip(&feeds_total).skip(unchanged) {
            let builtin_names = &mut self.builtin_names;
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let mut line_eval = LineEval {
                    base,
                    builtin_names,
                    settings,
                    rates,
                };
                line_eval.run(line, feeds, &mut context, &mut references)
            }));
            let result = outcome.unwrap_or_else(|_| {
                // A panic may leave fend's context half-updated: restore the
                // state after the previous line.
                (context, references) = match self.lines.last() {
                    Some(cached) => (cached.context_after.clone(), cached.references_after),
                    None => (base.clone(), References::default()),
                };
                LineResult::error("Internal error while evaluating this line")
            });
            self.lines.push(CachedLine {
                text: (*line).to_string(),
                feeds_total: feeds,
                result,
                context_after: context.clone(),
                references_after: references,
            });
        }
        self.lines.iter().map(|c| c.result.clone()).collect()
    }
}

fn new_context(settings: &Settings, rates: Option<&Rates>) -> Context {
    let mut context = Context::new();
    if settings.locale == Locale::Id {
        context.set_decimal_separator_style(DecimalSeparatorStyle::Comma);
    }
    if let Some(rates) = rates {
        context.set_exchange_rate_handler_v2(rates.clone());
    }
    context
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
}

impl LineEval<'_> {
    fn run(
        &mut self,
        line: &str,
        feeds_total: bool,
        context: &mut Context,
        references: &mut References,
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
        match self.evaluate_expr(line, feeds_total, context, references) {
            Ok(Some(result)) => result,
            Ok(None) => LineResult::default(),
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
    ) -> Result<Option<LineResult>, String> {
        let assignment = preprocess::split_assignment(line);
        let expr = assignment.map_or(line, |(_, rhs)| rhs);
        if let Some((name, _)) = assignment {
            self.check_name(name)?;
        }
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
        if let Some((name, _)) = assignment {
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
            precision: 6,
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
}
