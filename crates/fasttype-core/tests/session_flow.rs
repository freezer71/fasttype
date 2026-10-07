use fasttype_core::event::EventKind;
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{EndReason, InputOutcome, SessionState, TestSession};
use fasttype_core::sources::{RandomWords, SequenceWords};
use fasttype_core::spec::TestSpec;
use std::sync::Arc;

fn time_session(seconds: u32) -> TestSession {
    let words = Arc::new(
        ["the", "cat", "sat", "mat"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );
    let generator = WordGenerator::new(
        Box::new(RandomWords::new(words, "english", false, false, None)),
        "english",
        false,
        false,
    );
    TestSession::new(
        TestSpec::time(seconds, "english", false, false),
        generator,
        Box::new(SplitMix64::new(5)),
    )
}

fn words_session(words: &[&str]) -> TestSession {
    let seq = SequenceWords::new(words.iter().map(|s| s.to_string()).collect());
    let generator = WordGenerator::new(Box::new(seq), "english", false, false);
    TestSession::new(
        TestSpec::words(words.len() as u32, "english", false, false),
        generator,
        Box::new(SplitMix64::new(1)),
    )
}

#[test]
fn time_test_ends_exactly_at_limit() {
    let mut s = time_session(15);
    let first = s.word(0).chars().next().unwrap();
    s.insert(first, 500.0);
    assert_eq!(s.next_tick_at(), Some(1500.0));
    assert!(s.tick(15_499.0));
    assert_eq!(s.state(), SessionState::Running);
    s.tick(15_500.0);
    assert_eq!(s.end_reason(), Some(EndReason::TimeUp));
    let last = s.log().events.last().unwrap();
    assert_eq!((last.ms, &last.kind), (15_000.0, &EventKind::TimerEnd));
    assert_eq!(s.next_tick_at(), None);
}

#[test]
fn input_after_time_limit_is_ignored() {
    let mut s = time_session(15);
    s.insert(s.word(0).chars().next().unwrap(), 0.0);
    assert_eq!(s.insert('x', 16_000.0), InputOutcome::Ignored);
    assert_eq!(s.state(), SessionState::Finished);
}

#[test]
fn infinite_time_never_ends_by_itself() {
    let mut s = time_session(0);
    s.insert(s.word(0).chars().next().unwrap(), 0.0);
    s.tick(10_000_000.0);
    assert_eq!(s.state(), SessionState::Running);
}

#[test]
fn live_stats_after_first_second() {
    let mut s = words_session(&["the", "cat", "sat"]);
    for (i, ch) in "the ".chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
    s.insert('x', 400.0); // erreur
    s.tick(1000.0);
    let live = s.live_stats();
    assert_eq!(live.seconds, 1);
    assert_eq!(live.wpm, 48.0); // 4 car. justes en 1 s
    assert_eq!(live.raw, 60.0); // 5 car. tapés
    assert_eq!(live.acc, 80.0);
}

#[test]
fn live_stats_before_start_are_neutral() {
    let s = words_session(&["the"]);
    let live = s.live_stats();
    assert_eq!((live.seconds, live.wpm, live.acc), (0, 0.0, 100.0));
}

#[test]
fn zen_grows_words_and_finishes_on_demand() {
    let generator = WordGenerator::empty();
    let mut s = TestSession::new(
        TestSpec::zen("english"),
        generator,
        Box::new(SplitMix64::new(1)),
    );
    s.insert('h', 0.0);
    s.insert('i', 100.0);
    assert!(matches!(
        s.insert(' ', 200.0),
        InputOutcome::Committed { correct: true, .. }
    ));
    assert_eq!(s.words().len(), 2);
    s.finish_zen(20_000.0);
    assert_eq!(s.end_reason(), Some(EndReason::ZenFinished));
    assert!(s.log().context.target_words.is_empty());
}

#[test]
fn bail_out_marks_log() {
    let mut s = time_session(0);
    s.insert(s.word(0).chars().next().unwrap(), 0.0);
    s.bail_out(5000.0);
    assert_eq!(s.end_reason(), Some(EndReason::BailedOut));
    assert!(s.log().context.bailed_out);
}

#[test]
fn bail_out_before_start_does_nothing() {
    let mut s = time_session(30);
    s.bail_out(100.0);
    assert_eq!(s.state(), SessionState::Ready);
}

#[test]
fn ticks_are_logged_before_later_inputs() {
    let mut s = words_session(&["abc", "d"]);
    s.insert('a', 0.0);
    s.insert('b', 2500.0);
    let ms: Vec<f64> = s.log().events.iter().map(|e| e.ms).collect();
    let mut sorted = ms.clone();
    sorted.sort_by(f64::total_cmp);
    assert_eq!(ms, sorted);
}

#[test]
fn repeat_keeps_words_and_is_flagged() {
    let mut s = words_session(&["ab", "cd"]);
    for (i, ch) in "ab cd".chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
    let words = s.words().to_vec();
    let r = s.into_repeat().unwrap();
    assert_eq!(r.words(), words.as_slice());
    assert!(r.is_repeated());
    assert_eq!(r.state(), SessionState::Ready);
}

#[test]
fn zen_cannot_be_repeated() {
    let s = TestSession::new(
        TestSpec::zen("english"),
        WordGenerator::empty(),
        Box::new(SplitMix64::new(1)),
    );
    assert!(s.into_repeat().is_none());
}

#[test]
fn finished_session_produces_monkeytype_numbers() {
    let mut s = words_session(&["the", "cat"]);
    for (i, ch) in "the cat".chars().enumerate() {
        s.insert(ch, 1000.0 + i as f64 * 100.0);
    }
    let r = s.result(0).unwrap();
    assert_eq!(r.wpm, 140.0);
    assert_eq!(r.acc, 100.0);
    assert_eq!(r.char_stats, [7, 0, 0, 0]);
    assert_eq!(r.chart.wpm.len(), 1); // borne finale fractionnaire à 0,6 s
    assert_eq!(r.invalid, Some(fasttype_core::result::Invalid::TooShort));
}
