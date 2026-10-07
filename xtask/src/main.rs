//! `cargo xtask fetch-data [--rev SHA] [--source DIR] [--out DIR]`

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use xtask::{build_assets, fetch_source};

/// Commit de référence de Monkeytype (voir la spec et NOTICE).
const DEFAULT_REV: &str = "574d8193498f75d13e7296584c82b64cebe2efea";

fn usage() -> ExitCode {
    eprintln!("usage : cargo xtask fetch-data [--rev SHA] [--source DOSSIER] [--out DOSSIER]");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("fetch-data") {
        return usage();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("racine du workspace")
        .to_path_buf();
    let mut rev = DEFAULT_REV.to_string();
    let mut source: Option<PathBuf> = None;
    let mut out = root.join("assets");
    let mut it = args[1..].iter();
    while let Some(flag) = it.next() {
        let Some(value) = it.next() else {
            return usage();
        };
        match flag.as_str() {
            "--rev" => rev = value.clone(),
            "--source" => source = Some(PathBuf::from(value)),
            "--out" => out = PathBuf::from(value),
            _ => return usage(),
        }
    }
    let source = match source {
        Some(s) => s,
        None => {
            let dest = root.join("target").join(format!("monkeytype-{rev}"));
            eprintln!("récupération de Monkeytype @ {rev}…");
            if let Err(e) = fetch_source(&rev, &dest) {
                eprintln!("erreur : {e}");
                return ExitCode::FAILURE;
            }
            dest
        }
    };
    match build_assets(&source, &out, &rev) {
        Ok(s) => {
            println!(
                "{} langues, {} fichiers de citations, {} thèmes, {} groupes de langues → {}",
                s.languages,
                s.quotes,
                s.themes,
                s.groups,
                out.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}
