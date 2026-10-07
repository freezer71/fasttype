//! Graphique du résultat en braille (`ResultChart.tsx`) : wpm (main), raw (main
//! à 60 %, pointillés), burst (sub, rempli en subAlt à 50 %) sur l'axe de
//! gauche, erreurs (croix `error`) sur l'axe de droite. Courbes lissées comme
//! Chart.js (`tension: 0.5`) par une spline de Catmull-Rom.

use crate::theme::Palette;
use fasttype_core::result::ChartData;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// `smoothWithValueWindow` (utils/arrays.ts) : moyenne des voisins à ±`window`
/// positions dont la valeur ne s'écarte pas de plus de `value_window`.
pub fn smooth_with_value_window(values: &[f64], window: usize, value_window: f64) -> Vec<f64> {
    (0..values.len())
        .map(|i| {
            let current = values[i];
            let from = i.saturating_sub(window);
            let to = (i + window + 1).min(values.len());
            let near: Vec<f64> = values[from..to]
                .iter()
                .copied()
                .filter(|v| (v - current).abs() <= value_window)
                .collect();
            if near.is_empty() {
                current
            } else {
                near.iter().sum::<f64>() / near.len() as f64
            }
        })
        .collect()
}

/// Axe de gauche (`getMinMax`) : max arrondi à la dizaine supérieure ; min à 0
/// si `startGraphsAtZero`, sinon arrondi à la dizaine inférieure.
pub fn y_range(series: &[&[f64]], start_at_zero: bool) -> (f64, f64) {
    let all = series.iter().flat_map(|s| s.iter().copied());
    let max = all.clone().fold(0.0, f64::max);
    let min = all.fold(f64::INFINITY, f64::min);
    let max = ((max / 10.0).ceil() * 10.0).max(10.0);
    let min = if start_at_zero || !min.is_finite() {
        0.0
    } else {
        (min / 10.0).floor() * 10.0
    };
    (min, if max > min { max } else { min + 10.0 })
}

/// Valeur interpolée à l'abscisse `t` (en indices de points), spline de Catmull-Rom.
pub fn sample(values: &[f64], t: f64) -> f64 {
    match values.len() {
        0 => 0.0,
        1 => values[0],
        n => {
            let t = t.clamp(0.0, (n - 1) as f64);
            let i = (t.floor() as usize).min(n - 2);
            let u = t - i as f64;
            let p = |k: isize| values[(i as isize + k).clamp(0, n as isize - 1) as usize];
            let (p0, p1, p2, p3) = (p(-1), p(0), p(1), p(2));
            0.5 * (2.0 * p1
                + (-p0 + p2) * u
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u * u * u)
        }
    }
}

/// Grille de points braille : 2 × 4 points par case, une couleur par case.
struct Dots {
    w: usize,
    h: usize,
    bits: Vec<u8>,
    /// Série propriétaire de la case (priorité : plus petit = devant).
    owner: Vec<u8>,
}

const DOT: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

impl Dots {
    fn new(w: usize, h: usize) -> Self {
        Dots {
            w,
            h,
            bits: vec![0; w * h],
            owner: vec![u8::MAX; w * h],
        }
    }

    fn set(&mut self, px: usize, py: usize, series: u8) {
        let (cx, cy) = (px / 2, py / 4);
        if cx >= self.w || cy >= self.h {
            return;
        }
        let k = cy * self.w + cx;
        self.bits[k] |= DOT[px % 2][py % 4];
        self.owner[k] = self.owner[k].min(series);
    }
}

pub struct ChartView<'a> {
    pub chart: &'a ChartData,
    pub palette: &'a Palette,
    /// Facteur d'unité (`typingSpeedUnit`) appliqué aux séries de vitesse.
    pub factor: f64,
    pub unit: &'a str,
    pub start_at_zero: bool,
    /// Durée du test (s) : libellé de la dernière seconde fractionnaire.
    pub duration: f64,
}

/// Séries de la priorité d'affichage (Chart.js `order`).
const WPM: u8 = 0;
const RAW: u8 = 1;
const BURST: u8 = 2;

