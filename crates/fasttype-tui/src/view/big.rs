//! Grands chiffres sur trois lignes, en demi-blocs : les gros nombres du
//! résultat (wpm, acc) et le timer en style `text`.

use ratatui::buffer::Buffer;
use ratatui::style::Style;

/// Hauteur d'un grand caractère, en lignes.
pub const HEIGHT: u16 = 3;

fn glyph(c: char) -> Option<[&'static str; 3]> {
    Some(match c {
        '0' => ["█▀█", "█ █", "▀▀▀"],
        '1' => ["▀█ ", " █ ", "▀▀▀"],
        '2' => ["▀▀█", "█▀▀", "▀▀▀"],
        '3' => ["▀▀█", " ▀█", "▀▀▀"],
        '4' => ["█ █", "▀▀█", "  ▀"],
        '5' => ["█▀▀", "▀▀█", "▀▀▀"],
        '6' => ["█▀▀", "█▀█", "▀▀▀"],
        '7' => ["▀▀█", "  █", "  ▀"],
        '8' => ["█▀█", "█▀█", "▀▀▀"],
        '9' => ["█▀█", "▀▀█", "▀▀▀"],
        '%' => ["▀ █", " █ ", "█ ▄"],
        '.' => [" ", " ", "▀"],
        ':' => [" ", "▀", "▀"],
        '/' => ["  █", " █ ", "█  "],
        ' ' => [" ", " ", " "],
        _ => return None,
    })
}

/// Largeur en cases d'un texte écrit en grand (une case d'espace entre les caractères).
pub fn width(text: &str) -> u16 {
    let w: u16 = text
        .chars()
        .filter_map(glyph)
        .map(|g| g[0].chars().count() as u16 + 1)
        .sum();
    w.saturating_sub(1)
}

/// Peut-on écrire ce texte en grand ?
pub fn supported(text: &str) -> bool {
    text.chars().all(|c| glyph(c).is_some())
}

/// Écrit `text` en grand à partir de (`x`, `y`).
pub fn draw(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) {
    let area = buf.area;
    let mut cx = x;
    for g in text.chars().filter_map(glyph) {
        for (row, line) in g.iter().enumerate() {
            let ry = y + row as u16;
            if ry < area.bottom() && cx < area.right() {
                buf.set_string(cx, ry, line, style);
            }
        }
        cx += g[0].chars().count() as u16 + 1;
    }
}
