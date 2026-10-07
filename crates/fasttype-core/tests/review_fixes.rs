//! Écarts de fidélité relevés à la revue finale (comportements de Monkeytype).

mod common;

use common::steady_time_test;
use fasttype_core::generator::WordGenerator;
use fasttype_core::result::build_result;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{InputOutcome, SessionState, TestSession};
use fasttype_core::sources::SequenceWords;
use fasttype_core::spec::{QuoteMeta, TestSpec};

fn session(words: &[&str], language: &str) -> TestSession {
    let seq = SequenceWords::new(words.iter().map(|s| s.to_string()).collect());
    let generator = WordGenerator::new(Box::new(seq), language, false, false);
    TestSession::new(
        TestSpec::words(words.len() as u32, language, false, false),
        generator,
        Box::new(SplitMix64::new(1)),
    )
}

fn type_str(s: &mut TestSession, text: &str) {
    for (i, ch) in text.chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
}

#[test]
fn final_space_on_wrong_last_word_is_not_counted() {
    // helpers.ts applyInputEvent : espace qui valide un dernier mot faux → ignoré dans la saisie
    let mut s = session(&["ab", "café"], "english");
    type_str(&mut s, "ab cafe ");
    assert_eq!(s.state(), SessionState::Finished);
    let r = s.result(0).unwrap();
    assert_eq!(r.char_stats, [3, 1, 0, 0]);
    // la frappe compte quand même pour la précision : 6 justes, 2 fausses
    assert_eq!(r.acc, 75.0);
    assert_eq!(s.log().word_inputs(None)[&1], "cafe");
}

#[test]
fn typographic_equivalents_are_accepted() {
    let mut s = session(&["it’s", "ok"], "english");
    type_str(&mut s, "it's ok");
    assert_eq!(s.state(), SessionState::Finished);
    assert_eq!(s.input(0), "it’s ");
    assert_eq!(s.result(0).unwrap().char_stats, [7, 0, 0, 0]);
}

#[test]
fn unicode_spaces_commit_like_space() {
    let mut s = session(&["ab", "cd"], "english");
    s.insert('a', 0.0);
    s.insert('b', 100.0);
    assert!(matches!(
        s.insert('\u{3000}', 200.0),
        InputOutcome::Committed { correct: true, .. }
    ));
    assert_eq!(s.input(0), "ab ");
}

#[test]
fn russian_yo_matches_ye() {
    let mut s = session(&["ёж", "да"], "russian_1k");
    s.insert('е', 0.0);
    assert_eq!(s.input(0), "ё");
}

#[test]
fn repeated_quote_stays_valid() {
    // finish() : `if (isRepeated() && Config.mode === "quote") setIsRepeated(false)`
    let meta = QuoteMeta {
        id: 3,
        group: 0,
        source: "x".into(),
    };
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &TestSpec::quote(meta, "english"),
        true,
        0,
    );
    assert_eq!(r.invalid, None);
    assert!(!r.pb_eligible());
}

#[test]
fn zen_backspace_reopens_previous_word() {
    let mut s = TestSession::new(
        TestSpec::zen("english"),
        WordGenerator::empty(),
        Box::new(SplitMix64::new(1)),
    );
    type_str(&mut s, "hi ");
    assert!(s.backspace(500.0));
    assert_eq!((s.active_index(), s.input(0)), (0, "hi"));
    s.insert(' ', 600.0);
    assert_eq!(s.active_index(), 1);
    assert_eq!(
        s.words().len(),
        2,
        "pas de mot vide en double après un retour"
    );
}
