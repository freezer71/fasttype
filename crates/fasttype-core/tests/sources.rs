use fasttype_core::quote::{QuoteFile, QuoteLength, normalize_quote_text, quote_words};
use fasttype_core::rng::{Scripted, SplitMix64};
use fasttype_core::sources::{CustomMode, CustomWords, RandomWords, SequenceWords, WordSource};
use fasttype_core::spec::CustomLimit;
use std::sync::Arc;

fn list(words: &[&str]) -> Arc<Vec<String>> {
    Arc::new(words.iter().map(|s| s.to_string()).collect())
}

fn random(words: &[&str], lang: &str, punctuation: bool, numbers: bool) -> RandomWords {
    RandomWords::new(list(words), lang, punctuation, numbers, None)
}

#[test]
fn rejects_same_word_as_previous_two() {
    let mut s = random(&["the", "the", "cat"], "english", false, false);
    let mut rng = Scripted::new(&[0.0, 0.9]);
    assert_eq!(s.next_raw("The.", "", &mut rng).as_deref(), Some("cat"));
    assert_eq!(rng.consumed(), 2);
}

#[test]
fn rejects_capital_i_without_punctuation() {
    let mut s = random(&["I", "you"], "english", false, false);
    assert_eq!(
        s.next_raw("", "", &mut Scripted::new(&[0.0, 0.9]))
            .as_deref(),
        Some("you")
    );
}

#[test]
fn rejects_symbols_and_digits_when_disabled() {
    let mut s = random(&["don't", "x1", "ok"], "english", false, false);
    assert_eq!(
        s.next_raw("", "", &mut Scripted::new(&[0.0, 0.4, 0.9]))
            .as_deref(),
        Some("ok")
    );
}

#[test]
fn keeps_symbols_for_code_languages() {
    let mut s = random(&["a.b"], "code_python", false, false);
    assert_eq!(
        s.next_raw("", "", &mut Scripted::new(&[0.0])).as_deref(),
        Some("a.b")
    );
}

#[test]
fn lowercases_unless_punctuation_or_german() {
    let mut rng = Scripted::new(&[0.0]);
    assert_eq!(
        random(&["Hello"], "english", false, false)
            .next_raw("", "", &mut rng)
            .as_deref(),
        Some("hello")
    );
    assert_eq!(
        random(&["Hallo"], "german", false, false)
            .next_raw("", "", &mut rng)
            .as_deref(),
        Some("Hallo")
    );
    assert_eq!(
        random(&["Hello"], "english", true, false)
            .next_raw("", "", &mut rng)
            .as_deref(),
        Some("Hello")
    );
}

#[test]
fn multi_word_entries_are_queued() {
    let mut s = random(&["ice cream"], "english", false, false);
    let mut rng = Scripted::new(&[0.0]);
    assert_eq!(s.next_raw("", "", &mut rng).as_deref(), Some("ice"));
    assert_eq!(s.next_raw("ice", "", &mut rng).as_deref(), Some("cream"));
}

#[test]
fn empty_list_yields_nothing() {
    let mut s = random(&[], "english", false, false);
    assert_eq!(s.next_raw("", "", &mut SplitMix64::new(1)), None);
}

#[test]
fn words_limit_drives_generation() {
    let s = RandomWords::new(list(&["a"]), "english", false, false, Some(25));
    assert_eq!(s.initial_limit(), 25);
    assert!(!s.all_generated(24));
    assert!(s.all_generated(25));
    let big = RandomWords::new(list(&["a"]), "english", false, false, Some(500));
    assert_eq!(big.initial_limit(), 100);
    let infinite = RandomWords::new(list(&["a"]), "english", false, false, None);
    assert!(!infinite.all_generated(10_000));
}

#[test]
fn sequence_is_finite() {
    let mut s = SequenceWords::new(vec!["a".into(), "b".into()]);
    let mut rng = SplitMix64::new(1);
    assert_eq!(s.initial_limit(), 2);
    assert_eq!(s.next_raw("", "", &mut rng).as_deref(), Some("a"));
    assert_eq!(s.next_raw("", "", &mut rng).as_deref(), Some("b"));
    assert_eq!(s.next_raw("", "", &mut rng), None);
    assert!(s.all_generated(2));
}

#[test]
fn custom_repeat_cycles_until_word_limit() {
    let mut s = CustomWords::new("one two", CustomMode::Repeat, CustomLimit::Word(5), false);
    let mut rng = SplitMix64::new(1);
    let got: Vec<String> = (0..5)
        .filter_map(|_| s.next_raw("", "", &mut rng))
        .collect();
    assert_eq!(got, ["one", "two", "one", "two", "one"]);
    assert!(s.all_generated(5));
    assert_eq!(s.initial_limit(), 5);
}

#[test]
fn custom_shuffle_uses_each_word_once_per_round() {
    let mut s = CustomWords::new(
        "a b c d e",
        CustomMode::Shuffle,
        CustomLimit::Word(0),
        false,
    );
    let mut rng = SplitMix64::new(9);
    let mut round: Vec<String> = (0..5)
        .filter_map(|_| s.next_raw("", "", &mut rng))
        .collect();
    round.sort();
    assert_eq!(round, ["a", "b", "c", "d", "e"]);
    assert!(!s.all_generated(1_000));
}

#[test]
fn custom_sections_with_pipe() {
    let mut s = CustomWords::new(
        "hello world|good  bye",
        CustomMode::Repeat,
        CustomLimit::Section(2),
        true,
    );
    let mut rng = SplitMix64::new(1);
    let got: Vec<String> = std::iter::from_fn(|| s.next_raw("", "", &mut rng))
        .take(4)
        .collect();
    assert_eq!(got, ["hello", "world", "good", "bye"]);
    assert!(s.all_generated(4));
}

#[test]
fn quote_text_is_normalized() {
    assert_eq!(
        normalize_quote_text("  Hello  world…\r\n  next  "),
        "Hello world...\n next"
    );
    assert_eq!(
        quote_words("Hello  world…\n next"),
        ["Hello", "world...\n", "next"]
    );
}

#[test]
fn quotes_are_grouped_and_picked() {
    let json = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"short one","source":"a","length":9,"id":1,"approvedBy":"x"},
                  {"text":"medium","source":"b","length":150,"id":2}]}"#;
    let file = QuoteFile::from_json(json).unwrap();
    assert_eq!(file.group_of(&file.quotes[1]), Some(1));
    let mut rng = Scripted::new(&[0.0]);
    assert_eq!(
        file.pick(QuoteLength::Medium, &mut rng).map(|q| q.id),
        Some(2)
    );
    assert_eq!(file.pick(QuoteLength::Thicc, &mut rng).map(|q| q.id), None);
    assert_eq!(file.by_id(1).map(|q| q.source.as_str()), Some("a"));
}
