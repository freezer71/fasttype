# fasttype-tui, partie C : mots agrandis, barre de config — plan d'implémentation (plan 4c)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Afficher les mots du test plus grands que le reste de l'interface, comme sur Monkeytype (`fontSize` 2 par défaut), dans les terminaux qui le permettent. Remplacer le résumé de l'en-tête par la barre de config du site, et ajouter le badge de langue au-dessus des mots et la source de la citation sur le résultat.

**Architecture:**
- **Texte agrandi** (`sized.rs`) : protocole OSC 66 de Kitty (≥ 0.40).
  - Au démarrage, une sonde vérifie que le terminal le gère : un espace agrandi doit faire avancer le curseur de 2 cases.
  - La zone des mots est réservée dans le tampon ratatui (`CellDiffOption::Skip`), qui n'y écrit jamais.
  - Après chaque dessin, la boucle efface puis réécrit cette zone d'un bloc, lettre par lettre (`\e]66;s=2;a\a`), et seulement si elle a changé.
  - Quand la zone disparaît ou bouge, ses cases sont marquées `AlwaysUpdate` : ratatui y réécrit tout, ce qui efface les lettres agrandies restées à l'écran.
- **Le caret** gagne une hauteur. L'image Kitty prend la taille des lettres, et le caret bloc teinte le fond des lettres agrandies.
- **La barre de config** (`view/config_bar.rs`) est calculée depuis la config. Elle se place à côté du logo si elle tient, sinon en dessous, et en version compacte (`@ #`) sur un terminal étroit. Elle s'efface en focus mode, comme le résumé qu'elle remplace.

**Tech Stack:** Rust 1.97 (édition 2024), `ratatui` 0.30.2 (`CellDiffOption`), `crossterm` 0.29.0, `libc` (sondes), Python 3 pour le test pty.

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§5.3 écran de test et résultat, §7). Les plans 4a et 4b sont livrés sur `main`.

**Code vérifié avant rédaction :** tout le code de ce plan a été assemblé et exécuté dans une copie de travail jetable.
- **Tests :** 341 tests du workspace au vert, et clippy ne signale rien.
- **Benchmark :** `key_and_frame_200x60` à 76 µs, sans changement par rapport à 4b.
- **Test pty, dont `--kitty --sized` :**
  - les mots sont écrits en OSC 66 ;
  - le caret Kitty est placé, puis supprimé à la sortie ;
  - latence touche → écran : p50 0,15 ms, p99 0,28 ms.

**Comportements de référence (Monkeytype, commit 574d819) :**
- **Taille du texte :** les mots sont à `fontSize` rem (2 par défaut), le reste de l'interface à 1 rem. L'échelle se fait par cases entières : 1 à 4, `fontSize` arrondi.
- **Barre de config** (`TestConfig.tsx`) : `@ punctuation`, `# numbers` │ `time words quote zen custom` │ options du mode.
  - time : `15 30 60 120 custom` ;
  - words : `10 25 50 100 custom` ;
  - quote : `all short medium long thicc` ;
  - custom : `change` ;
  - zen : rien.
- **États :**
  - punctuation et numbers sont désactivés en quote et cachés en zen ;
  - la valeur en cours est en couleur `main`, les autres en `sub`, sur un fond `subAlt` ;
  - la barre passe à l'opacité 0 en focus mode et sur le résultat.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Textes de l'interface en anglais, identiques à Monkeytype. Commentaires en français.
- `App` et les vues ne font aucune E/S terminal. Les séquences OSC 66 sont produites par `ScaledText::write` et écrites par `runner.rs`, dans la même sortie synchronisée que l'image.
- **Les sondes de démarrage** (graphique Kitty, taille du texte) lisent l'entrée directement, avant le thread clavier, et attendent 300 ms au plus.
- **Fluidité :**
  - la zone agrandie n'est réécrite que si elle a changé, ou après un effacement d'écran (redimensionnement) ;
  - le coût par frappe reste celui de 4b (benchmark sous 1 ms) ;
  - au repos, aucune image.
- Sans OSC 66, ou sans la place nécessaire (moins de 20 lettres par ligne, ou en hauteur), l'échelle redescend jusqu'à 1 : jamais de mots coupés ni d'écran vide.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace, tous au vert.

**Choix assumés :**
- **Échelle entière** : `fontSize` 1,25 donne 1, 1,5 donne 2. Les échelles fractionnaires d'OSC 66 laisseraient des trous entre les lettres.
- **Seuls les mots sont agrandis**, comme sur le site : la barre, les stats en direct et les notifications gardent la taille de base.
- **Barre en lecture seule** pour l'instant : la changer passe par `config.toml`, puis par la palette de commandes (plan 4d).
- **Caret bloc en Kitty agrandi** : il clignote en teintant le fond des lettres, ce qui réécrit la zone agrandie à chaque palier d'opacité (32 fois par seconde au plus, avant la première frappe seulement).

## Review Focus

1. **Passage du test au résultat, puis retour, en texte agrandi.** Aucune lettre agrandie ne doit rester à l'écran. Test : `leaving_the_scaled_screen_redraws_its_region` (tâche 4). Pour le redimensionnement, vérifier par lecture que `Output::present` oublie la zone écrite quand la taille change.
2. **Terminal trop petit pour l'échelle demandée.** L'échelle doit baisser, et les mots ne jamais déborder. Test : `without_support_or_room_words_stay_normal_size` (tâche 4).
3. **Terminal qui ne répond pas aux sondes** (Terminal.app, ou tmux qui avale les requêtes). Le démarrage ne doit pas bloquer plus de 300 ms par sonde, ni montrer de déchets. Tests : `text_sizing_probe_answers` (tâche 1), et le test pty sans `--sized`, qui ne renvoie qu'une position sur deux (tâche 4).
4. **Barre de config sur un terminal étroit.** Elle doit être compacte, sous le logo, ou absente si même la version compacte ne tient pas : jamais tronquée au milieu. Tests : `test_screen_shows_header_words_and_tips` (80 colonnes) et `wide_terminal_puts_the_bar_next_to_the_logo` (140 colonnes) (tâche 4).
5. **Caret Kitty en texte agrandi.** Il doit faire la hauteur de la lettre, avancer d'une lettre agrandie par frappe, et un même caret ne doit jamais être renvoyé deux fois. Tests : `words_are_scaled_when_the_terminal_supports_it` (tâche 4) et `images_have_monkeytype_proportions` (tâche 3).

---

## Structure des fichiers

```
crates/fasttype-tui/
├── src/sized.rs                   PROBE, probe_answer, scale_for, ScaledCell, ScaledText::write
├── src/view/config_bar.rs         BarItem, bar_groups, bar_width, render_bar
├── src/caret.rs                   CaretTarget/CaretFrame.height, coverage_box
├── src/kitty.rs                   caret_image(…, height_cells, …), identifiant avec la hauteur
├── src/view/live.rs               WordsBox.scale, rows(), rect()
├── src/view/test.rs               words_box(…, scale), WordsView → Option<ScaledText>, Chrome avec la barre
├── src/view/result.rs             quote_source
├── src/app.rs                     set_text_sizing, scaled_text, badge de langue, caret et teinte à l'échelle
├── src/terminal.rs                sonde générique, text_sizing_supported
├── src/runner.rs                  écriture de la zone agrandie
├── tests/{sized,config_bar,scaled}.rs   nouveaux
└── tests/{caret,kitty,view}.rs, tests/common/mod.rs   modifiés
scripts/pty_smoke.py               + --sized
```

---

### Task 1 : texte agrandi (OSC 66)

**Files:**
- Create: `crates/fasttype-tui/src/sized.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod sized;` après `session_factory`)
- Test: `crates/fasttype-tui/tests/sized.rs`

**Interfaces:**
- Produces :
  - `sized::PROBE` et `probe_answer(&[u8]) -> Option<bool>` (deux positions de curseur, la seconde 2 cases plus loin) ;
  - `scale_for(font_size: f64) -> u16` (1 à 4) ;
  - `ScaledCell { x, y, ch, style }` et `ScaledText { scale, region, bg, cells }`, avec `write(&mut impl Write)` : efface la zone, puis écrit une lettre par séquence OSC 66 avec son SGR complet.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/sized.rs` :
```rust
use fasttype_tui::sized::{ScaledCell, ScaledText, probe_answer, scale_for};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

#[test]
fn text_sizing_probe_answers() {
    assert_eq!(probe_answer(b""), None);
    assert_eq!(
        probe_answer(b"\x1b[1;1R"),
        None,
        "attend la seconde position"
    );
    assert_eq!(probe_answer(b"\x1b[1;1R\x1b[1;3R"), Some(true));
    assert_eq!(
        probe_answer(b"\x1b[1;1R\x1b[1;2R"),
        Some(false),
        "un espace normal"
    );
    assert_eq!(probe_answer(b"\x1b[1;1R\x1b[1;1R"), Some(false), "ignoré");
}

#[test]
fn font_size_to_scale() {
    assert_eq!(scale_for(2.0), 2, "défaut du site : mots à 2rem");
    assert_eq!(scale_for(1.0), 1);
    assert_eq!(scale_for(1.5), 2);
    assert_eq!(scale_for(1.25), 1);
    assert_eq!(scale_for(10.0), 4);
    assert_eq!(scale_for(f64::NAN), 1);
}

#[test]
fn scaled_text_clears_the_region_then_writes_each_letter() {
    let t = ScaledText {
        scale: 2,
        region: Rect::new(4, 10, 3, 2),
        bg: Color::Rgb(1, 2, 3),
        cells: vec![ScaledCell {
            x: 4,
            y: 10,
            ch: 'a',
            style: Style::default()
                .fg(Color::Rgb(9, 9, 9))
                .bg(Color::Rgb(1, 2, 3)),
        }],
    };
    let mut out = Vec::new();
    t.write(&mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(
        s,
        "\x1b[0;48;2;1;2;3m\x1b[11;5H   \x1b[12;5H   \x1b[11;5H\x1b[0;38;2;9;9;9;48;2;1;2;3m\x1b]66;s=2;a\x07\x1b[0m"
    );
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test sized`
Expected: échec de compilation (`unresolved import fasttype_tui::sized`).

- [ ] **Step 3 : implémenter `sized.rs`**

```rust
//! Texte agrandi avec le protocole de taille du texte de Kitty (OSC 66,
//! Kitty ≥ 0.40) : les mots du test s'affichent à `fontSize` fois la taille
//! du reste de l'interface, comme sur le site (mots à 2rem, interface à 1rem).
//!
//! Chaque lettre agrandie occupe un bloc de `scale × scale` cases. La zone des
//! mots est marquée `skip` dans le tampon ratatui, qui n'y écrit donc jamais :
//! elle est effacée puis réécrite ici, d'un bloc, à chaque changement.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::io::{self, Write};

/// Sonde : position du curseur, un espace en double taille, position du
/// curseur. Un terminal qui gère OSC 66 avance de 2 cases au lieu d'une.
pub const PROBE: &[u8] = b"\r\x1b[6n\x1b]66;s=2; \x07\x1b[6n";

/// Lit la réponse à `PROBE` : `None` tant que les deux positions ne sont pas
/// arrivées, puis `Some(true)` si le curseur a avancé de 2 cases.
pub fn probe_answer(bytes: &[u8]) -> Option<bool> {
    let text = String::from_utf8_lossy(bytes);
    let mut cols = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find("\x1b[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('R') else {
            break;
        };
        if let Some((_, col)) = after[..end].split_once(';')
            && let Ok(c) = col.parse::<u32>()
        {
            cols.push(c);
        }
        rest = &after[end + 1..];
    }
    match cols[..] {
        [a, b, ..] => Some(b == a + 2),
        _ => None,
    }
}

/// Échelle du texte des mots : `fontSize` arrondi, de 1 à 4.
pub fn scale_for(font_size: f64) -> u16 {
    if font_size.is_finite() {
        font_size.round().clamp(1.0, 4.0) as u16
    } else {
        1
    }
}

/// Une lettre agrandie, coin haut-gauche (`x`, `y`) en cases de l'écran.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledCell {
    pub x: u16,
    pub y: u16,
    pub ch: char,
    pub style: Style,
}

/// La zone des mots agrandis : à effacer puis réécrire d'un bloc.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledText {
    pub scale: u16,
    pub region: Rect,
    /// Fond de la zone (couleur `bg` du thème).
    pub bg: Color,
    pub cells: Vec<ScaledCell>,
}

fn color(out: &mut Vec<u8>, base: u8, c: Color) {
    let _ = match c {
        Color::Rgb(r, g, b) => write!(out, ";{base};2;{r};{g};{b}"),
        Color::Indexed(i) => write!(out, ";{base};5;{i}"),
        _ => Ok(()),
    };
}

/// SGR complet d'un style (repart de zéro : `0`).
fn sgr(out: &mut Vec<u8>, style: &Style) {
    out.extend_from_slice(b"\x1b[0");
    if let Some(fg) = style.fg {
        color(out, 38, fg);
    }
    if let Some(bg) = style.bg {
        color(out, 48, bg);
    }
    if style.add_modifier.contains(Modifier::BOLD) {
        out.extend_from_slice(b";1");
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        out.extend_from_slice(b";4");
        if let Some(u) = style.underline_color {
            color(out, 58, u);
        }
    }
    out.push(b'm');
}

