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

/// Image du caret : taille en pixels et pixels RGBA (alpha = `alpha`).
pub fn caret_image(
    style: CaretStyle,
    cell: CellPx,
    width_cells: u32,
    rgb: Rgb,
    alpha: u8,
) -> (u32, u32, Vec<u8>) {
    let full = width_cells.max(1) * cell.w;
    let (w, h) = match style {
        CaretStyle::Underline => (full, thin(cell.h)),
        CaretStyle::Outline => (full, cell.h),
        _ => (thin(cell.h), cell.h),
    };
    let border = (cell.h as f64 / 24.0).round().max(1.0) as u32;
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
    let mut x = f.x * cw;
    let mut y = f.y * ch;
    match f.style {
        // barre centrée sur le bord gauche de la lettre
        CaretStyle::Bar => x -= f64::from(thin(cell.h)) / 2.0,
        CaretStyle::Underline => y += ch - f64::from(thin(cell.h)),
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

    fn image_id(style: CaretStyle, width_cells: u32, level: u32) -> u32 {
        let s = match style {
            CaretStyle::Underline => 2,
            CaretStyle::Outline => 3,
            _ => 1,
        };
        s * 1000 + width_cells.min(9) * 100 + level + 1
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
        let id = Self::image_id(f.style, width_cells, level);
        if self.sent.insert(id) {
            let alpha = (f64::from(level) / f64::from(LEVELS) * 255.0).round() as u8;
            let (w, h, px) = caret_image(f.style, self.cell, width_cells, rgb, alpha);
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
