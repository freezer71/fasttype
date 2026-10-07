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
    };
    let mut out = Vec::new();
    t.write(&mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(
        s,
        "\x1b[0;48;2;1;2;3m\x1b[11;5H   \x1b[12;5H   \x1b[11;5H\x1b[0;38;2;9;9;9;48;2;1;2;3m\x1b]66;s=2;a\x07\x1b[0m"
    );
}
