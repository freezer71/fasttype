//! Journal d'événements d'un test, sur le modèle de
//! `frontend/src/ts/test/events/types.ts`. Toutes les stats en sont déduites.

use crate::spec::Mode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    TimerStart,
    TimerStep {
        second: u32,
    },
    TimerEnd,
    KeyDown {
        code: u32,
    },
    KeyUp {
        code: u32,
    },
    /// Insertion d'un caractère (y compris le séparateur qui valide le mot).
    /// `dropped` : espace qui valide un dernier mot faux ; compté pour la
    /// précision mais pas ajouté à la saisie (`applyInputEvent`, helpers.ts).
    Insert {
        word_index: u32,
        char_index: u32,
        ch: char,
        correct: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        dropped: bool,
    },
    DeleteChar {
        word_index: u32,
    },
    DeleteWord {
        word_index: u32,
    },
}

impl EventKind {
    pub fn word_index(&self) -> Option<u32> {
        match self {
            EventKind::Insert { word_index, .. }
            | EventKind::DeleteChar { word_index }
            | EventKind::DeleteWord { word_index } => Some(*word_index),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestEvent {
    /// Millisecondes depuis le début du test.
    pub ms: f64,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventContext {
    pub mode: Mode,
    /// `isTimedTest`.
    pub timed: bool,
    pub bailed_out: bool,
    /// Mots cibles avec leur séparateur. Vide en zen (la cible est alors la saisie).
    pub target_words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    pub context: EventContext,
    pub events: Vec<TestEvent>,
}

impl EventLog {
    pub fn with_capacity(context: EventContext, capacity: usize) -> Self {
        Self {
            context,
            events: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, ms: f64, kind: EventKind) {
        self.events.push(TestEvent { ms, kind });
    }

    pub fn target(&self, word_index: u32) -> Option<&str> {
        self.context
            .target_words
            .get(word_index as usize)
            .map(String::as_str)
    }

    /// Saisie de chaque mot touché, en rejouant les événements jusqu'à
    /// `until_ms` inclus (`getEventsPerWord` + `getInputFromDom`).
    pub fn word_inputs(&self, until_ms: Option<f64>) -> BTreeMap<u32, String> {
        let mut inputs = BTreeMap::new();
        for e in &self.events {
            if until_ms.is_some_and(|u| e.ms > u) {
                break;
            }
            apply_event(&mut inputs, &e.kind);
        }
        inputs
    }
}

/// Applique un événement de saisie à l'état des mots.
pub fn apply_event(inputs: &mut BTreeMap<u32, String>, kind: &EventKind) {
    match kind {
        EventKind::Insert {
            word_index,
            ch,
            dropped,
            ..
        } => {
            let input = inputs.entry(*word_index).or_default();
            if !dropped {
                input.push(*ch);
            }
        }
        EventKind::DeleteChar { word_index } => {
            inputs.entry(*word_index).or_default().pop();
        }
        EventKind::DeleteWord { word_index } => inputs.entry(*word_index).or_default().clear(),
        _ => {}
    }
}

/// `inferActiveWordIndex` : dernier mot non vide, ou le suivant s'il a été
/// validé par un espace.
pub fn active_word_index(inputs: &BTreeMap<u32, String>) -> u32 {
    match inputs.iter().rev().find(|(_, s)| !s.is_empty()) {
        None => 0,
        Some((&i, s)) if s.ends_with(' ') => i + 1,
        Some((&i, _)) => i,
    }
}
