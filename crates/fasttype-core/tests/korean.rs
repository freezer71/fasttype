use fasttype_core::chars::{
    CharCounts, contains_korean, count_chars_for, count_words, hangul_disassemble,
};
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{SessionState, TestSession};
use fasttype_core::sources::SequenceWords;
use fasttype_core::spec::TestSpec;

#[test]
fn syllables_split_into_jamo_with_compounds() {
    assert_eq!(hangul_disassemble("값"), "ㄱㅏㅂㅅ"); // ㅄ → ㅂㅅ
    assert_eq!(hangul_disassemble("와"), "ㅇㅗㅏ"); // ㅘ → ㅗㅏ
    assert_eq!(hangul_disassemble("한국 "), "ㅎㅏㄴㄱㅜㄱ ");
    assert_eq!(hangul_disassemble("가a"), "ㄱㅏa");
    assert_eq!(hangul_disassemble("ㄲ"), "ㄲ"); // consonne double : une seule touche
    assert_eq!(hangul_disassemble("ㄳ"), "ㄱㅅ");
}

#[test]
fn detects_korean_ranges() {
    assert!(contains_korean("한국"));
    assert!(contains_korean("ㄱ"));
    assert!(!contains_korean("hello"));
    assert!(!contains_korean("日本"));
}

#[test]
fn korean_counts_jamo() {
    let c = count_chars_for("한국 ", "한국 ", false, true);
    assert_eq!(
        c,
        CharCounts {
            all_correct: 7,
            correct_word: 7,
            ..Default::default()
        }
    );
    // syllabe incomplète en cours de frappe : préfixe en jamo, crédit partiel
    let partial = count_chars_for("하", "한 ", true, true);
    assert_eq!((partial.all_correct, partial.correct_word), (2, 2));
    assert_eq!(
        count_words([("한 ", "한 ", true)], false, true).correct_word,
        4
    );
    assert_eq!(
        count_words([("한 ", "한 ", true)], false, false).correct_word,
        2
    );
}

#[test]
fn korean_session_counts_jamo_in_result() {
    let seq = SequenceWords::new(vec!["한국".into(), "사람".into()]);
    let generator = WordGenerator::new(Box::new(seq), "korean", false, false);
    let mut s = TestSession::new(
        TestSpec::words(2, "korean", false, false),
        generator,
        Box::new(SplitMix64::new(1)),
    );
    assert!(s.log().context.korean);
    for (i, ch) in "한국 사람".chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
    assert_eq!(s.state(), SessionState::Finished);
    // 한국 + espace = 7 jamo ; 사람 = ㅅㅏㄹㅏㅁ = 5
    assert_eq!(s.result(0).unwrap().char_stats, [12, 0, 0, 0]);
}

#[test]
fn non_korean_session_is_unflagged() {
    let seq = SequenceWords::new(vec!["the".into()]);
    let generator = WordGenerator::new(Box::new(seq), "english", false, false);
    let s = TestSession::new(
        TestSpec::words(1, "english", false, false),
        generator,
        Box::new(SplitMix64::new(1)),
    );
    assert!(!s.log().context.korean);
}
