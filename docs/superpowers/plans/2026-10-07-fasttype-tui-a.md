# fasttype-tui, partie A : le cœur jouable — plan d'implémentation (plan 4a)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un premier `fasttype` jouable dans le terminal : écran de test avec les couleurs du thème, frappe, timer, écran de résultat, enregistrement dans l'historique et les records. Les fondations de la fluidité (spec §7) sont en place dès cette partie.

**Architecture:**
- **Crate `fasttype-tui`**, avec le binaire `fasttype`.
- **Application pure.** `App` contient l'état (session, écran, notifications) et réagit à des entrées horodatées. Elle dessine dans un `Buffer` ratatui. Elle ne fait aucune E/S terminal, ce qui permet de la tester entièrement avec `TestBackend`.
- **Boucle (`runner`).** Un thread lit le clavier et horodate chaque touche. La boucle principale attend une touche ou la prochaine échéance (tick du timer, fin d'une notification). Elle applique toutes les touches reçues, puis dessine **immédiatement** une seule image.
- **Envoi des images.** Chaque image part en **une seule écriture**, encadrée par la sortie synchronisée (mode 2026). Quand rien ne bouge, la boucle dort : aucune image, 0 % de CPU.
- **Caret.** C'est le curseur du terminal lui-même, avec sa forme (barre, bloc, souligné), son clignotement avant la première frappe et la couleur du thème (OSC 12). C'est gratuit, net, et identique au caret « default » du site. Le glissement fluide au pixel près viendra au plan 4b.

**Tech Stack:** Rust 1.97 (édition 2024), `ratatui` 0.30.2, `crossterm` 0.29.0 (protocole clavier Kitty, sortie synchronisée), `unicode-width` 0.2.2, `criterion` 0.8.2, et Python 3 (test de bout en bout dans un pseudo-terminal, outil de développement seulement).

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§5, §7, §8, §9). Les plans 1 à 3 sont livrés sur `main`.

**Découpage de l'interface :**
- **4a (ce plan)** : le cœur jouable.
- **4b** : la fluidité visible (caret glissant au pixel près avec le protocole graphique Kitty, puis par demi-case ; moteur d'animation ; défilement et fondus de 125 ms ; focus mode animé ; graphique en braille ; stats live).
- **4c** : la palette de commandes construite sur `SCHEMA`, la barre de config, le changement de mode, de langue et de thème avec chargement en arrière-plan, la recherche de citations, le texte custom, bail out et repeat.

**Code vérifié avant rédaction :** tout le code de ce plan a été assemblé et exécuté dans un projet jetable :
- 47 tests au vert et clippy propre ;
- benchmark `frame_200x60` à 0,17 ms et `key_and_frame_200x60` à 0,39 ms ;
- test réel dans un pseudo-terminal : latence touche → écran p50 = 0,13 ms.

Dans le pseudo-terminal, les pires frappes (1 à 3 ms) viennent du réveil du thread par le système et de l'écriture dans le pty (mesuré frappe par frappe sur 5 exécutions), pas du calcul de fasttype.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`.
- **Textes de l'interface en anglais, identiques à Monkeytype** (« tab + enter - restart », « Test invalid - too short »). Commentaires en français. Les avertissements venant de `fasttype-store` restent en français (traduction reportée).
- `App` et les vues ne font aucune E/S terminal. Seuls `terminal.rs`, `input.rs` (thread de lecture) et `runner.rs` touchent au terminal.
- **Fluidité (spec §7) :**
  - rendu immédiat après une touche ;
  - toutes les touches en attente appliquées avant de dessiner ;
  - une image par écriture, encadrée par `BeginSynchronizedUpdate` / `EndSynchronizedUpdate` ;
  - aucune image ni aucun réveil quand rien ne change (`next_deadline` vaut `None` au repos).
- Aucune panique sur une donnée lue : langue inconnue → `english` avec notification ; thème inconnu → `serika_dark` ; aucune citation → mode time.
- Le terminal est toujours restauré à la sortie, même en cas de panique : écran normal, curseur et couleur du curseur d'origine, protocole clavier retiré, mode raw coupé.
- Les tests n'écrivent jamais dans le vrai `$HOME`.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace (somme de toutes les lignes `test result`), tous au vert.

**Limite connue de 4a, assumée :** sans protocole clavier Kitty (Terminal.app, par exemple), Shift+Entrée n'est pas distinguable d'Entrée, donc un test zen ne peut se terminer qu'avec Ctrl+C. Le « Bail out » de la palette (plan 4c) lèvera cette limite.

## Review Focus

1. **Panique ou interruption pendant la partie** : le terminal doit être rendu intact (écran, curseur, mode raw). À vérifier par lecture de `terminal.rs` (hook de panique, `restore` idempotent) et par le test pty de la tâche 7 pour la sortie normale.
2. **Rafale de touches plus rapide que l'affichage** : toutes appliquées dans l'ordre, puis une seule image. À vérifier par lecture de `runner.rs` (`try_iter` avant `present`).
3. **Redimensionnement en pleine frappe** : le caret reste juste après les lettres tapées, quelle que soit la largeur. Test dans la tâche 6 (`resize_keeps_caret_after_the_typed_letters`).
4. **Mot plus large que la ligne, ou terminal très étroit** : aucune panique, le mot a sa propre ligne, et sous 40 × 10 un message s'affiche. Tests dans les tâches 3 et 6.
5. **Config pointant vers une langue ou un thème inexistant** : repli et notification, jamais d'écran vide. Tests dans la tâche 6.

---

## Structure des fichiers

```
Cargo.toml                                  + membre crates/fasttype-tui ; ratatui, crossterm, unicode-width
crates/fasttype-tui/
├── Cargo.toml                              binaire `fasttype`
├── src/lib.rs
├── src/main.rs                             options : --perf, --rebuild-pbs, --version, --help
├── src/theme.rs                            ColorMode, Palette, nearest_256 (OKLab)
├── src/input.rs                            Key, Phase, Input, map_key, map_event, spawn_reader
├── src/layout.rs                           mots → lignes, caret, fenêtre de 3 lignes
├── src/perf.rs                             LatencyStats, Perf
├── src/session_factory.rs                  config → TestSession (langues et citations en cache)
├── src/view/mod.rs                         fond, centrage, « terminal too small »
├── src/view/notify.rs                      Notifications
├── src/view/test.rs                        TestView
├── src/view/result.rs                      ResultView
├── src/app.rs                              App : état, touches, dessin
├── src/terminal.rs                         FrameWriter, TerminalGuard, restore, caret du terminal
├── src/runner.rs                           boucle principale
├── tests/common/mod.rs                     App de test, frappe simulée, rendu TestBackend
├── tests/{theme,input,layout,perf,factory,app,view,frame_writer}.rs
└── benches/frame.rs
scripts/pty_smoke.py                        test de bout en bout dans un pseudo-terminal
```

---

### Task 1 : crate fasttype-tui et couleurs des thèmes

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/fasttype-tui/Cargo.toml`, `crates/fasttype-tui/src/lib.rs`, `crates/fasttype-tui/src/theme.rs`
- Test: `crates/fasttype-tui/tests/theme.rs`

**Interfaces:**
- Consumes : `fasttype_data::themes::{Rgba, Theme}`, `fasttype_data::{theme, themes, DEFAULT_THEME}`.
- Produces :
  - `ColorMode { TrueColor, Ansi256 }` et `ColorMode::detect(get: impl Fn(&str) -> Option<String>)` ;
  - `Palette { bg, main, caret, sub, sub_alt, text, error, error_extra, colorful_error, colorful_error_extra: Color, caret_rgb: (u8, u8, u8) }` et `Palette::from_theme(&Theme, ColorMode)` ;
  - `nearest_256(r, g, b) -> u8`.

- [ ] **Step 1 : déclarer la crate**

Dans `Cargo.toml` (racine) :
- `members = ["crates/fasttype-core", "crates/fasttype-data", "crates/fasttype-store", "crates/fasttype-tui", "xtask"]` ;
- sous `[workspace.dependencies]`, ajouter :
```toml
ratatui = "0.30.2"
crossterm = "0.29.0"
unicode-width = "0.2.2"
```

`crates/fasttype-tui/Cargo.toml` (le binaire et le benchmark sont déclarés à la tâche 7, quand leurs fichiers existent) :
```toml
[package]
name = "fasttype-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
fasttype-core.workspace = true
fasttype-data = { workspace = true, features = ["embedded"] }
fasttype-store.workspace = true
ratatui.workspace = true
crossterm.workspace = true
unicode-width.workspace = true

[dev-dependencies]
criterion.workspace = true
```

`crates/fasttype-tui/src/lib.rs` :
```rust
//! Interface terminal de fasttype : écrans, entrées, boucle de rendu.

pub mod theme;
```

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/theme.rs` :
```rust
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_tui::theme::{ColorMode, Palette, nearest_256};
use ratatui::style::Color;

#[test]
fn detects_truecolor() {
    assert_eq!(
        ColorMode::detect(|k| (k == "COLORTERM").then(|| "truecolor".into())),
        ColorMode::TrueColor
    );
    assert_eq!(
        ColorMode::detect(|k| (k == "COLORTERM").then(|| "24bit".into())),
        ColorMode::TrueColor
    );
    assert_eq!(ColorMode::detect(|_| None), ColorMode::Ansi256);
}

#[test]
fn nearest_xterm_colors() {
    assert_eq!(nearest_256(0, 0, 0), 16);
    assert_eq!(nearest_256(255, 255, 255), 231);
    assert_eq!(nearest_256(255, 0, 0), 196);
    assert_eq!(nearest_256(128, 128, 128), 244);
}

#[test]
fn serika_dark_palette() {
    let t = theme(DEFAULT_THEME).unwrap();
    let p = Palette::from_theme(t, ColorMode::TrueColor);
    assert_eq!(p.bg, Color::Rgb(0x32, 0x34, 0x37));
    assert_eq!(p.main, Color::Rgb(0xe2, 0xb7, 0x14));
    assert_eq!(p.caret_rgb, (0xe2, 0xb7, 0x14));
    let p256 = Palette::from_theme(t, ColorMode::Ansi256);
    assert!(matches!(p256.main, Color::Indexed(_)));
}

#[test]
fn translucent_colors_are_blended_on_background() {
    // slambook : sub = #1c82adc4, seule couleur translucide des 187 thèmes
    let t = theme("slambook").unwrap();
    assert!(t.sub.a < 255);
    let p = Palette::from_theme(t, ColorMode::TrueColor);
    let blended = t.sub.over(t.bg);
    assert_eq!(p.sub, Color::Rgb(blended.r, blended.g, blended.b));
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test theme`
Expected: échec de compilation (`unresolved imports fasttype_tui::theme::{...}`).

- [ ] **Step 4 : implémenter `theme.rs`**

```rust
//! Couleurs d'un thème Monkeytype pour le terminal : truecolor si le terminal
//! l'annonce, sinon la couleur la plus proche des 256 couleurs xterm (OKLab).

use fasttype_data::themes::{Rgba, Theme};
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    TrueColor,
    Ansi256,
}

impl ColorMode {
    /// `COLORTERM=truecolor` ou `24bit` : truecolor ; sinon 256 couleurs.
    pub fn detect(get: impl Fn(&str) -> Option<String>) -> Self {
        match get("COLORTERM").as_deref() {
            Some("truecolor") | Some("24bit") => ColorMode::TrueColor,
            _ => ColorMode::Ansi256,
        }
    }
}

/// Les 10 couleurs d'un thème, prêtes pour ratatui.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub main: Color,
    pub caret: Color,
    pub sub: Color,
    pub sub_alt: Color,
    pub text: Color,
    pub error: Color,
    pub error_extra: Color,
    pub colorful_error: Color,
    pub colorful_error_extra: Color,
    /// Couleur du caret en RGB, pour la séquence OSC 12 du curseur.
    pub caret_rgb: (u8, u8, u8),
}

impl Palette {
    pub fn from_theme(theme: &Theme, mode: ColorMode) -> Self {
        let black = Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        let bg = theme.bg.over(black);
        let c = |x: Rgba| to_color(x.over(bg), mode);
        let caret = theme.caret.over(bg);
        Palette {
            bg: to_color(bg, mode),
            main: c(theme.main),
            caret: c(theme.caret),
            sub: c(theme.sub),
            sub_alt: c(theme.sub_alt),
            text: c(theme.text),
            error: c(theme.error),
            error_extra: c(theme.error_extra),
            colorful_error: c(theme.colorful_error),
            colorful_error_extra: c(theme.colorful_error_extra),
            caret_rgb: (caret.r, caret.g, caret.b),
        }
    }
}

fn to_color(c: Rgba, mode: ColorMode) -> Color {
    match mode {
        ColorMode::TrueColor => Color::Rgb(c.r, c.g, c.b),
        ColorMode::Ansi256 => Color::Indexed(nearest_256(c.r, c.g, c.b)),
    }
}

fn oklab(r: u8, g: u8, b: u8) -> [f64; 3] {
    let lin = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(r), lin(g), lin(b));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    ]
}

/// RGB d'une couleur xterm 16..=255 (cube 6×6×6 puis 24 gris).
fn xterm_rgb(index: u8) -> (u8, u8, u8) {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    if index >= 232 {
        let v = 8 + 10 * (index - 232);
        return (v, v, v);
    }
    let i = index - 16;
    (
        LEVELS[usize::from(i / 36)],
        LEVELS[usize::from((i / 6) % 6)],
        LEVELS[usize::from(i % 6)],
    )
}

/// Couleur xterm (16..=255) la plus proche en distance OKLab.
pub fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    let target = oklab(r, g, b);
    let dist = |i: u8| {
        let (cr, cg, cb) = xterm_rgb(i);
        let c = oklab(cr, cg, cb);
        (0..3).map(|k| (c[k] - target[k]).powi(2)).sum::<f64>()
    };
    (16..=255u8)
        .min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
        .unwrap_or(16)
}
```

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test theme && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 4 tests PASS, aucun avertissement.

- [ ] **Step 6 : commit**

```bash
git add Cargo.toml Cargo.lock crates/fasttype-tui
git commit -m "feat(tui): crate fasttype-tui et couleurs des thèmes (truecolor ou 256 couleurs)"
```

---

### Task 2 : entrées clavier

**Files:**
- Create: `crates/fasttype-tui/src/input.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod input;`)
- Test: `crates/fasttype-tui/tests/input.rs`

