//! Rendu des écrans dans un `Buffer` ratatui (sans E/S : testable hors terminal).

pub mod big;
pub mod chart;
pub mod config_bar;
pub mod live;
pub mod notify;
pub mod result;
pub mod test;

use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

/// Taille minimale utilisable (spec §8).
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 10;

/// Peint tout l'écran avec le fond du thème.
pub fn fill_background(buf: &mut Buffer, area: Rect, palette: &Palette) {
    buf.set_style(area, Style::default().bg(palette.bg).fg(palette.sub));
}

/// Écrit des segments stylés, centrés sur la ligne `y`.
pub fn centered_segments(buf: &mut Buffer, area: Rect, y: u16, segments: &[(String, Style)]) {
    let width: usize = segments.iter().map(|(s, _)| s.width()).sum();
    let mut x = area.x + area.width.saturating_sub(width as u16) / 2;
    for (text, style) in segments {
        buf.set_string(x, y, text, *style);
        x += text.width() as u16;
    }
}

pub fn too_small(buf: &mut Buffer, area: Rect, palette: &Palette) {
    let y = area.y + area.height / 2;
    let msg = format!("terminal too small ({}×{} min)", MIN_WIDTH, MIN_HEIGHT);
    centered_segments(buf, area, y, &[(msg, Style::default().fg(palette.text))]);
}