impl ChartView<'_> {
    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        let p = self.palette;
        let speed = |v: &[f64]| -> Vec<f64> { v.iter().map(|x| x * self.factor).collect() };
        let wpm = speed(&self.chart.wpm);
        let raw = speed(&self.chart.raw);
        let burst_raw = speed(&self.chart.burst);
        let burst_max = burst_raw.iter().copied().fold(0.0, f64::max);
        let burst = smooth_with_value_window(&burst_raw, 1, burst_max * 0.25);
        let n = wpm.len();
        if n == 0 || area.width < 12 || area.height < 4 {
            return;
        }
        let (min, max) = y_range(&[&wpm, &raw, &burst], self.start_at_zero);
        let err_max = self.chart.err.iter().copied().max().unwrap_or(0);
        let left_labels = [
            format!("{max}"),
            format!("{}", (min + max) / 2.0),
            format!("{min}"),
        ];
        let lw = left_labels.iter().map(String::len).max().unwrap_or(1) as u16 + 1;
        let rw = err_max.to_string().len() as u16 + 1;
        let plot = Rect {
            x: area.x + lw,
            y: area.y + 1,
            width: area.width.saturating_sub(lw + rw),
            height: area.height.saturating_sub(2),
        };
        if plot.width < 4 || plot.height < 2 {
            return;
        }
        let (w, h) = (usize::from(plot.width), usize::from(plot.height));
        let (pw, ph) = (w * 2, h * 4);
        let to_py = |v: f64| {
            let k = ((v - min) / (max - min)).clamp(0.0, 1.0);
            ((1.0 - k) * (ph - 1) as f64).round() as usize
        };
        let at = |px: usize| px as f64 / (pw - 1).max(1) as f64 * (n - 1) as f64;
        let mut dots = Dots::new(w, h);
        // burst : remplissage sous la courbe en subAlt à 50 %
        let fill = p.over_bg(p.rgb.sub_alt, 0.5);
        for cx in 0..w {
            let py = to_py(sample(&burst, at(cx * 2 + 1)));
            for cy in (py / 4 + 1)..h {
                buf[(plot.x + cx as u16, plot.y + cy as u16)].set_bg(fill);
            }
        }
        let mut line = |values: &[f64], series: u8, dashed: bool| {
            if values.len() != n {
                return;
            }
            let mut prev: Option<usize> = None;
            for px in 0..pw {
                let py = to_py(sample(values, at(px)));
                let on = !dashed || (px / 4) % 2 == 0;
                if on {
                    let (a, b) = match prev {
                        Some(q) => (q.min(py), q.max(py)),
                        None => (py, py),
                    };
                    // relie verticalement au point précédent : pas de trou
                    for y in a..=b {
                        dots.set(px, y, series);
                    }
                }
                prev = Some(py);
            }
        };
        line(&burst, BURST, false);
        line(&raw, RAW, true);
        line(&wpm, WPM, false);
        let color = |series: u8| -> Color {
            match series {
                WPM => p.main,
                RAW => p.over_bg(p.rgb.main, 0.6),
                _ => p.sub,
            }
        };
        for cy in 0..h {
            for cx in 0..w {
                let k = cy * w + cx;
                if dots.bits[k] == 0 {
                    continue;
                }
                let ch = char::from_u32(0x2800 + u32::from(dots.bits[k])).unwrap_or(' ');
                let cell = &mut buf[(plot.x + cx as u16, plot.y + cy as u16)];
                cell.set_char(ch).set_fg(color(dots.owner[k]));
            }
        }
        // erreurs : croix sur l'axe de droite (0..max), rien quand il n'y en a pas
        if err_max > 0 {
            for (i, &e) in self.chart.err.iter().enumerate() {
                if e == 0 {
                    continue;
                }
                let cx = if n > 1 {
                    (i as f64 / (n - 1) as f64 * (w - 1) as f64).round() as u16
                } else {
                    0
                };
                let k = f64::from(e) / f64::from(err_max);
                let cy = ((1.0 - k) * (h - 1) as f64).round() as u16;
                buf[(plot.x + cx, plot.y + cy)]
                    .set_char('×')
                    .set_fg(p.error);
            }
        }
        // axes
        let label = Style::default().fg(p.sub);
        buf.set_string(area.x, area.y, self.unit, label);
        for (k, text) in left_labels.iter().enumerate() {
            let y = match k {
                0 => plot.y,
                1 => plot.y + plot.height / 2,
                _ => plot.bottom() - 1,
            };
            buf.set_string(area.x + lw - 1 - text.len() as u16, y, text, label);
        }
        if err_max > 0 {
            let right = plot.right() + 1;
            buf.set_string(right, plot.y, err_max.to_string(), label);
            buf.set_string(right, plot.bottom() - 1, "0", label);
            let title = "errors";
            if area.right() >= title.len() as u16 {
                buf.set_string(area.right() - title.len() as u16, area.y, title, label);
            }
        }
        // abscisses : les secondes, espacées pour ne pas se chevaucher
        let fractional = n as f64 > self.duration.floor() && self.duration.fract() > 0.0;
        let mut next_free = plot.x;
        for i in 0..n {
            let text = if fractional && i == n - 1 {
                format!("{:.2}", self.duration)
            } else {
                (i + 1).to_string()
            };
            let cx = if n > 1 {
                (i as f64 / (n - 1) as f64 * (w - 1) as f64).round() as u16
            } else {
                0
            };
            let x = (plot.x + cx).saturating_sub(text.len() as u16 / 2);
            if x < next_free || x + text.len() as u16 > area.right() {
                continue;
            }
            buf.set_string(x, plot.bottom(), &text, label);
            next_free = x + text.len() as u16 + 2;
        }
    }
}
