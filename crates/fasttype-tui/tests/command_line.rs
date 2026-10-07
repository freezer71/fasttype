mod common;

use common::{app, press, render, row, screen_text, settle, type_text, type_whole_test};
use fasttype_core::session::SessionState;
use fasttype_tui::app::{App, Screen};
use fasttype_tui::input::{Input, Key};

fn type_query(a: &mut App, text: &str, at: f64) {
    for c in text.chars() {
        a.handle(press(Key::Char(c), at));
    }
}

#[test]
fn escape_opens_and_closes_the_command_line() {
    let mut a = app("cl-open", "");
    a.handle(press(Key::Esc, 0.0));
    assert!(a.command_line().is_some());
    // les lettres vont à la recherche, pas au test
    type_query(&mut a, "caret", 10.0);
    assert_eq!(a.session().state(), SessionState::Ready);
    assert_eq!(a.command_line().unwrap().query(), "caret");
    a.handle(press(Key::Esc, 20.0));
    assert!(a.command_line().is_none());
    a.handle(press(Key::Palette, 30.0));
    assert!(a.command_line().is_some(), "Ctrl+Shift+P aussi");
}

#[test]
fn with_quick_restart_on_escape_tab_opens_it() {
    let mut a = app("cl-tab", "quick_restart = \"esc\"\n");
    a.handle(press(Key::Tab, 0.0));
    assert!(a.command_line().is_some());
    a.handle(press(Key::Esc, 10.0));
    assert!(a.command_line().is_none());
    a.handle(press(Key::Esc, 20.0));
    assert!(a.command_line().is_none(), "Échap relance");
}

#[test]
fn choosing_a_value_saves_it_and_restarts() {
    let mut a = app("cl-set", "");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "time", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Down, 0.0));
    a.handle(press(Key::Enter, 0.0));
    assert!(a.command_line().is_none());
    assert_eq!(a.store.config.int("time"), 60);
    let dir = std::env::temp_dir().join(format!("fasttype-tui-{}-cl-set", std::process::id()));
    let saved = std::fs::read_to_string(dir.join("config/config.toml")).unwrap();
    assert!(saved.contains("time = 60"), "{saved}");
    settle(&mut a, 0.0);
    assert_eq!(
        a.session().spec().time_limit,
        Some(60),
        "nouveau test de 60 s"
    );
}

#[test]
fn hovering_a_theme_previews_it_and_escape_reverts() {
    let mut a = app("cl-theme", "");
    let original = *a.palette();
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "theme", 0.0);
    a.handle(press(Key::Enter, 0.0));
    // le curseur part du thème en cours ; on descend sur le suivant
    a.handle(press(Key::Down, 0.0));
    let hovered = a.command_line().unwrap().hovered().unwrap().display.clone();
    assert_ne!(*a.palette(), original, "aperçu de {hovered}");
    a.handle(press(Key::Esc, 0.0));
    a.handle(press(Key::Esc, 0.0));
    assert!(a.command_line().is_none());
    assert_eq!(*a.palette(), original, "retour au thème en cours");
    assert_eq!(a.store.config.str("theme"), "serika_dark");
}

#[test]
fn choosing_a_theme_applies_it() {
    let mut a = app("cl-theme-set", "");
    let original = *a.palette();
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "theme", 0.0);
    a.handle(press(Key::Enter, 0.0));
    type_query(&mut a, "dracula", 0.0);
    a.handle(press(Key::Enter, 0.0));
    assert_eq!(a.store.config.str("theme"), "dracula");
    assert_ne!(*a.palette(), original);
}

#[test]
fn repeat_test_replays_the_same_words() {
    let mut a = app("cl-repeat", "mode = \"words\"\nwords = 10\n");
    let words = a.session().words().to_vec();
    let end = type_whole_test(&mut a, 0.0, 20.0);
    let t = settle(&mut a, end);
    a.handle(press(Key::Esc, t));
    assert_eq!(
        a.command_line().unwrap().hovered().unwrap().display,
        "Next test"
    );
    a.handle(press(Key::Down, t));
    a.handle(press(Key::Enter, t));
    settle(&mut a, t);
    assert!(matches!(a.screen(), Screen::Test));
    assert_eq!(a.session().words(), words.as_slice());
    assert!(a.session().is_repeated());
}

#[test]
fn bail_out_ends_a_zen_test() {
    let mut a = app("cl-bail", "mode = \"zen\"\n");
    type_text(&mut a, "abc ", 0.0, 50.0);
    a.handle(press(Key::Esc, 300.0));
    type_query(&mut a, "bail", 300.0);
    a.handle(press(Key::Enter, 300.0));
    a.handle(press(Key::Down, 300.0));
    a.handle(press(Key::Enter, 300.0));
    match a.screen() {
        Screen::Result(info) => assert!(info.result.bailed_out),
        _ => panic!("écran de résultat attendu"),
    }
}

#[test]
fn clear_all_notifications() {
    let mut a = app("cl-clear", "language = \"klingon_9000k\"\n");
    assert!(!a.notifications.items().is_empty());
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "dismiss", 0.0);
    a.handle(press(Key::Enter, 0.0));
    assert!(a.notifications.items().is_empty());
}

#[test]
fn quit_command() {
    let mut a = app("cl-quit", "");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "quit", 0.0);
    a.handle(press(Key::Enter, 0.0));
    assert!(a.quit);
}

