use fasttype_store::{Config, ConfigError, ConfigWarning};
use toml::Value;

fn s(x: &str) -> Value {
    Value::String(x.into())
}

#[test]
fn defaults_are_readable_through_typed_getters() {
    let c = Config::defaults();
    assert_eq!(c.str("mode"), "time");
    assert_eq!(c.int("time"), 30);
    assert_eq!(c.int("words"), 50);
    assert!(!c.bool("punctuation"));
    assert!(c.bool("resultSaving"));
    assert_eq!(c.float("soundVolume"), 0.5);
    assert_eq!(c.float("fontSize"), 2.0);
    assert_eq!(c.int_list("quoteLength"), [1]);
    assert_eq!(
        c.str_list("customPolyglot"),
        ["english", "spanish", "french", "german"]
    );
    assert_eq!(c.str("theme"), "serika_dark");
    assert_eq!(c.str("doesNotExist"), "");
}

#[test]
fn choosing_a_duration_switches_to_time_mode() {
    let mut c = Config::defaults();
    c.set("mode", s("words")).unwrap();
    let changed = c.set("time", Value::Integer(60)).unwrap();
    assert_eq!(changed, ["time", "mode"]);
    assert_eq!((c.str("mode"), c.int("time")), ("time", 60));
    assert_eq!(
        c.set("time", Value::Integer(60)).unwrap(),
        Vec::<&str>::new(),
        "rien ne change"
    );
}

#[test]
fn quote_mode_turns_off_punctuation_and_numbers() {
    let mut c = Config::defaults();
    c.set("punctuation", Value::Boolean(true)).unwrap();
    c.set("numbers", Value::Boolean(true)).unwrap();
    let changed = c.set("mode", s("quote")).unwrap();
    assert_eq!(changed, ["mode", "numbers", "punctuation"]);
    assert!(!c.bool("punctuation") && !c.bool("numbers"));
}

#[test]
fn error_handling_modes_exclude_each_other() {
    let mut c = Config::defaults();
    c.set("freedomMode", Value::Boolean(true)).unwrap();
    c.set("confidenceMode", s("on")).unwrap();
    assert!(!c.bool("freedomMode"));
    c.set("stopOnError", s("word")).unwrap();
    assert_eq!(c.str("confidenceMode"), "off");
    c.set("deleteOnError", s("letter")).unwrap();
    assert_eq!(c.str("stopOnError"), "off");
}

#[test]
fn monkey_downgrades_text_live_stats() {
    let mut c = Config::defaults();
    c.set("liveSpeedStyle", s("text")).unwrap();
    c.set("monkey", Value::Boolean(true)).unwrap();
    assert_eq!(c.str("liveSpeedStyle"), "mini");
    c.set("liveAccStyle", s("text")).unwrap();
    assert!(!c.bool("monkey"));
}

#[test]
fn other_overrides() {
    let mut c = Config::defaults();
    c.set("customTheme", Value::Boolean(true)).unwrap();
    c.set("theme", s("nord")).unwrap();
    assert!(!c.bool("customTheme"));
    c.set("keymapSize", Value::Float(1.5)).unwrap();
    assert_eq!(c.str("keymapMode"), "static");
    c.set("showAllLines", Value::Boolean(true)).unwrap();
    c.set("tapeMode", s("word")).unwrap();
    assert!(!c.bool("showAllLines"));
    c.set("minAccCustom", Value::Integer(95)).unwrap();
    assert_eq!(c.str("minAcc"), "custom");
}

#[test]
fn invalid_set_is_refused_and_changes_nothing() {
    let mut c = Config::defaults();
    assert!(matches!(
        c.set("smoothCaret", s("turbo")),
        Err(ConfigError::Invalid { .. })
    ));
    assert!(matches!(
        c.set("nope", s("x")),
        Err(ConfigError::UnknownKey(_))
    ));
    assert_eq!(c, Config::defaults());
}

#[test]
fn toml_roundtrip_is_lossless() {
    let mut c = Config::defaults();
    c.set("smoothCaret", s("fast")).unwrap();
    c.set(
        "customBackgroundFilter",
        toml::from_str::<toml::Table>("v = [2, 0.5, 1, 1]").unwrap()["v"].clone(),
    )
    .unwrap();
    c.set("theme", s("serika \"quoted\"")).unwrap();
    let text = c.to_toml();
    assert!(text.contains("smooth_caret = \"fast\""), "{text}");
    assert!(text.contains("# caret"), "{text}");
    let (back, warnings) = Config::from_toml(&text);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(back, c);
}

#[test]
fn hand_edited_file_keeps_valid_keys_and_warns_on_the_rest() {
    let (c, warnings) =
        Config::from_toml("smooth_caret = \"slow\"\nturbo = true\ntime = -5\nmode = \"words\"\n");
    assert_eq!(c.str("smoothCaret"), "slow");
    assert_eq!(c.int("time"), 30, "valeur invalide remplacée par le défaut");
    assert_eq!(c.str("mode"), "words", "aucun effet de bord à la lecture");
    assert_eq!(warnings.len(), 2);
    assert!(warnings.contains(&ConfigWarning::UnknownKey("turbo".into())));
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, ConfigWarning::Invalid { key, .. } if key == "time"))
    );
    assert!(warnings.iter().all(|w| !w.to_string().is_empty()));
}

#[test]
fn broken_toml_gives_defaults_and_one_warning() {
    let (c, warnings) = Config::from_toml("smooth_caret = \"slow");
    assert_eq!(c, Config::defaults());
    assert!(matches!(warnings.as_slice(), [ConfigWarning::Syntax(_)]));
}

#[test]
fn reset_restores_defaults() {
    let mut c = Config::defaults();
    c.set("words", Value::Integer(10)).unwrap();
    c.reset();
    assert_eq!(c, Config::defaults());
}
