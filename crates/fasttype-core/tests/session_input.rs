use fasttype_core::event::EventKind;
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{EndReason, InputOutcome, SessionState, TestSession};
use fasttype_core::sources::SequenceWords;
use fasttype_core::spec::TestSpec;

fn session(words: &[&str]) -> TestSession {
    let seq = SequenceWords::new(words.iter().map(|s| s.to_string()).collect());
    let generator = WordGenerator::new(Box::new(seq), "english", false, false);
    TestSession::new(
        TestSpec::words(words.len() as u32, "english", false, false),
        generator,
        Box::new(SplitMix64::new(1)),
    )
}

fn type_str(s: &mut TestSession, text: &str, mut t: f64) -> f64 {
    for ch in text.chars() {
        s.insert(ch, t);
        t += 100.0;
    }
    t
}

#[test]
fn last_word_has_no_separator() {
    let s = session(&["the", "cat"]);
    assert_eq!(s.words(), ["the ", "cat"]);
    assert_eq!(s.state(), SessionState::Ready);
}

#[test]
fn space_on_empty_word_is_ignored_and_does_not_start() {
    let mut s = session(&["the", "cat"]);
    assert_eq!(s.insert(' ', 0.0), InputOutcome::Ignored);
    assert_eq!(s.state(), SessionState::Ready);
}

#[test]
fn first_insert_starts_test_at_zero() {
    let mut s = session(&["the", "cat"]);
    s.key_down(84, 1000.0);
    assert_eq!(
        s.insert('t', 1000.0),
        InputOutcome::Inserted { correct: true }
    );
    assert_eq!(s.state(), SessionState::Running);
    let kinds: Vec<_> = s
        .log()
        .events
        .iter()
        .map(|e| (e.ms, e.kind.clone()))
        .collect();
    assert_eq!(kinds[0], (0.0, EventKind::TimerStart));
    assert_eq!(kinds[1], (0.0, EventKind::KeyDown { code: 84 }));
    assert!(matches!(kinds[2], (0.0, EventKind::Insert { ch: 't', .. })));
}

#[test]
fn typing_everything_finishes_on_last_exact_letter() {
    let mut s = session(&["the", "cat"]);
    let t = type_str(&mut s, "the ca", 1000.0);
    assert_eq!(s.insert('t', t), InputOutcome::Finished);
    assert_eq!(s.end_reason(), Some(EndReason::Completed));
    assert_eq!(s.state(), SessionState::Finished);
}

#[test]
fn space_on_last_word_finishes_even_if_wrong() {
    let mut s = session(&["the", "cat"]);
    let t = type_str(&mut s, "the cx", 0.0);
    assert_eq!(s.insert(' ', t), InputOutcome::Finished);
}

#[test]
fn wrong_word_is_committed_and_reported() {
    let mut s = session(&["the", "cat", "sat"]);
    let t = type_str(&mut s, "thx", 0.0);
    match s.insert(' ', t) {
        InputOutcome::Committed { correct, burst } => {
            assert!(!correct);
            assert_eq!(burst, 160.0); // "thx " = 4 car. de 0 à 300 ms → 4/5/(0,3/60)
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(s.active_index(), 1);
    assert!(s.is_committed(0));
    assert_eq!(s.input(0), "thx ");
}

#[test]
fn backspace_returns_to_wrong_previous_word_only() {
    let mut s = session(&["ab", "cd", "ef"]);
    type_str(&mut s, "ax ", 0.0);
    assert!(s.backspace(400.0));
    assert_eq!((s.active_index(), s.input(0)), (0, "ax"));

    let mut s = session(&["ab", "cd", "ef"]);
    type_str(&mut s, "ab ", 0.0);
    assert!(!s.backspace(400.0));
    assert_eq!(s.active_index(), 1);
}

#[test]
fn backspace_at_very_start_is_refused() {
    let mut s = session(&["ab", "cd"]);
    assert!(!s.backspace(0.0));
    s.insert('a', 0.0);
    assert!(s.backspace(100.0));
    assert!(!s.backspace(200.0));
    assert!(!s.backspace(300.0));
    assert_eq!(s.active_index(), 0);
}

#[test]
fn delete_word_clears_current_then_previous_wrong_word() {
    let mut s = session(&["ab", "cd", "ef"]);
    type_str(&mut s, "ax cd", 0.0);
    assert!(s.delete_word(600.0));
    assert_eq!(s.input(1), "");
    assert!(s.delete_word(700.0));
    assert_eq!((s.active_index(), s.input(0)), (0, ""));
}

#[test]
fn input_is_capped_at_target_plus_twenty() {
    let mut s = session(&["ab", "cd"]);
    for i in 0..30 {
        s.insert('z', f64::from(i));
    }
    assert_eq!(s.input(0).chars().count(), 23); // "ab " = 3 car. + 20
}

#[test]
fn newline_rejected_without_newlines_in_text() {
    let mut s = session(&["ab", "cd"]);
    s.insert('a', 0.0);
    assert_eq!(s.insert('\n', 10.0), InputOutcome::Ignored);
}

#[test]
fn newline_commits_quote_line() {
    let mut s = session(&["Hi", "there\n", "you"]);
    let t = type_str(&mut s, "Hi there", 0.0);
    assert!(matches!(
        s.insert('\n', t),
        InputOutcome::Committed { correct: true, .. }
    ));
    assert_eq!(s.active_index(), 2);
}

#[test]
fn input_after_finish_is_ignored() {
    let mut s = session(&["a"]);
    assert_eq!(s.insert('a', 0.0), InputOutcome::Finished);
    assert_eq!(s.insert('b', 100.0), InputOutcome::Ignored);
    assert!(!s.backspace(200.0));
}

#[test]
fn empty_word_list_never_panics() {
    let mut s = session(&[]);
    assert!(s.words().is_empty());
    assert_eq!(s.insert('a', 0.0), InputOutcome::Ignored);
    assert!(!s.backspace(0.0));
    assert!(!s.delete_word(0.0));
}

#[test]
fn clock_going_backwards_is_clamped() {
    let mut s = session(&["abc", "d"]);
    s.insert('a', 1000.0);
    s.insert('b', 900.0);
    assert!(s.log().events.iter().all(|e| e.ms >= 0.0));
}
