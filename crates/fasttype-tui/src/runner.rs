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
