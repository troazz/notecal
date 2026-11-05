//! Golden tests: every file in `tests/golden/` is a set of notes with the
//! expected result for each line.
//!
//! Format:
//! - `input => expected` asserts the displayed result.
//! - `input => expected || copy` also asserts the bare-number copy.
//! - A line without `=>` asserts that the line shows no result.
//! - `---` starts a new note. `@locale id` and `@precision N` apply to the
//!   next note only. Lines starting with `;;` are test comments, not note text.
//!
//! Every note gets a fixed rate table: USD 1, IDR 16000, EUR 0.9 per USD.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use notecal_core::{Locale, Rates, Settings, evaluate_note};

struct Expect {
    file: String,
    line_no: usize,
    input: String,
    display: Option<String>,
    copy: Option<String>,
}

struct Note {
    settings: Settings,
    lines: Vec<Expect>,
}

fn parse(file: &str, content: &str) -> Vec<Note> {
    let mut notes = Vec::new();
    let mut settings = Settings::default();
    let mut lines = Vec::new();
    for (i, raw) in content.lines().enumerate() {
        if raw.starts_with(";;") {
            continue;
        }
        if raw == "---" {
            notes.push(Note {
                settings: std::mem::take(&mut settings),
                lines: std::mem::take(&mut lines),
            });
            continue;
        }
        if let Some(directive) = raw.strip_prefix('@') {
            match directive.split_once(' ') {
                Some(("locale", "id")) => settings.locale = Locale::Id,
                Some(("locale", "en")) => settings.locale = Locale::En,
                Some(("precision", n)) => settings.precision = n.parse().expect("precision"),
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
    notes.push(Note { settings, lines });
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
            let results = evaluate_note(&text.join("\n"), &note.settings, Some(&rates));
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
        cases >= 150,
        "M1 requires at least 150 golden cases, found {cases}"
    );
    assert!(
        failures.is_empty(),
        "{} of {cases} golden cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
