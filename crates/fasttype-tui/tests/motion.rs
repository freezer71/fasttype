mod common;

use common::{app, press, render, row, screen_text, settle, type_text, type_whole_test};
use fasttype_tui::app::FADE_MS;
use fasttype_tui::input::Key;
use fasttype_tui::kitty::{CaretRenderer, CellPx};
use ratatui::style::Color;

/// Lettre juste suivante du test en cours.
fn next_char(a: &fasttype_tui::app::App) -> char {
    let s = a.session();
    let i = s.active_index();
    s.word(i)
        .chars()
        .nth(s.input(i).chars().count())
        .unwrap_or(' ')
}

#[test]
fn cell_caret_glides_letter_by_letter() {
    // smoothCaret slow : 150 ms
    let mut a = app("m-glide", "smooth_caret = \"slow\"\n");
    let (_, start) = render(&mut a, 80, 24);
    let (x0, y0) = start.unwrap();
    let c = next_char(&a);
    a.handle(press(Key::Char(c), 1000.0));
    let (_, mid) = render(&mut a, 80, 24);
    assert_eq!(
        mid,
        Some((x0, y0)),
        "le glissement part de l'ancienne position"
    );
    assert!(
        a.next_deadline().is_some_and(|d| d < 1020.0),
        "images d'animation"
    );
    a.tick(1150.0);
    let (_, end) = render(&mut a, 80, 24);
    assert_eq!(end, Some((x0 + 1, y0)));
}

#[test]
fn block_caret_tints_two_cells_mid_glide() {
    let mut a = app(
        "m-block",
        "caret_style = \"block\"\nsmooth_caret = \"medium\"\n",
    );
    let (buf, cursor) = render(&mut a, 80, 24);
    assert_eq!(cursor, None, "le bloc n'utilise pas le curseur du terminal");
    let p = *a.palette();
    // caret au repos sur la première lettre : son fond est la couleur du caret
    let (x0, y0) = (0..80u16)
        .flat_map(|x| (0..24u16).map(move |y| (x, y)))
        .find(|&(x, y)| buf[(x, y)].bg != p.bg)
        .expect("une case teintée");
    let c = next_char(&a);
    a.handle(press(Key::Char(c), 1000.0));
    a.tick(1050.0);
    let (buf, _) = render(&mut a, 80, 24);
    let left = buf[(x0, y0)].bg;
    let right = buf[(x0 + 1, y0)].bg;
    assert_ne!(left, p.bg);
    assert_ne!(right, p.bg);
    assert_ne!(left, p.caret, "à mi-chemin, aucune case n'est pleine");
}

#[test]
fn caret_jumps_without_animation_after_a_restart_and_a_resize() {
    let mut a = app(
        "m-reset",
        "quick_restart = \"tab\"\nsmooth_caret = \"slow\"\n",
    );
    render(&mut a, 80, 24);
    type_text(&mut a, "x", 1000.0, 10.0);
    a.handle(press(Key::Tab, 1100.0));
    let t = settle(&mut a, 1100.0);
    let (_, after_restart) = render(&mut a, 80, 24);
    let (_, fresh) = render(&mut app("m-reset-ref", ""), 80, 24);
    assert_eq!(after_restart, fresh, "replacé au début, sans glisser");
    let (_, resized) = render(&mut a, 120, 30);
    let (_, fresh) = render(&mut app("m-reset-ref2", ""), 120, 30);
    assert_eq!(resized, fresh);
    assert!(a.next_deadline().is_none_or(|d| d > t + 100.0));
}

#[test]
fn kitty_renderer_hides_the_cursor_and_exposes_a_pixel_frame() {
    let mut a = app("m-kitty", "");
    a.set_caret_renderer(CaretRenderer::Kitty(CellPx { w: 10, h: 20 }));
    let (_, cursor) = render(&mut a, 80, 24);
    assert_eq!(cursor, None);
    let f = a.caret_frame().expect("caret à dessiner en image");
    assert_eq!(f.y.fract(), 0.0);
    // le caret clignote avant la première frappe : des images sont prévues
    assert!(a.next_deadline().is_some());
    assert!(
        a.caret_look().is_none(),
        "le curseur du terminal n'est pas utilisé"
    );
}

