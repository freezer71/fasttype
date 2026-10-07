//! Statistiques calculées depuis le journal, fonction par fonction comme
//! `frontend/src/ts/test/events/stats.ts`.

use crate::chars::{CharCounts, count_chars_for, count_words};
use crate::event::apply_event;
use crate::event::{EventKind, EventLog, active_word_index};
use crate::numbers::calculate_wpm;
use crate::numbers::{js_round, round2};
use crate::spec::Mode;
use std::collections::BTreeMap;

fn last_end_ms(log: &EventLog) -> Option<f64> {
    log.events
        .iter()
        .rev()
        .find(|e| e.kind == EventKind::TimerEnd)
        .map(|e| e.ms)
}

fn trims_trailing_idle(log: &EventLog) -> bool {
    log.context.mode == Mode::Zen || log.context.bailed_out
}

/// `getRawLastKeypressToEndMs`.
fn raw_last_keypress_to_end_ms(log: &EventLog) -> f64 {
    let last_key = log
        .events
        .iter()
        .rev()
        .find(|e| matches!(e.kind, EventKind::KeyDown { .. }))
        .map(|e| e.ms);
    match (last_key, last_end_ms(log)) {
        (Some(k), Some(end)) => round2(end - k).max(0.0),
        _ => 0.0,
    }
}

/// `getLastKeypressToEndMs` : toujours 0 en zen.
pub fn last_keypress_to_end_ms(log: &EventLog) -> f64 {
    if log.context.mode == Mode::Zen {
        0.0
    } else {
        raw_last_keypress_to_end_ms(log)
    }
}

/// `getStartToFirstKeypressMs`.
pub fn start_to_first_keypress_ms(log: &EventLog) -> f64 {
    if log.context.mode == Mode::Zen {
        return 0.0;
    }
    let first_key = log
        .events
        .iter()
        .find(|e| matches!(e.kind, EventKind::KeyDown { .. }))
        .map(|e| e.ms);
    let start = log
        .events
        .iter()
        .find(|e| e.kind == EventKind::TimerStart)
        .map(|e| e.ms);
    match (first_key, start) {
        (Some(k), Some(s)) => round2(k - s).max(0.0),
        _ => 0.0,
    }
}

/// `getTimerBoundaries` : grille idéale d'une seconde, plus une borne finale
/// fractionnaire (≥ 0,5 s) pour les tests non chronométrés.
pub fn timer_boundaries(log: &EventLog) -> Vec<f64> {
    let Some(mut end) = last_end_ms(log) else {
        return Vec::new();
    };
    let mut ticks = (end / 1000.0).floor() as u32;
    if trims_trailing_idle(log) {
        let idle = raw_last_keypress_to_end_ms(log);
        if idle < 7000.0 {
            end -= idle;
            ticks = ticks.min((end / 1000.0).floor() as u32);
        }
    }
    let mut boundaries: Vec<f64> = (1..=ticks).map(|i| f64::from(i) * 1000.0).collect();
    if !log.context.timed && js_round(round2(end / 1000.0) % 1.0) >= 0.5 {
        boundaries.push(end);
    }
    boundaries
}

/// `getTestDurationMs` : arrondi au centième de seconde sauf en custom.
pub fn test_duration_ms(log: &EventLog) -> f64 {
    let Some(mut end) = last_end_ms(log) else {
        return 0.0;
    };
    if trims_trailing_idle(log) {
        let idle = raw_last_keypress_to_end_ms(log);
        if idle < 7000.0 {
            end -= idle;
        }
    }
    if log.context.mode != Mode::Custom {
        end = round2(end / 1000.0) * 1000.0;
    }
    end
}

