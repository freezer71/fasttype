//! Écran de test : en-tête, timer, trois lignes de mots, raccourcis.

use crate::layout::{Layout, char_width, extras, layout_words, letters};
use crate::theme::Palette;
use crate::view::centered_segments;
use fasttype_core::session::{SessionState, TestSession};
use fasttype_core::spec::Mode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

pub struct TestView<'a> {
    pub session: &'a TestSession,
    pub palette: &'a Palette,
    pub flip_test_colors: bool,
    pub colorful_mode: bool,
    /// `maxLineWidth` : 0 = automatique.
    pub max_line_width: u16,
    /// Résumé de la config affiché dans l'en-tête (« time 30 · english »).
    pub summary: String,
    /// Timer ou progression (style mini), affiché pendant le test.
    pub timer: Option<String>,
}

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

impl TestView<'_> {
    /// Largeur de la zone de mots.
    pub fn text_width(&self, area: Rect) -> u16 {
        let auto = area.width.saturating_sub(8).min(100);
        if self.max_line_width >= 20 {
            self.max_line_width.min(area.width.saturating_sub(2))
        } else {
            auto
        }
    }

    /// Dessine l'écran et renvoie la position du caret à l'écran.
    pub fn render(&self, buf: &mut Buffer, area: Rect) -> Option<(u16, u16)> {
        let s = self.session;
        let running = s.state() == SessionState::Running;
        let zen = s.spec().mode == Mode::Zen;
        let width = self.text_width(area);
        let left = area.x + area.width.saturating_sub(width) / 2;
        let shown = if zen { 2 } else { 3 };
        let top = area.y + area.height.saturating_sub(shown) / 2;

        if !running {
            buf.set_string(
                area.x + 2,
                area.y + 1,
                "fasttype",
                Style::default()
                    .fg(self.palette.main)
                    .add_modifier(Modifier::BOLD),
            );
            let summary_x = area
                .right()
                .saturating_sub(self.summary.chars().count() as u16 + 2);
            buf.set_string(
                summary_x,
                area.y + 1,
                &self.summary,
                Style::default().fg(self.palette.sub),
            );
            let tips = [
                (
                    "tab".to_string(),
                    Style::default()
                        .fg(self.palette.sub_alt)
                        .bg(self.palette.sub),
                ),
                (" + ".to_string(), Style::default().fg(self.palette.sub)),
                (
                    "enter".to_string(),
                    Style::default()
                        .fg(self.palette.sub_alt)
                        .bg(self.palette.sub),
                ),
                (
                    " - restart".to_string(),
                    Style::default().fg(self.palette.sub),
                ),
            ];
            centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
        }
        if running && let Some(timer) = &self.timer {
            buf.set_string(
                left,
                top.saturating_sub(1),
                timer,
                Style::default().fg(self.palette.main),
            );
        }

        let layout = layout_words(s.words(), s.inputs(), s.active_index(), width);
        let first = layout.first_visible();
        for (row, line) in layout
            .lines
            .iter()
            .enumerate()
            .skip(first)
            .take(usize::from(shown))
        {
            let y = top + (row - first) as u16;
            for b in line {
                self.draw_word(buf, left + b.x, y, b.index, zen);
            }
        }
        self.caret(&layout, left, top, first, shown)
    }

    fn caret(
        &self,
        layout: &Layout,
        left: u16,
        top: u16,
        first: usize,
        shown: u16,
    ) -> Option<(u16, u16)> {
        let s = self.session;
        if s.state() == SessionState::Finished || s.words().is_empty() {
            return None;
        }
        let (line, x) = layout.caret;
        (line >= first && line < first + usize::from(shown))
            .then(|| (left + x, top + (line - first) as u16))
    }

    fn draw_word(&self, buf: &mut Buffer, x0: u16, y: u16, index: usize, zen: bool) {
        let s = self.session;
        let (correct, untyped, incorrect, extra) =
            letter_colors(self.palette, self.flip_test_colors, self.colorful_mode);
        let target = s.word(index);
        let input = s.input(index);
        let typed: Vec<char> = letters(input).chars().collect();
        let wrong_committed = !zen && s.is_committed(index) && input != target;
        let base = |fg: Color| {
            let st = Style::default().fg(fg).bg(self.palette.bg);
            if wrong_committed {
                st.add_modifier(Modifier::UNDERLINED)
                    .underline_color(self.palette.error)
            } else {
                st
            }
        };
        let mut x = x0;
        let mut put = |c: char, style: Style| {
            buf.set_string(x, y, c.to_string(), style);
            x += char_width(c);
        };
        if zen {
            for &c in &typed {
                put(c, base(correct));
            }
            return;
        }
        for (k, tc) in letters(target).chars().enumerate() {
            let style = match typed.get(k) {
                None => base(untyped),
                Some(&ic) if ic == tc => base(correct),
                Some(_) => base(incorrect),
            };
            put(tc, style);
        }
        for c in extras(target, input).chars() {
            put(c, base(extra));
        }
    }
}
