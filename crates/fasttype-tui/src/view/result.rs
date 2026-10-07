//! Écran de résultat : vitesse et précision, puis le détail du test.

use crate::theme::Palette;
use crate::view::big;
use crate::view::centered_segments;
use crate::view::chart::ChartView;
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
    /// `startGraphsAtZero`.
    pub start_graphs_at_zero: bool,
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

/// La couronne : nouveau record, sauf en quote (`result-pb.ts`).
pub fn shows_crown(r: &TestResult, outcome: Option<RecordOutcome>) -> bool {
    r.mode != Mode::Quote
        && matches!(
            outcome,
            Some(RecordOutcome::Saved(PbOutcome::NewBest { .. }))
        )
}

impl ResultView<'_> {
    fn number(&self, v: f64) -> String {
        if self.decimals {
            format!("{v:.2}")
        } else {
            format!("{}", v.round())
        }
    }

    /// Vitesse en grand : « Infinite » à partir de 1000 wpm (`ResultMainStats.tsx`).
    pub fn speed_text(&self) -> String {
        if self.result.wpm >= 1000.0 {
            "Infinite".to_string()
        } else {
            self.number(self.result.wpm * unit_factor(self.unit))
        }
    }

    /// Précision en grand : « 100% », sinon arrondie vers le bas.
    pub fn acc_text(&self) -> String {
        let acc = self.result.acc;
        if acc >= 100.0 {
            "100%".to_string()
        } else if self.decimals {
            format!("{acc:.2}%")
        } else {
            format!("{}%", acc.floor())
        }
    }

    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        let p = self.palette;
        let r = self.result;
        let label = Style::default().fg(p.sub);
        let value = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
        let detail = Style::default().fg(p.text);
        let factor = unit_factor(self.unit);
        // hauteur : grands chiffres (4), graphique, détails (3) et marges
        let chart_h = area.height.saturating_sub(16).min(12);
        let chart_h = if chart_h >= 5 { chart_h } else { 0 };
        let block = 4 + if chart_h > 0 { chart_h + 1 } else { 0 } + 4;
        let y0 = area.y + area.height.saturating_sub(block + 2) / 2;

        // vitesse et précision en grand, avec leur libellé au-dessus
        let speed = self.speed_text();
        let acc = self.acc_text();
        let w_of = |t: &str| {
            if big::supported(t) {
                big::width(t)
            } else {
                t.chars().count() as u16
            }
        };
        let gap = 6;
        let total = w_of(&speed) + gap + w_of(&acc);
        let x0 = area.x + area.width.saturating_sub(total) / 2;
        let x1 = x0 + w_of(&speed) + gap;
        buf.set_string(x0, y0, self.unit, label);
        if shows_crown(r, self.outcome) {
            buf.set_string(
                x0 + self.unit.len() as u16 + 1,
                y0,
                "♛",
                Style::default().fg(p.main),
            );
        }
        buf.set_string(x1, y0, "acc", label);
        for (x, text) in [(x0, &speed), (x1, &acc)] {
            if big::supported(text) {
                big::draw(buf, x, y0 + 1, text, value);
            } else {
                buf.set_string(x, y0 + 2, text, value);
            }
        }

        let mut y = y0 + 5;
        if chart_h > 0 {
            let width = area.width.saturating_sub(8).min(110);
            let chart_area = Rect {
                x: area.x + area.width.saturating_sub(width) / 2,
                y,
                width,
                height: chart_h,
            };
            ChartView {
                chart: &r.chart,
                palette: p,
                factor,
                unit: self.unit,
                start_at_zero: self.start_graphs_at_zero,
                duration: r.test_duration,
            }
            .render(buf, chart_area);
            y += chart_h + 1;
        }

        let [c, i, e, m] = r.char_stats;
        centered_segments(
            buf,
            area,
            y,
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
            y + 1,
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
                y + 2,
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
