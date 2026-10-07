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

/// Envoie `kitty::PROBE` et lit la réponse directement sur l'entrée, avant
/// que le thread clavier ne la lise : 300 ms au plus (DA1 arrive bien avant).
#[cfg(unix)]
fn probe_graphics(out: &mut impl Write) -> io::Result<bool> {
    use std::time::{Duration, Instant};
    out.write_all(crate::kitty::PROBE)?;
    out.flush()?;
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut got = Vec::new();
    loop {
        if let Some(ok) = crate::kitty::probe_answer(&got) {
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
fn probe_graphics(_out: &mut impl Write) -> io::Result<bool> {
    Ok(false)
}

/// Des images Kitty (caret) seront affichées : `restore` les supprimera.
pub fn mark_kitty_images() {
    KITTY_IMAGES.store(true, Ordering::SeqCst);
}
static HOOK: Once = Once::new();

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
        GRAPHICS.store(probe_graphics(&mut out)?, Ordering::SeqCst);
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