**Interfaces:**
- Produces :
  - `Key { Char(char), Backspace, DeleteWord, Tab, BackTab, Enter, ShiftEnter, Esc, Quit }` et `Phase { Press, Repeat, Release }` ;
  - `Input { Key { key, phase, code: u32, at: f64 }, Resize }`, avec `Input::at() -> Option<f64>` ;
  - `map_key(&KeyEvent) -> Option<(Key, u32)>` et `map_event(Event, at: f64) -> Option<Input>` ;
  - `spawn_reader(Arc<SystemClock>, SyncSender<Input>) -> JoinHandle<()>`.
- Raccourcis :
  - Ctrl+C quitte ;
  - Ctrl/Alt+Backspace, Ctrl+W et Ctrl+H effacent le mot (Ctrl+Backspace arrive en Ctrl+H dans beaucoup de terminaux) ;
  - Shift+Entrée n'est reconnu qu'avec le protocole clavier Kitty.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/input.rs` :
```rust
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fasttype_tui::input::{Input, Key, Phase, map_event, map_key};

fn k(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new_with_kind(code, mods, KeyEventKind::Press)
}

#[test]
fn maps_typing_keys() {
    assert_eq!(
        map_key(&k(KeyCode::Char('a'), KeyModifiers::NONE))
            .unwrap()
            .0,
        Key::Char('a')
    );
    assert_eq!(
        map_key(&k(KeyCode::Char('A'), KeyModifiers::SHIFT))
            .unwrap()
            .0,
        Key::Char('A')
    );
    assert_eq!(
        map_key(&k(KeyCode::Char('é'), KeyModifiers::NONE))
            .unwrap()
            .0,
        Key::Char('é')
    );
    assert_eq!(
        map_key(&k(KeyCode::Backspace, KeyModifiers::NONE))
            .unwrap()
            .0,
        Key::Backspace
    );
}

#[test]
fn maps_shortcuts() {
    assert_eq!(
        map_key(&k(KeyCode::Char('c'), KeyModifiers::CONTROL))
            .unwrap()
            .0,
        Key::Quit
    );
    assert_eq!(
        map_key(&k(KeyCode::Backspace, KeyModifiers::CONTROL))
            .unwrap()
            .0,
        Key::DeleteWord
    );
    assert_eq!(
        map_key(&k(KeyCode::Backspace, KeyModifiers::ALT))
            .unwrap()
            .0,
        Key::DeleteWord
    );
    assert_eq!(
        map_key(&k(KeyCode::Char('w'), KeyModifiers::CONTROL))
            .unwrap()
            .0,
        Key::DeleteWord
    );
    assert_eq!(
        map_key(&k(KeyCode::Enter, KeyModifiers::SHIFT)).unwrap().0,
        Key::ShiftEnter
    );
    assert_eq!(
        map_key(&k(KeyCode::Tab, KeyModifiers::NONE)).unwrap().0,
        Key::Tab
    );
    assert_eq!(
        map_key(&k(KeyCode::Esc, KeyModifiers::NONE)).unwrap().0,
        Key::Esc
    );
    assert!(map_key(&k(KeyCode::Char('x'), KeyModifiers::CONTROL)).is_none());
    assert!(map_key(&k(KeyCode::F(1), KeyModifiers::NONE)).is_none());
}

