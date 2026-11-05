use criterion::{Criterion, criterion_group, criterion_main};
use notecal_core::{Settings, evaluate_note};
use std::hint::black_box;

fn single_line(c: &mut Criterion) {
    let settings = Settings::default();
    c.bench_function("single_line", |b| {
        b.iter(|| evaluate_note(black_box("5 km in miles"), &settings, None));
    });
}

criterion_group!(benches, single_line);
criterion_main!(benches);
