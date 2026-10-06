//! `countChars` de Monkeytype (`frontend/src/ts/utils/strings.ts`).

use serde::{Deserialize, Serialize};
use std::ops::AddAssign;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharCounts {
    pub all_correct: u32,
    pub correct_word: u32,
    pub incorrect: u32,
    pub extra: u32,
    pub missed: u32,
}

impl AddAssign for CharCounts {
    fn add_assign(&mut self, o: Self) {
        self.all_correct += o.all_correct;
        self.correct_word += o.correct_word;
        self.incorrect += o.incorrect;
        self.extra += o.extra;
        self.missed += o.missed;
    }
}

/// Compare la saisie d'un mot à sa cible (séparateur compris), position par position.
pub fn count_chars(input: &str, target: &str, credit_partial: bool) -> CharCounts {
    let mut c = CharCounts::default();
    let word_correct = input == target;
    let partially_correct = target.starts_with(input);
    let input_has_space = input.contains(' ');
    let mut ii = input.chars();
    let mut ti = target.chars();
    loop {
        match (ii.next(), ti.next()) {
            (None, None) => break,
            (Some(i), Some(t)) if i == t => {
                if t == ' ' && !word_correct {
                    c.extra += 1;
                } else {
                    c.all_correct += 1;
                }
                if word_correct || (credit_partial && partially_correct) {
                    c.correct_word += 1;
                }
            }
            (None, Some(_)) => {
                if !credit_partial {
                    c.missed += 1;
                }
            }
            // au-delà de la cible, ou lettre tapée à la place de l'espace final
            (Some(_), None) => c.extra += 1,
            (Some(_), Some(' ')) if !input_has_space => c.extra += 1,
            _ => c.incorrect += 1,
        }
    }
    c
}

/// Additionne `count_chars` mot par mot et s'arrête après le dernier mot
/// (boucle de `getChars`). Seul le dernier mot peut recevoir le crédit partiel.
pub fn count_words<'a, I>(words: I, credit_partial_last: bool) -> CharCounts
where
    I: IntoIterator<Item = (&'a str, &'a str, bool)>,
{
    let mut total = CharCounts::default();
    for (input, target, last) in words {
        total += count_chars(input, target, last && credit_partial_last);
        if last {
            break;
        }
    }
    total
}
