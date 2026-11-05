//! Calculation engine for notecal.
//!
//! [`Engine::evaluate`] takes the full text of a note and returns one
//! [`LineResult`] per line. The engine caches each line's result and the
//! evaluation state after it, so a later call only re-evaluates from the
//! first changed line downward. [`evaluate_note`] is a one-shot shortcut.
//!
//! Variables live in the engine's evaluation state, which belongs to one
//! note: use one `Engine` per open note.

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
}

/// Evaluation settings that come from the user's preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
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
    pub fn from_per_usd(per_usd: HashMap<String, f64>) -> Self {
        Self { per_usd }
    }

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
pub fn evaluate_note(text: &str, settings: &Settings, rates: Option<&Rates>) -> Vec<LineResult> {
    Engine::default().evaluate(text, settings, rates)
}

/// Which earlier lines `prev` and totals refer to.
#[derive(Debug, Clone, Default)]
struct References {
    /// Nearest line above with a result.
    last: Option<usize>,
    /// Lines counted by `sum`/`total`/`avg` in the current block.
    block: Vec<usize>,
}

struct CachedLine {
    text: String,
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
    /// the first line that changed since the previous call are reused.
    pub fn evaluate(
        &mut self,
        text: &str,
        settings: &Settings,
        rates: Option<&Rates>,
    ) -> Vec<LineResult> {
        let config = (settings.clone(), rates.cloned());
        if self.config.as_ref() != Some(&config) {
            self.lines.clear();
            self.builtin_names.clear();
            self.base = Some(new_context(settings, rates));
            self.config = Some(config);
        }
        let base = self.base.as_ref().expect("set above");

        let lines: Vec<&str> = text.split('\n').collect();
        let unchanged = self
            .lines
            .iter()
            .zip(&lines)
            .take_while(|(cached, line)| cached.text == **line)
            .count();
        self.lines.truncate(unchanged);

        let (mut context, mut references) = match self.lines.last() {
            Some(cached) => (
                cached.context_after.clone(),
                cached.references_after.clone(),
            ),
            None => (base.clone(), References::default()),
        };
        for (index, line) in lines.iter().enumerate().skip(unchanged) {
            let before = (context.clone(), references.clone());
            let builtin_names = &mut self.builtin_names;
            let outcome = catch_unwind(AssertUnwindSafe(|| {
                let mut line_eval = LineEval {
                    base,
                    builtin_names,
                    settings,
                    rates,
                };
                line_eval.run(index, line, &mut context, &mut references)
            }));
            let result = outcome.unwrap_or_else(|_| {
                (context, references) = before;
                LineResult::error("Internal error while evaluating this line")
            });
            self.lines.push(CachedLine {
                text: (*line).to_string(),
                result,
                context_after: context.clone(),
                references_after: references.clone(),
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
            "This calculation took too long".to_string()
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
        index: usize,
        line: &str,
        context: &mut Context,
        references: &mut References,
    ) -> LineResult {
        let line = line.trim();
        match preprocess::classify(line) {
            LineKind::Blank | LineKind::Header => {
                references.block.clear();
                return LineResult::default();
            }
            LineKind::Comment => return LineResult::default(),
            LineKind::Expr => {}
        }
        match self.evaluate_expr(index, line, context, references) {
            Ok(Some((result, aggregates))) => {
                references.last = Some(index);
                if !aggregates {
                    references.block.push(index);
                }
                result
            }
            Ok(None) => LineResult::default(),
            Err(message) => LineResult::error(message),
        }
    }

    /// Returns the formatted result and whether the line used a total, or
    /// `None` when fend produced no output.
    fn evaluate_expr(
        &mut self,
        index: usize,
        line: &str,
        context: &mut Context,
        references: &References,
    ) -> Result<Option<(LineResult, bool)>, String> {
        let assignment = preprocess::split_assignment(line);
        let expr = assignment.map_or(line, |(_, rhs)| rhs);
        if let Some((name, _)) = assignment {
            self.check_name(name)?;
        }
        preprocess::check_group_separators(expr, self.settings.locale.group_separator())?;
        preprocess::check_parens(expr)?;
        let rewritten = preprocess::rewrite_references(expr, references.last, &references.block)?;
        let fend_expr =
            preprocess::rewrite_percent_change(&preprocess::rewrite_as_percent_of(&rewritten.expr));

        let hidden = preprocess::result_var(index);
        let input = match assignment {
            Some((name, _)) => format!("{name} = ({fend_expr})"),
            None => format!("{hidden} = ({fend_expr})"),
        };
        let output = run_fend(&input, context)?;
        if output.output_is_empty() {
            return Ok(None);
        }
        if let Some((name, _)) = assignment {
            run_fend(&format!("{hidden} = {name}"), context)?;
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
        Ok(Some((result, rewritten.aggregates)))
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
