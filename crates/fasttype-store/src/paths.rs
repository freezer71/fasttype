//! Emplacements XDG des fichiers de fasttype.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

/// Variable XDG utilisable : définie, non vide et absolue.
fn xdg(get: &impl Fn(&str) -> Option<String>, var: &str) -> Option<PathBuf> {
    get(var)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}

impl Paths {
    /// `get` lit une variable d'environnement (injectée pour les tests).
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Option<Paths> {
        let home = get("HOME").filter(|h| !h.is_empty()).map(PathBuf::from);
        let config =
            xdg(&get, "XDG_CONFIG_HOME").or_else(|| home.as_ref().map(|h| h.join(".config")))?;
        let data =
            xdg(&get, "XDG_DATA_HOME").or_else(|| home.as_ref().map(|h| h.join(".local/share")))?;
        Some(Paths {
            config_dir: config.join("fasttype"),
            data_dir: data.join("fasttype"),
        })
    }

    pub fn from_system() -> Option<Paths> {
        Self::from_env(|k| std::env::var(k).ok())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn results_file(&self) -> PathBuf {
        self.data_dir.join("results.jsonl")
    }

    pub fn pbs_file(&self) -> PathBuf {
        self.data_dir.join("personal_bests.json")
    }

    pub fn custom_texts_dir(&self) -> PathBuf {
        self.data_dir.join("custom_texts")
    }

    pub fn favorites_file(&self) -> PathBuf {
        self.data_dir.join("favorite_quotes.json")
    }
}

/// Dossier parent existant ou à créer (`.` pour un chemin nu).
pub(crate) fn parent_of(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
