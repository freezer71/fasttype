use fasttype_core::chars::contains_korean;
use fasttype_core::result::remove_language_size;
use fasttype_data::{
    DEFAULT_LANGUAGE, DEFAULT_THEME, DataError, LanguageCache, language_names, load_language,
    quote_file_names, quotes_for, theme, themes,
};
use std::sync::Arc;

#[test]
fn counts_match_monkeytype() {
    assert_eq!(language_names().unwrap().len(), 446);
    assert_eq!(quote_file_names().unwrap().len(), 87);
    assert_eq!(themes().unwrap().len(), 187);
}

#[test]
fn default_language_and_theme_exist() {
    let english = load_language(DEFAULT_LANGUAGE).unwrap();
    assert_eq!(english.words.len(), 200);
    assert_eq!(english.words[0], "the");
    let serika = theme(DEFAULT_THEME).unwrap();
    assert_eq!(serika.bg.to_hex(), "#323437");
    assert_eq!(serika.main.to_hex(), "#e2b714");
    assert_eq!(serika.error.to_hex(), "#ca4754");
}

#[test]
fn unknown_language_is_an_error() {
    assert_eq!(
        load_language("klingon_9000k"),
        Err(DataError::UnknownLanguage("klingon_9000k".into()))
    );
    assert!(theme("does_not_exist").is_none());
}

#[test]
fn quotes_follow_language_without_size() {
    let q = quotes_for("english_1k").unwrap().unwrap();
    assert_eq!(q.language, "english");
    assert!(q.quotes.len() > 6000);
}

#[test]
fn language_without_quotes_returns_none() {
    let quote_names = quote_file_names().unwrap();
    let without = language_names()
        .unwrap()
        .into_iter()
        .find(|n| !quote_names.contains(&remove_language_size(n).as_str()))
        .expect("au moins une langue sans citations");
    assert_eq!(quotes_for(without).unwrap(), None);
}

#[test]
fn every_language_and_quote_file_loads() {
    for name in language_names().unwrap() {
        let lang = load_language(name).unwrap_or_else(|e| panic!("{e}"));
        assert!(!lang.words.is_empty(), "{name}");
    }
    for name in quote_file_names().unwrap() {
        assert!(quotes_for(name).unwrap().is_some(), "{name}");
    }
    assert!(
        load_language("korean")
            .unwrap()
            .words
            .iter()
            .any(|w| contains_korean(w))
    );
    assert!(load_language("english_450k").unwrap().words.len() > 400_000);
}

#[test]
fn cache_is_shared_across_threads() {
    let cache = Arc::new(LanguageCache::new());
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let cache = Arc::clone(&cache);
            std::thread::spawn(move || cache.get("english_1k").unwrap())
        })
        .collect();
    let langs: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(langs.windows(2).all(|w| Arc::ptr_eq(&w[0], &w[1])));
    assert!(matches!(
        cache.get("nope"),
        Err(DataError::UnknownLanguage(_))
    ));
}

#[test]
fn language_groups_cover_known_languages() {
    let groups = fasttype_data::language_groups().unwrap();
    assert_eq!(groups[0].name, "english");
    assert!(groups[0].languages.contains(&"english_450k".to_string()));
    let names = language_names().unwrap();
    for g in groups {
        for l in &g.languages {
            assert!(names.contains(&l.as_str()), "{l} ({})", g.name);
        }
    }
}

#[test]
fn cache_tells_whether_a_language_is_ready_without_loading_it() {
    let cache = LanguageCache::new();
    assert!(!cache.is_ready("french"));
    assert!(!cache.is_ready("french"), "la question ne charge rien");
    cache.get("french").unwrap();
    assert!(cache.is_ready("french"));
    assert!(!cache.is_ready("english"));
}
