//! Golden tests: every file in `tests/golden/` is a set of notes with the
//! expected result for each line.
//!
//! Format:
//! - `input => expected` asserts the displayed result.
//! - `input => expected || copy` also asserts the bare-number copy.
//! - A line without `=>` asserts that the line shows no result.
//! - `---` starts a new note. `@locale id`, `@precision N`, `@now
//!   YYYY-MM-DDTHH:MM`, `@zone Area/City`, `@ppi N`, `@em N` and `@define
//!   <definitions line>` apply to the next note only.
//!   Lines starting with `;;` are test comments, not note text.
//!
//! Every note gets a fixed rate table (USD 1, IDR 16000, EUR 0.9 per USD) and
//! a fixed clock: Wed 18 Nov 2026 10:00 in Asia/Jakarta unless overridden.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use jiff::ZonedDateTime;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use notecal_core::{Locale, Rates, Settings, evaluate_note_at};

const DEFAULT_NOW: &str = "2026-11-18T10:00";
const DEFAULT_ZONE: &str = "Asia/Jakarta";

struct Expect {
    file: String,
    line_no: usize,
    input: String,
    display: Option<String>,
    copy: Option<String>,
}

struct Note {
    settings: Settings,
    now: ZonedDateTime,
    lines: Vec<Expect>,
}

fn clock(now: &str, zone: &str) -> ZonedDateTime {
    let datetime: DateTime = now.parse().expect("@now is YYYY-MM-DDTHH:MM");
    datetime
        .to_zoned_datetime(TimeZone::get(zone).expect("@zone is an IANA name"))
        .expect("valid local time")
}

fn parse(file: &str, content: &str) -> Vec<Note> {
    let mut notes = Vec::new();
    let mut settings = Settings::default();
    let mut now = DEFAULT_NOW.to_string();
    let mut zone = DEFAULT_ZONE.to_string();
    let mut lines = Vec::new();
    for (i, raw) in content.lines().enumerate() {
        if raw.starts_with(";;") {
            continue;
        }
        if raw == "---" {
            notes.push(Note {
                settings: std::mem::take(&mut settings),
                now: clock(&now, &zone),
                lines: std::mem::take(&mut lines),
            });
            now = DEFAULT_NOW.to_string();
            zone = DEFAULT_ZONE.to_string();
            continue;
        }
        if let Some(line) = raw
            .strip_prefix("@define ")
            .or((raw == "@define").then_some(""))
        {
            settings.definitions.push_str(line);
            settings.definitions.push('\n');
            continue;
        }
        if let Some(directive) = raw.strip_prefix('@') {
            match directive.split_once(' ') {
                Some(("locale", "id")) => settings.locale = Locale::Id,
                Some(("locale", "en")) => settings.locale = Locale::En,
                Some(("precision", n)) => settings.precision = n.parse().expect("precision"),
                Some(("now", value)) => now = value.to_string(),
                Some(("zone", value)) => zone = value.to_string(),
                Some(("ppi", n)) => settings.css_ppi = n.parse().expect("ppi"),
                Some(("em", n)) => settings.css_em_px = n.parse().expect("em"),
                _ => panic!("{file}:{}: unknown directive {raw}", i + 1),
            }
            continue;
        }
        let (input, display, copy) = match raw.split_once(" => ") {
            Some((input, expected)) => {
                let (display, copy) = match expected.split_once(" || ") {
                    Some((d, c)) => (d.trim(), Some(c.trim().to_string())),
                    None => (expected.trim(), None),
                };
                (
                    input.trim_end().to_string(),
                    Some(display.to_string()),
                    copy,
                )
            }
            None => (raw.to_string(), None, None),
        };
        lines.push(Expect {
            file: file.to_string(),
            line_no: i + 1,
            input,
            display,
            copy,
        });
    }
    notes.push(Note {
        settings,
        now: clock(&now, &zone),
        lines,
    });
    notes
}

#[test]
fn golden() {
    let rates = Rates::from_per_usd(HashMap::from([
        ("USD".to_string(), 1.0),
        ("IDR".to_string(), 16000.0),
        ("EUR".to_string(), 0.9),
    ]));
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();

    let mut failures = Vec::new();
    let mut cases = 0;
    for path in paths
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
    {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        for note in parse(&name, &fs::read_to_string(path).unwrap()) {
            let text: Vec<&str> = note.lines.iter().map(|l| l.input.as_str()).collect();
            let results =
                evaluate_note_at(&text.join("\n"), &note.settings, Some(&rates), &note.now);
            for (expect, got) in note.lines.iter().zip(&results) {
                if expect.input.trim().is_empty() {
                    continue;
                }
                cases += 1;
                let display_ok = got.display == expect.display;
                let copy_ok = expect.copy.is_none() || got.copy == expect.copy;
                if !(display_ok && copy_ok) {
                    failures.push(format!(
                        "{}:{}: {:?}\n    expected {:?}{}\n    got      {:?} (copy {:?}, error {:?})",
                        expect.file, expect.line_no, expect.input, expect.display,
                        expect.copy.as_ref().map(|c| format!(" copy {c:?}")).unwrap_or_default(),
                        got.display, got.copy, got.error,
                    ));
                }
            }
        }
    }
    println!("golden cases: {cases}");
    assert!(
        cases >= 400,
        "the suite has at least 400 golden cases (PRD M5 and M6), found {cases}"
    );
    assert!(
        failures.is_empty(),
        "{} of {cases} golden cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