impl ScaledText {
    /// Efface la zone (écraser la case haut-gauche d'une lettre agrandie
    /// l'efface en entier) puis écrit chaque lettre.
    pub fn write(&self, out: &mut impl Write) -> io::Result<()> {
        let mut buf = Vec::with_capacity(64 * self.cells.len() + 8 * self.region.area() as usize);
        sgr(&mut buf, &Style::default().bg(self.bg));
        let blank = " ".repeat(usize::from(self.region.width));
        for y in self.region.top()..self.region.bottom() {
            let _ = write!(buf, "\x1b[{};{}H{blank}", y + 1, self.region.x + 1);
        }
        for c in &self.cells {
            let _ = write!(buf, "\x1b[{};{}H", c.y + 1, c.x + 1);
            sgr(&mut buf, &c.style);
            let _ = write!(buf, "\x1b]66;s={};{}\x07", self.scale, c.ch);
        }
        buf.extend_from_slice(b"\x1b[0m");
        out.write_all(&buf)
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test sized && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 3 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): texte agrandi par le protocole OSC 66 (sonde, échelle, écriture)"
```

---

### Task 2 : barre de config

**Files:**
- Create: `crates/fasttype-tui/src/view/config_bar.rs`
- Modify: `crates/fasttype-tui/src/view/mod.rs` (`pub mod config_bar;` après `chart`)
- Test: `crates/fasttype-tui/tests/config_bar.rs`

**Interfaces:**
- Consumes : `Palette::over_bg`, `theme::mix` (plan 4b), `fasttype_store::Config`.
- Produces :
  - `BarItem { text, active, enabled }` ;
  - `bar_groups(&Config, compact) -> Vec<Vec<BarItem>>` ;
  - `bar_width(&[Vec<BarItem>]) -> u16` : éléments séparés de 2 espaces, groupes de « │ » sur 5 cases, 2 cases de marge de chaque côté ;
  - `render_bar(buf, area, y, groups, palette)`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/config_bar.rs` :
```rust
mod common;

use common::app;
use fasttype_tui::view::config_bar::{bar_groups, bar_width};

#[test]
fn config_bar_follows_the_mode() {
    let a = app("bar-time", "");
    let g = bar_groups(&a.store.config, false);
    let texts: Vec<Vec<(&str, bool)>> = g
        .iter()
        .map(|grp| grp.iter().map(|i| (i.text.as_str(), i.active)).collect())
        .collect();
    assert_eq!(texts[0], [("@ punctuation", false), ("# numbers", false)]);
    assert_eq!(
        texts[1],
        [
            ("time", true),
            ("words", false),
            ("quote", false),
            ("zen", false),
            ("custom", false)
        ]
    );
    assert_eq!(
        texts[2],
        [
            ("15", false),
            ("30", true),
            ("60", false),
            ("120", false),
            ("custom", false)
        ]
    );
    assert_eq!(bar_width(&g), 92);
    let compact = bar_groups(&a.store.config, true);
    assert_eq!(compact[0][0].text, "@");
    assert_eq!(bar_width(&compact), 72);

    let a = app("bar-quote", "mode = \"quote\"\npunctuation = true\n");
    let g = bar_groups(&a.store.config, false);
    assert!(!g[0][0].enabled && !g[0][0].active, "désactivé en quote");
    let lengths: Vec<_> = g[2].iter().map(|i| (i.text.as_str(), i.active)).collect();
    assert_eq!(
        lengths,
        [
            ("all", false),
            ("short", false),
            ("medium", true),
            ("long", false),
            ("thicc", false)
        ]
    );

    let a = app("bar-zen", "mode = \"zen\"\n");
    let g = bar_groups(&a.store.config, false);
    assert_eq!(g.len(), 1, "en zen : seulement les modes");

    let a = app("bar-words", "mode = \"words\"\nwords = 42\n");
    let g = bar_groups(&a.store.config, false);
    assert!(
        g[2].last().unwrap().active,
        "valeur hors préréglages : custom"
    );
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test config_bar`
Expected: échec de compilation (`unresolved import fasttype_tui::view::config_bar`).

- [ ] **Step 3 : implémenter `config_bar.rs`**

```rust
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
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test config_bar && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 1 test PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): barre de config du site (punctuation, numbers, modes, options du mode)"
```

---

### Task 3 : caret à la hauteur des lettres

**Files:**
- Modify: `crates/fasttype-tui/src/caret.rs`, `crates/fasttype-tui/src/kitty.rs` (versions complètes)
- Modify: `crates/fasttype-tui/src/app.rs` (une ligne)
- Test: `crates/fasttype-tui/tests/caret.rs`, `crates/fasttype-tui/tests/kitty.rs` (versions complètes)

**Interfaces:**
- Produces :
  - `CaretTarget.height` et `CaretFrame.height`, en lignes ;
  - `coverage_box(&CaretFrame, col, row, size) -> f64` : part couverte d'une lettre agrandie de `size × size` cases ;
  - `caret_image(style, cell, width_cells, height_cells, rgb, alpha)` ;
  - l'identifiant d'image vaut hauteur × 10 000 + style × 1000 + largeur × 100 + niveau + 1.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/caret.rs` (version complète) :
```rust
use fasttype_tui::caret::{Caret, CaretFrame, CaretStyle, CaretTarget, coverage, smooth_caret_ms};

fn at(x: f64, y: f64) -> CaretTarget {
    CaretTarget {
        x,
        y,
        width: 1.0,
        height: 1.0,
    }
}

#[test]
fn smooth_caret_durations() {
    assert_eq!(smooth_caret_ms("off"), 0.0);
    assert_eq!(smooth_caret_ms("slow"), 150.0);
    assert_eq!(smooth_caret_ms("medium"), 100.0);
    assert_eq!(smooth_caret_ms("fast"), 85.0);
}

#[test]
fn styles_from_config() {
    assert_eq!(CaretStyle::from_config("default"), CaretStyle::Bar);
    assert_eq!(CaretStyle::from_config("carrot"), CaretStyle::Bar);
    assert_eq!(CaretStyle::from_config("block"), CaretStyle::Block);
    assert_eq!(CaretStyle::from_config("off"), CaretStyle::Off);
    assert!(CaretStyle::Underline.is_full_width());
    assert!(!CaretStyle::Bar.is_full_width());
}

#[test]
fn caret_glides_with_in_out_curve() {
    let mut c = Caret::default();
    c.jump(at(10.0, 5.0));
    c.go_to(at(11.0, 5.0), 0.0, 100.0);
    let f = |t| c.frame(CaretStyle::Bar, t, true);
    assert_eq!(f(0.0).x, 10.0);
    assert_eq!(
        f(50.0).x,
        10.5,
        "inOut est symétrique : mi-chemin à mi-temps"
    );
    assert!(f(25.0).x < 10.25, "départ lent");
    assert_eq!(f(150.0).x, 11.0);
    assert!(f(50.0).moving && !f(100.0).moving);
}

#[test]
fn retarget_mid_glide_never_jumps_back() {
    let mut c = Caret::default();
    c.jump(at(0.0, 0.0));
    c.go_to(at(1.0, 0.0), 0.0, 100.0);
    let shown = c.frame(CaretStyle::Bar, 50.0, true).x;
    c.go_to(at(2.0, 0.0), 50.0, 100.0);
    assert_eq!(c.frame(CaretStyle::Bar, 50.0, true).x, shown);
    let mut last = shown;
    for t in (50..=150).step_by(5) {
        let x = c.frame(CaretStyle::Bar, f64::from(t), true).x;
        assert!(x >= last, "le caret ne recule jamais");
        last = x;
    }
    assert_eq!(last, 2.0);
}

#[test]
fn smooth_off_is_a_jump() {
    let mut c = Caret::default();
    c.go_to(at(7.0, 2.0), 0.0, 0.0);
    assert_eq!(c.frame(CaretStyle::Bar, 0.0, false).x, 7.0);
}

#[test]
fn blink_is_smooth_then_solid_once_typing() {
    let mut c = Caret::default();
    c.stop_blinking();
    c.start_blinking(1000.0);
    assert_eq!(c.opacity(1000.0, true), 0.0);
    assert!((c.opacity(1500.0, true) - 1.0).abs() < 1e-9);
    assert!(
        (c.opacity(1250.0, true) - 0.8024).abs() < 1e-3,
        "courbe ease"
    );
    assert_eq!(c.opacity(2000.0, true), 0.0, "période d'une seconde");
    // clignotement franc quand smoothCaret est off
    assert_eq!(c.opacity(1200.0, false), 1.0);
    assert_eq!(c.opacity(1700.0, false), 0.0);
    c.stop_blinking();
    assert_eq!(c.opacity(1700.0, true), 1.0);
}

#[test]
fn coverage_splits_a_block_between_two_cells() {
    let f = CaretFrame {
        style: CaretStyle::Block,
        x: 3.25,
        y: 2.0,
        width: 1.0,
        height: 1.0,
        opacity: 1.0,
        moving: true,
    };
    assert_eq!(coverage(&f, 3, 2), 0.75);
    assert_eq!(coverage(&f, 4, 2), 0.25);
    assert_eq!(coverage(&f, 5, 2), 0.0);
    assert_eq!(coverage(&f, 3, 1), 0.0);
    let wide = CaretFrame {
        width: 2.0,
        x: 3.0,
        ..f
    };
    assert_eq!(coverage(&wide, 4, 2), 1.0);
}
```

`crates/fasttype-tui/tests/kitty.rs` (version complète) :
```rust
use fasttype_tui::caret::{CaretFrame, CaretStyle};
use fasttype_tui::kitty::{
    CaretRenderer, CellPx, KittyCaret, base64, caret_image, caret_origin, place, probe_answer,
    transmit,
};

const CELL: CellPx = CellPx { w: 10, h: 24 };

fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.to_string())
    }
}

fn bar(x: f64, y: f64, opacity: f64) -> CaretFrame {
    CaretFrame {
        style: CaretStyle::Bar,
        x,
        y,
        width: 1.0,
        height: 1.0,
        opacity,
        moving: false,
    }
}

#[test]
fn base64_matches_rfc_4648() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn cell_size_from_window() {
    assert_eq!(
        CellPx::from_window(100, 50, 1000, 1200),
        Some(CellPx { w: 10, h: 24 })
    );
    assert_eq!(CellPx::from_window(100, 50, 0, 0), None);
}

#[test]
fn renderer_detection() {
    let cell = Some(CELL);
    let none = env(&[]);
    assert_eq!(
        CaretRenderer::detect(&none, cell, true),
        CaretRenderer::Kitty(CELL),
        "le terminal a répondu OK à la sonde graphique"
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM", "xterm-kitty")]), cell, false),
        CaretRenderer::Cell,
        "pas de réponse (Zellij, tmux, terminal imbriqué) : pas d'image"
    );
    assert_eq!(
        CaretRenderer::detect(&none, None, true),
        CaretRenderer::Cell,
        "taille des cases inconnue"
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("FASTTYPE_CARET", "kitty")]), cell, false),
        CaretRenderer::Kitty(CELL)
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("FASTTYPE_CARET", "cell")]), cell, true),
        CaretRenderer::Cell
    );
}

#[test]
fn graphics_probe_answers() {
    assert_eq!(probe_answer(b""), None, "réponse pas encore arrivée");
    assert_eq!(probe_answer(b"\x1b_Gi=31;OK\x1b\\"), None, "attend DA1");
    assert_eq!(probe_answer(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;22c"), Some(true));
    assert_eq!(probe_answer(b"\x1b[?62c"), Some(false));
    assert_eq!(
        probe_answer(b"\x1b_Gi=31;EINVAL:bad\x1b\\\x1b[?1;2c"),
        Some(false)
    );
}

#[test]
fn unchanged_caret_is_not_sent_again() {
    let mut k = KittyCaret::new(CELL);
    let mut out = Vec::new();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), (1, 2, 3))
        .unwrap();
    out.clear();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), (1, 2, 3))
        .unwrap();
    assert!(out.is_empty(), "même image au même endroit : rien à écrire");
    k.invalidate();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), (1, 2, 3))
        .unwrap();
    assert!(!out.is_empty(), "après un effacement d'écran, on replace");
}

#[test]
fn transmit_is_chunked_at_4096() {
    let small = transmit(7, 1, 1, &[1, 2, 3, 4]);
    assert_eq!(small, b"\x1b_Ga=t,f=32,s=1,v=1,i=7,q=2,m=0;AQIDBA==\x1b\\");
    let big = String::from_utf8(transmit(8, 40, 40, &[0u8; 40 * 40 * 4])).unwrap();
    // 6400 octets → 8536 caractères base64 → 3 morceaux
    assert_eq!(big.matches("\x1b_G").count(), 3);
    assert!(big.starts_with("\x1b_Ga=t,f=32,s=40,v=40,i=8,q=2,m=1;"));
    assert_eq!(big.matches("\x1b_Gm=1;").count(), 1);
    assert_eq!(big.matches("\x1b_Gm=0;").count(), 1);
}

#[test]
fn placement_moves_the_cursor_then_places() {
    let mut out = Vec::new();
    place(&mut out, 1101, 4, 2, 3, 0).unwrap();
    assert_eq!(out, b"\x1b[3;5H\x1b_Ga=p,i=1101,p=1,X=3,Y=0,C=1,q=2\x1b\\");
}

#[test]
fn images_have_monkeytype_proportions() {
    let rgb = (226, 183, 20);
    let (w, h, px) = caret_image(CaretStyle::Bar, CELL, 1, 1, rgb, 255);
    assert_eq!((w, h), (2, 24), "0,1em de large, toute la hauteur");
    assert_eq!(px.len(), (w * h * 4) as usize);
    assert_eq!(&px[..4], &[226, 183, 20, 255]);
    let (w, h, _) = caret_image(CaretStyle::Underline, CELL, 2, 1, rgb, 255);
    assert_eq!((w, h), (20, 2), "largeur de la lettre (CJK : 2 cases)");
    let (w, h, px) = caret_image(CaretStyle::Outline, CELL, 1, 1, rgb, 255);
    assert_eq!((w, h), (10, 24));
    let alpha = |x: u32, y: u32| px[((y * w + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 5), 255, "bord");
    assert_eq!(alpha(5, 12), 0, "intérieur transparent");
}

#[test]
fn bar_is_centred_on_the_letter_edge() {
    // x = 3.5 cases → 35 px, moins la moitié de la barre (1 px)
    assert_eq!(caret_origin(&bar(3.5, 2.0, 1.0), CELL), (34, 48));
    assert_eq!(
        caret_origin(&bar(0.0, 0.0, 1.0), CELL),
        (0, 0),
        "jamais négatif"
    );
    let u = CaretFrame {
        style: CaretStyle::Underline,
        ..bar(1.0, 1.0, 1.0)
    };
    assert_eq!(caret_origin(&u, CELL), (10, 46), "sous la lettre");
}

#[test]
fn kitty_caret_sends_each_image_once_and_moves_it() {
    let mut k = KittyCaret::new(CELL);
    let rgb = (1, 2, 3);
    let mut out = Vec::new();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), rgb).unwrap();
    let first = String::from_utf8(out.clone()).unwrap();
    assert!(first.contains("a=t"), "transmission");
    assert!(first.ends_with("\x1b[3;4H\x1b_Ga=p,i=11117,p=1,X=4,Y=0,C=1,q=2\x1b\\"));
    out.clear();
    k.draw(&mut out, Some(bar(3.6, 2.0, 1.0)), rgb).unwrap();
    let second = String::from_utf8(out.clone()).unwrap();
    assert!(!second.contains("a=t"), "image déjà transmise");
    assert!(second.contains("X=5"));
    out.clear();
    // clignotement : une autre opacité est une autre image ; l'ancienne est retirée
    k.draw(&mut out, Some(bar(3.6, 2.0, 0.5)), rgb).unwrap();
    let third = String::from_utf8(out.clone()).unwrap();
    assert!(third.starts_with("\x1b_Ga=t"));
    assert!(third.contains("\x1b_Ga=d,d=i,i=11117,q=2\x1b\\"));
    out.clear();
    k.draw(&mut out, None, rgb).unwrap();
    assert_eq!(out, b"\x1b_Ga=d,d=i,i=11109,q=2\x1b\\");
    out.clear();
    // changement de thème : tout est supprimé puis retransmis
    k.draw(&mut out, Some(bar(1.0, 1.0, 1.0)), (9, 9, 9))
        .unwrap();
    let fourth = String::from_utf8(out).unwrap();
    assert!(fourth.starts_with("\x1b_Ga=d,d=A,q=2\x1b\\\x1b_Ga=t"));
}

