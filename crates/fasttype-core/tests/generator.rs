use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::{Scripted, SplitMix64};
use fasttype_core::sources::{RandomWords, SequenceWords};
use std::sync::Arc;

fn words(ws: &[&str]) -> Arc<Vec<String>> {
    Arc::new(ws.iter().map(|s| s.to_string()).collect())
}

#[test]
fn appends_space_separator() {
    let mut g = WordGenerator::new(
        Box::new(RandomWords::new(
            words(&["hello"]),
            "english",
            false,
            false,
            None,
        )),
        "english",
        false,
        false,
    );
    assert_eq!(
        g.next(0, 100, &mut Scripted::new(&[0.0])).as_deref(),
        Some("hello ")
    );
}

#[test]
fn newline_words_have_no_extra_separator() {
    let seq = SequenceWords::new(vec!["Hi".into(), "there\n".into(), "you".into()]);
    let mut g = WordGenerator::new(Box::new(seq), "english", false, false);
    let mut rng = SplitMix64::new(1);
    let got: Vec<String> = (0..3).filter_map(|i| g.next(i, 3, &mut rng)).collect();
    assert_eq!(got, ["Hi ", "there\n", "you "]);
    assert!(g.all_generated());
}

#[test]
fn numbers_replace_word_one_time_in_ten() {
    let mut g = WordGenerator::new(
        Box::new(RandomWords::new(
            words(&["cat"]),
            "english",
            false,
            true,
            None,
        )),
        "english",
        false,
        true,
    );
    // tirage du mot, 0.05 < 0.1 → nombre, longueur 1, chiffre 1
    assert_eq!(
        g.next(0, 100, &mut Scripted::new(&[0.0, 0.05, 0.0, 0.0]))
            .as_deref(),
        Some("1 ")
    );
}

#[test]
fn punctuation_uses_previous_generated_word() {
    let seq = SequenceWords::new(vec!["hello".into(), "world".into()]);
    let mut g = WordGenerator::new(Box::new(seq), "english", true, false);
    // mot 0 : capitalisé sans tirage ; mot 1 = dernier (index 1 = bound − 1) → 0.5 puis 0.3 → "."
    let mut rng = Scripted::new(&[0.5, 0.3]);
    assert_eq!(g.next(0, 2, &mut rng).as_deref(), Some("Hello "));
    assert_eq!(g.next(1, 2, &mut rng).as_deref(), Some("world. "));
}

#[test]
fn swiss_german_replaces_eszett() {
    let mut g = WordGenerator::new(
        Box::new(SequenceWords::new(vec!["straße".into()])),
        "swiss_german",
        false,
        false,
    );
    assert_eq!(
        g.next(0, 1, &mut SplitMix64::new(1)).as_deref(),
        Some("strasse ")
    );
}

#[test]
fn empty_generator_is_done_immediately() {
    let mut g = WordGenerator::empty();
    assert_eq!(g.initial_limit(), 0);
    assert!(g.all_generated());
    assert_eq!(g.next(0, 0, &mut SplitMix64::new(1)), None);
}
