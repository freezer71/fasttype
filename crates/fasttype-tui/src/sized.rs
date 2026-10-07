//! Texte agrandi avec le protocole de taille du texte de Kitty (OSC 66,
//! Kitty ≥ 0.40) : les mots du test s'affichent à `fontSize` fois la taille
//! du reste de l'interface, comme sur le site (mots à 2rem, interface à 1rem).
//!
//! Chaque lettre agrandie occupe un bloc de `scale × scale` cases. La zone des
//! mots est marquée `skip` dans le tampon ratatui, qui n'y écrit donc jamais :
//! elle est effacée puis réécrite ici, d'un bloc, à chaque changement.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::io::{self, Write};

/// Sonde : position du curseur, un espace en double taille, position du
/// curseur. Un terminal qui gère OSC 66 avance de 2 cases au lieu d'une.
pub const PROBE: &[u8] = b"\r\x1b[6n\x1b]66;s=2; \x07\x1b[6n";

/// Lit la réponse à `PROBE` : `None` tant que les deux positions ne sont pas
/// arrivées, puis `Some(true)` si le curseur a avancé de 2 cases.
pub fn probe_answer(bytes: &[u8]) -> Option<bool> {
    let text = String::from_utf8_lossy(bytes);
    let mut cols = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find("\x1b[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('R') else {
            break;
        };
        if let Some((_, col)) = after[..end].split_once(';')
            && let Ok(c) = col.parse::<u32>()
        {
            cols.push(c);
        }
        rest = &after[end + 1..];
    }
    match cols[..] {
        [a, b, ..] => Some(b == a + 2),
        _ => None,
    }
}

/// Échelle du texte des mots : `fontSize` arrondi, de 1 à 4.
pub fn scale_for(font_size: f64) -> u16 {
    if font_size.is_finite() {
        font_size.round().clamp(1.0, 4.0) as u16
    } else {
        1
    }
}

/// Une lettre agrandie, coin haut-gauche (`x`, `y`) en cases de l'écran.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledCell {
    pub x: u16,
    pub y: u16,
    pub ch: char,
    pub style: Style,
}

/// La zone des mots agrandis : à effacer puis réécrire d'un bloc.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledText {
    pub scale: u16,
    pub region: Rect,
    /// Fond de la zone (couleur `bg` du thème).
    pub bg: Color,
    pub cells: Vec<ScaledCell>,
}

fn color(out: &mut Vec<u8>, base: u8, c: Color) {
    let _ = match c {
        Color::Rgb(r, g, b) => write!(out, ";{base};2;{r};{g};{b}"),
        Color::Indexed(i) => write!(out, ";{base};5;{i}"),
        _ => Ok(()),
    };
}

/// SGR complet d'un style (repart de zéro : `0`).
fn sgr(out: &mut Vec<u8>, style: &Style) {
    out.extend_from_slice(b"\x1b[0");
    if let Some(fg) = style.fg {
        color(out, 38, fg);
    }
    if let Some(bg) = style.bg {
        color(out, 48, bg);
    }
    if style.add_modifier.contains(Modifier::BOLD) {
        out.extend_from_slice(b";1");
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        out.extend_from_slice(b";4");
        if let Some(u) = style.underline_color {
            color(out, 58, u);
        }
    }
    out.push(b'm');
}

impl ScaledText {
    /// Efface la zone (écraser la case haut-gauche d'une lettre agrandie
    /// l'efface en entier) puis écrit chaque lettre.
    pub fn write(&self, out: &mut impl Write) -> io::Result<()> {
        let mut buf = Vec::with_capacity(64 * self.cells.len() + 8 * self.region.area() as usize);
        sgr(&mut buf, &Style::default().bg(self.bg));
        let blank = " ".repeat(usize::from(self.region.width));
        for y in self.region.top()..self.region.bottom() {
            let _ = write!(buf, "\x1b[{};{}H{blank}", y + 1, self.region.x + 1);
        }
        for c in &self.cells {
            let _ = write!(buf, "\x1b[{};{}H", c.y + 1, c.x + 1);
            sgr(&mut buf, &c.style);
            let _ = write!(buf, "\x1b]66;s={};{}\x07", self.scale, c.ch);
        }
        buf.extend_from_slice(b"\x1b[0m");
        out.write_all(&buf)
    }
}