#[test]
fn block_and_off_are_not_images() {
    let mut k = KittyCaret::new(CELL);
    let mut out = Vec::new();
    let block = CaretFrame {
        style: CaretStyle::Block,
        ..bar(1.0, 1.0, 1.0)
    };
    k.draw(&mut out, Some(block), (1, 2, 3)).unwrap();
    assert!(out.is_empty());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test caret --test kitty`
Expected: échec de compilation (`missing field height`, `this function takes 5 arguments but 6 arguments were supplied`).

- [ ] **Step 3 : implémenter**

`crates/fasttype-tui/src/caret.rs` (version complète) :
```rust
//! Caret de Monkeytype (`elements/caret.ts`, `caret.scss`) : position en cases
//! fractionnaires, glissement `inOut(1.25)`, clignotement d'une seconde.

use crate::anim::{CARET_EASE, CSS_EASE, Tween};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretStyle {
    Off,
    /// `default` : barre fine avant la lettre.
    Bar,
    /// Pavé derrière la lettre.
    Block,
    /// Contour de la lettre.
    Outline,
    /// Trait sous la lettre.
    Underline,
}

impl CaretStyle {
    /// `caretStyle` ; les carets en image (carrot, banana, monkey) deviennent une barre.
    pub fn from_config(name: &str) -> Self {
        match name {
            "off" => CaretStyle::Off,
            "block" => CaretStyle::Block,
            "outline" => CaretStyle::Outline,
            "underline" => CaretStyle::Underline,
            _ => CaretStyle::Bar,
        }
    }

    /// Block, outline et underline prennent la largeur de la lettre visée.
    pub fn is_full_width(self) -> bool {
        matches!(
            self,
            CaretStyle::Block | CaretStyle::Outline | CaretStyle::Underline
        )
    }
}

/// `smoothCaret` → durée du glissement (ms).
pub fn smooth_caret_ms(setting: &str) -> f64 {
    match setting {
        "slow" => 150.0,
        "medium" => 100.0,
        "fast" => 85.0,
        _ => 0.0,
    }
}

/// Où le caret doit aller : bord gauche de la lettre visée (colonne, ligne, en
/// cases de l'écran) et largeur de cette lettre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretTarget {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    /// Hauteur de la lettre en lignes (taille du texte, `fontSize`).
    pub height: f64,
}

/// Ce qu'il faut dessiner à un instant donné.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretFrame {
    pub style: CaretStyle,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    /// Encore en mouvement : l'image suivante sera différente.
    pub moving: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Caret {
    x: Tween,
    y: Tween,
    width: Tween,
    height: f64,
    /// Début du cycle de clignotement ; `None` : caret plein (on tape).
    blink_since: Option<f64>,
}

impl Default for Caret {
    fn default() -> Self {
        Caret {
            x: Tween::fixed(0.0),
            y: Tween::fixed(0.0),
            width: Tween::fixed(1.0),
            height: 1.0,
            blink_since: Some(0.0),
        }
    }
}

impl Caret {
    /// `resetPosition` : place le caret sans animation.
    pub fn jump(&mut self, t: CaretTarget) {
        self.x.jump(t.x);
        self.y.jump(t.y);
        self.width.jump(t.width);
        self.height = t.height;
    }

    /// `goTo` : glisse vers la cible en `duration` ms (0 : saut), en repartant
    /// de la position affichée si une animation est en cours.
    pub fn go_to(&mut self, t: CaretTarget, now: f64, duration: f64) {
        self.x.retarget(t.x, now, duration, CARET_EASE);
        self.y.retarget(t.y, now, duration, CARET_EASE);
        self.width.retarget(t.width, now, duration, CARET_EASE);
        self.height = t.height;
    }

    pub fn target(&self) -> CaretTarget {
        CaretTarget {
            x: self.x.to,
            y: self.y.to,
            width: self.width.to,
            height: self.height,
        }
    }

    pub fn start_blinking(&mut self, now: f64) {
        if self.blink_since.is_none() {
            self.blink_since = Some(now);
        }
    }

    /// Chaque frappe rend le caret plein (`stopBlinking`).
    pub fn stop_blinking(&mut self) {
        self.blink_since = None;
    }

    /// Début du cycle de clignotement en cours.
    pub fn blink_since(&self) -> Option<f64> {
        self.blink_since
    }

    pub fn is_blinking(&self) -> bool {
        self.blink_since.is_some()
    }

    /// Opacité du clignotement : `caretFlashSmooth` (0 → 1 → 0 en 1 s, courbe
    /// `ease` entre les étapes) ou `caretFlashHard` (plein 50 %, éteint 50 %).
    pub fn opacity(&self, now: f64, smooth: bool) -> f64 {
        let Some(since) = self.blink_since else {
            return 1.0;
        };
        let phase = (now - since).rem_euclid(1000.0) / 1000.0;
        if !smooth {
            return if phase < 0.5 { 1.0 } else { 0.0 };
        }
        if phase < 0.5 {
            CSS_EASE.apply(phase * 2.0)
        } else {
            1.0 - CSS_EASE.apply((phase - 0.5) * 2.0)
        }
    }

    pub fn is_moving(&self, now: f64) -> bool {
        self.x.is_running(now) || self.y.is_running(now) || self.width.is_running(now)
    }

    pub fn frame(&self, style: CaretStyle, now: f64, smooth_blink: bool) -> CaretFrame {
        CaretFrame {
            style,
            x: self.x.value(now),
            y: self.y.value(now),
            width: self.width.value(now),
            height: self.height,
            opacity: self.opacity(now, smooth_blink),
            moving: self.is_moving(now),
        }
    }
}

/// Part de la case (`col`, `row`) couverte par le rectangle du caret
/// `[x, x + width) × [y, y + height)` : sert à teinter le fond des lettres pendant
/// le glissement du caret bloc (rendu demi-case et plus fin).
pub fn coverage(f: &CaretFrame, col: u16, row: u16) -> f64 {
    coverage_box(f, col, row, 1)
}

/// Part d'un bloc de `size × size` cases (une lettre agrandie) couverte par le caret.
pub fn coverage_box(f: &CaretFrame, col: u16, row: u16, size: u16) -> f64 {
    let s = f64::from(size.max(1));
    let overlap = |a0: f64, a1: f64, b0: f64| (a1.min(b0 + s) - a0.max(b0)).max(0.0) / s;
    overlap(f.x, f.x + f.width, f64::from(col)) * overlap(f.y, f.y + f.height, f64::from(row))
}
```

`crates/fasttype-tui/src/kitty.rs` (version complète) :
```rust
//! Caret au pixel près avec le protocole graphique de Kitty (aussi géré par
//! Ghostty) : une petite image de la couleur du caret, transmise une fois, puis
//! replacée à chaque image avec un décalage en pixels dans la case.
//! Les réponses du terminal sont coupées (`q=2`) : rien ne revient sur l'entrée.

use crate::caret::{CaretFrame, CaretStyle};
use crate::theme::Rgb;
use std::collections::HashSet;
use std::io::{self, Write};

/// Taille d'une case en pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellPx {
    pub w: u32,
    pub h: u32,
}

impl CellPx {
    /// Depuis la taille de la fenêtre (`TIOCGWINSZ`) ; `None` si le terminal
    /// ne donne pas sa taille en pixels.
    pub fn from_window(columns: u16, rows: u16, width_px: u16, height_px: u16) -> Option<Self> {
        if columns == 0 || rows == 0 || width_px == 0 || height_px == 0 {
            return None;
        }
        Some(CellPx {
            w: u32::from(width_px) / u32::from(columns),
            h: u32::from(height_px) / u32::from(rows),
        })
        .filter(|c| c.w > 0 && c.h > 0)
    }
}

/// Façon de dessiner le caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretRenderer {
    /// Curseur du terminal, case par case.
    Cell,
    /// Image Kitty, au pixel près.
    Kitty(CellPx),
}

impl CaretRenderer {
    /// `FASTTYPE_CARET=cell|kitty` force le choix. Sinon, l'image n'est
    /// utilisée que si le terminal a répondu OK à la sonde graphique
    /// (`graphics`) : un terminal imbriqué ou un multiplexeur (tmux, Zellij)
    /// qui hérite des variables de Kitty mais n'affiche pas les images n'a
    /// jamais de caret invisible.
    pub fn detect(
        get: impl Fn(&str) -> Option<String>,
        cell: Option<CellPx>,
        graphics: bool,
    ) -> Self {
        let kitty = match get("FASTTYPE_CARET").as_deref() {
            Some("cell") => false,
            Some("kitty") => true,
            _ => graphics,
        };
        match cell {
            Some(c) if kitty => CaretRenderer::Kitty(c),
            _ => CaretRenderer::Cell,
        }
    }
}

/// Sonde graphique : une image 1 × 1 en requête (`a=q`), suivie d'une
/// demande d'attributs (DA1) à laquelle tous les terminaux répondent.
pub const PROBE: &[u8] = b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c";