#[test]
fn pasted_text_goes_to_the_search() {
    let mut a = app("cl-paste", "");
    a.handle(Input::Paste("smooth caret".into()));
    assert!(
        a.command_line().is_none(),
        "palette fermée : collage ignoré"
    );
    a.handle(press(Key::Esc, 0.0));
    a.handle(Input::Paste("smooth\ncaret".into()));
    assert_eq!(a.command_line().unwrap().query(), "smooth caret");
}

#[test]
fn command_line_is_drawn_over_the_test() {
    let mut a = app("cl-draw", "");
    a.handle(press(Key::Esc, 0.0));
    let (buf, cursor) = render(&mut a, 100, 30);
    let text = screen_text(&buf);
    assert!(text.contains("Search..."), "{text}");
    assert!(text.contains("Punctuation..."));
    let (x, y) = cursor.expect("curseur dans la saisie");
    assert!(row(&buf, y).contains("Search..."));
    assert!(x > 0);
    // la ligne active est inversée
    let line = (0..30u16)
        .find(|&yy| row(&buf, yy).contains("Punctuation..."))
        .unwrap();
    let col = row(&buf, line).find("Punctuation").unwrap() as u16;
    assert_eq!(buf[(col, line)].bg, a.palette().text);
    // le fond est assombri
    assert_ne!(buf[(0, 0)].bg, a.palette().bg);
}

#[test]
fn key_tips_follow_quick_restart() {
    let mut a = app("cl-tips", "");
    let (buf, _) = render(&mut a, 100, 30);
    assert!(screen_text(&buf).contains("tab + enter - restart   esc - command line"));
    let mut a = app("cl-tips-esc", "quick_restart = \"esc\"\n");
    let (buf, _) = render(&mut a, 100, 30);
    assert!(screen_text(&buf).contains("esc - restart   tab - command line"));
    let mut a = app("cl-tips-off", "show_key_tips = false\n");
    let (buf, _) = render(&mut a, 100, 30);
    assert!(!screen_text(&buf).contains("restart"));
}

#[test]
fn scaled_words_make_room_for_the_command_line() {
    let mut a = app("cl-scaled", "");
    a.set_text_sizing(true);
    render(&mut a, 120, 30);
    a.handle(press(Key::Esc, 0.0));
    render(&mut a, 120, 30);
    let t = a.scaled_text().expect("mots agrandis").clone();
    let hole = t.hole.expect("la palette recouvre une partie de la zone");
    assert!(hole.intersects(t.region));
    assert!(a.caret_frame().is_none(), "caret caché pendant la palette");
}

#[test]
fn command_line_fits_any_terminal_size() {
    let mut a = app("cl-small", "");
    a.handle(press(Key::Esc, 0.0));
    for (w, h) in [(40, 10), (41, 11), (60, 12), (200, 60), (0, 0), (10, 3)] {
        render(&mut a, w, h);
    }
    // une saisie libre avec une erreur, au plus petit
    type_query(&mut a, "font", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Char('x'), 0.0));
    a.handle(press(Key::Enter, 0.0));
    for (w, h) in [(40, 10), (100, 30)] {
        let (buf, _) = render(&mut a, w, h);
        if w == 100 {
            assert!(screen_text(&buf).contains("Must be a number"));
        }
    }
}

#[test]
fn a_test_that_ends_behind_the_command_line_keeps_it_usable() {
    let mut a = app("cl-ends", "time = 15\n");
    type_text(&mut a, "x", 0.0, 50.0);
    a.handle(press(Key::Esc, 100.0));
    a.tick(15_000.0);
    assert!(
        matches!(a.screen(), Screen::Result(_)),
        "le test continue derrière"
    );
    assert!(a.command_line().is_some());
    render(&mut a, 100, 30);
    a.handle(press(Key::Esc, 15_100.0));
    assert!(a.command_line().is_none());
}

#[test]
fn the_screen_is_dimmed_in_256_colours_too() {
    use fasttype_tui::theme::ColorMode;
    let store = common::store_with("cl-256", "");
    let mut a = App::new(store, ColorMode::Ansi256, 0.0, 1);
    let (before, _) = render(&mut a, 100, 30);
    a.handle(press(Key::Esc, 0.0));
    let (after, _) = render(&mut a, 100, 30);
    assert_ne!(after[(0, 0)].bg, before[(0, 0)].bg, "voile en 256 couleurs");
}

#[test]
fn a_previewed_theme_is_not_dimmed() {
    let mut a = app("cl-preview-light", "");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "theme", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Down, 0.0));
    let (buf, _) = render(&mut a, 100, 30);
    assert_eq!(
        buf[(0, 29)].bg,
        a.palette().bg,
        "le thème survolé, sans voile"
    );
}

#[test]
fn tab_then_the_command_line_disarms_the_restart() {
    let mut a = app("cl-armed", "");
    type_text(&mut a, "x", 0.0, 50.0);
    a.handle(press(Key::Tab, 100.0));
    a.handle(press(Key::Esc, 110.0));
    a.handle(press(Key::Esc, 120.0));
    a.handle(press(Key::Enter, 130.0));
    assert_eq!(a.transition(), None, "pas de restart surprise");
    assert_eq!(a.session().state(), SessionState::Running);
}

#[test]
fn choosing_the_current_value_still_restarts_and_theme_leaves_custom_colours() {
    let mut a = app("cl-same", "");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "time", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Enter, 0.0));
    assert!(
        a.transition().is_some(),
        "time 30 choisi à nouveau : restart"
    );
    let mut a = app("cl-custom-theme", "custom_theme = true\n");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "theme", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Enter, 0.0));
    assert!(
        !a.store.config.bool("customTheme"),
        "le thème choisi remplace le thème custom"
    );
}
