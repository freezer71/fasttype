//! Palette de commandes à l'écran (`CommandlineModal.tsx`) : fond assombri,
//! boîte en haut au centre, saisie puis liste ; la ligne active est inversée
//! (`bg-text text-bg`), les autres en `sub`, une coche devant la valeur en cours.

use crate::palette::state::PaletteState;
use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Lignes de liste visibles au plus.
pub const MAX_ROWS: u16 = 12;

/// Couleur vue derrière le voile noir à 50 % de la palette (truecolor).
pub fn dim_color(c: Color) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
        other => other,
    }
}

pub fn dim_style(s: Style) -> Style {
    Style {
        fg: s.fg.map(dim_color),
        bg: s.bg.map(dim_color),
        underline_color: s.underline_color.map(dim_color),
        ..s
    }
}

/// Assombrit l'écran derrière la palette.
pub fn dim(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            let (fg, bg) = (dim_color(cell.fg), dim_color(cell.bg));
            cell.set_fg(fg).set_bg(bg);
        }
    }
}

/// Place de la boîte : en haut au centre, à la hauteur de son contenu.
pub fn palette_rect(area: Rect, state: &PaletteState) -> Rect {
    let width = area.width.saturating_sub(4).min(72);
    let error = state.input().is_some_and(|m| m.error.is_some());
    let rows = if state.input().is_some() {
        u16::from(error)
    } else {
        (state.shown().0.len() as u16).min(MAX_ROWS)
    };
    let top = area.y + (area.height / 8).max(1);
    let height = (2 + rows + 1).min(area.bottom().saturating_sub(top));
    Rect {
        x: area.x + (area.width - width) / 2,
        y: top,
        width,
        height,
    }
}

/// Coupe un texte à `width` cases.
fn fit(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        used += w;
        out.push(c);
    }
    out
}

/// Dessine la palette ; renvoie la position du curseur de saisie.
pub fn render(buf: &mut Buffer, area: Rect, state: &PaletteState, p: &Palette) -> (u16, u16) {
    dim(buf, area);
    let r = palette_rect(area, state);
    let back = Style::default().bg(p.bg);
    for y in r.top()..r.bottom() {
        buf.set_string(r.x, y, " ".repeat(usize::from(r.width)), back);
    }
    let inner = usize::from(r.width.saturating_sub(4));
    // saisie : la recherche, ou la valeur libre en cours
    let (placeholder, text) = match state.input() {
        Some(m) => (m.title.as_str(), m.text.as_str()),
        None => (state.title(), state.query()),
    };
    let y = r.y + 1;
    buf.set_string(r.x + 2, y, "›", back.fg(p.sub));
    let shown = if text.is_empty() {
        buf.set_string(
            r.x + 4,
            y,
            fit(placeholder, inner.saturating_sub(2)),
            back.fg(p.sub),
        );
        String::new()
    } else {
        // la fin de la saisie reste visible
        let room = inner.saturating_sub(3);
        let skip = text.width().saturating_sub(room);
        let visible: String = text.chars().skip(skip).collect();
        buf.set_string(r.x + 4, y, &visible, back.fg(p.text));
        visible
    };
    let cursor = (r.x + 4 + shown.width() as u16, y);
    if let Some(m) = state.input() {
        if let Some(e) = &m.error {
            buf.set_string(
                r.x + 2,
                y + 1,
                fit(&format!("⚠ {e}"), inner),
                back.fg(p.error),
            );
        }
        return cursor;
    }
    let (list, active) = state.shown();
    let rows = usize::from(r.height.saturating_sub(3));
    let start = active
        .saturating_sub(rows / 2)
        .min(list.len().saturating_sub(rows));
    for (k, cmd) in list.iter().enumerate().skip(start).take(rows) {
        let y = r.y + 2 + (k - start) as u16;
        let style = if k == active {
            Style::default().bg(p.text).fg(p.bg)
        } else {
            back.fg(p.sub)
        };
        let check = if cmd.checkable && cmd.active {
            "✓"
        } else {
            " "
        };
        let line = fit(&format!(" {check} {}", cmd.display), inner + 2);
        let pad = (inner + 2).saturating_sub(line.width());
        buf.set_string(r.x + 1, y, format!("{line}{}", " ".repeat(pad)), style);
    }
    cursor
}