/// Lit la réponse à `PROBE` : `None` tant que la réponse DA1 n'est pas
/// arrivée, puis `Some(true)` si le terminal a répondu OK à la requête graphique.
pub fn probe_answer(bytes: &[u8]) -> Option<bool> {
    let text = String::from_utf8_lossy(bytes);
    let da1 = text.find("\x1b[?")?;
    text[da1..].find('c')?;
    Some(text[..da1].contains("\x1b_Gi=31;OK"))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(B64[((n >> (18 - 6 * k)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Transmission d'une image RGBA (`a=t`), découpée en morceaux de 4096 octets.
pub fn transmit(id: u32, w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let data = base64(rgba);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    let mut out = Vec::with_capacity(data.len() + 64 * chunks.len());
    for (k, chunk) in chunks.iter().enumerate() {
        let more = u8::from(k + 1 < chunks.len());
        if k == 0 {
            let _ = write!(out, "\x1b_Ga=t,f=32,s={w},v={h},i={id},q=2,m={more};");
        } else {
            let _ = write!(out, "\x1b_Gm={more};");
        }
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }
    out
}

/// Place l'image `id` dans la case (`col`, `row`), décalée de (`dx`, `dy`) pixels.
/// Replacer la même image déplace son unique placement (`p=1`), sans clignotement.
pub fn place(
    out: &mut impl Write,
    id: u32,
    col: u32,
    row: u32,
    dx: u32,
    dy: u32,
) -> io::Result<()> {
    write!(
        out,
        "\x1b[{};{}H\x1b_Ga=p,i={id},p=1,X={dx},Y={dy},C=1,q=2\x1b\\",
        row + 1,
        col + 1
    )
}

/// Retire le placement de l'image `id` (les pixels restent en mémoire du terminal).
pub fn hide(out: &mut impl Write, id: u32) -> io::Result<()> {
    write!(out, "\x1b_Ga=d,d=i,i={id},q=2\x1b\\")
}

/// Supprime toutes les images et libère leur mémoire.
pub const DELETE_ALL: &[u8] = b"\x1b_Ga=d,d=A,q=2\x1b\\";

/// Épaisseur d'un trait fin : 0,1em, pour une case de 1,2em de haut.
fn thin(cell_h: u32) -> u32 {
    (cell_h as f64 / 12.0).round().max(2.0) as u32
}

/// Image du caret pour une lettre de `width_cells` × `height_cells` cases :
/// taille en pixels et pixels RGBA (alpha = `alpha`).
pub fn caret_image(
    style: CaretStyle,
    cell: CellPx,
    width_cells: u32,
    height_cells: u32,
    rgb: Rgb,
    alpha: u8,
) -> (u32, u32, Vec<u8>) {
    let full = width_cells.max(1) * cell.w;
    let tall = height_cells.max(1) * cell.h;
    let (w, h) = match style {
        CaretStyle::Underline => (full, thin(tall)),
        CaretStyle::Outline => (full, tall),
        _ => (thin(tall), tall),
    };
    let border = (tall as f64 / 24.0).round().max(1.0) as u32;
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let on = style != CaretStyle::Outline
                || x < border
                || y < border
                || x + border >= w
                || y + border >= h;
            px.extend_from_slice(&[rgb.0, rgb.1, rgb.2, if on { alpha } else { 0 }]);
        }
    }
    (w, h, px)
}

/// Coin haut-gauche de l'image en pixels de l'écran.
pub fn caret_origin(f: &CaretFrame, cell: CellPx) -> (u32, u32) {
    let (cw, ch) = (f64::from(cell.w), f64::from(cell.h));
    let tall = (f64::from(cell.h) * f.height.max(1.0)).round() as u32;
    let mut x = f.x * cw;
    let mut y = f.y * ch;
    match f.style {
        // barre centrée sur le bord gauche de la lettre
        CaretStyle::Bar => x -= f64::from(thin(tall)) / 2.0,
        CaretStyle::Underline => y += f64::from(tall) - f64::from(thin(tall)),
        _ => {}
    }
    (x.round().max(0.0) as u32, y.round().max(0.0) as u32)
}

/// Niveaux d'opacité pré-transmis pour le clignotement doux.
pub const LEVELS: u32 = 16;

/// Caret Kitty : transmet les images au besoin et déplace le placement.
#[derive(Debug, Clone)]
pub struct KittyCaret {
    cell: CellPx,
    rgb: Option<Rgb>,
    sent: HashSet<u32>,
    placed: Option<u32>,
    /// Dernier placement écrit (image, case, décalage) : rien à renvoyer s'il ne change pas.
    last: Option<(u32, u32, u32, u32, u32)>,
}

impl KittyCaret {
    pub fn new(cell: CellPx) -> Self {
        KittyCaret {
            cell,
            rgb: None,
            sent: HashSet::new(),
            placed: None,
            last: None,
        }
    }

    /// L'écran a été effacé : le prochain dessin replace l'image.
    pub fn invalidate(&mut self) {
        self.last = None;
    }

    fn image_id(style: CaretStyle, width_cells: u32, height_cells: u32, level: u32) -> u32 {
        let s = match style {
            CaretStyle::Underline => 2,
            CaretStyle::Outline => 3,
            _ => 1,
        };
        height_cells.min(9) * 10_000 + s * 1000 + width_cells.min(9) * 100 + level + 1
    }

    /// Écrit ce qu'il faut pour montrer `frame` (ou rien si `None`).
    /// Le style bloc est dessiné dans les cases (teinte), pas en image.
    pub fn draw(
        &mut self,
        out: &mut impl Write,
        frame: Option<CaretFrame>,
        rgb: Rgb,
    ) -> io::Result<()> {
        if self.rgb != Some(rgb) {
            if self.rgb.is_some() {
                out.write_all(DELETE_ALL)?;
            }
            self.rgb = Some(rgb);
            self.sent.clear();
            self.placed = None;
            self.last = None;
        }
        let shown = frame
            .filter(|f| !matches!(f.style, CaretStyle::Off | CaretStyle::Block) && f.opacity > 0.0);
        let Some(f) = shown else {
            if let Some(id) = self.placed.take() {
                hide(out, id)?;
            }
            self.last = None;
            return Ok(());
        };
        let width_cells = f.width.round().max(1.0) as u32;
        let level = (f.opacity * f64::from(LEVELS))
            .round()
            .clamp(1.0, f64::from(LEVELS)) as u32;
        let height_cells = f.height.round().max(1.0) as u32;
        let id = Self::image_id(f.style, width_cells, height_cells, level);
        if self.sent.insert(id) {
            let alpha = (f64::from(level) / f64::from(LEVELS) * 255.0).round() as u8;
            let (w, h, px) = caret_image(f.style, self.cell, width_cells, height_cells, rgb, alpha);
            out.write_all(&transmit(id, w, h, &px))?;
        }
        if let Some(old) = self.placed
            && old != id
        {
            hide(out, old)?;
        }
        let (px, py) = caret_origin(&f, self.cell);
        let (col, row) = (px / self.cell.w, py / self.cell.h);
        let spot = (id, col, row, px % self.cell.w, py % self.cell.h);
        if self.last == Some(spot) {
            return Ok(());
        }
        place(out, id, col, row, spot.3, spot.4)?;
        self.placed = Some(id);
        self.last = Some(spot);
        Ok(())
    }
}
```

Dans `crates/fasttype-tui/src/app.rs`, dans `place_caret`, ajouter `height: 1.0,` après `width: f64::from(width),` dans la construction de `CaretTarget`. La tâche 4 remplace ce fichier en entier.

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): caret à la hauteur des lettres (image Kitty et couverture)"
```

---

### Task 4 : mots agrandis à l'écran, barre de config, badge de langue, source de la citation

**Files:**
- Modify: `crates/fasttype-tui/src/view/live.rs`, `crates/fasttype-tui/src/view/test.rs`, `crates/fasttype-tui/src/view/result.rs`, `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/terminal.rs`, `crates/fasttype-tui/src/runner.rs` (versions complètes)
- Modify: `crates/fasttype-tui/tests/common/mod.rs`, `crates/fasttype-tui/tests/view.rs` (versions complètes), `scripts/pty_smoke.py` (version complète)
- Test: `crates/fasttype-tui/tests/scaled.rs`

**Interfaces:**
- Consumes : tâches 1 à 3.
- Produces :
  - `WordsBox { left, top, width, lines, scale }`, avec `rows()` et `rect()` (`width` en lettres) ;
  - `words_box(area, max_line_width, zen, scale)`, où l'échelle baisse si la zone ne tient pas ;
  - `WordsView::render(&self, &mut Buffer) -> Option<ScaledText>` ;
  - `Chrome { palette, logo, config, opacity, tips }` ;
  - `ResultView.quote_source` ;
  - `App::set_text_sizing(bool)` et `App::scaled_text() -> Option<&ScaledText>` ;
  - `terminal::text_sizing_supported()` ;
  - `tests/common::col(buf, y, pat)`, qui donne la colonne en cases et non en octets.
- Comportement :
  - avec OSC 66, les mots sont à l'échelle `scale_for(fontSize)` ;
  - le caret prend la hauteur et la largeur des lettres agrandies ;
  - la barre de config remplace le résumé de l'en-tête ;
  - le badge de langue (« english ») se trouve deux lignes au-dessus des mots et s'efface en focus mode ;
  - la source de la citation apparaît sur le résultat.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/common/mod.rs` (version complète) :
```rust
#![allow(dead_code)]

use fasttype_store::Store;
use fasttype_store::paths::Paths;
use fasttype_tui::app::{App, Screen};
use fasttype_tui::input::{Input, Key, Phase};
use fasttype_tui::theme::ColorMode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use std::fs;
use std::path::PathBuf;

/// Dossier temporaire propre au test : jamais le vrai `$HOME`.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fasttype-tui-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn paths(dir: &std::path::Path) -> Paths {
    Paths {
        config_dir: dir.join("config"),
        data_dir: dir.join("data"),
    }
}

/// Store dont `config.toml` contient `config` (TOML).
pub fn store_with(name: &str, config: &str) -> Store {
    let dir = scratch(name);
    fs::create_dir_all(dir.join("config")).unwrap();
    fs::write(dir.join("config/config.toml"), config).unwrap();
    Store::open(paths(&dir))
}

pub fn app(name: &str, config: &str) -> App {
    App::new(store_with(name, config), ColorMode::TrueColor, 0.0, 42)
}

fn code(key: Key) -> u32 {
    match key {
        Key::Char(c) => c as u32,
        _ => 0x11_0001,
    }
}

pub fn press(key: Key, at: f64) -> Input {
    Input::Key {
        key,
        phase: Phase::Press,
        code: code(key),
        at,
    }
}

pub fn release(key: Key, at: f64) -> Input {
    Input::Key {
        key,
        phase: Phase::Release,
        code: code(key),
        at,
    }
}

/// Tape `text` (appui puis relâchement), une touche toutes les `step` ms.
pub fn type_text(app: &mut App, text: &str, start: f64, step: f64) -> f64 {
    let mut t = start;
    for c in text.chars() {
        app.tick(t);
        app.handle(press(Key::Char(c), t));
        app.handle(release(Key::Char(c), t + step / 2.0));
        t += step;
    }
    t
}

/// Tape correctement les mots du test en cours jusqu'à l'écran de résultat.
pub fn type_whole_test(app: &mut App, start: f64, step: f64) -> f64 {
    let mut t = start;
    while matches!(app.screen(), Screen::Test) {
        let s = app.session();
        let a = s.active_index();
        let typed = s.input(a).chars().count();
        let next = s.word(a).chars().nth(typed).unwrap_or(' ');
        t = type_text(app, &next.to_string(), t, step);
    }
    t
}

/// Laisse finir les fondus (restart, résultat) : 250 ms après `t`.
pub fn settle(app: &mut App, t: f64) -> f64 {
    let fade = fasttype_tui::app::FADE_MS;
    app.tick(t + fade);
    app.tick(t + 2.0 * fade);
    t + 2.0 * fade
}

/// Rend l'application dans un terminal de test ; renvoie le tampon et le caret.
pub fn render(app: &mut App, w: u16, h: u16) -> (Buffer, Option<(u16, u16)>) {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| app.draw(f, None)).unwrap();
    let backend = term.backend();
    let caret = backend.cursor_visible().then(|| {
        let p = backend.cursor_position();
        (p.x, p.y)
    });
    (backend.buffer().clone(), caret)
}

/// Colonne (en cases) de la première occurrence de `pat` dans la ligne `y`.
pub fn col(buf: &Buffer, y: u16, pat: &str) -> Option<u16> {
    let r = row(buf, y);
    r.find(pat).map(|i| r[..i].chars().count() as u16)
}

/// Texte d'une ligne du tampon.
pub fn row(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

pub fn screen_text(buf: &Buffer) -> String {
    (0..buf.area.height)
        .map(|y| row(buf, y))
        .collect::<Vec<_>>()
        .join("\n")
}
```

`crates/fasttype-tui/tests/view.rs` (version complète) :
```rust
mod common;

use common::{app, col, render, row, screen_text, settle, type_text, type_whole_test};

#[test]
fn test_screen_shows_header_words_and_tips() {
    let mut a = app("v-start", "");
    let (buf, caret) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(row(&buf, 1).contains("fasttype"));
    // barre de config compacte à 80 colonnes, sous le logo
    assert!(row(&buf, 3).contains("time"), "{text}");
    assert!(row(&buf, 3).contains("30"));
    assert!(text.contains("english"), "badge de langue");
    assert!(text.contains("tab + enter - restart"));
    let first = a.session().word(0).trim_end().to_string();
    let words_row = (0..24)
        .find(|&y| row(&buf, y).contains(&first))
        .expect("premier mot affiché");
    let x = col(&buf, words_row, &first).unwrap();
    assert_eq!(
        caret,
        Some((x, words_row)),
        "caret avant la première lettre"
    );
}

#[test]
fn letters_are_colored_by_correctness() {
    let mut a = app("v-colors", "");
    let target: Vec<char> = a.session().word(0).chars().collect();
    let wrong = if target[1] == 'z' { 'y' } else { 'z' };
    type_text(&mut a, &format!("{}{}", target[0], wrong), 0.0, 100.0);
    let (buf, caret) = render(&mut a, 80, 24);
    let (cx, cy) = caret.unwrap();
    let p = *a.palette();
    assert_eq!(buf[(cx - 2, cy)].fg, p.text, "lettre juste");
    assert_eq!(buf[(cx - 1, cy)].fg, p.error, "lettre fausse");
    assert_eq!(
        buf[(cx - 1, cy)].symbol(),
        target[1].to_string(),
        "la lettre attendue est affichée"
    );
    assert_eq!(buf[(cx, cy)].fg, p.sub, "lettre à venir");
}

#[test]
fn focus_mode_hides_chrome_and_shows_timer() {
    let mut a = app("v-focus", "");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 100.0);
    // le focus mode s'installe en 125 ms (logo : 250 ms)
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(!text.contains("restart"));
    assert!(!text.contains("time 30"));
    assert!(text.contains("30"), "timer mini");
    // le logo reste, en couleur sub
    let x = row(&buf, 1).find("fasttype").unwrap() as u16;
    assert_eq!(buf[(x, 1)].fg, a.palette().sub);
}

#[test]
fn focus_mode_fades_the_chrome() {
    let mut a = app("v-focus-fade", "");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 100.0);
    a.tick(40.0);
    let (buf, _) = render(&mut a, 80, 24);
    let x = col(&buf, 3, "words").expect("encore visible");
    let fg = buf[(x, 3)].fg;
    assert_ne!(fg, a.palette().sub, "en cours de fondu");
    assert_ne!(fg, a.palette().bg);
}

#[test]
fn too_small_terminal_says_so() {
    let mut a = app("v-small", "");
    let (buf, caret) = render(&mut a, 30, 8);
    assert!(screen_text(&buf).contains("terminal too small"));
    assert_eq!(caret, None);
}

#[test]
fn result_screen_shows_speed_and_details() {
    let mut a = app("v-result", "mode = \"words\"\nwords = 10\n");
    let end = type_whole_test(&mut a, 0.0, 300.0);
    settle(&mut a, end);
    let (buf, caret) = render(&mut a, 100, 24);
    let text = screen_text(&buf);
    assert!(text.contains("wpm"), "{text}");
    assert!(text.contains("acc"));
    assert!(text.contains("characters"));
    assert!(text.contains("test type words 10 english"));
    assert!(text.contains('♛'), "couronne du nouveau record");
    assert!(
        text.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)),
        "graphique en braille"
    );
    assert!(text.contains("next test"));
    assert_eq!(caret, None);
}

#[test]
fn background_is_painted() {
    let mut a = app("v-bg", "");
    let (buf, _) = render(&mut a, 80, 24);
    assert_eq!(buf[(0, 0)].bg, a.palette().bg);
    assert_eq!(buf[(79, 23)].bg, a.palette().bg);
}

#[test]
fn resize_keeps_caret_after_the_typed_letters() {
    let mut a = app("v-resize", "");
    let word: String = a.session().word(0).trim_end().to_string();
    let typed: String = word.chars().take(2).collect();
    type_text(&mut a, &typed, 0.0, 100.0);
    for (w, h) in [(80, 24), (50, 12), (120, 40)] {
        let (buf, caret) = render(&mut a, w, h);
        let (cx, cy) = caret.expect("caret visible");
        let before: String = (cx - 2..cx)
            .map(|x| buf[(x, cy)].symbol().to_string())
            .collect();
        assert_eq!(before, typed, "{w}×{h}");
    }
}

#[test]
fn zero_sized_terminal_does_not_panic() {
    let mut a = app("v-zero", "");
    for (w, h) in [(100, 0), (0, 30), (0, 0), (39, 24), (80, 9)] {
        let (_, caret) = render(&mut a, w, h);
        assert_eq!(caret, None, "{w}×{h}");
    }
}
```

`crates/fasttype-tui/tests/scaled.rs` :
```rust
mod common;

use common::{app, col, press, render, row, screen_text, settle, type_text, type_whole_test};
use fasttype_tui::input::Key;
use fasttype_tui::kitty::{CaretRenderer, CellPx};
use ratatui::buffer::CellDiffOption;
use ratatui::style::Color;

#[test]
fn wide_terminal_puts_the_bar_next_to_the_logo() {
    let mut a = app("bar-wide", "");
    let (buf, _) = render(&mut a, 140, 30);
    assert!(row(&buf, 1).contains("@ punctuation"));
    let x = col(&buf, 1, "30").unwrap();
    assert_eq!(buf[(x, 1)].fg, a.palette().main, "valeur en cours");
    assert_eq!(buf[(x, 1)].bg, a.palette().sub_alt, "sur fond subAlt");
}

