mod common;

use common::app;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fasttype_tui::input::{Input, Key, map_event, map_key};
use fasttype_tui::palette::filter::{filter, split_words};
use fasttype_tui::palette::lists::{Context, root};
use fasttype_tui::palette::state::{Outcome, PaletteState, parse_input};
use fasttype_tui::palette::{Action, AppAction, InputTarget, Subgroup};
use toml::Value;

fn words(labels: &[&str]) -> Vec<Vec<String>> {
    labels.iter().map(|l| split_words(l)).collect()
}

fn run(input: &str, labels: &[&str]) -> Vec<String> {
    let w = words(labels);
    let refs: Vec<Option<&[String]>> = w.iter().map(|v| Some(v.as_slice())).collect();
    filter(input, &refs)
        .into_iter()
        .map(|i| labels[i].to_string())
        .collect()
}

const LABELS: &[&str] = &[
    "Smooth caret...",
    "Caret style...",
    "Smooth line scroll...",
    "Theme...",
    "Timer style...",
];

#[test]
fn filter_keeps_the_best_word_prefix_matches_in_list_order() {
    assert_eq!(run("", LABELS).len(), 5, "saisie vide : tout");
    assert_eq!(run("sm car", LABELS), ["Smooth caret..."]);
    assert_eq!(run("caret", LABELS), ["Smooth caret...", "Caret style..."]);
    assert_eq!(run("t", LABELS), ["Theme...", "Timer style..."]);
    // un mot sans correspondance : on retombe sur les meilleures commandes
    assert_eq!(
        run("caret zzz", LABELS),
        ["Smooth caret...", "Caret style..."]
    );
    assert_eq!(run(">smooth  ca", LABELS), ["Smooth caret..."]);
    assert!(run("qqq", LABELS).is_empty());
}

#[test]
fn an_input_word_claims_a_single_label_word() {
    // « s s » : deux mots saisis, il faut deux mots du libellé commençant par s
    assert_eq!(run("s s", LABELS), ["Smooth line scroll..."]);
}

fn ctx_root(config: &fasttype_store::Config, on_result: bool, bail: bool) -> Subgroup {
    let languages = ["english", "english_1k", "french"];
    let themes = ["dracula", "serika_dark"];
    root(&Context {
        config,
        custom_text: "hello world",
        on_result,
        can_bail_out: bail,
        languages: &languages,
        themes: &themes,
    })
}

fn labels(g: &Subgroup) -> Vec<&str> {
    g.list.iter().map(|c| c.display.as_str()).collect()
}

fn sub<'a>(g: &'a Subgroup, label: &str) -> &'a Subgroup {
    match &g.list.iter().find(|c| c.display == label).unwrap().action {
        Action::Open(s) => s,
        other => panic!("{label} : {other:?}"),
    }
}

#[test]
fn root_list_follows_the_site_order() {
    let a = app("pal-root", "");
    let r = ctx_root(&a.store.config, false, false);
    let l = labels(&r);
    assert_eq!(
        &l[..7],
        [
            "Punctuation...",
            "Numbers...",
            "Mode...",
            "Time...",
            "Word count...",
            "Quote length...",
            "Language..."
        ]
    );
    assert!(l.contains(&"Smooth caret..."));
    assert!(l.contains(&"Live progress color..."));
    assert!(l.contains(&"Key tips..."));
    assert!(!l.contains(&"Next test"), "pas sur l'écran de test");
    assert!(!l.contains(&"Bail out..."));
    assert_eq!(l.last(), Some(&"Quit"));
    let r = ctx_root(&a.store.config, true, true);
    assert_eq!(&labels(&r)[..2], ["Next test", "Repeat test"]);
    assert!(labels(&r).contains(&"Bail out..."));
}

