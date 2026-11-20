//! Examples for the in-app cheatsheet (PRD M11). The engine owns the
//! calculation language, so it owns these; a test keeps every one working.
//! An example may span lines (`a = 2\na * 3`); its result is the last line's.

/// A cheatsheet section: an id the UI translates, and its examples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub id: &'static str,
    pub examples: &'static [Example],
}

/// An example: a stable id the UI translates into a one-line explanation
/// (`cheat.<id>`), and the text inserted into the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Example {
    pub id: &'static str,
    pub text: &'static str,
}

const fn ex(id: &'static str, text: &'static str) -> Example {
    Example { id, text }
}

pub const SECTIONS: &[Section] = &[
    Section {
        id: "arithmetic",
        examples: &[
            ex("precedence", "2 + 3 * 4"),
            ex("parentheses", "(2 + 3) * 4"),
            ex("power", "2^10"),
            ex("decimals", "0.1 + 0.2"),
            ex("modulo", "17 mod 5"),
            ex("grouping", "1,234,567 / 3"),
        ],
    },
    Section {
        id: "percentages",
        examples: &[
            ex("percent-of", "20% of 150"),
            ex("percent-add", "150 + 20%"),
            ex("percent-subtract", "150 - 20%"),
            ex("as-percent", "30 as % of 120"),
        ],
    },
    Section {
        id: "variables",
        examples: &[
            ex("assign", "price = 25\nprice * 4"),
            ex("total", "rent = 1500\nfood = 600\ntotal"),
            ex("average", "x = 10\ny = 20\navg"),
            ex("prev", "12 * 3\nprev + 1"),
        ],
    },
    Section {
        id: "units",
        examples: &[
            ex("length", "5 km in miles"),
            ex("temperature", "72 fahrenheit to celsius"),
            ex("data-size", "2 GB in MB"),
            ex("speed", "60 mph in km/h"),
            ex("volume", "3 cups in mL"),
            ex("duration", "90 minutes in hours"),
        ],
    },
    Section {
        id: "currency",
        examples: &[
            ex("currency-convert", "100 USD in IDR"),
            ex("currency-to", "50 EUR to USD"),
            ex("currency-large", "1,000,000 IDR in USD"),
        ],
    },
    Section {
        id: "dates",
        examples: &[
            ex("date-add", "today + 2 weeks"),
            ex("date-diff", "2026-12-25 - today"),
            ex("date-diff-unit", "25 Dec - today in weeks"),
            ex("month-end", "31 Jan 2026 + 1 month"),
            ex("time-zone", "now in Tokyo"),
            ex("time-convert", "3pm Jakarta in London"),
        ],
    },
    Section {
        id: "css",
        examples: &[
            ex("pt-to-px", "12 pt in px"),
            ex("rem-to-px", "2 rem in px"),
            ex("px-to-rem", "24px in rem"),
        ],
    },
    Section {
        id: "bases",
        examples: &[
            ex("to-hex", "255 in hex"),
            ex("to-binary", "0xff in binary"),
            ex("binary-math", "0b1010 + 5"),
            ex("octal", "0o17 in decimal"),
        ],
    },
    Section {
        id: "functions",
        examples: &[
            ex("sqrt", "sqrt(144)"),
            ex("round", "round(3.14159)"),
            ex("abs", "abs(-7)"),
            ex("log", "log(1000)"),
            ex("floor", "floor(2.7)"),
            ex("pi", "pi * 2^2"),
        ],
    },
    Section {
        id: "comments",
        examples: &[
            ex("header", "# Budget\nrent = 1500"),
            ex("comment", "rent = 1500 // monthly"),
        ],
    },
];

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use jiff::ZonedDateTime;

    use super::*;
    use crate::{Rates, Settings, evaluate_note_at};

    #[test]
    fn every_example_has_a_result() {
        let rates = Rates::from_per_usd(HashMap::from([
            ("USD".to_string(), 1.0),
            ("IDR".to_string(), 16000.0),
            ("EUR".to_string(), 0.9),
        ]));
        let now: ZonedDateTime = "2026-11-18T10:00[Asia/Jakarta]".parse().unwrap();
        for section in SECTIONS {
            for example in section.examples {
                let example = example.text;
                let results = evaluate_note_at(example, &Settings::default(), Some(&rates), &now);
                let last = results.last().unwrap();
                assert!(
                    last.display.is_some() && last.error.is_none(),
                    "{}: {example:?} gave {last:?}",
                    section.id
                );
            }
        }
    }

    #[test]
    fn example_ids_are_unique_and_kebab_case() {
        let mut seen = HashSet::new();
        for example in SECTIONS.iter().flat_map(|s| s.examples) {
            assert!(seen.insert(example.id), "duplicate id {}", example.id);
            assert!(
                example
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not kebab-case",
                example.id
            );
        }
    }
}
