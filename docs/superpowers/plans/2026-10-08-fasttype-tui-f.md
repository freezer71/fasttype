# fasttype-tui, partie F : mots agrandis dans Ghostty et WezTerm — plan d'implémentation (plan 4f)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Afficher les mots du test en grand dans Ghostty, le terminal de l'utilisateur, et dans WezTerm. Ces terminaux affichent les images Kitty mais pas le texte agrandi (OSC 66). Chaque lettre est donc dessinée en image avec la police du site, Roboto Mono.

**Architecture:**
- **`glyphs.rs`** rend une lettre avec `fontdue` dans une image RGBA de `scale × scale` cases. La police est Roboto Mono, embarquée par `include_bytes!`.
  - L'image est transmise une seule fois par (lettre, couleur, soulignement), compressée en zlib (`o=z`, `miniz_oxide`), puis placée à chaque position avec un identifiant de placement unique.
  - `GlyphText::write` ne replace que les lettres qui changent. Il repart de zéro si la zone, l'échelle, le fond, la partie couverte par la palette ou le thème changent.
  - Les couleurs sont ramenées aux 10 couleurs du thème : un fondu ne crée pas une image par teinte. Une lettre qui retombe sur la couleur du fond n'est pas dessinée.
- **La boucle choisit le rendu des mots agrandis :**
  - OSC 66 si la sonde de taille du texte répond (Kitty ≥ 0.40) ;
  - sinon des images de lettres si le caret est en image Kitty (Ghostty, WezTerm) ;
  - sinon la taille de base.
- **`App`** reste sans E/S. Elle reçoit la liste des caractères que le rendu sait dessiner (`set_scalable_chars`). Les mots restent à la taille de base si une lettre des 200 mots autour de la fenêtre manque à la police (chinois, japonais, hébreu…).
- **Le caret Kitty** passe au-dessus des lettres (`z=1`).

**Tech Stack:** Rust 1.97 (édition 2024), `fontdue` 0.9.4 (MIT/Apache/Zlib), `miniz_oxide` 0.9.1 (MIT/Zlib/Apache), Roboto Mono (SIL Open Font License 1.1), protocole graphique de Kitty (`o=z`, `d=r`/`d=R`, `z`, `C=1`, tous gérés par Ghostty), Python 3 pour le test pty.

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§5.3, ajout « Taille des mots » de cette partie). Les plans 4a à 4e sont livrés sur `main`, ainsi que « Tab seul relance » et la largeur des mots.

**Code vérifié avant rédaction :** tout le code de ce plan a été assemblé et exécuté dans une copie de travail jetable.
- **Tests :** 397 tests du workspace au vert, et clippy ne signale rien.
- **Test pty `--kitty`** (images Kitty sans OSC 66, comme Ghostty) : les lettres sont placées en images.
- **Octets envoyés :**
  - première image : 18 Ko (28 lettres transmises compressées, au lieu de 124 Ko sans compression) ;
  - par frappe : environ 540 octets ;
  - fin du test : aucune nouvelle transmission.
- **Coût d'une nouvelle lettre** (dessin et compression) : 46 µs avec des cases de 10 × 20 px, 110 µs avec des cases de 18 × 40 px (écran Retina), une seule fois par couleur.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Commentaires en français.
- **La police et sa licence vivent dans `assets/fonts/`.** `NOTICE` crédite Roboto Mono (SIL OFL 1.1). Le binaire de la police est vérifié par SHA-256.
- **Kitty :**
  - toutes les commandes graphiques portent `q=2` ;
  - les identifiants d'images de lettres vont de 1 000 000 à 1 999 999, ceux du caret restent en dessous ;
  - les lettres sont retirées par plage (`d=r`), et libérées au changement de thème (`d=R`).
- **Fluidité :**
  - une lettre n'est dessinée et transmise qu'une fois par couleur du thème ;
  - une frappe ne replace que les lettres qui changent ;
  - le benchmark reste sous 1 ms.
- `App` ne fait aucune E/S terminal ; seul `runner.rs` écrit les images.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace, tous au vert.

**Choix assumés :**
- **Couleurs des lettres :** ramenées aux 10 couleurs du thème. Le fondu de 125 ms des mots devient un changement net, et sous le voile de la palette les mots prennent la couleur du thème la plus proche, au lieu d'une teinte assombrie.
- **Police unique :** Roboto Mono, sans choix de police (`fontFamily`) pour l'instant.
- **Kitty garde OSC 66**, avec la police du terminal : rendu natif, sans images.

## Review Focus

1. **Lettres qui restent à l'écran** au passage test → résultat, à l'ouverture de la palette (trou dans la zone), au changement de thème ou au redimensionnement. Tests : `hiding_moving_and_retheming`, `letters_are_sent_once_and_only_changes_are_redrawn` (tâche 1). Pour le redimensionnement, vérifier par lecture que `Output::present` recrée `GlyphText` après `DELETE_ALL`.
2. **Langue dont la police n'a pas les lettres**, y compris une lettre tapée en zen. Elle doit rester à la taille de base, sans carré vide. Test : `languages_outside_the_font_stay_normal_size` (tâche 2).
3. **Fondus et voile de la palette.** Ils ne doivent pas multiplier les images transmises. Tests : `letters_are_sent_once_and_only_changes_are_redrawn` (lettre fondue ramenée au thème) et `a_letter_faded_to_the_background_is_not_drawn` (tâche 1).
4. **Caret au-dessus des lettres** (`z=1`), et jamais retiré par la suppression des lettres. Tests : `placement_moves_the_cursor_then_places` (tâche 1). Les identifiants des lettres sont hors de la plage du caret par construction.
5. **Taille des images envoyées.** La compression doit garder une lettre sous 1,2 Ko. Test : `images_are_compressed_and_small` (tâche 1).

---

## Structure des fichiers

```
Cargo.toml                                  + fontdue, miniz_oxide
.gitattributes                              + assets/fonts/*.ttf binary
assets/fonts/RobotoMono-Regular.ttf         police (SHA-256 vérifié)
assets/fonts/RobotoMono-OFL.txt             sa licence
NOTICE, README.md                           crédit de la police ; terminaux
docs/superpowers/specs/…-v1-design.md       ajout « Taille des mots » et « Restart »
crates/fasttype-tui/
├── Cargo.toml                              + fontdue, miniz_oxide
├── src/glyphs.rs                           police, rendu, compression, GlyphText
├── src/kitty.rs                            caret en z=1
├── src/app.rs                              set_scalable_chars, window_is_scalable
├── src/runner.rs                           choix du rendu des mots agrandis
├── tests/glyphs.rs                         nouveau
├── tests/kitty.rs                          z=1
└── tests/scaled.rs                         + languages_outside_the_font_stay_normal_size
scripts/pty_smoke.py                        + « glyph letters placed », FASTTYPE_DUMP
```

---

### Task 1 : lettres en images (police, rendu, compression, placements)

**Files:**
- Create: `assets/fonts/RobotoMono-Regular.ttf`, `assets/fonts/RobotoMono-OFL.txt`, `crates/fasttype-tui/src/glyphs.rs`
- Modify: `Cargo.toml`, `crates/fasttype-tui/Cargo.toml`, `.gitattributes`, `crates/fasttype-tui/src/lib.rs` (`pub mod glyphs;` avant `input`), `crates/fasttype-tui/src/kitty.rs`
- Test: `crates/fasttype-tui/tests/glyphs.rs`, `crates/fasttype-tui/tests/kitty.rs`

**Interfaces:**
- Consumes : `kitty::{CellPx, base64}`, `sized::{ScaledCell, ScaledText}`, `theme::{Rgb, RgbColors, xterm_rgb}`.
- Produces :
  - `glyphs::{FIRST_ID, has_glyph(char) -> bool, render_glyph(c, cell, scale, width_cells, fg, underline) -> (w, h, rgba), transmit_compressed(id, w, h, rgba) -> Vec<u8>}` ;
  - `GlyphText::{new(CellPx), write(out, &ScaledText, prev: Option<&ScaledText>, theme: &RgbColors), clear(out)}` ;
  - `kitty::place` ajoute `z=1`.

