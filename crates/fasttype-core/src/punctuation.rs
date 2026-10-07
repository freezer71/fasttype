//! `punctuateWord` (words-generator.ts), `english-punctuation.ts`,
//! `getNumbers` (utils/generate.ts) et conversions de chiffres (utils/misc.ts).
//! L'ordre des tirages aléatoires est celui de Monkeytype : chaque branche
//! ne tire que si les précédentes ont échoué.

use crate::rng::RandomSource;

const CONTRACTIONS: &[(&str, &[&str])] = &[
    ("are", &["aren't"]),
    ("can", &["can't"]),
    ("could", &["couldn't"]),
    ("did", &["didn't"]),
    ("does", &["doesn't"]),
    ("do", &["don't"]),
    ("had", &["hadn't"]),
    ("has", &["hasn't"]),
    ("have", &["haven't"]),
    ("is", &["isn't"]),
    ("it", &["it's", "it'll"]),
    ("i", &["i'm", "i'll", "i've", "i'd"]),
    ("you", &["you'll", "you're", "you've", "you'd"]),
    ("that", &["that's", "that'll", "that'd"]),
    ("must", &["mustn't", "must've"]),
    ("there", &["there's", "there'll", "there'd"]),
    ("he", &["he's", "he'll", "he'd"]),
    ("she", &["she's", "she'll", "she'd"]),
    ("we", &["we're", "we'll", "we'd"]),
    ("they", &["they're", "they'll", "they'd"]),
    ("should", &["shouldn't", "should've"]),
    ("was", &["wasn't"]),
    ("were", &["weren't"]),
    ("will", &["won't"]),
    ("would", &["wouldn't", "would've"]),
    ("going", &["goin'"]),
];

const SPECIALS: &[&str] = &["{", "}", "[", "]", "(", ")", ";", "=", "+", "%", "/"];
const SPECIALS_C: &[&str] = &[
    "{", "}", "[", "]", "(", ")", ";", "=", "+", "%", "/", "/*", "*/", "//", "!=", "==", "<=",
    ">=", "||", "&&", "<<", ">>", "%=", "&=", "*=", "++", "+=", "--", "-=", "/=", "^=", "|=",
];

fn pick<'a>(items: &[&'a str], rng: &mut dyn RandomSource) -> &'a str {
    items[rng.below(items.len())]
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `\W` de JavaScript (sans drapeau `u`) : tout sauf `[A-Za-z0-9_]`.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Découpe `word` en (préfixe non-mot, cœur, suffixe non-mot).
fn split_core(word: &str) -> (&str, &str, &str) {
    let start = word.find(is_word_char).unwrap_or(word.len());
    let end = word
        .rfind(is_word_char)
        .map(|i| i + 1)
        .unwrap_or(start)
        .max(start);
    (&word[..start], &word[start..end], &word[end..])
}

fn contraction_for(word: &str) -> Option<&'static [&'static str]> {
    let (_, core, _) = split_core(word);
    CONTRACTIONS
        .iter()
        .find(|(base, _)| core.eq_ignore_ascii_case(base))
        .map(|(_, r)| *r)
}

fn apply_contraction(word: &str, replacements: &[&str], rng: &mut dyn RandomSource) -> String {
    let (prefix, core, suffix) = split_core(word);
    let replacement = pick(replacements, rng);
    let starts_upper = core.chars().next().is_some_and(char::is_uppercase);
    let replaced = if !starts_upper {
        replacement.to_string()
    } else if core != "I" && core == core.to_uppercase() {
        replacement.to_uppercase()
    } else {
        capitalize_first(replacement)
    };
    format!("{prefix}{replaced}{suffix}")
}

/// Ajoute la ponctuation à un mot ; garde l'état « ¿ / ¡ » de l'espagnol.
#[derive(Debug, Default)]
pub struct Punctuator {
    spanish_closing: Option<char>,
}

