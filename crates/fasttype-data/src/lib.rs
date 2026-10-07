//! Données de Monkeytype embarquées : langues, citations et thèmes.
//! Aucune E/S à l'exécution ; `xtask` produit les fichiers de `assets/`.

use std::fmt;

pub mod language;
pub mod pack;
pub mod themes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    UnknownLanguage(String),
    /// Données embarquées illisibles (pack ou index).
    Corrupt(String),
    Json {
        name: String,
        message: String,
    },
    Invalid {
        name: String,
        reason: String,
    },
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::UnknownLanguage(name) => write!(f, "langue inconnue : {name}"),
            DataError::Corrupt(why) => write!(f, "données embarquées illisibles : {why}"),
            DataError::Json { name, message } => write!(f, "{name} : JSON invalide ({message})"),
            DataError::Invalid { name, reason } => write!(f, "{name} : {reason}"),
        }
    }
}

impl std::error::Error for DataError {}
