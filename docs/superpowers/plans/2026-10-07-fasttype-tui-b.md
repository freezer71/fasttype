# fasttype-tui, partie B : la fluidité visible — plan d'implémentation (plan 4b)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner à `fasttype` les animations de Monkeytype : caret qui glisse (au pixel près dans Kitty et Ghostty), focus mode, fondus entre le test et le résultat, stats en direct, graphique du résultat en braille. Le coût d'une frappe reste constant, quelle que soit la longueur du test.

**Architecture:**
- **Moteur d'animation pur** (`anim.rs`). Il fournit les courbes d'anime.js v4 et du CSS, des interpolations reciblables et une cadence d'images absolue. `App` garde l'instant `now` de la dernière touche ou du dernier `tick` et calcule chaque animation à cet instant. Tout se teste à un instant fixé, sans horloge réelle.
- **Le caret** (`caret.rs`) est une position en cases fractionnaires qui glisse avec `inOut(1.25)`. Trois rendus :
  1. **Image Kitty** au pixel près (`kitty.rs`), pour Kitty et Ghostty.
  2. **Teinte des cases** pour le style bloc : le fond des deux lettres couvertes est teinté au prorata, ce qui donne un rendu à la demi-case et plus fin, dans tous les terminaux.
  3. **Curseur du terminal** case par case, en repli.
- **Boucle.** Quand une animation tourne, `next_deadline` renvoie la prochaine image sur un planning absolu (60, 120 ou 144 images/s). Au repos, la boucle attend sans délai : 0 % de CPU.
- **Fenêtre de mise en page.** Elle part du premier mot de la ligne du haut et s'arrête deux lignes après le caret. Les stats en direct sont calculées une fois par seconde. Une frappe coûte ainsi le même temps à la 10 000e qu'à la première.

**Tech Stack:** Rust 1.97 (édition 2024), `ratatui` 0.30.2, `crossterm` 0.29.0, `signal-hook` 0.3.18 (déjà présent via crossterm), `criterion` 0.8.2, Python 3 pour le test pty.

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§5.1 à §5.3, §7, §8, §9). Le plan 4a est livré sur `main`.

**Code vérifié avant rédaction :** tout le code de ce plan a été assemblé et exécuté dans une copie de travail jetable.
- **Tests :** 325 tests du workspace au vert, et clippy ne signale rien.
- **Benchmark :**
  - `frame_200x60` : 78 µs ;
  - `key_and_frame_200x60` : 78 µs, contre 482 µs en 4a, où le timer recalculait les stats live à chaque image ;
  - `key_and_frame_after_10k_keys` : 77 µs.
- **Test pty :** latence touche → écran p50 0,10 ms, p99 0,18 ms. Le caret Kitty est placé puis supprimé à la sortie, et SIGTERM restaure le terminal.

**Comportements de référence (lus dans le code de Monkeytype, commit 574d819) :**
- **Caret** (`elements/caret.ts`) :
  - glissement : smoothCaret off/slow/medium/fast = 0/150/100/85 ms, courbe `inOut(1.25)` ;
  - clignotement : `caretFlashSmooth`, 1 s, opacité 0 → 1 → 0 avec la courbe CSS `ease` ; `caretFlashHard` si smoothCaret est off ;
  - le caret ne clignote plus dès la première frappe ;
  - styles : barre de 0,1 em ; bloc, contour ou souligné à la largeur de la lettre.
- **Changement de ligne** (`test-ui.ts` `lineJump`) :
  - le premier saut ne fait pas défiler : le caret descend en glissant ;
  - ensuite, la ligne du haut est retirée et le caret ne glisse qu'horizontalement ;
  - `smoothLineScroll` : 125 ms `out(2)`, désactivé par défaut.
- **Focus mode** (`focus.ts`) : la barre de config et les raccourcis passent à l'opacité 0 en 125 ms (courbe Tailwind) ; le logo passe en couleur sub en 250 ms.
- **Fondus** (`test-logic.ts`) :
  - fin du test : le test disparaît en 125 ms, puis le résultat apparaît en 125 ms, courbe `out(2)` ;
  - restart : fondu de sortie de 125 ms, pendant lequel les touches sont ignorées (`input.ts:103`), puis fondu d'entrée.
- **Stats en direct** (`live-stats.ts`) :
  - timer : secondes restantes (« 01:15 » au-delà d'une minute) ou « mots/total » ;
  - vitesse au tick d'1 s, précision à chaque entrée, burst à chaque mot ;
  - styles mini, text et bar ; couleur et opacité selon `timerColor` et `timerOpacity` ;
  - apparition en 125 ms.
- **Graphique** (`ResultChart.tsx`) :
  - courbes : wpm en couleur main ; raw en main à 60 %, en pointillés ; burst en sub, lissé, rempli en subAlt à 50 % ;
  - erreurs : croix sur l'axe de droite ;
  - axe de gauche arrondi à la dizaine, et commencé à 0 si `startGraphsAtZero`.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Textes de l'interface en anglais, identiques à Monkeytype. Commentaires en français.
- `App` et les vues ne font aucune E/S terminal. Les séquences Kitty sont produites par des fonctions pures (`kitty.rs`) et écrites par `runner.rs`.
- **Toute animation est une fonction de `now`.** Aucune lecture d'horloge dans `App`, et aucune animation qui avance « d'une image » : sauter des images ne change pas le résultat.
- **Au repos, aucune image.** `next_deadline` ne renvoie une échéance d'image que si une animation tourne. C'est le cas du clignotement du caret Kitty ou bloc avant la première frappe : le site le fait aussi, en CSS.
- **Coût par frappe constant.** Aucun parcours de tous les mots dans le chemin frappe → image. Le benchmark `key_and_frame_after_10k_keys` doit rester sous 2 ms.
- **Kitty :** toutes les commandes graphiques portent `q=2`, pour que le terminal ne réponde rien sur l'entrée. `restore` supprime les images (`a=d,d=A`).
- Les touches tapées pendant le fondu de sortie d'un restart sont ignorées, comme sur le site.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace (somme des lignes `test result`), tous au vert.

**Choix assumés (écarts au site ou à la spec) :**
- **Rendu demi-case de la barre.** Une barre dessinée dans une case remplacerait la lettre. Hors Kitty, la barre reste donc le curseur du terminal, qui glisse case par case. Le rendu demi-case de la spec est appliqué au caret bloc, qui teinte le fond des lettres sans les masquer.
- **`smoothLineScroll`.** Une ligne de texte ne peut pas monter d'une fraction de case. Le défilement est donc immédiat, et la nouvelle ligne du bas apparaît en fondu de 125 ms.
- **Détection de Kitty** par les variables d'environnement (`TERM`, `TERM_PROGRAM`, `KITTY_WINDOW_ID`, hors tmux et screen), et non par une requête au terminal : la réponse arriverait sur l'entrée lue par le thread clavier. `FASTTYPE_CARET=cell|kitty` force le choix.
- **Reportés :**
  - la sortie du focus mode au mouvement de la souris (il faudrait capturer la souris) ;
  - l'avertissement Caps Lock ;
  - la ligne de PB sur le graphique ;
  - les notifications qui chevauchent l'en-tête ;
  - le badge de langue au-dessus des mots et la source de la citation sur le résultat, qui iront au plan 4c avec la barre de config.

## Review Focus

1. **Redimensionnement ou zoom pendant un glissement.** Le caret doit être replacé sans animation à la bonne position, et les images Kitty refaites à la nouvelle taille de case. Test : `caret_jumps_without_animation_after_a_restart_and_a_resize` (tâche 9). Pour le zoom, vérifier par lecture la détection du changement de taille dans `Output::present` (tâche 10).
2. **Frappe pendant le fondu d'un restart.** Elle doit être ignorée, et jamais tapée dans un test encore invisible. Test : `restart_fades_out_then_in_and_ignores_keys_meanwhile` (tâche 9).
3. **Retour arrière jusqu'à une ligne déjà retirée.** La fenêtre de mise en page doit repartir du début, sans panique, avec le caret à l'écran. Test : `backspacing_into_a_removed_line_shows_it_again` (tâche 9).
4. **Test très long (time infini).** Le coût par frappe ne doit pas croître. Test : `window_layout_starts_at_a_word_and_stops_after_the_caret` (tâche 4) et benchmark `key_and_frame_after_10k_keys` (tâche 10).
5. **Terminal 256 couleurs.** Les fondus doivent être calculés en RGB puis convertis, avec un cache, sans couleurs aberrantes. Test : `faded_palette_blends_toward_background` (tâche 3).

---

## Structure des fichiers

```
Cargo.toml                                   + signal-hook
crates/fasttype-core/src/stats.rs            + raw_history (getRawHistory)
crates/fasttype-core/src/result.rs           ChartData.raw (#[serde(default)])
crates/fasttype-core/src/session.rs          + has_newlines, elapsed_seconds, live_accuracy
crates/fasttype-tui/
├── src/anim.rs                              Easing, Tween, FrameClock
├── src/theme.rs                             RgbColors, Palette::{faded, over_bg}, mix, cache 256
├── src/layout.rs                            + layout_window ; saut de ligne zen
├── src/caret.rs                             CaretStyle, Caret (glissement, clignotement), coverage
├── src/kitty.rs                             CellPx, CaretRenderer, KittyCaret, séquences graphiques
├── src/input.rs                             + Input::Interrupt, spawn_signal_watcher
├── src/view/big.rs                          grands chiffres sur 3 lignes
├── src/view/chart.rs                        graphique braille du résultat
├── src/view/live.rs                         stats en direct (mini, text, bar)
├── src/view/test.rs                         WordsView, Chrome, words_box
├── src/view/result.rs                       grands chiffres, couronne, graphique
├── src/app.rs                               animations, transitions, fenêtre de mots
├── src/terminal.rs                          garde créé dès le mode raw ; images supprimées
├── src/runner.rs                            apply_burst, Output (Kitty), parse_options
├── src/main.rs                              --fps
├── tests/{anim,caret,kitty,chart,motion,options}.rs   nouveaux
├── tests/{app,view,layout,theme}.rs, tests/common/mod.rs   modifiés
└── benches/frame.rs                         + key_and_frame_after_10k_keys
scripts/pty_smoke.py                         + --kitty, --sigterm
```

---

### Task 1 : courbe raw et accesseurs du moteur

**Files:**
- Modify: `crates/fasttype-core/src/stats.rs`, `crates/fasttype-core/src/result.rs`, `crates/fasttype-core/src/session.rs`
- Modify: `crates/fasttype-store/tests/common/mod.rs`
- Test: `crates/fasttype-core/tests/stats_history.rs`, `crates/fasttype-store/tests/results.rs`

**Interfaces:**
- Produces :
  - `stats::raw_history(&EventLog) -> Vec<f64>` ;
  - `ChartData { wpm, raw, burst, err }`, où `raw` vaut `#[serde(default)]` : les anciens résultats se relisent ;
  - `TestSession::has_newlines() -> bool`, `elapsed_seconds() -> u32` et `live_accuracy() -> f64` (sans parcours des mots).

- [ ] **Step 1 : écrire les tests qui échouent**

Ajouter à la fin de `crates/fasttype-core/tests/stats_history.rs` :
```rust
#[test]
fn raw_history_counts_every_typed_char_cumulatively() {
    let log = LogBuilder::new(Mode::Time, true, &["ab ", "cd ", "ef "])
        .at(100.0)
        .typ(0, "xb ")
        .tick(1)
        .at(1100.0)
        .typ(1, "cd")
        .tick(2)
        .end(2000.0);
    // 1 s : "xb " → 3 car. tapés → 36 ; 2 s : 3 + "cd" (crédit partiel) = 5 → 30
    assert_eq!(stats::raw_history(&log), vec![36.0, 30.0]);
    // le wpm ne compte que les mots justes
    assert_eq!(stats::wpm_history(&log), vec![0.0, 12.0]);
    assert_eq!(stats::raw_history(&two_second_test()), vec![36.0, 30.0]);
}
```

Ajouter à la fin de `crates/fasttype-store/tests/results.rs` :
```rust
#[test]
fn results_saved_before_the_raw_series_still_load() {
    let line = serde_json::to_string(&result("30", 50.0, 1)).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&line).unwrap();
    v["chart"].as_object_mut().unwrap().remove("raw");
    let old: fasttype_core::result::TestResult = serde_json::from_value(v).unwrap();
    assert!(old.chart.raw.is_empty());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test stats_history`
Expected: échec de compilation (`cannot find function raw_history in module stats`).

- [ ] **Step 3 : implémenter**

Dans `crates/fasttype-core/src/stats.rs`, remplacer l'en-tête de `wpm_history` :
```rust
/// `getWpmHistory` : wpm cumulé à chaque borne, avec crédit partiel du mot actif.
pub fn wpm_history(log: &EventLog) -> Vec<f64> {
    let boundaries = timer_boundaries(log);
```
par :
```rust
/// `getWpmHistory` : wpm cumulé à chaque borne, avec crédit partiel du mot actif.
pub fn wpm_history(log: &EventLog) -> Vec<f64> {
    history(log, |c| c.correct_word)
}

/// `getRawHistory` : raw cumulé à chaque borne (lettres justes, fausses et en
/// trop), avec crédit partiel du mot actif.
pub fn raw_history(log: &EventLog) -> Vec<f64> {
    history(log, |c| c.all_correct + c.extra + c.incorrect)
}

/// Série cumulée commune à `wpm_history` et `raw_history` : `count` choisit les
/// caractères comptés dans chaque mot.
fn history(log: &EventLog, count: fn(&CharCounts) -> u32) -> Vec<f64> {
    let boundaries = timer_boundaries(log);
```
puis, dans le même corps, remplacer :
```rust
            let fresh = count_chars_for(input, target, false, log.context.korean).correct_word;
            not_last_sum = not_last_sum - not_last.insert(w, fresh).unwrap_or(0) + fresh;
            as_last.insert(
                w,
                count_chars_for(input, target, true, log.context.korean).correct_word,
            );
```
par :
```rust
            let fresh = count(&count_chars_for(input, target, false, log.context.korean));
            not_last_sum = not_last_sum - not_last.insert(w, fresh).unwrap_or(0) + fresh;
            as_last.insert(
                w,
                count(&count_chars_for(input, target, true, log.context.korean)),
            );
```
(`CharCounts` est déjà importé : `use crate::chars::{CharCounts, count_chars_for, count_words};`.)

Dans `crates/fasttype-core/src/result.rs` :
- ajouter le champ `raw` après `wpm` dans `ChartData` :
```rust
    /// raw cumulé à chaque seconde (absent des résultats enregistrés avant la v0.2).
    #[serde(default)]
    pub raw: Vec<f64>,
```
- dans `build_result`, ajouter `raw: stats::raw_history(log),` après `wpm: stats::wpm_history(log),`.

Dans `crates/fasttype-store/tests/common/mod.rs`, ajouter `raw: vec![wpm; 3],` après `wpm: vec![wpm; 3],`.

Dans `crates/fasttype-core/src/session.rs` :
- remplacer le début de `live_stats` :
```rust
    /// Stats live du dernier tick (`timerStep`) : wpm et raw arrondis avec
    /// crédit partiel du mot actif, précision tronquée (100 sans frappe).
    pub fn live_stats(&self) -> LiveStats {
        let seconds = self.next_tick.saturating_sub(1);
        let acc = match self.correct_inputs + self.incorrect_inputs {
            0 => 100.0,
            total => (f64::from(self.correct_inputs) / f64::from(total) * 100.0).floor(),
        };
```
par :
```rust
    /// Secondes entières écoulées au dernier tick (sans calcul : pour le timer).
    pub fn elapsed_seconds(&self) -> u32 {
        self.next_tick.saturating_sub(1)
    }

    /// Précision en direct, tronquée (100 sans frappe) : mise à jour à chaque
    /// entrée sur le site, sans attendre le tick.
    pub fn live_accuracy(&self) -> f64 {
        match self.correct_inputs + self.incorrect_inputs {
            0 => 100.0,
            total => (f64::from(self.correct_inputs) / f64::from(total) * 100.0).floor(),
        }
    }

    /// Stats live du dernier tick (`timerStep`) : wpm et raw arrondis avec
    /// crédit partiel du mot actif, précision tronquée (100 sans frappe).
    /// Coûte un parcours des mots : à appeler une fois par tick, pas par image.
    pub fn live_stats(&self) -> LiveStats {
        let seconds = self.elapsed_seconds();
        let acc = self.live_accuracy();
```
- ajouter, juste avant `pub fn is_repeated(&self) -> bool {` :
```rust
    /// Le texte contient des sauts de ligne (Entrée les tape au lieu de relancer).
    pub fn has_newlines(&self) -> bool {
        self.has_newlines
    }

```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-core --test stats_history && cargo test -p fasttype-store --test results && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (dont `raw_history_counts_every_typed_char_cumulatively` et `results_saved_before_the_raw_series_still_load`).

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core crates/fasttype-store
git commit -m "feat(core): courbe raw du graphique et accesseurs sans calcul pour le direct"
```

---

### Task 2 : moteur d'animation

**Files:**
- Create: `crates/fasttype-tui/src/anim.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod anim;`)
- Test: `crates/fasttype-tui/tests/anim.rs`

**Interfaces:**
- Produces :
  - `Easing { Linear, Out(p), InOut(p), CubicBezier(x1, y1, x2, y2) }` avec `apply(t)` ;
  - les constantes `OUT2`, `CARET_EASE`, `CSS_EASE` et `TAILWIND_EASE` ;
  - `Tween { from, to, start, duration, easing }`, avec `fixed`, `value(now)`, `is_running(now)`, `retarget(to, now, duration, easing)` et `jump(to)` ;
  - `FrameClock::{new(fps, origin), next_after(now)}`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/anim.rs` :
```rust
use fasttype_tui::anim::{Easing, FrameClock, Tween};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn easings_match_anime_js() {
    assert!(close(Easing::Linear.apply(0.3), 0.3));
    // out(2) : 1 - (1 - t)²
    assert!(close(Easing::Out(2.0).apply(0.5), 0.75));
    // inOut(1.25) : symétrique autour de (0.5, 0.5)
    let e = Easing::InOut(1.25);
    assert!(close(e.apply(0.5), 0.5));
    assert!(close(e.apply(0.25), 0.5f64.powf(1.25) / 2.0));
    assert!(close(e.apply(0.25) + e.apply(0.75), 1.0));
    for e in [Easing::Linear, Easing::Out(2.0), Easing::InOut(1.25)] {
        assert!(close(e.apply(0.0), 0.0) && close(e.apply(1.0), 1.0));
        assert!(close(e.apply(-1.0), 0.0) && close(e.apply(2.0), 1.0));
    }
}

#[test]
fn tween_at_fixed_times() {
    let t = Tween {
        from: 0.0,
        to: 10.0,
        start: 100.0,
        duration: 100.0,
        easing: Easing::Linear,
    };
    assert_eq!(t.value(50.0), 0.0);
    assert_eq!(t.value(100.0), 0.0);
    assert_eq!(t.value(150.0), 5.0);
    assert_eq!(t.value(200.0), 10.0);
    assert_eq!(t.value(1e9), 10.0);
    assert!(t.is_running(150.0));
    assert!(!t.is_running(200.0));
}

#[test]
fn retarget_starts_from_the_shown_value() {
    let mut t = Tween::fixed(0.0);
    t.retarget(10.0, 0.0, 100.0, Easing::Linear);
    assert_eq!(t.value(50.0), 5.0);
    t.retarget(20.0, 50.0, 100.0, Easing::Linear);
    assert_eq!(t.value(50.0), 5.0, "aucun saut au reciblage");
    assert_eq!(t.value(100.0), 12.5);
    assert_eq!(t.value(150.0), 20.0);
    // même cible : l'animation continue sans repartir
    t.retarget(20.0, 120.0, 100.0, Easing::Linear);
    assert_eq!(t.start, 50.0);
}

#[test]
fn zero_duration_is_a_jump() {
    let mut t = Tween::fixed(3.0);
    t.retarget(7.0, 10.0, 0.0, Easing::InOut(1.25));
    assert_eq!(t.value(10.0), 7.0);
    assert!(!t.is_running(10.0));
}

#[test]
fn frame_clock_is_absolute() {
    let c = FrameClock::new(60, 0.0);
    assert!(close(c.next_after(0.0), 1000.0 / 60.0));
    assert!(close(c.next_after(16.0), 1000.0 / 60.0));
    // une image en retard ne décale pas les suivantes
    assert!(close(c.next_after(40.0), 3.0 * 1000.0 / 60.0));
    let c = FrameClock::new(120, 5.0);
    assert!(close(c.next_after(5.0), 5.0 + 1000.0 / 120.0));
}

#[test]
fn css_cubic_bezier_curves() {
    use fasttype_tui::anim::{CSS_EASE, TAILWIND_EASE};
    for e in [CSS_EASE, TAILWIND_EASE] {
        assert!(e.apply(0.0).abs() < 1e-6 && (e.apply(1.0) - 1.0).abs() < 1e-6);
    }
    // valeurs de référence (navigateur) : ease(0.5) ≈ 0.8024, Tailwind(0.5) ≈ 0.7756
    assert!((CSS_EASE.apply(0.5) - 0.8024).abs() < 1e-3);
    assert!((TAILWIND_EASE.apply(0.5) - 0.7756).abs() < 1e-3);
    // cubic-bezier(0, 0, 1, 1) est linéaire
    assert!((Easing::CubicBezier(0.0, 0.0, 1.0, 1.0).apply(0.3) - 0.3).abs() < 1e-6);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test anim`
Expected: échec de compilation (`unresolved import fasttype_tui::anim`).

- [ ] **Step 3 : implémenter `anim.rs`**

Ajouter `pub mod anim;` en tête des modules de `crates/fasttype-tui/src/lib.rs`, puis créer `crates/fasttype-tui/src/anim.rs` :
```rust
//! Moteur d'animation : courbes d'anime.js v4 et interpolations reciblables.
//! Tout se calcule à un instant `now` donné (ms) : testable sans horloge réelle.

/// Courbes d'anime.js v4 (`eases.out(p)`, `eases.inOut(p)`) et courbes CSS
/// (`cubic-bezier`), pour les transitions que le site confie au CSS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    /// `out(p)` : `1 - (1 - t)^p`.
    Out(f64),
    /// `inOut(p)` : `(2t)^p / 2` puis `1 - (2 - 2t)^p / 2`.
    InOut(f64),
    /// `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier(f64, f64, f64, f64),
}

