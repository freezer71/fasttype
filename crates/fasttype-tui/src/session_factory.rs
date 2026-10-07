//! Construction d'un test à partir de la config : mode, langue, citation.
//! Les langues et les citations chargées restent en mémoire.

use fasttype_core::generator::WordGenerator;
use fasttype_core::quote::{Quote, QuoteFile, quote_words};
use fasttype_core::rng::{RandomSource, SplitMix64};
use fasttype_core::session::TestSession;
use fasttype_core::sources::{CustomMode, CustomWords, RandomWords, SequenceWords};
use fasttype_core::spec::{CustomLimit, QuoteMeta, TestSpec};
use fasttype_data::{DEFAULT_LANGUAGE, LanguageCache, quotes_for};
use fasttype_store::Config;
use std::collections::HashMap;
use std::sync::Arc;

/// Texte custom par défaut de Monkeytype.
pub const DEFAULT_CUSTOM_TEXT: &str = "The quick brown fox jumps over the lazy dog";

pub struct Built {
    pub session: TestSession,
    /// Repli effectué (langue ou citations introuvables), à notifier.
    pub warning: Option<String>,
}

#[derive(Default)]
pub struct SessionFactory {
    languages: LanguageCache,
    quotes: HashMap<String, Option<Arc<QuoteFile>>>,
}

/// Citation tirée parmi les groupes de `quoteLength` (0 à 3) ; tous si aucun.
pub fn pick_quote<'a>(
    file: &'a QuoteFile,
    groups: &[i64],
    rng: &mut dyn RandomSource,
) -> Option<&'a Quote> {
    let wanted: Vec<u8> = groups
        .iter()
        .filter(|g| (0..=3).contains(*g))
        .map(|g| *g as u8)
        .collect();
    let candidates: Vec<&Quote> = file
        .quotes
        .iter()
        .filter(|q| wanted.is_empty() || file.group_of(q).is_some_and(|g| wanted.contains(&g)))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.below(candidates.len())])
}

impl SessionFactory {
    pub fn new() -> Self {
        Self::default()
    }

    fn quotes(&mut self, language: &str) -> Option<Arc<QuoteFile>> {
        self.quotes
            .entry(language.to_string())
            .or_insert_with(|| quotes_for(language).ok().flatten().map(Arc::new))
            .clone()
    }

    /// Construit le test décrit par la config. Ne échoue jamais : une langue
    /// inconnue repasse sur `english`, un mode sans citations sur `time`.
    pub fn build(&mut self, config: &Config, seed: u64) -> Built {
        let mut rng = SplitMix64::new(seed);
        let session_rng = Box::new(SplitMix64::new(seed ^ 0x9E37_79B9_7F4A_7C15));
        let mut warning = None;
        let requested = config.str("language");
        let (language, words) = match self.languages.get(requested) {
            Ok(l) => (requested.to_string(), Arc::clone(&l.words)),
            Err(e) => {
                warning = Some(format!("{e} — using {DEFAULT_LANGUAGE}"));
                match self.languages.get(DEFAULT_LANGUAGE) {
                    Ok(l) => (DEFAULT_LANGUAGE.to_string(), Arc::clone(&l.words)),
                    Err(_) => (DEFAULT_LANGUAGE.to_string(), Arc::new(Vec::new())),
                }
            }
        };
        let punctuation = config.bool("punctuation");
        let numbers = config.bool("numbers");
        let random = |max: Option<u32>| {
            let source = RandomWords::new(Arc::clone(&words), &language, punctuation, numbers, max);
            WordGenerator::new(Box::new(source), &language, punctuation, numbers)
        };
        let time = config.int("time").max(0) as u32;
        let (spec, generator) = match config.str("mode") {
            "words" => {
                let n = config.int("words").max(0) as u32;
                (
                    TestSpec::words(n, &language, punctuation, numbers),
                    random(Some(n)),
                )
            }
            "zen" => (TestSpec::zen(&language), WordGenerator::empty()),
            "custom" => {
                let limit =
                    CustomLimit::Word(DEFAULT_CUSTOM_TEXT.split_whitespace().count() as u32);
                let source =
                    CustomWords::new(DEFAULT_CUSTOM_TEXT, CustomMode::Repeat, limit, false);
                (
                    TestSpec::custom(limit, &language, punctuation, numbers),
                    WordGenerator::new(Box::new(source), &language, punctuation, numbers),
                )
            }
            "quote" => {
                let picked = self.quotes(&language).and_then(|file| {
                    let q = pick_quote(&file, &config.int_list("quoteLength"), &mut rng)?;
                    let meta = QuoteMeta {
                        id: q.id,
                        group: file.group_of(q).unwrap_or(0),
                        source: q.source.clone(),
                    };
                    Some((meta, quote_words(&q.text)))
                });
                match picked {
                    Some((meta, words)) => (
                        TestSpec::quote(meta, &language),
                        WordGenerator::new(
                            Box::new(SequenceWords::new(words)),
                            &language,
                            false,
                            false,
                        ),
                    ),
                    None => {
                        warning = Some(format!("no quotes found for {language} — using time mode"));
                        (
                            TestSpec::time(time, &language, punctuation, numbers),
                            random(None),
                        )
                    }
                }
            }
            _ => (
                TestSpec::time(time, &language, punctuation, numbers),
                random(None),
            ),
        };
        Built {
            session: TestSession::new(spec, generator, session_rng),
            warning,
        }
    }
}
