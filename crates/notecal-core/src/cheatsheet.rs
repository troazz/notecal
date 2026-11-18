//! Examples for the in-app cheatsheet (PRD M11). The engine owns the
//! calculation language, so it owns these; a test keeps every one working.
//! An example may span lines (`a = 2\na * 3`); its result is the last line's.

/// A cheatsheet section: an id the UI translates, and its examples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub id: &'static str,
    pub examples: &'static [&'static str],
}

pub const SECTIONS: &[Section] = &[
    Section {
        id: "arithmetic",
        examples: &[
            "2 + 3 * 4",
            "(2 + 3) * 4",
            "2^10",
            "0.1 + 0.2",
            "17 mod 5",
            "1,234,567 / 3",
        ],
    },
    Section {
        id: "percentages",
        examples: &["20% of 150", "150 + 20%", "150 - 20%", "30 as % of 120"],
    },
    Section {
        id: "variables",
        examples: &[
            "price = 25\nprice * 4",
            "rent = 1500\nfood = 600\ntotal",
            "x = 10\ny = 20\navg",
            "12 * 3\nprev + 1",
        ],
    },
    Section {
        id: "units",
        examples: &[
            "5 km in miles",
            "72 fahrenheit to celsius",
            "2 GB in MB",
            "60 mph in km/h",
            "3 cups in mL",
            "90 minutes in hours",
        ],
    },
    Section {
        id: "currency",
        examples: &["100 USD in IDR", "50 EUR to USD", "1,000,000 IDR in USD"],
    },
    Section {
        id: "dates",
        examples: &[
            "today + 2 weeks",
            "2026-12-25 - today",
            "25 Dec - today in weeks",
            "31 Jan 2026 + 1 month",
            "now in Tokyo",
            "3pm Jakarta in London",
        ],
    },
    Section {
        id: "css",
        examples: &["12 pt in px", "2 rem in px", "24px in rem"],
    },
    Section {
        id: "bases",
        examples: &[
            "255 in hex",
            "0xff in binary",
            "0b1010 + 5",
            "0o17 in decimal",
        ],
    },
    Section {
        id: "functions",
        examples: &[
            "sqrt(144)",
            "round(3.14159)",
            "abs(-7)",
            "log(1000)",
            "floor(2.7)",
            "pi * 2^2",
        ],
    },
    Section {
        id: "comments",
        examples: &["# Budget\nrent = 1500", "rent = 1500 // monthly"],
    },
];

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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
}
