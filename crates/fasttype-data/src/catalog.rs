//! Données embarquées dans le binaire. Seul l'index des packs est lu au
//! premier accès ; chaque langue est décompressée à la demande.

use crate::DataError;
use crate::groups::LanguageGroup;
use crate::language::{Language, parse_language, parse_quotes};
use crate::pack::Pack;
use crate::themes::Theme;
use fasttype_core::quote::QuoteFile;
use fasttype_core::result::remove_language_size;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

static LANGUAGES_PACK: &[u8] = include_bytes!("../../../assets/languages.pack");
static QUOTES_PACK: &[u8] = include_bytes!("../../../assets/quotes.pack");
static THEMES_JSON: &str = include_str!("../../../assets/themes.json");

pub const DEFAULT_LANGUAGE: &str = "english";
pub const DEFAULT_THEME: &str = "serika_dark";

fn parsed_pack(
    cell: &'static OnceLock<Result<Pack<'static>, String>>,
    bytes: &'static [u8],
) -> Result<&'static Pack<'static>, DataError> {
    cell.get_or_init(|| Pack::parse(bytes).map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| DataError::Corrupt(e.clone()))
}

fn languages_pack() -> Result<&'static Pack<'static>, DataError> {
    static CELL: OnceLock<Result<Pack<'static>, String>> = OnceLock::new();
    parsed_pack(&CELL, LANGUAGES_PACK)
}

fn quotes_pack() -> Result<&'static Pack<'static>, DataError> {
    static CELL: OnceLock<Result<Pack<'static>, String>> = OnceLock::new();
    parsed_pack(&CELL, QUOTES_PACK)
}

fn names(pack: &'static Pack<'static>) -> Vec<&'static str> {
    pack.entries().iter().map(|e| e.name.as_str()).collect()
}

/// Noms de toutes les langues, triés.
pub fn language_names() -> Result<Vec<&'static str>, DataError> {
    languages_pack().map(names)
}

/// Noms des fichiers de citations (langues sans suffixe de taille), triés.
pub fn quote_file_names() -> Result<Vec<&'static str>, DataError> {
    quotes_pack().map(names)
}

/// Décompresse et valide une langue. Pour les grosses listes (jusqu'à 12 Mo),
/// l'appelant fait cet appel hors du fil de l'interface.
pub fn load_language(name: &str) -> Result<Language, DataError> {
    let bytes = languages_pack()?
        .decompress(name)
        .map_err(|e| DataError::Corrupt(format!("{name} : {e}")))?
        .ok_or_else(|| DataError::UnknownLanguage(name.to_string()))?;
    parse_language(name, &bytes).map(Language::from)
}

/// Citations de la langue (`english_1k` → `english`) ; `None` s'il n'y en a pas.
pub fn quotes_for(language: &str) -> Result<Option<QuoteFile>, DataError> {
    let base = remove_language_size(language);
    match quotes_pack()?
        .decompress(&base)
        .map_err(|e| DataError::Corrupt(format!("{base} : {e}")))?
    {
        Some(bytes) => parse_quotes(&base, &bytes).map(Some),
        None => Ok(None),
    }
}

/// Les 187 thèmes, triés par nom.
pub fn themes() -> Result<&'static [Theme], DataError> {
    static CELL: OnceLock<Result<Vec<Theme>, String>> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(THEMES_JSON).map_err(|e| e.to_string()))
        .as_deref()
        .map_err(|e| DataError::Corrupt(format!("themes.json : {e}")))
}

pub fn theme(name: &str) -> Option<&'static Theme> {
    themes().ok()?.iter().find(|t| t.name == name)
}

/// Résultat du chargement d'une langue, rempli une seule fois.
type LoadSlot = OnceLock<Result<Arc<Language>, DataError>>;

/// Langues déjà chargées, partagées entre threads. Une langue n'est décompressée
/// qu'une fois, même si plusieurs threads la demandent en même temps.
#[derive(Default)]
pub struct LanguageCache {
    loaded: Mutex<HashMap<String, Arc<LoadSlot>>>,
}

impl LanguageCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Result<Arc<Language>, DataError> {
        // Le verrou ne couvre que la table : la décompression se fait hors verrou.
        let slot = {
            let mut loaded = self.loaded.lock().unwrap_or_else(|p| p.into_inner());
            Arc::clone(loaded.entry(name.to_string()).or_default())
        };
        slot.get_or_init(|| load_language(name).map(Arc::new))
            .clone()
    }

    /// La langue est déjà chargée (sans attendre ni la charger).
    pub fn is_ready(&self, name: &str) -> bool {
        let loaded = self.loaded.lock().unwrap_or_else(|p| p.into_inner());
        loaded.get(name).is_some_and(|slot| slot.get().is_some())
    }
}

static LANGUAGE_GROUPS_JSON: &str = include_str!("../../../assets/language_groups.json");

/// Groupes de langues (`english` → `english`, `english_1k`…), dans l'ordre de Monkeytype.
pub fn language_groups() -> Result<&'static [LanguageGroup], DataError> {
    static CELL: OnceLock<Result<Vec<LanguageGroup>, String>> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(LANGUAGE_GROUPS_JSON).map_err(|e| e.to_string()))
        .as_deref()
        .map_err(|e| DataError::Corrupt(format!("language_groups.json : {e}")))
}
