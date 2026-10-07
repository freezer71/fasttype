//! Écran de test : lignes de mots visibles, en-tête et raccourcis.

use crate::layout::{Layout, char_width, extras, letters};
use crate::theme::Palette;
use crate::view::centered_segments;
use crate::view::live::WordsBox;
use fasttype_core::session::TestSession;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

/// Couleurs (juste, non tapé, faux, en trop) selon flipTestColors et colorfulMode.
fn letter_colors(p: &Palette, flip: bool, colorful: bool) -> (Color, Color, Color, Color) {
    let (correct, untyped) = match (flip, colorful) {
        (false, false) => (p.text, p.sub),
        (true, false) => (p.sub, p.text),
        (false, true) => (p.main, p.sub),
        (true, true) => (p.sub, p.main),
    };
    let (incorrect, extra) = if colorful {
        (p.colorful_error, p.colorful_error_extra)
    } else {
        (p.error, p.error_extra)
    };
    (correct, untyped, incorrect, extra)
}

/// Place de la zone de mots : centrée, `maxLineWidth` cases au plus (0 =
/// automatique, 100 cases au plus), 3 lignes (2 en zen).
pub fn words_box(area: Rect, max_line_width: u16, zen: bool) -> WordsBox {
    let width = if max_line_width >= 20 {
        max_line_width.min(area.width.saturating_sub(2))
    } else {
        area.width.saturating_sub(8).min(100)
    };
    let lines = if zen { 2 } else { 3 };
    WordsBox {
        left: area.x + area.width.saturating_sub(width) / 2,
        top: area.y + area.height.saturating_sub(lines) / 2,
        width,
        lines,
    }
}

pub struct WordsView<'a> {
    pub session: &'a TestSession,
    pub palette: &'a Palette,
    /// Lignes visibles, la première en haut de la zone.
    pub layout: &'a Layout,
    pub words: WordsBox,
    pub flip_test_colors: bool,
    pub colorful_mode: bool,
    pub zen: bool,
    /// Palette de la dernière ligne pendant son apparition (`smoothLineScroll`).
    pub last_line: Option<Palette>,
}

impl WordsView<'_> {
    pub fn render(&self, buf: &mut Buffer) {
        let shown = usize::from(self.words.lines);
        for (row, line) in self.layout.lines.iter().take(shown).enumerate() {
            let y = self.words.top + row as u16;
            let palette = match self.last_line {
                Some(ref p) if row + 1 == shown => p,
                _ => self.palette,
            };
            for b in line {
                self.draw_word(buf, palette, self.words.left + b.x, y, b.index);
            }
        }
    }

    fn draw_word(&self, buf: &mut Buffer, p: &Palette, x0: u16, y: u16, index: usize) {
        let s = self.session;
        let (correct, untyped, incorrect, extra) =
            letter_colors(p, self.flip_test_colors, self.colorful_mode);
        let target = s.word(index);
        let input = s.input(index);
        let wrong_committed = !self.zen && s.is_committed(index) && input != target;
        let base = |fg: Color| {
            let st = Style::default().fg(fg).bg(p.bg);
            if wrong_committed {
                st.add_modifier(Modifier::UNDERLINED)
                    .underline_color(p.error)
            } else {
                st
            }
        };
        let right = buf.area.right();
        let mut x = x0;
        let mut put = |c: char, style: Style| {
            if x < right {
                buf[(x, y)].set_char(c).set_style(style);
            }
            x += char_width(c);
        };
        if self.zen {
            for c in letters(input).chars() {
                put(c, base(correct));
            }
            return;
        }
        let mut typed = letters(input).chars();
        for tc in letters(target).chars() {
            let style = match typed.next() {
                None => base(untyped),
                Some(ic) if ic == tc => base(correct),
                Some(_) => base(incorrect),
            };
            put(tc, style);
        }
        for c in extras(target, input).chars() {
            put(c, base(extra));
        }
    }
}

/// En-tête (logo et résumé de la config) et raccourcis du bas, avec leurs
/// opacités du focus mode.
pub struct Chrome<'a> {
    pub palette: &'a Palette,
    /// Couleur du logo : `main`, qui passe à `sub` en focus mode.
    pub logo: Color,
    pub summary: &'a str,
    /// Opacité du résumé de la config et des raccourcis.
    pub opacity: f64,
    pub tips: &'a str,
}

impl Chrome<'_> {
    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        buf.set_string(
            area.x + 2,
            area.y + 1,
            "fasttype",
            Style::default().fg(self.logo).add_modifier(Modifier::BOLD),
        );
        if self.opacity <= 0.0 {
            return;
        }
        let p = self.palette.faded(self.opacity);
        let summary_x = area
            .right()
            .saturating_sub(self.summary.chars().count() as u16 + 2);
        buf.set_string(
            summary_x,
            area.y + 1,
            self.summary,
            Style::default().fg(p.sub),
        );
        let key = Style::default().fg(p.sub_alt).bg(p.sub);
        let text = Style::default().fg(p.sub);
        let tips = [
            ("tab".to_string(), key),
            (" + ".to_string(), text),
            ("enter".to_string(), key),
            (format!(" - {}", self.tips), text),
        ];
        centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
    }
}
