//! Statistiques calculées depuis le journal, fonction par fonction comme
//! `frontend/src/ts/test/events/stats.ts`.

use crate::chars::{CharCounts, count_words};
use crate::event::{EventKind, EventLog, active_word_index};
use crate::numbers::{js_round, round2};
use crate::spec::Mode;

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
