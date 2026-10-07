//! Écran de test : lignes de mots visibles, en-tête et raccourcis.

use crate::layout::{Layout, char_width, extras, letters};
use crate::sized::{ScaledCell, ScaledText};
use crate::theme::Palette;
use crate::view::centered_segments;
use crate::view::config_bar::{bar_groups, bar_width, render_bar};
use crate::view::live::WordsBox;
use fasttype_core::session::TestSession;
use fasttype_store::Config;
use ratatui::buffer::{Buffer, CellDiffOption};
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

/// Place de la zone de mots : centrée, 3 lignes (2 en zen) de lettres à
/// l'échelle `scale`. Largeur : `maxLineWidth` lettres (0 = automatique :
/// 100 cases de l'écran au plus). L'échelle baisse si la zone ne tient pas.
pub fn words_box(area: Rect, max_line_width: u16, zen: bool, scale: u16) -> WordsBox {
    let lines = if zen { 2 } else { 3 };
    let mut scale = scale.max(1);
    // la place des mots, plus l'en-tête, les stats et les raccourcis
    while scale > 1
        && (area.width.saturating_sub(8) / scale < 20 || lines * scale + 8 > area.height)
    {
        scale -= 1;
    }
    let screen = if max_line_width >= 20 {
        max_line_width
            .saturating_mul(scale)
            .min(area.width.saturating_sub(2))
    } else {
        area.width.saturating_sub(8).min(100)
    };
    let width = screen / scale;
    WordsBox {
        left: area.x + area.width.saturating_sub(width * scale) / 2,
        top: area.y + area.height.saturating_sub(lines * scale) / 2,
        width,
        lines,
        scale,
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
    /// Dessine les mots. À l'échelle 1, dans le tampon ; au-delà, la zone est
    /// réservée (`skip`) et les lettres sont renvoyées pour l'écriture OSC 66.
    pub fn render(&self, buf: &mut Buffer) -> Option<ScaledText> {
        let w = self.words;
        let shown = usize::from(w.lines);
        let mut letters = Vec::new();
        for (row, line) in self.layout.lines.iter().take(shown).enumerate() {
            let palette = match self.last_line {
                Some(ref p) if row + 1 == shown => p,
                _ => self.palette,
            };
            for b in line {
                self.word_letters(palette, b.x, row as u16, b.index, &mut letters);
            }
        }
        let area = buf.area;
        if w.scale <= 1 {
            for (col, row, ch, style) in letters {
                let (x, y) = (w.left + col, w.top + row);
                if x < area.right() && y < area.bottom() {
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
            return None;
        }
        let region = w.rect().intersection(area);
        for y in region.top()..region.bottom() {
            for x in region.left()..region.right() {
                buf[(x, y)].set_diff_option(CellDiffOption::Skip);
            }
        }
        let cells = letters
            .into_iter()
            .map(|(col, row, ch, style)| ScaledCell {
                x: w.left + col * w.scale,
                y: w.top + row * w.scale,
                ch,
                style,
            })
            .filter(|c| c.x + w.scale <= region.right() && c.y + w.scale <= region.bottom())
            .collect();
        Some(ScaledText {
            scale: w.scale,
            region,
            bg: self.palette.bg,
            cells,
        })
    }

    /// Lettres d'un mot : (colonne, ligne, caractère, style), en lettres.
    fn word_letters(
        &self,
        p: &Palette,
        x0: u16,
        row: u16,
        index: usize,
        out: &mut Vec<(u16, u16, char, Style)>,
    ) {
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
        let mut x = x0;
        let mut put = |c: char, style: Style| {
            out.push((x, row, c, style));
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

/// En-tête (logo et barre de config) et raccourcis du bas, avec leurs
/// opacités du focus mode.
pub struct Chrome<'a> {
    pub palette: &'a Palette,
    /// Couleur du logo : `main`, qui passe à `sub` en focus mode.
    pub logo: Color,
    pub config: &'a Config,
    /// Opacité de la barre de config et des raccourcis.
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
        // la barre à côté du logo si elle tient, sinon en dessous, compacte au besoin
        let full = bar_groups(self.config, false);
        let bar = if bar_width(&full) <= area.width {
            full
        } else {
            bar_groups(self.config, true)
        };
        let y = if bar_width(&bar) + 24 <= area.width {
            area.y + 1
        } else {
            area.y + 3
        };
        render_bar(buf, area, y, &bar, &p);
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
