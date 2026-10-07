mod common;

use common::{result, scratch};
use fasttype_core::result::Invalid;
use fasttype_store::paths::Paths;
use fasttype_store::pbs::PbOutcome;
use fasttype_store::{RecordOutcome, Store};

fn paths(dir: &std::path::Path) -> Paths {
    Paths {
        config_dir: dir.join("config"),
        data_dir: dir.join("data"),
    }
}

#[test]
fn opens_on_an_empty_home() {
    let dir = scratch("store-empty");
    let store = Store::open(paths(&dir));
    assert!(store.warnings.is_empty(), "{:?}", store.warnings);
    assert_eq!(store.config.str("mode"), "time");
    assert!(!dir.join("data").exists(), "ouvrir n'écrit rien");
}

#[test]
fn records_results_and_personal_bests() {
    let dir = scratch("store-record");
    let mut store = Store::open(paths(&dir));
    assert_eq!(
        store.record(&result("30", 80.0, 1)).unwrap(),
        RecordOutcome::Saved(PbOutcome::NewBest { previous: None })
    );
    assert_eq!(
        store.record(&result("30", 70.0, 2)).unwrap(),
        RecordOutcome::Saved(PbOutcome::NotBest { best: 80.0 })
    );
    assert_eq!(store.history().unwrap().results.len(), 2);

    let reopened = Store::open(paths(&dir));
    assert_eq!(
        reopened
            .personal_best(&result("30", 0.0, 0).pb_key())
            .unwrap()
            .wpm,
        80.0
    );
}

#[test]
fn saving_disabled_and_invalid_results_write_nothing() {
    let dir = scratch("store-nosave");
    let mut store = Store::open(paths(&dir));
    let mut invalid = result("30", 80.0, 1);
    invalid.invalid = Some(Invalid::TooShort);
    assert_eq!(store.record(&invalid).unwrap(), RecordOutcome::Invalid);
    store
        .config
        .set("resultSaving", toml::Value::Boolean(false))
        .unwrap();
    assert_eq!(
        store.record(&result("30", 80.0, 2)).unwrap(),
        RecordOutcome::SavingDisabled
    );
    assert!(store.history().unwrap().results.is_empty());
}

#[test]
fn config_changes_persist() {
    let dir = scratch("store-config");
    let mut store = Store::open(paths(&dir));
    store
        .config
        .set("theme", toml::Value::String("nord".into()))
        .unwrap();
    store.save_config().unwrap();
    assert_eq!(Store::open(paths(&dir)).config.str("theme"), "nord");
}

#[test]
fn corrupt_pbs_are_rebuilt_from_history() {
    let dir = scratch("store-rebuild");
    let mut store = Store::open(paths(&dir));
    store.record(&result("30", 80.0, 1)).unwrap();
    store.record(&result("30", 90.0, 2)).unwrap();
    std::fs::write(paths(&dir).pbs_file(), "garbage").unwrap();

    let reopened = Store::open(paths(&dir));
    assert_eq!(reopened.warnings.len(), 1, "{:?}", reopened.warnings);
    assert_eq!(
        reopened
            .personal_best(&result("30", 0.0, 0).pb_key())
            .unwrap()
            .wpm,
        90.0
    );
    let saved = fasttype_store::pbs::PersonalBests::load(&paths(&dir).pbs_file()).unwrap();
    assert_eq!(saved.len(), 1, "le cache reconstruit est réécrit");
}

#[test]
fn rebuild_pbs_reports_history() {
    let dir = scratch("store-rebuild-cmd");
    let mut store = Store::open(paths(&dir));
    store.record(&result("30", 80.0, 1)).unwrap();
    let report = store.rebuild_pbs().unwrap();
    assert_eq!((report.results.len(), report.skipped_lines), (1, 0));
}

#[test]
fn config_warnings_surface_in_store() {
    let dir = scratch("store-warn");
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::write(dir.join("config/config.toml"), "turbo = true\n").unwrap();
    let store = Store::open(paths(&dir));
    assert_eq!(store.warnings.len(), 1);
    assert!(store.warnings[0].contains("turbo"));
}
