//! Point d'entrée de la persistance pour la TUI.

use crate::config::Config;
use crate::fs::{load_config, save_config};
use crate::paths::Paths;
use crate::pbs::{PbEntry, PbOutcome, PersonalBests};
use crate::results::{LoadedResults, ResultLog};
use crate::texts::{CustomTexts, FavoriteQuotes};
use fasttype_core::result::{PbKey, TestResult};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordOutcome {
    Saved(PbOutcome),
    /// Réglage `resultSaving` désactivé.
    SavingDisabled,
    /// Résultat invalide : affiché, jamais enregistré.
    Invalid,
}

pub struct Store {
    pub paths: Paths,
    pub config: Config,
    /// Messages pour les notifications de la TUI.
    pub warnings: Vec<String>,
    pub custom_texts: CustomTexts,
    pub favorites: FavoriteQuotes,
    results: ResultLog,
    pbs: PersonalBests,
}

impl Store {
    /// Lit la config et les records (jamais tout l'historique). Ne échoue jamais.
    pub fn open(paths: Paths) -> Store {
        let (config, mut warnings) = load_config(&paths.config_file());
        let results = ResultLog::new(paths.results_file());
        let pbs = match PersonalBests::load(&paths.pbs_file()) {
            Ok(pbs) => pbs,
            Err(e) => {
                let rebuilt = results
                    .load()
                    .map(|h| PersonalBests::rebuild(&h.results))
                    .unwrap_or_default();
                let saved = rebuilt.save(&paths.pbs_file());
                warnings.push(format!(
                    "records personnels illisibles ({e}) : reconstruits depuis l'historique{}",
                    if saved.is_err() {
                        ", sans pouvoir les réécrire"
                    } else {
                        ""
                    }
                ));
                rebuilt
            }
        };
        Store {
            custom_texts: CustomTexts::new(paths.custom_texts_dir()),
            favorites: FavoriteQuotes::new(paths.favorites_file()),
            paths,
            config,
            warnings,
            results,
            pbs,
        }
    }

    pub fn save_config(&self) -> io::Result<()> {
        save_config(&self.paths.config_file(), &self.config)
    }

    /// Enregistre un résultat terminé et met à jour les records.
    pub fn record(&mut self, r: &TestResult) -> io::Result<RecordOutcome> {
        if !self.config.bool("resultSaving") {
            return Ok(RecordOutcome::SavingDisabled);
        }
        if !r.is_saveable() {
            return Ok(RecordOutcome::Invalid);
        }
        self.results.append(r)?;
        let outcome = self.pbs.update(r);
        if matches!(outcome, PbOutcome::NewBest { .. }) {
            self.pbs.save(&self.paths.pbs_file())?;
        }
        Ok(RecordOutcome::Saved(outcome))
    }

    pub fn personal_best(&self, key: &PbKey) -> Option<&PbEntry> {
        self.pbs.get(key)
    }

    /// Tout l'historique (lecture complète : à faire hors du fil de l'interface).
    pub fn history(&self) -> io::Result<LoadedResults> {
        self.results.load()
    }

    /// `fasttype --rebuild-pbs` : recalcule et réécrit les records.
    pub fn rebuild_pbs(&mut self) -> io::Result<LoadedResults> {
        let history = self.results.load()?;
        self.pbs = PersonalBests::rebuild(&history.results);
        self.pbs.save(&self.paths.pbs_file())?;
        Ok(history)
    }
}
