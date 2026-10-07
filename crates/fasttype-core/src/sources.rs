//! Sources de mots bruts (avant ponctuation, nombres et séparateur), sur le
//! modèle de `getNextWord` (words-generator.ts) et de `Wordset` (wordset.ts).

use crate::rng::{RandomSource, shuffle};
use crate::spec::CustomLimit;
use std::collections::VecDeque;
use std::sync::Arc;

pub trait WordSource: Send {
    /// Mot brut suivant. `prev` et `prev2` sont les deux derniers mots générés
    /// (avec leur ponctuation). `None` : la source est épuisée.
    fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String>;
    /// `areAllWordsGenerated`, sachant que `generated` mots ont été produits.
    fn all_generated(&self, generated: usize) -> bool;
    /// `getLimit` : nombre de mots générés au départ.
    fn initial_limit(&self) -> usize;
}

fn strip_lower(word: &str, removed: &str) -> String {
    word.chars()
        .filter(|c| !removed.contains(*c))
        .collect::<String>()
        .to_lowercase()
}

fn first_token(word: &str) -> &str {
    word.split(' ').next().unwrap_or(word)
}

/// Découpe une entrée (mot ou section) en mots, espaces multiples ignorés.
fn split_entry(entry: &str, queue: &mut VecDeque<String>) {
    queue.extend(entry.split(' ').filter(|w| !w.is_empty()).map(String::from));
}

/// Tire un index en rejetant les entrées égales aux deux mots précédents
/// (100 tentatives au plus), ainsi que celles refusées par `extra_reject`.
/// Reproduit une bizarrerie de Monkeytype : seul le premier tirage est
/// comparé en minuscules.
fn pick_avoiding_previous(
    words: &[String],
    prev: &str,
    prev2: &str,
    rng: &mut dyn RandomSource,
    extra_reject: impl Fn(&str) -> bool,
) -> usize {
    let prev_raw = strip_lower(prev, ".?!\":-,");
    let prev2_raw = strip_lower(prev2, ".?!\":-,'");
    let mut idx = rng.below(words.len());
    let mut key = first_token(&words[idx]).to_lowercase();
    let mut tries = 0;
    while tries < 100 && (prev_raw == key || prev2_raw == key || extra_reject(&words[idx])) {
        tries += 1;
        idx = rng.below(words.len());
        key = first_token(&words[idx]).to_string();
    }
    idx
}

/// Mots tirés au hasard dans la liste d'une langue (modes time et words).
pub struct RandomWords {
    words: Arc<Vec<String>>,
    language: String,
    punctuation: bool,
    numbers: bool,
    max_words: Option<u32>,
    queue: VecDeque<String>,
}

impl RandomWords {
    /// `max_words` : `Some(n > 0)` en mode words ; `None` ou `Some(0)` = infini.
    pub fn new(
        words: Arc<Vec<String>>,
        language: &str,
        punctuation: bool,
        numbers: bool,
        max_words: Option<u32>,
    ) -> Self {
        Self {
            words,
            language: language.to_string(),
            punctuation,
            numbers,
            max_words,
            queue: VecDeque::new(),
        }
    }

    fn rejected(&self, word: &str) -> bool {
        (!self.punctuation && word == "I")
            || (!self.punctuation
                && !self.language.starts_with("code")
                && word.chars().any(|c| "-=_+[]{};'\\:\"|,./<>?".contains(c)))
            || (!self.numbers && word.chars().any(|c| c.is_ascii_digit()))
    }

    /// Hors ponctuation, les majuscules sont abaissées (sauf allemand, code, klingon).
    fn finish(&self, word: String) -> String {
        let keeps_case = ["german", "swiss_german", "code", "klingon"]
            .iter()
            .any(|p| self.language.starts_with(p));
        if !self.punctuation && !keeps_case && word.chars().any(|c| c.is_ascii_uppercase()) {
            word.to_lowercase()
        } else {
            word
        }
    }
}

impl WordSource for RandomWords {
    fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String> {
        if self.queue.is_empty() {
            if self.words.is_empty() {
                return None;
            }
            let idx = pick_avoiding_previous(&self.words, prev, prev2, rng, |w| self.rejected(w));
            split_entry(&self.words[idx], &mut self.queue);
        }
        let word = self.queue.pop_front()?;
        Some(self.finish(word))
    }

