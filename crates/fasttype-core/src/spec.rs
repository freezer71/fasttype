//! Description d'un test : mode, sous-option, langue, options.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Time,
    Words,
    Quote,
    Zen,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    #[default]
    Normal,
    Expert,
    Master,
}

/// Limite d'un texte custom ; 0 = infini.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "value", rename_all = "lowercase")]
pub enum CustomLimit {
    Word(u32),
    Section(u32),
    Time(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteMeta {
    pub id: u32,
    /// Groupe de longueur : 0 short, 1 medium, 2 long, 3 thicc.
    pub group: u8,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSpec {
    pub mode: Mode,
    /// `getMode2` : durée, nombre de mots, id de citation, "custom" ou "zen".
    pub mode2: String,
    pub language: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub difficulty: Difficulty,
    pub lazy_mode: bool,
    /// Fin au temps (time, custom limité au temps). `Some(0)` = infini.
    pub time_limit: Option<u32>,
    pub custom_limit: Option<CustomLimit>,
    pub quote: Option<QuoteMeta>,
}

impl TestSpec {
    fn base(mode: Mode, mode2: String, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self {
            mode,
            mode2,
            language: language.to_string(),
            punctuation,
            numbers,
            difficulty: Difficulty::Normal,
            lazy_mode: false,
            time_limit: None,
            custom_limit: None,
            quote: None,
        }
    }

    pub fn time(seconds: u32, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self {
            time_limit: Some(seconds),
            ..Self::base(
                Mode::Time,
                seconds.to_string(),
                language,
                punctuation,
                numbers,
            )
        }
    }

    pub fn words(count: u32, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self::base(
            Mode::Words,
            count.to_string(),
            language,
            punctuation,
            numbers,
        )
    }

    /// En quote, punctuation et numbers sont toujours désactivés.
    pub fn quote(meta: QuoteMeta, language: &str) -> Self {
        Self {
            quote: Some(meta.clone()),
            ..Self::base(Mode::Quote, meta.id.to_string(), language, false, false)
        }
    }

    pub fn zen(language: &str) -> Self {
        Self::base(Mode::Zen, "zen".into(), language, false, false)
    }

    pub fn custom(limit: CustomLimit, language: &str, punctuation: bool, numbers: bool) -> Self {
        let time_limit = match limit {
            CustomLimit::Time(s) => Some(s),
            _ => None,
        };
        Self {
            time_limit,
            custom_limit: Some(limit),
            ..Self::base(
                Mode::Custom,
                "custom".into(),
                language,
                punctuation,
                numbers,
            )
        }
    }

    /// `isTimedTest` (stats.ts) : le dernier mot reçoit le crédit partiel.
    pub fn is_timed(&self) -> bool {
        match self.mode {
            Mode::Time => true,
            Mode::Words => self.mode2 == "0",
            Mode::Custom => matches!(
                self.custom_limit,
                Some(CustomLimit::Time(_))
                    | Some(CustomLimit::Word(0))
                    | Some(CustomLimit::Section(0))
            ),
            Mode::Quote | Mode::Zen => false,
        }
    }

    /// Test « long » : le restart rapide est refusé (`utils/quick-restart.ts`).
    pub fn is_long(&self) -> bool {
        let big = |n: u32, threshold: u32| n == 0 || n >= threshold;
        match (self.mode, self.custom_limit) {
            (Mode::Time, _) => big(self.time_limit.unwrap_or(0), 900),
            (Mode::Words, _) => big(self.mode2.parse().unwrap_or(0), 1000),
            (Mode::Custom, Some(CustomLimit::Time(s))) => big(s, 900),
            (Mode::Custom, Some(CustomLimit::Word(n) | CustomLimit::Section(n))) => big(n, 1000),
            _ => false,
        }
    }
}
