use fasttype_core::chars::{CharCounts, count_chars, count_words};

fn c(all_correct: u32, correct_word: u32, incorrect: u32, extra: u32, missed: u32) -> CharCounts {
    CharCounts {
        all_correct,
        correct_word,
        incorrect,
        extra,
        missed,
    }
}

#[test]
fn exact_word_with_separator() {
    assert_eq!(count_chars("hello ", "hello ", false), c(6, 6, 0, 0, 0));
}

#[test]
fn wrong_letter_then_early_space() {
    // h e l justes ; 'o' au lieu de 'l' ; ' ' au lieu de 'o' (l'input contient un espace) ; ' ' manquant
    assert_eq!(count_chars("helo ", "hello ", false), c(3, 0, 2, 0, 1));
}

#[test]
fn letter_typed_instead_of_separator_is_extra() {
    assert_eq!(count_chars("hellox", "hello ", false), c(5, 0, 0, 1, 0));
}

#[test]
fn letter_after_full_word_then_space() {
    // 's' tapé à la place de l'espace alors que l'input contient un espace → incorrect ; l'espace final dépasse → extra
    assert_eq!(count_chars("hellos ", "hello ", false), c(5, 0, 1, 1, 0));
}

#[test]
fn separator_of_wrong_word_is_extra() {
    assert_eq!(count_chars("hxllo ", "hello ", false), c(4, 0, 1, 1, 0));
}

#[test]
fn partial_credit_only_for_correct_prefix() {
    assert_eq!(count_chars("hel", "hello ", true), c(3, 3, 0, 0, 0));
    assert_eq!(count_chars("hel", "hello ", false), c(3, 0, 0, 0, 3));
    assert_eq!(count_chars("hex", "hello ", true), c(2, 0, 1, 0, 0));
}

#[test]
fn counts_accented_letters_as_one_char() {
    assert_eq!(count_chars("café ", "café ", false), c(5, 5, 0, 0, 0));
    // é ≠ e → incorrect ; l'espace d'un mot faux compte en extra
    assert_eq!(count_chars("cafe ", "café ", false), c(3, 0, 1, 1, 0));
    assert_eq!(count_chars("straße", "straße", false), c(6, 6, 0, 0, 0));
}

#[test]
fn count_words_stops_after_last_and_credits_it() {
    let words = [
        ("the ", "the ", false),
        ("ca", "cat ", true),
        ("ignored", "x", false),
    ];
    assert_eq!(count_words(words, true), c(6, 6, 0, 0, 0));
    assert_eq!(count_words(words, false), c(6, 4, 0, 0, 2));
}