    fn all_generated(&self, generated: usize) -> bool {
        matches!(self.max_words, Some(n) if n > 0 && generated >= n as usize)
    }

    fn initial_limit(&self) -> usize {
        match self.max_words {
            Some(n) if n > 0 => (n as usize).min(100),
            _ => 100,
        }
    }
}

/// Suite finie de mots, dans l'ordre (citations).
pub struct SequenceWords {
    words: Vec<String>,
    pos: usize,
}

impl SequenceWords {
    pub fn new(words: Vec<String>) -> Self {
        Self { words, pos: 0 }
    }
}

impl WordSource for SequenceWords {
    fn next_raw(&mut self, _: &str, _: &str, _: &mut dyn RandomSource) -> Option<String> {
        let word = self.words.get(self.pos)?.clone();
        self.pos += 1;
        Some(word)
    }

    fn all_generated(&self, generated: usize) -> bool {
        generated >= self.words.len()
    }

    fn initial_limit(&self) -> usize {
        self.words.len().min(100)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomMode {
    Repeat,
    Shuffle,
    Random,
}

/// Texte custom : mots (ou sections avec `|`), répétés, mélangés ou tirés au hasard.
pub struct CustomWords {
    items: Vec<String>,
    mode: CustomMode,
    limit: CustomLimit,
    pos: usize,
    bag: Vec<usize>,
    queue: VecDeque<String>,
    sections: u32,
    last_sections: [Option<usize>; 2],
}

impl CustomWords {
    pub fn new(text: &str, mode: CustomMode, limit: CustomLimit, pipe: bool) -> Self {
        let items: Vec<String> = if pipe {
            text.split('|')
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            text.split_whitespace().map(String::from).collect()
        };
        Self {
            items,
            mode,
            limit,
            pos: 0,
            bag: Vec::new(),
            queue: VecDeque::new(),
            sections: 0,
            last_sections: [None; 2],
        }
    }

    fn next_index(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> usize {
        let len = self.items.len();
        match self.mode {
            CustomMode::Repeat => {
                if self.pos >= len {
                    self.pos = 0;
                }
                self.pos += 1;
                self.pos - 1
            }
            CustomMode::Random if len < 4 => rng.below(len),
            CustomMode::Shuffle => {
                if self.bag.is_empty() {
                    self.bag = (0..len).collect();
                    shuffle(&mut self.bag, rng);
                }
                self.bag.pop().unwrap_or(0)
            }
            CustomMode::Random if matches!(self.limit, CustomLimit::Section(_)) => {
                let same_as_recent = |i: usize, last: &[Option<usize>; 2], items: &[String]| {
                    last.iter().flatten().any(|&p| items[p] == items[i])
                };
                let mut idx = rng.below(len);
                let mut tries = 0;
                while tries < 100 && same_as_recent(idx, &self.last_sections, &self.items) {
                    tries += 1;
                    idx = rng.below(len);
                }
                idx
            }
            // Les rejets « I », symboles et chiffres ne s'appliquent pas en custom.
            CustomMode::Random => pick_avoiding_previous(&self.items, prev, prev2, rng, |_| false),
        }
    }
}

impl WordSource for CustomWords {
    fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String> {
        if self.queue.is_empty() {
            if self.items.is_empty() {
                return None;
            }
            let idx = self.next_index(prev, prev2, rng);
            self.last_sections = [Some(idx), self.last_sections[0]];
            self.sections += 1;
            split_entry(&self.items[idx], &mut self.queue);
        }
        self.queue.pop_front()
    }

    fn all_generated(&self, generated: usize) -> bool {
        match self.limit {
            CustomLimit::Word(n) if n > 0 => generated >= n as usize,
            CustomLimit::Section(n) if n > 0 => self.sections >= n && self.queue.is_empty(),
            _ => false,
        }
    }

    fn initial_limit(&self) -> usize {
        match self.limit {
            CustomLimit::Word(n) | CustomLimit::Section(n) if n > 0 => (n as usize).min(100),
            _ => 100,
        }
    }
}