/// Courbe par défaut d'anime.js v4 (fondus, défilement des lignes).
pub const OUT2: Easing = Easing::Out(2.0);
/// Courbe du caret (`elements/caret.ts`).
pub const CARET_EASE: Easing = Easing::InOut(1.25);
/// `ease` du CSS : clignotement du caret.
pub const CSS_EASE: Easing = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0);
/// Courbe des transitions Tailwind : focus mode.
pub const TAILWIND_EASE: Easing = Easing::CubicBezier(0.4, 0.0, 0.2, 1.0);

/// Point d'une courbe de Bézier cubique de (0, 0) à (1, 1), coordonnée par coordonnée.
fn bezier(a: f64, b: f64, s: f64) -> f64 {
    let u = 1.0 - s;
    3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
}

/// Valeur `y` de la courbe au point d'abscisse `x` : `s` trouvé par dichotomie
/// (`x(s)` est croissante quand `x1` et `x2` sont dans [0, 1]).
fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    if x <= 0.0 || x >= 1.0 {
        return x.clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if bezier(x1, x2, mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bezier(y1, y2, (lo + hi) / 2.0)
}

impl Easing {
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::Out(p) => 1.0 - (1.0 - t).powf(p),
            Easing::InOut(p) => {
                if t < 0.5 {
                    (2.0 * t).powf(p) / 2.0
                } else {
                    1.0 - (2.0 - 2.0 * t).powf(p) / 2.0
                }
            }
            Easing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
        }
    }
}

/// Interpolation d'une valeur de `from` vers `to` entre `start` et `start + duration`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    pub from: f64,
    pub to: f64,
    pub start: f64,
    pub duration: f64,
    pub easing: Easing,
}

impl Tween {
    /// Valeur fixe, sans animation.
    pub fn fixed(value: f64) -> Self {
        Tween {
            from: value,
            to: value,
            start: 0.0,
            duration: 0.0,
            easing: Easing::Linear,
        }
    }

    pub fn value(&self, now: f64) -> f64 {
        if self.duration <= 0.0 || now >= self.start + self.duration {
            return self.to;
        }
        if now <= self.start {
            return self.from;
        }
        let k = self.easing.apply((now - self.start) / self.duration);
        self.from + (self.to - self.from) * k
    }

    pub fn is_running(&self, now: f64) -> bool {
        self.duration > 0.0 && now < self.start + self.duration && self.from != self.to
    }

    /// Nouvelle cible : repart de la valeur affichée à `now` (jamais de saut).
    /// Sans effet si la cible ne change pas.
    pub fn retarget(&mut self, to: f64, now: f64, duration: f64, easing: Easing) {
        if to == self.to {
            return;
        }
        self.from = self.value(now);
        self.to = to;
        self.start = now;
        self.duration = duration;
        self.easing = easing;
    }

    /// Place la valeur sans animation.
    pub fn jump(&mut self, to: f64) {
        *self = Tween::fixed(to);
    }
}

/// Cadence des images d'animation : un planning absolu (`origin + k × période`),
/// sans dérive même si une image prend du retard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameClock {
    pub period: f64,
    pub origin: f64,
}

impl FrameClock {
    pub fn new(fps: u32, origin: f64) -> Self {
        FrameClock {
            period: 1000.0 / f64::from(fps.max(1)),
            origin,
        }
    }

    /// Première échéance d'image strictement après `now`.
    pub fn next_after(&self, now: f64) -> f64 {
        let k = ((now - self.origin) / self.period).floor() + 1.0;
        self.origin + k * self.period
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test anim && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 6 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): moteur d'animation (courbes anime.js et CSS, reciblage, cadence absolue)"
```

---

### Task 3 : palette en RGB et fondus

**Files:**
- Modify: `crates/fasttype-tui/src/theme.rs` (version complète ci-dessous)
- Test: `crates/fasttype-tui/tests/theme.rs`

**Interfaces:**
- Produces :
  - `Rgb = (u8, u8, u8)`, `RgbColors` (les 10 couleurs) et `mix(a, b, t) -> Rgb` ;
  - `Palette` reçoit en plus `rgb: RgbColors` et `mode: ColorMode`, avec `from_rgb`, `over_bg(rgb, opacity) -> Color` et `faded(opacity) -> Palette` ;
  - `to_color(rgb, mode)`, avec un cache pour les 256 couleurs.
- Les champs existants (`bg`, `main`…, `caret_rgb`) ne changent pas.

- [ ] **Step 1 : écrire le test qui échoue**

Ajouter à la fin de `crates/fasttype-tui/tests/theme.rs` :
```rust
#[test]
fn faded_palette_blends_toward_background() {
    let t = theme(DEFAULT_THEME).unwrap();
    let p = Palette::from_theme(t, ColorMode::TrueColor);
    assert_eq!(p.faded(1.0), p);
    let zero = p.faded(0.0);
    assert_eq!(zero.main, p.bg);
    assert_eq!(zero.text, p.bg);
    let half = p.faded(0.5);
    // main #e2b714 sur bg #323437 à 50 %
    assert_eq!(half.main, Color::Rgb(0x8a, 0x76, 0x26));
    let p256 = Palette::from_theme(t, ColorMode::Ansi256);
    assert_eq!(p256.faded(0.0).main, p256.bg);
}
```

- [ ] **Step 2 : lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p fasttype-tui --test theme`
Expected: échec de compilation (`no method named faded`).

- [ ] **Step 3 : implémenter**

`crates/fasttype-tui/src/theme.rs` (version complète) :
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

pub type Rgb = (u8, u8, u8);

/// Les 10 couleurs d'un thème en RGB opaque (les couleurs translucides sont
/// déjà posées sur le fond).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColors {
    pub bg: Rgb,
    pub main: Rgb,
    pub caret: Rgb,
    pub sub: Rgb,
    pub sub_alt: Rgb,
    pub text: Rgb,
    pub error: Rgb,
    pub error_extra: Rgb,
    pub colorful_error: Rgb,
    pub colorful_error_extra: Rgb,
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
    pub caret_rgb: Rgb,
    pub rgb: RgbColors,
    pub mode: ColorMode,
}

/// `a` vers `b` : `t = 0` donne `a`, `t = 1` donne `b`.
pub fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * t).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
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
        let c = |x: Rgba| {
            let o = x.over(bg);
            (o.r, o.g, o.b)
        };
        let rgb = RgbColors {
            bg: (bg.r, bg.g, bg.b),
            main: c(theme.main),
            caret: c(theme.caret),
            sub: c(theme.sub),
            sub_alt: c(theme.sub_alt),
            text: c(theme.text),
            error: c(theme.error),
            error_extra: c(theme.error_extra),
            colorful_error: c(theme.colorful_error),
            colorful_error_extra: c(theme.colorful_error_extra),
        };
        Self::from_rgb(rgb, mode)
    }

    pub fn from_rgb(rgb: RgbColors, mode: ColorMode) -> Self {
        let c = |x: Rgb| to_color(x, mode);
        Palette {
            bg: c(rgb.bg),
            main: c(rgb.main),
            caret: c(rgb.caret),
            sub: c(rgb.sub),
            sub_alt: c(rgb.sub_alt),
            text: c(rgb.text),
            error: c(rgb.error),
            error_extra: c(rgb.error_extra),
            colorful_error: c(rgb.colorful_error),
            colorful_error_extra: c(rgb.colorful_error_extra),
            caret_rgb: rgb.caret,
            rgb,
            mode,
        }
    }

    /// Couleur `x` vue à l'opacité `opacity` sur le fond du thème.
    pub fn over_bg(&self, x: Rgb, opacity: f64) -> Color {
        to_color(mix(self.rgb.bg, x, opacity), self.mode)
    }

    /// Palette à l'opacité `opacity` : chaque couleur est mêlée au fond, comme
    /// un élément HTML à `opacity < 1` sur le fond de la page.
    pub fn faded(&self, opacity: f64) -> Palette {
        if opacity >= 1.0 {
            return *self;
        }
        let r = self.rgb;
        let f = |x: Rgb| mix(r.bg, x, opacity);
        let faded = RgbColors {
            bg: r.bg,
            main: f(r.main),
            caret: f(r.caret),
            sub: f(r.sub),
            sub_alt: f(r.sub_alt),
            text: f(r.text),
            error: f(r.error),
            error_extra: f(r.error_extra),
            colorful_error: f(r.colorful_error),
            colorful_error_extra: f(r.colorful_error_extra),
        };
        Palette::from_rgb(faded, self.mode)
    }
}

pub fn to_color(c: Rgb, mode: ColorMode) -> Color {
    match mode {
        ColorMode::TrueColor => Color::Rgb(c.0, c.1, c.2),
        ColorMode::Ansi256 => Color::Indexed(nearest_256_cached(c)),
    }
}