#[test]
fn words_are_scaled_when_the_terminal_supports_it() {
    let mut a = app("sized", "");
    a.set_text_sizing(true);
    a.set_caret_renderer(CaretRenderer::Kitty(CellPx { w: 10, h: 20 }));
    let (buf, cursor) = render(&mut a, 120, 30);
    assert_eq!(cursor, None);
    let t = a.scaled_text().expect("mots agrandis").clone();
    assert_eq!(t.scale, 2, "fontSize 2 par défaut");
    assert_eq!(t.region.height, 6, "3 lignes de 2 cases");
    let first = a.session().word(0).chars().next().unwrap();
    assert_eq!(t.cells[0].ch, first);
    assert_eq!((t.cells[1].x - t.cells[0].x), 2, "une lettre = 2 colonnes");
    // ratatui n'écrit pas dans la zone réservée : le terminal n'en reçoit rien
    let r = t.region;
    assert_eq!(buf[(r.x, r.y)].bg, Color::Reset);
    assert_ne!(buf[(r.x, r.y - 1)].bg, Color::Reset, "le reste est peint");
    // le caret prend la taille des lettres
    let f = a.caret_frame().unwrap();
    assert_eq!(f.height, 2.0);
    assert_eq!((f.x, f.y), (f64::from(r.x), f64::from(r.y)));
    // la frappe avance d'une lettre agrandie
    a.handle(press(Key::Char(first), 1000.0));
    a.tick(1500.0);
    render(&mut a, 120, 30);
    assert_eq!(a.caret_frame().unwrap().x, f64::from(r.x) + 2.0);
}

#[test]
fn without_support_or_room_words_stay_normal_size() {
    let mut a = app("sized-off", "");
    render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none(), "terminal sans OSC 66");
    let mut a = app("sized-small", "");
    a.set_text_sizing(true);
    render(&mut a, 50, 12);
    assert!(a.scaled_text().is_none(), "pas la place : échelle 1");
    let mut a = app("sized-1", "font_size = 1.0\n");
    a.set_text_sizing(true);
    render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none(), "fontSize 1");
}

#[test]
fn block_caret_tints_scaled_letters() {
    let mut a = app("sized-block", "caret_style = \"block\"\n");
    a.set_text_sizing(true);
    // mi-période du clignotement : caret pleinement visible
    a.tick(500.0);
    render(&mut a, 120, 30);
    let t = a.scaled_text().unwrap();
    let caret = a.palette().caret;
    assert_eq!(t.cells[0].style.bg, Some(caret), "la lettre sous le caret");
    assert_ne!(t.cells[1].style.bg, Some(caret));
}

#[test]
fn leaving_the_scaled_screen_redraws_its_region() {
    let mut a = app("sized-leave", "mode = \"words\"\nwords = 10\n");
    a.set_text_sizing(true);
    render(&mut a, 120, 30);
    let r = a.scaled_text().unwrap().region;
    let end = type_whole_test(&mut a, 0.0, 20.0);
    settle(&mut a, end);
    let (buf, _) = render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none(), "écran de résultat");
    assert_eq!(
        buf[(r.x, r.y)].diff_option,
        CellDiffOption::AlwaysUpdate,
        "les lettres agrandies restées à l'écran sont effacées"
    );
    let (buf, _) = render(&mut a, 120, 30);
    assert_eq!(
        buf[(r.x, r.y)].diff_option,
        CellDiffOption::None,
        "une seule fois"
    );
}

#[test]
fn quote_result_shows_the_source() {
    let mut a = app("quote-source", "mode = \"quote\"\n");
    let source = a.session().spec().quote.as_ref().unwrap().source.clone();
    let end = type_whole_test(&mut a, 0.0, 20.0);
    settle(&mut a, end);
    let (buf, _) = render(&mut a, 140, 40);
    let text = screen_text(&buf);
    assert!(text.contains("source"), "{text}");
    let shown: String = source.chars().take(20).collect();
    assert!(text.contains(&shown));
}

#[test]
fn language_badge_fades_with_the_chrome() {
    let mut a = app("badge", "");
    let (buf, _) = render(&mut a, 120, 30);
    let y = (0..30u16)
        .find(|&y| row(&buf, y).trim() == "english")
        .expect("badge");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 50.0);
    a.tick(400.0);
    let (buf, _) = render(&mut a, 120, 30);
    assert!(!row(&buf, y).contains("english"), "caché pendant la frappe");
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test scaled`
Expected: échec de compilation (`no method named set_text_sizing`, `no method named scaled_text`).

- [ ] **Step 3 : implémenter les vues**

`crates/fasttype-tui/src/view/live.rs` (version complète) :
```rust
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
```

`crates/fasttype-tui/src/view/test.rs` (version complète) :
```rust
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
```

`crates/fasttype-tui/src/view/result.rs` (version complète) :
```rust
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
    /// Source de la citation (mode quote).
    pub quote_source: Option<&'a str>,
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
        let mut next = y + 2;
        if let Some(source) = self.quote_source {
            centered_segments(
                buf,
                area,
                next,
                &[("source ".to_string(), label), (source.to_string(), detail)],
            );
            next += 1;
        }
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
                next,
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
```

- [ ] **Step 4 : implémenter l'application, les sondes et la boucle**

`crates/fasttype-tui/src/app.rs` (version complète) :
```rust
//! État de l'application et réaction aux touches. Aucune E/S terminal : la
//! boucle (`runner`) lui passe les entrées horodatées, la fait avancer dans le
//! temps (`tick`) et lui demande de dessiner.
//!
//! Animations (spec §5) : caret glissant, focus mode, fondus de 125 ms entre
//! le test et le résultat, stats en direct. Elles sont calculées à l'instant
//! `now` du dernier `tick` ou de la dernière touche.

use crate::anim::{FrameClock, OUT2, TAILWIND_EASE, Tween};
use crate::caret::{
    Caret, CaretFrame, CaretStyle, CaretTarget, coverage, coverage_box, smooth_caret_ms,
};
use crate::input::{Input, Key, Phase};
use crate::kitty::CaretRenderer;
use crate::layout::{Layout, char_width, layout_window, letters};
use crate::perf::Perf;
use crate::session_factory::SessionFactory;
use crate::sized::{ScaledCell, ScaledText, scale_for};
use crate::theme::{ColorMode, Palette, Rgb, mix};
use crate::view::live::{LiveItem, LiveStats, Style3, WordsBox, render_bar, seconds_to_string};
use crate::view::notify::{Level, Notifications};
use crate::view::result::{ResultView, invalid_label, unit_factor};
use crate::view::test::{Chrome, WordsView, words_box};
use crate::view::{MIN_HEIGHT, MIN_WIDTH, fill_background, too_small};
use fasttype_core::result::TestResult;
use fasttype_core::session::{InputOutcome, SessionState, TestSession};
use fasttype_core::spec::Mode;
use fasttype_data::themes::{Rgba, Theme};
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_store::{Config, RecordOutcome, Store};
use ratatui::Frame;
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;
use ratatui::style::Style;
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

/// Durée des fondus (test, résultat, restart, stats en direct, focus mode).
pub const FADE_MS: f64 = 125.0;

pub struct ResultInfo {
    pub result: TestResult,
    /// `None` si l'enregistrement a échoué.
    pub outcome: Option<RecordOutcome>,
}

pub enum Screen {
    Test,
    Result(Box<ResultInfo>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretShape {
    Bar,
    Block,
    Underline,
}

/// Forme, clignotement et couleur du curseur du terminal, quand il sert de caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaretLook {
    pub shape: CaretShape,
    pub blinking: bool,
    pub rgb: (u8, u8, u8),
}

/// Fondu en cours (`test-logic.ts`) : les touches sont ignorées pendant que
/// l'écran disparaît avant un restart, comme sur le site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transition {
    /// L'écran actuel disparaît ; le nouveau test est créé à `start + FADE_MS`.
    Restart { start: f64 },
    /// Le test disparaît, puis le résultat apparaît.
    ToResult { start: f64 },
    /// Le nouveau test apparaît.
    FadeIn { start: f64 },
}

/// Premier mot de la ligne du haut : la mise en page part de là, son coût ne
/// dépend pas de la longueur du test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Window {
    width: u16,
    start: usize,
}

pub struct App {
    pub store: Store,
    factory: SessionFactory,
    palette: Palette,
    session: TestSession,
    screen: Screen,
    pub notifications: Notifications,
    /// Tab vient d'être pressé : Entrée relance (« tab + enter »).
    restart_armed: Option<bool>,
    seed: u64,
    pub quit: bool,
    /// Instant du dernier `tick` ou de la dernière touche.
    now: f64,
    /// Instant de la dernière touche prise en compte : le caret glisse depuis là.
    input_at: f64,
    frames: FrameClock,
    renderer: CaretRenderer,
    caret: Caret,
    /// Le caret sera placé sans animation au prochain dessin (restart, redimensionnement).
    caret_reset: bool,
    /// Ce que le rendu Kitty doit montrer après ce dessin.
    caret_frame: Option<CaretFrame>,
    last_area: Rect,
    window: Window,
    /// Apparition de la dernière ligne après un défilement (`smoothLineScroll`).
    line_fade: Option<f64>,
    /// Opacité du résumé de la config et des raccourcis (focus mode).
    chrome: Tween,
    /// Logo : 0 = `main`, 1 = `sub` (focus mode).
    logo: Tween,
    /// Opacité des stats en direct.
    live: Tween,
    /// Longueur de la barre de progression (`timerStyle: bar`), de 0 à 1.
    bar: Tween,
    /// wpm et raw du dernier tick (une fois par seconde, comme le site).
    live_wpm: (f64, f64),
    transition: Option<Transition>,
    /// Le terminal gère OSC 66 : les mots s'affichent à `fontSize` fois la taille.
    text_sizing: bool,
    /// Mots agrandis du dernier dessin, à écrire par la boucle.
    scaled: Option<ScaledText>,
    /// Zone des mots agrandis de l'image précédente : à redessiner en entier
    /// quand elle disparaît (ratatui n'y avait rien écrit).
    last_scaled_region: Option<Rect>,
    /// Avertissements déjà montrés : un repli n'est signalé qu'une fois.
    warned: HashSet<String>,
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Thème de secours si les données embarquées sont illisibles (serika_dark).
fn fallback_theme() -> Theme {
    let c = |hex: &str| {
        Rgba::parse_hex(hex).unwrap_or(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        })
    };
    Theme {
        name: DEFAULT_THEME.to_string(),
        bg: c("#323437"),
        main: c("#e2b714"),
        caret: c("#e2b714"),
        sub: c("#646669"),
        sub_alt: c("#2c2e31"),
        text: c("#d1d0c5"),
        error: c("#ca4754"),
        error_extra: c("#7e2a33"),
        colorful_error: c("#ca4754"),
        colorful_error_extra: c("#7e2a33"),
        has_css: false,
    }
}

/// Thème de la config : `customThemeColors` si `customTheme`, sinon le thème
/// nommé, sinon `serika_dark` ; chaque repli est signalé.
fn resolve_theme(config: &Config) -> (Theme, Vec<String>) {
    let mut warnings = Vec::new();
    if config.bool("customTheme") {
        let colors: Vec<Rgba> = config
            .str_list("customThemeColors")
            .iter()
            .filter_map(|h| Rgba::parse_hex(h))
            .collect();
        if let [
            bg,
            main,
            caret,
            sub,
            sub_alt,
            text,
            error,
            error_extra,
            colorful_error,
            colorful_error_extra,
        ] = colors[..]
        {
            let t = Theme {
                name: "custom".into(),
                bg,
                main,
                caret,
                sub,
                sub_alt,
                text,
                error,
                error_extra,
                colorful_error,
                colorful_error_extra,
                has_css: false,
            };
            return (t, warnings);
        }
    }
    let name = config.str("theme");
    let t = match theme(name) {
        Some(t) => t.clone(),
        None => {
            warnings.push(format!("theme {name} not found - using {DEFAULT_THEME}"));
            theme(DEFAULT_THEME).cloned().unwrap_or_else(fallback_theme)
        }
    };
    (t, warnings)
}

/// Couleur `timerColor` en RGB.
fn timer_rgb(p: &Palette, name: &str) -> Rgb {
    match name {
        "black" => (0, 0, 0),
        "sub" => p.rgb.sub,
        "text" => p.rgb.text,
        _ => p.rgb.main,
    }
}

impl App {
    pub fn new(store: Store, color_mode: ColorMode, now: f64, seed: u64) -> App {
        let (theme, theme_warnings) = resolve_theme(&store.config);
        let mut factory = SessionFactory::new();
        let built = factory.build(&store.config, seed);
        let mut app = App {
            palette: Palette::from_theme(&theme, color_mode),
            factory,
            session: built.session,
            screen: Screen::Test,
            notifications: Notifications::default(),
            restart_armed: None,
            seed,
            quit: false,
            now,
            input_at: now,
            frames: FrameClock::new(60, now),
            renderer: CaretRenderer::Cell,
            caret: Caret::default(),
            caret_reset: true,
            caret_frame: None,
            last_area: Rect::default(),
            window: Window::default(),
            line_fade: None,
            chrome: Tween::fixed(1.0),
            logo: Tween::fixed(0.0),
            live: Tween::fixed(0.0),
            bar: Tween::fixed(0.0),
            live_wpm: (0.0, 0.0),
            transition: None,
            text_sizing: false,
            scaled: None,
            last_scaled_region: None,
            warned: HashSet::new(),
            store,
        };
        let warnings: Vec<String> = app
            .store
            .warnings
            .iter()
            .cloned()
            .chain(theme_warnings)
            .chain(built.warning)
            .collect();
        for w in warnings {
            app.warn(w, now);
        }
        app.caret.start_blinking(now);
        app.update_motion(now);
        app
    }

    /// Choisit le rendu du caret (détecté par la boucle au démarrage).
    pub fn set_caret_renderer(&mut self, renderer: CaretRenderer) {
        self.renderer = renderer;
    }

    /// Le terminal sait agrandir le texte (OSC 66, détecté au démarrage).
    pub fn set_text_sizing(&mut self, supported: bool) {
        self.text_sizing = supported;
    }

