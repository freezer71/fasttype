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
pub fn count_words<'a, I>(words: I, credit_partial_last: bool, korean: bool) -> CharCounts
where
    I: IntoIterator<Item = (&'a str, &'a str, bool)>,
{
    let mut total = CharCounts::default();
    for (input, target, last) in words {
        total += count_chars_for(input, target, last && credit_partial_last, korean);
        if last {
            break;
        }
    }
    total
}

/// Espaces Unicode tapables (`SPACE_CODE_POINTS`, utils/strings.ts).
const SPACE_CODE_POINTS: &[char] = &[
    '\u{0020}', '\u{2002}', '\u{2003}', '\u{2009}', '\u{3000}', '\u{00A0}', '\u{1680}', '\u{202F}',
    '\u{FEFF}', '\u{2007}', '\u{2008}', '\u{2004}', '\u{200A}', '\u{200B}',
];

/// Caractères typographiquement équivalents (`CHAR_EQUIVALENCE_SETS`).
const CHAR_EQUIVALENCE_SETS: &[&[char]] = &[
    &['’', '‘', '\'', 'ʼ', '׳', 'ʻ', '᾽'],
    &['"', '”', '“', '„'],
    &['–', '—', '-', '‐', '‑'],
    &[',', '‚'],
];

/// Équivalences propres à une langue (`LANGUAGE_EQUIVALENCE_SETS`).
const LANGUAGE_EQUIVALENCE_SETS: &[(&str, &[char])] = &[("russian", &['ё', 'е', 'e'])];

/// `isSpace` : espace Unicode tapable.
pub fn is_space(c: char) -> bool {
    SPACE_CODE_POINTS.contains(&c)
}

/// `areCharactersVisuallyEqual`.
pub fn visually_equal(a: char, b: char, language: &str) -> bool {
    if a == b {
        return true;
    }
    if (a == ' ' || b == ' ') && is_space(a) && is_space(b) {
        return true;
    }
    if CHAR_EQUIVALENCE_SETS
        .iter()
        .any(|set| set.contains(&a) && set.contains(&b))
    {
        return true;
    }
    let base = crate::result::remove_language_size(language);
    LANGUAGE_EQUIVALENCE_SETS
        .iter()
        .any(|(lang, set)| *lang == base && set.contains(&a) && set.contains(&b))
}

/// `normalizeData` : la frappe prend la forme du caractère attendu quand ils
/// sont équivalents ; toute espace Unicode devient U+0020.
pub fn normalize_typed(typed: char, target: Option<char>, language: &str) -> char {
    if let Some(t) = target
        && visually_equal(typed, t, language)
    {
        return t;
    }
    if is_space(typed) { ' ' } else { typed }
}

/// Initiales (jamo de compatibilité), dans l'ordre Unicode des syllabes.
const CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];
/// Voyelles médianes.
const JUNG: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
    'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];
/// Finales (la première case : pas de finale).
const JONG: [Option<char>; 28] = [
    None,
    Some('ㄱ'),
    Some('ㄲ'),
    Some('ㄳ'),
    Some('ㄴ'),
    Some('ㄵ'),
    Some('ㄶ'),
    Some('ㄷ'),
    Some('ㄹ'),
    Some('ㄺ'),
    Some('ㄻ'),
    Some('ㄼ'),
    Some('ㄽ'),
    Some('ㄾ'),
    Some('ㄿ'),
    Some('ㅀ'),
    Some('ㅁ'),
    Some('ㅂ'),
    Some('ㅄ'),
    Some('ㅅ'),
    Some('ㅆ'),
    Some('ㅇ'),
    Some('ㅈ'),
    Some('ㅊ'),
    Some('ㅋ'),
    Some('ㅌ'),
    Some('ㅍ'),
    Some('ㅎ'),
];

/// Jamo composés tapés en deux touches (`COMPLEX_CONSONANTS` / `COMPLEX_VOWELS`
/// de hangul-js). Les consonnes doubles (ㄲ, ㄸ…) restent d'un seul tenant.
fn split_compound(c: char) -> Option<[char; 2]> {
    Some(match c {
        'ㄳ' => ['ㄱ', 'ㅅ'],
        'ㄵ' => ['ㄴ', 'ㅈ'],
        'ㄶ' => ['ㄴ', 'ㅎ'],
        'ㄺ' => ['ㄹ', 'ㄱ'],
        'ㄻ' => ['ㄹ', 'ㅁ'],
        'ㄼ' => ['ㄹ', 'ㅂ'],
        'ㄽ' => ['ㄹ', 'ㅅ'],
        'ㄾ' => ['ㄹ', 'ㅌ'],
        'ㄿ' => ['ㄹ', 'ㅍ'],
        'ㅀ' => ['ㄹ', 'ㅎ'],
        'ㅄ' => ['ㅂ', 'ㅅ'],
        'ㅘ' => ['ㅗ', 'ㅏ'],
        'ㅙ' => ['ㅗ', 'ㅐ'],
        'ㅚ' => ['ㅗ', 'ㅣ'],
        'ㅝ' => ['ㅜ', 'ㅓ'],
        'ㅞ' => ['ㅜ', 'ㅔ'],
        'ㅟ' => ['ㅜ', 'ㅣ'],
        'ㅢ' => ['ㅡ', 'ㅣ'],
        _ => return None,
    })
}

fn push_jamo(out: &mut String, c: char) {
    match split_compound(c) {
        Some([a, b]) => {
            out.push(a);
            out.push(b);
        }
        None => out.push(c),
    }
}

/// `Hangul.disassemble(s).join("")` : chaque syllabe devient ses jamo, et les
/// jamo composés sont scindés ; les autres caractères sont gardés tels quels.
pub fn hangul_disassemble(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        let code = c as u32;
        if (0xAC00..=0xD7A3).contains(&code) {
            let i = code - 0xAC00;
            push_jamo(&mut out, CHO[(i / 588) as usize]);
            push_jamo(&mut out, JUNG[((i % 588) / 28) as usize]);
            if let Some(t) = JONG[(i % 28) as usize] {
                push_jamo(&mut out, t);
            }
        } else {
            push_jamo(&mut out, c);
        }
    }
    out
}

/// Détection de `koreanStatus` (test-logic.ts).
pub fn contains_korean(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c as u32, 0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xD7B0..=0xD7FF)
    })
}

/// `countChars`, précédé de la décomposition en jamo quand le test est coréen.
pub fn count_chars_for(
    input: &str,
    target: &str,
    credit_partial: bool,
    korean: bool,
) -> CharCounts {
    if korean {
        count_chars(
            &hangul_disassemble(input),
            &hangul_disassemble(target),
            credit_partial,
        )
    } else {
        count_chars(input, target, credit_partial)
    }
}
