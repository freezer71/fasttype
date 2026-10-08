mod common;

use common::{app, col, press, render, row, screen_text, settle, type_text, type_whole_test};
use fasttype_tui::input::Key;
use fasttype_tui::kitty::{CaretRenderer, CellPx};
use ratatui::buffer::CellDiffOption;
use ratatui::style::Color;

#[test]
fn wide_terminal_puts_the_bar_next_to_the_logo() {
    let mut a = app("bar-wide", "");
    let (buf, _) = render(&mut a, 140, 30);
    assert!(row(&buf, 1).contains("@ punctuation"));
    let x = col(&buf, 1, "30").unwrap();
    assert_eq!(buf[(x, 1)].fg, a.palette().main, "valeur en cours");
    assert_eq!(buf[(x, 1)].bg, a.palette().sub_alt, "sur fond subAlt");
}

#[test]
fn words_are_scaled_when_the_terminal_supports_it() {
    let mut a = app("sized", "");
    a.set_text_sizing(true);
    a.set_caret_renderer(CaretRenderer::Kitty(CellPx { w: 10, h: 20 }));
    let (buf, cursor) = render(&mut a, 120, 30);
    assert_eq!(cursor, None);
    let t = a.scaled_text().expect("mots agrandis").clone();
    assert_eq!(t.scale, 2, "fontSize 2 par défaut");
    assert_eq!(t.region.height, 6, "3 lignes de 2 cases");
    let first = a.session().word(0).chars().next().unwrap();
    assert_eq!(t.cells[0].ch, first);
    assert_eq!((t.cells[1].x - t.cells[0].x), 2, "une lettre = 2 colonnes");
    // ratatui n'écrit pas dans la zone réservée : le terminal n'en reçoit rien
    let r = t.region;
    assert_eq!(buf[(r.x, r.y)].bg, Color::Reset);
    assert_ne!(buf[(r.x, r.y - 1)].bg, Color::Reset, "le reste est peint");
    // le caret prend la taille des lettres
    let f = a.caret_frame().unwrap();
    assert_eq!(f.height, 2.0);
    assert_eq!((f.x, f.y), (f64::from(r.x), f64::from(r.y)));
    // la frappe avance d'une lettre agrandie
    a.handle(press(Key::Char(first), 1000.0));
    a.tick(1500.0);
    render(&mut a, 120, 30);
    assert_eq!(a.caret_frame().unwrap().x, f64::from(r.x) + 2.0);
}

#[test]
fn without_support_or_room_words_stay_normal_size() {
    let mut a = app("sized-off", "");
    render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none(), "terminal sans OSC 66");
    let mut a = app("sized-small", "");
    a.set_text_sizing(true);
    render(&mut a, 50, 12);
    assert!(a.scaled_text().is_none(), "pas la place : échelle 1");
    let mut a = app("sized-1", "font_size = 1.0\n");
    a.set_text_sizing(true);
    render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none(), "fontSize 1");
}

#[test]
fn block_caret_tints_scaled_letters() {
    let mut a = app("sized-block", "caret_style = \"block\"\n");
    a.set_text_sizing(true);
    // mi-période du clignotement : caret pleinement visible
    a.tick(500.0);
    render(&mut a, 120, 30);
    let t = a.scaled_text().unwrap();
    let caret = a.palette().caret;
    assert_eq!(t.cells[0].style.bg, Some(caret), "la lettre sous le caret");
    assert_ne!(t.cells[1].style.bg, Some(caret));
}

#[test]
fn leaving_the_scaled_screen_redraws_its_region() {
    let mut a = app("sized-leave", "mode = \"words\"\nwords = 10\n");
    a.set_text_sizing(true);
    render(&mut a, 120, 30);
    let r = a.scaled_text().unwrap().region;
    let end = type_whole_test(&mut a, 0.0, 20.0);
    settle(&mut a, end);
    let (buf, _) = render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none(), "écran de résultat");
    assert_eq!(
        buf[(r.x, r.y)].diff_option,
        CellDiffOption::AlwaysUpdate,
        "les lettres agrandies restées à l'écran sont effacées"
    );
    let (buf, _) = render(&mut a, 120, 30);
    assert_eq!(
        buf[(r.x, r.y)].diff_option,
        CellDiffOption::None,
        "une seule fois"
    );
}

#[test]
fn quote_result_shows_the_source() {
    let mut a = app("quote-source", "mode = \"quote\"\n");
    let source = a.session().spec().quote.as_ref().unwrap().source.clone();
    let end = type_whole_test(&mut a, 0.0, 20.0);
    settle(&mut a, end);
    let (buf, _) = render(&mut a, 140, 40);
    let text = screen_text(&buf);
    assert!(text.contains("source"), "{text}");
    let shown: String = source.chars().take(20).collect();
    assert!(text.contains(&shown));
}

#[test]
fn language_badge_fades_with_the_chrome() {
    let mut a = app("badge", "");
    let (buf, _) = render(&mut a, 120, 30);
    let y = (0..30u16)
        .find(|&y| row(&buf, y).trim() == "english")
        .expect("badge");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 50.0);
    a.tick(400.0);
    let (buf, _) = render(&mut a, 120, 30);
    assert!(!row(&buf, y).contains("english"), "caché pendant la frappe");
}

#[test]
fn the_config_bar_never_covers_the_words() {
    // 80 × 10 : la barre compacte irait sur la ligne des mots
    let mut a = app("bar-low", "");
    let (buf, caret) = render(&mut a, 80, 10);
    let (_, y) = caret.expect("caret");
    let first = a.session().word(0).trim_end().to_string();
    assert!(
        row(&buf, y).contains(&first),
        "la ligne du caret montre ses mots"
    );
    assert!(
        !screen_text(&buf).contains("words"),
        "pas de place pour la barre"
    );
}

#[test]
fn bar_and_language_badge_each_get_their_row() {
    let mut a = app("badge-rows", "");
    let (buf, _) = render(&mut a, 80, 16);
    let bar = (0..16u16)
        .find(|&y| row(&buf, y).contains("time"))
        .expect("barre");
    let badge = (0..16u16)
        .find(|&y| row(&buf, y).trim() == "english")
        .expect("badge seul sur sa ligne");
    assert!(badge > bar);
    // 80 × 13 : le badge tomberait sur la ligne de la barre : il s'efface
    let mut a = app("badge-clash", "");
    let (buf, _) = render(&mut a, 80, 13);
    let bar = (0..13u16)
        .find(|&y| row(&buf, y).contains("time"))
        .expect("barre");
    assert!(row(&buf, bar).contains("@") && row(&buf, bar).contains("custom"));
}

#[test]
fn words_use_the_whole_width_like_the_site() {
    use fasttype_tui::view::test::words_box;
    use ratatui::layout::Rect;
    // comme le site (maxLineWidth 0) : presque toute la largeur, sans plafond
    let w = words_box(Rect::new(0, 0, 200, 50), 0, false, 1);
    assert_eq!(w.width, 192);
    let w = words_box(Rect::new(0, 0, 200, 50), 0, false, 2);
    assert_eq!(w.width, 96, "96 lettres agrandies");
    assert_eq!(w.left, 4);
    // maxLineWidth limite toujours la ligne, en lettres
    let w = words_box(Rect::new(0, 0, 200, 50), 60, false, 2);
    assert_eq!(w.width, 60);
}
