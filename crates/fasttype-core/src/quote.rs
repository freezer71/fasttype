//! Fichiers de citations Monkeytype (`frontend/static/quotes/<langue>.json`)
//! et normalisation du texte (`words-generator.ts`).

use crate::rng::RandomSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuoteLength {
    All,
    Short,
    Medium,
    Long,
    Thicc,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Quote {
    pub id: u32,
    pub text: String,
    pub source: String,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct QuoteFile {
    pub language: String,
    /// Bornes incluses de chaque groupe : short, medium, long, thicc.
    pub groups: Vec<[u32; 2]>,
    pub quotes: Vec<Quote>,
}

impl QuoteFile {
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Groupe tel que `lower ≤ length ≤ upper`.
    pub fn group_of(&self, quote: &Quote) -> Option<u8> {
        self.groups
            .iter()
            .position(|[lo, hi]| *lo <= quote.length && quote.length <= *hi)
            .map(|i| i as u8)
    }

    pub fn pick(&self, length: QuoteLength, rng: &mut dyn RandomSource) -> Option<&Quote> {
        let wanted = match length {
            QuoteLength::All => None,
            QuoteLength::Short => Some(0),
            QuoteLength::Medium => Some(1),
            QuoteLength::Long => Some(2),
            QuoteLength::Thicc => Some(3),
        };
        let candidates: Vec<&Quote> = self
            .quotes
            .iter()
            .filter(|q| wanted.is_none() || self.group_of(q) == wanted)
            .collect();
        if candidates.is_empty() {
            return None;
        }
        Some(candidates[rng.below(candidates.len())])
    }

    pub fn by_id(&self, id: u32) -> Option<&Quote> {
        self.quotes.iter().find(|q| q.id == id)
    }
}

/// Espaces multiples réduits, chaque saut de ligne (avec ses espaces autour)
/// remplacé par `"\n "`, `…` remplacé par `...`, puis `trim`.
pub fn normalize_quote_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' => pending_space = true,
            '\r' | '\n' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                pending_space = false;
                out.push_str("\n ");
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            _ => {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                out.push(c);
            }
        }
    }
    out.replace('…', "...").trim().to_string()
}

/// Mots d'une citation (`textSplit`), séparés par des espaces.
pub fn quote_words(text: &str) -> Vec<String> {
    normalize_quote_text(text)
        .split(' ')
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}
