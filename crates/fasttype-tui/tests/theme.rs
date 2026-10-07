use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_tui::theme::{ColorMode, Palette, nearest_256};
use ratatui::style::Color;

#[test]
fn detects_truecolor() {
    assert_eq!(
        ColorMode::detect(|k| (k == "COLORTERM").then(|| "truecolor".into())),
        ColorMode::TrueColor
    );
    assert_eq!(
        ColorMode::detect(|k| (k == "COLORTERM").then(|| "24bit".into())),
        ColorMode::TrueColor
    );
    assert_eq!(ColorMode::detect(|_| None), ColorMode::Ansi256);
}

#[test]
fn nearest_xterm_colors() {
    assert_eq!(nearest_256(0, 0, 0), 16);
    assert_eq!(nearest_256(255, 255, 255), 231);
    assert_eq!(nearest_256(255, 0, 0), 196);
    assert_eq!(nearest_256(128, 128, 128), 244);
}

#[test]
fn serika_dark_palette() {
    let t = theme(DEFAULT_THEME).unwrap();
    let p = Palette::from_theme(t, ColorMode::TrueColor);
    assert_eq!(p.bg, Color::Rgb(0x32, 0x34, 0x37));
    assert_eq!(p.main, Color::Rgb(0xe2, 0xb7, 0x14));
    assert_eq!(p.caret_rgb, (0xe2, 0xb7, 0x14));
    let p256 = Palette::from_theme(t, ColorMode::Ansi256);
    assert!(matches!(p256.main, Color::Indexed(_)));
}

#[test]
fn translucent_colors_are_blended_on_background() {
    // slambook : sub = #1c82adc4, seule couleur translucide des 187 thèmes
    let t = theme("slambook").unwrap();
    assert!(t.sub.a < 255);
    let p = Palette::from_theme(t, ColorMode::TrueColor);
    let blended = t.sub.over(t.bg);
    assert_eq!(p.sub, Color::Rgb(blended.r, blended.g, blended.b));
}

#[test]
fn faded_palette_blends_toward_background() {
    let t = theme(DEFAULT_THEME).unwrap();
    let p = Palette::from_theme(t, ColorMode::TrueColor);
    assert_eq!(p.faded(1.0), p);
    let zero = p.faded(0.0);
    assert_eq!(zero.main, p.bg);
    assert_eq!(zero.text, p.bg);
    let half = p.faded(0.5);
    // main #e2b714 sur bg #323437 à 50 %
    assert_eq!(half.main, Color::Rgb(0x8a, 0x76, 0x26));
    let p256 = Palette::from_theme(t, ColorMode::Ansi256);
    assert_eq!(p256.faded(0.0).main, p256.bg);
}