#[test]
fn press_and_release_share_a_code() {
    let (_, down) = map_key(&k(KeyCode::Char('A'), KeyModifiers::SHIFT)).unwrap();
    let (_, up) = map_key(&KeyEvent::new_with_kind(
        KeyCode::Char('a'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ))
    .unwrap();
    assert_eq!(down, up);
}

#[test]
fn events_carry_phase_and_time() {
    let ev = Event::Key(KeyEvent::new_with_kind(
        KeyCode::Char('a'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
    ));
    assert_eq!(
        map_event(ev, 12.5),
        Some(Input::Key {
            key: Key::Char('a'),
            phase: Phase::Release,
            code: 'a' as u32,
            at: 12.5
        })
    );
    assert_eq!(map_event(Event::Resize(80, 24), 0.0), Some(Input::Resize));
    assert_eq!(map_event(Event::FocusGained, 0.0), None);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test input`
Expected: échec de compilation (`unresolved import fasttype_tui::input`).

- [ ] **Step 3 : implémenter `input.rs`**

```rust
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
    /// Ctrl+C.
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Press,
    Repeat,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input {
    Key {
        key: Key,
        phase: Phase,
        code: u32,
        at: f64,
    },
    Resize,
}

impl Input {
    /// Horodatage d'une touche (pour mesurer la latence touche → écran).
    pub fn at(&self) -> Option<f64> {
        match self {
            Input::Key { at, .. } => Some(*at),
            Input::Resize => None,
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
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test input && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 4 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): touches horodatées lues dans un thread dédié"
```

---

### Task 3 : placement des mots

**Files:**
- Create: `crates/fasttype-tui/src/layout.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod layout;`)
- Test: `crates/fasttype-tui/tests/layout.rs`

**Interfaces:**
- Produces :
  - `char_width(char) -> u16`, `letters(&str) -> &str`, `extras<'a>(target, input: &'a str) -> &'a str`, `word_cells(target, input) -> u16` et `prefix_cells(target, input, n) -> u16` ;
  - `WordBox { index, x, width }` et `Layout { lines: Vec<Vec<WordBox>>, caret: (usize, u16) }`, avec `first_visible()` ;
  - `layout_words(words, inputs, active, width) -> Layout`.
- Règles :
  - un mot n'est jamais coupé ;
  - les lettres en trop élargissent le mot ;
  - un mot finissant par `\n` termine la ligne ;
  - la fenêtre de 3 lignes garde le caret sur la 2e ligne après le premier saut.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/layout.rs` :
```rust
use fasttype_tui::layout::{extras, layout_words, letters, prefix_cells, word_cells};

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn letters_and_extras() {
    assert_eq!(letters("hello "), "hello");
    assert_eq!(letters("line\n"), "line");
    assert_eq!(extras("ab ", "abcd"), "cd");
    assert_eq!(extras("ab ", "abcd "), "cd");
    assert_eq!(extras("ab ", "a"), "");
}

#[test]
fn widths_count_cells_not_bytes() {
    assert_eq!(word_cells("café ", ""), 4);
    assert_eq!(word_cells("日本 ", ""), 4);
    assert_eq!(
        word_cells("ab ", "abxx"),
        4,
        "les lettres en trop élargissent le mot"
    );
    assert_eq!(prefix_cells("日本 ", "日", 1), 2);
    assert_eq!(prefix_cells("ab ", "abxx", 3), 3);
}

#[test]
fn words_wrap_without_splitting() {
    let words = s(&["aa ", "bb ", "cc ", "dd"]);
    let l = layout_words(&words, &s(&["", "", "", ""]), 0, 5);
    let lines: Vec<Vec<usize>> = l
        .lines
        .iter()
        .map(|line| line.iter().map(|b| b.index).collect())
        .collect();
    assert_eq!(lines, [vec![0, 1], vec![2, 3]]);
    assert_eq!(l.lines[0][1].x, 3);
}

#[test]
fn caret_follows_input_and_extras() {
    let words = s(&["aa ", "bb ", "cc"]);
    let l = layout_words(&words, &s(&["aa ", "b", ""]), 1, 20);
    assert_eq!(l.caret, (0, 4));
    let l = layout_words(&words, &s(&["aa ", "bbxx", ""]), 1, 20);
    assert_eq!(l.caret, (0, 7));
    assert_eq!(
        l.lines[0][2].x, 8,
        "le mot suivant est poussé par les lettres en trop"
    );
}

#[test]
fn newline_word_ends_the_line() {
    let words = s(&["a\n", "b "]);
    let l = layout_words(&words, &s(&["", ""]), 0, 20);
    assert_eq!(l.lines.len(), 2);
}

#[test]
fn window_keeps_caret_on_second_line() {
    let words = s(&["aa ", "bb ", "cc ", "dd ", "ee"]);
    let inputs = s(&["aa ", "bb ", "cc ", "dd ", ""]);
    // largeur 2 : un mot par ligne
    let l = layout_words(&words, &inputs, 0, 2);
    assert_eq!(l.first_visible(), 0);
    let l = layout_words(&words, &inputs, 1, 2);
    assert_eq!(
        l.first_visible(),
        0,
        "le premier saut de ligne ne fait pas défiler"
    );
    let l = layout_words(&words, &inputs, 3, 2);
    assert_eq!(l.first_visible(), 2);
}

#[test]
fn word_wider_than_line_gets_its_own_line() {
    let words = s(&["a ", "abcdefghij ", "b"]);
    let l = layout_words(&words, &s(&["", "", ""]), 1, 4);
    let lines: Vec<Vec<usize>> = l
        .lines
        .iter()
        .map(|line| line.iter().map(|b| b.index).collect())
        .collect();
    assert_eq!(lines, [vec![0], vec![1], vec![2]]);
    assert_eq!(l.caret, (1, 0));
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test layout`
Expected: échec de compilation (`unresolved import fasttype_tui::layout`).

- [ ] **Step 3 : implémenter `layout.rs`**

```rust
//! Placement des mots en lignes, comme le retour à la ligne du site : un mot
//! ne se coupe pas, les lettres en trop élargissent le mot, et le caret suit la
//! saisie. Les largeurs sont en cases de terminal (CJK = 2 cases).

use unicode_width::UnicodeWidthChar;

/// Largeur d'affichage d'un caractère (0 pour un caractère de contrôle).
pub fn char_width(c: char) -> u16 {
    c.width().unwrap_or(0) as u16
}

/// Lettres affichées d'un mot cible : sans le séparateur final ni le saut de ligne.
pub fn letters(target: &str) -> &str {
    target
        .strip_suffix(' ')
        .or_else(|| target.strip_suffix('\n'))
        .unwrap_or(target)
}

/// Lettres de saisie au-delà de la cible (hors séparateur) : affichées en « extra ».
pub fn extras<'a>(target: &str, input: &'a str) -> &'a str {
    let n = letters(target).chars().count();
    let typed = letters(input);
    typed.char_indices().nth(n).map_or("", |(i, _)| &typed[i..])
}

/// Largeur d'un mot à l'écran : lettres cibles puis lettres en trop.
pub fn word_cells(target: &str, input: &str) -> u16 {
    letters(target)
        .chars()
        .chain(extras(target, input).chars())
        .map(char_width)
        .sum()
}

/// Largeur des `n` premières positions saisies : la cible tant qu'il y en a, puis la saisie.
pub fn prefix_cells(target: &str, input: &str, n: usize) -> u16 {
    letters(target)
        .chars()
        .chain(extras(target, input).chars())
        .take(n)
        .map(char_width)
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordBox {
    pub index: usize,
    pub x: u16,
    pub width: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub lines: Vec<Vec<WordBox>>,
    /// Ligne et colonne du caret.
    pub caret: (usize, u16),
}

impl Layout {
    /// Première ligne visible : le caret reste sur la deuxième ligne une fois le
    /// premier saut passé (Monkeytype retire la ligne du haut en arrivant à la 3e).
    pub fn first_visible(&self) -> usize {
        self.caret.0.saturating_sub(1)
    }
}

/// Place tous les mots sur des lignes de `width` cases.
pub fn layout_words(words: &[String], inputs: &[String], active: usize, width: u16) -> Layout {
    let mut lines: Vec<Vec<WordBox>> = vec![Vec::new()];
    let mut x: u16 = 0;
    let mut caret = (0, 0);
    for (index, target) in words.iter().enumerate() {
        let input = inputs.get(index).map_or("", String::as_str);
        let w = word_cells(target, input);
        if x > 0 && x.saturating_add(w) > width {
            lines.push(Vec::new());
            x = 0;
        }
        let line = lines.len() - 1;
        lines[line].push(WordBox { index, x, width: w });
        if index == active {
            let typed = letters(input).chars().count();
            caret = (line, x + prefix_cells(target, input, typed));
        }
        x = x.saturating_add(w + 1);
        if target.ends_with('\n') {
            lines.push(Vec::new());
            x = 0;
        }
    }
    if lines.last().is_some_and(Vec::is_empty) && lines.len() > 1 {
        lines.pop();
    }
    Layout { lines, caret }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test layout && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 7 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): placement des mots en lignes, caret et fenêtre de trois lignes"
```

---

### Task 4 : mesures de fluidité

**Files:**
- Create: `crates/fasttype-tui/src/perf.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod perf;`)
- Test: `crates/fasttype-tui/tests/perf.rs`

**Interfaces:**
- Produces : `LatencyStats::{record(ms), count(), percentile(p) -> Option<f64>}` (1024 derniers échantillons) et `Perf { enabled, input_to_flush, frame }`, avec `Perf::new(bool)` et `summary() -> String`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/perf.rs` :
```rust
use fasttype_tui::perf::{LatencyStats, Perf};

#[test]
fn percentiles_by_nearest_rank() {
    let mut s = LatencyStats::default();
    assert_eq!(s.percentile(50.0), None);
    for v in 1..=100 {
        s.record(f64::from(v));
    }
    assert_eq!(s.percentile(50.0), Some(50.0));
    assert_eq!(s.percentile(99.0), Some(99.0));
    assert_eq!(s.percentile(100.0), Some(100.0));
}

#[test]
fn keeps_only_recent_samples() {
    let mut s = LatencyStats::default();
    for _ in 0..1024 {
        s.record(100.0);
    }
    for _ in 0..1024 {
        s.record(1.0);
    }
    assert_eq!(s.count(), 1024);
    assert_eq!(s.percentile(99.0), Some(1.0));
}

#[test]
fn summary_mentions_latency() {
    let mut p = Perf::new(true);
    p.input_to_flush.record(0.5);
    assert!(p.summary().contains("p99 0.50"), "{}", p.summary());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test perf`
Expected: échec de compilation (`unresolved import fasttype_tui::perf`).

- [ ] **Step 3 : implémenter `perf.rs`**

```rust
//! Mesures de fluidité pour `fasttype --perf` : latence touche → écran et
//! temps de calcul d'une image, sur les 1024 derniers échantillons.

const CAPACITY: usize = 1024;

#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    samples: Vec<f64>,
    next: usize,
}

impl LatencyStats {
    pub fn record(&mut self, ms: f64) {
        if self.samples.len() < CAPACITY {
            self.samples.push(ms);
        } else {
            self.samples[self.next] = ms;
        }
        self.next = (self.next + 1) % CAPACITY;
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// Percentile `p` (0 à 100), par rang le plus proche.
    pub fn percentile(&self, p: f64) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(f64::total_cmp);
        let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
        Some(sorted[rank.min(sorted.len()) - 1])
    }
}

#[derive(Debug, Clone, Default)]
pub struct Perf {
    pub enabled: bool,
    pub input_to_flush: LatencyStats,
    pub frame: LatencyStats,
}

impl Perf {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }

    /// Ligne affichée en surimpression.
    pub fn summary(&self) -> String {
        let fmt = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.2}"));
        format!(
            "key→flush p50 {} p99 {} ms · frame p99 {} ms · n {}",
            fmt(self.input_to_flush.percentile(50.0)),
            fmt(self.input_to_flush.percentile(99.0)),
            fmt(self.frame.percentile(99.0)),
            self.input_to_flush.count()
        )
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test perf && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 3 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): mesures de latence et de temps d'image pour --perf"
```

---

### Task 5 : construction des tests depuis la config

**Files:**
- Create: `crates/fasttype-tui/src/session_factory.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod session_factory;`)
- Test: `crates/fasttype-tui/tests/factory.rs`

**Interfaces:**
- Consumes : `fasttype_core` (générateur, sources, spec, session), `fasttype_data::{LanguageCache, quotes_for, DEFAULT_LANGUAGE}`, `fasttype_store::Config`.
- Produces :
  - `DEFAULT_CUSTOM_TEXT` ;
  - `Built { session: TestSession, warning: Option<String> }` ;
  - `SessionFactory::{new(), build(&mut self, &Config, seed: u64) -> Built}` ;
  - `pick_quote(&QuoteFile, groups: &[i64], &mut dyn RandomSource) -> Option<&Quote>`.
- Modes :
  - time, words et zen, comme dans Monkeytype ;
  - quote : citation tirée parmi les groupes de `quoteLength` ;
  - custom : texte par défaut de Monkeytype en mode repeat. L'édition du texte arrive en 4c.
- Les citations décompressées restent en cache (point mineur reporté du plan 2).

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/factory.rs` :
```rust
use fasttype_core::quote::QuoteFile;
use fasttype_core::rng::Scripted;
use fasttype_core::spec::Mode;
use fasttype_store::Config;
use fasttype_tui::session_factory::{SessionFactory, pick_quote};

fn config(toml: &str) -> Config {
    let (c, w) = Config::from_toml(toml);
    assert!(w.is_empty(), "{w:?}");
    c
}

#[test]
fn builds_each_mode() {
    let mut f = SessionFactory::new();
    for (mode, expected) in [
        ("time", Mode::Time),
        ("words", Mode::Words),
        ("zen", Mode::Zen),
        ("custom", Mode::Custom),
        ("quote", Mode::Quote),
    ] {
        let built = f.build(&config(&format!("mode = \"{mode}\"\n")), 7);
        assert_eq!(built.session.spec().mode, expected, "{mode}");
        assert!(built.warning.is_none(), "{mode} : {:?}", built.warning);
    }
}

#[test]
fn words_mode_has_exactly_n_words() {
    let built = SessionFactory::new().build(&config("mode = \"words\"\nwords = 10\n"), 1);
    assert_eq!(built.session.words().len(), 10);
}

#[test]
fn same_seed_same_words() {
    let a = SessionFactory::new().build(&Config::defaults(), 99);
    let b = SessionFactory::new().build(&Config::defaults(), 99);
    assert_eq!(a.session.words(), b.session.words());
}

#[test]
fn quote_groups_follow_quote_length() {
    let file = QuoteFile::from_json(
        br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"short","source":"a","length":5,"id":1},{"text":"long","source":"b","length":400,"id":2}]}"#,
    )
    .unwrap();
    assert_eq!(
        pick_quote(&file, &[2], &mut Scripted::new(&[0.0])).map(|q| q.id),
        Some(2)
    );
    assert_eq!(
        pick_quote(&file, &[0], &mut Scripted::new(&[0.0])).map(|q| q.id),
        Some(1)
    );
    assert_eq!(pick_quote(&file, &[3], &mut Scripted::new(&[0.0])), None);
    assert!(
        pick_quote(&file, &[-3], &mut Scripted::new(&[0.0])).is_some(),
        "favoris non gérés : toutes"
    );
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test factory`
Expected: échec de compilation (`unresolved import fasttype_tui::session_factory`).

- [ ] **Step 3 : implémenter `session_factory.rs`**

```rust
//! Construction d'un test à partir de la config : mode, langue, citation.
//! Les langues et les citations chargées restent en mémoire.

use fasttype_core::generator::WordGenerator;
use fasttype_core::quote::{Quote, QuoteFile, quote_words};
use fasttype_core::rng::{RandomSource, SplitMix64};
use fasttype_core::session::TestSession;
use fasttype_core::sources::{CustomMode, CustomWords, RandomWords, SequenceWords};
use fasttype_core::spec::{CustomLimit, QuoteMeta, TestSpec};
use fasttype_data::{DEFAULT_LANGUAGE, LanguageCache, quotes_for};
use fasttype_store::Config;
use std::collections::HashMap;
use std::sync::Arc;

/// Texte custom par défaut de Monkeytype.
pub const DEFAULT_CUSTOM_TEXT: &str = "The quick brown fox jumps over the lazy dog";

pub struct Built {
    pub session: TestSession,
    /// Repli effectué (langue ou citations introuvables), à notifier.
    pub warning: Option<String>,
}

#[derive(Default)]
pub struct SessionFactory {
    languages: LanguageCache,
    quotes: HashMap<String, Option<Arc<QuoteFile>>>,
}

/// Citation tirée parmi les groupes de `quoteLength` (0 à 3) ; tous si aucun.
pub fn pick_quote<'a>(
    file: &'a QuoteFile,
    groups: &[i64],
    rng: &mut dyn RandomSource,
) -> Option<&'a Quote> {
    let wanted: Vec<u8> = groups
        .iter()
        .filter(|g| (0..=3).contains(*g))
        .map(|g| *g as u8)
        .collect();
    let candidates: Vec<&Quote> = file
        .quotes
        .iter()
        .filter(|q| wanted.is_empty() || file.group_of(q).is_some_and(|g| wanted.contains(&g)))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(candidates[rng.below(candidates.len())])
}

