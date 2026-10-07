//! Session de frappe : règles de saisie et de fin de Monkeytype
//! (`input/handlers/*`, `input/helpers/fail-or-finish.ts`, `test/test-logic.ts`).
//! Chaque action est horodatée par l'appelant (`now`, en ms d'une horloge monotone).

use crate::chars::{contains_korean, count_words, normalize_typed};
use crate::event::{EventContext, EventKind, EventLog};
use crate::generator::WordGenerator;
use crate::numbers::{calculate_wpm, js_round};
use crate::result::{TestResult, build_result};
use crate::rng::RandomSource;
use crate::spec::{Mode, TestSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Ready,
    Running,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Completed,
    TimeUp,
    ZenFinished,
    BailedOut,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputOutcome {
    Ignored,
    Inserted { correct: bool },
    Committed { correct: bool, burst: f64 },
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveStats {
    pub seconds: u32,
    pub wpm: f64,
    pub raw: f64,
    pub acc: f64,
}

/// Mots d'avance maintenus pendant le test (`addWord`).
const WORDS_AHEAD: usize = 100;
/// Capacité réservée du journal : un test de plusieurs minutes sans réallocation.
const LOG_CAPACITY: usize = 16_384;

pub struct TestSession {
    spec: TestSpec,
    generator: WordGenerator,
    rng: Box<dyn RandomSource>,
    words: Vec<String>,
    inputs: Vec<String>,
    first_insert: Vec<Option<f64>>,
    active: usize,
    state: SessionState,
    start_at: f64,
    next_tick: u32,
    log: EventLog,
    pending_keydown: Option<u32>,
    has_newlines: bool,
    correct_inputs: u32,
    incorrect_inputs: u32,
    last_burst: Option<f64>,
    end_reason: Option<EndReason>,
    repeated: bool,
}

impl TestSession {
    pub fn new(spec: TestSpec, generator: WordGenerator, rng: Box<dyn RandomSource>) -> Self {
        let mut s = Self::empty(spec, generator, rng);
        if s.is_zen() {
            s.push_word(String::new());
            return s;
        }
        let limit = s.generator.initial_limit();
        for i in 0..limit {
            match s.generator.next(i, limit, s.rng.as_mut()) {
                Some(w) => s.push_word(w),
                None => break,
            }
        }
        s.strip_last_separator_if_done();
        s.detect_korean();
        s
    }

    fn empty(spec: TestSpec, generator: WordGenerator, rng: Box<dyn RandomSource>) -> Self {
        let context = EventContext {
            mode: spec.mode,
            timed: spec.is_timed(),
            bailed_out: false,
            korean: false,
            target_words: Vec::new(),
        };
        Self {
            spec,
            generator,
            rng,
            words: Vec::with_capacity(256),
            inputs: Vec::with_capacity(256),
            first_insert: Vec::with_capacity(256),
            active: 0,
            state: SessionState::Ready,
            start_at: 0.0,
            next_tick: 1,
            log: EventLog::with_capacity(context, LOG_CAPACITY),
            pending_keydown: None,
            has_newlines: false,
            correct_inputs: 0,
            incorrect_inputs: 0,
            last_burst: None,
            end_reason: None,
            repeated: false,
        }
    }

    /// `koreanStatus` : activé si les mots générés au départ contiennent du coréen.
    fn detect_korean(&mut self) {
        self.log.context.korean = self.words.iter().any(|w| contains_korean(w));
    }

    fn is_zen(&self) -> bool {
        self.spec.mode == Mode::Zen
    }

    fn push_word(&mut self, word: String) {
        self.has_newlines |= word.contains('\n');
        self.inputs.push(String::with_capacity(word.len() + 32));
        self.first_insert.push(None);
        self.words.push(word);
    }

    /// `removeCommitCharacterFromLastWord` une fois tous les mots générés.
    fn strip_last_separator_if_done(&mut self) {
        if self.is_zen() || !self.generator.all_generated() {
            return;
        }
        if let Some(last) = self.words.last_mut()
            && last.ends_with(' ')
        {
            last.pop();
        }
    }

    /// `addWord` : un mot de plus, tant qu'on a moins de 100 mots d'avance.
    fn add_word(&mut self) {
        if self.words.len() > self.active + 1 + WORDS_AHEAD || self.generator.all_generated() {
            return;
        }
        let index = self.words.len();
        if let Some(w) = self.generator.next(index, WORDS_AHEAD, self.rng.as_mut()) {
            self.push_word(w);
        }
        self.strip_last_separator_if_done();
    }

    fn ms(&self, now: f64) -> f64 {
        (now - self.start_at).max(0.0)
    }

    fn start(&mut self, now: f64) {
        self.state = SessionState::Running;
        self.start_at = now;
        self.log.push(0.0, EventKind::TimerStart);
        if let Some(code) = self.pending_keydown.take() {
            self.log.push(0.0, EventKind::KeyDown { code });
        }
    }

    fn is_last_word(&self) -> bool {
        !self.is_zen() && self.generator.all_generated() && self.active + 1 == self.words.len()
    }

    pub fn key_down(&mut self, code: u32, now: f64) {
        match self.state {
            SessionState::Ready => self.pending_keydown = Some(code),
            SessionState::Running => {
                self.tick(now);
                if self.state == SessionState::Running {
                    let ms = self.ms(now);
                    self.log.push(ms, EventKind::KeyDown { code });
                }
            }
            SessionState::Finished => {}
        }
    }

    pub fn key_up(&mut self, code: u32, now: f64) {
        if self.state == SessionState::Running {
            let ms = self.ms(now);
            self.log.push(ms, EventKind::KeyUp { code });
        }
    }

    pub fn insert(&mut self, ch: char, now: f64) -> InputOutcome {
        if self.state == SessionState::Running {
            self.tick(now);
        }
        if self.state == SessionState::Finished || self.words.is_empty() {
            return InputOutcome::Ignored;
        }
        let zen = self.is_zen();
        let a = self.active;
        let input_len = self.inputs[a].chars().count();
        let target = if zen {
            None
        } else {
            self.words[a].chars().nth(input_len)
        };
        let ch = normalize_typed(ch, target, &self.spec.language);
        let commit = ch == ' ' || ch == '\n';
        if ch == '\n' && !zen && !self.has_newlines {
            return InputOutcome::Ignored;
        }
        if commit && input_len == 0 {
            return InputOutcome::Ignored;
        }
        if !commit {
            let max = if zen {
                30
            } else {
                self.words[a].chars().count() + 20
            };
            if input_len >= max {
                return InputOutcome::Ignored;
            }
        }
        if self.state == SessionState::Ready {
            self.start(now);
        }
        let ms = self.ms(now);
        let correct = zen || target == Some(ch);
        if correct {
            self.correct_inputs += 1;
        } else {
            self.incorrect_inputs += 1;
        }
        // espace qui valide le dernier mot généré alors qu'il est faux : compté
        // pour la précision mais pas ajouté à la saisie (helpers.ts)
        let dropped = ch == ' ' && !correct && a + 1 == self.words.len();
        if !dropped {
            self.inputs[a].push(ch);
        }
        self.log.push(
            ms,
            EventKind::Insert {
                word_index: a as u32,
                char_index: input_len as u32,
                ch,
                correct,
                dropped,
            },
        );
        if input_len == 0 && self.first_insert[a].is_none() {
            self.first_insert[a] = Some(ms);
        }
        if commit {
            return self.commit(ms, input_len + 1);
        }
        if self.is_last_word() && self.inputs[a] == self.words[a] {
            self.finish_at(ms, EndReason::Completed);
            return InputOutcome::Finished;
        }
        InputOutcome::Inserted { correct }
    }

    /// Validation du mot actif (le séparateur vient d'être inséré).
    fn commit(&mut self, ms: f64, typed_len: usize) -> InputOutcome {
        let a = self.active;
        let zen = self.is_zen();
        let correct = zen || self.inputs[a] == self.words[a];
        // `computeBurst` : longueur saisie, séparateur compris, depuis la 1re lettre
        let len = typed_len as f64;
        let burst = match self.first_insert[a] {
            Some(start) if ms > start => js_round(calculate_wpm(len, (ms - start) / 1000.0)),
            Some(_) => f64::INFINITY,
            None => 0.0,
        };
        self.last_burst = Some(burst);
        if self.is_last_word() {
            self.finish_at(ms, EndReason::Completed);
            return InputOutcome::Finished;
        }
        self.active += 1;
        if zen {
            // après un retour arrière, le mot suivant existe déjà
            if self.active >= self.words.len() {
                self.push_word(String::new());
            }
        } else {
            self.add_word();
        }
        if self.active >= self.words.len() {
            self.finish_at(ms, EndReason::Completed);
            return InputOutcome::Finished;
        }
        InputOutcome::Committed { correct, burst }
    }

    /// Mot précédent rouvrable : seulement s'il est faux (pas de freedom mode en v1).
    /// En zen, la cible est vide : le mot précédent n'est jamais « juste ».
    fn previous_editable_word(&self) -> Option<usize> {
        if self.active == 0 {
            return None;
        }
        let prev = self.active - 1;
        (self.inputs[prev] != self.words[prev]).then_some(prev)
    }

    pub fn backspace(&mut self, now: f64) -> bool {
        if self.state == SessionState::Running {
            self.tick(now);
        }
        if self.state != SessionState::Running {
            return false;
        }
        let ms = self.ms(now);
        let a = self.active;
        if self.inputs[a].pop().is_some() {
            self.log.push(
                ms,
                EventKind::DeleteChar {
                    word_index: a as u32,
                },
            );
            return true;
        }
        let Some(prev) = self.previous_editable_word() else {
            return false;
        };
        self.active = prev;
        self.inputs[prev].pop();
        self.log.push(
            ms,
            EventKind::DeleteChar {
                word_index: prev as u32,
            },
        );
        true
    }

    /// Ctrl/Alt + Backspace.
    pub fn delete_word(&mut self, now: f64) -> bool {
        if self.state == SessionState::Running {
            self.tick(now);
        }
        if self.state != SessionState::Running {
            return false;
        }
        let ms = self.ms(now);
        let a = self.active;
        if !self.inputs[a].is_empty() {
            self.inputs[a].clear();
            self.log.push(
                ms,
                EventKind::DeleteWord {
                    word_index: a as u32,
                },
            );
            return true;
        }
        let Some(prev) = self.previous_editable_word() else {
            return false;
        };
        self.active = prev;
        self.inputs[prev].clear();
        self.log.push(
            ms,
            EventKind::DeleteWord {
                word_index: prev as u32,
            },
        );
        true
    }

    /// Avance le timer jusqu'à `now` : un `TimerStep` par seconde écoulée, et
    /// fin du test à la limite de temps (`timerStep` + `checkIfTimeIsUp`).
    /// Le test se termine à la seconde pile, comme sur une grille idéale.
    pub fn tick(&mut self, now: f64) -> bool {
        let mut ticked = false;
        while self.state == SessionState::Running {
            let due = f64::from(self.next_tick) * 1000.0;
            if self.ms(now) < due {
                break;
            }
            ticked = true;
            self.log.push(
                due,
                EventKind::TimerStep {
                    second: self.next_tick,
                },
            );
            if let Some(limit) = self.spec.time_limit
                && limit > 0
                && self.next_tick >= limit
            {
                self.finish_at(due, EndReason::TimeUp);
                break;
            }
            self.next_tick += 1;
        }
        ticked
    }

    /// Instant (horloge de l'appelant) du prochain tick, pour programmer le réveil.
    pub fn next_tick_at(&self) -> Option<f64> {
        (self.state == SessionState::Running)
            .then(|| self.start_at + f64::from(self.next_tick) * 1000.0)
    }

    /// Secondes entières écoulées au dernier tick (sans calcul : pour le timer).
    pub fn elapsed_seconds(&self) -> u32 {
        self.next_tick.saturating_sub(1)
    }

    /// Précision en direct, tronquée (100 sans frappe) : mise à jour à chaque
    /// entrée sur le site, sans attendre le tick.
    pub fn live_accuracy(&self) -> f64 {
        match self.correct_inputs + self.incorrect_inputs {
            0 => 100.0,
            total => (f64::from(self.correct_inputs) / f64::from(total) * 100.0).floor(),
        }
    }

    /// Stats live du dernier tick (`timerStep`) : wpm et raw arrondis avec
    /// crédit partiel du mot actif, précision tronquée (100 sans frappe).
    /// Coûte un parcours des mots : à appeler une fois par tick, pas par image.
    pub fn live_stats(&self) -> LiveStats {
        let seconds = self.elapsed_seconds();
        let acc = self.live_accuracy();
        if self.words.is_empty() || seconds == 0 {
            return LiveStats {
                seconds,
                wpm: 0.0,
                raw: 0.0,
                acc,
            };
        }
        let zen = self.is_zen();
        let last = self.active.min(self.words.len() - 1);
        let c = count_words(
            (0..=last).map(|i| {
                let input = self.inputs[i].as_str();
                (
                    input,
                    if zen { input } else { self.words[i].as_str() },
                    i == last,
                )
            }),
            true,
            self.log.context.korean,
        );
        let s = f64::from(seconds);
        LiveStats {
            seconds,
            wpm: js_round(calculate_wpm(f64::from(c.correct_word), s)),
            raw: js_round(calculate_wpm(
                f64::from(c.all_correct + c.incorrect + c.extra),
                s,
            )),
            acc,
        }
    }

    /// Shift + Entrée en zen.
    pub fn finish_zen(&mut self, now: f64) {
        if self.is_zen() && self.state == SessionState::Running {
            let ms = self.ms(now);
            self.finish_at(ms, EndReason::ZenFinished);
        }
    }

    /// « Bail out » de la palette : fin immédiate, sans PB.
    pub fn bail_out(&mut self, now: f64) {
        if self.state != SessionState::Running {
            return;
        }
        self.tick(now);
        if self.state == SessionState::Running {
            let ms = self.ms(now);
            self.finish_at(ms, EndReason::BailedOut);
        }
    }

    /// `repeatTest` : mêmes mots, la génération reprend ensuite là où elle
    /// s'était arrêtée. Indisponible en zen.
    pub fn into_repeat(self) -> Option<TestSession> {
        if self.is_zen() {
            return None;
        }
        let mut s = Self::empty(self.spec, self.generator, self.rng);
        for w in self.words {
            s.push_word(w);
        }
        s.detect_korean();
        s.repeated = true;
        Some(s)
    }

    fn finish_at(&mut self, ms: f64, reason: EndReason) {
        self.log.push(ms, EventKind::TimerEnd);
        self.state = SessionState::Finished;
        self.end_reason = Some(reason);
        if reason == EndReason::BailedOut {
            self.log.context.bailed_out = true;
        }
        if !self.is_zen() {
            self.log.context.target_words = self.words.clone();
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn spec(&self) -> &TestSpec {
        &self.spec
    }

    pub fn words(&self) -> &[String] {
        &self.words
    }

    pub fn word(&self, i: usize) -> &str {
        self.words.get(i).map_or("", String::as_str)
    }

    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    pub fn input(&self, i: usize) -> &str {
        self.inputs.get(i).map_or("", String::as_str)
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Un mot est validé quand le caret l'a dépassé.
    pub fn is_committed(&self, i: usize) -> bool {
        i < self.active
            || (self.state == SessionState::Finished
                && i == self.active
                && self.inputs.get(i).is_some_and(|s| !s.is_empty()))
    }

    pub fn last_burst(&self) -> Option<f64> {
        self.last_burst
    }

    pub fn log(&self) -> &EventLog {
        &self.log
    }

    pub fn end_reason(&self) -> Option<EndReason> {
        self.end_reason
    }

    /// Le texte contient des sauts de ligne (Entrée les tape au lieu de relancer).
    pub fn has_newlines(&self) -> bool {
        self.has_newlines
    }

    pub fn is_repeated(&self) -> bool {
        self.repeated
    }

    /// Résultat du test terminé ; `timestamp` en ms Unix (fourni par l'appelant).
    pub fn result(&self, timestamp: u64) -> Option<TestResult> {
        (self.state == SessionState::Finished)
            .then(|| build_result(&self.log, &self.spec, self.repeated, timestamp))
    }
}
