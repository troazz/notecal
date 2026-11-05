//! No input may crash, hang or freeze the engine (PRD reliability, C13).

use std::time::{Duration, Instant};

use notecal_core::{Engine, LINE_TIME_LIMIT, Settings, evaluate_note};
use proptest::prelude::*;

/// Generous bound per line: the interrupt plus formatting and overhead.
const LINE_BOUND: Duration = Duration::from_millis(150);

fn atom() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9]{1,6}(\\.[0-9]{1,4})?",
        "[0-9]{1,3}(,[0-9]{1,4}){0,2}",
        Just("0xff".to_string()),
        Just("1e308".to_string()),
        Just("10^10^10".to_string()),
        Just("1000000!".to_string()),
        Just("prev".to_string()),
        Just("sum".to_string()),
        Just("avg".to_string()),
        Just("x".to_string()),
        "(km|m|kg|IDR|USD|GB|hours|°C|%|miles)",
    ]
}

fn expr() -> impl Strategy<Value = String> {
    atom().prop_recursive(4, 32, 4, |inner| {
        prop_oneof![
            (
                inner.clone(),
                "[-+*/^]| mod | of | in | to | as % of ",
                inner.clone()
            )
                .prop_map(|(a, op, b)| format!("{a} {op} {b}")),
            inner.clone().prop_map(|a| format!("({a})")),
            inner.clone().prop_map(|a| format!("{a}%")),
            inner.clone().prop_map(|a| format!("x = {a}")),
            inner.prop_map(|a| format!("{a} in hex")),
        ]
    })
}

fn line() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => expr(),
        1 => "[ -~]{0,40}",
        1 => "\\PC{0,20}",
    ]
}

fn check_note(lines: &[String]) {
    let text = lines.join("\n");
    let start = Instant::now();
    let results = evaluate_note(&text, &Settings::default(), None);
    let elapsed = start.elapsed();
    assert_eq!(results.len(), text.split('\n').count());
    assert!(
        elapsed <= LINE_BOUND * results.len() as u32,
        "{elapsed:?} for {} lines: {text:?}",
        results.len()
    );
    for (line, result) in text.split('\n').zip(&results) {
        assert!(
            !result
                .error
                .as_deref()
                .is_some_and(|e| e.starts_with("Internal error")),
            "engine panicked on {line:?}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]

    #[test]
    fn notes_never_panic_or_hang(lines in prop::collection::vec(line(), 1..8)) {
        check_note(&lines);
    }

    /// A sequence of random notes through one engine must match fresh
    /// evaluation each time, however much the text changes between calls.
    #[test]
    fn incremental_engine_matches_fresh(
        notes in prop::collection::vec(prop::collection::vec(line(), 1..6), 2..6)
    ) {
        let mut engine = Engine::default();
        let settings = Settings::default();
        for lines in notes {
            let text = lines.join("\n");
            prop_assert_eq!(
                engine.evaluate(&text, &settings, None),
                evaluate_note(&text, &settings, None),
                "{:?}", text
            );
        }
    }
}

#[test]
fn pathological_line_does_not_affect_neighbours() {
    let text = "1 + 1\n10^10^10\n1000000!\n2 + 2";
    let start = Instant::now();
    let results = evaluate_note(text, &Settings::default(), None);
    assert!(start.elapsed() < LINE_TIME_LIMIT * 2 + LINE_BOUND);
    assert_eq!(results[0].display.as_deref(), Some("2"));
    assert_eq!(results[1].display, None);
    assert_eq!(
        results[1].error.as_deref(),
        Some("This calculation took too long")
    );
    assert_eq!(results[2].display, None);
    assert_eq!(results[3].display.as_deref(), Some("4"));
}