impl SessionFactory {
    pub fn new() -> Self {
        Self::default()
    }

    fn quotes(&mut self, language: &str) -> Option<Arc<QuoteFile>> {
        self.quotes
            .entry(language.to_string())
            .or_insert_with(|| quotes_for(language).ok().flatten().map(Arc::new))
            .clone()
    }

    /// Construit le test décrit par la config. Ne échoue jamais : une langue
    /// inconnue repasse sur `english`, un mode sans citations sur `time`.
    pub fn build(&mut self, config: &Config, seed: u64) -> Built {
        let mut rng = SplitMix64::new(seed);
        let session_rng = Box::new(SplitMix64::new(seed ^ 0x9E37_79B9_7F4A_7C15));
        let mut warning = None;
        let requested = config.str("language");
        let (language, words) = match self.languages.get(requested) {
            Ok(l) => (requested.to_string(), Arc::clone(&l.words)),
            Err(e) => {
                warning = Some(format!("{e} — using {DEFAULT_LANGUAGE}"));
                match self.languages.get(DEFAULT_LANGUAGE) {
                    Ok(l) => (DEFAULT_LANGUAGE.to_string(), Arc::clone(&l.words)),
                    Err(_) => (DEFAULT_LANGUAGE.to_string(), Arc::new(Vec::new())),
                }
            }
        };
        let punctuation = config.bool("punctuation");
        let numbers = config.bool("numbers");
        let random = |max: Option<u32>| {
            let source = RandomWords::new(Arc::clone(&words), &language, punctuation, numbers, max);
            WordGenerator::new(Box::new(source), &language, punctuation, numbers)
        };
        let time = config.int("time").max(0) as u32;
        let (spec, generator) = match config.str("mode") {
            "words" => {
                let n = config.int("words").max(0) as u32;
                (
                    TestSpec::words(n, &language, punctuation, numbers),
                    random(Some(n)),
                )
            }
            "zen" => (TestSpec::zen(&language), WordGenerator::empty()),
            "custom" => {
                let limit =
                    CustomLimit::Word(DEFAULT_CUSTOM_TEXT.split_whitespace().count() as u32);
                let source =
                    CustomWords::new(DEFAULT_CUSTOM_TEXT, CustomMode::Repeat, limit, false);
                (
                    TestSpec::custom(limit, &language, punctuation, numbers),
                    WordGenerator::new(Box::new(source), &language, punctuation, numbers),
                )
            }
            "quote" => {
                let picked = self.quotes(&language).and_then(|file| {
                    let q = pick_quote(&file, &config.int_list("quoteLength"), &mut rng)?;
                    let meta = QuoteMeta {
                        id: q.id,
                        group: file.group_of(q).unwrap_or(0),
                        source: q.source.clone(),
                    };
                    Some((meta, quote_words(&q.text)))
                });
                match picked {
                    Some((meta, words)) => (
                        TestSpec::quote(meta, &language),
                        WordGenerator::new(
                            Box::new(SequenceWords::new(words)),
                            &language,
                            false,
                            false,
                        ),
                    ),
                    None => {
                        warning = Some(format!("no quotes found for {language} — using time mode"));
                        (
                            TestSpec::time(time, &language, punctuation, numbers),
                            random(None),
                        )
                    }
                }
            }
            _ => (
                TestSpec::time(time, &language, punctuation, numbers),
                random(None),
            ),
        };
        Built {
            session: TestSession::new(spec, generator, session_rng),
            warning,
        }
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test factory && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 4 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): construction des tests depuis la config, avec replis"
```

---

### Task 6 : application et écrans

**Files:**
- Create: `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/view/mod.rs`, `crates/fasttype-tui/src/view/notify.rs`, `crates/fasttype-tui/src/view/test.rs`, `crates/fasttype-tui/src/view/result.rs`, `crates/fasttype-tui/tests/common/mod.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod app;` et `pub mod view;`)
- Test: `crates/fasttype-tui/tests/app.rs`, `crates/fasttype-tui/tests/view.rs`

**Interfaces:**
- Consumes : `Input`, `Key`, `Phase`, `Palette`, `ColorMode`, `Perf`, `SessionFactory`, `layout`, `Store` et `RecordOutcome`.
- Produces :
  - `Screen { Test, Result(Box<ResultInfo>) }` et `ResultInfo { result: TestResult, outcome: Option<RecordOutcome> }` ;
  - `CaretShape { Bar, Block, Underline }` et `CaretLook { shape, blinking, rgb }` ;
  - `App`, avec :
    - `new(Store, ColorMode, now, seed)` ;
    - `handle(Input)`, `tick(now)` et `next_deadline() -> Option<f64>` ;
    - `draw(&mut Frame, Option<&Perf>)` ;
    - `caret_look() -> Option<CaretLook>`, `summary()` et `timer()` ;
    - les accesseurs `session()`, `screen()` et `palette()`, et les champs publics `store`, `notifications` et `quit` ;
  - `view::notify::{Level, Notification, Notifications}`, `view::test::TestView`, et dans `view::result` : `ResultView`, `unit_factor`, `invalid_label` et `test_type`.
- Comportement (Monkeytype) :
  - `tab` puis `enter` relance, et toute autre touche annule ;
  - `quickRestart` tab, esc ou enter relance directement, mais pas pendant un test long sans Shift ;
  - le test démarre à la première lettre ;
  - le résultat est calculé et enregistré à la fin, et un test invalide est annoncé (« Test invalid - … ») ;
  - en-tête et raccourcis sont masqués pendant la frappe (focus mode simple), le timer « mini » est affiché ;
  - lettres colorées selon `flipTestColors` et `colorfulMode` ;
  - mot faux validé souligné en couleur d'erreur.

- [ ] **Step 1 : outil de test partagé**

`crates/fasttype-tui/tests/common/mod.rs` :
```rust
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

/// Rend l'application dans un terminal de test ; renvoie le tampon et le caret.
pub fn render(app: &App, w: u16, h: u16) -> (Buffer, Option<(u16, u16)>) {
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
```

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/app.rs` :
```rust
mod common;

use common::{app, press, release, type_text, type_whole_test};
use fasttype_core::event::EventKind;
use fasttype_core::session::SessionState;
use fasttype_core::spec::Mode;
use fasttype_store::RecordOutcome;
use fasttype_store::pbs::PbOutcome;
use fasttype_tui::app::Screen;
use fasttype_tui::input::Key;

#[test]
fn starts_on_a_test_of_english_words() {
    let a = app("start", "");
    assert!(matches!(a.screen(), Screen::Test));
    assert_eq!(a.session().spec().mode, Mode::Time);
    let english = fasttype_data::load_language("english").unwrap();
    for w in a.session().words().iter().take(20) {
        assert!(english.words.contains(&w.trim_end().to_string()), "{w:?}");
    }
    assert!(a.notifications.items().is_empty());
}

#[test]
fn a_finished_words_test_is_saved_with_a_personal_best() {
    let mut a = app("words", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 1000.0, 300.0);
    let Screen::Result(info) = a.screen() else {
        panic!("écran de résultat attendu")
    };
    assert_eq!(info.result.invalid, None, "{:?}", info.result);
    assert_eq!(
        info.outcome,
        Some(RecordOutcome::Saved(PbOutcome::NewBest { previous: None }))
    );
    assert_eq!(a.store.history().unwrap().results.len(), 1);
}

#[test]
fn tab_then_enter_restarts() {
    let mut a = app("tabenter", "");
    let first = a.session().words().to_vec();
    type_text(&mut a, "x", 0.0, 100.0);
    assert_eq!(a.session().state(), SessionState::Running);
    a.handle(press(Key::Tab, 200.0));
    a.handle(press(Key::Enter, 300.0));
    assert_eq!(a.session().state(), SessionState::Ready);
    assert_ne!(a.session().words(), first.as_slice(), "nouveaux mots");
}

#[test]
fn another_key_cancels_tab() {
    let mut a = app("disarm", "");
    let first = a.session().words().to_vec();
    a.handle(press(Key::Tab, 0.0));
    type_text(&mut a, "x", 10.0, 100.0);
    a.handle(press(Key::Enter, 200.0));
    assert_eq!(a.session().words(), first.as_slice());
    assert_eq!(a.session().state(), SessionState::Running);
}

#[test]
fn quick_restart_on_tab() {
    let mut a = app("quicktab", "quick_restart = \"tab\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.handle(press(Key::Tab, 200.0));
    assert_eq!(a.session().state(), SessionState::Ready);
}

#[test]
fn long_tests_need_shift_to_quick_restart() {
    let mut a = app("long", "quick_restart = \"tab\"\ntime = 0\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.handle(press(Key::Tab, 200.0));
    assert_eq!(a.session().state(), SessionState::Running);
    assert!(
        a.notifications.items()[0]
            .text
            .contains("Quick restart disabled")
    );
    a.handle(press(Key::BackTab, 300.0));
    assert_eq!(a.session().state(), SessionState::Ready);
}

#[test]
fn ctrl_c_quits() {
    let mut a = app("quit", "");
    a.handle(press(Key::Quit, 0.0));
    assert!(a.quit);
}

#[test]
fn unknown_language_falls_back_to_english() {
    let a = app("lang", "language = \"klingon_9000k\"\n");
    assert_eq!(a.session().spec().language, "english");
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("english"))
    );
}

#[test]
fn unknown_theme_falls_back_to_serika_dark() {
    let a = app("theme", "theme = \"nope\"\n");
    let serika = fasttype_tui::theme::Palette::from_theme(
        fasttype_data::theme("serika_dark").unwrap(),
        fasttype_tui::theme::ColorMode::TrueColor,
    );
    assert_eq!(*a.palette(), serika);
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("serika_dark"))
    );
}

#[test]
fn custom_theme_colors_are_used() {
    let a = app(
        "customtheme",
        "custom_theme = true\ncustom_theme_colors = [\"#000000\", \"#ff0000\", \"#ff0000\", \"#111111\", \"#222222\", \"#ffffff\", \"#00ff00\", \"#008800\", \"#00ff00\", \"#008800\"]\n",
    );
    assert_eq!(a.palette().main, ratatui::style::Color::Rgb(255, 0, 0));
}

#[test]
fn zen_ends_with_shift_enter() {
    let mut a = app("zen", "mode = \"zen\"\n");
    type_text(&mut a, "hi ", 0.0, 100.0);
    a.handle(press(Key::ShiftEnter, 500.0));
    assert!(matches!(a.screen(), Screen::Result(_)));
}

#[test]
fn time_test_ends_on_tick() {
    let mut a = app("time", "time = 15\n");
    type_text(&mut a, "x", 0.0, 100.0);
    assert_eq!(a.next_deadline(), Some(1000.0));
    a.tick(15_000.0);
    assert!(matches!(a.screen(), Screen::Result(_)));
}

#[test]
fn idle_test_needs_no_wakeup() {
    let a = app("idle", "");
    assert_eq!(a.next_deadline(), None, "0 % de CPU tant que rien ne bouge");
}

#[test]
fn key_release_is_logged() {
    let mut a = app("keyup", "");
    a.handle(press(Key::Char('x'), 0.0));
    a.handle(release(Key::Char('x'), 50.0));
    assert!(
        a.session()
            .log()
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::KeyUp { .. }))
    );
}

#[test]
fn result_screen_tab_enter_starts_next_test() {
    let mut a = app("next", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 0.0, 300.0);
    a.handle(press(Key::Tab, 99_000.0));
    a.handle(press(Key::Enter, 99_100.0));
    assert!(matches!(a.screen(), Screen::Test));
    assert_eq!(a.session().state(), SessionState::Ready);
}

#[test]
fn quote_mode_types_a_quote() {
    let a = app("quote", "mode = \"quote\"\n");
    assert_eq!(a.session().spec().mode, Mode::Quote);
    assert!(a.session().spec().quote.is_some());
}

#[test]
fn invalid_result_is_announced_and_not_saved() {
    let mut a = app("invalid", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 0.0, 5.0);
    let Screen::Result(info) = a.screen() else {
        panic!()
    };
    assert!(info.result.invalid.is_some());
    assert_eq!(info.outcome, Some(RecordOutcome::Invalid));
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.starts_with("Test invalid"))
    );
}
```

`crates/fasttype-tui/tests/view.rs` :
```rust
mod common;

use common::{app, render, row, screen_text, type_text, type_whole_test};

#[test]
fn test_screen_shows_header_words_and_tips() {
    let a = app("v-start", "");
    let (buf, caret) = render(&a, 80, 24);
    let text = screen_text(&buf);
    assert!(row(&buf, 1).contains("fasttype"));
    assert!(row(&buf, 1).contains("time 30 · english"));
    assert!(text.contains("tab + enter - restart"));
    let first = a.session().word(0).trim_end().to_string();
    let words_row = (0..24)
        .find(|&y| row(&buf, y).contains(&first))
        .expect("premier mot affiché");
    let x = row(&buf, words_row).find(&first).unwrap();
    assert_eq!(
        caret,
        Some((x as u16, words_row)),
        "caret avant la première lettre"
    );
}

#[test]
fn letters_are_colored_by_correctness() {
    let mut a = app("v-colors", "");
    let target: Vec<char> = a.session().word(0).chars().collect();
    let wrong = if target[1] == 'z' { 'y' } else { 'z' };
    type_text(&mut a, &format!("{}{}", target[0], wrong), 0.0, 100.0);
    let (buf, caret) = render(&a, 80, 24);
    let (cx, cy) = caret.unwrap();
    let p = *a.palette();
    assert_eq!(buf[(cx - 2, cy)].fg, p.text, "lettre juste");
    assert_eq!(buf[(cx - 1, cy)].fg, p.error, "lettre fausse");
    assert_eq!(
        buf[(cx - 1, cy)].symbol(),
        target[1].to_string(),
        "la lettre attendue est affichée"
    );
    assert_eq!(buf[(cx, cy)].fg, p.sub, "lettre à venir");
}

#[test]
fn focus_mode_hides_chrome_and_shows_timer() {
    let mut a = app("v-focus", "");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 100.0);
    let (buf, _) = render(&a, 80, 24);
    let text = screen_text(&buf);
    assert!(!text.contains("fasttype"));
    assert!(!text.contains("restart"));
    assert!(text.contains("30"), "timer mini");
}

#[test]
fn too_small_terminal_says_so() {
    let a = app("v-small", "");
    let (buf, caret) = render(&a, 30, 8);
    assert!(screen_text(&buf).contains("terminal too small"));
    assert_eq!(caret, None);
}

#[test]
fn result_screen_shows_speed_and_details() {
    let mut a = app("v-result", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 0.0, 300.0);
    let (buf, caret) = render(&a, 100, 24);
    let text = screen_text(&buf);
    assert!(text.contains("wpm"), "{text}");
    assert!(text.contains("acc"));
    assert!(text.contains("characters"));
    assert!(text.contains("test type words 10 english"));
    assert!(text.contains("new personal best"));
    assert!(text.contains("next test"));
    assert_eq!(caret, None);
}

#[test]
fn background_is_painted() {
    let a = app("v-bg", "");
    let (buf, _) = render(&a, 80, 24);
    assert_eq!(buf[(0, 0)].bg, a.palette().bg);
    assert_eq!(buf[(79, 23)].bg, a.palette().bg);
}

#[test]
fn resize_keeps_caret_after_the_typed_letters() {
    let mut a = app("v-resize", "");
    let word: String = a.session().word(0).trim_end().to_string();
    let typed: String = word.chars().take(2).collect();
    type_text(&mut a, &typed, 0.0, 100.0);
    for (w, h) in [(80, 24), (50, 12), (120, 40)] {
        let (buf, caret) = render(&a, w, h);
        let (cx, cy) = caret.expect("caret visible");
        let before: String = (cx - 2..cx)
            .map(|x| buf[(x, cy)].symbol().to_string())
            .collect();
        assert_eq!(before, typed, "{w}×{h}");
    }
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test app --test view`
Expected: échec de compilation (`unresolved import fasttype_tui::app`).

- [ ] **Step 4 : implémenter les vues**

`crates/fasttype-tui/src/view/mod.rs` :
```rust
//! Rendu des écrans dans un `Buffer` ratatui (sans E/S : testable hors terminal).

pub mod notify;
pub mod result;
pub mod test;

use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

/// Taille minimale utilisable (spec §8).
pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 10;

/// Peint tout l'écran avec le fond du thème.
pub fn fill_background(buf: &mut Buffer, area: Rect, palette: &Palette) {
    buf.set_style(area, Style::default().bg(palette.bg).fg(palette.sub));
}

/// Écrit des segments stylés, centrés sur la ligne `y`.
pub fn centered_segments(buf: &mut Buffer, area: Rect, y: u16, segments: &[(String, Style)]) {
    let width: usize = segments.iter().map(|(s, _)| s.width()).sum();
    let mut x = area.x + area.width.saturating_sub(width as u16) / 2;
    for (text, style) in segments {
        buf.set_string(x, y, text, *style);
        x += text.width() as u16;
    }
}

pub fn too_small(buf: &mut Buffer, area: Rect, palette: &Palette) {
    let y = area.y + area.height / 2;
    let msg = format!("terminal too small ({}×{} min)", MIN_WIDTH, MIN_HEIGHT);
    centered_segments(buf, area, y, &[(msg, Style::default().fg(palette.text))]);
}
```

`crates/fasttype-tui/src/view/notify.rs` :
```rust
//! Notifications en pile en haut à droite (3 s ; 6 s pour les erreurs).

use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Notice,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub text: String,
    pub level: Level,
    pub until: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Notifications {
    items: Vec<Notification>,
}

impl Notifications {
    pub fn push(&mut self, text: impl Into<String>, level: Level, now: f64) {
        let ttl = match level {
            Level::Notice => 3000.0,
            Level::Error => 6000.0,
        };
        self.items.insert(
            0,
            Notification {
                text: text.into(),
                level,
                until: now + ttl,
            },
        );
        self.items.truncate(5);
    }

    pub fn expire(&mut self, now: f64) {
        self.items.retain(|n| n.until > now);
    }

    pub fn next_expiry(&self) -> Option<f64> {
        self.items.iter().map(|n| n.until).min_by(f64::total_cmp)
    }

    pub fn items(&self) -> &[Notification] {
        &self.items
    }

    pub fn render(&self, buf: &mut Buffer, area: Rect, palette: &Palette) {
        for (i, n) in self.items.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            if y >= area.bottom() {
                break;
            }
            let max = area.width.saturating_sub(4) as usize;
            let text: String = n.text.chars().take(max).collect();
            let w = text.width() as u16 + 2;
            let x = area.right().saturating_sub(w + 1);
            let fg = match n.level {
                Level::Notice => palette.text,
                Level::Error => palette.error,
            };
            let style = Style::default().fg(fg).bg(palette.sub_alt);
            buf.set_string(x, y, format!(" {text} "), style);
        }
    }
}
```

`crates/fasttype-tui/src/view/test.rs` :
```rust
//! Écran de test : en-tête, timer, trois lignes de mots, raccourcis.

use crate::layout::{Layout, char_width, extras, layout_words, letters};
use crate::theme::Palette;
use crate::view::centered_segments;
use fasttype_core::session::{SessionState, TestSession};
use fasttype_core::spec::Mode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

pub struct TestView<'a> {
    pub session: &'a TestSession,
    pub palette: &'a Palette,
    pub flip_test_colors: bool,
    pub colorful_mode: bool,
    /// `maxLineWidth` : 0 = automatique.
    pub max_line_width: u16,
    /// Résumé de la config affiché dans l'en-tête (« time 30 · english »).
    pub summary: String,
    /// Timer ou progression (style mini), affiché pendant le test.
    pub timer: Option<String>,
}

/// Couleurs (juste, non tapé, faux, en trop) selon flipTestColors et colorfulMode.
fn letter_colors(p: &Palette, flip: bool, colorful: bool) -> (Color, Color, Color, Color) {
    let (correct, untyped) = match (flip, colorful) {
        (false, false) => (p.text, p.sub),
        (true, false) => (p.sub, p.text),
        (false, true) => (p.main, p.sub),
        (true, true) => (p.sub, p.main),
    };
    let (incorrect, extra) = if colorful {
        (p.colorful_error, p.colorful_error_extra)
    } else {
        (p.error, p.error_extra)
    };
    (correct, untyped, incorrect, extra)
}

impl TestView<'_> {
    /// Largeur de la zone de mots.
    pub fn text_width(&self, area: Rect) -> u16 {
        let auto = area.width.saturating_sub(8).min(100);
        if self.max_line_width >= 20 {
            self.max_line_width.min(area.width.saturating_sub(2))
        } else {
            auto
        }
    }

    /// Dessine l'écran et renvoie la position du caret à l'écran.
    pub fn render(&self, buf: &mut Buffer, area: Rect) -> Option<(u16, u16)> {
        let s = self.session;
        let running = s.state() == SessionState::Running;
        let zen = s.spec().mode == Mode::Zen;
        let width = self.text_width(area);
        let left = area.x + area.width.saturating_sub(width) / 2;
        let shown = if zen { 2 } else { 3 };
        let top = area.y + area.height.saturating_sub(shown) / 2;

        if !running {
            buf.set_string(
                area.x + 2,
                area.y + 1,
                "fasttype",
                Style::default()
                    .fg(self.palette.main)
                    .add_modifier(Modifier::BOLD),
            );
            let summary_x = area
                .right()
                .saturating_sub(self.summary.chars().count() as u16 + 2);
            buf.set_string(
                summary_x,
                area.y + 1,
                &self.summary,
                Style::default().fg(self.palette.sub),
            );
            let tips = [
                (
                    "tab".to_string(),
                    Style::default()
                        .fg(self.palette.sub_alt)
                        .bg(self.palette.sub),
                ),
                (" + ".to_string(), Style::default().fg(self.palette.sub)),
                (
                    "enter".to_string(),
                    Style::default()
                        .fg(self.palette.sub_alt)
                        .bg(self.palette.sub),
                ),
                (
                    " - restart".to_string(),
                    Style::default().fg(self.palette.sub),
                ),
            ];
            centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
        }
        if running && let Some(timer) = &self.timer {
            buf.set_string(
                left,
                top.saturating_sub(1),
                timer,
                Style::default().fg(self.palette.main),
            );
        }

        let layout = layout_words(s.words(), s.inputs(), s.active_index(), width);
        let first = layout.first_visible();
        for (row, line) in layout
            .lines
            .iter()
            .enumerate()
            .skip(first)
            .take(usize::from(shown))
        {
            let y = top + (row - first) as u16;
            for b in line {
                self.draw_word(buf, left + b.x, y, b.index, zen);
            }
        }
        self.caret(&layout, left, top, first, shown)
    }

    fn caret(
        &self,
        layout: &Layout,
        left: u16,
        top: u16,
        first: usize,
        shown: u16,
    ) -> Option<(u16, u16)> {
        let s = self.session;
        if s.state() == SessionState::Finished || s.words().is_empty() {
            return None;
        }
        let (line, x) = layout.caret;
        (line >= first && line < first + usize::from(shown))
            .then(|| (left + x, top + (line - first) as u16))
    }

    fn draw_word(&self, buf: &mut Buffer, x0: u16, y: u16, index: usize, zen: bool) {
        let s = self.session;
        let (correct, untyped, incorrect, extra) =
            letter_colors(self.palette, self.flip_test_colors, self.colorful_mode);
        let target = s.word(index);
        let input = s.input(index);
        let typed: Vec<char> = letters(input).chars().collect();
        let wrong_committed = !zen && s.is_committed(index) && input != target;
        let base = |fg: Color| {
            let st = Style::default().fg(fg).bg(self.palette.bg);
            if wrong_committed {
                st.add_modifier(Modifier::UNDERLINED)
                    .underline_color(self.palette.error)
            } else {
                st
            }
        };
        let mut x = x0;
        let mut put = |c: char, style: Style| {
            buf.set_string(x, y, c.to_string(), style);
            x += char_width(c);
        };
        if zen {
            for &c in &typed {
                put(c, base(correct));
            }
            return;
        }
        for (k, tc) in letters(target).chars().enumerate() {
            let style = match typed.get(k) {
                None => base(untyped),
                Some(&ic) if ic == tc => base(correct),
                Some(_) => base(incorrect),
            };
            put(tc, style);
        }
        for c in extras(target, input).chars() {
            put(c, base(extra));
        }
    }
}
```

`crates/fasttype-tui/src/view/result.rs` :
```rust
//! Écran de résultat : vitesse et précision, puis le détail du test.

use crate::theme::Palette;
use crate::view::centered_segments;
use fasttype_core::result::{Invalid, TestResult};
use fasttype_core::spec::Mode;
use fasttype_store::RecordOutcome;
use fasttype_store::pbs::PbOutcome;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

pub struct ResultView<'a> {
    pub result: &'a TestResult,
    /// `None` si l'enregistrement a échoué.
    pub outcome: Option<RecordOutcome>,
    pub palette: &'a Palette,
    /// `typingSpeedUnit` : wpm, cpm, wps, cps ou wph.
    pub unit: &'a str,
    /// `alwaysShowDecimalPlaces`.
    pub decimals: bool,
}

/// Facteur de conversion depuis le wpm (`typing-speed-units.ts`).
pub fn unit_factor(unit: &str) -> f64 {
    match unit {
        "cpm" => 5.0,
        "wps" => 1.0 / 60.0,
        "cps" => 5.0 / 60.0,
        "wph" => 60.0,
        _ => 1.0,
    }
}

pub fn invalid_label(reason: Invalid) -> &'static str {
    match reason {
        Invalid::TooShort => "too short",
        Invalid::Afk => "afk detected",
        Invalid::Repeated => "repeated",
        Invalid::Wpm => "invalid wpm",
        Invalid::Raw => "invalid raw",
        Invalid::Accuracy => "invalid accuracy",
    }
}

fn quote_length_label(group: Option<u8>) -> &'static str {
    match group {
        Some(0) => "short",
        Some(1) => "medium",
        Some(2) => "long",
        Some(3) => "thicc",
        _ => "",
    }
}

/// « time 30 english punctuation numbers ».
pub fn test_type(r: &TestResult) -> String {
    let mode = match r.mode {
        Mode::Time => format!("time {}", r.mode2),
        Mode::Words => format!("words {}", r.mode2),
        Mode::Quote => format!("quote {}", quote_length_label(r.quote_length)),
        Mode::Zen => "zen".to_string(),
        Mode::Custom => "custom".to_string(),
    };
    let mut out = format!("{} {}", mode.trim_end(), r.language);
    if r.punctuation {
        out.push_str(" punctuation");
    }
    if r.numbers {
        out.push_str(" numbers");
    }
    out
}

impl ResultView<'_> {
    fn number(&self, v: f64) -> String {
        if self.decimals {
            format!("{v:.2}")
        } else {
            format!("{}", v.round())
        }
    }

    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        let p = self.palette;
        let r = self.result;
        let label = Style::default().fg(p.sub);
        let value = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
        let detail = Style::default().fg(p.text);
        let factor = unit_factor(self.unit);
        let acc = if self.decimals {
            format!("{:.2}%", r.acc)
        } else {
            format!("{}%", r.acc.floor())
        };
        let mid = area.y + area.height / 2;
        let y0 = mid.saturating_sub(3);

        centered_segments(
            buf,
            area,
            y0,
            &[
                (format!("{} ", self.unit), label),
                (self.number(r.wpm * factor), value),
                ("     acc ".to_string(), label),
                (acc, value),
            ],
        );
        if let Some(RecordOutcome::Saved(PbOutcome::NewBest { .. })) = self.outcome {
            centered_segments(
                buf,
                area,
                y0 + 1,
                &[("new personal best".to_string(), Style::default().fg(p.main))],
            );
        }
        let [c, i, e, m] = r.char_stats;
        centered_segments(
            buf,
            area,
            y0 + 3,
            &[
                ("raw ".to_string(), label),
                (self.number(r.raw * factor), detail),
                ("   characters ".to_string(), label),
                (format!("{c}/{i}/{e}/{m}"), detail),
                ("   consistency ".to_string(), label),
                (format!("{}%", r.consistency.round()), detail),
                ("   time ".to_string(), label),
                (format!("{}s", r.test_duration.round()), detail),
            ],
        );
        centered_segments(
            buf,
            area,
            y0 + 4,
            &[("test type ".to_string(), label), (test_type(r), detail)],
        );
        let mut other = Vec::new();
        if let Some(reason) = r.invalid {
            other.push(invalid_label(reason));
        }
        if r.bailed_out {
            other.push("bailed out");
        }
        if !other.is_empty() {
            centered_segments(
                buf,
                area,
                y0 + 5,
                &[
                    ("other ".to_string(), label),
                    (other.join(", "), Style::default().fg(p.error)),
                ],
            );
        }
        let tips = [
            ("tab".to_string(), Style::default().fg(p.sub_alt).bg(p.sub)),
            (" + ".to_string(), label),
            (
                "enter".to_string(),
                Style::default().fg(p.sub_alt).bg(p.sub),
            ),
            (" - next test".to_string(), label),
        ];
        centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
    }
}
```

- [ ] **Step 5 : implémenter `app.rs`**

```rust
//! État de l'application et réaction aux touches. Aucune E/S terminal : la
//! boucle (`runner`) lui passe les entrées horodatées et lui demande de dessiner.

use crate::input::{Input, Key, Phase};
use crate::perf::Perf;
use crate::session_factory::SessionFactory;
use crate::theme::{ColorMode, Palette};
use crate::view::notify::{Level, Notifications};
use crate::view::result::{ResultView, invalid_label};
use crate::view::test::TestView;
use crate::view::{MIN_HEIGHT, MIN_WIDTH, fill_background, too_small};
use fasttype_core::result::TestResult;
use fasttype_core::session::{InputOutcome, SessionState, TestSession};
use fasttype_core::spec::Mode;
use fasttype_data::themes::{Rgba, Theme};
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_store::{Config, RecordOutcome, Store};
use ratatui::Frame;
use ratatui::style::Style;

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

/// Forme, clignotement et couleur du curseur du terminal, qui sert de caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaretLook {
    pub shape: CaretShape,
    pub blinking: bool,
    pub rgb: (u8, u8, u8),
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
/// nommé, sinon `serika_dark` avec un avertissement.
fn resolve_theme(config: &Config) -> (Theme, Option<String>) {
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
            return (t, None);
        }
    }
    let name = config.str("theme");
    match theme(name) {
        Some(t) => (t.clone(), None),
        None => (
            theme(DEFAULT_THEME).cloned().unwrap_or_else(fallback_theme),
            Some(format!("theme {name} not found — using {DEFAULT_THEME}")),
        ),
    }
}

impl App {
    pub fn new(store: Store, color_mode: ColorMode, now: f64, seed: u64) -> App {
        let mut notifications = Notifications::default();
        for w in &store.warnings {
            notifications.push(w.clone(), Level::Error, now);
        }
        let (theme, theme_warning) = resolve_theme(&store.config);
        if let Some(w) = theme_warning {
            notifications.push(w, Level::Error, now);
        }
        let mut factory = SessionFactory::new();
        let built = factory.build(&store.config, seed);
        if let Some(w) = built.warning {
            notifications.push(w, Level::Error, now);
        }
        App {
            palette: Palette::from_theme(&theme, color_mode),
            store,
            factory,
            session: built.session,
            screen: Screen::Test,
            notifications,
            restart_armed: None,
            seed,
            quit: false,
        }
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

    fn restart(&mut self, now: f64) {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let built = self.factory.build(&self.store.config, self.seed);
        if let Some(w) = built.warning {
            self.notifications.push(w, Level::Error, now);
        }
        self.session = built.session;
        self.screen = Screen::Test;
        self.restart_armed = None;
    }

    /// Le restart rapide est refusé pendant un test long, sauf avec Shift (`quick-restart.ts`).
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
    }

    fn check_finished(&mut self, now: f64) {
        if matches!(self.screen, Screen::Test) && self.session.state() == SessionState::Finished {
            self.finish(now);
        }
    }

    pub fn handle(&mut self, input: Input) {
        let Input::Key {
            key,
            phase,
            code,
            at,
        } = input
        else {
            return;
        };
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
        let has_newlines = self.session.words().iter().any(|w| w.contains('\n'));
        let quick = self.store.config.str("quickRestart");
        let is_quick = match (quick, key) {
            ("tab", Key::Tab | Key::BackTab) | ("esc", Key::Esc) => true,
            ("enter", Key::Enter) => !has_newlines,
            ("enter", Key::ShiftEnter) => true,
            _ => false,
        };
        if is_quick {
            self.try_restart(at, matches!(key, Key::BackTab | Key::ShiftEnter));
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
    }

    pub fn tick(&mut self, now: f64) {
        if matches!(self.screen, Screen::Test) {
            self.session.tick(now);
            self.check_finished(now);
        }
        self.notifications.expire(now);
    }

    /// Prochain instant où l'écran doit changer sans frappe (tick du timer,
    /// fin d'une notification). `None` : attendre la prochaine touche.
    pub fn next_deadline(&self) -> Option<f64> {
        let tick = matches!(self.screen, Screen::Test)
            .then(|| self.session.next_tick_at())
            .flatten();
        [tick, self.notifications.next_expiry()]
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

    /// Timer « mini » : temps restant en time, progression ailleurs.
    pub fn timer(&self) -> String {
        let s = &self.session;
        let seconds = s.live_stats().seconds;
        match (s.spec().mode, s.spec().time_limit) {
            (Mode::Time, Some(limit)) if limit > 0 => limit.saturating_sub(seconds).to_string(),
            (Mode::Time, _) => seconds.to_string(),
            (Mode::Zen, _) => s.active_index().to_string(),
            (Mode::Words, _) if s.spec().mode2 == "0" => s.active_index().to_string(),
            (Mode::Words, _) => format!("{}/{}", s.active_index(), s.spec().mode2),
            _ => format!("{}/{}", s.active_index(), s.words().len()),
        }
    }

    pub fn caret_look(&self) -> Option<CaretLook> {
        if !matches!(self.screen, Screen::Test) {
            return None;
        }
        let shape = match self.store.config.str("caretStyle") {
            "off" => return None,
            "block" | "outline" => CaretShape::Block,
            "underline" => CaretShape::Underline,
            _ => CaretShape::Bar,
        };
        Some(CaretLook {
            shape,
            blinking: self.session.state() == SessionState::Ready,
            rgb: self.palette.caret_rgb,
        })
    }

    pub fn draw(&self, frame: &mut Frame, perf: Option<&Perf>) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        fill_background(buf, area, &self.palette);
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            too_small(buf, area, &self.palette);
            return;
        }
        let c = &self.store.config;
        let caret = match &self.screen {
            Screen::Test => TestView {
                session: &self.session,
                palette: &self.palette,
                flip_test_colors: c.bool("flipTestColors"),
                colorful_mode: c.bool("colorfulMode"),
                max_line_width: c.int("maxLineWidth").clamp(0, i64::from(u16::MAX)) as u16,
                summary: self.summary(),
                timer: (c.str("timerStyle") != "off").then(|| self.timer()),
            }
            .render(buf, area),
            Screen::Result(info) => {
                ResultView {
                    result: &info.result,
                    outcome: info.outcome,
                    palette: &self.palette,
                    unit: c.str("typingSpeedUnit"),
                    decimals: c.bool("alwaysShowDecimalPlaces"),
                }
                .render(buf, area);
                None
            }
        };
        self.notifications.render(buf, area, &self.palette);
        if let Some(p) = perf {
            buf.set_string(
                area.x + 1,
                area.bottom() - 1,
                p.summary(),
                Style::default().fg(self.palette.sub),
            );
        }
        if let Some(pos) = caret
            && self.caret_look().is_some()
        {
            frame.set_cursor_position(pos);
        }
    }
}
```

- [ ] **Step 6 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous les tests de `fasttype-tui` PASS (17 pour app, 7 pour view, plus ceux des tâches 1 à 5).

- [ ] **Step 7 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): application, écran de test et écran de résultat"
```

---

### Task 7 : terminal, boucle principale, binaire `fasttype`

**Files:**
- Create: `crates/fasttype-tui/src/terminal.rs`, `crates/fasttype-tui/src/runner.rs`, `crates/fasttype-tui/src/main.rs`, `crates/fasttype-tui/benches/frame.rs`, `scripts/pty_smoke.py`
- Modify: `crates/fasttype-tui/Cargo.toml` (binaire et benchmark), `crates/fasttype-tui/src/lib.rs` (version finale)
- Test: `crates/fasttype-tui/tests/frame_writer.rs`

**Interfaces:**
- Produces :
  - `FrameWriter` (clonable, tampon partagé), avec `new()`, `pending()` et `present(&mut impl Write)` ;
  - `TerminalGuard::enter()`, `restore()` et `queue_caret_look(&mut impl Write, CaretLook)` ;
  - `runner::{Options { perf }, run(Options) -> io::Result<Perf>}` ;
  - le binaire `fasttype`, avec les options `--perf` (résumé affiché aussi à la sortie), `--rebuild-pbs`, `--version` et `--help`.
- Ratatui 0.30 ne donne accès au writer du backend que derrière une fonction instable (`writer_mut`). On ne l'active pas : le tampon d'image est partagé entre le backend et la boucle par un `Rc<RefCell<Vec<u8>>>`, et les séquences passent par le backend, qui implémente `Write`.

- [ ] **Step 1 : écrire le test qui échoue**

`crates/fasttype-tui/tests/frame_writer.rs` :
```rust
use fasttype_tui::terminal::FrameWriter;
use std::io::Write;

#[test]
fn clones_share_one_buffer_and_present_once() {
    let frame = FrameWriter::new();
    let mut backend_side = frame.clone();
    backend_side.write_all(b"abc").unwrap();
    backend_side.flush().unwrap();
    backend_side.write_all(b"def").unwrap();
    assert_eq!(frame.pending(), b"abcdef");
    let mut out = Vec::new();
    frame.present(&mut out).unwrap();
    assert_eq!(out, b"abcdef");
    assert!(frame.pending().is_empty());
}
```

Run: `cargo test -p fasttype-tui --test frame_writer`
Expected: échec de compilation (`unresolved import fasttype_tui::terminal`).

- [ ] **Step 2 : implémenter `terminal.rs` et `runner.rs`**

`crates/fasttype-tui/src/terminal.rs` :
```rust
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
```

`crates/fasttype-tui/src/runner.rs` :
```rust
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
```

`crates/fasttype-tui/src/lib.rs` (version finale) :
```rust
//! Interface terminal de fasttype : écrans, entrées, boucle de rendu.

pub mod app;
pub mod input;
pub mod layout;
pub mod perf;
pub mod runner;
pub mod session_factory;
pub mod terminal;
pub mod theme;
pub mod view;
```

Run: `cargo fmt && cargo test -p fasttype-tui --test frame_writer`
Expected: 1 test PASS.

- [ ] **Step 3 : binaire `fasttype`**

Ajouter à `crates/fasttype-tui/Cargo.toml` :
```toml
[[bin]]
name = "fasttype"
path = "src/main.rs"

[[bench]]
name = "frame"
harness = false
```

`crates/fasttype-tui/src/main.rs` :
```rust
//! `fasttype` : clone de Monkeytype pour le terminal.

use fasttype_store::Store;
use fasttype_store::paths::Paths;
use fasttype_tui::runner::{Options, run};
use std::process::ExitCode;

const USAGE: &str = "usage: fasttype [--perf | --rebuild-pbs | --version | --help]

