mod common;

use common::{result, scratch};
use fasttype_store::results::ResultLog;
use std::io::Write;

#[test]
fn appends_and_loads_in_order() {
    let dir = scratch("results");
    let log = ResultLog::new(dir.join("results.jsonl"));
    log.append(&result("30", 80.0, 1)).unwrap();
    log.append(&result("60", 90.0, 2)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(loaded.skipped_lines, 0);
    assert_eq!(
        loaded.results,
        [result("30", 80.0, 1), result("60", 90.0, 2)]
    );
    let text = std::fs::read_to_string(dir.join("results.jsonl")).unwrap();
    assert_eq!(text.lines().count(), 2);
}

#[test]
fn missing_file_is_empty_history() {
    let dir = scratch("noresults");
    let loaded = ResultLog::new(dir.join("results.jsonl")).load().unwrap();
    assert!(loaded.results.is_empty());
}

#[test]
fn corrupt_line_is_skipped_and_counted() {
    let dir = scratch("corrupt");
    let path = dir.join("results.jsonl");
    let log = ResultLog::new(path.clone());
    log.append(&result("30", 80.0, 1)).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"{not json}\n\n")
        .unwrap();
    log.append(&result("30", 81.0, 2)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(loaded.results.len(), 2);
    assert_eq!(loaded.skipped_lines, 1, "les lignes vides ne comptent pas");
}

#[test]
fn append_after_truncated_line_stays_readable() {
    let dir = scratch("truncated");
    let path = dir.join("results.jsonl");
    let log = ResultLog::new(path.clone());
    log.append(&result("30", 80.0, 1)).unwrap();
    // plantage au milieu d'une écriture : ligne sans fin
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"{\"timestamp\":2,\"mo")
        .unwrap();
    log.append(&result("30", 82.0, 3)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(
        loaded
            .results
            .iter()
            .map(|r| r.timestamp)
            .collect::<Vec<_>>(),
        [1, 3]
    );
    assert_eq!(loaded.skipped_lines, 1);
}

#[test]
fn long_chart_is_kept_whole() {
    let dir = scratch("longchart");
    let log = ResultLog::new(dir.join("results.jsonl"));
    let mut r = result("0", 70.0, 1);
    r.chart.wpm = vec![70.0; 3600];
    log.append(&r).unwrap();
    assert_eq!(log.load().unwrap().results[0].chart.wpm.len(), 3600);
}

#[test]
fn non_utf8_line_is_skipped_not_fatal() {
    let dir = scratch("nonutf8-results");
    let path = dir.join("results.jsonl");
    let log = ResultLog::new(path.clone());
    log.append(&result("30", 120.0, 1)).unwrap();
    // ligne tronquée au milieu d'un caractère UTF-8
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"{\"language\":\"fran\xc3")
        .unwrap();
    log.append(&result("30", 90.0, 2)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(
        loaded
            .results
            .iter()
            .map(|r| r.timestamp)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(loaded.skipped_lines, 1);
}

#[test]
fn results_saved_before_the_raw_series_still_load() {
    let line = serde_json::to_string(&result("30", 50.0, 1)).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&line).unwrap();
    v["chart"].as_object_mut().unwrap().remove("raw");
    let old: fasttype_core::result::TestResult = serde_json::from_value(v).unwrap();
    assert!(old.chart.raw.is_empty());
}
