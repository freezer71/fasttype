use fasttype_core::chars::count_words;
use fasttype_core::event::{EventLog, active_word_index};
use fasttype_core::generator::WordGenerator;
use fasttype_core::numbers::{calculate_wpm, js_round};
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{SessionState, TestSession};
use fasttype_core::sources::RandomWords;
use fasttype_core::spec::TestSpec;
use fasttype_core::stats;
use proptest::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
enum Op {
    Char(char),
    Space,
    Backspace,
    DeleteWord,
    Wait(u16),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => prop::sample::select(vec!['t', 'h', 'e', 'c', 'a', 'x']).prop_map(Op::Char),
        2 => Just(Op::Space),
        1 => Just(Op::Backspace),
        1 => Just(Op::DeleteWord),
        2 => (1u16..1500).prop_map(Op::Wait),
    ]
}

fn session(seed: u64) -> TestSession {
    let words = Arc::new(
        ["the", "cat", "hat", "tea", "ace"]
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>(),
    );
    let source = RandomWords::new(words, "english", false, false, Some(25));
    let generator = WordGenerator::new(Box::new(source), "english", false, false);
    TestSession::new(
        TestSpec::words(25, "english", false, false),
        generator,
        Box::new(SplitMix64::new(seed)),
    )
}

/// `getWpmHistory` sans cache : tout est recompté à chaque borne.
fn naive_wpm_history(log: &EventLog) -> Vec<f64> {
    stats::timer_boundaries(log)
        .iter()
        .map(|&b| {
            let inputs = log.word_inputs(Some(b));
            let active = active_word_index(&inputs);
            let c = count_words(
                inputs
                    .iter()
                    .map(|(&i, s)| (s.as_str(), log.target(i).unwrap_or(s.as_str()), i == active)),
                true,
                log.context.korean,
            );
            js_round(calculate_wpm(f64::from(c.correct_word), b / 1000.0))
        })
        .collect()
}

proptest! {
    #[test]
    fn cached_wpm_history_matches_naive(seed in any::<u64>(), zen in any::<bool>(), ops in prop::collection::vec(op(), 1..300)) {
        // en zen, la cible est la saisie : le compte d'un mot change à chaque frappe
        let mut s = if zen {
            TestSession::new(TestSpec::zen("english"), WordGenerator::empty(), Box::new(SplitMix64::new(seed)))
        } else {
            session(seed)
        };
        let mut t = 0.0;
        for op in ops {
            match op {
                Op::Char(c) => { s.insert(c, t); }
                Op::Space => { s.insert(' ', t); }
                Op::Backspace => { s.backspace(t); }
                Op::DeleteWord => { s.delete_word(t); }
                Op::Wait(ms) => t += f64::from(ms),
            }
            t += 10.0;
        }
        if s.state() == SessionState::Running {
            s.bail_out(t);
        }
        prop_assert_eq!(stats::wpm_history(s.log()), naive_wpm_history(s.log()));
    }

    #[test]
    fn stats_stay_coherent(seed in any::<u64>(), ops in prop::collection::vec(op(), 1..300)) {
        let mut s = session(seed);
        let mut t = 0.0;
        for op in ops {
            match op {
                Op::Char(c) => { s.insert(c, t); }
                Op::Space => { s.insert(' ', t); }
                Op::Backspace => { s.backspace(t); }
                Op::DeleteWord => { s.delete_word(t); }
                Op::Wait(ms) => t += f64::from(ms),
            }
            t += 10.0;
            prop_assert!(s.active_index() < s.words().len().max(1));
        }
        if s.state() == SessionState::Running {
            s.bail_out(t);
        }
        if let Some(r) = s.result(0) {
            prop_assert!(r.wpm <= r.raw + 1e-9, "wpm {} > raw {}", r.wpm, r.raw);
            prop_assert!((0.0..=100.0).contains(&r.acc));
            prop_assert!((0.0..=100.0).contains(&r.consistency));
            let replayed = s.log().word_inputs(None);
            for (i, input) in &replayed {
                prop_assert_eq!(input, &s.inputs()[*i as usize]);
            }
        }
    }

    #[test]
    fn correct_words_never_reopen(seed in any::<u64>(), backspaces in 1usize..50) {
        let mut s = session(seed);
        let mut t = 0.0;
        for _ in 0..3 {
            let word: Vec<char> = s.word(s.active_index()).chars().collect();
            for ch in word {
                s.insert(ch, t);
                t += 50.0;
            }
        }
        let active = s.active_index();
        for _ in 0..backspaces {
            s.backspace(t);
            t += 10.0;
        }
        prop_assert_eq!(s.active_index(), active);
    }
}
