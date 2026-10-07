mod common;

use common::{app, press, release, type_text, type_whole_test};
use fasttype_core::event::EventKind;
use fasttype_core::session::SessionState;
use fasttype_core::spec::Mode;
use fasttype_store::RecordOutcome;
use fasttype_store::pbs::PbOutcome;
use fasttype_tui::app::Screen;
use fasttype_tui::input::Key;

#[test]
fn starts_on_a_test_of_english_words() {
    let a = app("start", "");
    assert!(matches!(a.screen(), Screen::Test));
    assert_eq!(a.session().spec().mode, Mode::Time);
    let english = fasttype_data::load_language("english").unwrap();
    for w in a.session().words().iter().take(20) {
        assert!(english.words.contains(&w.trim_end().to_string()), "{w:?}");
    }
    assert!(a.notifications.items().is_empty());
}

#[test]
fn a_finished_words_test_is_saved_with_a_personal_best() {
    let mut a = app("words", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 1000.0, 300.0);
    let Screen::Result(info) = a.screen() else {
        panic!("écran de résultat attendu")
    };
    assert_eq!(info.result.invalid, None, "{:?}", info.result);
    assert_eq!(
        info.outcome,
        Some(RecordOutcome::Saved(PbOutcome::NewBest { previous: None }))
    );
    assert_eq!(a.store.history().unwrap().results.len(), 1);
}

#[test]
fn tab_then_enter_restarts() {
    let mut a = app("tabenter", "");
    let first = a.session().words().to_vec();
    type_text(&mut a, "x", 0.0, 100.0);
    assert_eq!(a.session().state(), SessionState::Running);
    a.handle(press(Key::Tab, 200.0));
    a.handle(press(Key::Enter, 300.0));
    assert_eq!(a.session().state(), SessionState::Ready);
    assert_ne!(a.session().words(), first.as_slice(), "nouveaux mots");
}

#[test]
fn another_key_cancels_tab() {
    let mut a = app("disarm", "");
    let first = a.session().words().to_vec();
    a.handle(press(Key::Tab, 0.0));
    type_text(&mut a, "x", 10.0, 100.0);
    a.handle(press(Key::Enter, 200.0));
    assert_eq!(a.session().words(), first.as_slice());
    assert_eq!(a.session().state(), SessionState::Running);
}

#[test]
fn quick_restart_on_tab() {
    let mut a = app("quicktab", "quick_restart = \"tab\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.handle(press(Key::Tab, 200.0));
    assert_eq!(a.session().state(), SessionState::Ready);
}

#[test]
fn long_tests_need_shift_to_quick_restart() {
    let mut a = app("long", "quick_restart = \"tab\"\ntime = 0\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.handle(press(Key::Tab, 200.0));
    assert_eq!(a.session().state(), SessionState::Running);
    assert!(
        a.notifications.items()[0]
            .text
            .contains("Quick restart disabled")
    );
    a.handle(press(Key::BackTab, 300.0));
    assert_eq!(a.session().state(), SessionState::Ready);
}

#[test]
fn ctrl_c_quits() {
    let mut a = app("quit", "");
    a.handle(press(Key::Quit, 0.0));
    assert!(a.quit);
}

#[test]
fn unknown_language_falls_back_to_english() {
    let a = app("lang", "language = \"klingon_9000k\"\n");
    assert_eq!(a.session().spec().language, "english");
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("english"))
    );
}

#[test]
fn unknown_theme_falls_back_to_serika_dark() {
    let a = app("theme", "theme = \"nope\"\n");
    let serika = fasttype_tui::theme::Palette::from_theme(
        fasttype_data::theme("serika_dark").unwrap(),
        fasttype_tui::theme::ColorMode::TrueColor,
    );
    assert_eq!(*a.palette(), serika);
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("serika_dark"))
    );
}

#[test]
fn custom_theme_colors_are_used() {
    let a = app(
        "customtheme",
        "custom_theme = true\ncustom_theme_colors = [\"#000000\", \"#ff0000\", \"#ff0000\", \"#111111\", \"#222222\", \"#ffffff\", \"#00ff00\", \"#008800\", \"#00ff00\", \"#008800\"]\n",
    );
    assert_eq!(a.palette().main, ratatui::style::Color::Rgb(255, 0, 0));
}

#[test]
fn zen_ends_with_shift_enter() {
    let mut a = app("zen", "mode = \"zen\"\n");
    type_text(&mut a, "hi ", 0.0, 100.0);
    a.handle(press(Key::ShiftEnter, 500.0));
    assert!(matches!(a.screen(), Screen::Result(_)));
}

