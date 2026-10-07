//! Boucle principale : attend une touche ou la prochaine échéance, applique
//! toutes les touches en attente, puis dessine aussitôt une seule image.

use crate::app::{App, CaretLook};
use crate::input::{Input, spawn_reader};
use crate::perf::Perf;
use crate::terminal::{FrameWriter, TerminalGuard, queue_caret_look};
use crate::theme::ColorMode;
use crossterm::queue;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use fasttype_core::clock::{Clock, SystemClock};
use fasttype_store::Store;
use fasttype_store::paths::Paths;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

pub struct Options {
    pub perf: bool,
}

type Term = Terminal<CrosstermBackend<FrameWriter>>;

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1)
}

/// Dessine une image et l'envoie en une écriture, encadrée par la sortie
/// synchronisée (mode 2026) : le terminal ne montre jamais d'image partielle.
fn present(
    term: &mut Term,
    frame: &FrameWriter,
    app: &App,
    perf: &Perf,
    last_look: &mut Option<CaretLook>,
) -> io::Result<()> {
    queue!(term.backend_mut(), BeginSynchronizedUpdate)?;
    let look = app.caret_look();
    if look != *last_look {
        if let Some(l) = look {
            queue_caret_look(term.backend_mut(), l)?;
        }
        *last_look = look;
    }
    term.draw(|f| app.draw(f, perf.enabled.then_some(perf)))?;
    queue!(term.backend_mut(), EndSynchronizedUpdate)?;
    frame.present(&mut io::stdout())
}

/// Lance l'interface ; renvoie les mesures de fluidité de la session.
pub fn run(opts: Options) -> io::Result<Perf> {
    let paths = Paths::from_system().ok_or_else(|| io::Error::other("HOME is not set"))?;
    let store = Store::open(paths);
    let clock = Arc::new(SystemClock::new());
    let color_mode = ColorMode::detect(|k| std::env::var(k).ok());
    let _guard = TerminalGuard::enter()?;
    let (tx, rx) = mpsc::sync_channel::<Input>(4096);
    spawn_reader(Arc::clone(&clock), tx);
    let frame = FrameWriter::new();
    let mut term = Terminal::new(CrosstermBackend::new(frame.clone()))?;
    let mut app = App::new(store, color_mode, clock.now_ms(), seed());
    let mut perf = Perf::new(opts.perf);
    let mut last_look = None;
    present(&mut term, &frame, &app, &perf, &mut last_look)?;

    while !app.quit {
        let now = clock.now_ms();
        let first = match app.next_deadline() {
            Some(d) => rx.recv_timeout(Duration::from_secs_f64((d - now).max(0.0) / 1000.0)),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        let mut oldest_key = None;
        match first {
            Ok(input) => {
                oldest_key = input.at();
                app.handle(input);
                // rafale : toutes les touches déjà reçues avant de dessiner
                for more in rx.try_iter() {
                    oldest_key = oldest_key.or(more.at());
                    app.handle(more);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        app.tick(clock.now_ms());
        let start = clock.now_ms();
        present(&mut term, &frame, &app, &perf, &mut last_look)?;
        let done = clock.now_ms();
        perf.frame.record(done - start);
        if let Some(t) = oldest_key {
            perf.input_to_flush.record(done - t);
        }
    }
    Ok(perf)
}
