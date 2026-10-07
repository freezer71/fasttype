#![allow(dead_code)]

use fasttype_core::result::{ChartData, TestResult};
use fasttype_core::spec::{Difficulty, Mode};
use std::fs;
use std::path::PathBuf;

/// Dossier temporaire propre au test : jamais le vrai `$HOME`.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fasttype-store-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Résultat valide de test time en anglais.
pub fn result(mode2: &str, wpm: f64, timestamp: u64) -> TestResult {
    TestResult {
        timestamp,
        mode: Mode::Time,
        mode2: mode2.into(),
        language: "english".into(),
        punctuation: false,
        numbers: false,
        difficulty: Difficulty::Normal,
        lazy_mode: false,
        wpm,
        raw: wpm + 5.0,
        acc: 97.5,
        consistency: 80.0,
        char_stats: [100, 2, 0, 1],
        test_duration: 30.0,
        afk_duration: 0,
        afk_detected: false,
        bailed_out: false,
        quote_id: None,
        quote_length: None,
        chart: ChartData {
            wpm: vec![wpm; 3],
            raw: vec![wpm; 3],
            burst: vec![wpm; 3],
            err: vec![0; 3],
        },
        invalid: None,
    }
}
