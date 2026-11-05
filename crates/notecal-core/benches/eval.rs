//! Engine budgets (PRD non-functional requirements):
//! single line ≤ 200 µs median, uncached 200-line note ≤ 15 ms,
//! re-evaluation after editing the last line of that note ≤ 2 ms.

use std::collections::HashMap;
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use notecal_core::{Engine, Rates, Settings, evaluate_note};

const BLOCK: [&str; 9] = [
    "rent = 4,500,000 IDR",
    "electricity = 350,000 IDR",
    "rent + electricity",
    "20% of prev",
    "100 USD in IDR",
    "total",
    "5 km in miles",
    "1.5 GB in MB + 10%",
    "",
];

fn rates() -> Rates {
    Rates::from_per_usd(HashMap::from([
        ("USD".to_string(), 1.0),
        ("IDR".to_string(), 16000.0),
    ]))
}

fn note_200() -> String {
    let mut lines: Vec<&str> = BLOCK.iter().cycle().take(199).copied().collect();
    lines.push("3 days 4 hours in minutes");
    lines.join("\n")
}

fn engine(c: &mut Criterion) {
    let settings = Settings::default();
    let rates = rates();

    c.bench_function("single_line", |b| {
        b.iter(|| evaluate_note(black_box("5 km in miles"), &settings, Some(&rates)));
    });

    let note = note_200();
    c.bench_function("note_200_uncached", |b| {
        b.iter(|| evaluate_note(black_box(&note), &settings, Some(&rates)));
    });

    // One long block (no blank lines) with a total at the end: guards
    // against per-line cost growing with block length.
    let list: Vec<String> = (1..200).map(|i| format!("{i} IDR")).collect();
    let list = format!("{}\ntotal", list.join("\n"));
    c.bench_function("list_200_one_block", |b| {
        b.iter(|| evaluate_note(black_box(&list), &settings, Some(&rates)));
    });

    let edited = note.replace("3 days 4 hours", "3 days 5 hours");
    let mut engine = Engine::default();
    engine.evaluate(&note, &settings, Some(&rates));
    let mut flip = false;
    c.bench_function("note_200_edit_last_line", |b| {
        b.iter(|| {
            flip = !flip;
            let text = if flip { &edited } else { &note };
            engine.evaluate(black_box(text), &settings, Some(&rates))
        });
    });
}

criterion_group!(benches, engine);
criterion_main!(benches);