thread_local! {
    static NEAREST: std::cell::RefCell<std::collections::HashMap<Rgb, u8>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// `nearest_256` avec un cache : les fondus recalculent les mêmes couleurs à chaque image.
fn nearest_256_cached(c: Rgb) -> u8 {
    NEAREST.with(|m| {
        *m.borrow_mut()
            .entry(c)
            .or_insert_with(|| nearest_256(c.0, c.1, c.2))
    })
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

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (5 tests dans `theme`).

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): palette en RGB, fondus vers le fond et cache des 256 couleurs"
```

---

### Task 4 : fenêtre de mise en page

**Files:**
- Modify: `crates/fasttype-tui/src/layout.rs` (version complète ci-dessous)
- Test: `crates/fasttype-tui/tests/layout.rs`

**Interfaces:**
- Produces :
  - `layout_window(words, inputs, active, width, start, lines_after) -> Layout` : mise en page à partir du mot `start`, arrêtée `lines_after` lignes complètes après celle du caret ;
  - `layout_words` devient `layout_window(…, 0, usize::MAX)`.
- Un saut de ligne dans la **saisie** termine aussi la ligne (zen).

- [ ] **Step 1 : écrire les tests qui échouent**

Dans `crates/fasttype-tui/tests/layout.rs`, remplacer la ligne `use` par :
```rust
use fasttype_tui::layout::{
    extras, layout_window, layout_words, letters, prefix_cells, word_cells,
};
```
puis ajouter à la fin :
```rust
#[test]
fn zen_newline_in_the_input_ends_the_line() {
    // zen : les cibles sont vides, le saut de ligne est dans la saisie
    let words = s(&["", "", ""]);
    let l = layout_words(&words, &s(&["ab\n", "cd ", ""]), 2, 40);
    let lines: Vec<Vec<usize>> = l
        .lines
        .iter()
        .map(|line| line.iter().map(|b| b.index).collect())
        .collect();
    assert_eq!(lines, [vec![0], vec![1, 2]]);
    assert_eq!(l.caret, (1, 3));
}

#[test]
fn window_layout_starts_at_a_word_and_stops_after_the_caret() {
    let words: Vec<String> = (0..1000).map(|_| "ab ".to_string()).collect();
    let inputs = vec![String::new(); 1000];
    // largeur 5 : deux mots par ligne
    let l = layout_window(&words, &inputs, 500, 5, 498, 2);
    assert_eq!(l.lines[0][0].index, 498);
    assert_eq!(l.caret, (1, 0));
    assert_eq!(
        l.lines.len(),
        4,
        "ligne du caret + 2 lignes complètes après"
    );
    let full = layout_words(&words, &inputs, 500, 5);
    assert_eq!(full.lines[250], l.lines[1]);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test layout`
Expected: échec de compilation (`no layout_window in layout`).

- [ ] **Step 3 : implémenter**

`crates/fasttype-tui/src/layout.rs` (version complète) :
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
    layout_window(words, inputs, active, width, 0, usize::MAX)
}

/// Place les mots à partir du mot `start` (début d'une ligne), et s'arrête
/// `lines_after` lignes complètes après celle du caret : le coût d'une image
/// ne dépend pas de la longueur du test. Les numéros de ligne partent de `start`.
pub fn layout_window(
    words: &[String],
    inputs: &[String],
    active: usize,
    width: u16,
    start: usize,
    lines_after: usize,
) -> Layout {
    let mut lines: Vec<Vec<WordBox>> = vec![Vec::new()];
    let mut x: u16 = 0;
    let mut caret = None;
    for (index, target) in words.iter().enumerate().skip(start) {
        let input = inputs.get(index).map_or("", String::as_str);
        let w = word_cells(target, input);
        if x > 0 && x.saturating_add(w) > width {
            if caret.is_some_and(|(l, _)| lines.len() - l > lines_after) {
                break;
            }
            lines.push(Vec::new());
            x = 0;
        }
        let line = lines.len() - 1;
        lines[line].push(WordBox { index, x, width: w });
        if index == active {
            let typed = letters(input).chars().count();
            caret = Some((line, x + prefix_cells(target, input, typed)));
        }
        x = x.saturating_add(w + 1);
        if target.ends_with('\n') || input.ends_with('\n') {
            if caret.is_some_and(|(l, _)| lines.len() - l > lines_after) {
                break;
            }
            lines.push(Vec::new());
            x = 0;
        }
    }
    if lines.last().is_some_and(Vec::is_empty) && lines.len() > 1 {
        lines.pop();
    }
    Layout {
        lines,
        caret: caret.unwrap_or((0, 0)),
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (9 tests dans `layout`).

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): mise en page à partir de la ligne visible, saut de ligne zen affiché"
```

---

### Task 5 : modèle du caret

**Files:**
- Create: `crates/fasttype-tui/src/caret.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod caret;`)
- Test: `crates/fasttype-tui/tests/caret.rs`

**Interfaces:**
- Consumes : `anim::{Tween, CARET_EASE, CSS_EASE}`.
- Produces :
  - `CaretStyle { Off, Bar, Block, Outline, Underline }`, avec `from_config` et `is_full_width` ;
  - `smooth_caret_ms(&str) -> f64` ;
  - `CaretTarget { x, y, width }` et `CaretFrame { style, x, y, width, opacity, moving }` ;
  - `Caret`, avec `jump`, `go_to(target, now, duration)`, `target`, `start_blinking(now)`, `stop_blinking`, `is_blinking`, `opacity(now, smooth)`, `is_moving(now)` et `frame(style, now, smooth_blink)` ;
  - `coverage(&CaretFrame, col, row) -> f64`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/caret.rs` :
```rust
use fasttype_tui::caret::{Caret, CaretFrame, CaretStyle, CaretTarget, coverage, smooth_caret_ms};

fn at(x: f64, y: f64) -> CaretTarget {
    CaretTarget { x, y, width: 1.0 }
}

#[test]
fn smooth_caret_durations() {
    assert_eq!(smooth_caret_ms("off"), 0.0);
    assert_eq!(smooth_caret_ms("slow"), 150.0);
    assert_eq!(smooth_caret_ms("medium"), 100.0);
    assert_eq!(smooth_caret_ms("fast"), 85.0);
}

#[test]
fn styles_from_config() {
    assert_eq!(CaretStyle::from_config("default"), CaretStyle::Bar);
    assert_eq!(CaretStyle::from_config("carrot"), CaretStyle::Bar);
    assert_eq!(CaretStyle::from_config("block"), CaretStyle::Block);
    assert_eq!(CaretStyle::from_config("off"), CaretStyle::Off);
    assert!(CaretStyle::Underline.is_full_width());
    assert!(!CaretStyle::Bar.is_full_width());
}

#[test]
fn caret_glides_with_in_out_curve() {
    let mut c = Caret::default();
    c.jump(at(10.0, 5.0));
    c.go_to(at(11.0, 5.0), 0.0, 100.0);
    let f = |t| c.frame(CaretStyle::Bar, t, true);
    assert_eq!(f(0.0).x, 10.0);
    assert_eq!(
        f(50.0).x,
        10.5,
        "inOut est symétrique : mi-chemin à mi-temps"
    );
    assert!(f(25.0).x < 10.25, "départ lent");
    assert_eq!(f(150.0).x, 11.0);
    assert!(f(50.0).moving && !f(100.0).moving);
}

#[test]
fn retarget_mid_glide_never_jumps_back() {
    let mut c = Caret::default();
    c.jump(at(0.0, 0.0));
    c.go_to(at(1.0, 0.0), 0.0, 100.0);
    let shown = c.frame(CaretStyle::Bar, 50.0, true).x;
    c.go_to(at(2.0, 0.0), 50.0, 100.0);
    assert_eq!(c.frame(CaretStyle::Bar, 50.0, true).x, shown);
    let mut last = shown;
    for t in (50..=150).step_by(5) {
        let x = c.frame(CaretStyle::Bar, f64::from(t), true).x;
        assert!(x >= last, "le caret ne recule jamais");
        last = x;
    }
    assert_eq!(last, 2.0);
}

#[test]
fn smooth_off_is_a_jump() {
    let mut c = Caret::default();
    c.go_to(at(7.0, 2.0), 0.0, 0.0);
    assert_eq!(c.frame(CaretStyle::Bar, 0.0, false).x, 7.0);
}

#[test]
fn blink_is_smooth_then_solid_once_typing() {
    let mut c = Caret::default();
    c.stop_blinking();
    c.start_blinking(1000.0);
    assert_eq!(c.opacity(1000.0, true), 0.0);
    assert!((c.opacity(1500.0, true) - 1.0).abs() < 1e-9);
    assert!(
        (c.opacity(1250.0, true) - 0.8024).abs() < 1e-3,
        "courbe ease"
    );
    assert_eq!(c.opacity(2000.0, true), 0.0, "période d'une seconde");
    // clignotement franc quand smoothCaret est off
    assert_eq!(c.opacity(1200.0, false), 1.0);
    assert_eq!(c.opacity(1700.0, false), 0.0);
    c.stop_blinking();
    assert_eq!(c.opacity(1700.0, true), 1.0);
}

#[test]
fn coverage_splits_a_block_between_two_cells() {
    let f = CaretFrame {
        style: CaretStyle::Block,
        x: 3.25,
        y: 2.0,
        width: 1.0,
        opacity: 1.0,
        moving: true,
    };
    assert_eq!(coverage(&f, 3, 2), 0.75);
    assert_eq!(coverage(&f, 4, 2), 0.25);
    assert_eq!(coverage(&f, 5, 2), 0.0);
    assert_eq!(coverage(&f, 3, 1), 0.0);
    let wide = CaretFrame {
        width: 2.0,
        x: 3.0,
        ..f
    };
    assert_eq!(coverage(&wide, 4, 2), 1.0);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test caret`
Expected: échec de compilation (`unresolved import fasttype_tui::caret`).

- [ ] **Step 3 : implémenter `caret.rs`**

Ajouter `pub mod caret;` à `lib.rs`, puis créer `crates/fasttype-tui/src/caret.rs` :
```rust
//! Caret de Monkeytype (`elements/caret.ts`, `caret.scss`) : position en cases
//! fractionnaires, glissement `inOut(1.25)`, clignotement d'une seconde.

use crate::anim::{CARET_EASE, CSS_EASE, Tween};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretStyle {
    Off,
    /// `default` : barre fine avant la lettre.
    Bar,
    /// Pavé derrière la lettre.
    Block,
    /// Contour de la lettre.
    Outline,
    /// Trait sous la lettre.
    Underline,
}

impl CaretStyle {
    /// `caretStyle` ; les carets en image (carrot, banana, monkey) deviennent une barre.
    pub fn from_config(name: &str) -> Self {
        match name {
            "off" => CaretStyle::Off,
            "block" => CaretStyle::Block,
            "outline" => CaretStyle::Outline,
            "underline" => CaretStyle::Underline,
            _ => CaretStyle::Bar,
        }
    }

    /// Block, outline et underline prennent la largeur de la lettre visée.
    pub fn is_full_width(self) -> bool {
        matches!(
            self,
            CaretStyle::Block | CaretStyle::Outline | CaretStyle::Underline
        )
    }
}

/// `smoothCaret` → durée du glissement (ms).
pub fn smooth_caret_ms(setting: &str) -> f64 {
    match setting {
        "slow" => 150.0,
        "medium" => 100.0,
        "fast" => 85.0,
        _ => 0.0,
    }
}

/// Où le caret doit aller : bord gauche de la lettre visée (colonne, ligne, en
/// cases de l'écran) et largeur de cette lettre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretTarget {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

/// Ce qu'il faut dessiner à un instant donné.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretFrame {
    pub style: CaretStyle,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub opacity: f64,
    /// Encore en mouvement : l'image suivante sera différente.
    pub moving: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Caret {
    x: Tween,
    y: Tween,
    width: Tween,
    /// Début du cycle de clignotement ; `None` : caret plein (on tape).
    blink_since: Option<f64>,
}

impl Default for Caret {
    fn default() -> Self {
        Caret {
            x: Tween::fixed(0.0),
            y: Tween::fixed(0.0),
            width: Tween::fixed(1.0),
            blink_since: Some(0.0),
        }
    }
}

impl Caret {
    /// `resetPosition` : place le caret sans animation.
    pub fn jump(&mut self, t: CaretTarget) {
        self.x.jump(t.x);
        self.y.jump(t.y);
        self.width.jump(t.width);
    }

    /// `goTo` : glisse vers la cible en `duration` ms (0 : saut), en repartant
    /// de la position affichée si une animation est en cours.
    pub fn go_to(&mut self, t: CaretTarget, now: f64, duration: f64) {
        self.x.retarget(t.x, now, duration, CARET_EASE);
        self.y.retarget(t.y, now, duration, CARET_EASE);
        self.width.retarget(t.width, now, duration, CARET_EASE);
    }

    pub fn target(&self) -> CaretTarget {
        CaretTarget {
            x: self.x.to,
            y: self.y.to,
            width: self.width.to,
        }
    }

    pub fn start_blinking(&mut self, now: f64) {
        if self.blink_since.is_none() {
            self.blink_since = Some(now);
        }
    }

    /// Chaque frappe rend le caret plein (`stopBlinking`).
    pub fn stop_blinking(&mut self) {
        self.blink_since = None;
    }

    pub fn is_blinking(&self) -> bool {
        self.blink_since.is_some()
    }

    /// Opacité du clignotement : `caretFlashSmooth` (0 → 1 → 0 en 1 s, courbe
    /// `ease` entre les étapes) ou `caretFlashHard` (plein 50 %, éteint 50 %).
    pub fn opacity(&self, now: f64, smooth: bool) -> f64 {
        let Some(since) = self.blink_since else {
            return 1.0;
        };
        let phase = (now - since).rem_euclid(1000.0) / 1000.0;
        if !smooth {
            return if phase < 0.5 { 1.0 } else { 0.0 };
        }
        if phase < 0.5 {
            CSS_EASE.apply(phase * 2.0)
        } else {
            1.0 - CSS_EASE.apply((phase - 0.5) * 2.0)
        }
    }

    pub fn is_moving(&self, now: f64) -> bool {
        self.x.is_running(now) || self.y.is_running(now) || self.width.is_running(now)
    }

    pub fn frame(&self, style: CaretStyle, now: f64, smooth_blink: bool) -> CaretFrame {
        CaretFrame {
            style,
            x: self.x.value(now),
            y: self.y.value(now),
            width: self.width.value(now),
            opacity: self.opacity(now, smooth_blink),
            moving: self.is_moving(now),
        }
    }
}

/// Part de la case (`col`, `row`) couverte par le rectangle du caret
/// `[x, x + width) × [y, y + 1)` : sert à teinter le fond des lettres pendant
/// le glissement du caret bloc (rendu demi-case et plus fin).
pub fn coverage(f: &CaretFrame, col: u16, row: u16) -> f64 {
    let overlap = |a0: f64, a1: f64, b0: f64| (a1.min(b0 + 1.0) - a0.max(b0)).max(0.0);
    overlap(f.x, f.x + f.width, f64::from(col)) * overlap(f.y, f.y + 1.0, f64::from(row))
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test caret && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 7 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): caret glissant (inOut 1.25), clignotement d'une seconde, couverture des cases"
```

---

### Task 6 : caret au pixel près (protocole graphique Kitty)

**Files:**
- Create: `crates/fasttype-tui/src/kitty.rs`
- Modify: `crates/fasttype-tui/src/lib.rs` (`pub mod kitty;`)
- Test: `crates/fasttype-tui/tests/kitty.rs`

**Interfaces:**
- Consumes : `caret::{CaretFrame, CaretStyle}` et `theme::Rgb`.
- Produces :
  - `CellPx { w, h }` avec `from_window(columns, rows, width_px, height_px)` ;
  - `CaretRenderer { Cell, Kitty(CellPx) }` avec `detect(get_env, Option<CellPx>)` ;
  - `base64`, `transmit`, `place`, `hide`, `DELETE_ALL`, `caret_image` et `caret_origin` ;
  - `KittyCaret::{new(cell), draw(out, Option<CaretFrame>, rgb)}`.
- Identifiant d'image = style × 1000 + largeur × 100 + niveau d'opacité + 1. Il y a 16 niveaux d'opacité ; chaque image est transmise une seule fois.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/kitty.rs` :
```rust
use fasttype_tui::caret::{CaretFrame, CaretStyle};
use fasttype_tui::kitty::{
    CaretRenderer, CellPx, KittyCaret, base64, caret_image, caret_origin, place, transmit,
};

const CELL: CellPx = CellPx { w: 10, h: 24 };

fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.to_string())
    }
}

fn bar(x: f64, y: f64, opacity: f64) -> CaretFrame {
    CaretFrame {
        style: CaretStyle::Bar,
        x,
        y,
        width: 1.0,
        opacity,
        moving: false,
    }
}

#[test]
fn base64_matches_rfc_4648() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn cell_size_from_window() {
    assert_eq!(
        CellPx::from_window(100, 50, 1000, 1200),
        Some(CellPx { w: 10, h: 24 })
    );
    assert_eq!(CellPx::from_window(100, 50, 0, 0), None);
}

#[test]
fn renderer_detection() {
    let cell = Some(CELL);
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM", "xterm-kitty")]), cell),
        CaretRenderer::Kitty(CELL)
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM_PROGRAM", "ghostty")]), cell),
        CaretRenderer::Kitty(CELL)
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM", "xterm-kitty"), ("TMUX", "/tmp/x")]), cell),
        CaretRenderer::Cell,
        "tmux ne relaie pas les images"
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM", "xterm-kitty")]), None),
        CaretRenderer::Cell,
        "taille des cases inconnue"
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM_PROGRAM", "Apple_Terminal")]), cell),
        CaretRenderer::Cell
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("FASTTYPE_CARET", "kitty")]), cell),
        CaretRenderer::Kitty(CELL)
    );
    assert_eq!(
        CaretRenderer::detect(
            env(&[("TERM", "xterm-kitty"), ("FASTTYPE_CARET", "cell")]),
            cell
        ),
        CaretRenderer::Cell
    );
}

#[test]
fn transmit_is_chunked_at_4096() {
    let small = transmit(7, 1, 1, &[1, 2, 3, 4]);
    assert_eq!(small, b"\x1b_Ga=t,f=32,s=1,v=1,i=7,q=2,m=0;AQIDBA==\x1b\\");
    let big = String::from_utf8(transmit(8, 40, 40, &[0u8; 40 * 40 * 4])).unwrap();
    // 6400 octets → 8536 caractères base64 → 3 morceaux
    assert_eq!(big.matches("\x1b_G").count(), 3);
    assert!(big.starts_with("\x1b_Ga=t,f=32,s=40,v=40,i=8,q=2,m=1;"));
    assert_eq!(big.matches("\x1b_Gm=1;").count(), 1);
    assert_eq!(big.matches("\x1b_Gm=0;").count(), 1);
}

#[test]
fn placement_moves_the_cursor_then_places() {
    let mut out = Vec::new();
    place(&mut out, 1101, 4, 2, 3, 0).unwrap();
    assert_eq!(out, b"\x1b[3;5H\x1b_Ga=p,i=1101,p=1,X=3,Y=0,C=1,q=2\x1b\\");
}

#[test]
fn images_have_monkeytype_proportions() {
    let rgb = (226, 183, 20);
    let (w, h, px) = caret_image(CaretStyle::Bar, CELL, 1, rgb, 255);
    assert_eq!((w, h), (2, 24), "0,1em de large, toute la hauteur");
    assert_eq!(px.len(), (w * h * 4) as usize);
    assert_eq!(&px[..4], &[226, 183, 20, 255]);
    let (w, h, _) = caret_image(CaretStyle::Underline, CELL, 2, rgb, 255);
    assert_eq!((w, h), (20, 2), "largeur de la lettre (CJK : 2 cases)");
    let (w, h, px) = caret_image(CaretStyle::Outline, CELL, 1, rgb, 255);
    assert_eq!((w, h), (10, 24));
    let alpha = |x: u32, y: u32| px[((y * w + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 5), 255, "bord");
    assert_eq!(alpha(5, 12), 0, "intérieur transparent");
}

#[test]
fn bar_is_centred_on_the_letter_edge() {
    // x = 3.5 cases → 35 px, moins la moitié de la barre (1 px)
    assert_eq!(caret_origin(&bar(3.5, 2.0, 1.0), CELL), (34, 48));
    assert_eq!(
        caret_origin(&bar(0.0, 0.0, 1.0), CELL),
        (0, 0),
        "jamais négatif"
    );
    let u = CaretFrame {
        style: CaretStyle::Underline,
        ..bar(1.0, 1.0, 1.0)
    };
    assert_eq!(caret_origin(&u, CELL), (10, 46), "sous la lettre");
}

#[test]
fn kitty_caret_sends_each_image_once_and_moves_it() {
    let mut k = KittyCaret::new(CELL);
    let rgb = (1, 2, 3);
    let mut out = Vec::new();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), rgb).unwrap();
    let first = String::from_utf8(out.clone()).unwrap();
    assert!(first.contains("a=t"), "transmission");
    assert!(first.ends_with("\x1b[3;4H\x1b_Ga=p,i=1117,p=1,X=4,Y=0,C=1,q=2\x1b\\"));
    out.clear();
    k.draw(&mut out, Some(bar(3.6, 2.0, 1.0)), rgb).unwrap();
    let second = String::from_utf8(out.clone()).unwrap();
    assert!(!second.contains("a=t"), "image déjà transmise");
    assert!(second.contains("X=5"));
    out.clear();
    // clignotement : une autre opacité est une autre image ; l'ancienne est retirée
    k.draw(&mut out, Some(bar(3.6, 2.0, 0.5)), rgb).unwrap();
    let third = String::from_utf8(out.clone()).unwrap();
    assert!(third.starts_with("\x1b_Ga=t"));
    assert!(third.contains("\x1b_Ga=d,d=i,i=1117,q=2\x1b\\"));
    out.clear();
    k.draw(&mut out, None, rgb).unwrap();
    assert_eq!(out, b"\x1b_Ga=d,d=i,i=1109,q=2\x1b\\");
    out.clear();
    // changement de thème : tout est supprimé puis retransmis
    k.draw(&mut out, Some(bar(1.0, 1.0, 1.0)), (9, 9, 9))
        .unwrap();
    let fourth = String::from_utf8(out).unwrap();
    assert!(fourth.starts_with("\x1b_Ga=d,d=A,q=2\x1b\\\x1b_Ga=t"));
}

#[test]
fn block_and_off_are_not_images() {
    let mut k = KittyCaret::new(CELL);
    let mut out = Vec::new();
    let block = CaretFrame {
        style: CaretStyle::Block,
        ..bar(1.0, 1.0, 1.0)
    };
    k.draw(&mut out, Some(block), (1, 2, 3)).unwrap();
    assert!(out.is_empty());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test kitty`
Expected: échec de compilation (`unresolved import fasttype_tui::kitty`).

- [ ] **Step 3 : implémenter `kitty.rs`**

Ajouter `pub mod kitty;` à `lib.rs`, puis créer `crates/fasttype-tui/src/kitty.rs` :
```rust
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
    /// `FASTTYPE_CARET=cell|kitty` force le choix ; sinon Kitty et Ghostty
    /// (hors tmux et screen, qui ne relaient pas les images) ont le rendu Kitty.
    pub fn detect(get: impl Fn(&str) -> Option<String>, cell: Option<CellPx>) -> Self {
        let kitty = match get("FASTTYPE_CARET").as_deref() {
            Some("cell") => false,
            Some("kitty") => true,
            _ => {
                let term = get("TERM").unwrap_or_default();
                let program = get("TERM_PROGRAM").unwrap_or_default();
                let multiplexed = get("TMUX").is_some() || get("STY").is_some();
                !multiplexed
                    && (term == "xterm-kitty"
                        || term == "xterm-ghostty"
                        || program == "ghostty"
                        || get("KITTY_WINDOW_ID").is_some())
            }
        };
        match cell {
            Some(c) if kitty => CaretRenderer::Kitty(c),
            _ => CaretRenderer::Cell,
        }
    }
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
}

impl KittyCaret {
    pub fn new(cell: CellPx) -> Self {
        KittyCaret {
            cell,
            rgb: None,
            sent: HashSet::new(),
            placed: None,
        }
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
        }
        let shown = frame
            .filter(|f| !matches!(f.style, CaretStyle::Off | CaretStyle::Block) && f.opacity > 0.0);
        let Some(f) = shown else {
            if let Some(id) = self.placed.take() {
                hide(out, id)?;
            }
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
        place(out, id, col, row, px % self.cell.w, py % self.cell.h)?;
        self.placed = Some(id);
        Ok(())
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test kitty && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 9 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): caret en image Kitty, déplacé au pixel près, détection du terminal"
```

---

### Task 7 : grands chiffres, graphique braille et stats en direct

**Files:**
- Create: `crates/fasttype-tui/src/view/big.rs`, `crates/fasttype-tui/src/view/chart.rs`, `crates/fasttype-tui/src/view/live.rs`
- Modify: `crates/fasttype-tui/src/view/mod.rs` (`pub mod big; pub mod chart; pub mod live;`)
- Test: `crates/fasttype-tui/tests/chart.rs`

**Interfaces:**
- Consumes : `ChartData.raw` (tâche 1), `Palette::over_bg` (tâche 3).
- Produces :
  - `big::{HEIGHT, width, supported, draw}` ;
  - `chart::{smooth_with_value_window, y_range, sample, ChartView { chart, palette, factor, unit, start_at_zero, duration }}` ;
  - `live::{seconds_to_string, Style3, LiveItem { text, style, is_timer }, LiveStats { items }, WordsBox { left, top, width, lines }, render_bar}`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/chart.rs` :
```rust
use fasttype_core::result::ChartData;
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_tui::theme::{ColorMode, Palette};
use fasttype_tui::view::big;
use fasttype_tui::view::chart::{ChartView, sample, smooth_with_value_window, y_range};
use fasttype_tui::view::live::seconds_to_string;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

fn palette() -> Palette {
    Palette::from_theme(theme(DEFAULT_THEME).unwrap(), ColorMode::TrueColor)
}

#[test]
fn seconds_like_monkeytype() {
    assert_eq!(seconds_to_string(30), "30");
    assert_eq!(seconds_to_string(5), "5");
    assert_eq!(seconds_to_string(60), "01:00");
    assert_eq!(seconds_to_string(75), "01:15");
    assert_eq!(seconds_to_string(605), "10:05");
    assert_eq!(seconds_to_string(3605), "01:00:05");
}

#[test]
fn value_window_smoothing() {
    // le pic isolé n'est pas moyenné avec ses voisins trop éloignés
    let v = [10.0, 12.0, 100.0, 14.0];
    let s = smooth_with_value_window(&v, 1, 25.0);
    assert_eq!(s[0], 11.0);
    assert_eq!(s[1], 11.0);
    assert_eq!(s[2], 100.0);
    assert_eq!(s[3], 14.0);
}

#[test]
fn left_axis_range() {
    assert_eq!(y_range(&[&[42.0, 87.0], &[95.0]], true), (0.0, 100.0));
    assert_eq!(y_range(&[&[42.0, 87.0]], false), (40.0, 90.0));
    assert_eq!(y_range(&[&[]], true), (0.0, 10.0));
}

#[test]
fn spline_passes_through_the_points() {
    let v = [10.0, 30.0, 20.0, 40.0];
    for (i, &x) in v.iter().enumerate() {
        assert!((sample(&v, i as f64) - x).abs() < 1e-9);
    }
    let mid = sample(&v, 1.5);
    assert!(mid > 20.0 && mid < 30.0);
}

#[test]
fn chart_draws_series_errors_and_labels() {
    let p = palette();
    let chart = ChartData {
        wpm: vec![40.0, 60.0, 80.0, 90.0],
        raw: vec![50.0, 70.0, 85.0, 95.0],
        burst: vec![50.0, 80.0, 100.0, 95.0],
        err: vec![0, 2, 0, 1],
    };
    let area = Rect::new(0, 0, 60, 12);
    let mut buf = Buffer::empty(area);
    ChartView {
        chart: &chart,
        palette: &p,
        factor: 1.0,
        unit: "wpm",
        start_at_zero: true,
        duration: 4.0,
    }
    .render(&mut buf, area);
    let cells: Vec<_> = buf.content().iter().collect();
    assert!(
        cells.iter().any(|c| c
            .symbol()
            .starts_with(|ch| ('\u{2801}'..='\u{28ff}').contains(&ch))
            && c.fg == p.main),
        "wpm en main"
    );
    assert!(
        cells.iter().any(|c| c.symbol() == "×" && c.fg == p.error),
        "croix d'erreurs"
    );
    let text: String = cells.iter().map(|c| c.symbol()).collect();
    assert!(text.contains("100"), "max de l'axe arrondi à la dizaine");
    assert!(text.contains("errors"));
    assert!(text.contains("wpm"));
}

#[test]
fn fractional_last_second_is_labelled() {
    let p = palette();
    let chart = ChartData {
        wpm: vec![40.0, 60.0, 62.0],
        raw: vec![40.0, 60.0, 62.0],
        burst: vec![40.0, 60.0, 62.0],
        err: vec![0, 0, 0],
    };
    let area = Rect::new(0, 0, 60, 10);
    let mut buf = Buffer::empty(area);
    ChartView {
        chart: &chart,
        palette: &p,
        factor: 1.0,
        unit: "wpm",
        start_at_zero: true,
        duration: 2.75,
    }
    .render(&mut buf, area);
    let text: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(text.contains("2.75"), "{text}");
}

#[test]
fn big_digits() {
    assert_eq!(big::width("8"), 3);
    assert_eq!(big::width("100%"), 15);
    assert!(big::supported("01:15"));
    assert!(!big::supported("Infinite"));
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));
    big::draw(&mut buf, 0, 0, "7", Style::default());
    assert_eq!(buf[(0, 0)].symbol(), "▀");
    assert_eq!(buf[(2, 2)].symbol(), "▀");
    // hors de l'écran : rien, sans panique
    big::draw(&mut buf, 6, 2, "88", Style::default());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test chart`
Expected: échec de compilation (`unresolved imports fasttype_tui::view::big`…).

- [ ] **Step 3 : implémenter**

Ajouter `pub mod big;`, `pub mod chart;` et `pub mod live;` avant `pub mod notify;` dans `crates/fasttype-tui/src/view/mod.rs`.

`crates/fasttype-tui/src/view/big.rs` :
```rust
//! Grands chiffres sur trois lignes, en demi-blocs : les gros nombres du
//! résultat (wpm, acc) et le timer en style `text`.

use ratatui::buffer::Buffer;
use ratatui::style::Style;

/// Hauteur d'un grand caractère, en lignes.
pub const HEIGHT: u16 = 3;

fn glyph(c: char) -> Option<[&'static str; 3]> {
    Some(match c {
        '0' => ["█▀█", "█ █", "▀▀▀"],
        '1' => ["▀█ ", " █ ", "▀▀▀"],
        '2' => ["▀▀█", "█▀▀", "▀▀▀"],
        '3' => ["▀▀█", " ▀█", "▀▀▀"],
        '4' => ["█ █", "▀▀█", "  ▀"],
        '5' => ["█▀▀", "▀▀█", "▀▀▀"],
        '6' => ["█▀▀", "█▀█", "▀▀▀"],
        '7' => ["▀▀█", "  █", "  ▀"],
        '8' => ["█▀█", "█▀█", "▀▀▀"],
        '9' => ["█▀█", "▀▀█", "▀▀▀"],
        '%' => ["▀ █", " █ ", "█ ▄"],
        '.' => [" ", " ", "▀"],
        ':' => [" ", "▀", "▀"],
        '/' => ["  █", " █ ", "█  "],
        ' ' => [" ", " ", " "],
        _ => return None,
    })
}

/// Largeur en cases d'un texte écrit en grand (une case d'espace entre les caractères).
pub fn width(text: &str) -> u16 {
    let w: u16 = text
        .chars()
        .filter_map(glyph)
        .map(|g| g[0].chars().count() as u16 + 1)
        .sum();
    w.saturating_sub(1)
}

/// Peut-on écrire ce texte en grand ?
pub fn supported(text: &str) -> bool {
    text.chars().all(|c| glyph(c).is_some())
}

/// Écrit `text` en grand à partir de (`x`, `y`).
pub fn draw(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) {
    let area = buf.area;
    let mut cx = x;
    for g in text.chars().filter_map(glyph) {
        for (row, line) in g.iter().enumerate() {
            let ry = y + row as u16;
            if ry < area.bottom() && cx < area.right() {
                buf.set_string(cx, ry, line, style);
            }
        }
        cx += g[0].chars().count() as u16 + 1;
    }
}
```

`crates/fasttype-tui/src/view/chart.rs` :
```rust
//! Graphique du résultat en braille (`ResultChart.tsx`) : wpm (main), raw (main
//! à 60 %, pointillés), burst (sub, rempli en subAlt à 50 %) sur l'axe de
//! gauche, erreurs (croix `error`) sur l'axe de droite. Courbes lissées comme
//! Chart.js (`tension: 0.5`) par une spline de Catmull-Rom.

use crate::theme::Palette;
use fasttype_core::result::ChartData;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

/// `smoothWithValueWindow` (utils/arrays.ts) : moyenne des voisins à ±`window`
/// positions dont la valeur ne s'écarte pas de plus de `value_window`.
pub fn smooth_with_value_window(values: &[f64], window: usize, value_window: f64) -> Vec<f64> {
    (0..values.len())
        .map(|i| {
            let current = values[i];
            let from = i.saturating_sub(window);
            let to = (i + window + 1).min(values.len());
            let near: Vec<f64> = values[from..to]
                .iter()
                .copied()
                .filter(|v| (v - current).abs() <= value_window)
                .collect();
            if near.is_empty() {
                current
            } else {
                near.iter().sum::<f64>() / near.len() as f64
            }
        })
        .collect()
}

/// Axe de gauche (`getMinMax`) : max arrondi à la dizaine supérieure ; min à 0
/// si `startGraphsAtZero`, sinon arrondi à la dizaine inférieure.
pub fn y_range(series: &[&[f64]], start_at_zero: bool) -> (f64, f64) {
    let all = series.iter().flat_map(|s| s.iter().copied());
    let max = all.clone().fold(0.0, f64::max);
    let min = all.fold(f64::INFINITY, f64::min);
    let max = ((max / 10.0).ceil() * 10.0).max(10.0);
    let min = if start_at_zero || !min.is_finite() {
        0.0
    } else {
        (min / 10.0).floor() * 10.0
    };
    (min, if max > min { max } else { min + 10.0 })
}

/// Valeur interpolée à l'abscisse `t` (en indices de points), spline de Catmull-Rom.
pub fn sample(values: &[f64], t: f64) -> f64 {
    match values.len() {
        0 => 0.0,
        1 => values[0],
        n => {
            let t = t.clamp(0.0, (n - 1) as f64);
            let i = (t.floor() as usize).min(n - 2);
            let u = t - i as f64;
            let p = |k: isize| values[(i as isize + k).clamp(0, n as isize - 1) as usize];
            let (p0, p1, p2, p3) = (p(-1), p(0), p(1), p(2));
            0.5 * (2.0 * p1
                + (-p0 + p2) * u
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u * u * u)
        }
    }
}

/// Grille de points braille : 2 × 4 points par case, une couleur par case.
struct Dots {
    w: usize,
    h: usize,
    bits: Vec<u8>,
    /// Série propriétaire de la case (priorité : plus petit = devant).
    owner: Vec<u8>,
}

const DOT: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

impl Dots {
    fn new(w: usize, h: usize) -> Self {
        Dots {
            w,
            h,
            bits: vec![0; w * h],
            owner: vec![u8::MAX; w * h],
        }
    }

    fn set(&mut self, px: usize, py: usize, series: u8) {
        let (cx, cy) = (px / 2, py / 4);
        if cx >= self.w || cy >= self.h {
            return;
        }
        let k = cy * self.w + cx;
        self.bits[k] |= DOT[px % 2][py % 4];
        self.owner[k] = self.owner[k].min(series);
    }
}

pub struct ChartView<'a> {
    pub chart: &'a ChartData,
    pub palette: &'a Palette,
    /// Facteur d'unité (`typingSpeedUnit`) appliqué aux séries de vitesse.
    pub factor: f64,
    pub unit: &'a str,
    pub start_at_zero: bool,
    /// Durée du test (s) : libellé de la dernière seconde fractionnaire.
    pub duration: f64,
}