  --perf          show input latency and frame time
  --rebuild-pbs   recompute personal bests from the result history";

fn rebuild_pbs() -> ExitCode {
    let Some(paths) = Paths::from_system() else {
        eprintln!("HOME is not set");
        return ExitCode::FAILURE;
    };
    let mut store = Store::open(paths);
    match store.rebuild_pbs() {
        Ok(h) => {
            println!(
                "{} results read, {} unreadable lines skipped",
                h.results.len(),
                h.skipped_lines
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let perf = match args.first().map(String::as_str) {
        None => false,
        Some("--perf") => true,
        Some("--rebuild-pbs") => return rebuild_pbs(),
        Some("--version" | "-V") => {
            println!("fasttype {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help" | "-h") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("unknown option: {other}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match run(Options { perf }) {
        Ok(stats) => {
            if perf {
                println!("{}", stats.summary());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
```

Run: `cargo build --release -p fasttype-tui && ./target/release/fasttype --version && ./target/release/fasttype --help`
Expected: `fasttype 0.1.0`, puis le texte d'aide.

- [ ] **Step 4 : test de bout en bout dans un pseudo-terminal**

`scripts/pty_smoke.py` :
```python
"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf]"""
import fcntl, os, pty, re, select, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
perf = "--perf" in sys.argv[3:]
os.makedirs(f"{home}/.config/fasttype", exist_ok=True)
with open(f"{home}/.config/fasttype/config.toml", "w") as f:
    f.write('mode = "custom"\n')

pid, fd = pty.fork()
if pid == 0:
    os.environ.update({"HOME": home, "TERM": "xterm-256color", "COLORTERM": "truecolor"})
    os.environ.pop("XDG_CONFIG_HOME", None)
    os.environ.pop("XDG_DATA_HOME", None)
    os.execv(binary, [binary] + (["--perf"] if perf else []))

fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 0, 0))
out = bytearray()

def pump(seconds):
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
        # réponses d'un terminal sans protocole clavier Kitty
        if b"\x1b[6n" in data:
            os.write(fd, b"\x1b[1;1R")
        if b"\x1b[?u" in data or b"\x1b[c" in data:
            os.write(fd, b"\x1b[?62c")

def plain(b):
    t = b.decode("utf-8", "replace")
    t = re.sub(r"\x1b\[[0-9;?<>]*[ A-Za-z]|\x1b\][^\x07]*\x07", " ", t)
    return re.sub(r"\s+", " ", t)

pump(1.0)
for ch in "The quick brown fox jumps over the lazy dog":
    os.write(fd, ch.encode())
    pump(0.08)
pump(0.8)
screen = plain(bytes(out))
os.write(fd, b"\x03")  # Ctrl+C
pump(1.0)
_, status = os.waitpid(pid, 0)
print("exit", os.waitstatus_to_exitcode(status))
print("result screen:", "test type custom english" in screen)
print("alt screen left:", b"\x1b[?1049l" in out)
print("sync output used:", b"\x1b[?2026h" in out and b"\x1b[?2026l" in out)
print("cursor color reset:", b"\x1b]112\x07" in out)
if perf:
    found = re.findall(r"key→flush p50 ([0-9.]+) p99 ([0-9.]+) ms · frame p99 ([0-9.]+) ms · n (\d+)", bytes(out).decode("utf-8", "replace"))
    print("perf (p50, p99, frame p99, n):", found[-1] if found else None)
```

Run: `rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --perf`
Expected :
```
exit 0
result screen: True
alt screen left: True
sync output used: True
cursor color reset: True
perf (p50, p99, frame p99, n): ('…', '…', '…', '44')
```
Le p50 doit être sous 0,5 ms. Le p99 dépend du système et du pty piloté par Python (1 à 3 ms mesurés) : le noter dans le ledger, sans en faire un critère d'échec. Le coût propre de fasttype se mesure à l'étape 5.

- [ ] **Step 5 : benchmark**

`crates/fasttype-tui/benches/frame.rs` :
```rust
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
            let key = Key::Char(next_char(&a));
            a.handle(Input::Key {
                key,
                phase: Phase::Press,
                code: 1,
                at: t,
            });
            a.tick(t);
            t += 15.0;
            term.draw(|f| a.draw(f, None)).unwrap();
            black_box(a.session().active_index())
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
```

Run: `cargo bench -p fasttype-tui --bench frame 2>&1 | grep "time:"`
Expected : `frame_200x60` sous **1 ms** et `key_and_frame_200x60` sous **2 ms** (spec §7 et §9 ; mesuré à 0,17 et 0,39 ms). Recopier les médianes dans le message de commit. Si un seuil est dépassé, chercher la cause (profilage) sans remonter le seuil.

- [ ] **Step 6 : vérification finale et essai à la main**

Run: `cargo fmt && cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, aucun avertissement.

Puis lancer `cargo run --release -p fasttype-tui` dans un vrai terminal et faire un test time 30. Les tests automatiques ne peuvent pas juger l'aspect à l'œil : le noter dans le ledger, et signaler à l'utilisateur que l'essai visuel lui revient.

- [ ] **Step 7 : commit**

```bash
git add crates/fasttype-tui scripts
git commit -m "feat(tui): boucle de rendu, terminal restauré et binaire fasttype

frame_200x60 : <médiane> ; key_and_frame_200x60 : <médiane>"
```
(remplacer `<médiane>` par les valeurs mesurées)
