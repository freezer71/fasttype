//! Barre de config en haut de l'écran de test (`TestConfig.tsx`) :
//! `@ punctuation  # numbers │ time words quote zen custom │ 15 30 60 120 custom`.

use crate::theme::Palette;
use fasttype_store::Config;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarItem {
    pub text: String,
    /// Valeur en cours : couleur `main`.
    pub active: bool,
    /// Désactivé (punctuation et numbers en quote) : à moitié effacé.
    pub enabled: bool,
}

fn item(text: impl Into<String>, active: bool) -> BarItem {
    BarItem {
        text: text.into(),
        active,
        enabled: true,
    }
}

/// Groupes de la barre selon la config, dans l'ordre du site. `compact` :
/// « @ » et « # » seuls, pour les terminaux étroits.
pub fn bar_groups(c: &Config, compact: bool) -> Vec<Vec<BarItem>> {
    let mode = c.str("mode");
    let mut groups = Vec::new();
    // punctuation et numbers : cachés en zen, désactivés en quote
    if mode != "zen" {
        let enabled = mode != "quote";
        let (p, n) = if compact {
            ("@", "#")
        } else {
            ("@ punctuation", "# numbers")
        };
        groups.push(vec![
            BarItem {
                enabled,
                ..item(p, c.bool("punctuation") && enabled)
            },
            BarItem {
                enabled,
                ..item(n, c.bool("numbers") && enabled)
            },
        ]);
    }
    groups.push(
        ["time", "words", "quote", "zen", "custom"]
            .iter()
            .map(|m| item(*m, *m == mode))
            .collect(),
    );
    let presets = |current: i64, values: &[i64]| -> Vec<BarItem> {
        let mut v: Vec<BarItem> = values
            .iter()
            .map(|n| item(n.to_string(), *n == current))
            .collect();
        v.push(item("custom", !values.contains(&current)));
        v
    };
    match mode {
        "time" => groups.push(presets(c.int("time"), &[15, 30, 60, 120])),
        "words" => groups.push(presets(c.int("words"), &[10, 25, 50, 100])),
        "quote" => {
            let lengths = c.int_list("quoteLength");
            let all = [0, 1, 2, 3].iter().all(|g| lengths.contains(g));
            let mut v = vec![item("all", all)];
            for (g, name) in ["short", "medium", "long", "thicc"].iter().enumerate() {
                v.push(item(*name, !all && lengths.contains(&(g as i64))));
            }
            groups.push(v);
        }
        "custom" => groups.push(vec![item("change", false)]),
        _ => {}
    }
    groups
}

/// Largeur de la barre : éléments séparés de 2 espaces, groupes de « │ ».
pub fn bar_width(groups: &[Vec<BarItem>]) -> u16 {
    let items: usize = groups
        .iter()
        .map(|g| g.iter().map(|i| i.text.width()).sum::<usize>() + 2 * g.len().saturating_sub(1))
        .sum();
    (items + 5 * groups.len().saturating_sub(1) + 4) as u16
}

/// Place de la barre : à côté du logo (ligne 1) si elle tient, sinon en
/// dessous (ligne 3), compacte au besoin ; `None` si même compacte elle ne tient pas.
pub fn bar_layout(c: &Config, area: Rect) -> Option<(u16, Vec<Vec<BarItem>>)> {
    let full = bar_groups(c, false);
    let bar = if bar_width(&full) <= area.width {
        full
    } else {
        bar_groups(c, true)
    };
    if bar_width(&bar) > area.width {
        return None;
    }
    let y = if bar_width(&bar) + 24 <= area.width {
        area.y + 1
    } else {
        area.y + 3
    };
    Some((y, bar))
}

/// Dessine la barre centrée sur la ligne `y`, sur un fond `subAlt`.
pub fn render_bar(buf: &mut Buffer, area: Rect, y: u16, groups: &[Vec<BarItem>], p: &Palette) {
    let width = bar_width(groups);
    if width > area.width || y >= area.bottom() {
        return;
    }
    let x0 = area.x + (area.width - width) / 2;
    let back = Style::default().bg(p.sub_alt);
    buf.set_string(x0, y, " ".repeat(usize::from(width)), back);
    let mut x = x0 + 2;
    for (k, group) in groups.iter().enumerate() {
        if k > 0 {
            buf.set_string(x + 2, y, "│", back.fg(p.bg));
            x += 5;
        }
        for (j, it) in group.iter().enumerate() {
            if j > 0 {
                x += 2;
            }
            let fg = if !it.enabled {
                p.over_bg(crate::theme::mix(p.rgb.sub_alt, p.rgb.sub, 0.5), 1.0)
            } else if it.active {
                p.main
            } else {
                p.sub
            };
            buf.set_string(x, y, &it.text, back.fg(fg));
            x += it.text.width() as u16;
        }
    }
}