impl Punctuator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn punctuate(
        &mut self,
        previous: Option<&str>,
        word: &str,
        index: usize,
        max_index: usize,
        language: &str,
        rng: &mut dyn RandomSource,
    ) -> String {
        let lang = language.split('_').next().unwrap_or(language);
        let last = previous.and_then(|p| p.chars().last());
        let lc = |c: char| last == Some(c);
        let (i, max) = (index as i64, max_index as i64);
        let mut w = word.to_string();

        if lang != "code"
            && lang != "georgian"
            && (index == 0 || matches!(last, Some('?' | '!' | '.' | '؟')))
        {
            w = capitalize_first(&w);
            if lang == "turkish" {
                w = w.replace('I', "İ");
            }
            if lang == "spanish" {
                let r = rng.next_f64();
                if r > 0.9 {
                    w = format!("¿{w}");
                    self.spanish_closing = Some('?');
                } else if r > 0.8 {
                    w = format!("¡{w}");
                    self.spanish_closing = Some('!');
                }
            }
        } else if (rng.next_f64() < 0.1 && !lc('.') && !lc(',') && i != max - 2) || i == max - 1 {
            if lang == "spanish" {
                if let Some(c) = self.spanish_closing.take() {
                    w.push(c);
                }
            } else {
                let r = rng.next_f64();
                if r <= 0.8 {
                    w.push_str(match lang {
                        "nepali" | "bangla" | "hindi" => "।",
                        "japanese" | "chinese" => "。",
                        _ => ".",
                    });
                } else if r < 0.9 {
                    match lang {
                        "french" => w = "?".into(),
                        "arabic" | "persian" | "urdu" | "kurdish" => w.push('؟'),
                        "greek" => w.push(';'),
                        "japanese" | "chinese" => w.push('？'),
                        _ => w.push('?'),
                    }
                } else {
                    match lang {
                        "french" => w = "!".into(),
                        "japanese" | "chinese" => w.push('！'),
                        _ => w.push('!'),
                    }
                }
            }
        } else if rng.next_f64() < 0.01 && !lc(',') && !lc('.') && lang != "russian" {
            w = format!("\"{w}\"");
        } else if rng.next_f64() < 0.011
            && !lc(',')
            && !lc('.')
            && !matches!(lang, "russian" | "ukrainian" | "slovak")
        {
            w = format!("'{w}'");
        } else if rng.next_f64() < 0.012 && !lc(',') && !lc('.') {
            if lang == "code" {
                let mut brackets = vec!["()", "{}", "[]", "<>"];
                if language.starts_with("code_javascript") {
                    brackets.push("``");
                }
                let b = pick(&brackets, rng);
                let mut cs = b.chars();
                let (open, close) = (cs.next().unwrap_or('('), cs.next().unwrap_or(')'));
                w = format!("{open}{w}{close}");
            } else if matches!(lang, "japanese" | "chinese") {
                w = format!("（{w}）");
            } else {
                w = format!("({w})");
            }
        } else if rng.next_f64() < 0.013
            && ![',', '.', ';', '؛', ':', '；', '：'].into_iter().any(lc)
        {
            match lang {
                "french" => w = ":".into(),
                "chinese" => w.push('：'),
                _ => w.push(':'),
            }
        } else if rng.next_f64() < 0.014 && !lc(',') && !lc('.') && previous != Some("-") {
            w = "-".into();
        } else if rng.next_f64() < 0.015 && ![',', '.', ';', '؛', '；', '：'].into_iter().any(lc)
        {
            match lang {
                "french" => w = ";".into(),
                // le point médian grec est tombé en désuétude : Monkeytype met un point
                "greek" => w = ".".into(),
                "arabic" | "kurdish" => w.push('؛'),
                "chinese" => w.push('；'),
                _ => w.push(';'),
            }
        } else if rng.next_f64() < 0.2 && !lc(',') {
            match lang {
                "arabic" | "urdu" | "persian" | "kurdish" => w.push('،'),
                "japanese" => w.push('、'),
                "chinese" => w.push('，'),
                _ => w.push(','),
            }
        } else if rng.next_f64() < 0.25 && lang == "code" {
            let c_like = (language.starts_with("code_c") && !language.starts_with("code_css"))
                || language.starts_with("code_arduino");
            w = if c_like {
                pick(SPECIALS_C, rng).to_string()
            } else if language.starts_with("code_javascript") {
                let mut js = SPECIALS.to_vec();
                js.push("`");
                pick(&js, rng).to_string()
            } else {
                pick(SPECIALS, rng).to_string()
            };
        } else if rng.next_f64() < 0.5
            && lang == "english"
            && let Some(replacements) = contraction_for(&w)
        {
            w = apply_contraction(&w, replacements, rng);
        }

        if w.contains('\t') {
            w.retain(|c| c != '\t');
            w.push('\t');
        }
        if w.contains('\n') {
            w.retain(|c| c != '\n');
            w.push('\n');
        }
        w
    }
}

/// `getNumbers(len)` : 1 à `max_len` chiffres, le premier non nul.
pub fn get_numbers(max_len: u32, rng: &mut dyn RandomSource) -> String {
    let len = rng.int_in(1, max_len);
    let mut out = String::with_capacity(len as usize);
    for i in 0..len {
        let digit = if i == 0 {
            rng.int_in(1, 9)
        } else {
            rng.int_in(0, 9)
        };
        out.push(char::from_digit(digit, 10).unwrap_or('0'));
    }
    out
}

/// Chiffres arabes-indiens, devanagari ou bengali selon la langue.
pub fn localize_digits(digits: &str, language: &str) -> String {
    let table: &[char; 10] = if language.starts_with("kurdish") {
        &['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩']
    } else if language.starts_with("nepali") || language.starts_with("hindi") {
        &['०', '१', '२', '३', '४', '५', '६', '७', '८', '९']
    } else if language.starts_with("bangla") {
        &['০', '১', '২', '৩', '৪', '৫', '৬', '৭', '৮', '৯']
    } else {
        return digits.to_string();
    };
    digits
        .chars()
        .map(|c| c.to_digit(10).map_or(c, |d| table[d as usize]))
        .collect()
}
