//! Calculation engine for notecal.
//!
//! [`evaluate_note`] takes the full text of a note and returns one
//! [`LineResult`] per line. Each call is independent: variables live only
//! for the duration of one call, which scopes them to a single note.

use std::collections::HashMap;

/// Number formatting convention for input and output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Locale {
    /// `1,234.5`: comma groups thousands, dot is the decimal separator.
    #[default]
    En,
    /// `1.234,5`: dot groups thousands, comma is the decimal separator.
    Id,
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

/// Evaluates every line of `text`, top to bottom.
///
/// The returned vector has exactly one entry per line of `text`.
pub fn evaluate_note(text: &str, settings: &Settings, rates: Option<&Rates>) -> Vec<LineResult> {
    let _ = (settings, rates);
    text.split('\n').map(|_| LineResult::default()).collect()
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
}
