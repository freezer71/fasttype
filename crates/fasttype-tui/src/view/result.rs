//! Écran de résultat : vitesse et précision, puis le détail du test.

use crate::theme::Palette;
use crate::view::centered_segments;
use fasttype_core::result::{Invalid, TestResult};
use fasttype_core::spec::Mode;
use fasttype_store::RecordOutcome;
use fasttype_store::pbs::PbOutcome;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

pub struct ResultView<'a> {
    pub result: &'a TestResult,
    /// `None` si l'enregistrement a échoué.
    pub outcome: Option<RecordOutcome>,
    pub palette: &'a Palette,
    /// `typingSpeedUnit` : wpm, cpm, wps, cps ou wph.
    pub unit: &'a str,
    /// `alwaysShowDecimalPlaces`.
    pub decimals: bool,
}

/// Facteur de conversion depuis le wpm (`typing-speed-units.ts`).
pub fn unit_factor(unit: &str) -> f64 {
    match unit {
        "cpm" => 5.0,
        "wps" => 1.0 / 60.0,
        "cps" => 5.0 / 60.0,
        "wph" => 60.0,
        _ => 1.0,
    }
}

pub fn invalid_label(reason: Invalid) -> &'static str {
    match reason {
        Invalid::TooShort => "too short",
        Invalid::Afk => "afk detected",
        Invalid::Repeated => "repeated",
        Invalid::Wpm => "invalid wpm",
        Invalid::Raw => "invalid raw",
        Invalid::Accuracy => "invalid accuracy",
    }
}

fn quote_length_label(group: Option<u8>) -> &'static str {
    match group {
        Some(0) => "short",
        Some(1) => "medium",
        Some(2) => "long",
        Some(3) => "thicc",
        _ => "",
    }
}

/// « time 30 english punctuation numbers ».
pub fn test_type(r: &TestResult) -> String {
    let mode = match r.mode {
        Mode::Time => format!("time {}", r.mode2),
        Mode::Words => format!("words {}", r.mode2),
        Mode::Quote => format!("quote {}", quote_length_label(r.quote_length)),
        Mode::Zen => "zen".to_string(),
        Mode::Custom => "custom".to_string(),
    };
    let mut out = format!("{} {}", mode.trim_end(), r.language);
    if r.punctuation {
        out.push_str(" punctuation");
    }
    if r.numbers {
        out.push_str(" numbers");
    }
    out
}

impl ResultView<'_> {
    fn number(&self, v: f64) -> String {
        if self.decimals {
            format!("{v:.2}")
        } else {
            format!("{}", v.round())
        }
    }

    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        let p = self.palette;
        let r = self.result;
        let label = Style::default().fg(p.sub);
        let value = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
        let detail = Style::default().fg(p.text);
        let factor = unit_factor(self.unit);
        let acc = if self.decimals {
            format!("{:.2}%", r.acc)
        } else {
            format!("{}%", r.acc.floor())
        };
        let mid = area.y + area.height / 2;
        let y0 = mid.saturating_sub(3);

        centered_segments(
            buf,
            area,
            y0,
            &[
                (format!("{} ", self.unit), label),
                (self.number(r.wpm * factor), value),
                ("     acc ".to_string(), label),
                (acc, value),
            ],
        );
        if let Some(RecordOutcome::Saved(PbOutcome::NewBest { .. })) = self.outcome {
            centered_segments(
                buf,
                area,
                y0 + 1,
                &[("new personal best".to_string(), Style::default().fg(p.main))],
            );
        }
        let [c, i, e, m] = r.char_stats;
        centered_segments(
            buf,
            area,
            y0 + 3,
            &[
                ("raw ".to_string(), label),
                (self.number(r.raw * factor), detail),
                ("   characters ".to_string(), label),
                (format!("{c}/{i}/{e}/{m}"), detail),
                ("   consistency ".to_string(), label),
                (format!("{}%", r.consistency.round()), detail),
                ("   time ".to_string(), label),
                (format!("{}s", r.test_duration.round()), detail),
            ],
        );
        centered_segments(
            buf,
            area,
            y0 + 4,
            &[("test type ".to_string(), label), (test_type(r), detail)],
        );
        let mut other = Vec::new();
        if let Some(reason) = r.invalid {
            other.push(invalid_label(reason));
        }
        if r.bailed_out {
            other.push("bailed out");
        }
        if !other.is_empty() {
            centered_segments(
                buf,
                area,
                y0 + 5,
                &[
                    ("other ".to_string(), label),
                    (other.join(", "), Style::default().fg(p.error)),
                ],
            );
        }
        let tips = [
            ("tab".to_string(), Style::default().fg(p.sub_alt).bg(p.sub)),
            (" + ".to_string(), label),
            (
                "enter".to_string(),
                Style::default().fg(p.sub_alt).bg(p.sub),
            ),
            (" - next test".to_string(), label),
        ];
        centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
    }
}
