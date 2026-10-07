//! Construction de `assets/` à partir du dépôt Monkeytype, à un commit figé.

use fasttype_data::language::{parse_language, parse_quotes};
use fasttype_data::pack::PackWriter;
use fasttype_data::themes::parse_themes_ts;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const UPSTREAM: &str = "https://github.com/monkeytypegame/monkeytype";
const LANGUAGES_DIR: &str = "frontend/static/languages";
const QUOTES_DIR: &str = "frontend/static/quotes";
const THEMES_TS: &str = "frontend/src/ts/constants/themes.ts";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub languages: usize,
    pub quotes: usize,
    pub themes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub source: String,
    pub rev: String,
    pub languages: usize,
    pub quotes: usize,
    pub themes: usize,
    /// Empreinte de chaque fichier produit dans `assets/`.
    pub files: Vec<FileHash>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Fichiers `*.json` d'un dossier, triés par nom (pack reproductible).
fn json_files(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "json") {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("nom de fichier non UTF-8")?
                .to_string();
            files.push((stem, path));
        }
    }
    files.sort();
    Ok(files)
}

/// Lit, valide et compresse tous les fichiers d'un dossier dans un pack.
fn build_pack(
    dir: &Path,
    validate: impl Fn(&str, &[u8]) -> Result<(), String>,
) -> Result<(Vec<u8>, usize), String> {
    let mut writer = PackWriter::new();
    let files = json_files(dir)?;
    for (name, path) in &files {
        let bytes = fs::read(path).map_err(|e| format!("{} : {e}", path.display()))?;
        validate(name, &bytes).map_err(|e| format!("{} : {e}", path.display()))?;
        writer.add(name, &bytes).map_err(|e| e.to_string())?;
    }
    Ok((writer.finish(), files.len()))
}

/// Construit `out/` depuis une copie de Monkeytype. Tout est écrit dans
/// `out.tmp/` puis renommé : en cas d'erreur, `out/` n'est pas touché.
pub fn build_assets(source: &Path, out: &Path, rev: &str) -> Result<Summary, String> {
    let (languages, n_languages) = build_pack(&source.join(LANGUAGES_DIR), |name, bytes| {
        parse_language(name, bytes)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })?;
    let (quotes, n_quotes) = build_pack(&source.join(QUOTES_DIR), |name, bytes| {
        parse_quotes(name, bytes)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })?;
    let themes_path = source.join(THEMES_TS);
    let themes_src =
        fs::read_to_string(&themes_path).map_err(|e| format!("{} : {e}", themes_path.display()))?;
    let themes = parse_themes_ts(&themes_src).map_err(|e| e.to_string())?;
    let mut themes_json = serde_json::to_vec_pretty(&themes).map_err(|e| e.to_string())?;
    themes_json.push(b'\n');

    let outputs: [(&str, &[u8]); 3] = [
        ("languages.pack", &languages),
        ("quotes.pack", &quotes),
        ("themes.json", &themes_json),
    ];
    let manifest = Manifest {
        source: UPSTREAM.to_string(),
        rev: rev.to_string(),
        languages: n_languages,
        quotes: n_quotes,
        themes: themes.len(),
        files: outputs
            .iter()
            .map(|(path, bytes)| FileHash {
                path: path.to_string(),
                sha256: sha256_hex(bytes),
            })
            .chain(std::iter::once(FileHash {
                path: "LICENSE (amont)".into(),
                sha256: license_hash(source),
            }))
            .collect(),
    };
    let manifest_toml = toml::to_string(&manifest).map_err(|e| e.to_string())?;

    let tmp = out.with_extension("tmp");
    let _ = fs::remove_dir_all(&tmp);
    let write_all = || -> std::io::Result<()> {
        fs::create_dir_all(&tmp)?;
        for (name, bytes) in outputs {
            fs::write(tmp.join(name), bytes)?;
        }
        fs::write(tmp.join("manifest.toml"), &manifest_toml)
    };
    if let Err(e) = write_all() {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("écriture de {} : {e}", tmp.display()));
    }
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("{} : {e}", out.display()))?;
    }
    fs::rename(&tmp, out).map_err(|e| format!("{} : {e}", out.display()))?;
    Ok(Summary {
        languages: n_languages,
        quotes: n_quotes,
        themes: themes.len(),
    })
}

/// Empreinte du texte de licence amont (vide si absent, ex. dans les tests).
fn license_hash(source: &Path) -> String {
    fs::read(source.join("LICENSE"))
        .map(|b| sha256_hex(&b))
        .unwrap_or_default()
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git introuvable : {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {} : {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Récupère uniquement les fichiers utiles de Monkeytype, au commit `rev`
/// (clone partiel et sparse checkout). Ne refait rien si `dest` y est déjà.
pub fn fetch_source(rev: &str, dest: &Path) -> Result<(), String> {
    if dest.join(".git").exists() && git(dest, &["rev-parse", "HEAD"]).is_ok_and(|h| h == rev) {
        return Ok(());
    }
    let _ = fs::remove_dir_all(dest);
    fs::create_dir_all(dest).map_err(|e| format!("{} : {e}", dest.display()))?;
    git(dest, &["init", "--quiet"])?;
    git(
        dest,
        &["remote", "add", "origin", &format!("{UPSTREAM}.git")],
    )?;
    git(
        dest,
        &[
            "sparse-checkout",
            "set",
            "--no-cone",
            "/LICENSE",
            "/frontend/static/languages/",
            "/frontend/static/quotes/",
            "/frontend/src/ts/constants/themes.ts",
        ],
    )?;
    git(
        dest,
        &[
            "fetch",
            "--quiet",
            "--depth",
            "1",
            "--filter=blob:none",
            "origin",
            rev,
        ],
    )?;
    git(dest, &["checkout", "--quiet", "--detach", "FETCH_HEAD"])?;
    let head = git(dest, &["rev-parse", "HEAD"])?;
    if head != rev {
        return Err(format!("commit obtenu {head} au lieu de {rev}"));
    }
    Ok(())
}