    /// Mots agrandis du dernier `draw`, à écrire après le dessin ratatui.
    pub fn scaled_text(&self) -> Option<&ScaledText> {
        self.scaled.as_ref()
    }

    /// Cadence des images d'animation (60 par défaut).
    pub fn set_fps(&mut self, fps: u32) {
        self.frames = FrameClock::new(fps, self.now);
    }

    pub fn session(&self) -> &TestSession {
        &self.session
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn transition(&self) -> Option<Transition> {
        self.transition
    }

    /// Ce que le rendu Kitty doit dessiner après le dernier `draw`.
    pub fn caret_frame(&self) -> Option<CaretFrame> {
        self.caret_frame
    }

    /// Erreur montrée une seule fois par session (un repli de langue ne
    /// revient pas à chaque restart).
    fn warn(&mut self, text: String, now: f64) {
        if self.warned.insert(text.clone()) {
            self.notifications.push(text, Level::Error, now);
        }
    }

    fn caret_style(&self) -> CaretStyle {
        CaretStyle::from_config(self.store.config.str("caretStyle"))
    }

    /// Crée le nouveau test, qui apparaît en fondu.
    fn restart(&mut self, now: f64) {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let built = self.factory.build(&self.store.config, self.seed);
        if let Some(w) = built.warning {
            self.warn(w, now);
        }
        self.session = built.session;
        self.screen = Screen::Test;
        self.restart_armed = None;
        self.window = Window::default();
        self.line_fade = None;
        self.caret_reset = true;
        self.caret.start_blinking(now);
        self.live.jump(0.0);
        self.live_wpm = (0.0, 0.0);
        self.transition = Some(Transition::FadeIn { start: now });
    }

    /// Le restart rapide est refusé pendant un test long, sauf avec Shift
    /// (`quick-restart.ts`). Sinon l'écran disparaît en 125 ms avant le restart.
    fn try_restart(&mut self, now: f64, shifted: bool) {
        let running_long = matches!(self.screen, Screen::Test)
            && self.session.state() == SessionState::Running
            && self.session.spec().is_long();
        if running_long && !shifted {
            self.notifications.push(
                "Quick restart disabled in long tests. Use shift + tab.",
                Level::Notice,
                now,
            );
            self.restart_armed = None;
            return;
        }
        self.restart_armed = None;
        self.transition = Some(Transition::Restart { start: now });
    }

    fn finish(&mut self, now: f64) {
        let Some(result) = self.session.result(unix_ms()) else {
            return;
        };
        if let Some(reason) = result.invalid {
            self.notifications.push(
                format!("Test invalid - {}", invalid_label(reason)),
                Level::Notice,
                now,
            );
        }
        let outcome = match self.store.record(&result) {
            Ok(o) => Some(o),
            Err(e) => {
                self.notifications.push(
                    format!("could not save the result: {e}"),
                    Level::Error,
                    now,
                );
                None
            }
        };
        self.screen = Screen::Result(Box::new(ResultInfo { result, outcome }));
        self.transition = Some(Transition::ToResult { start: now });
    }

    fn check_finished(&mut self, now: f64) {
        if matches!(self.screen, Screen::Test) && self.session.state() == SessionState::Finished {
            self.finish(now);
        }
    }

    pub fn handle(&mut self, input: Input) {
        if input == Input::Interrupt {
            self.quit = true;
            return;
        }
        let Input::Key {
            key,
            phase,
            code,
            at,
        } = input
        else {
            return;
        };
        self.now = self.now.max(at);
        if phase == Phase::Release {
            if matches!(self.screen, Screen::Test) {
                self.session.key_up(code, at);
            }
            return;
        }
        if key == Key::Quit {
            self.quit = true;
            return;
        }
        // le restart dure ses deux fondus (`isTestRestarting`) : les touches sont ignorées
        self.advance_transitions(at);
        if matches!(
            self.transition,
            Some(Transition::Restart { .. } | Transition::FadeIn { .. })
        ) {
            return;
        }
        self.input_at = at;
        // En zen, Entrée insère un saut de ligne et Shift+Entrée termine le test :
        // ni l'une ni l'autre ne relance.
        let zen = self.session.spec().mode == Mode::Zen && matches!(self.screen, Screen::Test);
        let has_newlines = zen || self.session.has_newlines();
        let quick = self.store.config.str("quickRestart");
        let is_quick = match (quick, key) {
            ("tab", Key::Tab | Key::BackTab) | ("esc", Key::Esc) => true,
            ("enter", Key::Enter) => !has_newlines,
            ("enter", Key::ShiftEnter) => !zen,
            _ => false,
        };
        if is_quick {
            self.try_restart(at, matches!(key, Key::BackTab | Key::ShiftEnter));
            self.update_motion(at);
            return;
        }
        if quick == "off" {
            match (key, self.restart_armed) {
                (Key::Tab | Key::BackTab, _) => {
                    self.restart_armed = Some(key == Key::BackTab);
                    return;
                }
                (Key::Enter, Some(shifted)) => {
                    self.try_restart(at, shifted);
                    self.update_motion(at);
                    return;
                }
                _ => {}
            }
        }
        self.restart_armed = None;
        if !matches!(self.screen, Screen::Test) {
            return;
        }
        if phase == Phase::Press
            && matches!(
                key,
                Key::Char(_) | Key::Enter | Key::Backspace | Key::DeleteWord
            )
        {
            self.session.key_down(code, at);
        }
        match key {
            Key::Char(c) => {
                if self.session.insert(c, at) == InputOutcome::Finished {
                    self.finish(at);
                }
            }
            Key::Enter => {
                if self.session.insert('\n', at) == InputOutcome::Finished {
                    self.finish(at);
                }
            }
            Key::Backspace => {
                self.session.backspace(at);
            }
            Key::DeleteWord => {
                self.session.delete_word(at);
            }
            Key::ShiftEnter if self.session.spec().mode == Mode::Zen => {
                self.session.finish_zen(at);
            }
            _ => {}
        }
        self.check_finished(at);
        self.update_motion(at);
    }

    /// Fait avancer les fondus jusqu'à `now` : le restart a lieu à la fin du
    /// fondu de sortie, même si aucun tick n'est tombé pile à ce moment.
    fn advance_transitions(&mut self, now: f64) {
        if let Some(Transition::Restart { start }) = self.transition
            && now >= start + FADE_MS
        {
            self.restart(start + FADE_MS);
        }
        match self.transition {
            Some(Transition::ToResult { start }) if now >= start + 2.0 * FADE_MS => {
                self.transition = None;
            }
            Some(Transition::FadeIn { start }) if now >= start + FADE_MS => {
                self.transition = None;
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, now: f64) {
        self.now = self.now.max(now);
        let now = self.now;
        self.advance_transitions(now);
        if matches!(self.screen, Screen::Test) {
            if self.session.tick(now) {
                let s = self.session.live_stats();
                self.live_wpm = (s.wpm, s.raw);
            }
            self.check_finished(now);
        }
        if self.line_fade.is_some_and(|t| now >= t + FADE_MS) {
            self.line_fade = None;
        }
        self.notifications.expire(now);
        self.update_motion(now);
    }

    /// Recible les animations qui suivent l'état : focus mode, stats en direct,
    /// barre de progression, clignotement du caret.
    fn update_motion(&mut self, now: f64) {
        let on_test = matches!(self.screen, Screen::Test)
            && !matches!(self.transition, Some(Transition::Restart { .. }));
        let state = self.session.state();
        let running = on_test && state == SessionState::Running;
        self.chrome
            .retarget(if running { 0.0 } else { 1.0 }, now, FADE_MS, TAILWIND_EASE);
        self.logo.retarget(
            if running { 1.0 } else { 0.0 },
            now,
            2.0 * FADE_MS,
            TAILWIND_EASE,
        );
        self.live
            .retarget(if running { 1.0 } else { 0.0 }, now, FADE_MS, OUT2);
        match state {
            SessionState::Ready => self.caret.start_blinking(now),
            _ => self.caret.stop_blinking(),
        }
        let s = &self.session;
        let spec = s.spec();
        let (target, duration, easing) = match (spec.mode, spec.time_limit) {
            (Mode::Time, Some(limit)) if limit > 0 => {
                if state == SessionState::Ready {
                    (1.0, 0.0, OUT2)
                } else {
                    let next = f64::from(s.elapsed_seconds() + 1);
                    (
                        (1.0 - next / f64::from(limit)).max(0.0),
                        1000.0,
                        crate::anim::Easing::Linear,
                    )
                }
            }
            _ => {
                let total = self.progress_total();
                if state == SessionState::Finished {
                    (1.0, FADE_MS, OUT2)
                } else if total == 0 || state == SessionState::Ready {
                    (0.0, 0.0, OUT2)
                } else {
                    let pct = (s.active_index() as f64 / total as f64 * 100.0).floor();
                    (pct / 100.0, 250.0, OUT2)
                }
            }
        };
        self.bar.retarget(target, now, duration, easing);
    }

    /// Nombre de mots attendus (`wordsTotal` du site) ; 0 si illimité.
    fn progress_total(&self) -> usize {
        let spec = self.session.spec();
        match spec.mode {
            Mode::Words => spec.mode2.parse().unwrap_or(0),
            Mode::Zen | Mode::Time => 0,
            _ => self.session.words().len(),
        }
    }

    /// Prochain palier d'opacité du clignotement, s'il se dessine dans
    /// l'image (caret Kitty ou bloc) : 16 paliers par demi-période, soit 32
    /// réveils par seconde au plus au lieu d'une image à chaque 1/60 s.
    fn next_blink_step(&self, now: f64) -> Option<f64> {
        let drawn = matches!(self.screen, Screen::Test)
            && self.transition.is_none()
            && (matches!(self.renderer, CaretRenderer::Kitty(_))
                || self.caret_style() == CaretStyle::Block);
        let since = self.caret.blink_since().filter(|_| drawn)?;
        let step = 1000.0 / f64::from(2 * crate::kitty::LEVELS);
        Some(since + ((now - since) / step).floor() * step + step)
    }

    /// Une image change-t-elle sans frappe (hors clignotement) ?
    fn animating(&self, now: f64) -> bool {
        self.transition.is_some()
            || self.caret.is_moving(now)
            || self.line_fade.is_some()
            || self.chrome.is_running(now)
            || self.logo.is_running(now)
            || self.live.is_running(now)
            || (self.bar.is_running(now) && self.store.config.str("timerStyle") == "bar")
    }

    /// Prochain instant où l'écran doit changer sans frappe : image
    /// d'animation, tick du timer, fin d'une notification. `None` : attendre
    /// la prochaine touche (0 % de CPU).
    pub fn next_deadline(&self) -> Option<f64> {
        let tick = matches!(self.screen, Screen::Test)
            .then(|| self.session.next_tick_at())
            .flatten();
        let frame = self
            .animating(self.now)
            .then(|| self.frames.next_after(self.now));
        let blink = self.next_blink_step(self.now);
        [tick, self.notifications.next_expiry(), frame, blink]
            .into_iter()
            .flatten()
            .min_by(f64::total_cmp)
    }

    /// « time 30 · english ».
    pub fn summary(&self) -> String {
        let c = &self.store.config;
        let mode = match c.str("mode") {
            "time" => format!("time {}", c.int("time")),
            "words" => format!("words {}", c.int("words")),
            other => other.to_string(),
        };
        format!("{mode} · {}", self.session.spec().language)
    }

    /// Timer (`live-stats.ts`) : temps restant en time (« 01:15 » au-delà
    /// d'une minute), mots tapés sur le total ailleurs, mot actif en zen.
    pub fn timer(&self) -> String {
        let s = &self.session;
        let seconds = s.elapsed_seconds();
        match (s.spec().mode, s.spec().time_limit) {
            (Mode::Time, Some(limit)) if limit > 0 => {
                seconds_to_string(u64::from(limit.saturating_sub(seconds)))
            }
            (Mode::Time, _) => seconds_to_string(u64::from(seconds)),
            (Mode::Zen, _) => s.active_index().to_string(),
            _ => match self.progress_total() {
                0 => s.active_index().to_string(),
                total => format!("{}/{total}", s.active_index()),
            },
        }
    }

    /// Stats en direct selon `timerStyle`, `liveSpeedStyle`, `liveAccStyle`, `liveBurstStyle`.
    pub fn live_stats(&self) -> LiveStats {
        let c = &self.store.config;
        let factor = unit_factor(c.str("typingSpeedUnit"));
        let blind = c.str("blindMode") == "on";
        let s = &self.session;
        let speed = if blind {
            self.live_wpm.1
        } else {
            self.live_wpm.0
        };
        let acc = if blind { 100.0 } else { s.live_accuracy() };
        let burst = s.last_burst().unwrap_or(0.0);
        let items = [
            (c.str("timerStyle"), self.timer()),
            (
                c.str("liveSpeedStyle"),
                format!("{}", (speed * factor).round()),
            ),
            (c.str("liveAccStyle"), format!("{}%", acc.floor())),
            (
                c.str("liveBurstStyle"),
                format!("{}", (burst * factor).round()),
            ),
        ];
        LiveStats {
            items: items
                .into_iter()
                .enumerate()
                .map(|(k, (style, text))| LiveItem {
                    text,
                    style: Style3::from_config(style),
                    is_timer: k == 0,
                })
                .filter(|i| i.style != Style3::Off)
                .collect(),
        }
    }

    /// Curseur du terminal servant de caret (rendu `cell`, styles fins).
    pub fn caret_look(&self) -> Option<CaretLook> {
        if !matches!(self.screen, Screen::Test) || self.renderer != CaretRenderer::Cell {
            return None;
        }
        let shape = match self.caret_style() {
            CaretStyle::Off | CaretStyle::Block => return None,
            CaretStyle::Outline => CaretShape::Block,
            CaretStyle::Underline => CaretShape::Underline,
            CaretStyle::Bar => CaretShape::Bar,
        };
        Some(CaretLook {
            shape,
            blinking: self.caret.is_blinking(),
            rgb: self.palette.caret_rgb,
        })
    }

    /// Opacité du contenu (test ou résultat) pendant les fondus, et ce qu'il faut montrer.
    fn content(&self, now: f64) -> (bool, f64) {
        let on_result = matches!(self.screen, Screen::Result(_));
        let fade_in = |start: f64| OUT2.apply((now - start) / FADE_MS);
        match self.transition {
            Some(Transition::Restart { start }) => (on_result, 1.0 - fade_in(start)),
            Some(Transition::ToResult { start }) if now < start + FADE_MS => {
                (false, 1.0 - fade_in(start))
            }
            Some(Transition::ToResult { start }) => (true, fade_in(start + FADE_MS)),
            Some(Transition::FadeIn { start }) => (on_result, fade_in(start)),
            None => (on_result, 1.0),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, perf: Option<&Perf>) {
        let area = frame.area();
        self.caret_frame = None;
        self.scaled = None;
        if area.is_empty() {
            return;
        }
        let buf = frame.buffer_mut();
        fill_background(buf, area, &self.palette);
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            too_small(buf, area, &self.palette);
            return;
        }
        if area != self.last_area {
            // redimensionnement : le caret est replacé sans animation
            self.last_area = area;
            self.caret_reset = true;
            self.window = Window::default();
        }
        let now = self.now;
        let (show_result, opacity) = self.content(now);
        let content = self.palette.faded(opacity);
        let mut cursor = None;
        if show_result {
            if let Screen::Result(info) = &self.screen {
                let c = &self.store.config;
                ResultView {
                    result: &info.result,
                    outcome: info.outcome,
                    palette: &content,
                    unit: c.str("typingSpeedUnit"),
                    decimals: c.bool("alwaysShowDecimalPlaces"),
                    start_graphs_at_zero: c.bool("startGraphsAtZero"),
                    quote_source: self
                        .session
                        .spec()
                        .quote
                        .as_ref()
                        .map(|q| q.source.as_str()),
                }
                .render(buf, area);
            }
        } else {
            cursor = self.draw_test(buf, area, &content, opacity, now);
        }
        // logo et focus mode
        let logo = self.palette.over_bg(
            mix(
                self.palette.rgb.main,
                self.palette.rgb.sub,
                self.logo.value(now),
            ),
            1.0,
        );
        let chrome_opacity = if show_result {
            0.0
        } else {
            self.chrome.value(now) * opacity
        };
        Chrome {
            palette: &self.palette,
            logo,
            config: &self.store.config,
            opacity: chrome_opacity,
            tips: "restart",
        }
        .render(buf, area);
        self.notifications.render(buf, area, &self.palette);
        if let Some(p) = perf {
            buf.set_string(
                area.x + 1,
                area.bottom() - 1,
                p.summary(),
                Style::default().fg(self.palette.sub),
            );
        }
        // l'ancienne zone agrandie qui n'est plus réservée : tout y réécrire,
        // ce qui efface aussi les lettres agrandies restées à l'écran
        let region = self.scaled.as_ref().map(|s| s.region);
        if let Some(old) = self.last_scaled_region
            && region != Some(old)
        {
            let keep = region.unwrap_or_default();
            let old = old.intersection(area);
            for y in old.top()..old.bottom() {
                for x in old.left()..old.right() {
                    if !keep.contains(ratatui::layout::Position { x, y }) {
                        buf[(x, y)].set_diff_option(CellDiffOption::AlwaysUpdate);
                    }
                }
            }
        }
        self.last_scaled_region = region;
        if let Some(pos) = cursor {
            frame.set_cursor_position(pos);
        }
    }

    /// Mots, stats en direct et caret. Renvoie la position du curseur du
    /// terminal quand il sert de caret.
    fn draw_test(
        &mut self,
        buf: &mut Buffer,
        area: Rect,
        content: &Palette,
        opacity: f64,
        now: f64,
    ) -> Option<(u16, u16)> {
        let zen = self.session.spec().mode == Mode::Zen;
        let max_line_width = self
            .store
            .config
            .int("maxLineWidth")
            .clamp(0, i64::from(u16::MAX)) as u16;
        let scale = if self.text_sizing {
            scale_for(self.store.config.float("fontSize"))
        } else {
            1
        };
        let words = words_box(area, max_line_width, zen, scale);
        let layout = self.visible_lines(words, now);
        let c = &self.store.config;
        let last_line = self
            .line_fade
            .map(|start| content.faded(OUT2.apply((now - start) / FADE_MS)));
        self.scaled = WordsView {
            session: &self.session,
            palette: content,
            layout: &layout,
            words,
            flip_test_colors: c.bool("flipTestColors"),
            colorful_mode: c.bool("colorfulMode"),
            zen,
            last_line,
        }
        .render(buf);

        // badge de langue au-dessus des mots, effacé en focus mode comme la barre
        let badge = self.chrome.value(now) * opacity;
        if badge > 0.0 && words.top >= area.y + 3 {
            let name = self.session.spec().language.replace('_', " ");
            let x = area.x + area.width.saturating_sub(name.width() as u16) / 2;
            let fg = self.palette.over_bg(self.palette.rgb.sub, badge);
            buf.set_string(x, words.top - 2, &name, Style::default().fg(fg));
        }
        // stats en direct, visibles pendant la frappe
        let live = self.live.value(now) * opacity;
        if live > 0.0 {
            let base = c.str("timerOpacity").parse::<f64>().unwrap_or(1.0);
            let color = self
                .palette
                .over_bg(timer_rgb(&self.palette, c.str("timerColor")), base * live);
            self.live_stats().render(buf, area, words, color);
            if c.str("timerStyle") == "bar" && !zen {
                render_bar(buf, area, self.bar.value(now), color);
            }
        }
        self.place_caret(buf, &layout, words, content, opacity, now)
    }

    /// Lignes visibles, la première en haut de la zone de mots. Le caret reste
    /// sur la 2e ligne une fois le premier saut passé : la ligne du haut est
    /// alors retirée (`lineJump`).
    fn visible_lines(&mut self, words: WordsBox, now: f64) -> Layout {
        let s = &self.session;
        let active = s.active_index();
        if words.width != self.window.width || active < self.window.start {
            self.window = Window {
                width: words.width,
                start: 0,
            };
        }
        let mut layout = layout_window(
            s.words(),
            s.inputs(),
            active,
            words.width,
            self.window.start,
            usize::from(words.lines),
        );
        let first = layout.first_visible();
        if first > 0 && first < layout.lines.len() {
            self.window.start = layout.lines[first][0].index;
            layout.lines.drain(..first);
            layout.caret.0 -= first;
            if self.store.config.bool("smoothLineScroll") && !self.caret_reset {
                self.line_fade = Some(now);
            }
        }
        layout
    }

    /// Fait glisser le caret vers sa cible et le dessine : teinte des cases
    /// pour le style bloc, curseur du terminal ou image Kitty pour les autres.
    fn place_caret(
        &mut self,
        buf: &mut Buffer,
        layout: &Layout,
        words: WordsBox,
        content: &Palette,
        opacity: f64,
        now: f64,
    ) -> Option<(u16, u16)> {
        let style = self.caret_style();
        let s = &self.session;
        let restarting = matches!(self.transition, Some(Transition::Restart { .. }));
        if style == CaretStyle::Off
            || s.state() == SessionState::Finished
            || s.words().is_empty()
            || restarting
        {
            return None;
        }
        let (line, col) = layout.caret;
        let width = if style.is_full_width() && s.spec().mode != Mode::Zen {
            let a = s.active_index();
            let typed = letters(s.input(a)).chars().count();
            letters(s.word(a))
                .chars()
                .nth(typed)
                .map_or(1, char_width)
                .max(1)
        } else {
            1
        };
        let k = f64::from(words.scale);
        let target = CaretTarget {
            x: f64::from(words.left) + f64::from(col) * k,
            y: f64::from(words.top) + line as f64 * k,
            width: f64::from(width) * k,
            height: k,
        };
        if self.caret_reset {
            self.caret.jump(target);
            self.caret_reset = false;
        } else if target != self.caret.target() {
            // le curseur du terminal ne montre pas de position intermédiaire :
            // un glissement n'y ajouterait que du retard
            let ms = if self.renderer == CaretRenderer::Cell && style != CaretStyle::Block {
                0.0
            } else {
                smooth_caret_ms(self.store.config.str("smoothCaret"))
            };
            self.caret.go_to(target, self.input_at.min(now), ms);
        }
        let smooth_blink = self.store.config.str("smoothCaret") != "off";
        let mut f = self.caret.frame(style, now, smooth_blink);
        f.opacity *= opacity;
        match style {
            CaretStyle::Block if self.scaled.is_some() => {
                // lettres agrandies : on teinte le fond de chaque lettre couverte
                let caret = self.palette.rgb.caret;
                let st = self.scaled.as_mut()?;
                let s = st.scale;
                let col0 = ((f.x - f64::from(words.left)) / f64::from(s))
                    .floor()
                    .max(0.0) as u16;
                let row0 = ((f.y - f64::from(words.top)) / f64::from(s))
                    .floor()
                    .max(0.0) as u16;
                for row in row0..=row0 + 1 {
                    for col in col0..=col0 + (f.width / f64::from(s)).ceil() as u16 {
                        let (x, y) = (words.left + col * s, words.top + row * s);
                        let k = coverage_box(&f, x, y, s) * f.opacity;
                        if k <= 0.0 || x + s > st.region.right() || y + s > st.region.bottom() {
                            continue;
                        }
                        let bg = content.over_bg(caret, k);
                        match st.cells.iter_mut().find(|c| c.x == x && c.y == y) {
                            Some(c) => c.style = c.style.bg(bg),
                            None => st.cells.push(ScaledCell {
                                x,
                                y,
                                ch: ' ',
                                style: Style::default().bg(bg),
                            }),
                        }
                    }
                }
                None
            }
            CaretStyle::Block => {
                // pavé derrière la lettre : fond teinté au prorata de la case couverte
                let (x0, y0) = (f.x.floor().max(0.0) as u16, f.y.floor().max(0.0) as u16);
                for row in y0..=y0.saturating_add(1) {
                    for col in x0..=x0.saturating_add(f.width.ceil() as u16) {
                        if row >= area_bottom(buf) || col >= buf.area.right() {
                            continue;
                        }
                        let k = coverage(&f, col, row) * f.opacity;
                        if k > 0.0 {
                            let cell = &mut buf[(col, row)];
                            cell.set_bg(content.over_bg(self.palette.rgb.caret, k));
                        }
                    }
                }
                None
            }
            _ => match self.renderer {
                CaretRenderer::Kitty(_) => {
                    self.caret_frame = Some(f);
                    None
                }
                CaretRenderer::Cell => Some((f.x.round() as u16, f.y.round() as u16)),
            },
        }
    }
}

fn area_bottom(buf: &Buffer) -> u16 {
    buf.area.bottom()
}
```

`crates/fasttype-tui/src/terminal.rs` (version complète) :
```rust
//! Entrée et sortie du mode plein écran, restauration garantie (y compris en
//! cas de panique), et tampon qui envoie chaque image en une seule écriture.

use crate::app::{CaretLook, CaretShape};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    supports_keyboard_enhancement,
};
use crossterm::{execute, queue};
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

static KEYBOARD_PUSHED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static KITTY_IMAGES: AtomicBool = AtomicBool::new(false);
static GRAPHICS: AtomicBool = AtomicBool::new(false);

/// Le terminal a répondu OK à la sonde graphique Kitty au démarrage.
pub fn graphics_supported() -> bool {
    GRAPHICS.load(Ordering::SeqCst)
}

/// Des images Kitty (caret) seront affichées : `restore` les supprimera.
pub fn mark_kitty_images() {
    KITTY_IMAGES.store(true, Ordering::SeqCst);
}

static HOOK: Once = Once::new();
static TEXT_SIZING: AtomicBool = AtomicBool::new(false);

/// Le terminal a montré au démarrage qu'il agrandit le texte (OSC 66).
pub fn text_sizing_supported() -> bool {
    TEXT_SIZING.load(Ordering::SeqCst)
}

/// Envoie une sonde et lit la réponse directement sur l'entrée, avant que le
/// thread clavier ne la lise : 300 ms au plus (les terminaux répondent bien avant).
#[cfg(unix)]
fn probe(
    out: &mut impl Write,
    query: &[u8],
    answer: fn(&[u8]) -> Option<bool>,
) -> io::Result<bool> {
    use std::time::{Duration, Instant};
    out.write_all(query)?;
    out.flush()?;
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut got = Vec::new();
    loop {
        if let Some(ok) = answer(&got) {
            return Ok(ok);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(false);
        }
        let mut fds = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY : un seul descripteur valide, tampon local de taille connue.
        let ready = unsafe { libc::poll(&mut fds, 1, left.as_millis() as libc::c_int) };
        if ready <= 0 {
            return Ok(false);
        }
        let mut buf = [0u8; 256];
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            return Ok(false);
        }
        got.extend_from_slice(&buf[..n as usize]);
    }
}

#[cfg(not(unix))]
fn probe(
    _out: &mut impl Write,
    _query: &[u8],
    _answer: fn(&[u8]) -> Option<bool>,
) -> io::Result<bool> {
    Ok(false)
}

/// Accumule une image entière ; `present` l'envoie au terminal en un seul
/// `write`. Les clones partagent le même tampon : l'un est donné au backend
/// ratatui, l'autre garde la boucle pour envoyer l'image.
#[derive(Clone)]
pub struct FrameWriter {
    buf: Rc<RefCell<Vec<u8>>>,
}

impl Default for FrameWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameWriter {
    pub fn new() -> Self {
        Self {
            buf: Rc::new(RefCell::new(Vec::with_capacity(64 * 1024))),
        }
    }

