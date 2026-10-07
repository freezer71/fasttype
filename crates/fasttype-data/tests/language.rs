use fasttype_data::DataError;
use fasttype_data::language::{Language, parse_language, parse_quotes};

const EN: &[u8] = br#"{"name":"english","orderedByFrequency":true,"noLazyMode":true,"_comment":"x","preferredFont":"Roboto","words":["the","be","of"]}"#;

#[test]
fn parses_language_and_ignores_unused_fields() {
    let file = parse_language("english", EN).unwrap();
    assert_eq!(file.words, ["the", "be", "of"]);
    assert!(file.ordered_by_frequency && file.no_lazy_mode && !file.right_to_left);
    let lang = Language::from(file);
    assert_eq!(lang.words.len(), 3);
}

#[test]
fn parses_additional_accents() {
    let json = r#"{"name":"vietnamese","additionalAccents":[["áà","a"],["đ","d"]],"words":["a"]}"#
        .as_bytes();
    let file = parse_language("vietnamese", json).unwrap();
    assert_eq!(
        file.additional_accents,
        [
            ("áà".to_string(), "a".to_string()),
            ("đ".to_string(), "d".to_string())
        ]
    );
}

#[test]
fn name_must_match_file() {
    assert!(matches!(
        parse_language("french", EN),
        Err(DataError::Invalid { .. })
    ));
}

#[test]
fn empty_or_blank_words_are_invalid() {
    let empty = br#"{"name":"x","words":[]}"#;
    assert!(matches!(
        parse_language("x", empty),
        Err(DataError::Invalid { .. })
    ));
    let blank = br#"{"name":"x","words":["a","  "]}"#;
    let e = parse_language("x", blank).unwrap_err();
    assert!(e.to_string().contains("mot vide"), "{e}");
}

#[test]
fn broken_json_names_the_file() {
    let e = parse_language("english", b"{").unwrap_err();
    assert!(matches!(&e, DataError::Json { name, .. } if name == "english"));
}

#[test]
fn quotes_are_validated() {
    let ok = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"hi there","source":"a","length":8,"id":1}]}"#;
    assert_eq!(parse_quotes("english", ok).unwrap().quotes.len(), 1);
    let dup = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"a","source":"a","length":1,"id":1},{"text":"b","source":"b","length":1,"id":1}]}"#;
    assert!(
        parse_quotes("english", dup)
            .unwrap_err()
            .to_string()
            .contains("en double")
    );
    let blank = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"  ","source":"a","length":2,"id":1}]}"#;
    assert!(parse_quotes("english", blank).is_err());
    assert!(
        parse_quotes("french", ok).is_err(),
        "langue du fichier différente"
    );
}
