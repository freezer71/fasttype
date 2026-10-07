//! Chaîne de génération de `getNextWord` : source → ß suisse → ponctuation
//! → nombres → séparateur (`appendCommitCharacter`).

use crate::punctuation::{Punctuator, get_numbers, localize_digits};
use crate::rng::RandomSource;
use crate::sources::{SequenceWords, WordSource};

pub struct WordGenerator {
    source: Box<dyn WordSource>,
    language: String,
    punctuation: bool,
    numbers: bool,
    punctuator: Punctuator,
    prev: String,
    prev2: String,
    generated: usize,
}

impl WordGenerator {
    pub fn new(
        source: Box<dyn WordSource>,
        language: &str,
        punctuation: bool,
        numbers: bool,
    ) -> Self {
        Self {
            source,
            language: language.to_string(),
            punctuation,
            numbers,
            punctuator: Punctuator::new(),
            prev: String::new(),
            prev2: String::new(),
            generated: 0,
        }
    }

    /// Générateur sans mots (zen, ou liste introuvable).
    pub fn empty() -> Self {
        Self::new(
            Box::new(SequenceWords::new(Vec::new())),
            "english",
            false,
            false,
        )
    }

    /// Mot suivant avec son séparateur. `index` est sa position dans le test ;
    /// `bound` est le `wordsBound` de Monkeytype (le dernier mot avant la borne
    /// finit toujours une phrase en ponctuation).
    pub fn next(
        &mut self,
        index: usize,
        bound: usize,
        rng: &mut dyn RandomSource,
    ) -> Option<String> {
        let mut word = self.source.next_raw(&self.prev, &self.prev2, rng)?;
        if self.language.starts_with("swiss_german") {
            word = word.replace('ß', "ss");
        }
        if self.punctuation {
            let previous = (!self.prev.is_empty()).then_some(self.prev.as_str());
            word = self
                .punctuator
                .punctuate(previous, &word, index, bound, &self.language, rng);
        }
        if self.numbers && rng.next_f64() < 0.1 {
            word = localize_digits(&get_numbers(4, rng), &self.language);
        }
        self.generated += 1;
        self.prev2 = std::mem::replace(&mut self.prev, word.clone());
        if !word.ends_with('\n') {
            word.push(' ');
        }
        Some(word)
    }

    pub fn all_generated(&self) -> bool {
        self.source.all_generated(self.generated)
    }

    pub fn initial_limit(&self) -> usize {
        self.source.initial_limit()
    }
}
