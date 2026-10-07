mod common;

use common::{app, press, render, screen_text, settle, type_text};
use fasttype_core::spec::Mode;
use fasttype_tui::app::{App, CURRENT_CUSTOM_TEXT};
use fasttype_tui::input::{Input, Key};
use fasttype_tui::theme::ColorMode;

fn command(a: &mut App, query: &str, at: f64) {
    a.handle(press(Key::Esc, at));
    for c in query.chars() {
        a.handle(press(Key::Char(c), at));
    }
    a.handle(press(Key::Enter, at));
}

#[test]
fn change_custom_text_starts_a_custom_test_and_is_kept() {
    let mut a = app("ex-custom", "");
    command(&mut a, "custom text", 0.0);
    // la saisie part du texte en cours : on le remplace
    for _ in 0..400 {
        a.handle(press(Key::Backspace, 0.0));
    }
    a.handle(Input::Paste("the quick brown fox".into()));
    a.handle(press(Key::Enter, 0.0));
    settle(&mut a, 0.0);
    assert_eq!(a.store.config.str("mode"), "custom");
    assert_eq!(a.session().spec().mode, Mode::Custom);
    assert_eq!(a.session().word(0).trim_end(), "the");
    assert_eq!(
        a.store.custom_texts.load(CURRENT_CUSTOM_TEXT).unwrap(),
        "the quick brown fox"
    );
    // au prochain lancement, le texte est relu
    let reopened = App::new(
        fasttype_store::Store::open(common::paths(
            &std::env::temp_dir().join(format!("fasttype-tui-{}-ex-custom", std::process::id())),
        )),
        ColorMode::TrueColor,
        0.0,
        1,
    );
    assert_eq!(reopened.session().word(0).trim_end(), "the");
}

#[test]
fn search_for_quotes_picks_that_quote_every_time() {
    let mut a = app("ex-quote", "");
    command(&mut a, "search quotes", 0.0);
    let p = a.command_line().expect("liste des citations");
    assert_eq!(p.title(), "Search for quotes");
    let (shown, _) = p.shown();
    assert!(shown.len() > 1000, "toutes les citations : {}", shown.len());
    let wanted = shown[42].display.clone();
    // chercher les deux premiers mots de la citation
    let words: Vec<&str> = wanted.split(' ').take(3).collect();
    for c in words.join(" ").chars() {
        a.handle(press(Key::Char(c), 0.0));
    }
    let id = {
        let (shown, active) = a.command_line().unwrap().shown();
        assert!(!shown.is_empty());
        match &shown[active].action {
            fasttype_tui::palette::Action::App(fasttype_tui::palette::AppAction::SelectQuote(
                id,
            )) => *id,
            other => panic!("{other:?}"),
        }
    };
    a.handle(press(Key::Enter, 0.0));
    let t = settle(&mut a, 0.0);
    assert_eq!(a.store.config.int_list("quoteLength"), [-2]);
    assert_eq!(a.session().spec().quote.as_ref().unwrap().id, id);
    // un restart garde la même citation
    a.handle(press(Key::Tab, t));
    a.handle(press(Key::Enter, t));
    settle(&mut a, t);
    assert_eq!(a.session().spec().quote.as_ref().unwrap().id, id);
}

#[test]
fn export_copies_the_settings_and_import_applies_them() {
    let mut a = app("ex-export", "time = 60\n");
    command(&mut a, "export", 0.0);
    let toml = a.take_clipboard().expect("réglages à copier");
    assert!(toml.contains("time = 60"), "{toml}");
    assert!(a.take_clipboard().is_none(), "une seule fois");
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text == "Settings copied to clipboard")
    );
    let original = *a.palette();
    command(&mut a, "import", 0.0);
    a.handle(Input::Paste("time = 15\ntheme = \"dracula\"\n".into()));
    a.handle(press(Key::Enter, 0.0));
    settle(&mut a, 0.0);
    assert_eq!(a.store.config.int("time"), 15);
    assert_eq!(a.store.config.str("theme"), "dracula");
    assert_ne!(*a.palette(), original, "le thème importé est appliqué");
    assert_eq!(a.session().spec().time_limit, Some(15));
}

#[test]
fn a_new_language_loads_in_the_background() {
    let mut a = app("ex-lang", "");
    command(&mut a, "language", 0.0);
    for c in "french 600k".chars() {
        a.handle(press(Key::Char(c), 0.0));
    }
    a.handle(press(Key::Enter, 0.0));
    assert_eq!(a.loading(), Some("french_600k"));
    assert!(
        a.next_deadline().is_some(),
        "on surveille la fin du chargement"
    );
    let (buf, _) = render(&mut a, 100, 30);
    assert!(screen_text(&buf).contains("loading..."));
    let mut t = 0.0;
    while a.loading().is_some() && t < 10_000.0 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        t += 5.0;
        a.tick(t);
    }
    assert!(a.loading().is_none(), "chargé");
    settle(&mut a, t);
    assert_eq!(a.session().spec().language, "french_600k");
}

#[test]
fn errors_stay_and_notices_hide_while_typing() {
    let mut a = app("ex-notif", "language = \"klingon_9000k\"\n");
    a.tick(100_000.0);
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("klingon")),
        "l'erreur reste jusqu'à ce qu'on l'efface"
    );
    assert_eq!(
        a.next_deadline(),
        None,
        "pas de réveil pour une erreur sans fin"
    );
    a.notifications.push(
        "Quick restart disabled in long tests. Use shift + tab.",
        fasttype_tui::view::notify::Level::Notice,
        100_000.0,
    );
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 100_000.0, 10.0);
    let (buf, _) = render(&mut a, 120, 30);
    let text = screen_text(&buf);
    assert!(text.contains("klingon"), "l'erreur reste visible");
    assert!(
        !text.contains("Quick restart"),
        "la notice se cache pendant la frappe"
    );
}
