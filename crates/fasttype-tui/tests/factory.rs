use fasttype_core::quote::QuoteFile;
use fasttype_core::rng::Scripted;
use fasttype_core::spec::Mode;
use fasttype_store::Config;
use fasttype_tui::session_factory::{SessionFactory, pick_quote};

fn config(toml: &str) -> Config {
    let (c, w) = Config::from_toml(toml);
    assert!(w.is_empty(), "{w:?}");
    c
}

#[test]
fn builds_each_mode() {
    let mut f = SessionFactory::new();
    for (mode, expected) in [
        ("time", Mode::Time),
        ("words", Mode::Words),
        ("zen", Mode::Zen),
        ("custom", Mode::Custom),
        ("quote", Mode::Quote),
    ] {
        let built = f.build(&config(&format!("mode = \"{mode}\"\n")), 7);
        assert_eq!(built.session.spec().mode, expected, "{mode}");
        assert!(built.warning.is_none(), "{mode} : {:?}", built.warning);
    }
}

#[test]
fn words_mode_has_exactly_n_words() {
    let built = SessionFactory::new().build(&config("mode = \"words\"\nwords = 10\n"), 1);
    assert_eq!(built.session.words().len(), 10);
}

#[test]
fn same_seed_same_words() {
    let a = SessionFactory::new().build(&Config::defaults(), 99);
    let b = SessionFactory::new().build(&Config::defaults(), 99);
    assert_eq!(a.session.words(), b.session.words());
}

#[test]
fn quote_groups_follow_quote_length() {
    let file = QuoteFile::from_json(
        br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"short","source":"a","length":5,"id":1},{"text":"long","source":"b","length":400,"id":2}]}"#,
    )
    .unwrap();
    assert_eq!(
        pick_quote(&file, &[2], &mut Scripted::new(&[0.0])).map(|q| q.id),
        Some(2)
    );
    assert_eq!(
        pick_quote(&file, &[0], &mut Scripted::new(&[0.0])).map(|q| q.id),
        Some(1)
    );
    assert_eq!(pick_quote(&file, &[3], &mut Scripted::new(&[0.0])), None);
    assert!(
        pick_quote(&file, &[-3], &mut Scripted::new(&[0.0])).is_some(),
        "favoris non gérés : toutes"
    );
}