#[test]
fn time_test_ends_on_tick() {
    let mut a = app("time", "time = 15\n");
    type_text(&mut a, "x", 0.0, 100.0);
    assert_eq!(a.next_deadline(), Some(1000.0));
    a.tick(15_000.0);
    assert!(matches!(a.screen(), Screen::Result(_)));
}

#[test]
fn idle_test_needs_no_wakeup() {
    let a = app("idle", "");
    assert_eq!(a.next_deadline(), None, "0 % de CPU tant que rien ne bouge");
}

#[test]
fn key_release_is_logged() {
    let mut a = app("keyup", "");
    a.handle(press(Key::Char('x'), 0.0));
    a.handle(release(Key::Char('x'), 50.0));
    assert!(
        a.session()
            .log()
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::KeyUp { .. }))
    );
}

#[test]
fn result_screen_tab_enter_starts_next_test() {
    let mut a = app("next", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 0.0, 300.0);
    a.handle(press(Key::Tab, 99_000.0));
    a.handle(press(Key::Enter, 99_100.0));
    assert!(matches!(a.screen(), Screen::Test));
    assert_eq!(a.session().state(), SessionState::Ready);
}

#[test]
fn quote_mode_types_a_quote() {
    let a = app("quote", "mode = \"quote\"\n");
    assert_eq!(a.session().spec().mode, Mode::Quote);
    assert!(a.session().spec().quote.is_some());
}

#[test]
fn invalid_result_is_announced_and_not_saved() {
    let mut a = app("invalid", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 0.0, 5.0);
    let Screen::Result(info) = a.screen() else {
        panic!()
    };
    assert!(info.result.invalid.is_some());
    assert_eq!(info.outcome, Some(RecordOutcome::Invalid));
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.starts_with("Test invalid"))
    );
}

#[test]
fn zen_with_quick_restart_on_enter_keeps_newlines_and_ends_with_shift_enter() {
    let mut a = app("zen-enter", "mode = \"zen\"\nquick_restart = \"enter\"\n");
    type_text(&mut a, "hello", 0.0, 100.0);
    a.handle(press(Key::Enter, 600.0));
    assert!(
        a.session().inputs().concat().contains("hello"),
        "Entrée ne relance pas un test zen"
    );
    type_text(&mut a, "abc", 700.0, 100.0);
    a.handle(press(Key::ShiftEnter, 1100.0));
    assert!(matches!(a.screen(), Screen::Result(_)));
}

#[test]
fn unknown_language_warning_is_in_english() {
    let a = app("lang-en", "language = \"klingon_9000k\"\n");
    let texts: Vec<&str> = a
        .notifications
        .items()
        .iter()
        .map(|n| n.text.as_str())
        .collect();
    assert!(
        texts.contains(&"language klingon_9000k not found - using english"),
        "{texts:?}"
    );
}

#[test]
fn zen_result_screen_restarts_on_enter_with_quick_restart_enter() {
    let mut a = app(
        "zen-enter-result",
        "mode = \"zen\"\nquick_restart = \"enter\"\n",
    );
    type_text(&mut a, "hi ", 0.0, 100.0);
    a.handle(press(Key::ShiftEnter, 500.0));
    assert!(matches!(a.screen(), Screen::Result(_)));
    a.handle(press(Key::Enter, 900.0));
    assert!(matches!(a.screen(), Screen::Test));
}

#[test]
fn keys_after_ctrl_c_in_a_burst_are_dropped() {
    let mut a = app("burst-quit", "mode = \"words\"\nwords = 1\n");
    let word = a.session().word(0).trim_end().to_string();
    let mut chars = word.chars();
    let first = chars.next().unwrap();
    // la rafale : première lettre, Ctrl+C, puis la fin du mot et l'espace
    let rest: Vec<_> = std::iter::once(press(Key::Quit, 20.0))
        .chain(chars.map(|c| press(Key::Char(c), 30.0)))
        .chain(std::iter::once(press(Key::Char(' '), 40.0)))
        .collect();
    let oldest =
        fasttype_tui::runner::apply_burst(&mut a, press(Key::Char(first), 10.0), rest.into_iter());
    assert_eq!(oldest, Some(10.0));
    assert!(a.quit);
    assert!(
        matches!(a.screen(), Screen::Test),
        "test abandonné, pas enregistré"
    );
    assert!(a.store.history().unwrap().results.is_empty());
}

#[test]
fn a_signal_quits() {
    let mut a = app("signal", "");
    a.handle(fasttype_tui::input::Input::Interrupt);
    assert!(a.quit);
}
