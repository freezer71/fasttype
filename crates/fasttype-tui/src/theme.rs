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

pub type Rgb = (u8, u8, u8);

/// Les 10 couleurs d'un thème en RGB opaque (les couleurs translucides sont
/// déjà posées sur le fond).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColors {
    pub bg: Rgb,
    pub main: Rgb,
    pub caret: Rgb,
    pub sub: Rgb,
    pub sub_alt: Rgb,
    pub text: Rgb,
    pub error: Rgb,
    pub error_extra: Rgb,
    pub colorful_error: Rgb,
    pub colorful_error_extra: Rgb,
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
    pub caret_rgb: Rgb,
    pub rgb: RgbColors,
    pub mode: ColorMode,
}

/// `a` vers `b` : `t = 0` donne `a`, `t = 1` donne `b`.
pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
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
        let c = |x: Rgba| {
            let o = x.over(bg);
            (o.r, o.g, o.b)
        };
        let rgb = RgbColors {
            bg: (bg.r, bg.g, bg.b),
            main: c(theme.main),
            caret: c(theme.caret),
            sub: c(theme.sub),
            sub_alt: c(theme.sub_alt),
            text: c(theme.text),
            error: c(theme.error),
            error_extra: c(theme.error_extra),
            colorful_error: c(theme.colorful_error),
            colorful_error_extra: c(theme.colorful_error_extra),
        };
        Self::from_rgb(rgb, mode)
    }

    pub fn from_rgb(rgb: RgbColors, mode: ColorMode) -> Self {
        let c = |x: Rgb| to_color(x, mode);
        Palette {
            bg: c(rgb.bg),
            main: c(rgb.main),
            caret: c(rgb.caret),
            sub: c(rgb.sub),
            sub_alt: c(rgb.sub_alt),
            text: c(rgb.text),
            error: c(rgb.error),
            error_extra: c(rgb.error_extra),
            colorful_error: c(rgb.colorful_error),
            colorful_error_extra: c(rgb.colorful_error_extra),
            caret_rgb: rgb.caret,
            rgb,
            mode,
        }
    }

    /// Couleur `x` vue à l'opacité `opacity` sur le fond du thème.
    pub fn over_bg(&self, x: Rgb, opacity: f64) -> Color {
        to_color(mix(self.rgb.bg, x, opacity), self.mode)
    }

    /// Palette à l'opacité `opacity` : chaque couleur est mêlée au fond, comme
    /// un élément HTML à `opacity < 1` sur le fond de la page.
    pub fn faded(&self, opacity: f64) -> Palette {
        if opacity >= 1.0 {
            return *self;
        }
        let r = self.rgb;
        let f = |x: Rgb| mix(r.bg, x, opacity);
        let faded = RgbColors {
            bg: r.bg,
            main: f(r.main),
            caret: f(r.caret),
            sub: f(r.sub),
            sub_alt: f(r.sub_alt),
            text: f(r.text),
            error: f(r.error),
            error_extra: f(r.error_extra),
            colorful_error: f(r.colorful_error),
            colorful_error_extra: f(r.colorful_error_extra),
        };
        Palette::from_rgb(faded, self.mode)
    }
}

pub fn to_color(c: Rgb, mode: ColorMode) -> Color {
    match mode {
        ColorMode::TrueColor => Color::Rgb(c.0, c.1, c.2),
        ColorMode::Ansi256 => Color::Indexed(nearest_256_cached(c)),
    }
}

thread_local! {
    static NEAREST: std::cell::RefCell<std::collections::HashMap<Rgb, u8>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// `nearest_256` avec un cache : les fondus recalculent les mêmes couleurs à chaque image.
fn nearest_256_cached(c: Rgb) -> u8 {
    NEAREST.with(|m| {
        *m.borrow_mut()
            .entry(c)
            .or_insert_with(|| nearest_256(c.0, c.1, c.2))
    })
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

/// RGB d'une couleur xterm : les 16 de base (valeurs par défaut d'xterm),
/// puis le cube 6×6×6 et les 24 gris.
pub fn xterm_rgb(index: u8) -> (u8, u8, u8) {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    if index < 16 {
        return BASE[usize::from(index)];
    }
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
