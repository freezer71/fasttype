mod common;

use common::{LogBuilder, steady_time_test};
use fasttype_core::numbers::consistency;
use fasttype_core::spec::Mode;
use fasttype_core::stats;

fn two_second_test() -> fasttype_core::event::EventLog {
    LogBuilder::new(Mode::Time, true, &["ab ", "cd ", "ef "])
        .at(100.0)
        .typ(0, "ab ")
        .tick(1)
        .at(1100.0)
        .typ(1, "cd")
        .tick(2)
        .end(2000.0)
}

#[test]
fn wpm_history_is_cumulative_with_partial_last_word() {
    // 1 s : "ab " → 3 car. → 36 wpm ; 2 s : 3 + "cd" (crédit partiel) = 5 → 30 wpm
    assert_eq!(stats::wpm_history(&two_second_test()), vec![36.0, 30.0]);
}

#[test]
fn burst_history_is_per_interval() {
    assert_eq!(stats::burst_history(&two_second_test()), vec![36.0, 24.0]);
    assert_eq!(stats::keypresses_per_second(&two_second_test()), vec![3, 2]);
}

#[test]
fn error_history_counts_wrong_inserts() {
    let log = LogBuilder::new(Mode::Time, true, &["ab "])
        .at(100.0)
        .typ(0, "x")
        .tick(1)
        .tick(2)
        .end(2000.0);
    assert_eq!(stats::error_count_history(&log), vec![1, 0]);
}

#[test]
fn afk_counts_empty_seconds() {
    let log = LogBuilder::new(Mode::Time, true, &["ab "])
        .at(100.0)
        .typ(0, "ab")
        .tick(1)
        .tick(2)
        .tick(3)
        .end(3000.0);
    assert_eq!(stats::afk_duration(&log), 2);
}

#[test]
fn afk_detected_on_five_idle_seconds() {
    assert!(!stats::afk_detected(&steady_time_test(
        15, 15, "abcd ", "abcd ", 5
    )));
    assert!(stats::afk_detected(&steady_time_test(
        15, 10, "abcd ", "abcd ", 5
    )));
}

#[test]
fn afk_never_detected_after_bail_out() {
    let mut log = steady_time_test(15, 10, "abcd ", "abcd ", 5);
    log.context.bailed_out = true;
    assert!(!stats::afk_detected(&log));
}

#[test]
fn steady_typing_is_fully_consistent() {
    let log = steady_time_test(15, 15, "abcd ", "abcd ", 5);
    let burst = stats::burst_history(&log);
    assert_eq!(burst.len(), 15);
    assert!(burst.iter().all(|&b| b == 60.0));
    assert_eq!(consistency(&burst), 100.0);
}
