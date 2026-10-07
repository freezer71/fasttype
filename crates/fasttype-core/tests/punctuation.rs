use fasttype_core::punctuation::{Punctuator, get_numbers, localize_digits};
use fasttype_core::rng::Scripted;

fn run(
    prev: Option<&str>,
    word: &str,
    index: usize,
    max: usize,
    lang: &str,
    values: &[f64],
) -> (String, usize) {
    let mut rng = Scripted::new(values);
    let out = Punctuator::new().punctuate(prev, word, index, max, lang, &mut rng);
    (out, rng.consumed())
}

#[test]
fn first_word_is_capitalized_without_randomness() {
    assert_eq!(
        run(None, "hello", 0, 100, "english", &[]),
        ("Hello".into(), 0)
    );
}

#[test]
fn word_after_sentence_end_is_capitalized() {
    assert_eq!(
        run(Some("end."), "next", 4, 100, "english_1k", &[]),
        ("Next".into(), 0)
    );
}

#[test]
fn last_word_always_ends_sentence() {
    // random() du test 10 % (0.5 → non), puis index = max − 1 → fin de phrase ; 0.3 ≤ 0.8 → "."
    assert_eq!(
        run(Some("world"), "word", 99, 100, "english", &[0.5, 0.3]),
        ("word.".into(), 2)
    );
}

#[test]
fn french_question_mark_is_its_own_word() {
    assert_eq!(
        run(Some("chat"), "mot", 5, 100, "french", &[0.05, 0.85]),
        ("?".into(), 2)
    );
}

#[test]
fn comma_after_seven_failed_draws() {
    let values = [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.1];
    assert_eq!(
        run(Some("the"), "word", 5, 100, "english", &values),
        ("word,".into(), 8)
    );
}

#[test]
fn english_contraction_keeps_case() {
    // 9 tirages ratés (dont celui du code), 0.1 < 0.5 → contraction, 0.6 → 2e choix "it'll"
    let mut values = vec![0.5; 9];
    values.extend([0.1, 0.6]);
    assert_eq!(
        run(Some("the"), "it", 5, 100, "english", &values).0,
        "it'll"
    );
    // juste après un point, le mot est capitalisé avant tout tirage
    assert_eq!(run(Some("end."), "it", 5, 100, "english", &[]).0, "It");
}

#[test]
fn spanish_inverted_question_closes_at_sentence_end() {
    let mut p = Punctuator::new();
    let mut rng = Scripted::new(&[0.95]);
    assert_eq!(
        p.punctuate(None, "hola", 0, 100, "spanish", &mut rng),
        "¿Hola"
    );
    let mut rng = Scripted::new(&[0.5]);
    assert_eq!(
        p.punctuate(Some("que"), "tal", 99, 100, "spanish", &mut rng),
        "tal?"
    );
}

#[test]
fn code_is_never_capitalized() {
    assert_eq!(run(None, "word", 0, 100, "code_python", &[0.9]).0, "word");
}

#[test]
fn newline_moves_to_end() {
    assert_eq!(run(None, "a\nb", 0, 100, "code_python", &[0.9]).0, "ab\n");
}

#[test]
fn numbers_follow_get_numbers() {
    let mut rng = Scripted::new(&[0.0, 0.0]);
    assert_eq!(get_numbers(4, &mut rng), "1");
    let mut rng = Scripted::new(&[0.99, 0.5, 0.0, 0.99, 0.5]);
    assert_eq!(get_numbers(4, &mut rng), "5095");
}

#[test]
fn digits_are_localized() {
    assert_eq!(localize_digits("123", "hindi"), "१२३");
    assert_eq!(localize_digits("123", "nepali_1k"), "१२३");
    assert_eq!(localize_digits("123", "bangla"), "১২৩");
    assert_eq!(localize_digits("123", "kurdish"), "١٢٣");
    assert_eq!(localize_digits("123", "english"), "123");
}
