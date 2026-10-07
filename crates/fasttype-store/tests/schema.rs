use fasttype_store::schema::{Group, Kind, SCHEMA, key_def, key_def_by_toml, to_snake, validate};
use std::collections::HashSet;
use toml::Value;

#[test]
fn has_the_94_monkeytype_keys_once() {
    assert_eq!(SCHEMA.len(), 94);
    let names: HashSet<_> = SCHEMA.iter().map(|d| d.name).collect();
    assert_eq!(names.len(), 94);
    let snakes: HashSet<_> = SCHEMA.iter().map(|d| d.toml_key()).collect();
    assert_eq!(snakes.len(), 94);
}

#[test]
fn every_default_is_valid() {
    for d in SCHEMA {
        let v = d.default_value();
        assert_eq!(validate(&d.kind, &v), Ok(()), "{} = {}", d.name, d.default);
    }
}

#[test]
fn defaults_match_monkeytype() {
    let get = |k: &str| key_def(k).unwrap().default_value();
    assert_eq!(get("mode").as_str(), Some("time"));
    assert_eq!(get("time").as_integer(), Some(30));
    assert_eq!(get("words").as_integer(), Some(50));
    assert_eq!(get("smoothCaret").as_str(), Some("medium"));
    assert_eq!(get("timerStyle").as_str(), Some("mini"));
    assert_eq!(get("theme").as_str(), Some("serika_dark"));
    assert_eq!(get("soundVolume").as_float(), Some(0.5));
    assert_eq!(get("customThemeColors").as_array().unwrap().len(), 10);
    assert_eq!(get("singleListCommandLine").as_str(), Some("on"));
    assert_eq!(
        get("quoteLength").as_array().unwrap()[0].as_integer(),
        Some(1)
    );
}

#[test]
fn groups_and_labels() {
    assert_eq!(key_def("smoothCaret").unwrap().group, Group::Caret);
    assert_eq!(
        key_def("timerStyle").unwrap().display,
        "live progress style"
    );
    assert_eq!(key_def("showPb").unwrap().group, Group::HideElements);
    assert_eq!(Group::HideElements.label(), "hide elements");
}

#[test]
fn snake_case_keys() {
    assert_eq!(to_snake("smoothCaret"), "smooth_caret");
    assert_eq!(to_snake("customLayoutfluid"), "custom_layoutfluid");
    assert_eq!(to_snake("minWpmCustomSpeed"), "min_wpm_custom_speed");
    assert_eq!(
        key_def_by_toml("quick_restart").unwrap().name,
        "quickRestart"
    );
    assert!(key_def_by_toml("quickRestart").is_none());
}

fn v(src: &str) -> Value {
    toml::from_str::<toml::Table>(&format!("v = {src}"))
        .unwrap()
        .remove("v")
        .unwrap()
}

#[test]
fn validation_follows_zod_schemas() {
    let ok = |k: &str, s: &str| validate(&key_def(k).unwrap().kind, &v(s)).is_ok();
    assert!(ok("smoothCaret", "\"fast\""));
    assert!(!ok("smoothCaret", "\"turbo\""));
    assert!(!ok("smoothCaret", "true"));
    assert!(ok("time", "0"));
    assert!(!ok("time", "-1"));
    assert!(!ok("time", "1.5"));
    assert!(ok("minAccCustom", "100"));
    assert!(ok("minAccCustom", "99.5"));
    assert!(!ok("minAccCustom", "101"));
    assert!(ok("tapeMargin", "10"));
    assert!(!ok("tapeMargin", "9"));
    assert!(ok("maxLineWidth", "0"));
    assert!(!ok("maxLineWidth", "10"));
    assert!(ok("maxLineWidth", "20"));
    assert!(!ok("fontSize", "0"));
    assert!(ok("quoteLength", "[-3, 0, 3]"));
    assert!(!ok("quoteLength", "[4]"));
    assert!(ok(
        "customThemeColors",
        "[\"#fff\",\"#000000\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#ABCDEF\"]"
    ));
    assert!(!ok("customThemeColors", "[\"#fff\"]"));
    assert!(!ok("customPolyglot", "[\"english\"]"));
    assert!(ok("customPolyglot", "[\"english\", \"french\"]"));
    assert!(ok("accountChart", "[\"on\", \"off\", \"on\", \"on\"]"));
    assert!(!ok("accountChart", "[\"on\"]"));
    assert!(ok("customBackground", "\"\""));
    assert!(ok("customBackground", "\"https://example.com/cat.webp\""));
    assert!(!ok("customBackground", "\"ftp://example.com/cat.png\""));
    assert!(!ok("customBackground", "\"https://example.com/cat.svg\""));
    assert!(!ok("language", "\"\""));
    assert!(!ok("soundVolume", "nan"));
    assert!(matches!(
        key_def("customBackgroundFilter").unwrap().kind,
        Kind::Numbers(4)
    ));
}
