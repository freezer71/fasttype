mod common;

use common::{result, scratch};
use fasttype_core::result::Invalid;
use fasttype_core::spec::Mode;
use fasttype_store::pbs::{PbOutcome, PersonalBests};

#[test]
fn only_strictly_faster_results_beat_the_record() {
    let mut pbs = PersonalBests::default();
    assert_eq!(
        pbs.update(&result("30", 80.0, 1)),
        PbOutcome::NewBest { previous: None }
    );
    assert_eq!(
        pbs.update(&result("30", 80.0, 2)),
        PbOutcome::NotBest { best: 80.0 }
    );
    assert_eq!(
        pbs.update(&result("30", 85.5, 3)),
        PbOutcome::NewBest {
            previous: Some(80.0)
        }
    );
    let key = result("30", 0.0, 0).pb_key();
    assert_eq!(pbs.get(&key).unwrap().timestamp, 3);
}

#[test]
fn each_configuration_has_its_own_record() {
    let mut pbs = PersonalBests::default();
    pbs.update(&result("30", 80.0, 1));
    pbs.update(&result("60", 70.0, 2));
    let mut punct = result("30", 60.0, 3);
    punct.punctuation = true;
    assert_eq!(pbs.update(&punct), PbOutcome::NewBest { previous: None });
    assert_eq!(pbs.len(), 3);
}

#[test]
fn ineligible_results_are_ignored() {
    let mut pbs = PersonalBests::default();
    let mut invalid = result("30", 200.0, 1);
    invalid.invalid = Some(Invalid::Afk);
    let mut bailed = result("30", 200.0, 2);
    bailed.bailed_out = true;
    let mut quote = result("7", 200.0, 3);
    quote.mode = Mode::Quote;
    for r in [invalid, bailed, quote] {
        assert_eq!(pbs.update(&r), PbOutcome::Ineligible);
    }
    assert_eq!(pbs.len(), 0);
}

#[test]
fn rebuild_matches_incremental_updates() {
    let history = [
        result("30", 80.0, 1),
        result("30", 90.0, 2),
        result("60", 70.0, 3),
        result("30", 85.0, 4),
    ];
    let mut incremental = PersonalBests::default();
    for r in &history {
        incremental.update(r);
    }
    assert_eq!(PersonalBests::rebuild(&history), incremental);
}

#[test]
fn pbs_are_written_atomically_and_load_back() {
    let dir = scratch("pbs");
    let path = dir.join("personal_bests.json");
    let mut pbs = PersonalBests::default();
    pbs.update(&result("30", 80.0, 1));
    pbs.update(&result("60", 70.0, 2));
    pbs.save(&path).unwrap();
    assert_eq!(PersonalBests::load(&path).unwrap(), pbs);
    assert!(!dir.join(".personal_bests.json.tmp").exists());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn missing_and_corrupt_files() {
    let dir = scratch("pbs-bad");
    let path = dir.join("personal_bests.json");
    assert_eq!(
        PersonalBests::load(&path).unwrap(),
        PersonalBests::default()
    );
    std::fs::write(&path, "[{\"broken\"").unwrap();
    assert_eq!(
        PersonalBests::load(&path).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
}
