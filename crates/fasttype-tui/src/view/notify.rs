//! Notifications en pile en haut à droite (`states/notifications.ts`) : 3 s,
//! et les erreurs restent jusqu'à « Clear all notifications ». Pendant la
//! frappe, elles sont toutes cachées.

use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Notice,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub text: String,
    pub level: Level,
    pub until: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Notifications {
    items: Vec<Notification>,
}

impl Notifications {
    pub fn push(&mut self, text: impl Into<String>, level: Level, now: f64) {
        let ttl = match level {
            Level::Notice => 3000.0,
            Level::Error => f64::INFINITY,
        };
        self.items.insert(
            0,
            Notification {
                text: text.into(),
                level,
                until: now + ttl,
            },
        );
        self.items.truncate(5);
    }

    /// « Clear all notifications ».
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn expire(&mut self, now: f64) {
        self.items.retain(|n| n.until > now);
    }

    pub fn next_expiry(&self) -> Option<f64> {
        self.items
            .iter()
            .map(|n| n.until)
            .filter(|t| t.is_finite())
            .min_by(f64::total_cmp)
    }

    pub fn items(&self) -> &[Notification] {
        &self.items
    }

    /// Dessine la pile ; rien pendant la frappe (`Notifications.tsx` cache
    /// les notifications non importantes), pour ne jamais couvrir les mots.
    pub fn render(&self, buf: &mut Buffer, area: Rect, palette: &Palette, typing: bool) {
        if typing {
            return;
        }
        for (i, n) in self.items.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            if y >= area.bottom() {
                break;
            }
            let max = area.width.saturating_sub(4) as usize;
            let text: String = n.text.chars().take(max).collect();
            let w = text.width() as u16 + 2;
            let x = area.right().saturating_sub(w + 1);
            let fg = match n.level {
                Level::Notice => palette.text,
                Level::Error => palette.error,
            };
            let style = Style::default().fg(fg).bg(palette.sub_alt);
            buf.set_string(x, y, format!(" {text} "), style);
        }
    }
}
