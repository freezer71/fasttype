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
    /// Faux si les records n'ont pas pu être chargés ni reconstruits : on ne
    /// réécrit alors jamais leur fichier, pour ne rien perdre.
    pbs_reliable: bool,
}

impl Store {
    /// Lit la config et les records (jamais tout l'historique). Ne échoue jamais.
    pub fn open(paths: Paths) -> Store {
        let (config, mut warnings) = load_config(&paths.config_file());
        let results = ResultLog::new(paths.results_file());
        let pbs_path = paths.pbs_file();
        let (pbs, pbs_reliable) = if !pbs_path.exists() && paths.results_file().exists() {
            // cache absent (supprimé, historique copié d'une autre machine)
            rebuild_from_history(&results, &pbs_path, None, &mut warnings)
        } else {
            match PersonalBests::load(&pbs_path) {
                Ok(pbs) => (pbs, true),
                Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                    rebuild_from_history(&results, &pbs_path, Some(e), &mut warnings)
                }
                Err(e) => {
                    warnings.push(format!("records personnels illisibles ({e}) : non chargés"));
                    (PersonalBests::default(), false)
                }
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
            pbs_reliable,
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
        if self.pbs_reliable && matches!(outcome, PbOutcome::NewBest { .. }) {
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
        self.pbs_reliable = true;
        Ok(history)
    }
}

/// Recalcule les records depuis l'historique et les réécrit. Si l'historique
/// est illisible, rien n'est écrit et les records restent vides en mémoire.
fn rebuild_from_history(
    results: &ResultLog,
    pbs_path: &std::path::Path,
    cause: Option<io::Error>,
    warnings: &mut Vec<String>,
) -> (PersonalBests, bool) {
    let what = match &cause {
        Some(e) => format!("records personnels illisibles ({e})"),
        None => "records personnels absents".to_string(),
    };
    match results.load() {
        Ok(history) => {
            let rebuilt = PersonalBests::rebuild(&history.results);
            let saved = rebuilt.save(pbs_path);
            if cause.is_some() || saved.is_err() {
                warnings.push(format!(
                    "{what} : reconstruits depuis l'historique{}",
                    if saved.is_err() {
                        ", sans pouvoir les enregistrer"
                    } else {
                        ""
                    }
                ));
            }
            (rebuilt, saved.is_ok())
        }
        Err(e) => {
            warnings.push(format!(
                "{what} et historique illisible ({e}) : records non chargés, rien n'est réécrit"
            ));
            (PersonalBests::default(), false)
        }
    }
}
