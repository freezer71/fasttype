mod common;

use common::scratch;
use fasttype_store::Config;
use fasttype_store::fs::{load_config, save_config, write_atomic};
use fasttype_store::paths::Paths;
use std::collections::HashMap;
use std::path::PathBuf;

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k| map.get(k).cloned()
}

#[test]
fn xdg_paths_with_home_fallback() {
    let p = Paths::from_env(env(&[("HOME", "/home/a")])).unwrap();
    assert_eq!(p.config_dir, PathBuf::from("/home/a/.config/fasttype"));
    assert_eq!(p.data_dir, PathBuf::from("/home/a/.local/share/fasttype"));
    assert_eq!(
        p.config_file(),
        PathBuf::from("/home/a/.config/fasttype/config.toml")
    );
    assert_eq!(
        p.results_file(),
        PathBuf::from("/home/a/.local/share/fasttype/results.jsonl")
    );
    assert_eq!(
        p.pbs_file(),
        PathBuf::from("/home/a/.local/share/fasttype/personal_bests.json")
    );

    let x = Paths::from_env(env(&[
        ("HOME", "/home/a"),
        ("XDG_CONFIG_HOME", "/cfg"),
        ("XDG_DATA_HOME", "/data"),
    ]))
    .unwrap();
    assert_eq!(x.config_dir, PathBuf::from("/cfg/fasttype"));
    assert_eq!(x.data_dir, PathBuf::from("/data/fasttype"));

    // XDG relatif ou vide : ignoré, comme le veut la spécification XDG
    let rel = Paths::from_env(env(&[
        ("HOME", "/home/a"),
        ("XDG_CONFIG_HOME", "cfg"),
        ("XDG_DATA_HOME", ""),
    ]))
    .unwrap();
    assert_eq!(rel.config_dir, PathBuf::from("/home/a/.config/fasttype"));
    assert_eq!(rel.data_dir, PathBuf::from("/home/a/.local/share/fasttype"));

    assert!(Paths::from_env(env(&[])).is_none());
}

#[test]
fn atomic_write_leaves_no_temp_file() {
    let dir = scratch("atomic");
    let file = dir.join("sub/f.txt");
    write_atomic(&file, b"one").unwrap();
    write_atomic(&file, b"two").unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"two");
    let names: Vec<_> = std::fs::read_dir(dir.join("sub"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["f.txt"]);
}

#[test]
fn missing_config_gives_defaults_silently() {
    let dir = scratch("noconfig");
    let (c, warnings) = load_config(&dir.join("config.toml"));
    assert_eq!(c, Config::defaults());
    assert!(warnings.is_empty());
}

#[test]
fn saved_config_loads_back() {
    let dir = scratch("roundtrip");
    let path = dir.join("config.toml");
    let mut c = Config::defaults();
    c.set("smoothCaret", toml::Value::String("slow".into()))
        .unwrap();
    save_config(&path, &c).unwrap();
    let (back, warnings) = load_config(&path);
    assert_eq!(back, c);
    assert!(warnings.is_empty());
}

#[test]
fn broken_config_is_moved_aside() {
    let dir = scratch("broken");
    let path = dir.join("config.toml");
    std::fs::write(&path, "smooth_caret = \"slow").unwrap();
    let (c, warnings) = load_config(&path);
    assert_eq!(c, Config::defaults());
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings[1].contains("config.toml.bak"));
    assert_eq!(
        std::fs::read_to_string(dir.join("config.toml.bak")).unwrap(),
        "smooth_caret = \"slow"
    );
    assert!(!path.exists());
}

#[test]
fn partially_invalid_config_is_kept_in_place_and_backed_up() {
    let dir = scratch("partial");
    let path = dir.join("config.toml");
    let original = "# mes réglages\nsmooth_caret = \"slow\"\nturbo = 1\n";
    std::fs::write(&path, original).unwrap();
    let (c, warnings) = load_config(&path);
    assert_eq!(c.str("smoothCaret"), "slow");
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings[1].contains("config.toml.bak"));
    assert!(path.exists());
    // la prochaine sauvegarde réécrira le fichier : l'original doit survivre
    save_config(&path, &c).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("config.toml.bak")).unwrap(),
        original
    );
}

#[test]
fn non_utf8_config_is_backed_up() {
    let dir = scratch("nonutf8");
    let path = dir.join("config.toml");
    std::fs::write(&path, b"smooth_caret = \"sl\xffow\"\n").unwrap();
    let (c, warnings) = load_config(&path);
    assert_eq!(c, Config::defaults());
    assert!(
        warnings.iter().any(|w| w.contains("config.toml.bak")),
        "{warnings:?}"
    );
    assert_eq!(
        std::fs::read(dir.join("config.toml.bak")).unwrap(),
        b"smooth_caret = \"sl\xffow\"\n"
    );
}