/// Séries de la priorité d'affichage (Chart.js `order`).
const WPM: u8 = 0;
const RAW: u8 = 1;
const BURST: u8 = 2;

impl ChartView<'_> {
    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        let p = self.palette;
        let speed = |v: &[f64]| -> Vec<f64> { v.iter().map(|x| x * self.factor).collect() };
        let wpm = speed(&self.chart.wpm);
        let raw = speed(&self.chart.raw);
        let burst_raw = speed(&self.chart.burst);
        let burst_max = burst_raw.iter().copied().fold(0.0, f64::max);
        let burst = smooth_with_value_window(&burst_raw, 1, burst_max * 0.25);
        let n = wpm.len();
        if n == 0 || area.width < 12 || area.height < 4 {
            return;
        }
        let (min, max) = y_range(&[&wpm, &raw, &burst], self.start_at_zero);
        let err_max = self.chart.err.iter().copied().max().unwrap_or(0);
        let left_labels = [
            format!("{max}"),
            format!("{}", (min + max) / 2.0),
            format!("{min}"),
        ];
        let lw = left_labels.iter().map(String::len).max().unwrap_or(1) as u16 + 1;
        let rw = err_max.to_string().len() as u16 + 1;
        let plot = Rect {
            x: area.x + lw,
            y: area.y + 1,
            width: area.width.saturating_sub(lw + rw),
            height: area.height.saturating_sub(2),
        };
        if plot.width < 4 || plot.height < 2 {
            return;
        }
        let (w, h) = (usize::from(plot.width), usize::from(plot.height));
        let (pw, ph) = (w * 2, h * 4);
        let to_py = |v: f64| {
            let k = ((v - min) / (max - min)).clamp(0.0, 1.0);
            ((1.0 - k) * (ph - 1) as f64).round() as usize
        };
        let at = |px: usize| px as f64 / (pw - 1).max(1) as f64 * (n - 1) as f64;
        let mut dots = Dots::new(w, h);
        // burst : remplissage sous la courbe en subAlt à 50 %
        let fill = p.over_bg(p.rgb.sub_alt, 0.5);
        for cx in 0..w {
            let py = to_py(sample(&burst, at(cx * 2 + 1)));
            for cy in (py / 4 + 1)..h {
                buf[(plot.x + cx as u16, plot.y + cy as u16)].set_bg(fill);
            }
        }
        let mut line = |values: &[f64], series: u8, dashed: bool| {
            if values.len() != n {
                return;
            }
            let mut prev: Option<usize> = None;
            for px in 0..pw {
                let py = to_py(sample(values, at(px)));
                let on = !dashed || (px / 4) % 2 == 0;
                if on {
                    let (a, b) = match prev {
                        Some(q) => (q.min(py), q.max(py)),
                        None => (py, py),
                    };
                    // relie verticalement au point précédent : pas de trou
                    for y in a..=b {
                        dots.set(px, y, series);
                    }
                }
                prev = Some(py);
            }
        };
        line(&burst, BURST, false);
        line(&raw, RAW, true);
        line(&wpm, WPM, false);
        let color = |series: u8| -> Color {
            match series {
                WPM => p.main,
                RAW => p.over_bg(p.rgb.main, 0.6),
                _ => p.sub,
            }
        };
        for cy in 0..h {
            for cx in 0..w {
                let k = cy * w + cx;
                if dots.bits[k] == 0 {
                    continue;
                }
                let ch = char::from_u32(0x2800 + u32::from(dots.bits[k])).unwrap_or(' ');
                let cell = &mut buf[(plot.x + cx as u16, plot.y + cy as u16)];
                cell.set_char(ch).set_fg(color(dots.owner[k]));
            }
        }
        // erreurs : croix sur l'axe de droite (0..max), rien quand il n'y en a pas
        if err_max > 0 {
            for (i, &e) in self.chart.err.iter().enumerate() {
                if e == 0 {
                    continue;
                }
                let cx = if n > 1 {
                    (i as f64 / (n - 1) as f64 * (w - 1) as f64).round() as u16
                } else {
                    0
                };
                let k = f64::from(e) / f64::from(err_max);
                let cy = ((1.0 - k) * (h - 1) as f64).round() as u16;
                buf[(plot.x + cx, plot.y + cy)]
                    .set_char('×')
                    .set_fg(p.error);
            }
        }
        // axes
        let label = Style::default().fg(p.sub);
        buf.set_string(area.x, area.y, self.unit, label);
        for (k, text) in left_labels.iter().enumerate() {
            let y = match k {
                0 => plot.y,
                1 => plot.y + plot.height / 2,
                _ => plot.bottom() - 1,
            };
            buf.set_string(area.x + lw - 1 - text.len() as u16, y, text, label);
        }
        if err_max > 0 {
            let right = plot.right() + 1;
            buf.set_string(right, plot.y, err_max.to_string(), label);
            buf.set_string(right, plot.bottom() - 1, "0", label);
            let title = "errors";
            if area.right() >= title.len() as u16 {
                buf.set_string(area.right() - title.len() as u16, area.y, title, label);
            }
        }
        // abscisses : les secondes, espacées pour ne pas se chevaucher
        let fractional = n as f64 > self.duration.floor() && self.duration.fract() > 0.0;
        let mut next_free = plot.x;
        for i in 0..n {
            let text = if fractional && i == n - 1 {
                format!("{:.2}", self.duration)
            } else {
                (i + 1).to_string()
            };
            let cx = if n > 1 {
                (i as f64 / (n - 1) as f64 * (w - 1) as f64).round() as u16
            } else {
                0
            };
            let x = (plot.x + cx).saturating_sub(text.len() as u16 / 2);
            if x < next_free || x + text.len() as u16 > area.right() {
                continue;
            }
            buf.set_string(x, plot.bottom(), &text, label);
            next_free = x + text.len() as u16 + 2;
        }
    }
}
```

`crates/fasttype-tui/src/view/live.rs` :
```rust
//! Stats en direct (`live-stats.ts`, `LiveStatsMini.tsx`, `BarTimerProgress.tsx`) :
//! timer, vitesse, précision et burst, en style `mini`, `text` ou `bar`.