    pub fn pending(&self) -> Vec<u8> {
        self.buf.borrow().clone()
    }

    pub fn present(&self, out: &mut impl Write) -> io::Result<()> {
        let mut buf = self.buf.borrow_mut();
        out.write_all(&buf)?;
        out.flush()?;
        buf.clear();
        Ok(())
    }
}

impl Write for FrameWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.borrow_mut().extend_from_slice(data);
        Ok(data.len())
    }

    /// Ne fait rien : ratatui appelle `flush` à la fin de chaque dessin, mais
    /// l'image n'est envoyée qu'avec `present`, encadrée par la sortie synchronisée.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Séquences qui donnent au curseur du terminal la forme et la couleur du caret.
pub fn queue_caret_look(out: &mut impl Write, look: CaretLook) -> io::Result<()> {
    let style = match (look.shape, look.blinking) {
        (CaretShape::Bar, true) => SetCursorStyle::BlinkingBar,
        (CaretShape::Bar, false) => SetCursorStyle::SteadyBar,
        (CaretShape::Block, true) => SetCursorStyle::BlinkingBlock,
        (CaretShape::Block, false) => SetCursorStyle::SteadyBlock,
        (CaretShape::Underline, true) => SetCursorStyle::BlinkingUnderScore,
        (CaretShape::Underline, false) => SetCursorStyle::SteadyUnderScore,
    };
    queue!(out, style)?;
    let (r, g, b) = look.rgb;
    // OSC 12 : couleur du curseur
    write!(out, "\x1b]12;#{r:02x}{g:02x}{b:02x}\x07")
}

