//! Résultat d'un test (`buildCompletedEvent`) et règles d'invalidation
//! (`finish`, test-logic.ts).

use crate::event::EventLog;
use crate::numbers::{calculate_wpm, consistency, round2};
use crate::spec::{CustomLimit, Difficulty, Mode, TestSpec};
use crate::stats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartData {
    /// wpm cumulé à chaque seconde.
    pub wpm: Vec<f64>,
    /// raw cumulé à chaque seconde (absent des résultats enregistrés avant la v0.2).
    #[serde(default)]
    pub raw: Vec<f64>,
    /// « raw » de chaque seconde.
    pub burst: Vec<f64>,
    /// Erreurs de chaque seconde.
    pub err: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invalid {
    TooShort,
    Afk,
    Repeated,
    Wpm,
    Raw,
    Accuracy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PbKey {
    pub mode: Mode,
    pub mode2: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub language: String,
    pub difficulty: Difficulty,
    pub lazy_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub timestamp: u64,
    pub mode: Mode,
    pub mode2: String,
    pub language: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub difficulty: Difficulty,
    pub lazy_mode: bool,
    pub wpm: f64,
    pub raw: f64,
    pub acc: f64,
    pub consistency: f64,
    /// `[correct, incorrect, extra, missed]`.
    pub char_stats: [u32; 4],
    /// Secondes.
    pub test_duration: f64,
    pub afk_duration: u32,
    pub afk_detected: bool,
    pub bailed_out: bool,
    pub quote_id: Option<u32>,
    pub quote_length: Option<u8>,
    pub chart: ChartData,
    pub invalid: Option<Invalid>,
}

impl TestResult {
    pub fn pb_key(&self) -> PbKey {
        PbKey {
            mode: self.mode,
            mode2: self.mode2.clone(),
            punctuation: self.punctuation,
            numbers: self.numbers,
            language: self.language.clone(),
            difficulty: self.difficulty,
            lazy_mode: self.lazy_mode,
        }
    }

    /// Un résultat invalide est affiché mais jamais enregistré.
    pub fn is_saveable(&self) -> bool {
        self.invalid.is_none()
    }

    /// `getPbEligibility` (v1) : enregistrable, pas une citation, pas un bail out.
    pub fn pb_eligible(&self) -> bool {
        self.is_saveable() && self.mode != Mode::Quote && !self.bailed_out
    }
}

/// `removeLanguageSize` : retire un suffixe de taille `_<n>k` (`english_1k` → `english`).
pub fn remove_language_size(language: &str) -> String {
    if let Some((base, size)) = language.rsplit_once('_')
        && let Some(digits) = size.strip_suffix('k')
        && digits.chars().all(|c| c.is_ascii_digit())
    {
        return base.to_string();
    }
    language.to_string()
}

pub fn build_result(log: &EventLog, spec: &TestSpec, repeated: bool, timestamp: u64) -> TestResult {
    let chars = stats::chars(log, false);
    let duration = stats::test_duration_ms(log) / 1000.0;
    let burst = stats::burst_history(log);
    let language = if spec.mode == Mode::Quote {
        remove_language_size(&spec.language)
    } else {
        spec.language.clone()
    };
    let mut r = TestResult {
        timestamp,
        mode: spec.mode,
        mode2: spec.mode2.clone(),
        language,
        punctuation: spec.punctuation,
        numbers: spec.numbers,
        difficulty: spec.difficulty,
        lazy_mode: spec.lazy_mode,
        wpm: round2(calculate_wpm(f64::from(chars.correct_word), duration)),
        raw: round2(calculate_wpm(
            f64::from(chars.all_correct + chars.incorrect + chars.extra),
            duration,
        )),
        acc: round2(stats::accuracy(log, None).percentage),
        consistency: consistency(&burst),
        char_stats: [
            chars.correct_word,
            chars.incorrect,
            chars.extra,
            chars.missed,
        ],
        test_duration: duration,
        afk_duration: stats::afk_duration(log),
        afk_detected: stats::afk_detected(log),
        bailed_out: log.context.bailed_out,
        quote_id: spec.quote.as_ref().map(|q| q.id),
        quote_length: spec.quote.as_ref().map(|q| q.group),
        chart: ChartData {
            wpm: stats::wpm_history(log),
            raw: stats::raw_history(log),
            burst,
            err: stats::error_count_history(log),
        },
        invalid: None,
    };
    r.invalid = invalid_reason(&r, spec, repeated);
    r
}

/// Ordre des contrôles de `finish`. Les bizarreries de Monkeytype sont
/// reproduites telles quelles :
/// - une limite custom en mots/sections < 10 (y compris 0 = infini) ou en temps < 15 rend le test « trop court » ;
/// - la limite wpm de 350 ne s'applique pas au mode words (seul words 10 a la sienne : 420) ;
/// - une citation répétée reste valide (`setIsRepeated(false)` en quote).
fn invalid_reason(r: &TestResult, spec: &TestSpec, repeated: bool) -> Option<Invalid> {
    let mode2: Option<i64> = r.mode2.parse().ok();
    let d = r.test_duration;
    let too_short = d < 1.0
        || (r.mode == Mode::Time && mode2.is_some_and(|m| m > 0 && m < 15))
        || (r.mode == Mode::Time && mode2 == Some(0) && d < 15.0)
        || (r.mode == Mode::Words && mode2.is_some_and(|m| m > 0 && m < 10))
        || (r.mode == Mode::Words && mode2 == Some(0) && d < 15.0)
        || (r.mode == Mode::Custom
            && matches!(spec.custom_limit, Some(CustomLimit::Word(n) | CustomLimit::Section(n)) if n < 10))
        || (r.mode == Mode::Custom
            && matches!(spec.custom_limit, Some(CustomLimit::Time(n)) if n < 15))
        || (r.mode == Mode::Zen && d < 15.0);
    let words10 = r.mode == Mode::Words && r.mode2 == "10";
    let speed_invalid = |v: f64| {
        v < 0.0 || (v > 350.0 && r.mode != Mode::Words && r.mode2 != "10") || (v > 420.0 && words10)
    };
    if too_short {
        Some(Invalid::TooShort)
    } else if r.afk_detected {
        Some(Invalid::Afk)
    } else if repeated && r.mode != Mode::Quote {
        Some(Invalid::Repeated)
    } else if speed_invalid(r.wpm) {
        Some(Invalid::Wpm)
    } else if speed_invalid(r.raw) {
        Some(Invalid::Raw)
    } else if r.acc < 75.0 || r.acc > 100.0 {
        Some(Invalid::Accuracy)
    } else {
        None
    }
}