/// `getChars` : comptage de tous les mots jusqu'au mot actif.
pub fn chars(log: &EventLog, count_partial_last_word: bool) -> CharCounts {
    let inputs = log.word_inputs(None);
    let active = active_word_index(&inputs);
    let partial = log.context.timed || log.context.bailed_out || count_partial_last_word;
    count_words(
        inputs.iter().map(|(&i, input)| {
            (
                input.as_str(),
                log.target(i).unwrap_or(input.as_str()),
                i == active,
            )
        }),
        partial,
        log.context.korean,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accuracy {
    pub correct: u32,
    pub incorrect: u32,
    /// 0 sans aucune frappe (comme Monkeytype pour le résultat).
    pub percentage: f64,
}

/// `getAccuracy` : chaque insertion compte, même corrigée ensuite.
pub fn accuracy(log: &EventLog, until_ms: Option<f64>) -> Accuracy {
    let (mut correct, mut incorrect) = (0u32, 0u32);
    for e in &log.events {
        if until_ms.is_some_and(|u| e.ms > u) {
            break;
        }
        if let EventKind::Insert { correct: ok, .. } = e.kind {
            if ok { correct += 1 } else { incorrect += 1 }
        }
    }
    let total = correct + incorrect;
    let percentage = if total == 0 {
        0.0
    } else {
        f64::from(correct) / f64::from(total) * 100.0
    };
    Accuracy {
        correct,
        incorrect,
        percentage,
    }
}

/// `countPerInterval` : nombre d'événements satisfaisant `pred` dans chaque
/// intervalle `]borne précédente, borne]` (le premier inclut 0 ms).
fn count_per_interval(log: &EventLog, pred: impl Fn(&EventKind) -> bool) -> (Vec<u32>, Vec<f64>) {
    let boundaries = timer_boundaries(log);
    let mut counts = Vec::with_capacity(boundaries.len());
    let mut idx = 0;
    for &b in &boundaries {
        let mut n = 0;
        while let Some(e) = log.events.get(idx) {
            if e.ms > b {
                break;
            }
            if pred(&e.kind) {
                n += 1;
            }
            idx += 1;
        }
        counts.push(n);
    }
    (counts, boundaries)
}

fn is_insert(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Insert { .. })
}

/// `getKeypressesPerSecond` : insertions par seconde.
pub fn keypresses_per_second(log: &EventLog) -> Vec<u32> {
    count_per_interval(log, is_insert).0
}

/// `getBurstHistory` : « raw » de chaque intervalle (série burst du graphique).
pub fn burst_history(log: &EventLog) -> Vec<f64> {
    let (counts, boundaries) = count_per_interval(log, is_insert);
    let mut prev = 0.0;
    counts
        .iter()
        .zip(&boundaries)
        .map(|(&n, &b)| {
            let seconds = (b - prev) / 1000.0;
            prev = b;
            js_round(calculate_wpm(f64::from(n), seconds))
        })
        .collect()
}

/// `getErrorCountHistory` : insertions incorrectes par intervalle.
pub fn error_count_history(log: &EventLog) -> Vec<u32> {
    count_per_interval(log, |k| {
        matches!(k, EventKind::Insert { correct: false, .. })
    })
    .0
}

/// `getWpmHistory` : wpm cumulé à chaque borne, avec crédit partiel du mot actif.
pub fn wpm_history(log: &EventLog) -> Vec<f64> {
    history(log, |c| c.correct_word)
}

/// `getRawHistory` : raw cumulé à chaque borne (lettres justes, fausses et en
/// trop), avec crédit partiel du mot actif.
pub fn raw_history(log: &EventLog) -> Vec<f64> {
    history(log, |c| c.all_correct + c.extra + c.incorrect)
}

/// Série cumulée commune à `wpm_history` et `raw_history` : `count` choisit les
/// caractères comptés dans chaque mot.
fn history(log: &EventLog, count: fn(&CharCounts) -> u32) -> Vec<f64> {
    let boundaries = timer_boundaries(log);
    let mut inputs: BTreeMap<u32, String> = BTreeMap::new();
    // Comme Monkeytype (`cachedIfLast` / `cachedIfNotLast`), le compte de chaque
    // mot n'est recalculé que si ses événements ont changé depuis la borne
    // précédente : sans ce cache, le coût est (secondes × mots).
    let mut not_last: BTreeMap<u32, u32> = BTreeMap::new();
    let mut as_last: BTreeMap<u32, u32> = BTreeMap::new();
    let mut not_last_sum: u32 = 0;
    let mut dirty: Vec<u32> = Vec::new();
    let mut idx = 0;
    let mut out = Vec::with_capacity(boundaries.len());
    for &b in &boundaries {
        while let Some(e) = log.events.get(idx) {
            if e.ms > b {
                break;
            }
            apply_event(&mut inputs, &e.kind);
            if let Some(w) = e.kind.word_index() {
                dirty.push(w);
            }
            idx += 1;
        }
        dirty.sort_unstable();
        dirty.dedup();
        for &w in &dirty {
            let input = inputs[&w].as_str();
            let target = log.target(w).unwrap_or(input);
            let fresh = count(&count_chars_for(input, target, false, log.context.korean));
            not_last_sum = not_last_sum - not_last.insert(w, fresh).unwrap_or(0) + fresh;
            as_last.insert(
                w,
                count(&count_chars_for(input, target, true, log.context.korean)),
            );
        }
        dirty.clear();
        // Les mots après le mot actif sont vides (0) ; le mot actif compte
        // avec crédit partiel à la place de son compte « non dernier ».
        let active = active_word_index(&inputs);
        let correct_word = match as_last.get(&active) {
            Some(&last) => not_last_sum - not_last[&active] + last,
            None => not_last_sum,
        };
        out.push(js_round(calculate_wpm(f64::from(correct_word), b / 1000.0)));
    }
    out
}

/// `getAfkDuration` : secondes sans aucun keydown ni événement de saisie.
pub fn afk_duration(log: &EventLog) -> u32 {
    let (counts, _) = count_per_interval(log, |k| {
        matches!(k, EventKind::KeyDown { .. }) || k.word_index().is_some()
    });
    counts.iter().filter(|&&c| c == 0).count() as u32
}

/// AFK de `finish` : aucune insertion pendant les 5 dernières secondes.
/// Jamais en bail out. (Comme `[].every(...)`, une liste vide vaut AFK.)
pub fn afk_detected(log: &EventLog) -> bool {
    if log.context.bailed_out {
        return false;
    }
    keypresses_per_second(log)
        .iter()
        .rev()
        .take(5)
        .all(|&c| c == 0)
}
