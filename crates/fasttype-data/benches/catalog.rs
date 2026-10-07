use criterion::{Criterion, criterion_group, criterion_main};
use fasttype_data::pack::Pack;
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/languages.pack"
    ))
    .unwrap();
    c.bench_function("parse_language_index", |b| {
        b.iter(|| black_box(Pack::parse(&bytes).unwrap().entries().len()))
    });
    c.bench_function("load_english", |b| {
        b.iter(|| black_box(fasttype_data::load_language("english").unwrap()))
    });
    let mut slow = c.benchmark_group("big");
    slow.sample_size(10);
    slow.bench_function("load_english_450k", |b| {
        b.iter(|| black_box(fasttype_data::load_language("english_450k").unwrap()))
    });
    slow.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