use crate::view::big;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::UnicodeWidthStr;

/// `secondsToString` (utils/date-and-time.ts) : « 30 », « 01:15 », « 01:00:05 ».
pub fn seconds_to_string(sec: u64) -> String {
    let (h, m, s) = (sec / 3600, (sec % 3600) / 60, sec % 60);
    let mut out = String::new();
    if h > 0 {
        out.push_str(&format!("{h:02}:"));
    }
    if m > 0 || h > 0 {
        out.push_str(&format!("{m:02}:{s:02}"));
    } else {
        out.push_str(&s.to_string());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style3 {
    Off,
    Mini,
    Text,
}

impl Style3 {
    pub fn from_config(name: &str) -> Self {
        match name {
            "mini" | "flash_mini" => Style3::Mini,
            "text" | "flash_text" => Style3::Text,
            _ => Style3::Off,
        }
    }
}

/// Une valeur affichée et son style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveItem {
    pub text: String,
    pub style: Style3,
    /// Le timer : en style `text`, il va au-dessus des mots.
    pub is_timer: bool,
}

/// Les stats d'un instant, dans l'ordre du site : timer, vitesse, précision, burst.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveStats {
    pub items: Vec<LiveItem>,
}

/// Zone des mots : les stats `mini` vont juste au-dessus, les `text` en grand
/// au-dessus (timer) et au-dessous (le reste).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordsBox {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub lines: u16,
}

impl LiveStats {
    pub fn render(&self, buf: &mut Buffer, area: Rect, words: WordsBox, color: Color) {
        let style = Style::default().fg(color);
        // mini : une rangée juste au-dessus des mots, séparée d'une case
        let mini: Vec<&str> = self
            .items
            .iter()
            .filter(|i| i.style == Style3::Mini)
            .map(|i| i.text.as_str())
            .collect();
        if !mini.is_empty() && words.top > area.y {
            let line = mini.join(" ");
            buf.set_string(words.left, words.top - 1, &line, style);
        }
        // text : en grand, le timer au-dessus des mots, le reste au-dessous
        let big_items = self.items.iter().filter(|i| i.style == Style3::Text);
        let center = |text: &str| area.x + area.width.saturating_sub(big::width(text)) / 2;
        if let Some(timer) = big_items.clone().find(|i| i.is_timer) {
            let y = words.top.saturating_sub(big::HEIGHT + 2);
            if y > area.y && big::supported(&timer.text) {
                big::draw(buf, center(&timer.text), y, &timer.text, style);
            } else if words.top > area.y {
                buf.set_string(words.left, words.top - 1, &timer.text, style);
            }
        }
        let rest: Vec<&str> = big_items
            .filter(|i| !i.is_timer)
            .map(|i| i.text.as_str())
            .collect();
        if !rest.is_empty() {
            let line = rest.join("  ");
            let y = words.top + words.lines + 1;
            if y + big::HEIGHT < area.bottom() && big::supported(&line) {
                big::draw(buf, center(&line), y, &line, style);
            } else if y < area.bottom() {
                let x = area.x + area.width.saturating_sub(line.width() as u16) / 2;
                buf.set_string(x, y, &line, style);
            }
        }
    }
}

/// Barre de progression en haut de l'écran (`timerStyle: bar`), sur une demi-ligne.
pub fn render_bar(buf: &mut Buffer, area: Rect, fraction: f64, color: Color) {
    let width = (fraction.clamp(0.0, 1.0) * f64::from(area.width)).round() as u16;
    let style = Style::default().fg(color);
    for x in area.x..area.x + width {
        buf[(x, area.y)].set_char('▀').set_style(style);
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui --test chart && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 7 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): grands chiffres, graphique braille du résultat, vues des stats en direct"
```

---

### Task 8 : robustesse (Ctrl+C en rafale, signaux, entrée en plein écran)

**Files:**
- Modify: `Cargo.toml`, `crates/fasttype-tui/Cargo.toml`
- Modify: `crates/fasttype-tui/src/input.rs` (version complète), `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/runner.rs`, `crates/fasttype-tui/src/terminal.rs`
- Test: `crates/fasttype-tui/tests/app.rs`

**Interfaces:**
- Produces :
  - `Input::Interrupt` ;
  - `input::spawn_signal_watcher(SyncSender<Input>) -> io::Result<JoinHandle<()>>` (Unix : SIGTERM, SIGHUP, SIGINT, SIGQUIT) ;
  - `runner::apply_burst(&mut App, first, rest) -> Option<f64>`, qui s'arrête au premier Ctrl+C.
- `TerminalGuard` existe dès le mode raw : si la suite de l'entrée échoue, son `drop` restaure le terminal.

- [ ] **Step 1 : écrire les tests qui échouent**

Ajouter à la fin de `crates/fasttype-tui/tests/app.rs` :
```rust
#[test]
fn keys_after_ctrl_c_in_a_burst_are_dropped() {
    let mut a = app("burst-quit", "mode = \"words\"\nwords = 1\n");
    let word = a.session().word(0).trim_end().to_string();
    let mut chars = word.chars();
    let first = chars.next().unwrap();
    // la rafale : première lettre, Ctrl+C, puis la fin du mot et l'espace
    let rest: Vec<_> = std::iter::once(press(Key::Quit, 20.0))
        .chain(chars.map(|c| press(Key::Char(c), 30.0)))
        .chain(std::iter::once(press(Key::Char(' '), 40.0)))
        .collect();
    let oldest =
        fasttype_tui::runner::apply_burst(&mut a, press(Key::Char(first), 10.0), rest.into_iter());
    assert_eq!(oldest, Some(10.0));
    assert!(a.quit);
    assert!(
        matches!(a.screen(), Screen::Test),
        "test abandonné, pas enregistré"
    );
    assert!(a.store.history().unwrap().results.is_empty());
}

#[test]
fn a_signal_quits() {
    let mut a = app("signal", "");
    a.handle(fasttype_tui::input::Input::Interrupt);
    assert!(a.quit);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test app`
Expected: échec de compilation (`no variant Interrupt`, `cannot find function apply_burst`).

- [ ] **Step 3 : implémenter**

Dans `Cargo.toml` (racine), sous `[workspace.dependencies]`, ajouter `signal-hook = "0.3.18"` après `unicode-width`. Dans `crates/fasttype-tui/Cargo.toml`, avant `[dev-dependencies]` :
```toml
[target.'cfg(unix)'.dependencies]
signal-hook.workspace = true
```

`crates/fasttype-tui/src/input.rs` (version complète) :
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
    /// SIGTERM, SIGHUP, SIGINT ou SIGQUIT reçu : quitter proprement.
    Interrupt,
}

impl Input {
    /// Horodatage d'une touche (pour mesurer la latence touche → écran).
    pub fn at(&self) -> Option<f64> {
        match self {
            Input::Key { at, .. } => Some(*at),
            Input::Resize | Input::Interrupt => None,
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
```

Dans `crates/fasttype-tui/src/app.rs`, au début de `pub fn handle(&mut self, input: Input) {`, avant `let Input::Key {` :
```rust
        if input == Input::Interrupt {
            self.quit = true;
            return;
        }
```

Dans `crates/fasttype-tui/src/terminal.rs`, dans `TerminalGuard::enter`, remplacer :
```rust
        enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let mut out = io::stdout();
```
par :
```rust
        enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        // le garde existe dès le mode raw : si la suite échoue, son `drop` restaure
        let guard = TerminalGuard;
        let mut out = io::stdout();
```
et le `Ok(TerminalGuard)` final par `Ok(guard)`.

Dans `crates/fasttype-tui/src/runner.rs` :
- remplacer `use crate::input::{Input, spawn_reader};` par :
```rust
#[cfg(unix)]
use crate::input::spawn_signal_watcher;
use crate::input::{Input, spawn_reader};
```
- ajouter avant `/// Lance l'interface` :
```rust
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
```
- dans `run`, remplacer `spawn_reader(Arc::clone(&clock), tx);` par :
```rust
    #[cfg(unix)]
    spawn_signal_watcher(tx.clone())?;
    spawn_reader(Arc::clone(&clock), tx);
```
- remplacer le bloc `Ok(input) => { … }` de la boucle (avec son `for more in rx.try_iter()`) par :
```rust
            Ok(input) => oldest_key = apply_burst(&mut app, input, rx.try_iter()),
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (`keys_after_ctrl_c_in_a_burst_are_dropped`, `a_signal_quits`).

- [ ] **Step 5 : commit**

```bash
git add Cargo.toml Cargo.lock crates/fasttype-tui
git commit -m "fix(tui): Ctrl+C arrête la rafale, SIGTERM/SIGHUP restaurent le terminal, garde créé dès le mode raw"
```

---

### Task 9 : application animée (caret, focus mode, fondus, stats en direct, résultat)

**Files:**
- Modify: `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/view/test.rs`, `crates/fasttype-tui/src/view/result.rs` (versions complètes)
- Modify: `crates/fasttype-tui/src/runner.rs` (`present` prend `&mut App`)
- Modify: `crates/fasttype-tui/tests/common/mod.rs`, `crates/fasttype-tui/tests/app.rs`, `crates/fasttype-tui/tests/view.rs` (versions complètes)
- Test: `crates/fasttype-tui/tests/motion.rs`

**Interfaces:**
- Consumes : tout ce que produisent les tâches 1 à 8.
- Produces :
  - `App::draw(&mut self, &mut Frame, Option<&Perf>)`, qui met à jour au passage la fenêtre de mots, la cible du caret et le fondu de ligne ;
  - `App::set_caret_renderer`, `set_fps`, `transition() -> Option<Transition>`, `caret_frame() -> Option<CaretFrame>` et `live_stats() -> LiveStats` ;
  - `Transition { Restart { start }, ToResult { start }, FadeIn { start } }` et `FADE_MS = 125.0` ;
  - `view::test::{words_box, WordsView, Chrome}` ;
  - `ResultView`, qui reçoit en plus `start_graphs_at_zero`, avec `speed_text()`, `acc_text()` et `shows_crown(r, outcome)` ;
  - `tests/common` : `render(&mut App, w, h)` et `settle(&mut App, t) -> f64`.
- Comportement :
  - un restart fait disparaître l'écran en 125 ms (touches ignorées), crée le test, puis le fait apparaître en 125 ms ;
  - à la fin du test : fondu de sortie du test, puis fondu d'entrée du résultat ;
  - focus mode à la première frappe : le résumé et les raccourcis s'effacent en 125 ms, le logo passe en couleur sub en 250 ms ;
  - les stats en direct apparaissent en 125 ms ;
  - un avertissement de repli n'est montré qu'une fois ;
  - le caret glisse vers sa cible au moment du dessin. Il est placé sans animation après un restart ou un redimensionnement.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/common/mod.rs` (version complète) :
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
```

`crates/fasttype-tui/tests/app.rs` (version complète) :
```rust
mod common;

use common::{app, press, release, settle, type_text, type_whole_test};
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
    settle(&mut a, 300.0);
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
    settle(&mut a, 200.0);
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
    settle(&mut a, 300.0);
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
    // une fois le focus mode installé, plus rien ne bouge avant le tick
    a.tick(500.0);
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
    settle(&mut a, 99_100.0);
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

#[test]
fn zen_with_quick_restart_on_enter_keeps_newlines_and_ends_with_shift_enter() {
    let mut a = app("zen-enter", "mode = \"zen\"\nquick_restart = \"enter\"\n");
    type_text(&mut a, "hello", 0.0, 100.0);
    a.handle(press(Key::Enter, 600.0));
    assert!(
        a.session().inputs().concat().contains("hello"),
        "Entrée ne relance pas un test zen"
    );
    type_text(&mut a, "abc", 700.0, 100.0);
    a.handle(press(Key::ShiftEnter, 1100.0));
    assert!(matches!(a.screen(), Screen::Result(_)));
}

#[test]
fn unknown_language_warning_is_in_english() {
    let a = app("lang-en", "language = \"klingon_9000k\"\n");
    let texts: Vec<&str> = a
        .notifications
        .items()
        .iter()
        .map(|n| n.text.as_str())
        .collect();
    assert!(
        texts.contains(&"language klingon_9000k not found - using english"),
        "{texts:?}"
    );
}

#[test]
fn zen_result_screen_restarts_on_enter_with_quick_restart_enter() {
    let mut a = app(
        "zen-enter-result",
        "mode = \"zen\"\nquick_restart = \"enter\"\n",
    );
    type_text(&mut a, "hi ", 0.0, 100.0);
    a.handle(press(Key::ShiftEnter, 500.0));
    assert!(matches!(a.screen(), Screen::Result(_)));
    a.handle(press(Key::Enter, 900.0));
    settle(&mut a, 900.0);
    assert!(matches!(a.screen(), Screen::Test));
}

#[test]
fn keys_after_ctrl_c_in_a_burst_are_dropped() {
    let mut a = app("burst-quit", "mode = \"words\"\nwords = 1\n");
    let word = a.session().word(0).trim_end().to_string();
    let mut chars = word.chars();
    let first = chars.next().unwrap();
    // la rafale : première lettre, Ctrl+C, puis la fin du mot et l'espace
    let rest: Vec<_> = std::iter::once(press(Key::Quit, 20.0))
        .chain(chars.map(|c| press(Key::Char(c), 30.0)))
        .chain(std::iter::once(press(Key::Char(' '), 40.0)))
        .collect();
    let oldest =
        fasttype_tui::runner::apply_burst(&mut a, press(Key::Char(first), 10.0), rest.into_iter());
    assert_eq!(oldest, Some(10.0));
    assert!(a.quit);
    assert!(
        matches!(a.screen(), Screen::Test),
        "test abandonné, pas enregistré"
    );
    assert!(a.store.history().unwrap().results.is_empty());
}

#[test]
fn a_signal_quits() {
    let mut a = app("signal", "");
    a.handle(fasttype_tui::input::Input::Interrupt);
    assert!(a.quit);
}

#[test]
fn restart_fades_out_then_in_and_ignores_keys_meanwhile() {
    use fasttype_tui::app::{FADE_MS, Transition};
    let mut a = app("fade-restart", "quick_restart = \"tab\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    let first = a.session().words().to_vec();
    a.handle(press(Key::Tab, 200.0));
    assert_eq!(a.transition(), Some(Transition::Restart { start: 200.0 }));
    // pendant le fondu de sortie, le test n'a pas encore changé et les touches sont ignorées
    a.handle(press(Key::Char('z'), 250.0));
    assert_eq!(a.session().words(), first.as_slice());
    assert_eq!(a.session().input(0), "x");
    a.tick(200.0 + FADE_MS);
    assert_eq!(
        a.transition(),
        Some(Transition::FadeIn {
            start: 200.0 + FADE_MS
        })
    );
    assert_ne!(a.session().words(), first.as_slice());
    // le nouveau test accepte la frappe pendant qu'il apparaît
    let c = a.session().word(0).chars().next().unwrap();
    a.handle(press(Key::Char(c), 340.0));
    assert_eq!(a.session().state(), SessionState::Running);
    a.tick(200.0 + 2.0 * FADE_MS);
    assert_eq!(a.transition(), None);
}

#[test]
fn finishing_fades_the_test_out_then_the_result_in() {
    use fasttype_tui::app::{FADE_MS, Transition};
    let mut a = app("fade-result", "mode = \"words\"\nwords = 10\n");
    let end = type_whole_test(&mut a, 0.0, 300.0);
    let start = match a.transition() {
        Some(Transition::ToResult { start }) => start,
        other => panic!("{other:?}"),
    };
    assert!(start <= end);
    assert!(a.next_deadline().is_some(), "des images pendant le fondu");
    a.tick(start + 2.0 * FADE_MS);
    assert_eq!(a.transition(), None);
    assert_eq!(a.next_deadline(), None, "rien ne bouge sur le résultat");
}

#[test]
fn a_fallback_warning_is_shown_once() {
    let mut a = app(
        "warn-once",
        "language = \"klingon_9000k\"\nquick_restart = \"tab\"\n",
    );
    a.handle(press(Key::Tab, 0.0));
    settle(&mut a, 0.0);
    let n = a
        .notifications
        .items()
        .iter()
        .filter(|n| n.text.contains("klingon_9000k"))
        .count();
    assert_eq!(n, 1);
}

#[test]
fn invalid_custom_theme_colors_are_reported_by_the_config() {
    let a = app(
        "bad-custom",
        "custom_theme = true\ncustom_theme_colors = [\"#000000\"]\n",
    );
    // la config refuse la liste (10 couleurs attendues) et le signale
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("custom_theme_colors"))
    );
}
```

`crates/fasttype-tui/tests/view.rs` (version complète) :
```rust
mod common;

use common::{app, render, row, screen_text, settle, type_text, type_whole_test};

#[test]
fn test_screen_shows_header_words_and_tips() {
    let mut a = app("v-start", "");
    let (buf, caret) = render(&mut a, 80, 24);
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
    let (buf, caret) = render(&mut a, 80, 24);
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
    // le focus mode s'installe en 125 ms (logo : 250 ms)
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(!text.contains("restart"));
    assert!(!text.contains("time 30"));
    assert!(text.contains("30"), "timer mini");
    // le logo reste, en couleur sub
    let x = row(&buf, 1).find("fasttype").unwrap() as u16;
    assert_eq!(buf[(x, 1)].fg, a.palette().sub);
}

#[test]
fn focus_mode_fades_the_chrome() {
    let mut a = app("v-focus-fade", "");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 100.0);
    a.tick(40.0);
    let (buf, _) = render(&mut a, 80, 24);
    let x = row(&buf, 1).find("time 30").expect("encore visible") as u16;
    let fg = buf[(x, 1)].fg;
    assert_ne!(fg, a.palette().sub, "en cours de fondu");
    assert_ne!(fg, a.palette().bg);
}

