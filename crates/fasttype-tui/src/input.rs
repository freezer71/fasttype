//! Lecture du clavier dans un thread dédié : chaque événement est horodaté
//! dès sa réception, puis envoyé à la boucle principale.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fasttype_core::clock::{Clock, SystemClock};
use std::sync::Arc;
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    /// Ctrl/Alt + Backspace, Ctrl+W, Ctrl+H : efface le mot.
    DeleteWord,
    Tab,
    BackTab,
    Enter,
    /// Shift + Entrée (signalé seulement avec le protocole clavier Kitty).
    ShiftEnter,
    Esc,
    /// Flèche haut, Ctrl+K, Ctrl+P (navigation dans la palette).
    Up,
    /// Flèche bas, Ctrl+J, Ctrl+N.
    Down,
    /// Ctrl+Shift+P : ouvre la palette (avec le protocole clavier Kitty).
    Palette,
    /// Ctrl+C.
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Key {
        key: Key,
        phase: Phase,
        code: u32,
        at: f64,
    },
    Resize,
    /// SIGTERM, SIGHUP, SIGINT ou SIGQUIT reçu : quitter proprement.
    Interrupt,
    /// Texte collé (bracketed paste) : arrive d'un bloc, sauts de ligne compris.
    Paste(String),
}

impl Input {
    /// Horodatage d'une touche (pour mesurer la latence touche → écran).
    pub fn at(&self) -> Option<f64> {
        match self {
            Input::Key { at, .. } => Some(*at),
            Input::Resize | Input::Interrupt | Input::Paste(_) => None,
        }
    }
}

/// Codes des touches non imprimables, hors de la plage Unicode.
const CODE_BASE: u32 = 0x11_0000;

/// Traduit une touche crossterm. `code` identifie la touche physique pour
/// apparier appui et relâchement (la casse n'en change pas le code).
pub fn map_key(ev: &KeyEvent) -> Option<(Key, u32)> {
    let ctrl = ev.modifiers.contains(KeyModifiers::CONTROL);
    let alt = ev.modifiers.contains(KeyModifiers::ALT);
    let shift = ev.modifiers.contains(KeyModifiers::SHIFT);
    let key = match ev.code {
        KeyCode::Char('c') if ctrl => Key::Quit,
        KeyCode::Char('p' | 'P') if ctrl && shift => Key::Palette,
        KeyCode::Char('k' | 'p') if ctrl => Key::Up,
        KeyCode::Char('j' | 'n') if ctrl => Key::Down,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Char('w' | 'h') if ctrl => Key::DeleteWord,
        KeyCode::Backspace if ctrl || alt => Key::DeleteWord,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter if shift => Key::ShiftEnter,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::BackTab => Key::BackTab,
        KeyCode::Esc => Key::Esc,
        KeyCode::Char(c) if !ctrl => Key::Char(c),
        _ => return None,
    };
    let code = match key {
        Key::Char(c) => c.to_lowercase().next().unwrap_or(c) as u32,
        Key::Backspace | Key::DeleteWord => CODE_BASE + 1,
        Key::Tab | Key::BackTab => CODE_BASE + 2,
        Key::Enter | Key::ShiftEnter => CODE_BASE + 3,
        Key::Esc => CODE_BASE + 4,
        Key::Quit => CODE_BASE + 5,
        Key::Up => CODE_BASE + 6,
        Key::Down => CODE_BASE + 7,
        Key::Palette => CODE_BASE + 8,
    };
    Some((key, code))
}

pub fn map_event(ev: Event, at: f64) -> Option<Input> {
    match ev {
        Event::Key(k) => {
            let (key, code) = map_key(&k)?;
            let phase = match k.kind {
                KeyEventKind::Press => Phase::Press,
                KeyEventKind::Repeat => Phase::Repeat,
                KeyEventKind::Release => Phase::Release,
            };
            Some(Input::Key {
                key,
                phase,
                code,
                at,
            })
        }
        Event::Resize(..) => Some(Input::Resize),
        Event::Paste(text) => Some(Input::Paste(text)),
        _ => None,
    }
}

/// Lit le terminal en continu. S'arrête quand la boucle principale a fermé le canal.
pub fn spawn_reader(clock: Arc<SystemClock>, tx: SyncSender<Input>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("fasttype-input".into())
        .spawn(move || {
            while let Ok(ev) = crossterm::event::read() {
                let at = clock.now_ms();
                if let Some(input) = map_event(ev, at)
                    && tx.send(input).is_err()
                {
                    break;
                }
            }
        })
        .expect("création du thread de lecture du clavier")
}

/// Transforme SIGTERM, SIGHUP, SIGINT et SIGQUIT en `Input::Interrupt` : la
/// boucle s'arrête et le terminal est restauré au lieu de rester en mode raw.
#[cfg(unix)]
pub fn spawn_signal_watcher(tx: SyncSender<Input>) -> std::io::Result<JoinHandle<()>> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
    let mut signals = signal_hook::iterator::Signals::new([SIGTERM, SIGHUP, SIGINT, SIGQUIT])?;
    std::thread::Builder::new()
        .name("fasttype-signals".into())
        .spawn(move || {
            for _ in signals.forever() {
                if tx.send(Input::Interrupt).is_err() {
                    break;
                }
            }
        })
}
