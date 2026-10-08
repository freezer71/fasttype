use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_tui::glyphs::{FIRST_ID, GlyphText, has_glyph, render_glyph};
use fasttype_tui::kitty::CellPx;
use fasttype_tui::sized::{ScaledCell, ScaledText};
use fasttype_tui::theme::{ColorMode, Palette};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

const CELL: CellPx = CellPx { w: 10, h: 20 };

fn palette() -> Palette {
    Palette::from_theme(theme(DEFAULT_THEME).unwrap(), ColorMode::TrueColor)
}

fn cell(x: u16, ch: char, fg: Color) -> ScaledCell {
    ScaledCell {
        x,
        y: 10,
        ch,
        style: Style::default().fg(fg),
    }
}

fn layer(cells: Vec<ScaledCell>, bg: Color) -> ScaledText {
    ScaledText {
        scale: 2,
        region: Rect::new(0, 10, 40, 2),
        bg,
        cells,
        hole: None,
    }
}

fn text(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

#[test]
fn the_embedded_font_covers_latin_and_cyrillic() {
    for c in ['a', 'Z', 'é', 'ç', 'ß', 'ж', 'λ', ' ', '\'', '?'] {
        assert!(has_glyph(c), "{c}");
    }
    for c in ['日', 'ש', 'ก'] {
        assert!(!has_glyph(c), "{c} : pas dans Roboto Mono");
    }
}

#[test]
fn a_letter_fills_its_scaled_block() {
    let (w, h, px) = render_glyph('a', CELL, 2, 1, (255, 255, 255), None);
    assert_eq!((w, h), (20, 40), "2 × 2 cases de 10 × 20 pixels");
    let alpha = |x: u32, y: u32| px[((y * w + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 0), 0, "fond transparent");
    assert!(
        (0..w).any(|x| (0..h).any(|y| alpha(x, y) == 255)),
        "la lettre est dessinée"
    );
    let (w, _, _) = render_glyph('a', CELL, 2, 2, (255, 255, 255), None);
    assert_eq!(w, 40, "lettre large : deux fois plus");
    // le soulignement des mots faux tient dans le bloc
    let (_, _, px) = render_glyph('a', CELL, 2, 1, (255, 255, 255), Some((255, 0, 0)));
    let red_row = (0..40u32).find(|&y| {
        let k = ((y * 20 + 1) * 4) as usize;
        px[k] == 255 && px[k + 1] == 0 && px[k + 3] == 255
    });
    assert!(red_row.is_some_and(|y| y > 25 && y < 40), "{red_row:?}");
}

#[test]
fn letters_are_sent_once_and_only_changes_are_redrawn() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let first = layer(vec![cell(0, 'a', p.text), cell(2, 'b', p.sub)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &first, None, &p.rgb).unwrap();
    let s = text(out);
    assert!(
        s.starts_with("\x1b7") && s.ends_with("\x1b8"),
        "curseur rendu au caret"
    );
    assert_eq!(s.matches("a=t,").count(), 2, "deux images transmises");
    assert!(s.contains(&format!("\x1b[11;1H\x1b_Ga=p,i={FIRST_ID},p=")));
    // même image, nouvelle position ; même lettre dans un fondu : couleur ramenée au thème
    let faded = Color::Rgb(0xd0, 0xcf, 0xc4);
    let next = layer(
        vec![
            cell(0, 'a', p.text),
            cell(2, 'b', p.sub),
            cell(4, 'a', faded),
        ],
        p.bg,
    );
    let mut out = Vec::new();
    g.write(&mut out, &next, Some(&first), &p.rgb).unwrap();
    let s = text(out);
    assert!(!s.contains("a=t,"), "aucune nouvelle image : {s}");
    assert_eq!(
        s.matches("a=p,").count(),
        1,
        "seule la nouvelle lettre est placée"
    );
    assert!(s.contains(&format!("i={FIRST_ID},")));
    // une lettre disparaît : son placement est retiré, son fond repeint
    let last = layer(vec![cell(0, 'a', p.text), cell(2, 'b', p.sub)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &last, Some(&next), &p.rgb).unwrap();
    let s = text(out);
    assert!(s.contains("a=d,d=i,"), "{s}");
    assert!(s.contains("\x1b[11;5H  \x1b[12;5H  "));
    assert!(!s.contains("a=p,"));
}

#[test]
fn wrong_words_are_underlined_in_the_image() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let wrong = ScaledCell {
        style: Style::default()
            .fg(p.text)
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(p.error),
        ..cell(0, 'a', p.text)
    };
    let mut out = Vec::new();
    g.write(
        &mut out,
        &layer(vec![cell(2, 'a', p.text), wrong], p.bg),
        None,
        &p.rgb,
    )
    .unwrap();
    assert_eq!(
        text(out).matches("a=t,").count(),
        2,
        "souligné : une autre image"
    );
}

#[test]
fn hiding_moving_and_retheming() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let l = layer(vec![cell(0, 'a', p.text)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &l, None, &p.rgb).unwrap();
    // plus de mots agrandis : toutes les lettres retirées, images gardées
    let mut out = Vec::new();
    g.clear(&mut out).unwrap();
    assert_eq!(
        text(out),
        format!("\x1b_Ga=d,d=r,x={FIRST_ID},y=1999999,q=2\x1b\\")
    );
    // la palette couvre la lettre : elle n'est pas placée
    let covered = ScaledText {
        hole: Some(Rect::new(0, 10, 4, 2)),
        ..l.clone()
    };
    let mut out = Vec::new();
    g.write(&mut out, &covered, None, &p.rgb).unwrap();
    assert!(!text(out).contains("a=p,"));
    // un autre thème : les anciennes images sont libérées
    let other = Palette::from_theme(theme("dracula").unwrap(), ColorMode::TrueColor);
    let mut out = Vec::new();
    g.write(
        &mut out,
        &layer(vec![cell(0, 'a', other.text)], other.bg),
        None,
        &other.rgb,
    )
    .unwrap();
    let s = text(out);
    assert!(s.contains("a=d,d=R,"), "images libérées");
    assert!(s.contains("a=t,"), "nouvelle image");
}

#[test]
fn images_are_compressed_and_small() {
    let (w, h, px) = render_glyph('m', CELL, 2, 1, (209, 208, 197), None);
    let sent = fasttype_tui::glyphs::transmit_compressed(FIRST_ID, w, h, &px);
    let s = String::from_utf8(sent.clone()).unwrap();
    assert!(s.starts_with(&format!(
        "\x1b_Ga=t,f=32,s=20,v=40,i={FIRST_ID},o=z,q=2,m=0;"
    )));
    assert!(sent.len() < 1200, "{} octets au lieu de ~4300", sent.len());
}

#[test]
fn a_letter_faded_to_the_background_is_not_drawn() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let near_bg = Color::Rgb(0x34, 0x36, 0x39);
    let mut out = Vec::new();
    g.write(
        &mut out,
        &layer(vec![cell(0, 'a', near_bg)], p.bg),
        None,
        &p.rgb,
    )
    .unwrap();
    let s = text(out);
    assert!(!s.contains("a=t,") && !s.contains("a=p,"), "{s}");
}

#[test]
fn words_under_the_palette_veil_stay_visible() {
    use fasttype_tui::view::palette::{dim_color, dim_style};
    let p = palette();
    let mut g = GlyphText::new(CELL);
    // voile : couleurs et fond à moitié, comme `dim_style`
    let veiled = |c: ScaledCell| ScaledCell {
        style: dim_style(c.style),
        ..c
    };
    let l = layer(
        vec![veiled(cell(0, 'a', p.sub)), veiled(cell(2, 'b', p.text))],
        dim_color(p.bg),
    );
    let mut out = Vec::new();
    g.write(&mut out, &l, None, &p.rgb).unwrap();
    assert_eq!(
        text(out).matches("a=p,").count(),
        2,
        "les mots à venir restent visibles"
    );
}

#[test]
fn a_theme_change_with_the_same_background_redraws_every_letter() {
    let a = Palette::from_theme(theme("dark").unwrap(), ColorMode::TrueColor);
    let b = Palette::from_theme(theme("rgb").unwrap(), ColorMode::TrueColor);
    assert_eq!(a.bg, b.bg, "deux thèmes au même fond");
    let mut g = GlyphText::new(CELL);
    let la = layer(vec![cell(0, 'a', a.sub), cell(2, 'b', a.text)], a.bg);
    let mut out = Vec::new();
    g.write(&mut out, &la, None, &a.rgb).unwrap();
    let lb = layer(vec![cell(0, 'a', b.sub), cell(2, 'b', b.text)], b.bg);
    let mut out = Vec::new();
    g.write(&mut out, &lb, Some(&la), &b.rgb).unwrap();
    let s = text(out);
    assert!(s.contains("a=d,d=R,"));
    assert_eq!(
        s.matches("a=p,").count(),
        2,
        "toutes les lettres replacées : {s}"
    );
    // les identifiants repartent du début après la libération
    assert!(s.contains(&format!("i={FIRST_ID},")));
}

#[test]
fn a_restyle_with_the_same_image_writes_nothing() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let l = layer(vec![cell(0, 'a', p.text)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &l, None, &p.rgb).unwrap();
    // pendant un fondu : la couleur exacte change, l'image ramenée au thème non
    let faded = layer(vec![cell(0, 'a', Color::Rgb(0xd0, 0xcf, 0xc4))], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &faded, Some(&l), &p.rgb).unwrap();
    assert_eq!(text(out), "\x1b7\x1b[0m\x1b8", "rien à replacer");
}
