//! Records personnels : le meilleur wpm par configuration (`PbKey`).
//! Cache reconstructible depuis l'historique (`fasttype --rebuild-pbs`).

use crate::fs::write_atomic;
use fasttype_core::result::{PbKey, TestResult};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PbEntry {
    pub wpm: f64,
    pub raw: f64,
    pub acc: f64,
    pub consistency: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PbOutcome {
    NewBest { previous: Option<f64> },
    NotBest { best: f64 },
    Ineligible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PbRecord {
    key: PbKey,
    best: PbEntry,
}

/// Peu d'entrées (une par configuration jouée) : une simple liste suffit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonalBests {
    records: Vec<PbRecord>,
}

impl PersonalBests {
    pub fn get(&self, key: &PbKey) -> Option<&PbEntry> {
        self.records.iter().find(|r| &r.key == key).map(|r| &r.best)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn update(&mut self, r: &TestResult) -> PbOutcome {
        if !r.pb_eligible() {
            return PbOutcome::Ineligible;
        }
        let entry = PbEntry {
            wpm: r.wpm,
            raw: r.raw,
            acc: r.acc,
            consistency: r.consistency,
            timestamp: r.timestamp,
        };
        let key = r.pb_key();
        match self.records.iter_mut().find(|rec| rec.key == key) {
            None => {
                self.records.push(PbRecord { key, best: entry });
                PbOutcome::NewBest { previous: None }
            }
            Some(rec) if r.wpm > rec.best.wpm => {
                let previous = rec.best.wpm;
                rec.best = entry;
                PbOutcome::NewBest {
                    previous: Some(previous),
                }
            }
            Some(rec) => PbOutcome::NotBest { best: rec.best.wpm },
        }
    }

    pub fn rebuild<'a>(results: impl IntoIterator<Item = &'a TestResult>) -> Self {
        let mut pbs = Self::default();
        for r in results {
            pbs.update(r);
        }
        pbs
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        write_atomic(path, &json)
    }
}
