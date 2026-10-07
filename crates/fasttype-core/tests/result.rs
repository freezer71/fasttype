mod common;

use common::{LogBuilder, steady_time_test};
use fasttype_core::result::{Invalid, build_result, remove_language_size};
use fasttype_core::spec::{Mode, QuoteMeta, TestSpec};

fn time15() -> TestSpec {
    TestSpec::time(15, "english", false, false)
}

#[test]
fn steady_valid_test() {
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &time15(),
        false,
        42,
    );
    assert_eq!(r.invalid, None);
    assert_eq!(r.wpm, 60.0);
    assert_eq!(r.raw, 60.0);
    assert_eq!(r.acc, 100.0);
    assert_eq!(r.consistency, 100.0);
    assert_eq!(r.char_stats, [75, 0, 0, 0]);
    assert_eq!(r.test_duration, 15.0);
    assert_eq!(r.chart.wpm.len(), 15);
    assert_eq!(r.timestamp, 42);
    assert!(r.is_saveable());
    assert!(r.pb_eligible());
}

#[test]
fn too_short_for_short_time_modes() {
    let r = build_result(
        &steady_time_test(10, 10, "abcd ", "abcd ", 5),
        &TestSpec::time(10, "english", false, false),
        false,
        0,
    );
    assert_eq!(r.invalid, Some(Invalid::TooShort));
}

#[test]
fn too_short_under_one_second() {
    let log = LogBuilder::new(Mode::Words, false, &["ab ", "cd"])
        .typ(0, "ab ")
        .typ(1, "cd")
        .end(400.0);
    let r = build_result(
        &log,
        &TestSpec::words(25, "english", false, false),
        false,
        0,
    );
    assert_eq!(r.invalid, Some(Invalid::TooShort));
}

#[test]
fn afk_detected_invalidates() {
    let r = build_result(
        &steady_time_test(15, 10, "abcd ", "abcd ", 5),
        &time15(),
        false,
        0,
    );
    assert!(r.afk_detected);
    assert_eq!(r.invalid, Some(Invalid::Afk));
}

#[test]
fn repeated_invalidates() {
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &time15(),
        true,
        0,
    );
    assert_eq!(r.invalid, Some(Invalid::Repeated));
}

#[test]
fn inhuman_speed_invalidates() {
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 30),
        &time15(),
        false,
        0,
    );
    assert_eq!(r.wpm, 360.0);
    assert_eq!(r.invalid, Some(Invalid::Wpm));
}

#[test]
fn low_accuracy_invalidates() {
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "xxcd ", 5),
        &time15(),
        false,
        0,
    );
    assert_eq!(r.acc, 60.0);
    assert_eq!(r.invalid, Some(Invalid::Accuracy));
}

#[test]
fn pb_key_ignores_speed_but_not_settings() {
    let a = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &time15(),
        false,
        0,
    );
    let b = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 6),
        &time15(),
        false,
        0,
    );
    assert_eq!(a.pb_key(), b.pb_key());
    let c = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &TestSpec::time(15, "english", true, false),
        false,
        0,
    );
    assert_ne!(a.pb_key(), c.pb_key());
}

#[test]
fn quotes_drop_language_size_and_are_not_pb_eligible() {
    let meta = QuoteMeta {
        id: 7,
        group: 1,
        source: "x".into(),
    };
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &TestSpec::quote(meta, "english_1k"),
        false,
        0,
    );
    assert_eq!(r.language, "english");
    assert_eq!((r.quote_id, r.quote_length), (Some(7), Some(1)));
    assert!(!r.pb_eligible());
}

#[test]
fn language_size_suffix() {
    assert_eq!(remove_language_size("english_1k"), "english");
    assert_eq!(remove_language_size("french_600k"), "french");
    assert_eq!(remove_language_size("code_javascript"), "code_javascript");
    assert_eq!(remove_language_size("english"), "english");
}

#[test]
fn long_infinite_test_builds_result() {
    let mut spec = TestSpec::time(0, "english", false, false);
    spec.mode2 = "0".into();
    let mut log = steady_time_test(320, 320, "abcd ", "abcd ", 5);
    log.context.bailed_out = true;
    let r = build_result(&log, &spec, false, 0);
    // bail out : le temps mort final (< 7 s) est retiré → fin à 319,83 s → 319 secondes pleines
    assert_eq!(r.chart.wpm.len(), 319);
    assert!(r.wpm > 0.0);
    assert!(!r.pb_eligible()); // bail out
}

#[test]
fn result_serializes() {
    let r = build_result(
        &steady_time_test(15, 15, "abcd ", "abcd ", 5),
        &time15(),
        false,
        1,
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"mode\":\"time\""));
    assert_eq!(
        serde_json::from_str::<fasttype_core::result::TestResult>(&json).unwrap(),
        r
    );
}