#[test]
fn subgroups_list_values_with_the_current_one_checked() {
    let a = app("pal-sub", "smooth_caret = \"fast\"\n");
    let r = ctx_root(&a.store.config, false, false);
    let caret = sub(&r, "Smooth caret...");
    assert_eq!(labels(caret), ["off", "slow", "medium", "fast"]);
    assert!(caret.list[3].active && !caret.list[2].active);
    let time = sub(&r, "Time...");
    assert_eq!(labels(time), ["15", "30", "60", "120", "custom..."]);
    assert!(time.list[1].active, "30 par défaut");
    assert_eq!(
        time.list[4].action,
        Action::Input(InputTarget::Config("time"))
    );
    let punct = sub(&r, "Punctuation...");
    assert_eq!(labels(punct), ["off", "on"]);
    let lang = sub(&r, "Language...");
    assert_eq!(labels(lang), ["english", "english 1k", "french"]);
    let theme = sub(&r, "Theme...");
    assert_eq!(theme.list[0].preview.as_deref(), Some("dracula"));
    assert!(theme.list[1].active);
    let quote = sub(&r, "Quote length...");
    assert_eq!(labels(quote), ["all", "short", "medium", "long", "thicc"]);
    assert!(quote.list[2].active, "medium par défaut");
    // les nombres libres s'éditent directement
    let font = r.list.iter().find(|c| c.display == "Font size...").unwrap();
    assert_eq!(font.action, Action::Input(InputTarget::Config("fontSize")));
}

#[test]
fn navigation_search_and_subgroups() {
    let a = app("pal-nav", "");
    let c = &a.store.config;
    let mut p = PaletteState::open(ctx_root(c, false, false));
    assert_eq!(p.title(), "Search...");
    for ch in "time".chars() {
        assert_eq!(p.key(Key::Char(ch), c), Outcome::Stay);
    }
    assert_eq!(p.hovered().unwrap().display, "Time...");
    p.key(Key::Enter, c);
    assert_eq!(p.title(), "Time");
    assert_eq!(p.query(), "", "la recherche repart à vide");
    assert_eq!(
        p.hovered().unwrap().display,
        "30",
        "curseur sur la valeur en cours"
    );
    p.key(Key::Down, c);
    assert_eq!(p.hovered().unwrap().display, "60");
    p.key(Key::Up, c);
    p.key(Key::Up, c);
    assert_eq!(p.hovered().unwrap().display, "15");
    p.key(Key::Up, c);
    assert_eq!(p.hovered().unwrap().display, "custom...", "on boucle");
    p.key(Key::Tab, c);
    assert_eq!(p.hovered().unwrap().display, "15", "Tab descend");
    assert_eq!(
        p.key(Key::Esc, c),
        Outcome::Stay,
        "Échap remonte d'un niveau"
    );
    assert_eq!(p.title(), "Search...");
    assert_eq!(p.key(Key::Esc, c), Outcome::Close);
    // choisir une valeur
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "time".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    p.key(Key::Down, c);
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::Set {
            key: "time",
            value: Value::Integer(60)
        })
    );
}

#[test]
fn custom_values_are_typed_and_checked() {
    let a = app("pal-input", "");
    let c = &a.store.config;
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "time".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    // depuis « 30 » : « 15 », puis on boucle sur « custom... »
    p.key(Key::Up, c);
    p.key(Key::Up, c);
    p.key(Key::Enter, c);
    let m = p.input().expect("mode saisie");
    assert_eq!(m.title, "custom...");
    assert_eq!(m.text, "30", "valeur en cours");
    p.key(Key::Backspace, c);
    p.key(Key::Backspace, c);
    p.key(Key::Char('x'), c);
    assert_eq!(p.key(Key::Enter, c), Outcome::Stay);
    assert_eq!(
        p.input().unwrap().error.as_deref(),
        Some("Must be a whole number")
    );
    p.key(Key::Backspace, c);
    p.paste("45");
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::Set {
            key: "time",
            value: Value::Integer(45)
        })
    );
    // Échap quitte la saisie, pas la palette
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "font".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    assert_eq!(p.input().unwrap().text, "2");
    assert_eq!(p.key(Key::Esc, c), Outcome::Stay);
    assert!(p.input().is_none());
}

#[test]
fn parse_input_messages() {
    assert_eq!(parse_input("time", "45"), Ok(Value::Integer(45)));
    assert_eq!(parse_input("time", " 0 "), Ok(Value::Integer(0)));
    assert_eq!(parse_input("time", "-1"), Err("Must be at least 0".into()));
    assert_eq!(parse_input("fontSize", "1.5"), Ok(Value::Float(1.5)));
    assert_eq!(
        parse_input("fontSize", "0"),
        Err("Must be greater than 0".into())
    );
    assert_eq!(parse_input("maxLineWidth", "0"), Ok(Value::Integer(0)));
    assert_eq!(parse_input("maxLineWidth", "80"), Ok(Value::Integer(80)));
    assert_eq!(
        parse_input("maxLineWidth", "10"),
        Err("Must be 0, or between 20 and 1000".into())
    );
    assert_eq!(
        parse_input("fontSize", "big"),
        Err("Must be a number".into())
    );
}

