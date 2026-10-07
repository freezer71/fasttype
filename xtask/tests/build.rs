use fasttype_data::pack::Pack;
use std::fs;
use std::path::{Path, PathBuf};
use xtask::{Manifest, build_assets};

const THEMES_TS: &str = r##"export const themes: Record<ThemeName, Theme> = {
  serika_dark: {
    bg: "#323437",
    main: "#e2b714",
    caret: "#e2b714",
    sub: "#646669",
    subAlt: "#2c2e31",
    text: "#d1d0c5",
    error: "#ca4754",
    errorExtra: "#7e2a33",
    colorfulError: "#ca4754",
    colorfulErrorExtra: "#7e2a33",
  },
};
"##;

/// Répertoire temporaire propre au test (pas de dépendance `tempfile`).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fasttype-xtask-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fixture(root: &Path) {
    let st = root.join("frontend/static");
    write(
        &st.join("languages/english.json"),
        r#"{"name":"english","words":["the","be","of"]}"#,
    );
    write(
        &st.join("languages/korean.json"),
        r#"{"name":"korean","words":["한국","사람"]}"#,
    );
    write(
        &st.join("quotes/english.json"),
        r#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],"quotes":[{"text":"hi","source":"s","length":2,"id":1}]}"#,
    );
    write(&root.join("frontend/src/ts/constants/themes.ts"), THEMES_TS);
    write(&root.join("LICENSE"), "GPL-3.0 (fixture)\n");
}

#[test]
fn builds_packs_themes_and_manifest() {
    let dir = scratch("ok");
    fixture(&dir.join("src"));
    let out = dir.join("assets");
    let summary = build_assets(&dir.join("src"), &out, "abc123").unwrap();
    assert_eq!(
        (summary.languages, summary.quotes, summary.themes),
        (2, 1, 1)
    );

    let langs = fs::read(out.join("languages.pack")).unwrap();
    let pack = Pack::parse(&langs).unwrap();
    let names: Vec<&str> = pack.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["english", "korean"]);
    assert!(
        String::from_utf8(pack.decompress("korean").unwrap().unwrap())
            .unwrap()
            .contains("한국")
    );

    let themes: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join("themes.json")).unwrap()).unwrap();
    assert_eq!(themes[0]["name"], "serika_dark");

    let manifest: Manifest =
        toml::from_str(&fs::read_to_string(out.join("manifest.toml")).unwrap()).unwrap();
    assert_eq!(manifest.rev, "abc123");
    assert_eq!(manifest.files.len(), 4);
    assert!(manifest.files.iter().all(|f| f.sha256.len() == 64));
    assert!(!dir.join("assets.tmp").exists());
}

#[test]
fn build_is_reproducible() {
    let dir = scratch("repro");
    fixture(&dir.join("src"));
    build_assets(&dir.join("src"), &dir.join("a"), "r").unwrap();
    build_assets(&dir.join("src"), &dir.join("b"), "r").unwrap();
    for f in [
        "languages.pack",
        "quotes.pack",
        "themes.json",
        "manifest.toml",
    ] {
        assert_eq!(
            fs::read(dir.join("a").join(f)).unwrap(),
            fs::read(dir.join("b").join(f)).unwrap(),
            "{f}"
        );
    }
}

#[test]
fn invalid_source_leaves_previous_assets_untouched() {
    let dir = scratch("bad");
    fixture(&dir.join("src"));
    let out = dir.join("assets");
    build_assets(&dir.join("src"), &out, "r").unwrap();
    let before = fs::read(out.join("languages.pack")).unwrap();

    write(
        &dir.join("src/frontend/static/languages/broken.json"),
        r#"{"name":"other","words":["a"]}"#,
    );
    let err = build_assets(&dir.join("src"), &out, "r").unwrap_err();
    assert!(err.contains("broken"), "{err}");
    assert_eq!(fs::read(out.join("languages.pack")).unwrap(), before);
    assert!(!dir.join("assets.tmp").exists());
}

#[test]
fn missing_themes_file_is_reported() {
    let dir = scratch("nothemes");
    fixture(&dir.join("src"));
    fs::remove_file(dir.join("src/frontend/src/ts/constants/themes.ts")).unwrap();
    let err = build_assets(&dir.join("src"), &dir.join("assets"), "r").unwrap_err();
    assert!(err.contains("themes.ts"), "{err}");
}