- [ ] **Step 1 : la police et les dépendances**

```bash
mkdir -p assets/fonts
curl -sSL -o assets/fonts/RobotoMono-Regular.ttf https://github.com/googlefonts/RobotoMono/raw/main/fonts/ttf/RobotoMono-Regular.ttf
curl -sSL -o assets/fonts/RobotoMono-OFL.txt https://raw.githubusercontent.com/googlefonts/RobotoMono/main/OFL.txt
shasum -a 256 assets/fonts/RobotoMono-Regular.ttf
echo "assets/fonts/*.ttf binary" >> .gitattributes
```
Expected : `af0bff7599c3df3831755c16e39b3c496df74b8c8d8a1161b14dc8461be17cb4`. Si l'empreinte diffère, s'arrêter : ce n'est pas la police vérifiée.

Dans `Cargo.toml` (racine), sous `[workspace.dependencies]`, après `signal-hook` et `libc`, ajouter :
```toml
fontdue = "0.9.4"
miniz_oxide = "0.9.1"
```
Dans `crates/fasttype-tui/Cargo.toml`, après `toml.workspace = true`, ajouter `fontdue.workspace = true` et `miniz_oxide.workspace = true`.

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/glyphs.rs` :
```rust
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_tui::glyphs::{FIRST_ID, GlyphText, has_glyph, render_glyph};
use fasttype_tui::kitty::CellPx;
use fasttype_tui::sized::{ScaledCell, ScaledText};
use fasttype_tui::theme::{ColorMode, Palette};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

const CELL: CellPx = CellPx { w: 10, h: 20 };

fn palette() -> Palette {
    Palette::from_theme(theme(DEFAULT_THEME).unwrap(), ColorMode::TrueColor)
}

fn cell(x: u16, ch: char, fg: Color) -> ScaledCell {
    ScaledCell {
        x,
        y: 10,
        ch,
        style: Style::default().fg(fg),
    }
}

fn layer(cells: Vec<ScaledCell>, bg: Color) -> ScaledText {
    ScaledText {
        scale: 2,
        region: Rect::new(0, 10, 40, 2),
        bg,
        cells,
        hole: None,
    }
}

fn text(out: Vec<u8>) -> String {
    String::from_utf8(out).unwrap()
}

#[test]
fn the_embedded_font_covers_latin_and_cyrillic() {
    for c in ['a', 'Z', 'é', 'ç', 'ß', 'ж', 'λ', ' ', '\'', '?'] {
        assert!(has_glyph(c), "{c}");
    }
    for c in ['日', 'ש', 'ก'] {
        assert!(!has_glyph(c), "{c} : pas dans Roboto Mono");
    }
}

#[test]
fn a_letter_fills_its_scaled_block() {
    let (w, h, px) = render_glyph('a', CELL, 2, 1, (255, 255, 255), None);
    assert_eq!((w, h), (20, 40), "2 × 2 cases de 10 × 20 pixels");
    let alpha = |x: u32, y: u32| px[((y * w + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 0), 0, "fond transparent");
    assert!(
        (0..w).any(|x| (0..h).any(|y| alpha(x, y) == 255)),
        "la lettre est dessinée"
    );
    let (w, _, _) = render_glyph('a', CELL, 2, 2, (255, 255, 255), None);
    assert_eq!(w, 40, "lettre large : deux fois plus");
    // le soulignement des mots faux tient dans le bloc
    let (_, _, px) = render_glyph('a', CELL, 2, 1, (255, 255, 255), Some((255, 0, 0)));
    let red_row = (0..40u32).find(|&y| {
        let k = ((y * 20 + 1) * 4) as usize;
        px[k] == 255 && px[k + 1] == 0 && px[k + 3] == 255
    });
    assert!(red_row.is_some_and(|y| y > 25 && y < 40), "{red_row:?}");
}

#[test]
fn letters_are_sent_once_and_only_changes_are_redrawn() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let first = layer(vec![cell(0, 'a', p.text), cell(2, 'b', p.sub)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &first, None, &p.rgb).unwrap();
    let s = text(out);
    assert!(
        s.starts_with("\x1b7") && s.ends_with("\x1b8"),
        "curseur rendu au caret"
    );
    assert_eq!(s.matches("a=t,").count(), 2, "deux images transmises");
    assert!(s.contains(&format!("\x1b[11;1H\x1b_Ga=p,i={FIRST_ID},p=")));
    // même image, nouvelle position ; même lettre dans un fondu : couleur ramenée au thème
    let faded = Color::Rgb(0xd0, 0xcf, 0xc4);
    let next = layer(
        vec![
            cell(0, 'a', p.text),
            cell(2, 'b', p.sub),
            cell(4, 'a', faded),
        ],
        p.bg,
    );
    let mut out = Vec::new();
    g.write(&mut out, &next, Some(&first), &p.rgb).unwrap();
    let s = text(out);
    assert!(!s.contains("a=t,"), "aucune nouvelle image : {s}");
    assert_eq!(
        s.matches("a=p,").count(),
        1,
        "seule la nouvelle lettre est placée"
    );
    assert!(s.contains(&format!("i={FIRST_ID},")));
    // une lettre disparaît : son placement est retiré, son fond repeint
    let last = layer(vec![cell(0, 'a', p.text), cell(2, 'b', p.sub)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &last, Some(&next), &p.rgb).unwrap();
    let s = text(out);
    assert!(s.contains("a=d,d=i,"), "{s}");
    assert!(s.contains("\x1b[11;5H  \x1b[12;5H  "));
    assert!(!s.contains("a=p,"));
}

#[test]
fn wrong_words_are_underlined_in_the_image() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let wrong = ScaledCell {
        style: Style::default()
            .fg(p.text)
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(p.error),
        ..cell(0, 'a', p.text)
    };
    let mut out = Vec::new();
    g.write(
        &mut out,
        &layer(vec![cell(2, 'a', p.text), wrong], p.bg),
        None,
        &p.rgb,
    )
    .unwrap();
    assert_eq!(
        text(out).matches("a=t,").count(),
        2,
        "souligné : une autre image"
    );
}

#[test]
fn hiding_moving_and_retheming() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let l = layer(vec![cell(0, 'a', p.text)], p.bg);
    let mut out = Vec::new();
    g.write(&mut out, &l, None, &p.rgb).unwrap();
    // plus de mots agrandis : toutes les lettres retirées, images gardées
    let mut out = Vec::new();
    g.clear(&mut out).unwrap();
    assert_eq!(
        text(out),
        format!("\x1b_Ga=d,d=r,x={FIRST_ID},y=1999999,q=2\x1b\\")
    );
    // la palette couvre la lettre : elle n'est pas placée
    let covered = ScaledText {
        hole: Some(Rect::new(0, 10, 4, 2)),
        ..l.clone()
    };
    let mut out = Vec::new();
    g.write(&mut out, &covered, None, &p.rgb).unwrap();
    assert!(!text(out).contains("a=p,"));
    // un autre thème : les anciennes images sont libérées
    let other = Palette::from_theme(theme("dracula").unwrap(), ColorMode::TrueColor);
    let mut out = Vec::new();
    g.write(
        &mut out,
        &layer(vec![cell(0, 'a', other.text)], other.bg),
        None,
        &other.rgb,
    )
    .unwrap();
    let s = text(out);
    assert!(s.contains("a=d,d=R,"), "images libérées");
    assert!(s.contains("a=t,"), "nouvelle image");
}

#[test]
fn images_are_compressed_and_small() {
    let (w, h, px) = render_glyph('m', CELL, 2, 1, (209, 208, 197), None);
    let sent = fasttype_tui::glyphs::transmit_compressed(FIRST_ID, w, h, &px);
    let s = String::from_utf8(sent.clone()).unwrap();
    assert!(s.starts_with(&format!(
        "\x1b_Ga=t,f=32,s=20,v=40,i={FIRST_ID},o=z,q=2,m=0;"
    )));
    assert!(sent.len() < 1200, "{} octets au lieu de ~4300", sent.len());
}

