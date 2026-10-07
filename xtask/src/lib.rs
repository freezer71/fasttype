//! Construction de `assets/` à partir du dépôt Monkeytype, à un commit figé.

use fasttype_data::groups::parse_language_groups_ts;
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
const LANGUAGE_GROUPS_TS: &str = "frontend/src/ts/constants/languages.ts";
/// Seuls fichiers que `build_assets` accepte de remplacer dans `out/`.
const OUTPUT_FILES: [&str; 5] = [
    "languages.pack",
    "quotes.pack",
    "themes.json",
    "language_groups.json",
    "manifest.toml",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub languages: usize,
    pub quotes: usize,
    pub themes: usize,
    pub groups: usize,
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
) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut writer = PackWriter::new();
    let files = json_files(dir)?;
    for (name, path) in &files {
        let bytes = fs::read(path).map_err(|e| format!("{} : {e}", path.display()))?;
        validate(name, &bytes).map_err(|e| format!("{} : {e}", path.display()))?;
        writer.add(name, &bytes).map_err(|e| e.to_string())?;
    }
    Ok((
        writer.finish(),
        files.into_iter().map(|(name, _)| name).collect(),
    ))
}

/// Vérifie que `out` désigne un dossier nommé qu'on peut remplacer : il
/// n'existe pas encore, ou il ne contient que des fichiers produits ici.
/// Renvoie son nom et son dossier parent.
fn check_out(out: &Path) -> Result<(String, PathBuf), String> {
    let name = out
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| *n != "." && *n != "..")
        .ok_or_else(|| format!("{} : un dossier nommé est attendu", out.display()))?
        .to_string();
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    if out.exists() {
        let entries = fs::read_dir(out).map_err(|e| format!("{} : {e}", out.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let file = entry.file_name().to_string_lossy().to_string();
            if !OUTPUT_FILES.contains(&file.as_str()) {
                return Err(format!(
                    "{} contient « {file} », qui n'est pas un fichier d'assets : rien n'est remplacé",
                    out.display()
                ));
            }
        }
    }
    Ok((name, parent))
}

/// Construit `out/` depuis une copie de Monkeytype. Tout est écrit dans un
/// dossier temporaire voisin puis échangé : en cas d'erreur, `out/` n'est pas touché.
pub fn build_assets(source: &Path, out: &Path, rev: &str) -> Result<Summary, String> {
    let (out_name, parent) = check_out(out)?;
    let (languages, language_names) = build_pack(&source.join(LANGUAGES_DIR), |name, bytes| {
        parse_language(name, bytes)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })?;
    let (quotes, quote_names) = build_pack(&source.join(QUOTES_DIR), |name, bytes| {
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

    let groups_path = source.join(LANGUAGE_GROUPS_TS);
    let groups_src =
        fs::read_to_string(&groups_path).map_err(|e| format!("{} : {e}", groups_path.display()))?;
    let groups = parse_language_groups_ts(&groups_src).map_err(|e| e.to_string())?;
    for g in &groups {
        if let Some(unknown) = g.languages.iter().find(|l| !language_names.contains(l)) {
            return Err(format!(
                "{} : le groupe {} cite « {unknown} », langue inconnue",
                groups_path.display(),
                g.name
            ));
        }
    }
    let mut groups_json = serde_json::to_vec_pretty(&groups).map_err(|e| e.to_string())?;
    groups_json.push(b'\n');

    let n_languages = language_names.len();
    let n_quotes = quote_names.len();
    let outputs: [(&str, &[u8]); 4] = [
        ("languages.pack", &languages),
        ("quotes.pack", &quotes),
        ("themes.json", &themes_json),
        ("language_groups.json", &groups_json),
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

    let tmp = parent.join(format!(".{out_name}.tmp"));
    let old = parent.join(format!(".{out_name}.old"));
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
    // échange : l'ancien dossier n'est supprimé qu'une fois le nouveau en place
    let _ = fs::remove_dir_all(&old);
    if out.exists() {
        fs::rename(out, &old).map_err(|e| format!("{} : {e}", out.display()))?;
    }
    if let Err(e) = fs::rename(&tmp, out) {
        let _ = fs::rename(&old, out);
        return Err(format!("{} : {e}", out.display()));
    }
    let _ = fs::remove_dir_all(&old);
    Ok(Summary {
        languages: n_languages,
        quotes: n_quotes,
        themes: themes.len(),
        groups: groups.len(),
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
/// (clone partiel et sparse checkout). Ne refait rien si `dest` est déjà à ce
/// commit, sans modification locale, et contient tous les fichiers utiles.
pub fn fetch_source(rev: &str, dest: &Path) -> Result<(), String> {
    let reusable = dest.join(".git").exists()
        && git(dest, &["rev-parse", "HEAD"]).is_ok_and(|h| h == rev)
        && git(dest, &["status", "--porcelain"]).is_ok_and(|s| s.is_empty())
        && [
            LANGUAGES_DIR,
            QUOTES_DIR,
            THEMES_TS,
            LANGUAGE_GROUPS_TS,
            "LICENSE",
        ]
        .iter()
        .all(|p| dest.join(p).exists());
    if reusable {
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
            "/frontend/src/ts/constants/languages.ts",
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