#[test]
fn too_small_terminal_says_so() {
    let mut a = app("v-small", "");
    let (buf, caret) = render(&mut a, 30, 8);
    assert!(screen_text(&buf).contains("terminal too small"));
    assert_eq!(caret, None);
}

#[test]
fn result_screen_shows_speed_and_details() {
    let mut a = app("v-result", "mode = \"words\"\nwords = 10\n");
    let end = type_whole_test(&mut a, 0.0, 300.0);
    settle(&mut a, end);
    let (buf, caret) = render(&mut a, 100, 24);
    let text = screen_text(&buf);
    assert!(text.contains("wpm"), "{text}");
    assert!(text.contains("acc"));
    assert!(text.contains("characters"));
    assert!(text.contains("test type words 10 english"));
    assert!(text.contains('♛'), "couronne du nouveau record");
    assert!(
        text.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)),
        "graphique en braille"
    );
    assert!(text.contains("next test"));
    assert_eq!(caret, None);
}

#[test]
fn background_is_painted() {
    let mut a = app("v-bg", "");
    let (buf, _) = render(&mut a, 80, 24);
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
        let (buf, caret) = render(&mut a, w, h);
        let (cx, cy) = caret.expect("caret visible");
        let before: String = (cx - 2..cx)
            .map(|x| buf[(x, cy)].symbol().to_string())
            .collect();
        assert_eq!(before, typed, "{w}×{h}");
    }
}

#[test]
fn zero_sized_terminal_does_not_panic() {
    let mut a = app("v-zero", "");
    for (w, h) in [(100, 0), (0, 30), (0, 0), (39, 24), (80, 9)] {
        let (_, caret) = render(&mut a, w, h);
        assert_eq!(caret, None, "{w}×{h}");
    }
}
```

`crates/fasttype-tui/tests/motion.rs` :
```rust
mod common;

use common::{app, press, render, row, screen_text, settle, type_text, type_whole_test};
use fasttype_tui::app::FADE_MS;
use fasttype_tui::input::Key;
use fasttype_tui::kitty::{CaretRenderer, CellPx};
use ratatui::style::Color;

/// Lettre juste suivante du test en cours.
fn next_char(a: &fasttype_tui::app::App) -> char {
    let s = a.session();
    let i = s.active_index();
    s.word(i)
        .chars()
        .nth(s.input(i).chars().count())
        .unwrap_or(' ')
}

#[test]
fn cell_caret_glides_letter_by_letter() {
    // smoothCaret slow : 150 ms
    let mut a = app("m-glide", "smooth_caret = \"slow\"\n");
    let (_, start) = render(&mut a, 80, 24);
    let (x0, y0) = start.unwrap();
    let c = next_char(&a);
    a.handle(press(Key::Char(c), 1000.0));
    let (_, mid) = render(&mut a, 80, 24);
    assert_eq!(
        mid,
        Some((x0, y0)),
        "le glissement part de l'ancienne position"
    );
    assert!(
        a.next_deadline().is_some_and(|d| d < 1020.0),
        "images d'animation"
    );
    a.tick(1150.0);
    let (_, end) = render(&mut a, 80, 24);
    assert_eq!(end, Some((x0 + 1, y0)));
}

#[test]
fn block_caret_tints_two_cells_mid_glide() {
    let mut a = app(
        "m-block",
        "caret_style = \"block\"\nsmooth_caret = \"medium\"\n",
    );
    let (buf, cursor) = render(&mut a, 80, 24);
    assert_eq!(cursor, None, "le bloc n'utilise pas le curseur du terminal");
    let p = *a.palette();
    // caret au repos sur la première lettre : son fond est la couleur du caret
    let (x0, y0) = (0..80u16)
        .flat_map(|x| (0..24u16).map(move |y| (x, y)))
        .find(|&(x, y)| buf[(x, y)].bg != p.bg)
        .expect("une case teintée");
    let c = next_char(&a);
    a.handle(press(Key::Char(c), 1000.0));
    a.tick(1050.0);
    let (buf, _) = render(&mut a, 80, 24);
    let left = buf[(x0, y0)].bg;
    let right = buf[(x0 + 1, y0)].bg;
    assert_ne!(left, p.bg);
    assert_ne!(right, p.bg);
    assert_ne!(left, p.caret, "à mi-chemin, aucune case n'est pleine");
}

#[test]
fn caret_jumps_without_animation_after_a_restart_and_a_resize() {
    let mut a = app(
        "m-reset",
        "quick_restart = \"tab\"\nsmooth_caret = \"slow\"\n",
    );
    render(&mut a, 80, 24);
    type_text(&mut a, "x", 1000.0, 10.0);
    a.handle(press(Key::Tab, 1100.0));
    let t = settle(&mut a, 1100.0);
    let (_, after_restart) = render(&mut a, 80, 24);
    let (_, fresh) = render(&mut app("m-reset-ref", ""), 80, 24);
    assert_eq!(after_restart, fresh, "replacé au début, sans glisser");
    let (_, resized) = render(&mut a, 120, 30);
    let (_, fresh) = render(&mut app("m-reset-ref2", ""), 120, 30);
    assert_eq!(resized, fresh);
    assert!(a.next_deadline().is_none_or(|d| d > t + 100.0));
}

#[test]
fn kitty_renderer_hides_the_cursor_and_exposes_a_pixel_frame() {
    let mut a = app("m-kitty", "");
    a.set_caret_renderer(CaretRenderer::Kitty(CellPx { w: 10, h: 20 }));
    let (_, cursor) = render(&mut a, 80, 24);
    assert_eq!(cursor, None);
    let f = a.caret_frame().expect("caret à dessiner en image");
    assert_eq!(f.y.fract(), 0.0);
    // le caret clignote avant la première frappe : des images sont prévues
    assert!(a.next_deadline().is_some());
    assert!(
        a.caret_look().is_none(),
        "le curseur du terminal n'est pas utilisé"
    );
}

#[test]
fn smooth_line_scroll_fades_the_new_line_in() {
    let mut a = app(
        "m-scroll",
        "smooth_line_scroll = true\nmode = \"words\"\nwords = 200\n",
    );
    render(&mut a, 60, 24);
    let mut t = 1000.0;
    // tape jusqu'au début de la 3e ligne : la ligne du haut est retirée
    let top0 = a.session().words()[0].clone();
    while screen_text(&render(&mut a, 60, 24).0).contains(top0.trim_end()) && t < 60_000.0 {
        let c = next_char(&a);
        a.handle(press(Key::Char(c), t));
        a.tick(t);
        t += 50.0;
    }
    assert!(t < 60_000.0, "le défilement a eu lieu");
    let (buf, _) = render(&mut a, 60, 24);
    // dernière ligne visible encore pâle : sub mêlé au fond
    let p = *a.palette();
    let last = (0..24u16)
        .rev()
        .find(|&y| row(&buf, y).trim().chars().any(char::is_alphabetic) && y < 20)
        .unwrap();
    let x = row(&buf, last).find(|c: char| c.is_alphabetic()).unwrap() as u16;
    assert_ne!(buf[(x, last)].fg, p.sub, "en cours d'apparition");
    a.tick(t + FADE_MS);
    let (buf, _) = render(&mut a, 60, 24);
    assert_eq!(buf[(x, last)].fg, p.sub);
}

#[test]
fn result_fades_in_after_the_test_fades_out() {
    let mut a = app("m-result-fade", "mode = \"words\"\nwords = 10\n");
    render(&mut a, 100, 30);
    // frappe rapide : le relâchement de la dernière touche arrive avant la mi-fondu
    let end = type_whole_test(&mut a, 0.0, 20.0);
    let start = match a.transition() {
        Some(fasttype_tui::app::Transition::ToResult { start }) => start,
        other => panic!("{other:?}"),
    };
    // première moitié : le test pâlit, le résultat n'est pas encore là
    a.tick(start + FADE_MS / 2.0);
    let (buf, _) = render(&mut a, 100, 30);
    assert!(!screen_text(&buf).contains("characters"));
    // seconde moitié : le résultat apparaît
    a.tick(start + FADE_MS * 1.5);
    let (buf, _) = render(&mut a, 100, 30);
    let text = screen_text(&buf);
    assert!(text.contains("characters"));
    let y = (0..30u16)
        .find(|&y| row(&buf, y).contains("characters"))
        .unwrap();
    let x = row(&buf, y).find("characters").unwrap() as u16;
    assert_ne!(buf[(x, y)].fg, a.palette().sub, "encore pâle");
    settle(&mut a, end.max(start));
    let (buf, _) = render(&mut a, 100, 30);
    assert_eq!(buf[(x, y)].fg, a.palette().sub);
}

#[test]
fn live_stats_mini_text_and_bar() {
    let cfg =
        "live_speed_style = \"mini\"\nlive_acc_style = \"mini\"\nlive_burst_style = \"mini\"\n";
    let mut a = app("m-live-mini", cfg);
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(1000.0);
    a.tick(1200.0);
    let (buf, _) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(
        text.contains("29 0 0% 0"),
        "timer, vitesse, précision, burst : {text}"
    );

    let mut a = app("m-live-text", "timer_style = \"text\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    assert!(
        screen_text(&buf).contains("▀▀█"),
        "timer en grands chiffres"
    );

    let mut a = app("m-live-bar", "timer_style = \"bar\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(500.0);
    let (buf, _) = render(&mut a, 80, 24);
    let bar = row(&buf, 0).chars().filter(|&c| c == '▀').count();
    assert!(bar > 60 && bar < 80, "barre qui rétrécit : {bar}");
    assert_eq!(
        buf[(0, 0)].fg,
        a.palette().main,
        "couleur timerColor (main)"
    );
}

#[test]
fn timer_style_off_and_opacity() {
    let mut a = app("m-timer-op", "timer_opacity = \"0.5\"\n");
    type_text(&mut a, "x", 0.0, 100.0);
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    let y = (0..24u16)
        .find(|&y| row(&buf, y).trim() == "30")
        .expect("timer");
    let x = row(&buf, y).find("30").unwrap() as u16;
    let p = a.palette();
    assert_ne!(buf[(x, y)].fg, p.main, "mi-opaque");
    assert!(matches!(buf[(x, y)].fg, Color::Rgb(..)));
}

#[test]
fn backspacing_into_a_removed_line_shows_it_again() {
    // mots faux : le retour arrière peut remonter dans les mots précédents
    let mut a = app("m-back", "mode = \"words\"\nwords = 100\n");
    let mut t = 0.0;
    for _ in 0..30 {
        t = type_text(&mut a, "zz ", t, 20.0);
        a.tick(t);
        render(&mut a, 40, 20);
    }
    let far = a.session().active_index();
    for _ in 0..200 {
        a.handle(press(Key::Backspace, t));
        t += 20.0;
        a.tick(t);
        render(&mut a, 40, 20);
    }
    assert!(
        a.session().active_index() + 10 < far,
        "remonté de plusieurs lignes"
    );
    let (buf, cursor) = render(&mut a, 40, 20);
    let (x, y) = cursor.expect("caret visible");
    assert!(x < 40 && y < 20);
    // la ligne du caret est à l'écran
    assert!(!row(&buf, y).trim().is_empty());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test motion`
Expected: échec de compilation (`no method named set_caret_renderer`, `cannot borrow as mutable`…).

- [ ] **Step 3 : implémenter les vues**

`crates/fasttype-tui/src/view/test.rs` (version complète) :
```rust
//! Écran de test : lignes de mots visibles, en-tête et raccourcis.

use crate::layout::{Layout, char_width, extras, letters};
use crate::theme::Palette;
use crate::view::centered_segments;
use crate::view::live::WordsBox;
use fasttype_core::session::TestSession;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

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

/// Place de la zone de mots : centrée, `maxLineWidth` cases au plus (0 =
/// automatique, 100 cases au plus), 3 lignes (2 en zen).
pub fn words_box(area: Rect, max_line_width: u16, zen: bool) -> WordsBox {
    let width = if max_line_width >= 20 {
        max_line_width.min(area.width.saturating_sub(2))
    } else {
        area.width.saturating_sub(8).min(100)
    };
    let lines = if zen { 2 } else { 3 };
    WordsBox {
        left: area.x + area.width.saturating_sub(width) / 2,
        top: area.y + area.height.saturating_sub(lines) / 2,
        width,
        lines,
    }
}

pub struct WordsView<'a> {
    pub session: &'a TestSession,
    pub palette: &'a Palette,
    /// Lignes visibles, la première en haut de la zone.
    pub layout: &'a Layout,
    pub words: WordsBox,
    pub flip_test_colors: bool,
    pub colorful_mode: bool,
    pub zen: bool,
    /// Palette de la dernière ligne pendant son apparition (`smoothLineScroll`).
    pub last_line: Option<Palette>,
}

impl WordsView<'_> {
    pub fn render(&self, buf: &mut Buffer) {
        let shown = usize::from(self.words.lines);
        for (row, line) in self.layout.lines.iter().take(shown).enumerate() {
            let y = self.words.top + row as u16;
            let palette = match self.last_line {
                Some(ref p) if row + 1 == shown => p,
                _ => self.palette,
            };
            for b in line {
                self.draw_word(buf, palette, self.words.left + b.x, y, b.index);
            }
        }
    }

    fn draw_word(&self, buf: &mut Buffer, p: &Palette, x0: u16, y: u16, index: usize) {
        let s = self.session;
        let (correct, untyped, incorrect, extra) =
            letter_colors(p, self.flip_test_colors, self.colorful_mode);
        let target = s.word(index);
        let input = s.input(index);
        let wrong_committed = !self.zen && s.is_committed(index) && input != target;
        let base = |fg: Color| {
            let st = Style::default().fg(fg).bg(p.bg);
            if wrong_committed {
                st.add_modifier(Modifier::UNDERLINED)
                    .underline_color(p.error)
            } else {
                st
            }
        };
        let right = buf.area.right();
        let mut x = x0;
        let mut put = |c: char, style: Style| {
            if x < right {
                buf[(x, y)].set_char(c).set_style(style);
            }
            x += char_width(c);
        };
        if self.zen {
            for c in letters(input).chars() {
                put(c, base(correct));
            }
            return;
        }
        let mut typed = letters(input).chars();
        for tc in letters(target).chars() {
            let style = match typed.next() {
                None => base(untyped),
                Some(ic) if ic == tc => base(correct),
                Some(_) => base(incorrect),
            };
            put(tc, style);
        }
        for c in extras(target, input).chars() {
            put(c, base(extra));
        }
    }
}

/// En-tête (logo et résumé de la config) et raccourcis du bas, avec leurs
/// opacités du focus mode.
pub struct Chrome<'a> {
    pub palette: &'a Palette,
    /// Couleur du logo : `main`, qui passe à `sub` en focus mode.
    pub logo: Color,
    pub summary: &'a str,
    /// Opacité du résumé de la config et des raccourcis.
    pub opacity: f64,
    pub tips: &'a str,
}

impl Chrome<'_> {
    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        buf.set_string(
            area.x + 2,
            area.y + 1,
            "fasttype",
            Style::default().fg(self.logo).add_modifier(Modifier::BOLD),
        );
        if self.opacity <= 0.0 {
            return;
        }
        let p = self.palette.faded(self.opacity);
        let summary_x = area
            .right()
            .saturating_sub(self.summary.chars().count() as u16 + 2);
        buf.set_string(
            summary_x,
            area.y + 1,
            self.summary,
            Style::default().fg(p.sub),
        );
        let key = Style::default().fg(p.sub_alt).bg(p.sub);
        let text = Style::default().fg(p.sub);
        let tips = [
            ("tab".to_string(), key),
            (" + ".to_string(), text),
            ("enter".to_string(), key),
            (format!(" - {}", self.tips), text),
        ];
        centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
    }
}
```

`crates/fasttype-tui/src/view/result.rs` (version complète) :
```rust
//! Écran de résultat : vitesse et précision, puis le détail du test.

use crate::theme::Palette;
use crate::view::big;
use crate::view::centered_segments;
use crate::view::chart::ChartView;
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
    /// `startGraphsAtZero`.
    pub start_graphs_at_zero: bool,
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

/// La couronne : nouveau record, sauf en quote (`result-pb.ts`).
pub fn shows_crown(r: &TestResult, outcome: Option<RecordOutcome>) -> bool {
    r.mode != Mode::Quote
        && matches!(
            outcome,
            Some(RecordOutcome::Saved(PbOutcome::NewBest { .. }))
        )
}

