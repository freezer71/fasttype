//! Listes de mots (`frontend/static/languages/<nom>.json`) et fichiers de
//! citations (`frontend/static/quotes/<langue>.json`), avec leur validation.

use crate::DataError;
use fasttype_core::quote::QuoteFile;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

/// Schéma `LanguageObjectSchema` ; les champs d'affichage du site sont ignorés.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageFile {
    pub name: String,
    pub words: Vec<String>,
    #[serde(default)]
    pub right_to_left: bool,
    #[serde(default)]
    pub no_lazy_mode: bool,
    #[serde(default)]
    pub ordered_by_frequency: bool,
    #[serde(default)]
    pub original_punctuation: bool,
    #[serde(default)]
    pub bcp47: Option<String>,
    /// `[caractères accentués, remplacement]` pour le lazy mode.
    #[serde(default)]
    pub additional_accents: Vec<(String, String)>,
}

/// Langue chargée ; la liste de mots est partagée sans copie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub name: String,
    pub words: Arc<Vec<String>>,
    pub right_to_left: bool,
    pub no_lazy_mode: bool,
    pub ordered_by_frequency: bool,
    pub original_punctuation: bool,
    pub bcp47: Option<String>,
    pub additional_accents: Vec<(String, String)>,
}

impl From<LanguageFile> for Language {
    fn from(f: LanguageFile) -> Self {
        Self {
            name: f.name,
            words: Arc::new(f.words),
            right_to_left: f.right_to_left,
            no_lazy_mode: f.no_lazy_mode,
            ordered_by_frequency: f.ordered_by_frequency,
            original_punctuation: f.original_punctuation,
            bcp47: f.bcp47,
            additional_accents: f.additional_accents,
        }
    }
}

fn invalid(name: &str, reason: impl Into<String>) -> DataError {
    DataError::Invalid {
        name: name.to_string(),
        reason: reason.into(),
    }
}

fn json_error(name: &str, e: serde_json::Error) -> DataError {
    DataError::Json {
        name: name.to_string(),
        message: e.to_string(),
    }
}

/// Lit et valide une liste de mots : nom égal au fichier, au moins un mot,
/// aucun mot vide.
pub fn parse_language(expected_name: &str, json: &[u8]) -> Result<LanguageFile, DataError> {
    let file: LanguageFile =
        serde_json::from_slice(json).map_err(|e| json_error(expected_name, e))?;
    if file.name != expected_name {
        return Err(invalid(
            expected_name,
            format!("le champ name vaut « {} »", file.name),
        ));
    }
    if file.words.is_empty() {
        return Err(invalid(expected_name, "aucun mot"));
    }
    if let Some(i) = file.words.iter().position(|w| w.trim().is_empty()) {
        return Err(invalid(expected_name, format!("mot vide à l'index {i}")));
    }
    Ok(file)
}

/// Lit et valide un fichier de citations : langue égale au fichier, quatre
/// groupes, textes non vides, identifiants uniques.
pub fn parse_quotes(expected_name: &str, json: &[u8]) -> Result<QuoteFile, DataError> {
    let file = QuoteFile::from_json(json).map_err(|e| json_error(expected_name, e))?;
    if file.language != expected_name {
        return Err(invalid(
            expected_name,
            format!("le champ language vaut « {} »", file.language),
        ));
    }
    if file.groups.len() != 4 {
        return Err(invalid(
            expected_name,
            format!("{} groupes au lieu de 4", file.groups.len()),
        ));
    }
    let mut ids = HashSet::new();
    for q in &file.quotes {
        if q.text.trim().is_empty() {
            return Err(invalid(expected_name, format!("citation {} vide", q.id)));
        }
        if !ids.insert(q.id) {
            return Err(invalid(
                expected_name,
                format!("identifiant {} en double", q.id),
            ));
        }
    }
    Ok(file)
}