#[test]
fn a_letter_faded_to_the_background_is_not_drawn() {
    let p = palette();
    let mut g = GlyphText::new(CELL);
    let near_bg = Color::Rgb(0x34, 0x36, 0x39);
    let mut out = Vec::new();
    g.write(
        &mut out,
        &layer(vec![cell(0, 'a', near_bg)], p.bg),
        None,
        &p.rgb,
    )
    .unwrap();
    let s = text(out);
    assert!(!s.contains("a=t,") && !s.contains("a=p,"), "{s}");
}
```

Dans `crates/fasttype-tui/tests/kitty.rs`, remplacer `C=1,q=2\x1b\\"` par `C=1,z=1,q=2\x1b\\"` dans les deux attendus de placement (`placement_moves_the_cursor_then_places` et `kitty_caret_sends_each_image_once_and_moves_it`).

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test glyphs --test kitty`
Expected: échec de compilation (`unresolved import fasttype_tui::glyphs`). Une fois `glyphs` ajouté, `kitty` doit encore échouer sur `z=1` tant que `place` n'est pas modifié.

- [ ] **Step 4 : implémenter**

Ajouter `pub mod glyphs;` avant `pub mod input;` dans `crates/fasttype-tui/src/lib.rs`, puis créer `crates/fasttype-tui/src/glyphs.rs` :
```rust
//! Mots agrandis dessinés en images Kitty, pour les terminaux qui affichent
//! les images mais pas le texte agrandi (OSC 66) : Ghostty, WezTerm.
//!
//! Chaque lettre est rendue avec la police du site (Roboto Mono, embarquée)
//! dans une image de `scale × scale` cases, transmise une fois par couleur,
//! puis placée sur la grille. Seules les lettres qui changent sont replacées.
//! Les couleurs sont ramenées à celles du thème : un fondu ne crée pas des
//! centaines d'images.

use crate::kitty::CellPx;
use crate::sized::{ScaledCell, ScaledText};
use crate::theme::{Rgb, RgbColors, xterm_rgb};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::OnceLock;

/// Roboto Mono (SIL Open Font License 1.1, voir `assets/fonts/RobotoMono-OFL.txt`),
/// la police par défaut de Monkeytype.
static ROBOTO_MONO: &[u8] = include_bytes!("../../../assets/fonts/RobotoMono-Regular.ttf");

/// Identifiants des images de lettres : au-delà de ceux du caret (`kitty.rs`).
pub const FIRST_ID: u32 = 1_000_000;
const LAST_ID: u32 = 1_999_999;

fn font() -> &'static fontdue::Font {
    static FONT: OnceLock<fontdue::Font> = OnceLock::new();
    FONT.get_or_init(|| {
        fontdue::Font::from_bytes(ROBOTO_MONO, fontdue::FontSettings::default())
            .expect("Roboto Mono embarquée lisible")
    })
}

/// La police a un dessin pour ce caractère (les espaces n'en ont pas besoin).
pub fn has_glyph(c: char) -> bool {
    c.is_whitespace() || font().lookup_glyph_index(c) != 0
}

/// Image RGBA d'une lettre : `width_cells × scale` cases de large, `scale`
/// cases de haut, fond transparent, lettre centrée sur sa ligne, soulignée
/// au besoin (mot faux validé).
pub fn render_glyph(
    c: char,
    cell: CellPx,
    scale: u16,
    width_cells: u16,
    fg: Rgb,
    underline: Option<Rgb>,
) -> (u32, u32, Vec<u8>) {
    let s = f32::from(scale.max(1));
    let w = cell.w * u32::from(scale.max(1)) * u32::from(width_cells.max(1));
    let h = cell.h * u32::from(scale.max(1));
    let f = font();
    // l'avance de Roboto Mono vaut 0,6 em : la lettre remplit sa case comme dans le terminal
    let px = (cell.w as f32 * s / 0.6).min(cell.h as f32 * s / 1.3);
    let mut img = vec![0u8; (w * h * 4) as usize];
    let lm = f.horizontal_line_metrics(px);
    let (ascent, descent) = lm.map_or((px * 0.93, -px * 0.24), |m| (m.ascent, m.descent));
    let baseline = (h as f32 - (ascent - descent)) / 2.0 + ascent;
    let mut put = |x: i32, y: i32, rgb: Rgb, a: u8| {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 || a == 0 {
            return;
        }
        let k = ((y as u32 * w + x as u32) * 4) as usize;
        if a >= img[k + 3] {
            img[k..k + 4].copy_from_slice(&[rgb.0, rgb.1, rgb.2, a]);
        }
    };
    if !c.is_whitespace() {
        let (m, bitmap) = f.rasterize(c, px);
        let x0 = ((w as f32 - m.advance_width) / 2.0).round() as i32 + m.xmin;
        let y0 = (baseline - (m.ymin as f32 + m.height as f32)).round() as i32;
        for gy in 0..m.height {
            for gx in 0..m.width {
                put(
                    x0 + gx as i32,
                    y0 + gy as i32,
                    fg,
                    bitmap[gy * m.width + gx],
                );
            }
        }
    }
    if let Some(u) = underline {
        let thick = (px / 14.0).round().max(1.0) as i32;
        let y = (baseline + px * 0.12).round() as i32;
        for dy in 0..thick {
            for x in 0..w as i32 {
                put(x, y + dy, u, 255);
            }
        }
    }
    (w, h, img)
}

/// RGB d'une couleur ratatui (truecolor ou 256 couleurs).
fn rgb_of(c: Color) -> Option<Rgb> {
    match c {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Indexed(i) => Some(xterm_rgb(i)),
        _ => None,
    }
}

/// Couleur connue la plus proche (thème, et thème sous le voile de la palette).
fn snap(c: Rgb, known: &[Rgb]) -> Rgb {
    let d = |k: &Rgb| {
        let f = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        f(c.0, k.0) + f(c.1, k.1) + f(c.2, k.2)
    };
    known.iter().copied().min_by_key(d).unwrap_or(c)
}

/// Les 10 couleurs du thème : un fondu ou le voile de la palette retombent
/// sur l'une d'elles, sans créer une image par teinte intermédiaire.
fn known_colors(p: &RgbColors) -> Vec<Rgb> {
    vec![
        p.bg,
        p.main,
        p.caret,
        p.sub,
        p.sub_alt,
        p.text,
        p.error,
        p.error_extra,
        p.colorful_error,
        p.colorful_error_extra,
    ]
}

/// Transmission compressée (`o=z`, zlib) : une lettre est surtout transparente,
/// l'image tient en quelques centaines d'octets au lieu de quelques kilo-octets.
pub fn transmit_compressed(id: u32, w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let packed = miniz_oxide::deflate::compress_to_vec_zlib(rgba, 6);
    let data = crate::kitty::base64(&packed);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    let mut out = Vec::with_capacity(data.len() + 64 * chunks.len());
    for (k, chunk) in chunks.iter().enumerate() {
        let more = u8::from(k + 1 < chunks.len());
        if k == 0 {
            let _ = write!(out, "\x1b_Ga=t,f=32,s={w},v={h},i={id},o=z,q=2,m={more};");
        } else {
            let _ = write!(out, "\x1b_Gm={more};");
        }
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }
    out
}

/// Clé d'une image : lettre, largeur en cases, couleur, soulignement.
type GlyphKey = (char, u16, Rgb, Option<Rgb>);

/// Mots agrandis à l'écran, en images : ce qui a été transmis et placé.
pub struct GlyphText {
    cell: CellPx,
    theme: Option<RgbColors>,
    known: Vec<Rgb>,
    images: HashMap<GlyphKey, u32>,
    next_id: u32,
    /// Image placée à chaque position (coin haut-gauche de la lettre).
    placed: HashMap<(u16, u16), u32>,
}

/// Placement unique par position : plusieurs lettres partagent une image.
fn placement_id(x: u16, y: u16) -> u32 {
    (u32::from(y) << 16 | u32::from(x)) + 1
}

impl GlyphText {
    pub fn new(cell: CellPx) -> Self {
        GlyphText {
            cell,
            theme: None,
            known: Vec::new(),
            images: HashMap::new(),
            next_id: FIRST_ID,
            placed: HashMap::new(),
        }
    }

    /// Retire toutes les lettres de l'écran (les images restent en mémoire).
    pub fn clear(&mut self, out: &mut impl Write) -> io::Result<()> {
        if !self.placed.is_empty() {
            write!(out, "\x1b_Ga=d,d=r,x={FIRST_ID},y={LAST_ID},q=2\x1b\\")?;
            self.placed.clear();
        }
        Ok(())
    }

    fn image(&mut self, out: &mut impl Write, key: GlyphKey, scale: u16) -> io::Result<u32> {
        if let Some(&id) = self.images.get(&key) {
            return Ok(id);
        }
        let id = self.next_id;
        self.next_id = (self.next_id + 1).min(LAST_ID);
        let (w, h, px) = render_glyph(key.0, self.cell, scale, key.1, key.2, key.3);
        out.write_all(&transmit_compressed(id, w, h, &px))?;
        self.images.insert(key, id);
        Ok(id)
    }

    /// Fond du bloc d'une lettre (teinte du caret bloc, ou fond de la zone).
    fn paint(&self, buf: &mut Vec<u8>, c: &ScaledCell, scale: u16, bg: Color) {
        let style = Style::default().bg(c.style.bg.unwrap_or(bg));
        sgr_bg(buf, style);
        let blank = " ".repeat(usize::from(scale));
        for dy in 0..scale {
            let _ = write!(buf, "\x1b[{};{}H{blank}", c.y + dy + 1, c.x + 1);
        }
    }

    fn put(&mut self, buf: &mut Vec<u8>, c: &ScaledCell, t: &ScaledText) -> io::Result<()> {
        self.paint(buf, c, t.scale, t.bg);
        let pos = (c.x, c.y);
        let old = self.placed.remove(&pos);
        let fg = c.style.fg.and_then(rgb_of).map(|x| snap(x, &self.known));
        let under = c
            .style
            .add_modifier
            .contains(Modifier::UNDERLINED)
            .then(|| c.style.underline_color.and_then(rgb_of))
            .flatten()
            .map(|x| snap(x, &self.known));
        let bg = self.theme.map(|t| t.bg);
        // une lettre fondue jusqu'au fond n'est pas dessinée
        let visible = |fg: Rgb| Some(fg) != bg || under.is_some();
        let id = match fg {
            Some(fg) if (!c.ch.is_whitespace() || under.is_some()) && visible(fg) => {
                let width = unicode_width::UnicodeWidthChar::width(c.ch)
                    .unwrap_or(1)
                    .max(1) as u16;
                Some(self.image(buf, (c.ch, width, fg, under), t.scale)?)
            }
            _ => None,
        };
        let p = placement_id(c.x, c.y);
        if let Some(old) = old
            && Some(old) != id
        {
            let _ = write!(buf, "\x1b_Ga=d,d=i,i={old},p={p},q=2\x1b\\");
        }
        if let Some(id) = id {
            let _ = write!(
                buf,
                "\x1b[{};{}H\x1b_Ga=p,i={id},p={p},C=1,z=0,q=2\x1b\\",
                c.y + 1,
                c.x + 1
            );
            self.placed.insert(pos, id);
        }
        Ok(())
    }

    /// Montre `layer` en ne réécrivant que ce qui a changé depuis `prev`.
    /// Tout est refait si la zone, l'échelle, le fond, la partie recouverte
    /// par la palette ou le thème changent.
    pub fn write(
        &mut self,
        out: &mut impl Write,
        layer: &ScaledText,
        prev: Option<&ScaledText>,
        theme: &RgbColors,
    ) -> io::Result<()> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"\x1b7");
        if self.theme.as_ref() != Some(theme) {
            // nouveau thème : les images des anciennes couleurs sont libérées
            if self.theme.is_some() {
                let _ = write!(buf, "\x1b_Ga=d,d=R,x={FIRST_ID},y={LAST_ID},q=2\x1b\\");
            }
            self.theme = Some(*theme);
            self.known = known_colors(theme);
            self.images.clear();
            self.placed.clear();
        }
        let same = prev.filter(|p| {
            p.region == layer.region
                && p.scale == layer.scale
                && p.bg == layer.bg
                && p.hole == layer.hole
        });
        let hole = layer.hole;
        let covered = |c: &ScaledCell| {
            hole.is_some_and(|h| h.intersects(Rect::new(c.x, c.y, layer.scale, layer.scale)))
        };
        match same {
            None => {
                self.clear(&mut buf)?;
                clear_region(&mut buf, layer);
                for c in layer.cells.iter().filter(|c| !covered(c)) {
                    self.put(&mut buf, c, layer)?;
                }
            }
            Some(prev) => {
                let old: HashMap<(u16, u16), &ScaledCell> =
                    prev.cells.iter().map(|c| ((c.x, c.y), c)).collect();
                let new: std::collections::HashSet<(u16, u16)> =
                    layer.cells.iter().map(|c| (c.x, c.y)).collect();
                for c in prev
                    .cells
                    .iter()
                    .filter(|c| !new.contains(&(c.x, c.y)) && !covered(c))
                {
                    let blank = ScaledCell {
                        ch: ' ',
                        style: Style::default(),
                        ..c.clone()
                    };
                    self.put(&mut buf, &blank, layer)?;
                }
                for c in layer.cells.iter().filter(|c| !covered(c)) {
                    if old
                        .get(&(c.x, c.y))
                        .is_none_or(|o| o.ch != c.ch || o.style != c.style)
                    {
                        self.put(&mut buf, c, layer)?;
                    }
                }
            }
        }
        buf.extend_from_slice(b"\x1b[0m\x1b8");
        out.write_all(&buf)
    }
}

fn sgr_bg(buf: &mut Vec<u8>, style: Style) {
    let _ = match style.bg {
        Some(Color::Rgb(r, g, b)) => write!(buf, "\x1b[0;48;2;{r};{g};{b}m"),
        Some(Color::Indexed(i)) => write!(buf, "\x1b[0;48;5;{i}m"),
        _ => write!(buf, "\x1b[0m"),
    };
}

/// Efface la zone (hors palette) au fond du thème.
fn clear_region(buf: &mut Vec<u8>, t: &ScaledText) {
    sgr_bg(buf, Style::default().bg(t.bg));
    let r = t.region;
    let hole = t.hole.map(|h| h.intersection(r)).filter(|h| !h.is_empty());
    for y in r.top()..r.bottom() {
        let spans = match hole {
            Some(h) if y >= h.top() && y < h.bottom() => {
                vec![(r.left(), h.left()), (h.right(), r.right())]
            }
            _ => vec![(r.left(), r.right())],
        };
        for (a, b) in spans.into_iter().filter(|(a, b)| a < b) {
            let _ = write!(
                buf,
                "\x1b[{};{}H{}",
                y + 1,
                a + 1,
                " ".repeat(usize::from(b - a))
            );
        }
    }
}
```

