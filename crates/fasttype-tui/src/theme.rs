//! Couleurs d'un thème Monkeytype pour le terminal : truecolor si le terminal
//! l'annonce, sinon la couleur la plus proche des 256 couleurs xterm (OKLab).

use fasttype_data::themes::{Rgba, Theme};
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
}

impl ColorMode {
    /// `COLORTERM=truecolor` ou `24bit` : truecolor ; sinon 256 couleurs.
    pub fn detect(get: impl Fn(&str) -> Option<String>) -> Self {
        match get("COLORTERM").as_deref() {
            Some("truecolor") | Some("24bit") => ColorMode::TrueColor,
            _ => ColorMode::Ansi256,
        }
    }
}

/// Les 10 couleurs d'un thème, prêtes pour ratatui.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub main: Color,
    pub caret: Color,
    pub sub: Color,
    pub sub_alt: Color,
    pub text: Color,
    pub error: Color,
    pub error_extra: Color,
    pub colorful_error: Color,
    pub colorful_error_extra: Color,
    /// Couleur du caret en RGB, pour la séquence OSC 12 du curseur.
    pub caret_rgb: (u8, u8, u8),
}

impl Palette {
    pub fn from_theme(theme: &Theme, mode: ColorMode) -> Self {
        let black = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        let bg = theme.bg.over(black);
        let c = |x: Rgba| to_color(x.over(bg), mode);
        let caret = theme.caret.over(bg);
        Palette {
            bg: to_color(bg, mode),
            main: c(theme.main),
            caret: c(theme.caret),
            sub: c(theme.sub),
            sub_alt: c(theme.sub_alt),
            text: c(theme.text),
            error: c(theme.error),
            error_extra: c(theme.error_extra),
            colorful_error: c(theme.colorful_error),
            colorful_error_extra: c(theme.colorful_error_extra),
            caret_rgb: (caret.r, caret.g, caret.b),
        }
    }
}

fn to_color(c: Rgba, mode: ColorMode) -> Color {
    match mode {
        ColorMode::TrueColor => Color::Rgb(c.r, c.g, c.b),
        ColorMode::Ansi256 => Color::Indexed(nearest_256(c.r, c.g, c.b)),
    }
}

fn oklab(r: u8, g: u8, b: u8) -> [f64; 3] {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(r), lin(g), lin(b));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    ]
}

/// RGB d'une couleur xterm 16..=255 (cube 6×6×6 puis 24 gris).
fn xterm_rgb(index: u8) -> (u8, u8, u8) {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    if index >= 232 {
        let v = 8 + 10 * (index - 232);
        return (v, v, v);
    }
    let i = index - 16;
    (
        LEVELS[usize::from(i / 36)],
        LEVELS[usize::from((i / 6) % 6)],
        LEVELS[usize::from(i % 6)],
    )
}

/// Couleur xterm (16..=255) la plus proche en distance OKLab.
pub fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    let target = oklab(r, g, b);
    let dist = |i: u8| {
        let (cr, cg, cb) = xterm_rgb(i);
        let c = oklab(cr, cg, cb);
        (0..3).map(|k| (c[k] - target[k]).powi(2)).sum::<f64>()
    };
    (16..=255u8)
        .min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
        .unwrap_or(16)
}
