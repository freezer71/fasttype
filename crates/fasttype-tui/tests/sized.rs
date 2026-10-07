use fasttype_tui::sized::{ScaledCell, ScaledText, probe_answer, scale_for};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

#[test]
fn text_sizing_probe_answers() {
    assert_eq!(probe_answer(b""), None);
    assert_eq!(
        probe_answer(b"\x1b[1;1R"),
        None,
        "attend la seconde position"
    );
    assert_eq!(probe_answer(b"\x1b[1;1R\x1b[1;3R"), Some(true));
    assert_eq!(
        probe_answer(b"\x1b[1;1R\x1b[1;2R"),
        Some(false),
        "un espace normal"
    );
    assert_eq!(probe_answer(b"\x1b[1;1R\x1b[1;1R"), Some(false), "ignoré");
}

#[test]
fn font_size_to_scale() {
    assert_eq!(scale_for(2.0), 2, "défaut du site : mots à 2rem");
    assert_eq!(scale_for(1.0), 1);
    assert_eq!(scale_for(1.5), 2);
    assert_eq!(scale_for(1.25), 1);
    assert_eq!(scale_for(10.0), 4);
    assert_eq!(scale_for(f64::NAN), 1);
}

#[test]
fn scaled_text_clears_the_region_then_writes_each_letter() {
    let t = ScaledText {
        scale: 2,
        region: Rect::new(4, 10, 3, 2),
        bg: Color::Rgb(1, 2, 3),
        cells: vec![ScaledCell {
            x: 4,
            y: 10,
            ch: 'a',
            style: Style::default()
                .fg(Color::Rgb(9, 9, 9))
                .bg(Color::Rgb(1, 2, 3)),
        }],
        hole: None,
    };
    let mut out = Vec::new();
    t.write(&mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    // le curseur est sauvé puis rendu (DECSC/DECRC) : le caret du terminal reste en place
    assert_eq!(
        s,
        "\x1b7\x1b[0;48;2;1;2;3m\x1b[11;5H   \x1b[12;5H   \x1b[11;5H\x1b[0;38;2;9;9;9;48;2;1;2;3m\x1b]66;s=2;a\x07\x1b[0m\x1b8"
    );
}

fn cell(x: u16, ch: char, fg: u8) -> ScaledCell {
    ScaledCell {
        x,
        y: 10,
        ch,
        style: Style::default().fg(Color::Rgb(fg, fg, fg)),
    }
}

fn text(cells: Vec<ScaledCell>) -> ScaledText {
    ScaledText {
        scale: 2,
        region: Rect::new(0, 10, 20, 2),
        bg: Color::Rgb(1, 2, 3),
        cells,
        hole: None,
    }
}

#[test]
fn only_changed_letters_are_rewritten() {
    let before = text(vec![cell(0, 'a', 9), cell(2, 'b', 9), cell(4, 'c', 9)]);
    // une lettre change de couleur, une disparaît
    let after = text(vec![cell(0, 'a', 9), cell(2, 'b', 7)]);
    let mut out = Vec::new();
    after.write_changes(Some(&before), &mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("\x1b7") && s.ends_with("\x1b8"));
    assert!(!s.contains("s=2;a"), "lettre inchangée : rien à écrire");
    assert!(s.contains("\x1b[11;3H\x1b[0;38;2;7;7;7m\x1b]66;s=2;b\x07"));
    assert!(
        s.contains("\x1b[11;5H  \x1b[12;5H  "),
        "le bloc de « c » est effacé"
    );
    assert!(s.len() < 120, "pas toute la zone : {}", s.len());
    // sans image précédente, ou si la zone a changé : tout est réécrit
    let mut full = Vec::new();
    after.write_changes(None, &mut full).unwrap();
    let mut direct = Vec::new();
    after.write(&mut direct).unwrap();
    assert_eq!(full, direct);
    let moved = ScaledText {
        region: Rect::new(0, 12, 20, 2),
        ..after.clone()
    };
    let mut out = Vec::new();
    moved.write_changes(Some(&after), &mut out).unwrap();
    assert!(
        String::from_utf8(out).unwrap().contains("\x1b[13;1H"),
        "zone effacée"
    );
}