Dans `crates/fasttype-tui/src/kitty.rs`, fonction `place` :
- ajouter la ligne de doc `/// `z=1` : au-dessus des lettres agrandies dessinées en images (`glyphs.rs`).` ;
- remplacer `C=1,q=2` par `C=1,z=1,q=2` dans la séquence.

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (7 dans `glyphs`).

- [ ] **Step 6 : commit**

```bash
git add .gitattributes Cargo.toml Cargo.lock assets/fonts crates/fasttype-tui
git commit -m "feat(tui): lettres agrandies dessinées en images Kitty avec Roboto Mono (compressées, placées au besoin)"
```

---

### Task 2 : choix du rendu, langues hors police, documentation

**Files:**
- Modify: `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/runner.rs`, `scripts/pty_smoke.py`, `NOTICE`, `README.md` (versions complètes)
- Modify: `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md`
- Test: `crates/fasttype-tui/tests/scaled.rs`

**Interfaces:**
- Consumes : tâche 1.
- Produces :
  - `App::set_scalable_chars(fn(char) -> bool)` (par défaut : tout) ;
  - dans `runner`, `Output.glyphs: Option<GlyphText>`, choisi au démarrage.

- [ ] **Step 1 : écrire le test qui échoue**

Ajouter à la fin de `crates/fasttype-tui/tests/scaled.rs` :
```rust
#[test]
fn languages_outside_the_font_stay_normal_size() {
    let mut a = app("glyph-latin", "");
    a.set_text_sizing(true);
    a.set_scalable_chars(fasttype_tui::glyphs::has_glyph);
    render(&mut a, 120, 30);
    assert!(a.scaled_text().is_some(), "anglais : agrandi");
    let mut a = app("glyph-cjk", "language = \"chinese_simplified\"\n");
    a.set_text_sizing(true);
    a.set_scalable_chars(fasttype_tui::glyphs::has_glyph);
    render(&mut a, 120, 30);
    assert!(
        a.scaled_text().is_none(),
        "chinois : la police n'a pas ces lettres"
    );
    // zen : la saisie compte aussi
    let mut a = app("glyph-zen", "mode = \"zen\"\n");
    a.set_text_sizing(true);
    a.set_scalable_chars(fasttype_tui::glyphs::has_glyph);
    a.handle(press(Key::Char('日'), 0.0));
    render(&mut a, 120, 30);
    assert!(a.scaled_text().is_none());
}
```

