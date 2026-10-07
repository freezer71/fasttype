//! Stats en direct (`live-stats.ts`, `LiveStatsMini.tsx`, `BarTimerProgress.tsx`) :
//! timer, vitesse, précision et burst, en style `mini`, `text` ou `bar`.

use crate::view::big;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

/// `secondsToString` (utils/date-and-time.ts) : « 30 », « 01:15 », « 01:00:05 ».
pub fn seconds_to_string(sec: u64) -> String {
    let (h, m, s) = (sec / 3600, (sec % 3600) / 60, sec % 60);
    let mut out = String::new();
    if h > 0 {
        out.push_str(&format!("{h:02}:"));
    }
    if m > 0 || h > 0 {
        out.push_str(&format!("{m:02}:{s:02}"));
    } else {
        out.push_str(&s.to_string());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style3 {
    Off,
    Mini,
    Text,
}

impl Style3 {
    pub fn from_config(name: &str) -> Self {
        match name {
            "mini" | "flash_mini" => Style3::Mini,
            "text" | "flash_text" => Style3::Text,
            _ => Style3::Off,
        }
    }
}

/// Une valeur affichée et son style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveItem {
    pub text: String,
    pub style: Style3,
    /// Le timer : en style `text`, il va au-dessus des mots.
    pub is_timer: bool,
}

/// Les stats d'un instant, dans l'ordre du site : timer, vitesse, précision, burst.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveStats {
    pub items: Vec<LiveItem>,
}

/// Zone des mots : les stats `mini` vont juste au-dessus, les `text` en grand
/// au-dessus (timer) et au-dessous (le reste).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordsBox {
    pub left: u16,
    pub top: u16,
    /// Largeur en lettres (cases agrandies quand `scale > 1`).
    pub width: u16,
    pub lines: u16,
    /// Taille du texte des mots : chaque lettre occupe `scale × scale` cases.
    pub scale: u16,
}

impl WordsBox {
    /// Hauteur de la zone en lignes de l'écran.
    pub fn rows(&self) -> u16 {
        self.lines * self.scale
    }

    /// Zone occupée à l'écran.
    pub fn rect(&self) -> Rect {
        Rect {
            x: self.left,
            y: self.top,
            width: self.width * self.scale,
            height: self.rows(),
        }
    }
}

impl LiveStats {
    pub fn render(&self, buf: &mut Buffer, area: Rect, words: WordsBox, color: Color) {
        let style = Style::default().fg(color);
        // mini : une rangée juste au-dessus des mots, séparée d'une case
        let mini: Vec<&str> = self
            .items
            .iter()
            .filter(|i| i.style == Style3::Mini)
            .map(|i| i.text.as_str())
            .collect();
        if !mini.is_empty() && words.top > area.y {
            let line = mini.join(" ");
            buf.set_string(words.left, words.top - 1, &line, style);
        }
        // text : en grand, le timer au-dessus des mots, le reste au-dessous
        let big_items = self.items.iter().filter(|i| i.style == Style3::Text);
        let center = |text: &str| area.x + area.width.saturating_sub(big::width(text)) / 2;
        if let Some(timer) = big_items.clone().find(|i| i.is_timer) {
            let y = words.top.saturating_sub(big::HEIGHT + 2);
            if y > area.y && big::supported(&timer.text) {
                big::draw(buf, center(&timer.text), y, &timer.text, style);
            } else if words.top > area.y {
                buf.set_string(words.left, words.top - 1, &timer.text, style);
            }
        }
        let rest: Vec<&str> = big_items
            .filter(|i| !i.is_timer)
            .map(|i| i.text.as_str())
            .collect();
        if !rest.is_empty() {
            let line = rest.join("  ");
            let y = words.top + words.rows() + 1;
            if y + big::HEIGHT < area.bottom() && big::supported(&line) {
                big::draw(buf, center(&line), y, &line, style);
            } else if y < area.bottom() {
                let x = area.x + area.width.saturating_sub(line.width() as u16) / 2;
                buf.set_string(x, y, &line, style);
            }
        }
    }
}

/// Barre de progression en haut de l'écran (`timerStyle: bar`), sur une demi-ligne.
pub fn render_bar(buf: &mut Buffer, area: Rect, fraction: f64, color: Color) {
    let width = (fraction.clamp(0.0, 1.0) * f64::from(area.width)).round() as u16;
    let style = Style::default().fg(color);
    for x in area.x..area.x + width {
        buf[(x, area.y)].set_char('▀').set_style(style);
    }
}
