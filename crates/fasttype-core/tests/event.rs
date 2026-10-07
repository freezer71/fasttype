use fasttype_core::event::{EventContext, EventKind, EventLog, TestEvent, active_word_index};
use fasttype_core::spec::{CustomLimit, Mode, TestSpec};

fn log() -> EventLog {
    let ctx = EventContext {
        mode: Mode::Words,
        timed: false,
        bailed_out: false,
        target_words: vec!["ab ".into(), "cd".into()],
    };
    let mut log = EventLog::with_capacity(ctx, 16);
    log.push(0.0, EventKind::TimerStart);
    for (i, ch) in "ab ".chars().enumerate() {
        log.push(
            i as f64 * 100.0,
            EventKind::Insert {
                word_index: 0,
                char_index: i as u32,
                ch,
                correct: true,
            },
        );
    }
    log.push(
        300.0,
        EventKind::Insert {
            word_index: 1,
            char_index: 0,
            ch: 'x',
            correct: false,
        },
    );
    log.push(400.0, EventKind::DeleteChar { word_index: 1 });
    log.push(
        500.0,
        EventKind::Insert {
            word_index: 1,
            char_index: 0,
            ch: 'c',
            correct: true,
        },
    );
    log
}

#[test]
fn replays_inserts_and_deletes() {
    let inputs = log().word_inputs(None);
    assert_eq!(inputs[&0], "ab ");
    assert_eq!(inputs[&1], "c");
}

#[test]
fn replay_stops_at_given_time() {
    let inputs = log().word_inputs(Some(300.0));
    assert_eq!(inputs[&1], "x");
}

#[test]
fn delete_word_clears_input() {
    let mut l = log();
    l.push(600.0, EventKind::DeleteWord { word_index: 1 });
    assert_eq!(l.word_inputs(None)[&1], "");
}

#[test]
fn active_word_follows_committed_space() {
    let l = log();
    assert_eq!(active_word_index(&l.word_inputs(Some(250.0))), 1); // "ab " validé
    assert_eq!(active_word_index(&l.word_inputs(Some(150.0))), 0);
    assert_eq!(active_word_index(&Default::default()), 0);
}

#[test]
fn target_lookup() {
    let l = log();
    assert_eq!(l.target(1), Some("cd"));
    assert_eq!(l.target(5), None);
}

#[test]
fn event_serializes_with_type_tag() {
    let e = TestEvent {
        ms: 12.5,
        kind: EventKind::Insert {
            word_index: 0,
            char_index: 1,
            ch: 'é',
            correct: true,
        },
    };
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains("\"type\":\"insert\""), "{json}");
    assert_eq!(serde_json::from_str::<TestEvent>(&json).unwrap(), e);
}

#[test]
fn timed_matches_is_timed_test() {
    assert!(TestSpec::time(30, "english", false, false).is_timed());
    assert!(TestSpec::words(0, "english", false, false).is_timed());
    assert!(!TestSpec::words(25, "english", false, false).is_timed());
    assert!(TestSpec::custom(CustomLimit::Time(60), "english", false, false).is_timed());
    assert!(TestSpec::custom(CustomLimit::Word(0), "english", false, false).is_timed());
    assert!(!TestSpec::custom(CustomLimit::Word(50), "english", false, false).is_timed());
    assert!(!TestSpec::zen("english").is_timed());
}

#[test]
fn long_tests_refuse_quick_restart() {
    assert!(TestSpec::time(0, "english", false, false).is_long());
    assert!(TestSpec::time(900, "english", false, false).is_long());
    assert!(!TestSpec::time(120, "english", false, false).is_long());
    assert!(TestSpec::words(1000, "english", false, false).is_long());
    assert!(!TestSpec::words(100, "english", false, false).is_long());
}

#[test]
fn mode2_matches_monkeytype() {
    assert_eq!(TestSpec::time(30, "english", false, false).mode2, "30");
    assert_eq!(TestSpec::words(50, "english", false, false).mode2, "50");
    assert_eq!(TestSpec::zen("english").mode2, "zen");
    assert_eq!(
        TestSpec::custom(CustomLimit::Word(5), "english", false, false).mode2,
        "custom"
    );
}
