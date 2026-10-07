use criterion::{Criterion, criterion_group, criterion_main};
use fasttype_store::Store;
use fasttype_store::paths::Paths;
use fasttype_tui::app::App;
use fasttype_tui::input::{Input, Key, Phase};
use fasttype_tui::theme::ColorMode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::hint::black_box;

fn app() -> App {
    let dir = std::env::temp_dir().join(format!("fasttype-bench-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("config")).unwrap();
    // test infini : on peut taper sans fin
    std::fs::write(dir.join("config/config.toml"), "time = 0\n").unwrap();
    let store = Store::open(Paths {
        config_dir: dir.join("config"),
        data_dir: dir.join("data"),
    });
    App::new(store, ColorMode::TrueColor, 0.0, 7)
}

/// Prochaine lettre juste du test en cours.
fn next_char(app: &App) -> char {
    let s = app.session();
    let a = s.active_index();
    s.word(a)
        .chars()
        .nth(s.input(a).chars().count())
        .unwrap_or(' ')
}

fn key(a: &mut App, t: f64) {
    let key = Key::Char(next_char(a));
    a.handle(Input::Key {
        key,
        phase: Phase::Press,
        code: 1,
        at: t,
    });
    a.tick(t);
}

fn bench(c: &mut Criterion) {
    let mut term = Terminal::new(TestBackend::new(200, 60)).unwrap();
    let mut a = app();
    c.bench_function("frame_200x60", |b| {
        b.iter(|| {
            term.draw(|f| a.draw(f, None)).unwrap();
        })
    });
    let mut t = 0.0;
    c.bench_function("key_and_frame_200x60", |b| {
        b.iter(|| {
            key(&mut a, t);
            t += 15.0;
            term.draw(|f| a.draw(f, None)).unwrap();
            black_box(a.session().active_index())
        })
    });
    // spec §9 : le coût d'une frappe ne grandit pas avec la longueur du test
    let mut long = app();
    let mut t = 0.0;
    for _ in 0..10_000 {
        key(&mut long, t);
        t += 15.0;
        term.draw(|f| long.draw(f, None)).unwrap();
    }
    assert!(
        long.session().active_index() > 1500,
        "les 10 000 frappes ont été tapées"
    );
    c.bench_function("key_and_frame_after_10k_keys", |b| {
        b.iter(|| {
            key(&mut long, t);
            t += 15.0;
            term.draw(|f| long.draw(f, None)).unwrap();
            black_box(long.session().active_index())
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
