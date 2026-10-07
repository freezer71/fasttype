//! Groupes de langues de Monkeytype (`LanguageGroups`,
//! `frontend/src/ts/constants/languages.ts`) : `english` regroupe
//! `english`, `english_1k`… pour le sélecteur de langue.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageGroup {
    pub name: String,
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for GroupParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "languages.ts, ligne {} : {}", self.line, self.message)
    }
}

impl std::error::Error for GroupParseError {}

const START: &str = "export const LanguageGroups: Record<string, Language[]> = {";

fn err(line: usize, message: impl Into<String>) -> GroupParseError {
    GroupParseError {
        line,
        message: message.into(),
    }
}

/// Une chaîne entre guillemets doubles, sans autre forme acceptée.
fn quoted(item: &str, line: usize) -> Result<String, GroupParseError> {
    item.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .filter(|s| !s.is_empty() && !s.contains('"'))
        .map(String::from)
        .ok_or_else(|| {
            err(
                line,
                format!("nom de langue entre guillemets attendu : {item}"),
            )
        })
}

/// Lit l'objet `LanguageGroups`, dans l'ordre du fichier. Échoue à la moindre
/// forme inattendue plutôt que de deviner.
pub fn parse_language_groups_ts(src: &str) -> Result<Vec<LanguageGroup>, GroupParseError> {
    let start = src
        .find(START)
        .ok_or_else(|| err(0, "objet `LanguageGroups` introuvable"))?;
    let first_line = src[..start].lines().count() + 1;
    let mut groups = Vec::new();
    let mut current: Option<LanguageGroup> = None;
    for (i, raw) in src[start..].lines().enumerate().skip(1) {
        let line_no = first_line + i;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(mut group) = current.take() {
            if line == "]," || line == "]" {
                groups.push(group);
            } else {
                group
                    .languages
                    .push(quoted(line.strip_suffix(',').unwrap_or(line), line_no)?);
                current = Some(group);
            }
            continue;
        }
        if line == "};" {
            return Ok(groups);
        }
        let (key, rest) = line
            .split_once(':')
            .ok_or_else(|| err(line_no, format!("groupe attendu : {line}")))?;
        let name = key.trim().trim_matches('"').to_string();
        let rest = rest.trim();
        if rest == "[" {
            current = Some(LanguageGroup {
                name,
                languages: Vec::new(),
            });
        } else if let Some(items) = rest.strip_prefix('[').and_then(|r| r.strip_suffix("],")) {
            let languages = items
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| quoted(s, line_no))
                .collect::<Result<_, _>>()?;
            groups.push(LanguageGroup { name, languages });
        } else {
            return Err(err(line_no, format!("liste de langues attendue : {line}")));
        }
    }
    Err(err(
        first_line,
        "fin de l'objet `LanguageGroups` introuvable",
    ))
}