impl ResultView<'_> {
    fn number(&self, v: f64) -> String {
        if self.decimals {
            format!("{v:.2}")
        } else {
            format!("{}", v.round())
        }
    }

    /// Vitesse en grand : « Infinite » à partir de 1000 wpm (`ResultMainStats.tsx`).
    pub fn speed_text(&self) -> String {
        if self.result.wpm >= 1000.0 {
            "Infinite".to_string()
        } else {
            self.number(self.result.wpm * unit_factor(self.unit))
        }
    }

    /// Précision en grand : « 100% », sinon arrondie vers le bas.
    pub fn acc_text(&self) -> String {
        let acc = self.result.acc;
        if acc >= 100.0 {
            "100%".to_string()
        } else if self.decimals {
            format!("{acc:.2}%")
        } else {
            format!("{}%", acc.floor())
        }
    }

    pub fn render(&self, buf: &mut Buffer, area: Rect) {
        let p = self.palette;
        let r = self.result;
        let label = Style::default().fg(p.sub);
        let value = Style::default().fg(p.main).add_modifier(Modifier::BOLD);
        let detail = Style::default().fg(p.text);
        let factor = unit_factor(self.unit);
        // hauteur : grands chiffres (4), graphique, détails (3) et marges
        let chart_h = area.height.saturating_sub(16).min(12);
        let chart_h = if chart_h >= 5 { chart_h } else { 0 };
        let block = 4 + if chart_h > 0 { chart_h + 1 } else { 0 } + 4;
        let y0 = area.y + area.height.saturating_sub(block + 2) / 2;

        // vitesse et précision en grand, avec leur libellé au-dessus
        let speed = self.speed_text();
        let acc = self.acc_text();
        let w_of = |t: &str| {
            if big::supported(t) {
                big::width(t)
            } else {
                t.chars().count() as u16
            }
        };
        let gap = 6;
        let total = w_of(&speed) + gap + w_of(&acc);
        let x0 = area.x + area.width.saturating_sub(total) / 2;
        let x1 = x0 + w_of(&speed) + gap;
        buf.set_string(x0, y0, self.unit, label);
        if shows_crown(r, self.outcome) {
            buf.set_string(
                x0 + self.unit.len() as u16 + 1,
                y0,
                "♛",
                Style::default().fg(p.main),
            );
        }
        buf.set_string(x1, y0, "acc", label);
        for (x, text) in [(x0, &speed), (x1, &acc)] {
            if big::supported(text) {
                big::draw(buf, x, y0 + 1, text, value);
            } else {
                buf.set_string(x, y0 + 2, text, value);
            }
        }

        let mut y = y0 + 5;
        if chart_h > 0 {
            let width = area.width.saturating_sub(8).min(110);
            let chart_area = Rect {
                x: area.x + area.width.saturating_sub(width) / 2,
                y,
                width,
                height: chart_h,
            };
            ChartView {
                chart: &r.chart,
                palette: p,
                factor,
                unit: self.unit,
                start_at_zero: self.start_graphs_at_zero,
                duration: r.test_duration,
            }
            .render(buf, chart_area);
            y += chart_h + 1;
        }

        let [c, i, e, m] = r.char_stats;
        centered_segments(
            buf,
            area,
            y,
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
            y + 1,
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
                y + 2,
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

- [ ] **Step 4 : implémenter `app.rs`**

`crates/fasttype-tui/src/app.rs` (version complète) :
```rust
//! État de l'application et réaction aux touches. Aucune E/S terminal : la
//! boucle (`runner`) lui passe les entrées horodatées, la fait avancer dans le
//! temps (`tick`) et lui demande de dessiner.
//!
//! Animations (spec §5) : caret glissant, focus mode, fondus de 125 ms entre
//! le test et le résultat, stats en direct. Elles sont calculées à l'instant
//! `now` du dernier `tick` ou de la dernière touche.

use crate::anim::{FrameClock, OUT2, TAILWIND_EASE, Tween};
use crate::caret::{Caret, CaretFrame, CaretStyle, CaretTarget, coverage, smooth_caret_ms};
use crate::input::{Input, Key, Phase};
use crate::kitty::CaretRenderer;
use crate::layout::{Layout, char_width, layout_window, letters};
use crate::perf::Perf;
use crate::session_factory::SessionFactory;
use crate::theme::{ColorMode, Palette, Rgb, mix};
use crate::view::live::{LiveItem, LiveStats, Style3, WordsBox, render_bar, seconds_to_string};
use crate::view::notify::{Level, Notifications};
use crate::view::result::{ResultView, invalid_label, unit_factor};
use crate::view::test::{Chrome, WordsView, words_box};
use crate::view::{MIN_HEIGHT, MIN_WIDTH, fill_background, too_small};
use fasttype_core::result::TestResult;
use fasttype_core::session::{InputOutcome, SessionState, TestSession};
use fasttype_core::spec::Mode;
use fasttype_data::themes::{Rgba, Theme};
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_store::{Config, RecordOutcome, Store};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use std::collections::HashSet;

/// Durée des fondus (test, résultat, restart, stats en direct, focus mode).
pub const FADE_MS: f64 = 125.0;

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

/// Forme, clignotement et couleur du curseur du terminal, quand il sert de caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaretLook {
    pub shape: CaretShape,
    pub blinking: bool,
    pub rgb: (u8, u8, u8),
}

/// Fondu en cours (`test-logic.ts`) : les touches sont ignorées pendant que
/// l'écran disparaît avant un restart, comme sur le site.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transition {
    /// L'écran actuel disparaît ; le nouveau test est créé à `start + FADE_MS`.
    Restart { start: f64 },
    /// Le test disparaît, puis le résultat apparaît.
    ToResult { start: f64 },
    /// Le nouveau test apparaît.
    FadeIn { start: f64 },
}

/// Premier mot de la ligne du haut : la mise en page part de là, son coût ne
/// dépend pas de la longueur du test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Window {
    width: u16,
    start: usize,
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
    /// Instant du dernier `tick` ou de la dernière touche.
    now: f64,
    frames: FrameClock,
    renderer: CaretRenderer,
    caret: Caret,
    /// Le caret sera placé sans animation au prochain dessin (restart, redimensionnement).
    caret_reset: bool,
    /// Ce que le rendu Kitty doit montrer après ce dessin.
    caret_frame: Option<CaretFrame>,
    last_area: Rect,
    window: Window,
    /// Apparition de la dernière ligne après un défilement (`smoothLineScroll`).
    line_fade: Option<f64>,
    /// Opacité du résumé de la config et des raccourcis (focus mode).
    chrome: Tween,
    /// Logo : 0 = `main`, 1 = `sub` (focus mode).
    logo: Tween,
    /// Opacité des stats en direct.
    live: Tween,
    /// Longueur de la barre de progression (`timerStyle: bar`), de 0 à 1.
    bar: Tween,
    /// wpm et raw du dernier tick (une fois par seconde, comme le site).
    live_wpm: (f64, f64),
    transition: Option<Transition>,
    /// Avertissements déjà montrés : un repli n'est signalé qu'une fois.
    warned: HashSet<String>,
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
/// nommé, sinon `serika_dark` ; chaque repli est signalé.
fn resolve_theme(config: &Config) -> (Theme, Vec<String>) {
    let mut warnings = Vec::new();
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
            return (t, warnings);
        }
    }
    let name = config.str("theme");
    let t = match theme(name) {
        Some(t) => t.clone(),
        None => {
            warnings.push(format!("theme {name} not found - using {DEFAULT_THEME}"));
            theme(DEFAULT_THEME).cloned().unwrap_or_else(fallback_theme)
        }
    };
    (t, warnings)
}

/// Couleur `timerColor` en RGB.
fn timer_rgb(p: &Palette, name: &str) -> Rgb {
    match name {
        "black" => (0, 0, 0),
        "sub" => p.rgb.sub,
        "text" => p.rgb.text,
        _ => p.rgb.main,
    }
}

impl App {
    pub fn new(store: Store, color_mode: ColorMode, now: f64, seed: u64) -> App {
        let (theme, theme_warnings) = resolve_theme(&store.config);
        let mut factory = SessionFactory::new();
        let built = factory.build(&store.config, seed);
        let mut app = App {
            palette: Palette::from_theme(&theme, color_mode),
            factory,
            session: built.session,
            screen: Screen::Test,
            notifications: Notifications::default(),
            restart_armed: None,
            seed,
            quit: false,
            now,
            frames: FrameClock::new(60, now),
            renderer: CaretRenderer::Cell,
            caret: Caret::default(),
            caret_reset: true,
            caret_frame: None,
            last_area: Rect::default(),
            window: Window::default(),
            line_fade: None,
            chrome: Tween::fixed(1.0),
            logo: Tween::fixed(0.0),
            live: Tween::fixed(0.0),
            bar: Tween::fixed(0.0),
            live_wpm: (0.0, 0.0),
            transition: None,
            warned: HashSet::new(),
            store,
        };
        let warnings: Vec<String> = app
            .store
            .warnings
            .iter()
            .cloned()
            .chain(theme_warnings)
            .chain(built.warning)
            .collect();
        for w in warnings {
            app.warn(w, now);
        }
        app.caret.start_blinking(now);
        app.update_motion(now);
        app
    }

    /// Choisit le rendu du caret (détecté par la boucle au démarrage).
    pub fn set_caret_renderer(&mut self, renderer: CaretRenderer) {
        self.renderer = renderer;
    }

    /// Cadence des images d'animation (60 par défaut).
    pub fn set_fps(&mut self, fps: u32) {
        self.frames = FrameClock::new(fps, self.now);
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

    pub fn transition(&self) -> Option<Transition> {
        self.transition
    }

    /// Ce que le rendu Kitty doit dessiner après le dernier `draw`.
    pub fn caret_frame(&self) -> Option<CaretFrame> {
        self.caret_frame
    }

    /// Erreur montrée une seule fois par session (un repli de langue ne
    /// revient pas à chaque restart).
    fn warn(&mut self, text: String, now: f64) {
        if self.warned.insert(text.clone()) {
            self.notifications.push(text, Level::Error, now);
        }
    }

    fn caret_style(&self) -> CaretStyle {
        CaretStyle::from_config(self.store.config.str("caretStyle"))
    }

    /// Crée le nouveau test, qui apparaît en fondu.
    fn restart(&mut self, now: f64) {
        self.seed = self
            .seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let built = self.factory.build(&self.store.config, self.seed);
        if let Some(w) = built.warning {
            self.warn(w, now);
        }
        self.session = built.session;
        self.screen = Screen::Test;
        self.restart_armed = None;
        self.window = Window::default();
        self.line_fade = None;
        self.caret_reset = true;
        self.caret.start_blinking(now);
        self.live.jump(0.0);
        self.live_wpm = (0.0, 0.0);
        self.transition = Some(Transition::FadeIn { start: now });
    }

    /// Le restart rapide est refusé pendant un test long, sauf avec Shift
    /// (`quick-restart.ts`). Sinon l'écran disparaît en 125 ms avant le restart.
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
        self.restart_armed = None;
        self.transition = Some(Transition::Restart { start: now });
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
        self.transition = Some(Transition::ToResult { start: now });
    }

    fn check_finished(&mut self, now: f64) {
        if matches!(self.screen, Screen::Test) && self.session.state() == SessionState::Finished {
            self.finish(now);
        }
    }

    pub fn handle(&mut self, input: Input) {
        if input == Input::Interrupt {
            self.quit = true;
            return;
        }
        let Input::Key {
            key,
            phase,
            code,
            at,
        } = input
        else {
            return;
        };
        self.now = self.now.max(at);
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
        // l'écran disparaît avant un restart : les touches sont ignorées
        if matches!(self.transition, Some(Transition::Restart { .. })) {
            return;
        }
        // En zen, Entrée insère un saut de ligne et Shift+Entrée termine le test :
        // ni l'une ni l'autre ne relance.
        let zen = self.session.spec().mode == Mode::Zen && matches!(self.screen, Screen::Test);
        let has_newlines = zen || self.session.has_newlines();
        let quick = self.store.config.str("quickRestart");
        let is_quick = match (quick, key) {
            ("tab", Key::Tab | Key::BackTab) | ("esc", Key::Esc) => true,
            ("enter", Key::Enter) => !has_newlines,
            ("enter", Key::ShiftEnter) => !zen,
            _ => false,
        };
        if is_quick {
            self.try_restart(at, matches!(key, Key::BackTab | Key::ShiftEnter));
            self.update_motion(at);
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
                    self.update_motion(at);
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
        self.update_motion(at);
    }

    pub fn tick(&mut self, now: f64) {
        self.now = self.now.max(now);
        let now = self.now;
        match self.transition {
            Some(Transition::Restart { start }) if now >= start + FADE_MS => self.restart(now),
            Some(Transition::ToResult { start }) if now >= start + 2.0 * FADE_MS => {
                self.transition = None;
            }
            Some(Transition::FadeIn { start }) if now >= start + FADE_MS => {
                self.transition = None;
            }
            _ => {}
        }
        if matches!(self.screen, Screen::Test) {
            if self.session.tick(now) {
                let s = self.session.live_stats();
                self.live_wpm = (s.wpm, s.raw);
            }
            self.check_finished(now);
        }
        if self.line_fade.is_some_and(|t| now >= t + FADE_MS) {
            self.line_fade = None;
        }
        self.notifications.expire(now);
        self.update_motion(now);
    }

    /// Recible les animations qui suivent l'état : focus mode, stats en direct,
    /// barre de progression, clignotement du caret.
    fn update_motion(&mut self, now: f64) {
        let on_test = matches!(self.screen, Screen::Test)
            && !matches!(self.transition, Some(Transition::Restart { .. }));
        let state = self.session.state();
        let running = on_test && state == SessionState::Running;
        self.chrome
            .retarget(if running { 0.0 } else { 1.0 }, now, FADE_MS, TAILWIND_EASE);
        self.logo.retarget(
            if running { 1.0 } else { 0.0 },
            now,
            2.0 * FADE_MS,
            TAILWIND_EASE,
        );
        self.live
            .retarget(if running { 1.0 } else { 0.0 }, now, FADE_MS, OUT2);
        match state {
            SessionState::Ready => self.caret.start_blinking(now),
            _ => self.caret.stop_blinking(),
        }
        let s = &self.session;
        let spec = s.spec();
        let (target, duration, easing) = match (spec.mode, spec.time_limit) {
            (Mode::Time, Some(limit)) if limit > 0 => {
                if state == SessionState::Ready {
                    (1.0, 0.0, OUT2)
                } else {
                    let next = f64::from(s.elapsed_seconds() + 1);
                    (
                        (1.0 - next / f64::from(limit)).max(0.0),
                        1000.0,
                        crate::anim::Easing::Linear,
                    )
                }
            }
            _ => {
                let total = self.progress_total();
                if state == SessionState::Finished {
                    (1.0, FADE_MS, OUT2)
                } else if total == 0 || state == SessionState::Ready {
                    (0.0, 0.0, OUT2)
                } else {
                    let pct = (s.active_index() as f64 / total as f64 * 100.0).floor();
                    (pct / 100.0, 250.0, OUT2)
                }
            }
        };
        self.bar.retarget(target, now, duration, easing);
    }

    /// Nombre de mots attendus (`wordsTotal` du site) ; 0 si illimité.
    fn progress_total(&self) -> usize {
        let spec = self.session.spec();
        match spec.mode {
            Mode::Words => spec.mode2.parse().unwrap_or(0),
            Mode::Zen | Mode::Time => 0,
            _ => self.session.words().len(),
        }
    }

    /// Une image change-t-elle sans frappe ?
    fn animating(&self, now: f64) -> bool {
        let blink_frames = self.caret.is_blinking()
            && matches!(self.screen, Screen::Test)
            && (matches!(self.renderer, CaretRenderer::Kitty(_))
                || self.caret_style() == CaretStyle::Block);
        self.transition.is_some()
            || self.caret.is_moving(now)
            || blink_frames
            || self.line_fade.is_some()
            || self.chrome.is_running(now)
            || self.logo.is_running(now)
            || self.live.is_running(now)
            || (self.bar.is_running(now) && self.store.config.str("timerStyle") == "bar")
    }

    /// Prochain instant où l'écran doit changer sans frappe : image
    /// d'animation, tick du timer, fin d'une notification. `None` : attendre
    /// la prochaine touche (0 % de CPU).
    pub fn next_deadline(&self) -> Option<f64> {
        let tick = matches!(self.screen, Screen::Test)
            .then(|| self.session.next_tick_at())
            .flatten();
        let frame = self
            .animating(self.now)
            .then(|| self.frames.next_after(self.now));
        [tick, self.notifications.next_expiry(), frame]
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

    /// Timer (`live-stats.ts`) : temps restant en time (« 01:15 » au-delà
    /// d'une minute), mots tapés sur le total ailleurs, mot actif en zen.
    pub fn timer(&self) -> String {
        let s = &self.session;
        let seconds = s.elapsed_seconds();
        match (s.spec().mode, s.spec().time_limit) {
            (Mode::Time, Some(limit)) if limit > 0 => {
                seconds_to_string(u64::from(limit.saturating_sub(seconds)))
            }
            (Mode::Time, _) => seconds_to_string(u64::from(seconds)),
            (Mode::Zen, _) => s.active_index().to_string(),
            _ => match self.progress_total() {
                0 => s.active_index().to_string(),
                total => format!("{}/{total}", s.active_index()),
            },
        }
    }

    /// Stats en direct selon `timerStyle`, `liveSpeedStyle`, `liveAccStyle`, `liveBurstStyle`.
    pub fn live_stats(&self) -> LiveStats {
        let c = &self.store.config;
        let factor = unit_factor(c.str("typingSpeedUnit"));
        let blind = c.str("blindMode") == "on";
        let s = &self.session;
        let speed = if blind {
            self.live_wpm.1
        } else {
            self.live_wpm.0
        };
        let acc = if blind { 100.0 } else { s.live_accuracy() };
        let burst = s.last_burst().unwrap_or(0.0);
        let items = [
            (c.str("timerStyle"), self.timer()),
            (
                c.str("liveSpeedStyle"),
                format!("{}", (speed * factor).round()),
            ),
            (c.str("liveAccStyle"), format!("{}%", acc.floor())),
            (
                c.str("liveBurstStyle"),
                format!("{}", (burst * factor).round()),
            ),
        ];
        LiveStats {
            items: items
                .into_iter()
                .enumerate()
                .map(|(k, (style, text))| LiveItem {
                    text,
                    style: Style3::from_config(style),
                    is_timer: k == 0,
                })
                .filter(|i| i.style != Style3::Off)
                .collect(),
        }
    }

    /// Curseur du terminal servant de caret (rendu `cell`, styles fins).
    pub fn caret_look(&self) -> Option<CaretLook> {
        if !matches!(self.screen, Screen::Test) || self.renderer != CaretRenderer::Cell {
            return None;
        }
        let shape = match self.caret_style() {
            CaretStyle::Off | CaretStyle::Block => return None,
            CaretStyle::Outline => CaretShape::Block,
            CaretStyle::Underline => CaretShape::Underline,
            CaretStyle::Bar => CaretShape::Bar,
        };
        Some(CaretLook {
            shape,
            blinking: self.caret.is_blinking(),
            rgb: self.palette.caret_rgb,
        })
    }

    /// Opacité du contenu (test ou résultat) pendant les fondus, et ce qu'il faut montrer.
    fn content(&self, now: f64) -> (bool, f64) {
        let on_result = matches!(self.screen, Screen::Result(_));
        let fade_in = |start: f64| OUT2.apply((now - start) / FADE_MS);
        match self.transition {
            Some(Transition::Restart { start }) => (on_result, 1.0 - fade_in(start)),
            Some(Transition::ToResult { start }) if now < start + FADE_MS => {
                (false, 1.0 - fade_in(start))
            }
            Some(Transition::ToResult { start }) => (true, fade_in(start + FADE_MS)),
            Some(Transition::FadeIn { start }) => (on_result, fade_in(start)),
            None => (on_result, 1.0),
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, perf: Option<&Perf>) {
        let area = frame.area();
        self.caret_frame = None;
        if area.is_empty() {
            return;
        }
        let buf = frame.buffer_mut();
        fill_background(buf, area, &self.palette);
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            too_small(buf, area, &self.palette);
            return;
        }
        if area != self.last_area {
            // redimensionnement : le caret est replacé sans animation
            self.last_area = area;
            self.caret_reset = true;
            self.window = Window::default();
        }
        let now = self.now;
        let (show_result, opacity) = self.content(now);
        let content = self.palette.faded(opacity);
        let mut cursor = None;
        if show_result {
            if let Screen::Result(info) = &self.screen {
                let c = &self.store.config;
                ResultView {
                    result: &info.result,
                    outcome: info.outcome,
                    palette: &content,
                    unit: c.str("typingSpeedUnit"),
                    decimals: c.bool("alwaysShowDecimalPlaces"),
                    start_graphs_at_zero: c.bool("startGraphsAtZero"),
                }
                .render(buf, area);
            }
        } else {
            cursor = self.draw_test(buf, area, &content, opacity, now);
        }
        // logo et focus mode
        let logo = self.palette.over_bg(
            mix(
                self.palette.rgb.main,
                self.palette.rgb.sub,
                self.logo.value(now),
            ),
            1.0,
        );
        let chrome_opacity = if show_result {
            0.0
        } else {
            self.chrome.value(now) * opacity
        };
        Chrome {
            palette: &self.palette,
            logo,
            summary: &self.summary(),
            opacity: chrome_opacity,
            tips: "restart",
        }
        .render(buf, area);
        self.notifications.render(buf, area, &self.palette);
        if let Some(p) = perf {
            buf.set_string(
                area.x + 1,
                area.bottom() - 1,
                p.summary(),
                Style::default().fg(self.palette.sub),
            );
        }
        if let Some(pos) = cursor {
            frame.set_cursor_position(pos);
        }
    }

    /// Mots, stats en direct et caret. Renvoie la position du curseur du
    /// terminal quand il sert de caret.
    fn draw_test(
        &mut self,
        buf: &mut Buffer,
        area: Rect,
        content: &Palette,
        opacity: f64,
        now: f64,
    ) -> Option<(u16, u16)> {
        let zen = self.session.spec().mode == Mode::Zen;
        let max_line_width = self
            .store
            .config
            .int("maxLineWidth")
            .clamp(0, i64::from(u16::MAX)) as u16;
        let words = words_box(area, max_line_width, zen);
        let layout = self.visible_lines(words, now);
        let c = &self.store.config;
        let last_line = self
            .line_fade
            .map(|start| content.faded(OUT2.apply((now - start) / FADE_MS)));
        WordsView {
            session: &self.session,
            palette: content,
            layout: &layout,
            words,
            flip_test_colors: c.bool("flipTestColors"),
            colorful_mode: c.bool("colorfulMode"),
            zen,
            last_line,
        }
        .render(buf);

        // stats en direct, visibles pendant la frappe
        let live = self.live.value(now) * opacity;
        if live > 0.0 {
            let base = c.str("timerOpacity").parse::<f64>().unwrap_or(1.0);
            let color = self
                .palette
                .over_bg(timer_rgb(&self.palette, c.str("timerColor")), base * live);
            self.live_stats().render(buf, area, words, color);
            if c.str("timerStyle") == "bar" && !zen {
                render_bar(buf, area, self.bar.value(now), color);
            }
        }
        self.place_caret(buf, &layout, words, content, opacity, now)
    }

    /// Lignes visibles, la première en haut de la zone de mots. Le caret reste
    /// sur la 2e ligne une fois le premier saut passé : la ligne du haut est
    /// alors retirée (`lineJump`).
    fn visible_lines(&mut self, words: WordsBox, now: f64) -> Layout {
        let s = &self.session;
        let active = s.active_index();
        if words.width != self.window.width || active < self.window.start {
            self.window = Window {
                width: words.width,
                start: 0,
            };
        }
        let mut layout = layout_window(
            s.words(),
            s.inputs(),
            active,
            words.width,
            self.window.start,
            usize::from(words.lines),
        );
        let first = layout.first_visible();
        if first > 0 && first < layout.lines.len() {
            self.window.start = layout.lines[first][0].index;
            layout.lines.drain(..first);
            layout.caret.0 -= first;
            if self.store.config.bool("smoothLineScroll") && !self.caret_reset {
                self.line_fade = Some(now);
            }
        }
        layout
    }

    /// Fait glisser le caret vers sa cible et le dessine : teinte des cases
    /// pour le style bloc, curseur du terminal ou image Kitty pour les autres.
    fn place_caret(
        &mut self,
        buf: &mut Buffer,
        layout: &Layout,
        words: WordsBox,
        content: &Palette,
        opacity: f64,
        now: f64,
    ) -> Option<(u16, u16)> {
        let style = self.caret_style();
        let s = &self.session;
        let restarting = matches!(self.transition, Some(Transition::Restart { .. }));
        if style == CaretStyle::Off
            || s.state() == SessionState::Finished
            || s.words().is_empty()
            || restarting
        {
            return None;
        }
        let (line, col) = layout.caret;
        let width = if style.is_full_width() && s.spec().mode != Mode::Zen {
            let a = s.active_index();
            let typed = letters(s.input(a)).chars().count();
            letters(s.word(a))
                .chars()
                .nth(typed)
                .map_or(1, char_width)
                .max(1)
        } else {
            1
        };
        let target = CaretTarget {
            x: f64::from(words.left + col),
            y: f64::from(words.top) + line as f64,
            width: f64::from(width),
        };
        if self.caret_reset {
            self.caret.jump(target);
            self.caret_reset = false;
        } else if target != self.caret.target() {
            let ms = smooth_caret_ms(self.store.config.str("smoothCaret"));
            self.caret.go_to(target, now, ms);
        }
        let smooth_blink = self.store.config.str("smoothCaret") != "off";
        let mut f = self.caret.frame(style, now, smooth_blink);
        f.opacity *= opacity;
        match style {
            CaretStyle::Block => {
                // pavé derrière la lettre : fond teinté au prorata de la case couverte
                let (x0, y0) = (f.x.floor().max(0.0) as u16, f.y.floor().max(0.0) as u16);
                for row in y0..=y0.saturating_add(1) {
                    for col in x0..=x0.saturating_add(f.width.ceil() as u16) {
                        if row >= area_bottom(buf) || col >= buf.area.right() {
                            continue;
                        }
                        let k = coverage(&f, col, row) * f.opacity;
                        if k > 0.0 {
                            let cell = &mut buf[(col, row)];
                            cell.set_bg(content.over_bg(self.palette.rgb.caret, k));
                        }
                    }
                }
                None
            }
            _ => match self.renderer {
                CaretRenderer::Kitty(_) => {
                    self.caret_frame = Some(f);
                    None
                }
                CaretRenderer::Cell => Some((f.x.round() as u16, f.y.round() as u16)),
            },
        }
    }
}

fn area_bottom(buf: &Buffer) -> u16 {
    buf.area.bottom()
}
```

Dans `crates/fasttype-tui/src/runner.rs`, `present` prend `app: &mut App` au lieu de `app: &App`, et ses deux appels passent `&mut app`.

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (26 dans `app`, 9 dans `view`, 9 dans `motion`).

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-tui
git commit -m "feat(tui): caret glissant, focus mode, fondus de 125 ms, stats en direct, résultat en grand avec graphique"
```

---

### Task 10 : boucle animée, caret Kitty, --fps, benchmark et test pty

**Files:**
- Modify: `crates/fasttype-tui/src/runner.rs`, `crates/fasttype-tui/src/terminal.rs`, `crates/fasttype-tui/src/main.rs` (versions complètes)
- Modify: `crates/fasttype-tui/benches/frame.rs`, `scripts/pty_smoke.py` (versions complètes)
- Test: `crates/fasttype-tui/tests/options.rs`

**Interfaces:**
- Produces :
  - `runner::Options { perf, fps }` et `runner::parse_options(&[String]) -> Result<Options, String>` ;
  - `terminal::mark_kitty_images()`, après quoi `restore` supprime les images ;
  - le binaire accepte `--fps 60|120|144` ;
  - `scripts/pty_smoke.py` accepte `--kitty` et `--sigterm`.
- Boucle :
  - à chaque image, la boucle écrit le caret Kitty (`KittyCaret::draw`) après le dessin ratatui, dans la même sortie synchronisée ;
  - si la taille du terminal change, la taille des cases est relue et les images sont refaites.

- [ ] **Step 1 : écrire le test qui échoue**

`crates/fasttype-tui/tests/options.rs` :
```rust
use fasttype_tui::runner::parse_options;

#[test]
fn command_line_options() {
    let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_options(&args(&[])).unwrap();
    assert!(!o.perf && o.fps == 60);
    let o = parse_options(&args(&["--fps", "144", "--perf"])).unwrap();
    assert!(o.perf && o.fps == 144);
    assert!(parse_options(&args(&["--fps", "30"])).is_err());
    assert!(parse_options(&args(&["--fps"])).is_err());
    assert!(parse_options(&args(&["--nope"])).is_err());
}
```

- [ ] **Step 2 : lancer le test pour vérifier qu'il échoue**

Run: `cargo test -p fasttype-tui --test options`
Expected: échec de compilation (`no parse_options in runner`).

- [ ] **Step 3 : implémenter**

`crates/fasttype-tui/src/terminal.rs` (version complète) :
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
static KITTY_IMAGES: AtomicBool = AtomicBool::new(false);

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
```

`crates/fasttype-tui/src/runner.rs` (version complète) :
```rust
//! Boucle principale : attend une touche ou la prochaine échéance (image
//! d'animation, tick du timer), applique toutes les touches en attente, puis
//! dessine aussitôt une seule image.

use crate::app::{App, CaretLook};
#[cfg(unix)]
use crate::input::spawn_signal_watcher;
use crate::input::{Input, spawn_reader};
use crate::kitty::{CaretRenderer, CellPx, DELETE_ALL, KittyCaret};
use crate::perf::Perf;
use crate::terminal::{FrameWriter, TerminalGuard, mark_kitty_images, queue_caret_look};
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
}

impl Output {
    /// Dessine une image et l'envoie en une écriture, encadrée par la sortie
    /// synchronisée (mode 2026) : le terminal ne montre jamais d'image partielle.
    fn present(&mut self, app: &mut App, perf: &Perf) -> io::Result<()> {
        let size = self.term.size()?;
        if size != self.size {
            self.size = size;
            // la taille des cases a pu changer (zoom) : images à refaire
            if let Some(k) = &mut self.kitty
                && let Some(cell) = cell_px()
            {
                self.term.backend_mut().write_all(DELETE_ALL)?;
                *k = KittyCaret::new(cell);
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
        if let Some(k) = &mut self.kitty {
            k.draw(
                self.term.backend_mut(),
                app.caret_frame(),
                app.palette().caret_rgb,
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
    let renderer = CaretRenderer::detect(|k| std::env::var(k).ok(), cell_px());
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
    let mut out = Output {
        size: term.size()?,
        term,
        frame,
        last_look: None,
        kitty,
    };
    let mut app = App::new(store, color_mode, clock.now_ms(), seed());
    app.set_caret_renderer(renderer);
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
```

`crates/fasttype-tui/src/main.rs` (version complète) :
```rust
//! `fasttype` : clone de Monkeytype pour le terminal.

use fasttype_store::Store;
use fasttype_store::paths::Paths;
use fasttype_tui::runner::{parse_options, run};
use std::process::ExitCode;

const USAGE: &str =
    "usage: fasttype [--perf] [--fps 60|120|144] | --rebuild-pbs | --version | --help

  --perf          show input latency and frame time
  --fps N         animation frames per second (default 60)
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
    match args.first().map(String::as_str) {
        Some("--rebuild-pbs") => return rebuild_pbs(),
        Some("--version" | "-V") => {
            println!("fasttype {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help" | "-h") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let opts = match parse_options(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let perf = opts.perf;
    match run(opts) {
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

Run: `cargo fmt && cargo test -p fasttype-tui --test options`
Expected: 1 test PASS.

- [ ] **Step 4 : benchmark**

`crates/fasttype-tui/benches/frame.rs` (version complète) :
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
```

Run: `cargo bench -p fasttype-tui --bench frame 2>&1 | grep -A1 -E "^(frame|key)"`
Expected : les trois benchmarks sous **1 ms** (mesurés à 78, 78 et 77 µs). `key_and_frame_after_10k_keys` doit rester au niveau de `key_and_frame_200x60`. Un écart signifie qu'un parcours de tous les mots est revenu dans le chemin frappe → image : le chercher, ne pas relever le seuil.

- [ ] **Step 5 : test de bout en bout dans un pseudo-terminal**

`scripts/pty_smoke.py` (version complète) :
```python
"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf] [--kitty] [--sigterm]
  --kitty    caret en image Kitty (cases de 10 × 20 pixels annoncées)
  --sigterm  quitte par SIGTERM au lieu de Ctrl+C"""
import fcntl, os, pty, re, select, signal, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
flags = sys.argv[3:]
perf, kitty, sigterm = "--perf" in flags, "--kitty" in flags, "--sigterm" in flags
os.makedirs(f"{home}/.config/fasttype", exist_ok=True)
with open(f"{home}/.config/fasttype/config.toml", "w") as f:
    f.write('mode = "custom"\n')

pid, fd = pty.fork()
if pid == 0:
    os.environ.update({"HOME": home, "TERM": "xterm-256color", "COLORTERM": "truecolor"})
    os.environ.update({"FASTTYPE_CARET": "kitty" if kitty else "cell"})
    os.environ.pop("XDG_CONFIG_HOME", None)
    os.environ.pop("XDG_DATA_HOME", None)
    os.execv(binary, [binary] + (["--perf"] if perf else []))

# 30 lignes × 100 colonnes ; 1000 × 600 pixels → cases de 10 × 20
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 1000, 600))
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
    t = re.sub(r"\x1b_G[^\x1b]*\x1b\\\\", " ", t)
    t = re.sub(r"\x1b\[[0-9;?<>]*[ A-Za-z]|\x1b\][^\x07]*\x07", " ", t)
    return re.sub(r"\s+", " ", t)

pump(1.0)
for ch in "The quick brown fox jumps over the lazy dog":
    os.write(fd, ch.encode())
    pump(0.08)
pump(0.8)
screen = plain(bytes(out))
if sigterm:
    os.kill(pid, signal.SIGTERM)
else:
    os.write(fd, b"\x03")  # Ctrl+C
pump(1.0)
_, status = os.waitpid(pid, 0)
print("exit", os.waitstatus_to_exitcode(status))
print("result screen:", "test type custom english" in screen)
print("alt screen left:", b"\x1b[?1049l" in out)
print("sync output used:", b"\x1b[?2026h" in out and b"\x1b[?2026l" in out)
print("cursor color reset:", b"\x1b]112\x07" in out)
if kitty:
    print("kitty caret placed:", b"\x1b_Ga=p," in out)
    print("kitty images deleted:", out.rstrip().find(b"\x1b_Ga=d,d=A,q=2\x1b\\") > out.find(b"\x1b_Ga=p,"))
if perf:
    found = re.findall(r"key→flush p50 ([0-9.]+) p99 ([0-9.]+) ms · frame p99 ([0-9.]+) ms · n (\d+)", bytes(out).decode("utf-8", "replace"))
    print("perf (p50, p99, frame p99, n):", found[-1] if found else None)
```

Run: `cargo build --release -p fasttype-tui && for f in --perf --kitty --sigterm; do rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home $f; done`
Expected : pour chacun, `exit 0`, puis `result screen: True`, `alt screen left: True`, `sync output used: True` et `cursor color reset: True`. En plus :
- avec `--kitty` : `kitty caret placed: True` et `kitty images deleted: True` ;
- avec `--perf` : p50 sous 0,5 ms. Le p99 dépend du système et du pty piloté par Python : le noter dans le ledger, sans en faire un critère d'échec.

- [ ] **Step 6 : vérification finale et essai à la main**

Run: `cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, aucun avertissement.

Puis lancer `cargo run --release -p fasttype-tui -- --perf` dans Kitty ou Ghostty, et une seconde fois avec `FASTTYPE_CARET=cell`. Faire un test avec `caret_style = "block"` et un autre avec `smooth_line_scroll = true`. Les tests automatiques ne jugent pas l'aspect : le noter dans le ledger, et signaler à l'utilisateur que l'essai visuel lui revient.

- [ ] **Step 7 : commit**

```bash
git add crates/fasttype-tui scripts
git commit -m "feat(tui): boucle animée, caret Kitty à chaque image, --fps, benchmark long et test pty Kitty/SIGTERM"
```
