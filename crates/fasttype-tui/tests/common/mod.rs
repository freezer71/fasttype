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