- [ ] **Step 2 : lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p fasttype-tui --test scaled`
Expected: échec de compilation (`no method named set_scalable_chars`).

- [ ] **Step 3 : implémenter**

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
use crate::palette::state::{Outcome, PaletteState};
use crate::palette::{Action, AppAction, Command, Subgroup, lists, lists::Context};
use crate::perf::Perf;
use crate::session_factory::DEFAULT_CUSTOM_TEXT;
use crate::session_factory::SessionFactory;
use crate::sized::{ScaledCell, ScaledText, scale_for};
use crate::theme::{ColorMode, Palette, Rgb, mix};
use crate::view::config_bar::bar_layout;
use crate::view::live::{LiveItem, LiveStats, Style3, WordsBox, render_bar, seconds_to_string};
use crate::view::notify::{Level, Notifications};
use crate::view::palette as palette_view;
use crate::view::result::{ResultView, invalid_label, unit_factor};
use crate::view::test::{Chrome, WordsView, words_box};
use crate::view::{MIN_HEIGHT, MIN_WIDTH, fill_background, too_small};
use fasttype_core::quote::QuoteFile;
use fasttype_core::result::TestResult;
use fasttype_core::session::{InputOutcome, SessionState, TestSession};
use fasttype_core::spec::Mode;
use fasttype_data::themes::{Rgba, Theme};
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_store::{Config, ConfigWarning, RecordOutcome, Store};
use ratatui::Frame;
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;
use ratatui::style::Style;
use std::collections::HashSet;
use toml::Value;
use unicode_width::UnicodeWidthStr;

/// Réglages qui changent le test : le changer relance un test (`afterExec: restart`).
const RESTART_KEYS: &[&str] = &[
    "mode",
    "time",
    "words",
    "quoteLength",
    "language",
    "punctuation",
    "numbers",
];

/// `canBailOut` (lists/bail-out.ts) : tests longs, infinis ou zen.
fn can_bail_out(spec: &fasttype_core::spec::TestSpec) -> bool {
    use fasttype_core::spec::CustomLimit;
    let big = |n: u32, threshold: u32| n == 0 || n >= threshold;
    match (spec.mode, spec.custom_limit) {
        (Mode::Zen, _) => true,
        (Mode::Time, _) => big(spec.time_limit.unwrap_or(0), 3600),
        (Mode::Words, _) => big(spec.mode2.parse().unwrap_or(0), 5000),
        (Mode::Custom, Some(CustomLimit::Time(s))) => big(s, 3600),
        (Mode::Custom, Some(CustomLimit::Word(n) | CustomLimit::Section(n))) => big(n, 5000),
        _ => false,
    }
}

/// Nom du texte custom en cours dans `custom_texts/`.
pub const CURRENT_CUSTOM_TEXT: &str = "current";

/// Citation choisie par « Search for quotes » : « langue id », dans le dossier de données.
pub const SELECTED_QUOTE_FILE: &str = "selected_quote.txt";

/// La citation choisie au dernier lancement, si elle vaut pour la langue en cours.
fn saved_quote(store: &Store) -> Option<u32> {
    let text = std::fs::read_to_string(store.paths.data_dir.join(SELECTED_QUOTE_FILE)).ok()?;
    let (language, id) = text.trim().split_once(' ')?;
    (language == store.config.str("language")).then(|| id.parse().ok())?
}

/// Citations d'une langue dans la palette : le début du texte, la source en alias.
fn quote_commands(file: &QuoteFile) -> Vec<Command> {
    file.quotes
        .iter()
        .map(|q| {
            let mut text: String = q.text.chars().take(64).collect();
            if q.text.chars().count() > 64 {
                text.push('…');
            }
            // la recherche porte sur toute la citation et sa source
            let alias = format!("{} {}", q.source, q.text);
            Command::new(text, Action::App(AppAction::SelectQuote(q.id))).alias(&alias)
        })
        .collect()
}

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
    /// Caractères que le rendu des mots agrandis sait dessiner.
    scalable: fn(char) -> bool,
    /// Zone des mots agrandis de l'image précédente : à redessiner en entier
    /// quand elle disparaît (ratatui n'y avait rien écrit).
    last_scaled_region: Option<Rect>,
    /// Haut de la zone de mots du dernier dessin (la barre de config reste au-dessus).
    words_top: Option<u16>,
    /// Palette de commandes ouverte.
    command_line: Option<PaletteState>,
    /// Palette du thème en cours, gardée pendant l'aperçu d'un autre thème.
    saved_palette: Option<Palette>,
    color_mode: ColorMode,
    /// Le prochain restart rejoue le même texte (« Repeat test »).
    repeat_next: bool,
    /// Langue décompressée dans un thread ; le test suivant démarre à la fin.
    loading: Option<(String, std::thread::JoinHandle<()>)>,
    /// Réglages exportés, à copier dans le presse-papiers par la boucle.
    clipboard: Option<String>,
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
        factory.custom_text = store.custom_texts.load(CURRENT_CUSTOM_TEXT).ok();
        factory.selected_quote = saved_quote(&store);
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
            scalable: |_| true,
            last_scaled_region: None,
            words_top: None,
            command_line: None,
            saved_palette: None,
            color_mode,
            repeat_next: false,
            loading: None,
            clipboard: None,
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

    /// Mots agrandis dessinés en images : seulement si la police embarquée a
    /// toutes les lettres (sinon, taille de base pour ce test).
    pub fn set_scalable_chars(&mut self, has: fn(char) -> bool) {
        self.scalable = has;
    }

    /// Les mots autour de la fenêtre visible s'agrandissent tous : 200 mots à
    /// partir de la ligne du haut, cibles et saisies (coût borné).
    fn window_is_scalable(&self) -> bool {
        let s = &self.session;
        let from = self.window.start.min(s.words().len());
        let to = (from + 200).min(s.words().len());
        (from..to).all(|i| {
            s.word(i)
                .chars()
                .chain(s.input(i).chars())
                .all(self.scalable)
        })
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

    /// La palette est ouverte.
    pub fn command_line(&self) -> Option<&PaletteState> {
        self.command_line.as_ref()
    }

    /// Ouvre la palette sur la liste racine.
    fn open_palette(&mut self) {
        let languages = fasttype_data::language_names().unwrap_or_default();
        let themes: Vec<&str> = fasttype_data::themes()
            .map(|t| t.iter().map(|t| t.name.as_str()).collect())
            .unwrap_or_default();
        let spec = self.session.spec();
        let running =
            matches!(self.screen, Screen::Test) && self.session.state() == SessionState::Running;
        let custom_text = self
            .factory
            .custom_text
            .as_deref()
            .unwrap_or(DEFAULT_CUSTOM_TEXT);
        let ctx = Context {
            config: &self.store.config,
            custom_text,
            on_result: matches!(self.screen, Screen::Result(_)),
            can_bail_out: running && can_bail_out(spec),
            languages: &languages,
            themes: &themes,
        };
        self.command_line = Some(PaletteState::open(lists::root(&ctx)));
        // la palette prend le focus : un Tab d'avant ne relance plus avec Entrée
        self.restart_armed = None;
    }

    fn close_palette(&mut self) {
        self.command_line = None;
        self.preview_theme(None);
    }

    fn palette_key(&mut self, key: Key, now: f64) {
        let Some(p) = &mut self.command_line else {
            return;
        };
        match p.key(key, &self.store.config) {
            Outcome::Stay => {
                let hovered = p.hovered().and_then(|c| c.preview.clone());
                self.preview_theme(hovered.as_deref());
            }
            Outcome::Close => self.close_palette(),
            Outcome::Run(Action::App(AppAction::SearchQuotes)) => {
                // liste construite à la demande : des milliers de citations
                let language = self.store.config.str("language").to_string();
                let list = self.factory.quotes(&language).map(|f| quote_commands(&f));
                match (list, &mut self.command_line) {
                    (Some(list), Some(p)) if !list.is_empty() => p.push(Subgroup {
                        title: "Search for quotes".into(),
                        list,
                    }),
                    _ => {
                        self.notifications.push(
                            format!("no quotes for {language}"),
                            Level::Notice,
                            now,
                        );
                        self.close_palette();
                    }
                }
            }
            Outcome::Run(action) => {
                self.command_line = None;
                self.saved_palette = None;
                self.run(action, now);
                // un thème survolé mais pas choisi : retour au thème de la config
                self.palette =
                    Palette::from_theme(&resolve_theme(&self.store.config).0, self.color_mode);
            }
        }
    }

    fn save_settings(&mut self, now: f64) {
        if let Err(e) = self.store.save_config() {
            self.notifications.push(
                format!("could not save the settings: {e}"),
                Level::Error,
                now,
            );
        }
    }

    /// Relance un test, après avoir décompressé la langue dans un thread si
    /// elle n'est pas encore en mémoire (« loading... » sur le badge).
    fn restart_when_ready(&mut self, now: f64) {
        let language = self.store.config.str("language").to_string();
        if fasttype_data::language_names().is_ok_and(|n| n.contains(&language.as_str()))
            && !self.factory.language_ready(&language)
        {
            let handle = self.factory.preload(&language);
            self.loading = Some((language, handle));
            return;
        }
        self.try_restart(now, true);
    }

    /// Réglages à copier dans le presse-papiers (OSC 52), une seule fois.
    pub fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard.take()
    }

    /// Langue en cours de chargement.
    pub fn loading(&self) -> Option<&str> {
        self.loading.as_ref().map(|(l, _)| l.as_str())
    }

    /// Aperçu d'un thème au survol ; `None` remet le thème en cours.
    fn preview_theme(&mut self, name: Option<&str>) {
        match name.and_then(theme) {
            Some(t) => {
                if self.saved_palette.is_none() {
                    self.saved_palette = Some(self.palette);
                }
                self.palette = Palette::from_theme(t, self.color_mode);
            }
            None => {
                if let Some(p) = self.saved_palette.take() {
                    self.palette = p;
                }
            }
        }
    }

    /// Exécute une commande de la palette.
    fn run(&mut self, action: Action, now: f64) {
        match action {
            Action::Set { key, value } => match self.store.config.set(key, value) {
                Ok(changed) => {
                    // comme `overrideConfig` du site, même si la valeur ne change pas
                    if key == "theme" {
                        let _ = self.store.config.set("customTheme", Value::Boolean(false));
                    }
                    self.save_settings(now);
                    // `afterExec: restart` : même si la valeur ne change pas
                    if RESTART_KEYS.contains(&key)
                        || changed.iter().any(|k| RESTART_KEYS.contains(k))
                    {
                        self.restart_when_ready(now);
                    }
                }
                Err(_) => {
                    self.notifications
                        .push(format!("invalid value for {key}"), Level::Error, now)
                }
            },
            Action::App(AppAction::NextTest) => self.try_restart(now, true),
            Action::App(AppAction::RepeatTest) => {
                self.repeat_next = true;
                self.try_restart(now, true);
            }
            Action::App(AppAction::BailOut) => {
                self.session.bail_out(now);
                self.check_finished(now);
            }
            Action::App(AppAction::ClearNotifications) => self.notifications.clear(),
            Action::App(AppAction::Quit) => self.quit = true,
            Action::App(AppAction::SearchQuotes) => {}
            Action::App(AppAction::SelectQuote(id)) => {
                // `quoteLength = [-2]` : la citation choisie, à chaque restart
                self.factory.selected_quote = Some(id);
                // gardée pour le prochain lancement, comme `selectedQuoteId` du site
                let language = self.store.config.str("language");
                let file = self.store.paths.data_dir.join(SELECTED_QUOTE_FILE);
                if let Err(e) =
                    fasttype_store::fs::write_atomic(&file, format!("{language} {id}").as_bytes())
                {
                    self.notifications.push(
                        format!("could not save the selected quote: {e}"),
                        Level::Error,
                        now,
                    );
                }
                let lengths = Value::Array(vec![Value::Integer(-2)]);
                if self.store.config.set("quoteLength", lengths).is_ok() {
                    self.save_settings(now);
                }
                self.try_restart(now, true);
            }
            Action::App(AppAction::SetCustomText(text)) => {
                if let Err(e) = self.store.custom_texts.save(CURRENT_CUSTOM_TEXT, &text) {
                    self.notifications.push(
                        format!("could not save the custom text: {e}"),
                        Level::Error,
                        now,
                    );
                }
                self.factory.custom_text = Some(text);
                if self
                    .store
                    .config
                    .set("mode", Value::String("custom".into()))
                    .is_ok()
                {
                    self.save_settings(now);
                }
                self.try_restart(now, true);
            }
            Action::App(AppAction::ExportSettings) => {
                self.clipboard = Some(self.store.config.to_toml());
                // certains terminaux (Terminal.app, tmux par défaut) ignorent OSC 52
                let path = self.store.paths.config_file();
                self.notifications.push(
                    format!(
                        "Settings sent to the clipboard (OSC 52) - also in {}",
                        path.display()
                    ),
                    Level::Notice,
                    now,
                );
            }
            Action::App(AppAction::ImportSettings(text)) => {
                let (config, warnings) = Config::from_toml(&text);
                // texte illisible : on ne touche à rien (`applyConfigFromJson`)
                if let Some(ConfigWarning::Syntax(e)) = warnings
                    .iter()
                    .find(|w| matches!(w, ConfigWarning::Syntax(_)))
                {
                    let first = e.lines().next().unwrap_or_default();
                    self.notifications.push(
                        format!("Failed to import settings: {first}"),
                        Level::Error,
                        now,
                    );
                    return;
                }
                for w in warnings {
                    self.notifications.push(w.to_string(), Level::Error, now);
                }
                self.store.config = config;
                self.save_settings(now);
                self.notifications
                    .push("Settings imported", Level::Notice, now);
                self.restart_when_ready(now);
            }
            Action::Open(_) | Action::Input(_) | Action::Close => {}
        }
        self.update_motion(now);
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
        let previous = std::mem::replace(&mut self.session, built.session);
        if std::mem::take(&mut self.repeat_next)
            && let Some(again) = previous.into_repeat()
        {
            self.session = again;
        }
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
        // nouveau test tout de suite (il apparaît en fondu) : on peut taper sans
        // attendre, contrairement au site qui ignore les touches 250 ms
        self.restart(now);
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
        if let Input::Paste(text) = &input {
            if let Some(p) = &mut self.command_line {
                p.paste(text);
            }
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
        self.advance_transitions(at);
        if self.command_line.is_some() {
            self.palette_key(key, at);
            return;
        }
        let quick = self.store.config.str("quickRestart");
        // la palette s'ouvre avec Échap, ou Tab quand Échap relance (`hotkeys.ts`)
        let opens = match key {
            Key::Palette => true,
            Key::Esc => quick != "esc",
            Key::Tab => quick == "esc",
            _ => false,
        };
        if opens {
            self.open_palette();
            return;
        }
        self.input_at = at;
        // En zen, Entrée insère un saut de ligne et Shift+Entrée termine le test :
        // ni l'une ni l'autre ne relance.
        let zen = self.session.spec().mode == Mode::Zen && matches!(self.screen, Screen::Test);
        let has_newlines = zen || self.session.has_newlines();
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

    /// Fait avancer les fondus jusqu'à `now`.
    fn advance_transitions(&mut self, now: f64) {
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
        if self.loading.as_ref().is_some_and(|(_, h)| h.is_finished()) {
            self.loading = None;
            self.try_restart(now, true);
        }
        self.notifications.expire(now);
        self.update_motion(now);
    }

    /// Recible les animations qui suivent l'état : focus mode, stats en direct,
    /// barre de progression, clignotement du caret.
    fn update_motion(&mut self, now: f64) {
        let on_test = matches!(self.screen, Screen::Test);
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
            && self.command_line.is_none()
            && (matches!(self.renderer, CaretRenderer::Kitty(_))
                || self.caret_style() == CaretStyle::Block);
        let since = self.caret.blink_since().filter(|_| drawn)?;
        let step = 1000.0 / f64::from(2 * crate::kitty::LEVELS);
        Some(since + ((now - since) / step).floor() * step + step)
    }

    /// Une image change-t-elle sans frappe (hors clignotement) ?
    fn animating(&self, now: f64) -> bool {
        // une langue se charge : on regarde à chaque image si c'est fini
        self.loading.is_some()
            || self.transition.is_some()
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
        self.words_top = None;
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
                    quick_restart: c.str("quickRestart"),
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
            words_top: self.words_top,
        }
        .render(buf, area);
        let typing = matches!(self.screen, Screen::Test)
            && self.session.state() == SessionState::Running
            && self.command_line.is_none();
        self.notifications.render(buf, area, &self.palette, typing);
        if let Some(p) = &self.command_line {
            // pendant l'aperçu d'un thème, pas de voile : ses vraies couleurs
            let veil = self.saved_palette.is_none();
            cursor = Some(palette_view::render(buf, area, p, &self.palette, veil));
            // la boîte recouvre les mots agrandis : ratatui la redessine toujours,
            // et la zone agrandie la contourne
            let boxed = palette_view::palette_rect(area, p);
            for y in boxed.top()..boxed.bottom() {
                for x in boxed.left()..boxed.right() {
                    buf[(x, y)].set_diff_option(CellDiffOption::AlwaysUpdate);
                }
            }
            if let Some(st) = &mut self.scaled {
                st.hole = Some(boxed);
                if veil {
                    st.bg = palette_view::dim_color(st.bg);
                    for c in &mut st.cells {
                        c.style = palette_view::dim_style(c.style);
                    }
                }
            }
            self.caret_frame = None;
        }
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
        let scale = if self.text_sizing && self.window_is_scalable() {
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
        self.words_top = Some(words.top);
        let badge = self.chrome.value(now) * opacity;
        let badge_row = words.top.saturating_sub(2);
        // jamais sur la ligne du logo ni sous la barre de config
        let bar_row = bar_layout(&self.store.config, area)
            .map(|(y, _)| y)
            .filter(|y| y + 1 < words.top);
        if badge > 0.0 && badge_row > area.y + 1 && bar_row.is_none_or(|b| b < badge_row) {
            let name = match &self.loading {
                Some(_) => "loading...".to_string(),
                None => self.session.spec().language.replace('_', " "),
            };
            let x = area.x + area.width.saturating_sub(name.width() as u16) / 2;
            let fg = self.palette.over_bg(self.palette.rgb.sub, badge);
            buf.set_string(x, badge_row, &name, Style::default().fg(fg));
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
        if style == CaretStyle::Off || s.state() == SessionState::Finished || s.words().is_empty() {
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

`crates/fasttype-tui/src/runner.rs` (version complète) :
```rust
//! Boucle principale : attend une touche ou la prochaine échéance (image
//! d'animation, tick du timer), applique toutes les touches en attente, puis
//! dessine aussitôt une seule image.