/// Remet le terminal dans son état d'origine. Sans effet si déjà fait.
pub fn restore() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let mut out = io::stdout();
    if KITTY_IMAGES.swap(false, Ordering::SeqCst) {
        let _ = out.write_all(crate::kitty::DELETE_ALL);
    }
    if KEYBOARD_PUSHED.swap(false, Ordering::SeqCst) {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        out,
        SetCursorStyle::DefaultUserShape,
        crossterm::cursor::Show,
        LeaveAlternateScreen
    );
    // OSC 112 : couleur du curseur par défaut
    let _ = out.write_all(b"\x1b]112\x07");
    let _ = out.flush();
    let _ = disable_raw_mode();
}

/// Mode plein écran actif tant que la valeur vit.
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<TerminalGuard> {
        HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore();
                previous(info);
            }));
        });
        enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        // le garde existe dès le mode raw : si la suite échoue, son `drop` restaure
        let guard = TerminalGuard;
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen)?;
        // avant toute autre lecture de l'entrée (protocole clavier, thread clavier)
        let graphics = probe(&mut out, crate::kitty::PROBE, crate::kitty::probe_answer)?;
        GRAPHICS.store(graphics, Ordering::SeqCst);
        let sizing = probe(&mut out, crate::sized::PROBE, crate::sized::probe_answer)?;
        TEXT_SIZING.store(sizing, Ordering::SeqCst);
        if matches!(supports_keyboard_enhancement(), Ok(true)) {
            execute!(
                out,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            )?;
            KEYBOARD_PUSHED.store(true, Ordering::SeqCst);
        }
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}
```

`crates/fasttype-tui/src/runner.rs` (version complète) :
```rust
//! Boucle principale : attend une touche ou la prochaine échéance (image
//! d'animation, tick du timer), applique toutes les touches en attente, puis
//! dessine aussitôt une seule image.

use crate::app::{App, CaretLook};
#[cfg(unix)]
use crate::input::spawn_signal_watcher;
use crate::input::{Input, spawn_reader};
use crate::kitty::{CaretRenderer, CellPx, DELETE_ALL, KittyCaret};
use crate::perf::Perf;
use crate::sized::ScaledText;
use crate::terminal::{
    FrameWriter, TerminalGuard, graphics_supported, mark_kitty_images, queue_caret_look,
    text_sizing_supported,
};
use crate::theme::ColorMode;
use crossterm::queue;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use fasttype_core::clock::{Clock, SystemClock};
use fasttype_store::Store;
use fasttype_store::paths::Paths;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Size;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

pub struct Options {
    pub perf: bool,
    /// Images d'animation par seconde (60, 120 ou 144).
    pub fps: u32,
}

/// `--perf` et `--fps N` (60, 120 ou 144).
pub fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut opts = Options {
        perf: false,
        fps: 60,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--perf" => opts.perf = true,
            "--fps" => {
                opts.fps = match it.next().map(String::as_str) {
                    Some("60") => 60,
                    Some("120") => 120,
                    Some("144") => 144,
                    _ => return Err("--fps takes 60, 120 or 144".to_string()),
                }
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    Ok(opts)
}

type Term = Terminal<CrosstermBackend<FrameWriter>>;

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

/// Taille d'une case en pixels, si le terminal la donne.
fn cell_px() -> Option<CellPx> {
    let w = crossterm::terminal::window_size().ok()?;
    CellPx::from_window(w.columns, w.rows, w.width, w.height)
}

/// Ce qui survit d'une image à l'autre.
struct Output {
    term: Term,
    frame: FrameWriter,
    last_look: Option<CaretLook>,
    kitty: Option<KittyCaret>,
    size: Size,
    /// Mots agrandis déjà à l'écran : rien à réécrire s'ils n'ont pas changé.
    scaled: Option<ScaledText>,
}

impl Output {
    /// Dessine une image et l'envoie en une écriture, encadrée par la sortie
    /// synchronisée (mode 2026) : le terminal ne montre jamais d'image partielle.
    fn present(&mut self, app: &mut App, perf: &Perf) -> io::Result<()> {
        let size = self.term.size()?;
        if size != self.size {
            self.size = size;
            // ratatui efface l'écran : les mots agrandis sont à réécrire
            self.scaled = None;
            // la taille des cases a pu changer (zoom) : images à refaire
            if let Some(k) = &mut self.kitty {
                match cell_px() {
                    Some(cell) => {
                        self.term.backend_mut().write_all(DELETE_ALL)?;
                        *k = KittyCaret::new(cell);
                    }
                    // l'écran va être effacé : replacer l'image au prochain dessin
                    None => k.invalidate(),
                }
            }
        }
        queue!(self.term.backend_mut(), BeginSynchronizedUpdate)?;
        let look = app.caret_look();
        if look != self.last_look {
            if let Some(l) = look {
                queue_caret_look(self.term.backend_mut(), l)?;
            }
            self.last_look = look;
        }
        self.term
            .draw(|f| app.draw(f, perf.enabled.then_some(perf)))?;
        let scaled = app.scaled_text();
        if scaled != self.scaled.as_ref() {
            if let Some(t) = scaled {
                t.write(self.term.backend_mut())?;
            }
            self.scaled = scaled.cloned();
        }
        if let Some(k) = &mut self.kitty {
            k.draw(
                self.term.backend_mut(),
                app.caret_frame(),
                app.palette().caret_rgb,
            )?;
        }
        queue!(self.term.backend_mut(), EndSynchronizedUpdate)?;
        self.frame.present(&mut io::stdout())
    }
}

/// Applique une touche puis toutes celles déjà reçues (rafale), dans l'ordre,
/// en s'arrêtant à Ctrl+C. Renvoie l'horodatage de la plus ancienne touche.
pub fn apply_burst(app: &mut App, first: Input, rest: impl Iterator<Item = Input>) -> Option<f64> {
    let mut oldest = first.at();
    app.handle(first);
    for input in rest {
        if app.quit {
            break;
        }
        oldest = oldest.or(input.at());
        app.handle(input);
    }
    oldest
}

/// Lance l'interface ; renvoie les mesures de fluidité de la session.
pub fn run(opts: Options) -> io::Result<Perf> {
    let paths = Paths::from_system().ok_or_else(|| io::Error::other("HOME is not set"))?;
    let store = Store::open(paths);
    let clock = Arc::new(SystemClock::new());
    let color_mode = ColorMode::detect(|k| std::env::var(k).ok());
    let _guard = TerminalGuard::enter()?;
    let renderer =
        CaretRenderer::detect(|k| std::env::var(k).ok(), cell_px(), graphics_supported());
    let (tx, rx) = mpsc::sync_channel::<Input>(4096);
    #[cfg(unix)]
    spawn_signal_watcher(tx.clone())?;
    spawn_reader(Arc::clone(&clock), tx);
    let frame = FrameWriter::new();
    let term = Terminal::new(CrosstermBackend::new(frame.clone()))?;
    let kitty = match renderer {
        CaretRenderer::Kitty(cell) => {
            mark_kitty_images();
            Some(KittyCaret::new(cell))
        }
        CaretRenderer::Cell => None,
    };
    let mut out = Output {
        size: term.size()?,
        term,
        frame,
        last_look: None,
        kitty,
        scaled: None,
    };
    let mut app = App::new(store, color_mode, clock.now_ms(), seed());
    app.set_caret_renderer(renderer);
    app.set_text_sizing(text_sizing_supported());
    app.set_fps(opts.fps);
    let mut perf = Perf::new(opts.perf);
    app.tick(clock.now_ms());
    out.present(&mut app, &perf)?;

    while !app.quit {
        let now = clock.now_ms();
        let first = match app.next_deadline() {
            Some(d) => rx.recv_timeout(Duration::from_secs_f64((d - now).max(0.0) / 1000.0)),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let mut oldest_key = None;
        match first {
            Ok(input) => oldest_key = apply_burst(&mut app, input, rx.try_iter()),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if app.quit {
            break;
        }
        app.tick(clock.now_ms());
        let start = clock.now_ms();
        out.present(&mut app, &perf)?;
        let done = clock.now_ms();
        perf.frame.record(done - start);
        if let Some(t) = oldest_key {
            perf.input_to_flush.record(done - t);
        }
    }
    Ok(perf)
}
```

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (7 dans `scaled`, 9 dans `view`).

- [ ] **Step 6 : test pty et benchmark**

`scripts/pty_smoke.py` (version complète) :
```python
"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf] [--kitty] [--sized] [--sigterm]
  --kitty    le terminal répond OK à la sonde graphique Kitty (cases de 10 × 20 pixels)
  --sized    le terminal agrandit le texte (OSC 66, Kitty ≥ 0.40)
  --sigterm  quitte par SIGTERM au lieu de Ctrl+C"""
import fcntl, os, pty, re, select, signal, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
flags = sys.argv[3:]
perf, kitty, sigterm = "--perf" in flags, "--kitty" in flags, "--sigterm" in flags
sized = "--sized" in flags
os.makedirs(f"{home}/.config/fasttype", exist_ok=True)
with open(f"{home}/.config/fasttype/config.toml", "w") as f:
    f.write('mode = "custom"\n')

pid, fd = pty.fork()
if pid == 0:
    os.environ.update({"HOME": home, "TERM": "xterm-256color", "COLORTERM": "truecolor"})
    os.environ.pop("FASTTYPE_CARET", None)
    os.environ.pop("XDG_CONFIG_HOME", None)
    os.environ.pop("XDG_DATA_HOME", None)
    os.execv(binary, [binary] + (["--perf"] if perf else []))

# 30 lignes × 100 colonnes ; 1000 × 600 pixels → cases de 10 × 20
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 1000, 600))
out = bytearray()

def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.02)
        if not r:
            continue
        try:
            data = os.read(fd, 65536)
        except OSError:
            return
        if not data:
            return
        out.extend(data)
        # sonde graphique : OK seulement si l'on joue un terminal Kitty
        if kitty and b"a=q" in data:
            os.write(fd, b"\x1b_Gi=31;OK\x1b\\")
        # réponses d'un terminal sans protocole clavier Kitty
        cpr = data.count(b"\x1b[6n")
        if sized and cpr == 2 and b"\x1b]66;" in data:
            os.write(fd, b"\x1b[1;1R\x1b[1;3R")  # l'espace agrandi a avancé de 2 cases
        elif cpr:
            os.write(fd, b"\x1b[1;1R" * cpr)
        if b"\x1b[?u" in data or b"\x1b[c" in data:
            os.write(fd, b"\x1b[?62c")

def plain(b):
    t = b.decode("utf-8", "replace")
    t = re.sub(r"\x1b_G[^\x1b]*\x1b\\\\", " ", t)
    t = re.sub(r"\x1b\[[0-9;?<>]*[ A-Za-z]|\x1b\][^\x07]*\x07", " ", t)
    return re.sub(r"\s+", " ", t)

pump(1.0)
for ch in "The quick brown fox jumps over the lazy dog":
    os.write(fd, ch.encode())
    pump(0.08)
pump(0.8)
screen = plain(bytes(out))
if sigterm:
    os.kill(pid, signal.SIGTERM)
else:
    os.write(fd, b"\x03")  # Ctrl+C
pump(1.0)
_, status = os.waitpid(pid, 0)
print("exit", os.waitstatus_to_exitcode(status))
print("result screen:", "test type custom english" in screen)
print("alt screen left:", b"\x1b[?1049l" in out)
print("sync output used:", b"\x1b[?2026h" in out and b"\x1b[?2026l" in out)
print("cursor color reset:", b"\x1b]112\x07" in out)
if kitty:
    print("kitty caret placed:", b"\x1b_Ga=p," in out)
    print("kitty images deleted:", out.rstrip().find(b"\x1b_Ga=d,d=A,q=2\x1b\\") > out.find(b"\x1b_Ga=p,"))
if sized:
    print("scaled words written:", b"\x1b]66;s=2;" in out)
if perf:
    found = re.findall(r"key→flush p50 ([0-9.]+) p99 ([0-9.]+) ms · frame p99 ([0-9.]+) ms · n (\d+)", bytes(out).decode("utf-8", "replace"))
    print("perf (p50, p99, frame p99, n):", found[-1] if found else None)
```

Run: `cargo build --release -p fasttype-tui && for f in "--perf" "--kitty --sized --perf" "--sigterm"; do rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home $f; done`
(`$f` est volontairement sans guillemets : `--kitty --sized --perf` doit arriver en trois arguments.)
Expected : pour chacun, `exit 0`, `result screen: True`, `alt screen left: True`, `sync output used: True` et `cursor color reset: True`. En plus :
- avec `--kitty --sized` : `kitty caret placed: True`, `kitty images deleted: True` et `scaled words written: True` ;
- p50 sous 0,5 ms. Noter le p99 dans le ledger.

Run: `cargo bench -p fasttype-tui --bench frame 2>&1 | grep -A1 -E "^(frame|key)"`
Expected : les trois benchmarks sous 1 ms (mesurés vers 76 µs).

- [ ] **Step 7 : vérification finale et essai à la main**

Run: `cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, aucun avertissement.

Essai à la main, dans Kitty 0.40 ou plus récent : `cargo run --release -p fasttype-tui`. Les mots doivent être en double taille et le caret à leur hauteur. Recommencer avec `font_size = 3` dans `config.toml`, puis dans un autre terminal, où la taille doit rester normale. Les tests automatiques ne jugent pas l'aspect : le noter dans le ledger, et signaler à l'utilisateur que l'essai visuel lui revient.

- [ ] **Step 8 : commit**

```bash
git add crates/fasttype-tui scripts
git commit -m "feat(tui): mots agrandis (OSC 66), barre de config, badge de langue, source de la citation"
```
