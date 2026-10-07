//! Textes custom enregistrés et citations favorites.

use crate::fs::write_atomic;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

fn valid_name(name: &str) -> bool {
    let len = name.chars().count();
    (1..=64).contains(&len)
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'))
}

pub struct CustomTexts {
    dir: PathBuf,
}

impl CustomTexts {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn file(&self, name: &str) -> io::Result<PathBuf> {
        if !valid_name(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("nom de texte invalide : {name:?}"),
            ));
        }
        Ok(self.dir.join(format!("{name}.txt")))
    }

    /// Noms des textes enregistrés, triés.
    pub fn list(&self) -> io::Result<Vec<String>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut names = Vec::new();
        for entry in entries {
            let file = entry?.file_name().to_string_lossy().to_string();
            if let Some(name) = file.strip_suffix(".txt").filter(|n| valid_name(n)) {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn load(&self, name: &str) -> io::Result<String> {
        fs::read_to_string(self.file(name)?)
    }

    pub fn save(&self, name: &str, text: &str) -> io::Result<()> {
        write_atomic(&self.file(name)?, text.as_bytes())
    }

    pub fn delete(&self, name: &str) -> io::Result<()> {
        fs::remove_file(self.file(name)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteQuote {
    pub language: String,
    pub id: u32,
}

pub struct FavoriteQuotes {
    path: PathBuf,
}

impl FavoriteQuotes {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> io::Result<Vec<FavoriteQuote>> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Ajoute ou retire la citation ; renvoie `true` si elle est désormais favorite.
    pub fn toggle(&self, language: &str, id: u32) -> io::Result<bool> {
        let mut favs = self.load()?;
        let quote = FavoriteQuote {
            language: language.to_string(),
            id,
        };
        let now_favorite = match favs.iter().position(|f| *f == quote) {
            Some(i) => {
                favs.remove(i);
                false
            }
            None => {
                favs.push(quote);
                true
            }
        };
        let json = serde_json::to_vec_pretty(&favs).map_err(io::Error::other)?;
        write_atomic(&self.path, &json)?;
        Ok(now_favorite)
    }
}