#[test]
fn bail_out_asks_for_confirmation() {
    let a = app("pal-bail", "");
    let c = &a.store.config;
    let r = ctx_root(c, false, true);
    let bail = sub(&r, "Bail out...");
    assert_eq!(bail.title, "Are you sure...");
    assert_eq!(labels(bail), ["Nevermind", "Yes, I am sure"]);
    assert_eq!(bail.list[0].action, Action::Close);
    assert_eq!(bail.list[1].action, Action::App(AppAction::BailOut));
}

fn press(code: KeyCode, mods: KeyModifiers) -> Option<Key> {
    map_key(&KeyEvent::new_with_kind(code, mods, KeyEventKind::Press)).map(|(k, _)| k)
}

#[test]
fn palette_keys_are_mapped() {
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(press(KeyCode::Up, KeyModifiers::NONE), Some(Key::Up));
    assert_eq!(press(KeyCode::Down, KeyModifiers::NONE), Some(Key::Down));
    for c in ['k', 'p'] {
        assert_eq!(press(KeyCode::Char(c), ctrl), Some(Key::Up));
    }
    for c in ['j', 'n'] {
        assert_eq!(press(KeyCode::Char(c), ctrl), Some(Key::Down));
    }
    assert_eq!(
        press(KeyCode::Char('p'), ctrl | KeyModifiers::SHIFT),
        Some(Key::Palette)
    );
    assert_eq!(
        press(KeyCode::Char('P'), ctrl | KeyModifiers::SHIFT),
        Some(Key::Palette)
    );
    assert_eq!(
        map_event(Event::Paste("a\nb".into()), 0.0),
        Some(Input::Paste("a\nb".into()))
    );
}

#[test]
fn typing_replaces_the_prefilled_value() {
    let a = app("pal-replace", "");
    let c = &a.store.config;
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "time".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    p.key(Key::Up, c);
    p.key(Key::Up, c);
    p.key(Key::Enter, c);
    assert_eq!(p.input().unwrap().text, "30");
    assert!(
        p.input().unwrap().selected,
        "valeur pré-remplie sélectionnée"
    );
    p.key(Key::Char('4'), c);
    p.key(Key::Char('5'), c);
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::Set {
            key: "time",
            value: Value::Integer(45)
        }),
        "comme sur le site : la frappe remplace la valeur"
    );
}

#[test]
fn aliases_from_the_site() {
    let a = app("pal-alias", "");
    let c = &a.store.config;
    let find = |q: &str, on_result: bool| -> Vec<String> {
        let mut p = PaletteState::open(ctx_root(c, on_result, false));
        q.chars().for_each(|ch| {
            p.key(Key::Char(ch), c);
        });
        p.shown().0.iter().map(|c| c.display.clone()).collect()
    };
    assert_eq!(find("words", false), ["Word count..."]);
    assert_eq!(
        find("quotes", false),
        ["Quote length...", "Search for quotes"]
    );
    assert!(find("wpm", false).contains(&"Live speed style...".to_string()));
    assert!(find("timer", false).contains(&"Live progress style...".to_string()));
    assert!(find("page", false).contains(&"Max line width...".to_string()));
    assert!(find("restart", true).contains(&"Next test".to_string()));
    assert!(find("opacity", false).contains(&"Live progress opacity...".to_string()));
}

#[test]
fn custom_text_and_settings_entries() {
    let a = app("pal-extra", "");
    let c = &a.store.config;
    let r = ctx_root(c, false, false);
    let l = labels(&r);
    for want in [
        "Change custom text",
        "Search for quotes",
        "Import settings",
        "Export settings",
    ] {
        assert!(l.contains(&want), "{want}");
    }
    // le texte custom part du texte en cours ; vide : refusé
    let mut p = PaletteState::open(r);
    "custom text".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    assert_eq!(p.input().unwrap().text, "hello world");
    for _ in 0..11 {
        p.key(Key::Backspace, c);
    }
    assert_eq!(p.key(Key::Enter, c), Outcome::Stay);
    assert_eq!(
        p.input().unwrap().error.as_deref(),
        Some("Must not be empty")
    );
    p.paste("one\ntwo");
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::App(AppAction::SetCustomText("one two".into())))
    );
    // l'import garde les sauts de ligne du TOML collé
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "import".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    p.paste("time = 60\ntheme = \"dracula\"\n");
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::App(AppAction::ImportSettings(
            "time = 60\ntheme = \"dracula\"\n".into()
        )))
    );
}