#[test]
fn smooth_line_scroll_fades_the_new_line_in() {
    let mut a = app(
        "m-scroll",
        "smooth_line_scroll = true\nmode = \"words\"\nwords = 200\n",
    );
    render(&mut a, 60, 24);
    let mut t = 1000.0;
    // tape jusqu'au début de la 3e ligne : la ligne du haut est retirée
    let top0 = a.session().words()[0].clone();
    while screen_text(&render(&mut a, 60, 24).0).contains(top0.trim_end()) && t < 60_000.0 {
        let c = next_char(&a);
        a.handle(press(Key::Char(c), t));
        a.tick(t);
        t += 50.0;
    }
    assert!(t < 60_000.0, "le défilement a eu lieu");
    let (buf, _) = render(&mut a, 60, 24);
    // dernière ligne visible encore pâle : sub mêlé au fond
    let p = *a.palette();
    let last = (0..24u16)
        .rev()
        .find(|&y| row(&buf, y).trim().chars().any(char::is_alphabetic) && y < 20)
        .unwrap();
    let x = row(&buf, last).find(|c: char| c.is_alphabetic()).unwrap() as u16;
    assert_ne!(buf[(x, last)].fg, p.sub, "en cours d'apparition");
    a.tick(t + FADE_MS);
    let (buf, _) = render(&mut a, 60, 24);
    assert_eq!(buf[(x, last)].fg, p.sub);
}

#[test]
fn result_fades_in_after_the_test_fades_out() {
    let mut a = app("m-result-fade", "mode = \"words\"\nwords = 10\n");
    render(&mut a, 100, 30);
    // frappe rapide : le relâchement de la dernière touche arrive avant la mi-fondu
    let end = type_whole_test(&mut a, 0.0, 20.0);
    let start = match a.transition() {
        Some(fasttype_tui::app::Transition::ToResult { start }) => start,
        other => panic!("{other:?}"),
    };
    // première moitié : le test pâlit, le résultat n'est pas encore là
    a.tick(start + FADE_MS / 2.0);
    let (buf, _) = render(&mut a, 100, 30);
    assert!(!screen_text(&buf).contains("characters"));
    // seconde moitié : le résultat apparaît
    a.tick(start + FADE_MS * 1.5);
    let (buf, _) = render(&mut a, 100, 30);
    let text = screen_text(&buf);
    assert!(text.contains("characters"));
    let y = (0..30u16)
        .find(|&y| row(&buf, y).contains("characters"))
        .unwrap();
    let x = row(&buf, y).find("characters").unwrap() as u16;
    assert_ne!(buf[(x, y)].fg, a.palette().sub, "encore pâle");
    settle(&mut a, end.max(start));
    let (buf, _) = render(&mut a, 100, 30);
    assert_eq!(buf[(x, y)].fg, a.palette().sub);
}

#[test]
fn live_stats_mini_text_and_bar() {
    let cfg =
        "live_speed_style = \"mini\"\nlive_acc_style = \"mini\"\nlive_burst_style = \"mini\"\n";
    let mut a = app("m-live-mini", cfg);
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(1000.0);
    a.tick(1200.0);
    let (buf, _) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(
        text.contains("29 0 0% 0"),
        "timer, vitesse, précision, burst : {text}"
    );

    let mut a = app("m-live-text", "timer_style = \"text\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    assert!(
        screen_text(&buf).contains("▀▀█"),
        "timer en grands chiffres"
    );

    let mut a = app("m-live-bar", "timer_style = \"bar\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(500.0);
    let (buf, _) = render(&mut a, 80, 24);
    let bar = row(&buf, 0).chars().filter(|&c| c == '▀').count();
    assert!(bar > 60 && bar < 80, "barre qui rétrécit : {bar}");
    assert_eq!(
        buf[(0, 0)].fg,
        a.palette().main,
        "couleur timerColor (main)"
    );
}

#[test]
fn timer_style_off_and_opacity() {
    let mut a = app("m-timer-op", "timer_opacity = \"0.5\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    let y = (0..24u16)
        .find(|&y| row(&buf, y).trim() == "30")
        .expect("timer");
    let x = row(&buf, y).find("30").unwrap() as u16;
    let p = a.palette();
    assert_ne!(buf[(x, y)].fg, p.main, "mi-opaque");
    assert!(matches!(buf[(x, y)].fg, Color::Rgb(..)));
}

#[test]
fn backspacing_into_a_removed_line_shows_it_again() {
    // mots faux : le retour arrière peut remonter dans les mots précédents
    let mut a = app("m-back", "mode = \"words\"\nwords = 100\n");
    let mut t = 0.0;
    for _ in 0..30 {
        t = type_text(&mut a, "zz ", t, 20.0);
        a.tick(t);
        render(&mut a, 40, 20);
    }
    let far = a.session().active_index();
    for _ in 0..200 {
        a.handle(press(Key::Backspace, t));
        t += 20.0;
        a.tick(t);
        render(&mut a, 40, 20);
    }
    assert!(
        a.session().active_index() + 10 < far,
        "remonté de plusieurs lignes"
    );
    let (buf, cursor) = render(&mut a, 40, 20);
    let (x, y) = cursor.expect("caret visible");
    assert!(x < 40 && y < 20);
    // la ligne du caret est à l'écran
    assert!(!row(&buf, y).trim().is_empty());
}
