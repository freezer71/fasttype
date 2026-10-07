use criterion::{Criterion, criterion_group, criterion_main};
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::TestSession;
use fasttype_core::sources::RandomWords;
use fasttype_core::spec::TestSpec;
use std::hint::black_box;
use std::sync::Arc;

const ENGLISH: &[&str] = &[
    "the", "be", "of", "and", "a", "to", "in", "he", "have", "it", "that", "for", "they", "with",
    "as", "not", "on", "she", "at", "by",
];

fn session() -> TestSession {
    let words = Arc::new(ENGLISH.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let generator = WordGenerator::new(
        Box::new(RandomWords::new(words, "english", false, false, None)),
        "english",
        false,
        false,
    );
    TestSession::new(
        TestSpec::time(0, "english", false, false),
        generator,
        Box::new(SplitMix64::new(42)),
    )
}

/// Tape `count` caractères justes à 15 ms d'intervalle (~800 wpm), avec les
/// stats live à chaque tick, comme le fera l'interface.
fn type_chars(s: &mut TestSession, count: usize) {
    let mut t = 0.0;
    for _ in 0..count {
        let a = s.active_index();
        let typed = s.input(a).chars().count();
        let ch = s.word(a).chars().nth(typed).unwrap_or(' ');
        if s.tick(t) {
            black_box(s.live_stats());
        }
        s.insert(ch, t);
        t += 15.0;
    }
}

fn bench(c: &mut Criterion) {
    c.bench_function("insert_10k_keystrokes", |b| {
        b.iter(|| {
            let mut s = session();
            type_chars(&mut s, 10_000);
            black_box(s.active_index())
        })
    });
    let mut finished = session();
    type_chars(&mut finished, 10_000);
    finished.bail_out(200_000.0);
    c.bench_function("build_result_150s", |b| {
        b.iter(|| black_box(finished.result(0)))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
