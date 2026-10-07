mod common;

use common::{LogBuilder, assert_close};
use fasttype_core::chars::CharCounts;
use fasttype_core::numbers::calculate_wpm;
use fasttype_core::spec::Mode;
use fasttype_core::stats;

#[test]
fn words_test_end_to_end_numbers() {
    // "the cat" : 7 frappes de 0 à 600 ms
    let log = LogBuilder::new(Mode::Words, false, &["the ", "cat"])
        .typ(0, "the ")
        .typ(1, "cat")
        .end(600.0);
    assert_close(stats::test_duration_ms(&log), 600.0);
    let c = stats::chars(&log, false);
    assert_eq!(c.correct_word, 7);
    assert_eq!(c.all_correct, 7);
    assert_close(calculate_wpm(f64::from(c.correct_word), 0.6), 140.0);
    assert_eq!(stats::timer_boundaries(&log), vec![600.0]);
    assert_close(stats::accuracy(&log, None).percentage, 100.0);
}

#[test]
fn timed_test_boundaries_have_no_tail() {
    let log = LogBuilder::new(Mode::Time, true, &["ab ", "cd ", "ef "])
        .at(100.0)
        .typ(0, "ab ")
        .tick(1)
        .at(1100.0)
        .typ(1, "cd")
        .tick(2)
        .end(2000.0);
    assert_eq!(stats::timer_boundaries(&log), vec![1000.0, 2000.0]);
    let c = stats::chars(&log, false);
    assert_eq!((c.all_correct, c.correct_word), (5, 5)); // crédit partiel du dernier mot
}

#[test]
fn untimed_tail_only_from_half_second() {
    let short = LogBuilder::new(Mode::Words, false, &["a"])
        .typ(0, "a")
        .end(2400.0);
    assert_eq!(stats::timer_boundaries(&short), vec![1000.0, 2000.0]);
    let long = LogBuilder::new(Mode::Words, false, &["a"])
        .typ(0, "a")
        .end(2600.0);
    assert_eq!(stats::timer_boundaries(&long), vec![1000.0, 2000.0, 2600.0]);
}

#[test]
fn accuracy_counts_corrected_mistakes() {
    let log = LogBuilder::new(Mode::Words, false, &["to"])
        .typ(0, "x")
        .backspace(0)
        .typ(0, "to")
        .end(500.0);
    let a = stats::accuracy(&log, None);
    assert_eq!((a.correct, a.incorrect), (2, 1));
    assert_close(a.percentage, 200.0 / 3.0);
    assert_eq!(
        stats::chars(&log, false),
        CharCounts {
            all_correct: 2,
            correct_word: 2,
            ..Default::default()
        }
    );
}

#[test]
fn accuracy_is_zero_without_inserts() {
    let log = LogBuilder::new(Mode::Words, false, &["to"]).end(500.0);
    assert_eq!(stats::accuracy(&log, None).percentage, 0.0);
}

#[test]
fn zen_trims_trailing_idle_under_seven_seconds() {
    let short = LogBuilder::new(Mode::Zen, false, &[])
        .typ(0, "hi ")
        .end(5000.0);
    assert_close(stats::test_duration_ms(&short), 200.0);
    let long = LogBuilder::new(Mode::Zen, false, &[])
        .typ(0, "hi ")
        .end(9000.0);
    assert_close(stats::test_duration_ms(&long), 9000.0);
    // en zen, la cible est la saisie elle-même
    assert_eq!(stats::chars(&short, false).correct_word, 3);
    assert_eq!(stats::last_keypress_to_end_ms(&short), 0.0);
}

#[test]
fn bail_out_gives_partial_credit() {
    let base = || LogBuilder::new(Mode::Words, false, &["hello ", "world"]).typ(0, "hel");
    let bailed = base().bailed_out().end(1000.0);
    assert_eq!(
        stats::chars(&bailed, false),
        CharCounts {
            all_correct: 3,
            correct_word: 3,
            ..Default::default()
        }
    );
    let normal = base().end(1000.0);
    assert_eq!(
        stats::chars(&normal, false),
        CharCounts {
            all_correct: 3,
            missed: 3,
            ..Default::default()
        }
    );
}

#[test]
fn missed_and_incorrect_on_early_space() {
    let log = LogBuilder::new(Mode::Words, false, &["hello ", "world"])
        .typ(0, "hel ")
        .typ(1, "world")
        .end(900.0);
    assert_eq!(
        stats::chars(&log, false),
        CharCounts {
            all_correct: 8,
            correct_word: 5,
            incorrect: 1,
            extra: 0,
            missed: 2
        }
    );
}

#[test]
fn start_to_first_keypress() {
    let log = LogBuilder::new(Mode::Words, false, &["a"])
        .at(250.0)
        .typ(0, "a")
        .end(400.0);
    assert_close(stats::start_to_first_keypress_ms(&log), 250.0);
    assert_close(stats::last_keypress_to_end_ms(&log), 150.0);
}

#[test]
fn custom_duration_is_not_rounded() {
    let log = LogBuilder::new(Mode::Custom, false, &["a"])
        .typ(0, "a")
        .end(1234.567);
    assert_close(stats::test_duration_ms(&log), 1234.567);
    let words = LogBuilder::new(Mode::Words, false, &["a"])
        .typ(0, "a")
        .end(1234.567);
    assert_close(stats::test_duration_ms(&words), 1230.0);
}
