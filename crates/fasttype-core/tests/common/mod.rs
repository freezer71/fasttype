#![allow(dead_code)]

use fasttype_core::event::{EventContext, EventKind, EventLog};
use fasttype_core::spec::Mode;
use std::collections::BTreeMap;

/// Construit un journal d'événements à la main, dans l'ordre chronologique.
pub struct LogBuilder {
    log: EventLog,
    t: f64,
    step: f64,
    inputs: BTreeMap<u32, String>,
}

impl LogBuilder {
    /// Journal avec un `TimerStart` à 0 ms et des frappes espacées de 100 ms.
    pub fn new(mode: Mode, timed: bool, targets: &[&str]) -> Self {
        let context = EventContext {
            mode,
            timed,
            bailed_out: false,
            target_words: targets.iter().map(|s| s.to_string()).collect(),
        };
        let mut log = EventLog::with_capacity(context, 256);
        log.push(0.0, EventKind::TimerStart);
        Self {
            log,
            t: 0.0,
            step: 100.0,
            inputs: BTreeMap::new(),
        }
    }

    pub fn at(mut self, ms: f64) -> Self {
        self.t = ms;
        self
    }

    pub fn step(mut self, ms: f64) -> Self {
        self.step = ms;
        self
    }

    /// Tape `text` dans le mot `word_index` : keydown + insert par caractère.
    pub fn typ(mut self, word_index: u32, text: &str) -> Self {
        for ch in text.chars() {
            let input = self.inputs.entry(word_index).or_default();
            let char_index = input.chars().count() as u32;
            let correct = match self.log.context.target_words.get(word_index as usize) {
                Some(t) => t.chars().nth(char_index as usize) == Some(ch),
                None => true,
            };
            input.push(ch);
            self.log
                .push(self.t, EventKind::KeyDown { code: ch as u32 });
            self.log.push(
                self.t,
                EventKind::Insert {
                    word_index,
                    char_index,
                    ch,
                    correct,
                },
            );
            self.t += self.step;
        }
        self
    }

    pub fn backspace(mut self, word_index: u32) -> Self {
        self.inputs.entry(word_index).or_default().pop();
        self.log.push(self.t, EventKind::KeyDown { code: 8 });
        self.log.push(self.t, EventKind::DeleteChar { word_index });
        self.t += self.step;
        self
    }

    pub fn tick(mut self, second: u32) -> Self {
        self.log
            .push(f64::from(second) * 1000.0, EventKind::TimerStep { second });
        self
    }

    pub fn bailed_out(mut self) -> Self {
        self.log.context.bailed_out = true;
        self
    }

    pub fn end(mut self, ms: f64) -> EventLog {
        self.log.push(ms, EventKind::TimerEnd);
        self.log
    }
}

pub fn assert_close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

/// Test chronométré régulier : pendant `active_s` secondes, `per_second`
/// frappes par seconde (réparties à l'intérieur de la seconde) qui tapent
/// `typed_word` en boucle contre la cible `target_word` ; puis rien jusqu'à
/// `total_s`. Un tick par seconde, fin à `total_s × 1000`.
pub fn steady_time_test(
    total_s: u32,
    active_s: u32,
    target_word: &str,
    typed_word: &str,
    per_second: u32,
) -> EventLog {
    let len = target_word.chars().count();
    assert_eq!(len, typed_word.chars().count());
    let word_count = (active_s * per_second) as usize / len + 2;
    let targets: Vec<&str> = vec![target_word; word_count];
    let typed: Vec<char> = typed_word.chars().collect();
    let mut b = LogBuilder::new(Mode::Time, true, &targets);
    let mut n = 0usize;
    for s in 0..total_s {
        if s < active_s {
            for k in 0..per_second {
                let t =
                    f64::from(s) * 1000.0 + f64::from(k + 1) * 1000.0 / f64::from(per_second + 1);
                b = b.at(t).typ((n / len) as u32, &typed[n % len].to_string());
                n += 1;
            }
        }
        b = b.tick(s + 1);
    }
    b.end(f64::from(total_s) * 1000.0)
}
