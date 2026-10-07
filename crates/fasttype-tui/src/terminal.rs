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
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen)?;
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
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}