use crate::app::{App, CaretLook};
use crate::glyphs::{GlyphText, has_glyph};
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
    /// Mots agrandis dessinés en images (Ghostty, WezTerm) ; `None` : OSC 66.
    glyphs: Option<GlyphText>,
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
                        if self.glyphs.is_some() {
                            self.glyphs = Some(GlyphText::new(cell));
                        }
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
            let out = self.term.backend_mut();
            match (&mut self.glyphs, scaled) {
                // seules les lettres qui ont changé : quelques dizaines d'octets par frappe
                (None, Some(t)) => t.write_changes(self.scaled.as_ref(), out)?,
                (Some(g), Some(t)) => g.write(out, t, self.scaled.as_ref(), &app.palette().rgb)?,
                // plus de mots agrandis : retirer les images (ratatui réécrit les cases)
                (Some(g), None) => g.clear(out)?,
                (None, None) => {}
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
        if let Some(text) = app.take_clipboard() {
            // OSC 52 : le terminal copie le texte dans le presse-papiers
            write!(
                self.term.backend_mut(),
                "\x1b]52;c;{}\x07",
                crate::kitty::base64(text.as_bytes())
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
    // mots agrandis : OSC 66 (Kitty), sinon dessinés en images si le terminal
    // affiche les images Kitty (Ghostty, WezTerm), sinon taille de base
    let glyphs = match renderer {
        CaretRenderer::Kitty(cell) if !text_sizing_supported() => Some(GlyphText::new(cell)),
        _ => None,
    };
    let mut app = App::new(store, color_mode, clock.now_ms(), seed());
    app.set_caret_renderer(renderer);
    app.set_text_sizing(text_sizing_supported() || glyphs.is_some());
    if glyphs.is_some() {
        app.set_scalable_chars(has_glyph);
    }
    let mut out = Output {
        size: term.size()?,
        term,
        frame,
        last_look: None,
        kitty,
        scaled: None,
        glyphs,
    };
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

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (11 dans `scaled`).

- [ ] **Step 4 : documentation**

`NOTICE` (version complète) :
```text
fasttype — clone de Monkeytype pour le terminal.

Ce projet reprend le comportement, les formules et les données
(langues, citations, thèmes) de Monkeytype :
  https://github.com/monkeytypegame/monkeytype
  commit 574d819 (2026-10-06), sous licence GPL-3.0.

fasttype est distribué sous la même licence (GPL-3.0, voir LICENSE).

La police Roboto Mono (assets/fonts/RobotoMono-Regular.ttf), police par
défaut de Monkeytype, sert à dessiner les mots agrandis dans les terminaux
sans texte agrandi (Ghostty, WezTerm) :
  Copyright 2015 The Roboto Mono Project Authors
  https://github.com/googlefonts/robotomono
  sous licence SIL Open Font License 1.1 (assets/fonts/RobotoMono-OFL.txt).
```

`README.md` (version complète) :
```markdown
# fasttype

Monkeytype dans le terminal : mêmes modes, mêmes calculs, mêmes thèmes, mêmes animations, tout hors ligne.

## Lancer

```sh
cargo run --release -p fasttype-tui            # depuis la racine du projet
cargo run --release -p fasttype-tui -- --perf  # avec la latence touche → écran
```

Pour installer la commande `fasttype` : `cargo install --path crates/fasttype-tui`.

Options :

| Option | Effet |
|---|---|
| `--perf` | affiche la latence et le temps d'image, et un résumé à la sortie |
| `--fps 60\|120\|144` | cadence des animations (60 par défaut) |
| `--rebuild-pbs` | recalcule les records depuis l'historique |
| `--version`, `--help` | |

## Touches

| Touche | Effet |
|---|---|
| `tab` puis `enter` | nouveau test (`quick restart` : `tab`, `esc` ou `enter` seuls, dans la palette) |
| `esc` ou `ctrl+shift+p` | palette de commandes : tous les réglages, thèmes, langues, citations, texte custom… |
| `shift+enter` | termine un test zen |
| `ctrl+backspace`, `alt+backspace`, `ctrl+w` | efface le mot |
| `ctrl+c` | quitter |

Dans la palette : `↑`/`↓` (ou `ctrl+k`/`ctrl+j`, `tab`/`shift+tab`) pour se déplacer, `enter` pour choisir, `esc` pour revenir.

## Terminaux

- **Kitty (0.40 ou plus récent)** : caret au pixel près, et mots en grand avec la police du terminal (réglage `font size`, 2 par défaut comme sur le site).
- **Ghostty, WezTerm** : caret au pixel près, et mots en grand dessinés en images avec la police du site (Roboto Mono). Les écritures que Roboto Mono ne couvre pas (chinois, japonais, hébreu…) restent à la taille du terminal.
- **Autres terminaux** (iTerm2, Alacritty, Terminal.app…) : caret du terminal. Pour agrandir le texte, zoomer dans le terminal (`cmd +`).

`FASTTYPE_CARET=cell` ou `FASTTYPE_CARET=kitty` force le rendu du caret.

## Fichiers

| Fichier | Contenu |
|---|---|
| `~/.config/fasttype/config.toml` | réglages (toutes les clés de Monkeytype, en `snake_case`) |
| `~/.local/share/fasttype/results.jsonl` | historique des tests |
| `~/.local/share/fasttype/personal_bests.json` | records |
| `~/.local/share/fasttype/custom_texts/` | textes custom |

Les variables `XDG_CONFIG_HOME` et `XDG_DATA_HOME` sont respectées.

## Licence

GPL-3.0. Les données (langues, citations, thèmes) et les formules viennent de [Monkeytype](https://github.com/monkeytypegame/monkeytype) (voir `NOTICE`).
```

Dans la spec, ajouter juste avant `### 5.4 Thèmes et couleurs` :
```markdown
**Taille des mots** (ajout du 2026-10-08) : comme sur le site, les mots sont à `fontSize` fois la taille du reste de l'interface (2 par défaut, échelle entière de 1 à 4). Trois rendus, choisis au démarrage :
1. `osc66` : le protocole de texte agrandi de Kitty (≥ 0.40), avec la police du terminal.
2. `glyphs` : si le terminal affiche les images Kitty mais pas OSC 66 (Ghostty, WezTerm), chaque lettre est dessinée en image avec la police du site (Roboto Mono, embarquée, SIL OFL 1.1), transmise compressée une fois par couleur du thème, puis placée sur la grille. Seules les lettres qui changent sont replacées. Si la police n'a pas une lettre du test (CJK, hébreu…), les mots restent à la taille de base.
3. Sinon, taille de base : seul le zoom du terminal agrandit.

**Restart** (ajout du 2026-10-08, demande de l'utilisateur) : `quickRestart` vaut `tab` par défaut (le site : `off`, c'est-à-dire tab puis enter). Le nouveau test est créé tout de suite et apparaît en fondu ; les touches tapées pendant ce fondu comptent (le site les ignore 250 ms).
```

- [ ] **Step 5 : test pty**

`scripts/pty_smoke.py` (version complète) :
```python
"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf] [--kitty] [--sized] [--silent] [--palette] [--export] [--sigterm]
  --kitty    le terminal répond OK à la sonde graphique Kitty (cases de 10 × 20 pixels)
  --sized    le terminal agrandit le texte (OSC 66, Kitty ≥ 0.40)
  --silent   le terminal ne répond à aucune sonde (ni position, ni DA1) : chaque
             sonde doit abandonner au bout de 300 ms
  --palette  quitte par la palette de commandes (Échap, « quit », Entrée)
  --export   exporte les réglages depuis la palette (OSC 52), puis Ctrl+C
  --sigterm  quitte par SIGTERM au lieu de Ctrl+C"""
import fcntl, os, pty, re, select, signal, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
flags = sys.argv[3:]
perf, kitty, sigterm = "--perf" in flags, "--kitty" in flags, "--sigterm" in flags
sized, silent = "--sized" in flags, "--silent" in flags
palette, export = "--palette" in flags, "--export" in flags
probe_at, keyboard_at = None, None
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
    global probe_at, keyboard_at
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
        if probe_at is None and b"a=q" in data:
            probe_at = time.time()
        if keyboard_at is None and b"\x1b[?u" in data:
            keyboard_at = time.time()
        if silent:
            # seule la requête du protocole clavier (crossterm) reçoit sa réponse DA1
            if b"\x1b[?u" in data:
                os.write(fd, b"\x1b[?62c")
            continue
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
if export:
    os.write(fd, b"\x1b")
    pump(0.3)
    for ch in "export":
        os.write(fd, ch.encode())
        pump(0.05)
    os.write(fd, b"\r")
    pump(0.3)
if sigterm:
    os.kill(pid, signal.SIGTERM)
elif palette:
    os.write(fd, b"\x1b")
    pump(0.3)
    for ch in "quit":
        os.write(fd, ch.encode())
        pump(0.05)
    pump(0.2)
    palette_seen = "Quit" in plain(bytes(out))
    os.write(fd, b"\r")
else:
    os.write(fd, b"\x03")  # Ctrl+C
pump(1.0)
_, status = os.waitpid(pid, 0)
if os.environ.get("FASTTYPE_DUMP"):
    open(os.environ["FASTTYPE_DUMP"], "wb").write(bytes(out))
print("exit", os.waitstatus_to_exitcode(status))
print("result screen:", "test type custom english" in screen)
print("alt screen left:", b"\x1b[?1049l" in out)
print("sync output used:", b"\x1b[?2026h" in out and b"\x1b[?2026l" in out)
print("cursor color reset:", b"\x1b]112\x07" in out)
print("bracketed paste reset:", b"\x1b[?2004l" in out)
if kitty:
    print("kitty caret placed:", b"\x1b_Ga=p," in out)
    if not sized:
        # Ghostty, WezTerm : images Kitty sans OSC 66 → mots dessinés en images
        print("glyph letters placed:", b"\x1b_Ga=p,i=1000000," in out)
    print("kitty images deleted:", out.rstrip().find(b"\x1b_Ga=d,d=A,q=2\x1b\\") > out.find(b"\x1b_Ga=p,"))
if palette:
    print("command line shown:", palette_seen)
if export:
    import base64
    found = re.search(rb"\x1b\]52;c;([A-Za-z0-9+/=]+)\x07", bytes(out))
    copied = base64.b64decode(found.group(1)).decode() if found else ""
    print("settings copied (OSC 52):", 'mode = "custom"' in copied)
if silent:
    wait = (keyboard_at - probe_at) * 1000 if probe_at and keyboard_at else None
    print("probes gave up after (ms):", round(wait) if wait else None)
if sized:
    print("scaled words written:", b"\x1b]66;s=2;" in out)
if perf:
    found = re.findall(r"key→flush p50 ([0-9.]+) p99 ([0-9.]+) ms · frame p99 ([0-9.]+) ms · n (\d+)", bytes(out).decode("utf-8", "replace"))
    print("perf (p50, p99, frame p99, n):", found[-1] if found else None)
```

Lancer chaque mode **séparément**, avec ses options en arguments distincts :
```bash
cargo build --release -p fasttype-tui
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --kitty --perf
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --kitty --sized --perf
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --perf
```
Expected : pour chacun, `exit 0` et tous les contrôles à `True`. Avec `--kitty` seul, `glyph letters placed: True`.

Run: `cargo bench -p fasttype-tui --bench frame 2>&1 | grep -A1 -E "^(frame|key)"`
Expected : les trois benchmarks sous 1 ms.

- [ ] **Step 6 : vérification finale et essai à la main**

Run: `cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, aucun avertissement.

Essai à la main, dans Ghostty : `cargo run --release -p fasttype-tui`.
- Les mots doivent être en double taille, en Roboto Mono, avec le caret à leur hauteur.
- Taper un mot faux doit le souligner en rouge.
- `esc` doit ouvrir la palette par-dessus les mots.

Le noter dans le ledger, et signaler à l'utilisateur que l'essai visuel lui revient.

- [ ] **Step 7 : commit**

```bash
git add NOTICE README.md docs crates/fasttype-tui scripts
git commit -m "feat(tui): mots agrandis en images dans Ghostty et WezTerm, taille de base hors police"
```
