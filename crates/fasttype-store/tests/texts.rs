mod common;

use common::scratch;
use fasttype_store::texts::{CustomTexts, FavoriteQuote, FavoriteQuotes};
use std::io::ErrorKind;

#[test]
fn custom_texts_roundtrip() {
    let dir = scratch("texts");
    let texts = CustomTexts::new(dir.join("custom_texts"));
    assert!(texts.list().unwrap().is_empty());
    texts.save("lorem", "lorem ipsum dolor").unwrap();
    texts.save("Mon texte 2", "été à Paris").unwrap();
    assert_eq!(texts.list().unwrap(), ["Mon texte 2", "lorem"]);
    assert_eq!(texts.load("Mon texte 2").unwrap(), "été à Paris");
    texts.delete("lorem").unwrap();
    assert_eq!(texts.list().unwrap(), ["Mon texte 2"]);
    assert_eq!(texts.load("lorem").unwrap_err().kind(), ErrorKind::NotFound);
}

#[test]
fn hostile_names_are_refused() {
    let dir = scratch("hostile");
    let texts = CustomTexts::new(dir.join("custom_texts"));
    for name in [
        "",
        "../x",
        "a/b",
        "/etc/passwd",
        ".hidden",
        "..",
        &"x".repeat(65),
    ] {
        assert_eq!(
            texts.save(name, "x").unwrap_err().kind(),
            ErrorKind::InvalidInput,
            "{name:?}"
        );
    }
    assert!(!dir.join("x.txt").exists());
}

#[test]
fn favorites_toggle_and_persist() {
    let dir = scratch("favs");
    let favs = FavoriteQuotes::new(dir.join("favorite_quotes.json"));
    assert!(favs.load().unwrap().is_empty());
    assert!(favs.toggle("english", 12).unwrap());
    assert!(favs.toggle("french", 3).unwrap());
    assert_eq!(
        favs.load().unwrap(),
        [
            FavoriteQuote {
                language: "english".into(),
                id: 12
            },
            FavoriteQuote {
                language: "french".into(),
                id: 3
            }
        ]
    );
    assert!(!favs.toggle("english", 12).unwrap());
    assert_eq!(favs.load().unwrap().len(), 1);
}
