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
