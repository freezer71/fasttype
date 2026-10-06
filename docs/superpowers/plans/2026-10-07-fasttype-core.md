# fasttype-core — plan d'implémentation (plan 1 sur 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Créer le workspace Cargo et la crate `fasttype-core`. Elle contient la génération des mots, la session de frappe, le journal d'événements, les stats et le résultat, avec les formules exactes de Monkeytype.

**Architecture:** La logique est pure, sans E/S. Une `TestSession` reçoit les frappes horodatées et applique les règles de saisie et de fin de Monkeytype. Elle écrit chaque événement dans un `EventLog`. Toutes les stats du résultat sont des fonctions pures de ce journal (`stats.rs`), sur le modèle de `frontend/src/ts/test/events/stats.ts`. L'aléatoire passe par un trait injectable, ce qui rend les tests reproductibles.

**Tech Stack:** Rust 1.97 (édition 2024), `serde` 1.0.229, `serde_json` 1.0.151, `proptest` 1.11, `criterion` 0.8.2.

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§3, §4, §7, §9). Référence de comportement : `docs/reference/monkeytype-inventory.md` et le dépôt Monkeytype au commit `574d819`.

**Plans suivants** (écrits une fois celui-ci terminé, car ils consomment son API) :
- plan 2 : `fasttype-data` et `xtask` ;
- plan 3 : `fasttype-store` ;
- plan 4 : `fasttype-tui`.

## Global Constraints

- Rust 1.97, édition 2024, `resolver = "3"`. Licence du workspace : `GPL-3.0-only`.
- `fasttype-core` ne fait **aucune E/S** (ni fichier, ni terminal, ni réseau). Dépendances autorisées : `serde` et `serde_json`.
- Les formules et règles reprennent Monkeytype au commit `574d819`. Quand le code de Monkeytype a une bizarrerie, on la reproduit et on l'explique dans un commentaire.
- Chemin de frappe (`insert`, `backspace`, `delete_word`, `tick`, `live_stats`) : pas d'E/S, et pas de réallocation dans le cas normal (chaînes et journal réservés à l'avance).
- Les commentaires et la documentation sont en français ; les identifiants sont en anglais.
- Les chaînes sont comparées caractère par caractère (`char`), pas en unités UTF-16 comme en JavaScript. La seule différence concerne les caractères hors du plan multilingue de base (emoji), et elle est acceptée.
- Chaque tâche se termine par `cargo test -p fasttype-core`, `cargo clippy -p fasttype-core --all-targets -- -D warnings` et `cargo fmt` (le code du plan n'est pas mis en forme à 100 colonnes : rustfmt le reformate). La tâche 13 vérifie `cargo fmt --check` sur l'ensemble.

## Review Focus

1. **Caractères accentués ou non latins** (`é`, `ß`, `ü`, CJK) : les comptes de caractères doivent correspondre au nombre de lettres visibles, pas d'octets. Test dans la tâche 3 (`counts_accented_letters_as_one_char`).
2. **Horloge qui recule ou frappes après la fin** : aucune panique, les ms ne sont jamais négatives et une frappe après la fin est ignorée. Tests dans la tâche 10 (`clock_going_backwards_is_clamped`, `input_after_finish_is_ignored`).
3. **Liste de mots vide** (langue vide, citation introuvable) : la session existe, ignore les frappes et ne panique pas. Test dans la tâche 10 (`empty_word_list_never_panics`).
4. **Retour arrière en rafale au début d'un test ou sur des mots corrects** : il ne remonte jamais avant le mot 0 et ne rouvre jamais un mot correct. Tests dans la tâche 10 et propriété dans la tâche 13.
5. **Test infini très long** (time 0, plus de 300 s) : les séries du graphique grandissent sans panique et le résultat se calcule. Test dans la tâche 12 (`long_infinite_test_builds_result`).

---

## Structure des fichiers

```
Cargo.toml                         workspace + profil release
LICENSE                            texte GPL-3.0
NOTICE                             crédit Monkeytype + commit
.gitignore
crates/fasttype-core/
├── Cargo.toml
├── src/
│   ├── lib.rs                     déclaration des modules
│   ├── numbers.rs                 calculate_wpm, round2, js_round, mean, std_dev, kogasa, consistency, whorf
│   ├── rng.rs                     trait RandomSource, SplitMix64, Scripted
│   ├── clock.rs                   trait Clock, SystemClock, ManualClock
│   ├── chars.rs                   CharCounts, count_chars, count_words
│   ├── spec.rs                    Mode, Difficulty, CustomLimit, QuoteMeta, TestSpec
│   ├── event.rs                   EventKind, TestEvent, EventContext, EventLog, rejeu
│   ├── stats.rs                   bornes, durée, chars, précision, séries par seconde, AFK
│   ├── punctuation.rs             Punctuator, get_numbers, localize_digits
│   ├── quote.rs                   QuoteFile, Quote, QuoteLength, normalisation
│   ├── sources.rs                 trait WordSource, RandomWords, SequenceWords, CustomWords
│   ├── generator.rs               WordGenerator (ponctuation, nombres, séparateur)
│   ├── session.rs                 TestSession
│   └── result.rs                  TestResult, ChartData, Invalid, PbKey, build_result
├── tests/
│   ├── common/mod.rs              LogBuilder, assert_close, steady_time_test
│   ├── numbers.rs  rng_clock.rs  chars.rs  event.rs  stats.rs  stats_history.rs
│   ├── punctuation.rs  sources.rs  generator.rs  session_input.rs  session_flow.rs
│   ├── result.rs  properties.rs
└── benches/session.rs
```

---

### Task 1 : workspace, licence et utilitaires numériques

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `LICENSE`, `NOTICE`
- Create: `crates/fasttype-core/Cargo.toml`, `crates/fasttype-core/src/lib.rs`, `crates/fasttype-core/src/numbers.rs`
- Test: `crates/fasttype-core/tests/numbers.rs`

**Interfaces:**
- Produces : `fasttype_core::numbers::{calculate_wpm(chars: f64, seconds: f64) -> f64, js_round(x: f64) -> f64, round2(x: f64) -> f64, mean(xs: &[f64]) -> f64, std_dev(xs: &[f64]) -> f64, kogasa(cov: f64) -> f64, consistency(xs: &[f64]) -> f64, whorf(speed: u32, word_len: usize) -> u32}`

- [ ] **Step 1 : créer le workspace**

`Cargo.toml` :
```toml
[workspace]
resolver = "3"
members = ["crates/fasttype-core"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "GPL-3.0-only"
rust-version = "1.97"

[workspace.dependencies]
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
proptest = "1.11.0"
criterion = "0.8.2"

[profile.release]
lto = "fat"
codegen-units = 1
panic = "unwind"
```

`.gitignore` :
```
/target
```

`crates/fasttype-core/Cargo.toml` :
```toml
[package]
name = "fasttype-core"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true

[dev-dependencies]
proptest.workspace = true
criterion.workspace = true
```

`crates/fasttype-core/src/lib.rs` :
```rust
//! Moteur de frappe de fasttype : génération des mots, session, journal
//! d'événements et statistiques. Logique pure, sans aucune E/S.

pub mod numbers;
```

`NOTICE` :
```
fasttype — clone de Monkeytype pour le terminal.

Ce projet reprend le comportement, les formules et les données
(langues, citations, thèmes) de Monkeytype :
  https://github.com/monkeytypegame/monkeytype
  commit 574d819 (2026-10-06), sous licence GPL-3.0.

fasttype est distribué sous la même licence (GPL-3.0, voir LICENSE).
```

Récupérer le texte officiel de la licence :
```bash
curl -fsSL https://www.gnu.org/licenses/gpl-3.0.txt -o LICENSE
head -3 LICENSE
```
Attendu : la ligne `GNU GENERAL PUBLIC LICENSE` apparaît.

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-core/tests/numbers.rs` :
```rust
use fasttype_core::numbers::*;

#[test]
fn wpm_is_chars_over_five_per_minute() {
    assert_eq!(calculate_wpm(50.0, 6.0), 100.0);
    assert_eq!(calculate_wpm(250.0, 60.0), 50.0);
}

#[test]
fn wpm_is_zero_without_duration() {
    assert_eq!(calculate_wpm(10.0, 0.0), 0.0);
    assert_eq!(calculate_wpm(10.0, -1.0), 0.0);
}

#[test]
fn js_round_rounds_half_towards_positive_infinity() {
    assert_eq!(js_round(2.5), 3.0);
    assert_eq!(js_round(-2.5), -2.0);
    assert_eq!(js_round(2.4), 2.0);
    assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
}

#[test]
fn round2_keeps_two_decimals() {
    assert_eq!(round2(12.345678), 12.35);
    assert_eq!(round2(2.0), 2.0);
    assert_eq!(round2(99.999), 100.0);
}

#[test]
fn mean_and_population_std_dev() {
    let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    assert_eq!(mean(&xs), 5.0);
    assert_eq!(std_dev(&xs), 2.0);
    assert_eq!(mean(&[]), 0.0);
    assert_eq!(std_dev(&[]), 0.0);
}

#[test]
fn kogasa_is_100_without_variation() {
    assert_eq!(kogasa(0.0), 100.0);
    assert!(kogasa(0.5) < 100.0);
}

#[test]
fn consistency_matches_monkeytype() {
    assert_eq!(consistency(&[100.0, 100.0, 100.0]), 100.0);
    // cov = 20 / 100 = 0.2 → kogasa ≈ 80.0002 → 80
    assert_eq!(consistency(&[80.0, 120.0]), 80.0);
    // NaN (0 / 0) et liste vide → 0
    assert_eq!(consistency(&[]), 0.0);
    assert_eq!(consistency(&[0.0, 0.0]), 0.0);
}

#[test]
fn whorf_lowers_threshold_for_long_words() {
    assert_eq!(whorf(100, 3), 100);
    assert_eq!(whorf(100, 5), 88); // floor(100 × 1.03^-4) = 88
    assert_eq!(whorf(100, 1), 100); // plafonné à la vitesse
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test numbers`
Expected: échec de compilation (`cannot find function calculate_wpm`).

- [ ] **Step 4 : implémenter `numbers.rs`**

`crates/fasttype-core/src/numbers.rs` :
```rust
//! Utilitaires numériques repris de Monkeytype :
//! `frontend/src/ts/utils/numbers.ts` (calculateWpm),
//! `packages/util/src/numbers.ts` (roundTo2, mean, stdDev, kogasa),
//! `frontend/src/ts/utils/misc.ts` (whorf).

/// `calculateWpm` : un « mot » vaut 5 caractères.
pub fn calculate_wpm(chars: f64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return 0.0;
    }
    chars / 5.0 / (seconds / 60.0)
}

/// `Math.round` de JavaScript : les demis sont arrondis vers +∞.
pub fn js_round(x: f64) -> f64 {
    if x.is_infinite() || x.is_nan() {
        return x;
    }
    (x + 0.5).floor()
}

/// `roundTo2` : `Math.round((x + Number.EPSILON) * 100) / 100`.
pub fn round2(x: f64) -> f64 {
    js_round((x + f64::EPSILON) * 100.0) / 100.0
}

/// Moyenne ; 0 pour une liste vide (comme le `try/catch` de Monkeytype).
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Écart-type de **population** (division par n) ; 0 pour une liste vide.
pub fn std_dev(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let m = mean(xs);
    (xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64).sqrt()
}

/// `kogasa(cov) = 100 × (1 − tanh(cov + cov³/3 + cov⁵/5))`.
pub fn kogasa(cov: f64) -> f64 {
    100.0 * (1.0 - (cov + cov.powi(3) / 3.0 + cov.powi(5) / 5.0).tanh())
}

/// Consistency d'une série : `roundTo2(kogasa(stdDev / mean))`, 0 si NaN ou 0
/// (`if (!consistency || isNaN(consistency)) consistency = 0`).
pub fn consistency(xs: &[f64]) -> f64 {
    let c = round2(kogasa(std_dev(xs) / mean(xs)));
    if c.is_nan() { 0.0 } else { c }
}

/// Seuil « flex » du min burst : `min(speed, floor(speed × 1.03^(−2 × (len − 3))))`.
pub fn whorf(speed: u32, word_len: usize) -> u32 {
    let flex = (f64::from(speed) * 1.03_f64.powf(-2.0 * (word_len as f64 - 3.0))).floor();
    speed.min(flex as u32)
}
```

- [ ] **Step 5 : lancer les tests**

Run: `cargo test -p fasttype-core --test numbers && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 8 tests PASS, aucun avertissement.

- [ ] **Step 6 : commit**

```bash
git add Cargo.toml .gitignore LICENSE NOTICE crates/fasttype-core
git commit -m "feat(core): workspace, licence GPL-3.0 et utilitaires numériques Monkeytype"
```

---

### Task 2 : source aléatoire et horloge

**Files:**
- Create: `crates/fasttype-core/src/rng.rs`, `crates/fasttype-core/src/clock.rs`
- Modify: `crates/fasttype-core/src/lib.rs`
- Test: `crates/fasttype-core/tests/rng_clock.rs`

**Interfaces:**
- Produces :
  - `rng::RandomSource: Send` avec `next_f64(&mut self) -> f64` (dans [0, 1)), `below(&mut self, n: usize) -> usize` et `int_in(&mut self, min: u32, max: u32) -> u32` (bornes incluses) ;
  - `rng::SplitMix64::{new(seed: u64), from_entropy()}` ;
  - `rng::Scripted::{new(values: &[f64]), consumed(&self) -> usize}` ;
  - `rng::shuffle<T>(items: &mut [T], rng: &mut dyn RandomSource)` ;
  - `clock::Clock` avec `now_ms(&self) -> f64` ;
  - `clock::SystemClock::new()` ;
  - `clock::ManualClock::{new(ms: f64), set(&self, ms: f64), advance(&self, ms: f64)}`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/rng_clock.rs` :
```rust
use fasttype_core::clock::{Clock, ManualClock};
use fasttype_core::rng::{RandomSource, Scripted, SplitMix64, shuffle};

#[test]
fn splitmix_is_reproducible_and_in_unit_interval() {
    let mut a = SplitMix64::new(7);
    let mut b = SplitMix64::new(7);
    for _ in 0..1000 {
        let x = a.next_f64();
        assert_eq!(x, b.next_f64());
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn below_and_int_in_follow_monkeytype_formulas() {
    // Math.floor(random() * n) et randomIntFromRange(min, max)
    let mut r = Scripted::new(&[0.0, 0.999, 0.5]);
    assert_eq!(r.below(4), 0);
    assert_eq!(r.below(4), 3);
    assert_eq!(r.int_in(1, 9), 5); // 1 + floor(0.5 × 9)
    assert_eq!(r.consumed(), 3);
}

#[test]
fn scripted_cycles_through_values() {
    let mut r = Scripted::new(&[0.25]);
    assert_eq!(r.next_f64(), 0.25);
    assert_eq!(r.next_f64(), 0.25);
}

#[test]
#[should_panic(expected = "Scripted")]
fn scripted_without_values_panics_on_use() {
    Scripted::new(&[]).next_f64();
}

#[test]
fn shuffle_is_a_permutation() {
    let mut items: Vec<u32> = (0..20).collect();
    shuffle(&mut items, &mut SplitMix64::new(3));
    let mut sorted = items.clone();
    sorted.sort();
    assert_eq!(sorted, (0..20).collect::<Vec<_>>());
}

#[test]
fn manual_clock_moves_only_when_told() {
    let c = ManualClock::new(100.0);
    assert_eq!(c.now_ms(), 100.0);
    c.advance(50.0);
    assert_eq!(c.now_ms(), 150.0);
    c.set(10.0);
    assert_eq!(c.now_ms(), 10.0);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test rng_clock`
Expected: échec de compilation (`unresolved import fasttype_core::clock`).

- [ ] **Step 3 : implémenter**

`crates/fasttype-core/src/rng.rs` :
```rust
//! Aléatoire injectable. Les formules reprennent `Math.random()` et les
//! utilitaires de Monkeytype (`randomIntFromRange`, `randomElementFromArray`,
//! `shuffle`).

/// Source de nombres aléatoires uniformes, comme `Math.random()`.
pub trait RandomSource: Send {
    /// Nombre uniforme dans [0, 1).
    fn next_f64(&mut self) -> f64;

    /// `Math.floor(random() * n)`, borné à `n - 1`. `n` doit être > 0.
    fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "below(0)");
        ((self.next_f64() * n as f64) as usize).min(n - 1)
    }

    /// `randomIntFromRange(min, max)` : bornes incluses.
    fn int_in(&mut self, min: u32, max: u32) -> u32 {
        min + self.below((max - min + 1) as usize) as u32
    }
}

/// Générateur SplitMix64 : rapide, reproductible avec une graine.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Graine tirée de l'heure système et d'une adresse de pile.
    pub fn from_entropy() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let addr = &nanos as *const u64 as u64;
        Self::new(nanos ^ addr.rotate_left(32))
    }
}

impl RandomSource for SplitMix64 {
    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Source scriptée pour les tests : renvoie les valeurs données, en boucle.
pub struct Scripted {
    values: Vec<f64>,
    consumed: usize,
}

impl Scripted {
    pub fn new(values: &[f64]) -> Self {
        Self { values: values.to_vec(), consumed: 0 }
    }

    /// Nombre de valeurs déjà tirées.
    pub fn consumed(&self) -> usize {
        self.consumed
    }
}

impl RandomSource for Scripted {
    fn next_f64(&mut self) -> f64 {
        assert!(!self.values.is_empty(), "Scripted : aucune valeur disponible");
        let v = self.values[self.consumed % self.values.len()];
        self.consumed += 1;
        v
    }
}

/// Mélange de Fisher-Yates (`shuffle` de Monkeytype).
pub fn shuffle<T>(items: &mut [T], rng: &mut dyn RandomSource) {
    for i in (1..items.len()).rev() {
        let j = rng.below(i + 1);
        items.swap(i, j);
    }
}
```

`crates/fasttype-core/src/clock.rs` :
```rust
//! Horloge injectable : réelle dans l'application, manuelle dans les tests.

use std::cell::Cell;
use std::time::Instant;

pub trait Clock {
    /// Millisecondes écoulées depuis une origine fixe (monotone).
    fn now_ms(&self) -> f64;
}

pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self { origin: Instant::now() }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> f64 {
        self.origin.elapsed().as_secs_f64() * 1000.0
    }
}

pub struct ManualClock {
    ms: Cell<f64>,
}

impl ManualClock {
    pub fn new(ms: f64) -> Self {
        Self { ms: Cell::new(ms) }
    }

    pub fn set(&self, ms: f64) {
        self.ms.set(ms);
    }

    pub fn advance(&self, ms: f64) {
        self.ms.set(self.ms.get() + ms);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> f64 {
        self.ms.get()
    }
}
```

Ajouter dans `lib.rs`, en gardant l'ordre alphabétique :
```rust
pub mod clock;
pub mod numbers;
pub mod rng;
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: tous les tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): source aléatoire injectable et horloge"
```

---

### Task 3 : comptage des caractères

**Files:**
- Create: `crates/fasttype-core/src/chars.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod chars;`)
- Test: `crates/fasttype-core/tests/chars.rs`

**Interfaces:**
- Produces :
  - `chars::CharCounts { all_correct: u32, correct_word: u32, incorrect: u32, extra: u32, missed: u32 }` (`Copy`, `Default`, `AddAssign`, serde) ;
  - `chars::count_chars(input: &str, target: &str, credit_partial: bool) -> CharCounts` ;
  - `chars::count_words<'a, I: IntoIterator<Item = (&'a str, &'a str, bool)>>(words: I, credit_partial_last: bool) -> CharCounts`. Chaque élément est `(saisie, cible, est_le_dernier)` ; on s'arrête après le dernier.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/chars.rs` :
```rust
use fasttype_core::chars::{CharCounts, count_chars, count_words};

fn c(all_correct: u32, correct_word: u32, incorrect: u32, extra: u32, missed: u32) -> CharCounts {
    CharCounts { all_correct, correct_word, incorrect, extra, missed }
}

#[test]
fn exact_word_with_separator() {
    assert_eq!(count_chars("hello ", "hello ", false), c(6, 6, 0, 0, 0));
}

#[test]
fn wrong_letter_then_early_space() {
    // h e l justes ; 'o' au lieu de 'l' ; ' ' au lieu de 'o' (l'input contient un espace) ; ' ' manquant
    assert_eq!(count_chars("helo ", "hello ", false), c(3, 0, 2, 0, 1));
}

#[test]
fn letter_typed_instead_of_separator_is_extra() {
    assert_eq!(count_chars("hellox", "hello ", false), c(5, 0, 0, 1, 0));
}

#[test]
fn letter_after_full_word_then_space() {
    // 's' tapé à la place de l'espace alors que l'input contient un espace → incorrect ; l'espace final dépasse → extra
    assert_eq!(count_chars("hellos ", "hello ", false), c(5, 0, 1, 1, 0));
}

#[test]
fn separator_of_wrong_word_is_extra() {
    assert_eq!(count_chars("hxllo ", "hello ", false), c(4, 0, 1, 1, 0));
}

#[test]
fn partial_credit_only_for_correct_prefix() {
    assert_eq!(count_chars("hel", "hello ", true), c(3, 3, 0, 0, 0));
    assert_eq!(count_chars("hel", "hello ", false), c(3, 0, 0, 0, 3));
    assert_eq!(count_chars("hex", "hello ", true), c(2, 0, 1, 0, 0));
}

#[test]
fn counts_accented_letters_as_one_char() {
    assert_eq!(count_chars("café ", "café ", false), c(5, 5, 0, 0, 0));
    assert_eq!(count_chars("cafe ", "café ", false), c(4, 0, 1, 0, 0));
    assert_eq!(count_chars("straße", "straße", false), c(6, 6, 0, 0, 0));
}

#[test]
fn count_words_stops_after_last_and_credits_it() {
    let words = [("the ", "the ", false), ("ca", "cat ", true), ("ignored", "x", false)];
    assert_eq!(count_words(words, true), c(6, 6, 0, 0, 0));
    assert_eq!(count_words(words, false), c(6, 4, 0, 0, 2));
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test chars`
Expected: échec de compilation (`unresolved import fasttype_core::chars`).

- [ ] **Step 3 : implémenter `chars.rs`**

```rust
//! `countChars` de Monkeytype (`frontend/src/ts/utils/strings.ts`).

use serde::{Deserialize, Serialize};
use std::ops::AddAssign;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharCounts {
    pub all_correct: u32,
    pub correct_word: u32,
    pub incorrect: u32,
    pub extra: u32,
    pub missed: u32,
}

impl AddAssign for CharCounts {
    fn add_assign(&mut self, o: Self) {
        self.all_correct += o.all_correct;
        self.correct_word += o.correct_word;
        self.incorrect += o.incorrect;
        self.extra += o.extra;
        self.missed += o.missed;
    }
}

/// Compare la saisie d'un mot à sa cible (séparateur compris), position par position.
pub fn count_chars(input: &str, target: &str, credit_partial: bool) -> CharCounts {
    let mut c = CharCounts::default();
    let word_correct = input == target;
    let partially_correct = target.starts_with(input);
    let input_has_space = input.contains(' ');
    let mut ii = input.chars();
    let mut ti = target.chars();
    loop {
        match (ii.next(), ti.next()) {
            (None, None) => break,
            (Some(i), Some(t)) if i == t => {
                if t == ' ' && !word_correct {
                    c.extra += 1;
                } else {
                    c.all_correct += 1;
                }
                if word_correct || (credit_partial && partially_correct) {
                    c.correct_word += 1;
                }
            }
            (None, Some(_)) => {
                if !credit_partial {
                    c.missed += 1;
                }
            }
            // au-delà de la cible, ou lettre tapée à la place de l'espace final
            (Some(_), None) => c.extra += 1,
            (Some(_), Some(' ')) if !input_has_space => c.extra += 1,
            _ => c.incorrect += 1,
        }
    }
    c
}

/// Additionne `count_chars` mot par mot et s'arrête après le dernier mot
/// (boucle de `getChars`). Seul le dernier mot peut recevoir le crédit partiel.
pub fn count_words<'a, I>(words: I, credit_partial_last: bool) -> CharCounts
where
    I: IntoIterator<Item = (&'a str, &'a str, bool)>,
{
    let mut total = CharCounts::default();
    for (input, target, last) in words {
        total += count_chars(input, target, last && credit_partial_last);
        if last {
            break;
        }
    }
    total
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core --test chars && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 8 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): comptage des caractères identique à countChars"
```

---

### Task 4 : spécification du test et journal d'événements

**Files:**
- Create: `crates/fasttype-core/src/spec.rs`, `crates/fasttype-core/src/event.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod event;` et `pub mod spec;`)
- Test: `crates/fasttype-core/tests/event.rs`

**Interfaces:**
- Produces, dans `spec` :
  - `Mode { Time, Words, Quote, Zen, Custom }` (serde en minuscules) ;
  - `Difficulty { Normal, Expert, Master }` ;
  - `CustomLimit { Word(u32), Section(u32), Time(u32) }` (0 = infini) ;
  - `QuoteMeta { id: u32, group: u8, source: String }` ;
  - `TestSpec { mode, mode2: String, language: String, punctuation: bool, numbers: bool, difficulty, lazy_mode: bool, time_limit: Option<u32>, custom_limit: Option<CustomLimit>, quote: Option<QuoteMeta> }` ;
  - constructeurs `TestSpec::time(seconds, language, punctuation, numbers)`, `words(count, language, punctuation, numbers)`, `quote(meta, language)`, `zen(language)`, `custom(limit, language, punctuation, numbers)` ;
  - méthodes `is_timed(&self) -> bool` et `is_long(&self) -> bool`.
- Produces, dans `event` :
  - `EventKind { TimerStart, TimerStep { second: u32 }, TimerEnd, KeyDown { code: u32 }, KeyUp { code: u32 }, Insert { word_index: u32, char_index: u32, ch: char, correct: bool }, DeleteChar { word_index: u32 }, DeleteWord { word_index: u32 } }`, avec `word_index(&self) -> Option<u32>` ;
  - `TestEvent { ms: f64, kind: EventKind }` ;
  - `EventContext { mode: Mode, timed: bool, bailed_out: bool, target_words: Vec<String> }` ;
  - `EventLog { context, events: Vec<TestEvent> }`, avec `with_capacity(context, cap)`, `push(ms, kind)`, `target(i: u32) -> Option<&str>` et `word_inputs(until_ms: Option<f64>) -> BTreeMap<u32, String>` ;
  - `event::apply_event(inputs: &mut BTreeMap<u32, String>, kind: &EventKind)` ;
  - `event::active_word_index(inputs: &BTreeMap<u32, String>) -> u32`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/event.rs` :
```rust
use fasttype_core::event::{EventContext, EventKind, EventLog, TestEvent, active_word_index};
use fasttype_core::spec::{CustomLimit, Mode, TestSpec};

fn log() -> EventLog {
    let ctx = EventContext { mode: Mode::Words, timed: false, bailed_out: false, target_words: vec!["ab ".into(), "cd".into()] };
    let mut log = EventLog::with_capacity(ctx, 16);
    log.push(0.0, EventKind::TimerStart);
    for (i, ch) in "ab ".chars().enumerate() {
        log.push(i as f64 * 100.0, EventKind::Insert { word_index: 0, char_index: i as u32, ch, correct: true });
    }
    log.push(300.0, EventKind::Insert { word_index: 1, char_index: 0, ch: 'x', correct: false });
    log.push(400.0, EventKind::DeleteChar { word_index: 1 });
    log.push(500.0, EventKind::Insert { word_index: 1, char_index: 0, ch: 'c', correct: true });
    log
}

#[test]
fn replays_inserts_and_deletes() {
    let inputs = log().word_inputs(None);
    assert_eq!(inputs[&0], "ab ");
    assert_eq!(inputs[&1], "c");
}

#[test]
fn replay_stops_at_given_time() {
    let inputs = log().word_inputs(Some(300.0));
    assert_eq!(inputs[&1], "x");
}

#[test]
fn delete_word_clears_input() {
    let mut l = log();
    l.push(600.0, EventKind::DeleteWord { word_index: 1 });
    assert_eq!(l.word_inputs(None)[&1], "");
}

#[test]
fn active_word_follows_committed_space() {
    let l = log();
    assert_eq!(active_word_index(&l.word_inputs(Some(250.0))), 1); // "ab " validé
    assert_eq!(active_word_index(&l.word_inputs(Some(150.0))), 0);
    assert_eq!(active_word_index(&Default::default()), 0);
}

#[test]
fn target_lookup() {
    let l = log();
    assert_eq!(l.target(1), Some("cd"));
    assert_eq!(l.target(5), None);
}

#[test]
fn event_serializes_with_type_tag() {
    let e = TestEvent { ms: 12.5, kind: EventKind::Insert { word_index: 0, char_index: 1, ch: 'é', correct: true } };
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains("\"type\":\"insert\""), "{json}");
    assert_eq!(serde_json::from_str::<TestEvent>(&json).unwrap(), e);
}

#[test]
fn timed_matches_is_timed_test() {
    assert!(TestSpec::time(30, "english", false, false).is_timed());
    assert!(TestSpec::words(0, "english", false, false).is_timed());
    assert!(!TestSpec::words(25, "english", false, false).is_timed());
    assert!(TestSpec::custom(CustomLimit::Time(60), "english", false, false).is_timed());
    assert!(TestSpec::custom(CustomLimit::Word(0), "english", false, false).is_timed());
    assert!(!TestSpec::custom(CustomLimit::Word(50), "english", false, false).is_timed());
    assert!(!TestSpec::zen("english").is_timed());
}

#[test]
fn long_tests_refuse_quick_restart() {
    assert!(TestSpec::time(0, "english", false, false).is_long());
    assert!(TestSpec::time(900, "english", false, false).is_long());
    assert!(!TestSpec::time(120, "english", false, false).is_long());
    assert!(TestSpec::words(1000, "english", false, false).is_long());
    assert!(!TestSpec::words(100, "english", false, false).is_long());
}

#[test]
fn mode2_matches_monkeytype() {
    assert_eq!(TestSpec::time(30, "english", false, false).mode2, "30");
    assert_eq!(TestSpec::words(50, "english", false, false).mode2, "50");
    assert_eq!(TestSpec::zen("english").mode2, "zen");
    assert_eq!(TestSpec::custom(CustomLimit::Word(5), "english", false, false).mode2, "custom");
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test event`
Expected: échec de compilation (`unresolved import fasttype_core::event`).

- [ ] **Step 3 : implémenter `spec.rs`**

```rust
//! Description d'un test : mode, sous-option, langue, options.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Time,
    Words,
    Quote,
    Zen,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    #[default]
    Normal,
    Expert,
    Master,
}

/// Limite d'un texte custom ; 0 = infini.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "value", rename_all = "lowercase")]
pub enum CustomLimit {
    Word(u32),
    Section(u32),
    Time(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteMeta {
    pub id: u32,
    /// Groupe de longueur : 0 short, 1 medium, 2 long, 3 thicc.
    pub group: u8,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSpec {
    pub mode: Mode,
    /// `getMode2` : durée, nombre de mots, id de citation, "custom" ou "zen".
    pub mode2: String,
    pub language: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub difficulty: Difficulty,
    pub lazy_mode: bool,
    /// Fin au temps (time, custom limité au temps). `Some(0)` = infini.
    pub time_limit: Option<u32>,
    pub custom_limit: Option<CustomLimit>,
    pub quote: Option<QuoteMeta>,
}

impl TestSpec {
    fn base(mode: Mode, mode2: String, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self {
            mode,
            mode2,
            language: language.to_string(),
            punctuation,
            numbers,
            difficulty: Difficulty::Normal,
            lazy_mode: false,
            time_limit: None,
            custom_limit: None,
            quote: None,
        }
    }

    pub fn time(seconds: u32, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self { time_limit: Some(seconds), ..Self::base(Mode::Time, seconds.to_string(), language, punctuation, numbers) }
    }

    pub fn words(count: u32, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self::base(Mode::Words, count.to_string(), language, punctuation, numbers)
    }

    /// En quote, punctuation et numbers sont toujours désactivés.
    pub fn quote(meta: QuoteMeta, language: &str) -> Self {
        Self { quote: Some(meta.clone()), ..Self::base(Mode::Quote, meta.id.to_string(), language, false, false) }
    }

    pub fn zen(language: &str) -> Self {
        Self::base(Mode::Zen, "zen".into(), language, false, false)
    }

    pub fn custom(limit: CustomLimit, language: &str, punctuation: bool, numbers: bool) -> Self {
        let time_limit = match limit {
            CustomLimit::Time(s) => Some(s),
            _ => None,
        };
        Self { time_limit, custom_limit: Some(limit), ..Self::base(Mode::Custom, "custom".into(), language, punctuation, numbers) }
    }

    /// `isTimedTest` (stats.ts) : le dernier mot reçoit le crédit partiel.
    pub fn is_timed(&self) -> bool {
        match self.mode {
            Mode::Time => true,
            Mode::Words => self.mode2 == "0",
            Mode::Custom => matches!(
                self.custom_limit,
                Some(CustomLimit::Time(_)) | Some(CustomLimit::Word(0)) | Some(CustomLimit::Section(0))
            ),
            Mode::Quote | Mode::Zen => false,
        }
    }

    /// Test « long » : le restart rapide est refusé (`utils/quick-restart.ts`).
    pub fn is_long(&self) -> bool {
        let big = |n: u32, threshold: u32| n == 0 || n >= threshold;
        match (self.mode, self.custom_limit) {
            (Mode::Time, _) => big(self.time_limit.unwrap_or(0), 900),
            (Mode::Words, _) => big(self.mode2.parse().unwrap_or(0), 1000),
            (Mode::Custom, Some(CustomLimit::Time(s))) => big(s, 900),
            (Mode::Custom, Some(CustomLimit::Word(n) | CustomLimit::Section(n))) => big(n, 1000),
            _ => false,
        }
    }
}
```

- [ ] **Step 4 : implémenter `event.rs`**

```rust
//! Journal d'événements d'un test, sur le modèle de
//! `frontend/src/ts/test/events/types.ts`. Toutes les stats en sont déduites.

use crate::spec::Mode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    TimerStart,
    TimerStep { second: u32 },
    TimerEnd,
    KeyDown { code: u32 },
    KeyUp { code: u32 },
    /// Insertion d'un caractère (y compris le séparateur qui valide le mot).
    Insert { word_index: u32, char_index: u32, ch: char, correct: bool },
    DeleteChar { word_index: u32 },
    DeleteWord { word_index: u32 },
}

impl EventKind {
    pub fn word_index(&self) -> Option<u32> {
        match self {
            EventKind::Insert { word_index, .. }
            | EventKind::DeleteChar { word_index }
            | EventKind::DeleteWord { word_index } => Some(*word_index),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestEvent {
    /// Millisecondes depuis le début du test.
    pub ms: f64,
    #[serde(flatten)]
    pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventContext {
    pub mode: Mode,
    /// `isTimedTest`.
    pub timed: bool,
    pub bailed_out: bool,
    /// Mots cibles avec leur séparateur. Vide en zen (la cible est alors la saisie).
    pub target_words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLog {
    pub context: EventContext,
    pub events: Vec<TestEvent>,
}

impl EventLog {
    pub fn with_capacity(context: EventContext, capacity: usize) -> Self {
        Self { context, events: Vec::with_capacity(capacity) }
    }

    pub fn push(&mut self, ms: f64, kind: EventKind) {
        self.events.push(TestEvent { ms, kind });
    }

    pub fn target(&self, word_index: u32) -> Option<&str> {
        self.context.target_words.get(word_index as usize).map(String::as_str)
    }

    /// Saisie de chaque mot touché, en rejouant les événements jusqu'à
    /// `until_ms` inclus (`getEventsPerWord` + `getInputFromDom`).
    pub fn word_inputs(&self, until_ms: Option<f64>) -> BTreeMap<u32, String> {
        let mut inputs = BTreeMap::new();
        for e in &self.events {
            if until_ms.is_some_and(|u| e.ms > u) {
                break;
            }
            apply_event(&mut inputs, &e.kind);
        }
        inputs
    }
}

/// Applique un événement de saisie à l'état des mots.
pub fn apply_event(inputs: &mut BTreeMap<u32, String>, kind: &EventKind) {
    match kind {
        EventKind::Insert { word_index, ch, .. } => inputs.entry(*word_index).or_default().push(*ch),
        EventKind::DeleteChar { word_index } => {
            inputs.entry(*word_index).or_default().pop();
        }
        EventKind::DeleteWord { word_index } => inputs.entry(*word_index).or_default().clear(),
        _ => {}
    }
}

/// `inferActiveWordIndex` : dernier mot non vide, ou le suivant s'il a été
/// validé par un espace.
pub fn active_word_index(inputs: &BTreeMap<u32, String>) -> u32 {
    match inputs.iter().rev().find(|(_, s)| !s.is_empty()) {
        None => 0,
        Some((&i, s)) if s.ends_with(' ') => i + 1,
        Some((&i, _)) => i,
    }
}
```

- [ ] **Step 5 : lancer les tests**

Run: `cargo test -p fasttype-core --test event && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 9 tests PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): spécification du test et journal d'événements rejouable"
```

---

### Task 5 : stats de base (bornes, durée, caractères, précision)

**Files:**
- Create: `crates/fasttype-core/src/stats.rs`, `crates/fasttype-core/tests/common/mod.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod stats;`)
- Test: `crates/fasttype-core/tests/stats.rs`

**Interfaces:**
- Consumes : `EventLog`, `apply_event`, `active_word_index`, `count_words`, `round2`, `js_round`.
- Produces, dans `stats` :
  - `timer_boundaries(log) -> Vec<f64>` ;
  - `last_keypress_to_end_ms(log) -> f64` ;
  - `start_to_first_keypress_ms(log) -> f64` ;
  - `test_duration_ms(log) -> f64` ;
  - `chars(log, count_partial_last_word: bool) -> CharCounts` ;
  - `Accuracy { correct: u32, incorrect: u32, percentage: f64 }` et `accuracy(log, until_ms: Option<f64>) -> Accuracy`.
- Produces, dans `tests/common` : `LogBuilder::{new(mode, timed, targets), at(ms), step(ms), typ(word_index, text), backspace(word_index), tick(second), bailed_out(), end(ms) -> EventLog}`, `assert_close(a, b)` et `steady_time_test(total_s, active_s, target_word, typed_word, per_second) -> EventLog`.

- [ ] **Step 1 : écrire l'outil de test partagé**

`crates/fasttype-core/tests/common/mod.rs` :
```rust
#![allow(dead_code)]

use fasttype_core::event::{EventContext, EventKind, EventLog};
use fasttype_core::spec::Mode;
use std::collections::BTreeMap;

/// Construit un journal d'événements à la main, dans l'ordre chronologique.
pub struct LogBuilder {
    log: EventLog,
    t: f64,
    step: f64,
    inputs: BTreeMap<u32, String>,
}

impl LogBuilder {
    /// Journal avec un `TimerStart` à 0 ms et des frappes espacées de 100 ms.
    pub fn new(mode: Mode, timed: bool, targets: &[&str]) -> Self {
        let context = EventContext {
            mode,
            timed,
            bailed_out: false,
            target_words: targets.iter().map(|s| s.to_string()).collect(),
        };
        let mut log = EventLog::with_capacity(context, 256);
        log.push(0.0, EventKind::TimerStart);
        Self { log, t: 0.0, step: 100.0, inputs: BTreeMap::new() }
    }

    pub fn at(mut self, ms: f64) -> Self {
        self.t = ms;
        self
    }

    pub fn step(mut self, ms: f64) -> Self {
        self.step = ms;
        self
    }

    /// Tape `text` dans le mot `word_index` : keydown + insert par caractère.
    pub fn typ(mut self, word_index: u32, text: &str) -> Self {
        for ch in text.chars() {
            let input = self.inputs.entry(word_index).or_default();
            let char_index = input.chars().count() as u32;
            let correct = match self.log.context.target_words.get(word_index as usize) {
                Some(t) => t.chars().nth(char_index as usize) == Some(ch),
                None => true,
            };
            input.push(ch);
            self.log.push(self.t, EventKind::KeyDown { code: ch as u32 });
            self.log.push(self.t, EventKind::Insert { word_index, char_index, ch, correct });
            self.t += self.step;
        }
        self
    }

    pub fn backspace(mut self, word_index: u32) -> Self {
        self.inputs.entry(word_index).or_default().pop();
        self.log.push(self.t, EventKind::KeyDown { code: 8 });
        self.log.push(self.t, EventKind::DeleteChar { word_index });
        self.t += self.step;
        self
    }

    pub fn tick(mut self, second: u32) -> Self {
        self.log.push(f64::from(second) * 1000.0, EventKind::TimerStep { second });
        self
    }

    pub fn bailed_out(mut self) -> Self {
        self.log.context.bailed_out = true;
        self
    }

    pub fn end(mut self, ms: f64) -> EventLog {
        self.log.push(ms, EventKind::TimerEnd);
        self.log
    }
}

pub fn assert_close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

/// Test chronométré régulier : pendant `active_s` secondes, `per_second`
/// frappes par seconde (réparties à l'intérieur de la seconde) qui tapent
/// `typed_word` en boucle contre la cible `target_word` ; puis rien jusqu'à
/// `total_s`. Un tick par seconde, fin à `total_s × 1000`.
pub fn steady_time_test(total_s: u32, active_s: u32, target_word: &str, typed_word: &str, per_second: u32) -> EventLog {
    let len = target_word.chars().count();
    assert_eq!(len, typed_word.chars().count());
    let word_count = (active_s * per_second) as usize / len + 2;
    let targets: Vec<&str> = vec![target_word; word_count];
    let typed: Vec<char> = typed_word.chars().collect();
    let mut b = LogBuilder::new(Mode::Time, true, &targets);
    let mut n = 0usize;
    for s in 0..total_s {
        if s < active_s {
            for k in 0..per_second {
                let t = f64::from(s) * 1000.0 + f64::from(k + 1) * 1000.0 / f64::from(per_second + 1);
                b = b.at(t).typ((n / len) as u32, &typed[n % len].to_string());
                n += 1;
            }
        }
        b = b.tick(s + 1);
    }
    b.end(f64::from(total_s) * 1000.0)
}
```

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-core/tests/stats.rs` :
```rust
mod common;

use common::{LogBuilder, assert_close};
use fasttype_core::chars::CharCounts;
use fasttype_core::numbers::calculate_wpm;
use fasttype_core::spec::Mode;
use fasttype_core::stats;

#[test]
fn words_test_end_to_end_numbers() {
    // "the cat" : 7 frappes de 0 à 600 ms
    let log = LogBuilder::new(Mode::Words, false, &["the ", "cat"]).typ(0, "the ").typ(1, "cat").end(600.0);
    assert_close(stats::test_duration_ms(&log), 600.0);
    let c = stats::chars(&log, false);
    assert_eq!(c.correct_word, 7);
    assert_eq!(c.all_correct, 7);
    assert_close(calculate_wpm(f64::from(c.correct_word), 0.6), 140.0);
    assert_eq!(stats::timer_boundaries(&log), vec![600.0]);
    assert_close(stats::accuracy(&log, None).percentage, 100.0);
}

#[test]
fn timed_test_boundaries_have_no_tail() {
    let log = LogBuilder::new(Mode::Time, true, &["ab ", "cd ", "ef "])
        .at(100.0).typ(0, "ab ").tick(1)
        .at(1100.0).typ(1, "cd").tick(2)
        .end(2000.0);
    assert_eq!(stats::timer_boundaries(&log), vec![1000.0, 2000.0]);
    let c = stats::chars(&log, false);
    assert_eq!((c.all_correct, c.correct_word), (5, 5)); // crédit partiel du dernier mot
}

#[test]
fn untimed_tail_only_from_half_second() {
    let short = LogBuilder::new(Mode::Words, false, &["a"]).typ(0, "a").end(2400.0);
    assert_eq!(stats::timer_boundaries(&short), vec![1000.0, 2000.0]);
    let long = LogBuilder::new(Mode::Words, false, &["a"]).typ(0, "a").end(2600.0);
    assert_eq!(stats::timer_boundaries(&long), vec![1000.0, 2000.0, 2600.0]);
}

#[test]
fn accuracy_counts_corrected_mistakes() {
    let log = LogBuilder::new(Mode::Words, false, &["to"]).typ(0, "x").backspace(0).typ(0, "to").end(500.0);
    let a = stats::accuracy(&log, None);
    assert_eq!((a.correct, a.incorrect), (2, 1));
    assert_close(a.percentage, 200.0 / 3.0);
    assert_eq!(stats::chars(&log, false), CharCounts { all_correct: 2, correct_word: 2, ..Default::default() });
}

#[test]
fn accuracy_is_zero_without_inserts() {
    let log = LogBuilder::new(Mode::Words, false, &["to"]).end(500.0);
    assert_eq!(stats::accuracy(&log, None).percentage, 0.0);
}

#[test]
fn zen_trims_trailing_idle_under_seven_seconds() {
    let short = LogBuilder::new(Mode::Zen, false, &[]).typ(0, "hi ").end(5000.0);
    assert_close(stats::test_duration_ms(&short), 200.0);
    let long = LogBuilder::new(Mode::Zen, false, &[]).typ(0, "hi ").end(9000.0);
    assert_close(stats::test_duration_ms(&long), 9000.0);
    // en zen, la cible est la saisie elle-même
    assert_eq!(stats::chars(&short, false).correct_word, 3);
    assert_eq!(stats::last_keypress_to_end_ms(&short), 0.0);
}

#[test]
fn bail_out_gives_partial_credit() {
    let base = || LogBuilder::new(Mode::Words, false, &["hello ", "world"]).typ(0, "hel");
    let bailed = base().bailed_out().end(1000.0);
    assert_eq!(stats::chars(&bailed, false), CharCounts { all_correct: 3, correct_word: 3, ..Default::default() });
    let normal = base().end(1000.0);
    assert_eq!(stats::chars(&normal, false), CharCounts { all_correct: 3, missed: 3, ..Default::default() });
}

#[test]
fn missed_and_incorrect_on_early_space() {
    let log = LogBuilder::new(Mode::Words, false, &["hello ", "world"]).typ(0, "hel ").typ(1, "world").end(900.0);
    assert_eq!(
        stats::chars(&log, false),
        CharCounts { all_correct: 8, correct_word: 5, incorrect: 1, extra: 0, missed: 2 }
    );
}

#[test]
fn start_to_first_keypress() {
    let log = LogBuilder::new(Mode::Words, false, &["a"]).at(250.0).typ(0, "a").end(400.0);
    assert_close(stats::start_to_first_keypress_ms(&log), 250.0);
    assert_close(stats::last_keypress_to_end_ms(&log), 150.0);
}

#[test]
fn custom_duration_is_not_rounded() {
    let log = LogBuilder::new(Mode::Custom, false, &["a"]).typ(0, "a").end(1234.567);
    assert_close(stats::test_duration_ms(&log), 1234.567);
    let words = LogBuilder::new(Mode::Words, false, &["a"]).typ(0, "a").end(1234.567);
    assert_close(stats::test_duration_ms(&words), 1230.0);
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test stats`
Expected: échec de compilation (`unresolved import fasttype_core::stats`).

- [ ] **Step 4 : implémenter `stats.rs` (partie 1)**

```rust
//! Statistiques calculées depuis le journal, fonction par fonction comme
//! `frontend/src/ts/test/events/stats.ts`.

use crate::chars::{CharCounts, count_words};
use crate::event::{EventKind, EventLog, active_word_index};
use crate::numbers::{js_round, round2};
use crate::spec::Mode;

fn last_end_ms(log: &EventLog) -> Option<f64> {
    log.events.iter().rev().find(|e| e.kind == EventKind::TimerEnd).map(|e| e.ms)
}

fn trims_trailing_idle(log: &EventLog) -> bool {
    log.context.mode == Mode::Zen || log.context.bailed_out
}

/// `getRawLastKeypressToEndMs`.
fn raw_last_keypress_to_end_ms(log: &EventLog) -> f64 {
    let last_key = log.events.iter().rev().find(|e| matches!(e.kind, EventKind::KeyDown { .. })).map(|e| e.ms);
    match (last_key, last_end_ms(log)) {
        (Some(k), Some(end)) => round2(end - k).max(0.0),
        _ => 0.0,
    }
}

/// `getLastKeypressToEndMs` : toujours 0 en zen.
pub fn last_keypress_to_end_ms(log: &EventLog) -> f64 {
    if log.context.mode == Mode::Zen { 0.0 } else { raw_last_keypress_to_end_ms(log) }
}

/// `getStartToFirstKeypressMs`.
pub fn start_to_first_keypress_ms(log: &EventLog) -> f64 {
    if log.context.mode == Mode::Zen {
        return 0.0;
    }
    let first_key = log.events.iter().find(|e| matches!(e.kind, EventKind::KeyDown { .. })).map(|e| e.ms);
    let start = log.events.iter().find(|e| e.kind == EventKind::TimerStart).map(|e| e.ms);
    match (first_key, start) {
        (Some(k), Some(s)) => round2(k - s).max(0.0),
        _ => 0.0,
    }
}

/// `getTimerBoundaries` : grille idéale d'une seconde, plus une borne finale
/// fractionnaire (≥ 0,5 s) pour les tests non chronométrés.
pub fn timer_boundaries(log: &EventLog) -> Vec<f64> {
    let Some(mut end) = last_end_ms(log) else {
        return Vec::new();
    };
    let mut ticks = (end / 1000.0).floor() as u32;
    if trims_trailing_idle(log) {
        let idle = raw_last_keypress_to_end_ms(log);
        if idle < 7000.0 {
            end -= idle;
            ticks = ticks.min((end / 1000.0).floor() as u32);
        }
    }
    let mut boundaries: Vec<f64> = (1..=ticks).map(|i| f64::from(i) * 1000.0).collect();
    if !log.context.timed && js_round(round2(end / 1000.0) % 1.0) >= 0.5 {
        boundaries.push(end);
    }
    boundaries
}

/// `getTestDurationMs` : arrondi au centième de seconde sauf en custom.
pub fn test_duration_ms(log: &EventLog) -> f64 {
    let Some(mut end) = last_end_ms(log) else {
        return 0.0;
    };
    if trims_trailing_idle(log) {
        let idle = raw_last_keypress_to_end_ms(log);
        if idle < 7000.0 {
            end -= idle;
        }
    }
    if log.context.mode != Mode::Custom {
        end = round2(end / 1000.0) * 1000.0;
    }
    end
}

/// `getChars` : comptage de tous les mots jusqu'au mot actif.
pub fn chars(log: &EventLog, count_partial_last_word: bool) -> CharCounts {
    let inputs = log.word_inputs(None);
    let active = active_word_index(&inputs);
    let partial = log.context.timed || log.context.bailed_out || count_partial_last_word;
    count_words(
        inputs.iter().map(|(&i, input)| (input.as_str(), log.target(i).unwrap_or(input.as_str()), i == active)),
        partial,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accuracy {
    pub correct: u32,
    pub incorrect: u32,
    /// 0 sans aucune frappe (comme Monkeytype pour le résultat).
    pub percentage: f64,
}

/// `getAccuracy` : chaque insertion compte, même corrigée ensuite.
pub fn accuracy(log: &EventLog, until_ms: Option<f64>) -> Accuracy {
    let (mut correct, mut incorrect) = (0u32, 0u32);
    for e in &log.events {
        if until_ms.is_some_and(|u| e.ms > u) {
            break;
        }
        if let EventKind::Insert { correct: ok, .. } = e.kind {
            if ok { correct += 1 } else { incorrect += 1 }
        }
    }
    let total = correct + incorrect;
    let percentage = if total == 0 { 0.0 } else { f64::from(correct) / f64::from(total) * 100.0 };
    Accuracy { correct, incorrect, percentage }
}
```

- [ ] **Step 5 : lancer les tests**

Run: `cargo test -p fasttype-core --test stats && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 10 tests PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): bornes du timer, durée, caractères et précision depuis le journal"
```

---

### Task 6 : séries par seconde, AFK et consistency

**Files:**
- Modify: `crates/fasttype-core/src/stats.rs`
- Test: `crates/fasttype-core/tests/stats_history.rs`

**Interfaces:**
- Produces, dans `stats` :
  - `keypresses_per_second(log) -> Vec<u32>` ;
  - `burst_history(log) -> Vec<f64>` ;
  - `error_count_history(log) -> Vec<u32>` ;
  - `wpm_history(log) -> Vec<f64>` ;
  - `afk_duration(log) -> u32` ;
  - `afk_detected(log) -> bool`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/stats_history.rs` :
```rust
mod common;

use common::{LogBuilder, steady_time_test};
use fasttype_core::numbers::consistency;
use fasttype_core::spec::Mode;
use fasttype_core::stats;

fn two_second_test() -> fasttype_core::event::EventLog {
    LogBuilder::new(Mode::Time, true, &["ab ", "cd ", "ef "])
        .at(100.0).typ(0, "ab ").tick(1)
        .at(1100.0).typ(1, "cd").tick(2)
        .end(2000.0)
}

#[test]
fn wpm_history_is_cumulative_with_partial_last_word() {
    // 1 s : "ab " → 3 car. → 36 wpm ; 2 s : 3 + "cd" (crédit partiel) = 5 → 30 wpm
    assert_eq!(stats::wpm_history(&two_second_test()), vec![36.0, 30.0]);
}

#[test]
fn burst_history_is_per_interval() {
    assert_eq!(stats::burst_history(&two_second_test()), vec![36.0, 24.0]);
    assert_eq!(stats::keypresses_per_second(&two_second_test()), vec![3, 2]);
}

#[test]
fn error_history_counts_wrong_inserts() {
    let log = LogBuilder::new(Mode::Time, true, &["ab "]).at(100.0).typ(0, "x").tick(1).tick(2).end(2000.0);
    assert_eq!(stats::error_count_history(&log), vec![1, 0]);
}

#[test]
fn afk_counts_empty_seconds() {
    let log = LogBuilder::new(Mode::Time, true, &["ab "]).at(100.0).typ(0, "ab").tick(1).tick(2).tick(3).end(3000.0);
    assert_eq!(stats::afk_duration(&log), 2);
}

#[test]
fn afk_detected_on_five_idle_seconds() {
    assert!(!stats::afk_detected(&steady_time_test(15, 15, "abcd ", "abcd ", 5)));
    assert!(stats::afk_detected(&steady_time_test(15, 10, "abcd ", "abcd ", 5)));
}

#[test]
fn afk_never_detected_after_bail_out() {
    let mut log = steady_time_test(15, 10, "abcd ", "abcd ", 5);
    log.context.bailed_out = true;
    assert!(!stats::afk_detected(&log));
}

#[test]
fn steady_typing_is_fully_consistent() {
    let log = steady_time_test(15, 15, "abcd ", "abcd ", 5);
    let burst = stats::burst_history(&log);
    assert_eq!(burst.len(), 15);
    assert!(burst.iter().all(|&b| b == 60.0));
    assert_eq!(consistency(&burst), 100.0);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test stats_history`
Expected: échec de compilation (`cannot find function wpm_history`).

- [ ] **Step 3 : ajouter à la fin de `stats.rs`**

```rust
use crate::event::apply_event;
use crate::numbers::calculate_wpm;
use std::collections::BTreeMap;

/// `countPerInterval` : nombre d'événements satisfaisant `pred` dans chaque
/// intervalle `]borne précédente, borne]` (le premier inclut 0 ms).
fn count_per_interval(log: &EventLog, pred: impl Fn(&EventKind) -> bool) -> (Vec<u32>, Vec<f64>) {
    let boundaries = timer_boundaries(log);
    let mut counts = Vec::with_capacity(boundaries.len());
    let mut idx = 0;
    for &b in &boundaries {
        let mut n = 0;
        while let Some(e) = log.events.get(idx) {
            if e.ms > b {
                break;
            }
            if pred(&e.kind) {
                n += 1;
            }
            idx += 1;
        }
        counts.push(n);
    }
    (counts, boundaries)
}

fn is_insert(kind: &EventKind) -> bool {
    matches!(kind, EventKind::Insert { .. })
}

/// `getKeypressesPerSecond` : insertions par seconde.
pub fn keypresses_per_second(log: &EventLog) -> Vec<u32> {
    count_per_interval(log, is_insert).0
}

/// `getBurstHistory` : « raw » de chaque intervalle (série burst du graphique).
pub fn burst_history(log: &EventLog) -> Vec<f64> {
    let (counts, boundaries) = count_per_interval(log, is_insert);
    let mut prev = 0.0;
    counts
        .iter()
        .zip(&boundaries)
        .map(|(&n, &b)| {
            let seconds = (b - prev) / 1000.0;
            prev = b;
            js_round(calculate_wpm(f64::from(n), seconds))
        })
        .collect()
}

/// `getErrorCountHistory` : insertions incorrectes par intervalle.
pub fn error_count_history(log: &EventLog) -> Vec<u32> {
    count_per_interval(log, |k| matches!(k, EventKind::Insert { correct: false, .. })).0
}

/// `getWpmHistory` : wpm cumulé à chaque borne, avec crédit partiel du mot actif.
pub fn wpm_history(log: &EventLog) -> Vec<f64> {
    let boundaries = timer_boundaries(log);
    let mut inputs: BTreeMap<u32, String> = BTreeMap::new();
    let mut idx = 0;
    let mut out = Vec::with_capacity(boundaries.len());
    for &b in &boundaries {
        while let Some(e) = log.events.get(idx) {
            if e.ms > b {
                break;
            }
            apply_event(&mut inputs, &e.kind);
            idx += 1;
        }
        let active = active_word_index(&inputs);
        let c = count_words(
            inputs.iter().map(|(&i, s)| (s.as_str(), log.target(i).unwrap_or(s.as_str()), i == active)),
            true,
        );
        out.push(js_round(calculate_wpm(f64::from(c.correct_word), b / 1000.0)));
    }
    out
}

/// `getAfkDuration` : secondes sans aucun keydown ni événement de saisie.
pub fn afk_duration(log: &EventLog) -> u32 {
    let (counts, _) = count_per_interval(log, |k| {
        matches!(k, EventKind::KeyDown { .. }) || k.word_index().is_some()
    });
    counts.iter().filter(|&&c| c == 0).count() as u32
}

/// AFK de `finish` : aucune insertion pendant les 5 dernières secondes.
/// Jamais en bail out. (Comme `[].every(...)`, une liste vide vaut AFK.)
pub fn afk_detected(log: &EventLog) -> bool {
    if log.context.bailed_out {
        return false;
    }
    keypresses_per_second(log).iter().rev().take(5).all(|&c| c == 0)
}
```

Regrouper les nouveaux `use` avec ceux du début du fichier, puis lancer `cargo fmt`.

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: tous les tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): séries par seconde du graphique, AFK et consistency"
```

---

### Task 7 : ponctuation et nombres

**Files:**
- Create: `crates/fasttype-core/src/punctuation.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod punctuation;`)
- Test: `crates/fasttype-core/tests/punctuation.rs`

**Interfaces:**
- Consumes : `RandomSource`.
- Produces :
  - `punctuation::Punctuator::{new(), punctuate(&mut self, previous: Option<&str>, word: &str, index: usize, max_index: usize, language: &str, rng: &mut dyn RandomSource) -> String}` ;
  - `punctuation::get_numbers(max_len: u32, rng) -> String` ;
  - `punctuation::localize_digits(digits: &str, language: &str) -> String`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/punctuation.rs` :
```rust
use fasttype_core::punctuation::{Punctuator, get_numbers, localize_digits};
use fasttype_core::rng::Scripted;

fn run(prev: Option<&str>, word: &str, index: usize, max: usize, lang: &str, values: &[f64]) -> (String, usize) {
    let mut rng = Scripted::new(values);
    let out = Punctuator::new().punctuate(prev, word, index, max, lang, &mut rng);
    (out, rng.consumed())
}

#[test]
fn first_word_is_capitalized_without_randomness() {
    assert_eq!(run(None, "hello", 0, 100, "english", &[]), ("Hello".into(), 0));
}

#[test]
fn word_after_sentence_end_is_capitalized() {
    assert_eq!(run(Some("end."), "next", 4, 100, "english_1k", &[]), ("Next".into(), 0));
}

#[test]
fn last_word_always_ends_sentence() {
    // random() du test 10 % (0.5 → non), puis index = max − 1 → fin de phrase ; 0.3 ≤ 0.8 → "."
    assert_eq!(run(Some("world"), "word", 99, 100, "english", &[0.5, 0.3]), ("word.".into(), 2));
}

#[test]
fn french_question_mark_is_its_own_word() {
    assert_eq!(run(Some("chat"), "mot", 5, 100, "french", &[0.05, 0.85]), ("?".into(), 2));
}

#[test]
fn comma_after_seven_failed_draws() {
    let values = [0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.1];
    assert_eq!(run(Some("the"), "word", 5, 100, "english", &values), ("word,".into(), 8));
}

#[test]
fn english_contraction_keeps_case() {
    // 9 tirages ratés (dont celui du code), 0.1 < 0.5 → contraction, 0.6 → 2e choix "it'll"
    let mut values = vec![0.5; 9];
    values.extend([0.1, 0.6]);
    assert_eq!(run(Some("the"), "it", 5, 100, "english", &values).0, "it'll");
    // juste après un point, le mot est capitalisé avant tout tirage
    assert_eq!(run(Some("end."), "it", 5, 100, "english", &[]).0, "It");
}

#[test]
fn spanish_inverted_question_closes_at_sentence_end() {
    let mut p = Punctuator::new();
    let mut rng = Scripted::new(&[0.95]);
    assert_eq!(p.punctuate(None, "hola", 0, 100, "spanish", &mut rng), "¿Hola");
    let mut rng = Scripted::new(&[0.5]);
    assert_eq!(p.punctuate(Some("que"), "tal", 99, 100, "spanish", &mut rng), "tal?");
}

#[test]
fn code_is_never_capitalized() {
    assert_eq!(run(None, "word", 0, 100, "code_python", &[0.9]).0, "word");
}

#[test]
fn newline_moves_to_end() {
    assert_eq!(run(None, "a\nb", 0, 100, "code_python", &[0.9]).0, "ab\n");
}

#[test]
fn numbers_follow_get_numbers() {
    let mut rng = Scripted::new(&[0.0, 0.0]);
    assert_eq!(get_numbers(4, &mut rng), "1");
    let mut rng = Scripted::new(&[0.99, 0.5, 0.0, 0.99, 0.5]);
    assert_eq!(get_numbers(4, &mut rng), "5095");
}

#[test]
fn digits_are_localized() {
    assert_eq!(localize_digits("123", "hindi"), "१२३");
    assert_eq!(localize_digits("123", "nepali_1k"), "१२३");
    assert_eq!(localize_digits("123", "bangla"), "১২৩");
    assert_eq!(localize_digits("123", "kurdish"), "١٢٣");
    assert_eq!(localize_digits("123", "english"), "123");
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test punctuation`
Expected: échec de compilation (`unresolved import fasttype_core::punctuation`).

- [ ] **Step 3 : implémenter `punctuation.rs`**

```rust
//! `punctuateWord` (words-generator.ts), `english-punctuation.ts`,
//! `getNumbers` (utils/generate.ts) et conversions de chiffres (utils/misc.ts).
//! L'ordre des tirages aléatoires est celui de Monkeytype : chaque branche
//! ne tire que si les précédentes ont échoué.

use crate::rng::RandomSource;

const CONTRACTIONS: &[(&str, &[&str])] = &[
    ("are", &["aren't"]),
    ("can", &["can't"]),
    ("could", &["couldn't"]),
    ("did", &["didn't"]),
    ("does", &["doesn't"]),
    ("do", &["don't"]),
    ("had", &["hadn't"]),
    ("has", &["hasn't"]),
    ("have", &["haven't"]),
    ("is", &["isn't"]),
    ("it", &["it's", "it'll"]),
    ("i", &["i'm", "i'll", "i've", "i'd"]),
    ("you", &["you'll", "you're", "you've", "you'd"]),
    ("that", &["that's", "that'll", "that'd"]),
    ("must", &["mustn't", "must've"]),
    ("there", &["there's", "there'll", "there'd"]),
    ("he", &["he's", "he'll", "he'd"]),
    ("she", &["she's", "she'll", "she'd"]),
    ("we", &["we're", "we'll", "we'd"]),
    ("they", &["they're", "they'll", "they'd"]),
    ("should", &["shouldn't", "should've"]),
    ("was", &["wasn't"]),
    ("were", &["weren't"]),
    ("will", &["won't"]),
    ("would", &["wouldn't", "would've"]),
    ("going", &["goin'"]),
];

const SPECIALS: &[&str] = &["{", "}", "[", "]", "(", ")", ";", "=", "+", "%", "/"];
const SPECIALS_C: &[&str] = &[
    "{", "}", "[", "]", "(", ")", ";", "=", "+", "%", "/", "/*", "*/", "//", "!=", "==", "<=", ">=", "||", "&&",
    "<<", ">>", "%=", "&=", "*=", "++", "+=", "--", "-=", "/=", "^=", "|=",
];

fn pick<'a>(items: &[&'a str], rng: &mut dyn RandomSource) -> &'a str {
    items[rng.below(items.len())]
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// `\W` de JavaScript (sans drapeau `u`) : tout sauf `[A-Za-z0-9_]`.
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Découpe `word` en (préfixe non-mot, cœur, suffixe non-mot).
fn split_core(word: &str) -> (&str, &str, &str) {
    let start = word.find(is_word_char).unwrap_or(word.len());
    let end = word.rfind(is_word_char).map(|i| i + 1).unwrap_or(start).max(start);
    (&word[..start], &word[start..end], &word[end..])
}

fn contraction_for(word: &str) -> Option<&'static [&'static str]> {
    let (_, core, _) = split_core(word);
    CONTRACTIONS.iter().find(|(base, _)| core.eq_ignore_ascii_case(base)).map(|(_, r)| *r)
}

fn apply_contraction(word: &str, replacements: &[&str], rng: &mut dyn RandomSource) -> String {
    let (prefix, core, suffix) = split_core(word);
    let replacement = pick(replacements, rng);
    let starts_upper = core.chars().next().is_some_and(char::is_uppercase);
    let replaced = if !starts_upper {
        replacement.to_string()
    } else if core != "I" && core == core.to_uppercase() {
        replacement.to_uppercase()
    } else {
        capitalize_first(replacement)
    };
    format!("{prefix}{replaced}{suffix}")
}

/// Ajoute la ponctuation à un mot ; garde l'état « ¿ / ¡ » de l'espagnol.
#[derive(Debug, Default)]
pub struct Punctuator {
    spanish_closing: Option<char>,
}

impl Punctuator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn punctuate(
        &mut self,
        previous: Option<&str>,
        word: &str,
        index: usize,
        max_index: usize,
        language: &str,
        rng: &mut dyn RandomSource,
    ) -> String {
        let lang = language.split('_').next().unwrap_or(language);
        let last = previous.and_then(|p| p.chars().last());
        let lc = |c: char| last == Some(c);
        let (i, max) = (index as i64, max_index as i64);
        let mut w = word.to_string();

        if lang != "code" && lang != "georgian" && (index == 0 || matches!(last, Some('?' | '!' | '.' | '؟'))) {
            w = capitalize_first(&w);
            if lang == "turkish" {
                w = w.replace('I', "İ");
            }
            if lang == "spanish" {
                let r = rng.next_f64();
                if r > 0.9 {
                    w = format!("¿{w}");
                    self.spanish_closing = Some('?');
                } else if r > 0.8 {
                    w = format!("¡{w}");
                    self.spanish_closing = Some('!');
                }
            }
        } else if (rng.next_f64() < 0.1 && !lc('.') && !lc(',') && i != max - 2) || i == max - 1 {
            if lang == "spanish" {
                if let Some(c) = self.spanish_closing.take() {
                    w.push(c);
                }
            } else {
                let r = rng.next_f64();
                if r <= 0.8 {
                    w.push_str(match lang {
                        "nepali" | "bangla" | "hindi" => "।",
                        "japanese" | "chinese" => "。",
                        _ => ".",
                    });
                } else if r < 0.9 {
                    match lang {
                        "french" => w = "?".into(),
                        "arabic" | "persian" | "urdu" | "kurdish" => w.push('؟'),
                        "greek" => w.push(';'),
                        "japanese" | "chinese" => w.push('？'),
                        _ => w.push('?'),
                    }
                } else {
                    match lang {
                        "french" => w = "!".into(),
                        "japanese" | "chinese" => w.push('！'),
                        _ => w.push('!'),
                    }
                }
            }
        } else if rng.next_f64() < 0.01 && !lc(',') && !lc('.') && lang != "russian" {
            w = format!("\"{w}\"");
        } else if rng.next_f64() < 0.011
            && !lc(',')
            && !lc('.')
            && !matches!(lang, "russian" | "ukrainian" | "slovak")
        {
            w = format!("'{w}'");
        } else if rng.next_f64() < 0.012 && !lc(',') && !lc('.') {
            if lang == "code" {
                let mut brackets = vec!["()", "{}", "[]", "<>"];
                if language.starts_with("code_javascript") {
                    brackets.push("``");
                }
                let b = pick(&brackets, rng);
                let mut cs = b.chars();
                let (open, close) = (cs.next().unwrap_or('('), cs.next().unwrap_or(')'));
                w = format!("{open}{w}{close}");
            } else if matches!(lang, "japanese" | "chinese") {
                w = format!("（{w}）");
            } else {
                w = format!("({w})");
            }
        } else if rng.next_f64() < 0.013 && ![',', '.', ';', '؛', ':', '；', '：'].into_iter().any(lc) {
            match lang {
                "french" => w = ":".into(),
                "chinese" => w.push('：'),
                _ => w.push(':'),
            }
        } else if rng.next_f64() < 0.014 && !lc(',') && !lc('.') && previous != Some("-") {
            w = "-".into();
        } else if rng.next_f64() < 0.015 && ![',', '.', ';', '؛', '；', '：'].into_iter().any(lc) {
            match lang {
                "french" => w = ";".into(),
                // le point médian grec est tombé en désuétude : Monkeytype met un point
                "greek" => w = ".".into(),
                "arabic" | "kurdish" => w.push('؛'),
                "chinese" => w.push('；'),
                _ => w.push(';'),
            }
        } else if rng.next_f64() < 0.2 && !lc(',') {
            match lang {
                "arabic" | "urdu" | "persian" | "kurdish" => w.push('،'),
                "japanese" => w.push('、'),
                "chinese" => w.push('，'),
                _ => w.push(','),
            }
        } else if rng.next_f64() < 0.25 && lang == "code" {
            let c_like = (language.starts_with("code_c") && !language.starts_with("code_css"))
                || language.starts_with("code_arduino");
            w = if c_like {
                pick(SPECIALS_C, rng).to_string()
            } else if language.starts_with("code_javascript") {
                let mut js = SPECIALS.to_vec();
                js.push("`");
                pick(&js, rng).to_string()
            } else {
                pick(SPECIALS, rng).to_string()
            };
        } else if rng.next_f64() < 0.5 && lang == "english" {
            if let Some(replacements) = contraction_for(&w) {
                w = apply_contraction(&w, replacements, rng);
            }
        }

        if w.contains('\t') {
            w.retain(|c| c != '\t');
            w.push('\t');
        }
        if w.contains('\n') {
            w.retain(|c| c != '\n');
            w.push('\n');
        }
        w
    }
}

/// `getNumbers(len)` : 1 à `max_len` chiffres, le premier non nul.
pub fn get_numbers(max_len: u32, rng: &mut dyn RandomSource) -> String {
    let len = rng.int_in(1, max_len);
    let mut out = String::with_capacity(len as usize);
    for i in 0..len {
        let digit = if i == 0 { rng.int_in(1, 9) } else { rng.int_in(0, 9) };
        out.push(char::from_digit(digit, 10).unwrap_or('0'));
    }
    out
}

/// Chiffres arabes-indiens, devanagari ou bengali selon la langue.
pub fn localize_digits(digits: &str, language: &str) -> String {
    let table: &[char; 10] = if language.starts_with("kurdish") {
        &['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩']
    } else if language.starts_with("nepali") || language.starts_with("hindi") {
        &['०', '१', '२', '३', '४', '५', '६', '७', '८', '९']
    } else if language.starts_with("bangla") {
        &['০', '১', '২', '৩', '৪', '৫', '৬', '৭', '৮', '৯']
    } else {
        return digits.to_string();
    };
    digits.chars().map(|c| c.to_digit(10).map_or(c, |d| table[d as usize])).collect()
}
```

Note pour l'implémenteur : dans la branche anglaise, Monkeytype tire `random() < 0.5` **avant** de vérifier la langue et le mot. L'ordre `rng.next_f64() < 0.5 && lang == "english"` le respecte, et `contraction_for` ne tire rien.

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core --test punctuation && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 11 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): ponctuation et nombres identiques à punctuateWord"
```

---

### Task 8 : sources de mots et citations

**Files:**
- Create: `crates/fasttype-core/src/sources.rs`, `crates/fasttype-core/src/quote.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod quote;` et `pub mod sources;`)
- Test: `crates/fasttype-core/tests/sources.rs`

**Interfaces:**
- Consumes : `RandomSource`, `shuffle`, `CustomLimit`.
- Produces, dans `sources` :
  - `trait WordSource: Send { fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String>; fn all_generated(&self, generated: usize) -> bool; fn initial_limit(&self) -> usize; }` ;
  - `RandomWords::new(words: Arc<Vec<String>>, language: &str, punctuation: bool, numbers: bool, max_words: Option<u32>)` ;
  - `SequenceWords::new(words: Vec<String>)` ;
  - `CustomMode { Repeat, Shuffle, Random }` et `CustomWords::new(text: &str, mode: CustomMode, limit: CustomLimit, pipe: bool)`.
- Produces, dans `quote` :
  - `QuoteLength { All, Short, Medium, Long, Thicc }` ;
  - `Quote { id: u32, text: String, source: String, length: u32 }` ;
  - `QuoteFile { language: String, groups: Vec<[u32; 2]>, quotes: Vec<Quote> }`, avec `from_json(&[u8]) -> Result<Self, serde_json::Error>`, `group_of(&Quote) -> Option<u8>`, `pick(QuoteLength, rng) -> Option<&Quote>` et `by_id(u32) -> Option<&Quote>` ;
  - `normalize_quote_text(&str) -> String` et `quote_words(&str) -> Vec<String>`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/sources.rs` :
```rust
use fasttype_core::quote::{QuoteFile, QuoteLength, normalize_quote_text, quote_words};
use fasttype_core::rng::{Scripted, SplitMix64};
use fasttype_core::sources::{CustomMode, CustomWords, RandomWords, SequenceWords, WordSource};
use fasttype_core::spec::CustomLimit;
use std::sync::Arc;

fn list(words: &[&str]) -> Arc<Vec<String>> {
    Arc::new(words.iter().map(|s| s.to_string()).collect())
}

fn random(words: &[&str], lang: &str, punctuation: bool, numbers: bool) -> RandomWords {
    RandomWords::new(list(words), lang, punctuation, numbers, None)
}

#[test]
fn rejects_same_word_as_previous_two() {
    let mut s = random(&["the", "the", "cat"], "english", false, false);
    let mut rng = Scripted::new(&[0.0, 0.9]);
    assert_eq!(s.next_raw("The.", "", &mut rng).as_deref(), Some("cat"));
    assert_eq!(rng.consumed(), 2);
}

#[test]
fn rejects_capital_i_without_punctuation() {
    let mut s = random(&["I", "you"], "english", false, false);
    assert_eq!(s.next_raw("", "", &mut Scripted::new(&[0.0, 0.9])).as_deref(), Some("you"));
}

#[test]
fn rejects_symbols_and_digits_when_disabled() {
    let mut s = random(&["don't", "x1", "ok"], "english", false, false);
    assert_eq!(s.next_raw("", "", &mut Scripted::new(&[0.0, 0.4, 0.9])).as_deref(), Some("ok"));
}

#[test]
fn keeps_symbols_for_code_languages() {
    let mut s = random(&["a.b"], "code_python", false, false);
    assert_eq!(s.next_raw("", "", &mut Scripted::new(&[0.0])).as_deref(), Some("a.b"));
}

#[test]
fn lowercases_unless_punctuation_or_german() {
    let mut rng = Scripted::new(&[0.0]);
    assert_eq!(random(&["Hello"], "english", false, false).next_raw("", "", &mut rng).as_deref(), Some("hello"));
    assert_eq!(random(&["Hallo"], "german", false, false).next_raw("", "", &mut rng).as_deref(), Some("Hallo"));
    assert_eq!(random(&["Hello"], "english", true, false).next_raw("", "", &mut rng).as_deref(), Some("Hello"));
}

#[test]
fn multi_word_entries_are_queued() {
    let mut s = random(&["ice cream"], "english", false, false);
    let mut rng = Scripted::new(&[0.0]);
    assert_eq!(s.next_raw("", "", &mut rng).as_deref(), Some("ice"));
    assert_eq!(s.next_raw("ice", "", &mut rng).as_deref(), Some("cream"));
}

#[test]
fn empty_list_yields_nothing() {
    let mut s = random(&[], "english", false, false);
    assert_eq!(s.next_raw("", "", &mut SplitMix64::new(1)), None);
}

#[test]
fn words_limit_drives_generation() {
    let s = RandomWords::new(list(&["a"]), "english", false, false, Some(25));
    assert_eq!(s.initial_limit(), 25);
    assert!(!s.all_generated(24));
    assert!(s.all_generated(25));
    let big = RandomWords::new(list(&["a"]), "english", false, false, Some(500));
    assert_eq!(big.initial_limit(), 100);
    let infinite = RandomWords::new(list(&["a"]), "english", false, false, None);
    assert!(!infinite.all_generated(10_000));
}

#[test]
fn sequence_is_finite() {
    let mut s = SequenceWords::new(vec!["a".into(), "b".into()]);
    let mut rng = SplitMix64::new(1);
    assert_eq!(s.initial_limit(), 2);
    assert_eq!(s.next_raw("", "", &mut rng).as_deref(), Some("a"));
    assert_eq!(s.next_raw("", "", &mut rng).as_deref(), Some("b"));
    assert_eq!(s.next_raw("", "", &mut rng), None);
    assert!(s.all_generated(2));
}

#[test]
fn custom_repeat_cycles_until_word_limit() {
    let mut s = CustomWords::new("one two", CustomMode::Repeat, CustomLimit::Word(5), false);
    let mut rng = SplitMix64::new(1);
    let got: Vec<String> = (0..5).filter_map(|_| s.next_raw("", "", &mut rng)).collect();
    assert_eq!(got, ["one", "two", "one", "two", "one"]);
    assert!(s.all_generated(5));
    assert_eq!(s.initial_limit(), 5);
}

#[test]
fn custom_shuffle_uses_each_word_once_per_round() {
    let mut s = CustomWords::new("a b c d e", CustomMode::Shuffle, CustomLimit::Word(0), false);
    let mut rng = SplitMix64::new(9);
    let mut round: Vec<String> = (0..5).filter_map(|_| s.next_raw("", "", &mut rng)).collect();
    round.sort();
    assert_eq!(round, ["a", "b", "c", "d", "e"]);
    assert!(!s.all_generated(1_000));
}

#[test]
fn custom_sections_with_pipe() {
    let mut s = CustomWords::new("hello world|good  bye", CustomMode::Repeat, CustomLimit::Section(2), true);
    let mut rng = SplitMix64::new(1);
    let got: Vec<String> = std::iter::from_fn(|| s.next_raw("", "", &mut rng)).take(4).collect();
    assert_eq!(got, ["hello", "world", "good", "bye"]);
    assert!(s.all_generated(4));
}

#[test]
fn quote_text_is_normalized() {
    assert_eq!(normalize_quote_text("  Hello  world…\r\n  next  "), "Hello world...\n next");
    assert_eq!(quote_words("Hello  world…\n next"), ["Hello", "world...\n", "next"]);
}

#[test]
fn quotes_are_grouped_and_picked() {
    let json = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"short one","source":"a","length":9,"id":1,"approvedBy":"x"},
                  {"text":"medium","source":"b","length":150,"id":2}]}"#;
    let file = QuoteFile::from_json(json).unwrap();
    assert_eq!(file.group_of(&file.quotes[1]), Some(1));
    let mut rng = Scripted::new(&[0.0]);
    assert_eq!(file.pick(QuoteLength::Medium, &mut rng).map(|q| q.id), Some(2));
    assert_eq!(file.pick(QuoteLength::Thicc, &mut rng).map(|q| q.id), None);
    assert_eq!(file.by_id(1).map(|q| q.source.as_str()), Some("a"));
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test sources`
Expected: échec de compilation (`unresolved import fasttype_core::quote`).

- [ ] **Step 3 : implémenter `quote.rs`**

```rust
//! Fichiers de citations Monkeytype (`frontend/static/quotes/<langue>.json`)
//! et normalisation du texte (`words-generator.ts`).

use crate::rng::RandomSource;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuoteLength {
    All,
    Short,
    Medium,
    Long,
    Thicc,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Quote {
    pub id: u32,
    pub text: String,
    pub source: String,
    pub length: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct QuoteFile {
    pub language: String,
    /// Bornes incluses de chaque groupe : short, medium, long, thicc.
    pub groups: Vec<[u32; 2]>,
    pub quotes: Vec<Quote>,
}

impl QuoteFile {
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    /// Groupe tel que `lower ≤ length ≤ upper`.
    pub fn group_of(&self, quote: &Quote) -> Option<u8> {
        self.groups
            .iter()
            .position(|[lo, hi]| *lo <= quote.length && quote.length <= *hi)
            .map(|i| i as u8)
    }

    pub fn pick(&self, length: QuoteLength, rng: &mut dyn RandomSource) -> Option<&Quote> {
        let wanted = match length {
            QuoteLength::All => None,
            QuoteLength::Short => Some(0),
            QuoteLength::Medium => Some(1),
            QuoteLength::Long => Some(2),
            QuoteLength::Thicc => Some(3),
        };
        let candidates: Vec<&Quote> =
            self.quotes.iter().filter(|q| wanted.is_none() || self.group_of(q) == wanted).collect();
        if candidates.is_empty() {
            return None;
        }
        Some(candidates[rng.below(candidates.len())])
    }

    pub fn by_id(&self, id: u32) -> Option<&Quote> {
        self.quotes.iter().find(|q| q.id == id)
    }
}

/// Espaces multiples réduits, chaque saut de ligne (avec ses espaces autour)
/// remplacé par `"\n "`, `…` remplacé par `...`, puis `trim`.
pub fn normalize_quote_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_space = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' => pending_space = true,
            '\r' | '\n' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                pending_space = false;
                out.push_str("\n ");
                while chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            _ => {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                out.push(c);
            }
        }
    }
    out.replace('…', "...").trim().to_string()
}

/// Mots d'une citation (`textSplit`), séparés par des espaces.
pub fn quote_words(text: &str) -> Vec<String> {
    normalize_quote_text(text).split(' ').filter(|w| !w.is_empty()).map(String::from).collect()
}
```

- [ ] **Step 4 : implémenter `sources.rs`**

```rust
//! Sources de mots bruts (avant ponctuation, nombres et séparateur), sur le
//! modèle de `getNextWord` (words-generator.ts) et de `Wordset` (wordset.ts).

use crate::rng::{RandomSource, shuffle};
use crate::spec::CustomLimit;
use std::collections::VecDeque;
use std::sync::Arc;

pub trait WordSource: Send {
    /// Mot brut suivant. `prev` et `prev2` sont les deux derniers mots générés
    /// (avec leur ponctuation). `None` : la source est épuisée.
    fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String>;
    /// `areAllWordsGenerated`, sachant que `generated` mots ont été produits.
    fn all_generated(&self, generated: usize) -> bool;
    /// `getLimit` : nombre de mots générés au départ.
    fn initial_limit(&self) -> usize;
}

fn strip_lower(word: &str, removed: &str) -> String {
    word.chars().filter(|c| !removed.contains(*c)).collect::<String>().to_lowercase()
}

fn first_token(word: &str) -> &str {
    word.split(' ').next().unwrap_or(word)
}

/// Découpe une entrée (mot ou section) en mots, espaces multiples ignorés.
fn split_entry(entry: &str, queue: &mut VecDeque<String>) {
    queue.extend(entry.split(' ').filter(|w| !w.is_empty()).map(String::from));
}

/// Tire un index en rejetant les entrées égales aux deux mots précédents
/// (100 tentatives au plus), ainsi que celles refusées par `extra_reject`.
/// Reproduit une bizarrerie de Monkeytype : seul le premier tirage est
/// comparé en minuscules.
fn pick_avoiding_previous(
    words: &[String],
    prev: &str,
    prev2: &str,
    rng: &mut dyn RandomSource,
    extra_reject: impl Fn(&str) -> bool,
) -> usize {
    let prev_raw = strip_lower(prev, ".?!\":-,");
    let prev2_raw = strip_lower(prev2, ".?!\":-,'");
    let mut idx = rng.below(words.len());
    let mut key = first_token(&words[idx]).to_lowercase();
    let mut tries = 0;
    while tries < 100 && (prev_raw == key || prev2_raw == key || extra_reject(&words[idx])) {
        tries += 1;
        idx = rng.below(words.len());
        key = first_token(&words[idx]).to_string();
    }
    idx
}

/// Mots tirés au hasard dans la liste d'une langue (modes time et words).
pub struct RandomWords {
    words: Arc<Vec<String>>,
    language: String,
    punctuation: bool,
    numbers: bool,
    max_words: Option<u32>,
    queue: VecDeque<String>,
}

impl RandomWords {
    /// `max_words` : `Some(n > 0)` en mode words ; `None` ou `Some(0)` = infini.
    pub fn new(words: Arc<Vec<String>>, language: &str, punctuation: bool, numbers: bool, max_words: Option<u32>) -> Self {
        Self { words, language: language.to_string(), punctuation, numbers, max_words, queue: VecDeque::new() }
    }

    fn rejected(&self, word: &str) -> bool {
        (!self.punctuation && word == "I")
            || (!self.punctuation
                && !self.language.starts_with("code")
                && word.chars().any(|c| "-=_+[]{};'\\:\"|,./<>?".contains(c)))
            || (!self.numbers && word.chars().any(|c| c.is_ascii_digit()))
    }

    /// Hors ponctuation, les majuscules sont abaissées (sauf allemand, code, klingon).
    fn finish(&self, word: String) -> String {
        let keeps_case = ["german", "swiss_german", "code", "klingon"].iter().any(|p| self.language.starts_with(p));
        if !self.punctuation && !keeps_case && word.chars().any(|c| c.is_ascii_uppercase()) {
            word.to_lowercase()
        } else {
            word
        }
    }
}

impl WordSource for RandomWords {
    fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String> {
        if self.queue.is_empty() {
            if self.words.is_empty() {
                return None;
            }
            let idx = pick_avoiding_previous(&self.words, prev, prev2, rng, |w| self.rejected(w));
            split_entry(&self.words[idx], &mut self.queue);
        }
        let word = self.queue.pop_front()?;
        Some(self.finish(word))
    }

    fn all_generated(&self, generated: usize) -> bool {
        matches!(self.max_words, Some(n) if n > 0 && generated >= n as usize)
    }

    fn initial_limit(&self) -> usize {
        match self.max_words {
            Some(n) if n > 0 => (n as usize).min(100),
            _ => 100,
        }
    }
}

/// Suite finie de mots, dans l'ordre (citations).
pub struct SequenceWords {
    words: Vec<String>,
    pos: usize,
}

impl SequenceWords {
    pub fn new(words: Vec<String>) -> Self {
        Self { words, pos: 0 }
    }
}

impl WordSource for SequenceWords {
    fn next_raw(&mut self, _: &str, _: &str, _: &mut dyn RandomSource) -> Option<String> {
        let word = self.words.get(self.pos)?.clone();
        self.pos += 1;
        Some(word)
    }

    fn all_generated(&self, generated: usize) -> bool {
        generated >= self.words.len()
    }

    fn initial_limit(&self) -> usize {
        self.words.len().min(100)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomMode {
    Repeat,
    Shuffle,
    Random,
}

/// Texte custom : mots (ou sections avec `|`), répétés, mélangés ou tirés au hasard.
pub struct CustomWords {
    items: Vec<String>,
    mode: CustomMode,
    limit: CustomLimit,
    pos: usize,
    bag: Vec<usize>,
    queue: VecDeque<String>,
    sections: u32,
    last_sections: [Option<usize>; 2],
}

impl CustomWords {
    pub fn new(text: &str, mode: CustomMode, limit: CustomLimit, pipe: bool) -> Self {
        let items: Vec<String> = if pipe {
            text.split('|')
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            text.split_whitespace().map(String::from).collect()
        };
        Self { items, mode, limit, pos: 0, bag: Vec::new(), queue: VecDeque::new(), sections: 0, last_sections: [None; 2] }
    }

    fn next_index(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> usize {
        let len = self.items.len();
        match self.mode {
            CustomMode::Repeat => {
                if self.pos >= len {
                    self.pos = 0;
                }
                self.pos += 1;
                self.pos - 1
            }
            CustomMode::Random if len < 4 => rng.below(len),
            CustomMode::Shuffle => {
                if self.bag.is_empty() {
                    self.bag = (0..len).collect();
                    shuffle(&mut self.bag, rng);
                }
                self.bag.pop().unwrap_or(0)
            }
            CustomMode::Random if matches!(self.limit, CustomLimit::Section(_)) => {
                let same_as_recent = |i: usize, last: &[Option<usize>; 2], items: &[String]| {
                    last.iter().flatten().any(|&p| items[p] == items[i])
                };
                let mut idx = rng.below(len);
                let mut tries = 0;
                while tries < 100 && same_as_recent(idx, &self.last_sections, &self.items) {
                    tries += 1;
                    idx = rng.below(len);
                }
                idx
            }
            // Les rejets « I », symboles et chiffres ne s'appliquent pas en custom.
            CustomMode::Random => pick_avoiding_previous(&self.items, prev, prev2, rng, |_| false),
        }
    }
}

impl WordSource for CustomWords {
    fn next_raw(&mut self, prev: &str, prev2: &str, rng: &mut dyn RandomSource) -> Option<String> {
        if self.queue.is_empty() {
            if self.items.is_empty() {
                return None;
            }
            let idx = self.next_index(prev, prev2, rng);
            self.last_sections = [Some(idx), self.last_sections[0]];
            self.sections += 1;
            split_entry(&self.items[idx], &mut self.queue);
        }
        self.queue.pop_front()
    }

    fn all_generated(&self, generated: usize) -> bool {
        match self.limit {
            CustomLimit::Word(n) if n > 0 => generated >= n as usize,
            CustomLimit::Section(n) if n > 0 => self.sections >= n && self.queue.is_empty(),
            _ => false,
        }
    }

    fn initial_limit(&self) -> usize {
        match self.limit {
            CustomLimit::Word(n) | CustomLimit::Section(n) if n > 0 => (n as usize).min(100),
            _ => 100,
        }
    }
}
```

- [ ] **Step 5 : lancer les tests**

Run: `cargo test -p fasttype-core --test sources && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 14 tests PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): sources de mots (langue, citation, texte custom) et citations"
```

---

### Task 9 : générateur de mots

**Files:**
- Create: `crates/fasttype-core/src/generator.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod generator;`)
- Test: `crates/fasttype-core/tests/generator.rs`

**Interfaces:**
- Consumes : `WordSource`, `Punctuator`, `get_numbers`, `localize_digits`, `RandomSource`.
- Produces : `generator::WordGenerator`, avec :
  - `new(source: Box<dyn WordSource>, language: &str, punctuation: bool, numbers: bool)` ;
  - `empty()` ;
  - `next(&mut self, index: usize, bound: usize, rng: &mut dyn RandomSource) -> Option<String>` (le mot renvoyé inclut son séparateur) ;
  - `all_generated(&self) -> bool` ;
  - `initial_limit(&self) -> usize`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/generator.rs` :
```rust
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::{Scripted, SplitMix64};
use fasttype_core::sources::{RandomWords, SequenceWords};
use std::sync::Arc;

fn words(ws: &[&str]) -> Arc<Vec<String>> {
    Arc::new(ws.iter().map(|s| s.to_string()).collect())
}

#[test]
fn appends_space_separator() {
    let mut g = WordGenerator::new(Box::new(RandomWords::new(words(&["hello"]), "english", false, false, None)), "english", false, false);
    assert_eq!(g.next(0, 100, &mut Scripted::new(&[0.0])).as_deref(), Some("hello "));
}

#[test]
fn newline_words_have_no_extra_separator() {
    let seq = SequenceWords::new(vec!["Hi".into(), "there\n".into(), "you".into()]);
    let mut g = WordGenerator::new(Box::new(seq), "english", false, false);
    let mut rng = SplitMix64::new(1);
    let got: Vec<String> = (0..3).filter_map(|i| g.next(i, 3, &mut rng)).collect();
    assert_eq!(got, ["Hi ", "there\n", "you "]);
    assert!(g.all_generated());
}

#[test]
fn numbers_replace_word_one_time_in_ten() {
    let mut g = WordGenerator::new(Box::new(RandomWords::new(words(&["cat"]), "english", false, true, None)), "english", false, true);
    // tirage du mot, 0.05 < 0.1 → nombre, longueur 1, chiffre 1
    assert_eq!(g.next(0, 100, &mut Scripted::new(&[0.0, 0.05, 0.0, 0.0])).as_deref(), Some("1 "));
}

#[test]
fn punctuation_uses_previous_generated_word() {
    let seq = SequenceWords::new(vec!["hello".into(), "world".into()]);
    let mut g = WordGenerator::new(Box::new(seq), "english", true, false);
    // mot 0 : capitalisé sans tirage ; mot 1 = dernier (index 1 = bound − 1) → 0.5 puis 0.3 → "."
    let mut rng = Scripted::new(&[0.5, 0.3]);
    assert_eq!(g.next(0, 2, &mut rng).as_deref(), Some("Hello "));
    assert_eq!(g.next(1, 2, &mut rng).as_deref(), Some("world. "));
}

#[test]
fn swiss_german_replaces_eszett() {
    let mut g = WordGenerator::new(Box::new(SequenceWords::new(vec!["straße".into()])), "swiss_german", false, false);
    assert_eq!(g.next(0, 1, &mut SplitMix64::new(1)).as_deref(), Some("strasse "));
}

#[test]
fn empty_generator_is_done_immediately() {
    let mut g = WordGenerator::empty();
    assert_eq!(g.initial_limit(), 0);
    assert!(g.all_generated());
    assert_eq!(g.next(0, 0, &mut SplitMix64::new(1)), None);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test generator`
Expected: échec de compilation (`unresolved import fasttype_core::generator`).

- [ ] **Step 3 : implémenter `generator.rs`**

```rust
//! Chaîne de génération de `getNextWord` : source → ß suisse → ponctuation
//! → nombres → séparateur (`appendCommitCharacter`).

use crate::punctuation::{Punctuator, get_numbers, localize_digits};
use crate::rng::RandomSource;
use crate::sources::{SequenceWords, WordSource};

pub struct WordGenerator {
    source: Box<dyn WordSource>,
    language: String,
    punctuation: bool,
    numbers: bool,
    punctuator: Punctuator,
    prev: String,
    prev2: String,
    generated: usize,
}

impl WordGenerator {
    pub fn new(source: Box<dyn WordSource>, language: &str, punctuation: bool, numbers: bool) -> Self {
        Self {
            source,
            language: language.to_string(),
            punctuation,
            numbers,
            punctuator: Punctuator::new(),
            prev: String::new(),
            prev2: String::new(),
            generated: 0,
        }
    }

    /// Générateur sans mots (zen, ou liste introuvable).
    pub fn empty() -> Self {
        Self::new(Box::new(SequenceWords::new(Vec::new())), "english", false, false)
    }

    /// Mot suivant avec son séparateur. `index` est sa position dans le test ;
    /// `bound` est le `wordsBound` de Monkeytype (le dernier mot avant la borne
    /// finit toujours une phrase en ponctuation).
    pub fn next(&mut self, index: usize, bound: usize, rng: &mut dyn RandomSource) -> Option<String> {
        let mut word = self.source.next_raw(&self.prev, &self.prev2, rng)?;
        if self.language.starts_with("swiss_german") {
            word = word.replace('ß', "ss");
        }
        if self.punctuation {
            let previous = (!self.prev.is_empty()).then_some(self.prev.as_str());
            word = self.punctuator.punctuate(previous, &word, index, bound, &self.language, rng);
        }
        if self.numbers && rng.next_f64() < 0.1 {
            word = localize_digits(&get_numbers(4, rng), &self.language);
        }
        self.generated += 1;
        self.prev2 = std::mem::replace(&mut self.prev, word.clone());
        if !word.ends_with('\n') {
            word.push(' ');
        }
        Some(word)
    }

    pub fn all_generated(&self) -> bool {
        self.source.all_generated(self.generated)
    }

    pub fn initial_limit(&self) -> usize {
        self.source.initial_limit()
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core --test generator && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 6 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): générateur de mots (ponctuation, nombres, séparateur)"
```

---

### Task 10 : session de frappe, règles de saisie

**Files:**
- Create: `crates/fasttype-core/src/session.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod session;`)
- Test: `crates/fasttype-core/tests/session_input.rs`

**Interfaces:**
- Consumes : `TestSpec`, `WordGenerator`, `RandomSource`, `EventLog`, `count_words`, `calculate_wpm`, `js_round`.
- Produces, dans `session` :
  - `SessionState { Ready, Running, Finished }` ;
  - `EndReason { Completed, TimeUp, ZenFinished, BailedOut }` ;
  - `InputOutcome { Ignored, Inserted { correct: bool }, Committed { correct: bool, burst: f64 }, Finished }` ;
  - `LiveStats { seconds: u32, wpm: f64, raw: f64, acc: f64 }` ;
  - `TestSession`, avec :
    - `new(spec, generator, rng: Box<dyn RandomSource>)` ;
    - `insert(ch: char, now: f64) -> InputOutcome`, `backspace(now) -> bool`, `delete_word(now) -> bool`, `key_down(code: u32, now)`, `key_up(code: u32, now)` ;
    - les accesseurs `state()`, `spec()`, `words() -> &[String]`, `word(i) -> &str`, `inputs() -> &[String]`, `input(i) -> &str`, `active_index() -> usize`, `is_committed(i) -> bool`, `last_burst() -> Option<f64>`, `log() -> &EventLog`, `end_reason() -> Option<EndReason>` et `is_repeated() -> bool`.
  - La tâche 11 complétera `tick`, `next_tick_at`, `live_stats`, `finish_zen`, `bail_out`, `result` et `into_repeat`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/session_input.rs` :
```rust
use fasttype_core::event::EventKind;
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{EndReason, InputOutcome, SessionState, TestSession};
use fasttype_core::sources::SequenceWords;
use fasttype_core::spec::TestSpec;

fn session(words: &[&str]) -> TestSession {
    let seq = SequenceWords::new(words.iter().map(|s| s.to_string()).collect());
    let generator = WordGenerator::new(Box::new(seq), "english", false, false);
    TestSession::new(TestSpec::words(words.len() as u32, "english", false, false), generator, Box::new(SplitMix64::new(1)))
}

fn type_str(s: &mut TestSession, text: &str, mut t: f64) -> f64 {
    for ch in text.chars() {
        s.insert(ch, t);
        t += 100.0;
    }
    t
}

#[test]
fn last_word_has_no_separator() {
    let s = session(&["the", "cat"]);
    assert_eq!(s.words(), ["the ", "cat"]);
    assert_eq!(s.state(), SessionState::Ready);
}

#[test]
fn space_on_empty_word_is_ignored_and_does_not_start() {
    let mut s = session(&["the", "cat"]);
    assert_eq!(s.insert(' ', 0.0), InputOutcome::Ignored);
    assert_eq!(s.state(), SessionState::Ready);
}

#[test]
fn first_insert_starts_test_at_zero() {
    let mut s = session(&["the", "cat"]);
    s.key_down(84, 1000.0);
    assert_eq!(s.insert('t', 1000.0), InputOutcome::Inserted { correct: true });
    assert_eq!(s.state(), SessionState::Running);
    let kinds: Vec<_> = s.log().events.iter().map(|e| (e.ms, e.kind.clone())).collect();
    assert_eq!(kinds[0], (0.0, EventKind::TimerStart));
    assert_eq!(kinds[1], (0.0, EventKind::KeyDown { code: 84 }));
    assert!(matches!(kinds[2], (0.0, EventKind::Insert { ch: 't', .. })));
}

#[test]
fn typing_everything_finishes_on_last_exact_letter() {
    let mut s = session(&["the", "cat"]);
    let t = type_str(&mut s, "the ca", 1000.0);
    assert_eq!(s.insert('t', t), InputOutcome::Finished);
    assert_eq!(s.end_reason(), Some(EndReason::Completed));
    assert_eq!(s.state(), SessionState::Finished);
}

#[test]
fn space_on_last_word_finishes_even_if_wrong() {
    let mut s = session(&["the", "cat"]);
    let t = type_str(&mut s, "the cx", 0.0);
    assert_eq!(s.insert(' ', t), InputOutcome::Finished);
}

#[test]
fn wrong_word_is_committed_and_reported() {
    let mut s = session(&["the", "cat", "sat"]);
    let t = type_str(&mut s, "thx", 0.0);
    match s.insert(' ', t) {
        InputOutcome::Committed { correct, burst } => {
            assert!(!correct);
            assert_eq!(burst, 160.0); // "thx " = 4 car. de 0 à 300 ms → 4/5/(0,3/60)
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(s.active_index(), 1);
    assert!(s.is_committed(0));
    assert_eq!(s.input(0), "thx ");
}

#[test]
fn backspace_returns_to_wrong_previous_word_only() {
    let mut s = session(&["ab", "cd", "ef"]);
    type_str(&mut s, "ax ", 0.0);
    assert!(s.backspace(400.0));
    assert_eq!((s.active_index(), s.input(0)), (0, "ax"));

    let mut s = session(&["ab", "cd", "ef"]);
    type_str(&mut s, "ab ", 0.0);
    assert!(!s.backspace(400.0));
    assert_eq!(s.active_index(), 1);
}

#[test]
fn backspace_at_very_start_is_refused() {
    let mut s = session(&["ab", "cd"]);
    assert!(!s.backspace(0.0));
    s.insert('a', 0.0);
    assert!(s.backspace(100.0));
    assert!(!s.backspace(200.0));
    assert!(!s.backspace(300.0));
    assert_eq!(s.active_index(), 0);
}

#[test]
fn delete_word_clears_current_then_previous_wrong_word() {
    let mut s = session(&["ab", "cd", "ef"]);
    type_str(&mut s, "ax cd", 0.0);
    assert!(s.delete_word(600.0));
    assert_eq!(s.input(1), "");
    assert!(s.delete_word(700.0));
    assert_eq!((s.active_index(), s.input(0)), (0, ""));
}

#[test]
fn input_is_capped_at_target_plus_twenty() {
    let mut s = session(&["ab", "cd"]);
    for i in 0..30 {
        s.insert('z', f64::from(i));
    }
    assert_eq!(s.input(0).chars().count(), 23); // "ab " = 3 car. + 20
}

#[test]
fn newline_rejected_without_newlines_in_text() {
    let mut s = session(&["ab", "cd"]);
    s.insert('a', 0.0);
    assert_eq!(s.insert('\n', 10.0), InputOutcome::Ignored);
}

#[test]
fn newline_commits_quote_line() {
    let mut s = session(&["Hi", "there\n", "you"]);
    let t = type_str(&mut s, "Hi there", 0.0);
    assert!(matches!(s.insert('\n', t), InputOutcome::Committed { correct: true, .. }));
    assert_eq!(s.active_index(), 2);
}

#[test]
fn input_after_finish_is_ignored() {
    let mut s = session(&["a"]);
    assert_eq!(s.insert('a', 0.0), InputOutcome::Finished);
    assert_eq!(s.insert('b', 100.0), InputOutcome::Ignored);
    assert!(!s.backspace(200.0));
}

#[test]
fn empty_word_list_never_panics() {
    let mut s = session(&[]);
    assert!(s.words().is_empty());
    assert_eq!(s.insert('a', 0.0), InputOutcome::Ignored);
    assert!(!s.backspace(0.0));
    assert!(!s.delete_word(0.0));
}

#[test]
fn clock_going_backwards_is_clamped() {
    let mut s = session(&["abc", "d"]);
    s.insert('a', 1000.0);
    s.insert('b', 900.0);
    assert!(s.log().events.iter().all(|e| e.ms >= 0.0));
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test session_input`
Expected: échec de compilation (`unresolved import fasttype_core::session`).

- [ ] **Step 3 : implémenter `session.rs` (saisie)**

```rust
//! Session de frappe : règles de saisie et de fin de Monkeytype
//! (`input/handlers/*`, `input/helpers/fail-or-finish.ts`, `test/test-logic.ts`).
//! Chaque action est horodatée par l'appelant (`now`, en ms d'une horloge monotone).

use crate::event::{EventContext, EventKind, EventLog};
use crate::generator::WordGenerator;
use crate::numbers::{calculate_wpm, js_round};
use crate::rng::RandomSource;
use crate::spec::{Mode, TestSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Ready,
    Running,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Completed,
    TimeUp,
    ZenFinished,
    BailedOut,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputOutcome {
    Ignored,
    Inserted { correct: bool },
    Committed { correct: bool, burst: f64 },
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveStats {
    pub seconds: u32,
    pub wpm: f64,
    pub raw: f64,
    pub acc: f64,
}

/// Mots d'avance maintenus pendant le test (`addWord`).
const WORDS_AHEAD: usize = 100;
/// Capacité réservée du journal : un test de plusieurs minutes sans réallocation.
const LOG_CAPACITY: usize = 16_384;

pub struct TestSession {
    spec: TestSpec,
    generator: WordGenerator,
    rng: Box<dyn RandomSource>,
    words: Vec<String>,
    inputs: Vec<String>,
    first_insert: Vec<Option<f64>>,
    active: usize,
    state: SessionState,
    start_at: f64,
    next_tick: u32,
    log: EventLog,
    pending_keydown: Option<u32>,
    has_newlines: bool,
    correct_inputs: u32,
    incorrect_inputs: u32,
    last_burst: Option<f64>,
    end_reason: Option<EndReason>,
    repeated: bool,
}

impl TestSession {
    pub fn new(spec: TestSpec, generator: WordGenerator, rng: Box<dyn RandomSource>) -> Self {
        let mut s = Self::empty(spec, generator, rng);
        if s.is_zen() {
            s.push_word(String::new());
            return s;
        }
        let limit = s.generator.initial_limit();
        for i in 0..limit {
            match s.generator.next(i, limit, s.rng.as_mut()) {
                Some(w) => s.push_word(w),
                None => break,
            }
        }
        s.strip_last_separator_if_done();
        s
    }

    fn empty(spec: TestSpec, generator: WordGenerator, rng: Box<dyn RandomSource>) -> Self {
        let context = EventContext { mode: spec.mode, timed: spec.is_timed(), bailed_out: false, target_words: Vec::new() };
        Self {
            spec,
            generator,
            rng,
            words: Vec::with_capacity(256),
            inputs: Vec::with_capacity(256),
            first_insert: Vec::with_capacity(256),
            active: 0,
            state: SessionState::Ready,
            start_at: 0.0,
            next_tick: 1,
            log: EventLog::with_capacity(context, LOG_CAPACITY),
            pending_keydown: None,
            has_newlines: false,
            correct_inputs: 0,
            incorrect_inputs: 0,
            last_burst: None,
            end_reason: None,
            repeated: false,
        }
    }

    fn is_zen(&self) -> bool {
        self.spec.mode == Mode::Zen
    }

    fn push_word(&mut self, word: String) {
        self.has_newlines |= word.contains('\n');
        self.inputs.push(String::with_capacity(word.len() + 32));
        self.first_insert.push(None);
        self.words.push(word);
    }

    /// `removeCommitCharacterFromLastWord` une fois tous les mots générés.
    fn strip_last_separator_if_done(&mut self) {
        if self.is_zen() || !self.generator.all_generated() {
            return;
        }
        if let Some(last) = self.words.last_mut()
            && last.ends_with(' ')
        {
            last.pop();
        }
    }

    /// `addWord` : un mot de plus, tant qu'on a moins de 100 mots d'avance.
    fn add_word(&mut self) {
        if self.words.len() > self.active + 1 + WORDS_AHEAD || self.generator.all_generated() {
            return;
        }
        let index = self.words.len();
        if let Some(w) = self.generator.next(index, WORDS_AHEAD, self.rng.as_mut()) {
            self.push_word(w);
        }
        self.strip_last_separator_if_done();
    }

    fn ms(&self, now: f64) -> f64 {
        (now - self.start_at).max(0.0)
    }

    fn start(&mut self, now: f64) {
        self.state = SessionState::Running;
        self.start_at = now;
        self.log.push(0.0, EventKind::TimerStart);
        if let Some(code) = self.pending_keydown.take() {
            self.log.push(0.0, EventKind::KeyDown { code });
        }
    }

    fn is_last_word(&self) -> bool {
        !self.is_zen() && self.generator.all_generated() && self.active + 1 == self.words.len()
    }

    pub fn key_down(&mut self, code: u32, now: f64) {
        match self.state {
            SessionState::Ready => self.pending_keydown = Some(code),
            SessionState::Running => {
                self.tick(now);
                if self.state == SessionState::Running {
                    let ms = self.ms(now);
                    self.log.push(ms, EventKind::KeyDown { code });
                }
            }
            SessionState::Finished => {}
        }
    }

    pub fn key_up(&mut self, code: u32, now: f64) {
        if self.state == SessionState::Running {
            let ms = self.ms(now);
            self.log.push(ms, EventKind::KeyUp { code });
        }
    }

    pub fn insert(&mut self, ch: char, now: f64) -> InputOutcome {
        if self.state == SessionState::Running {
            self.tick(now);
        }
        if self.state == SessionState::Finished || self.words.is_empty() {
            return InputOutcome::Ignored;
        }
        let zen = self.is_zen();
        let commit = ch == ' ' || ch == '\n';
        if ch == '\n' && !zen && !self.has_newlines {
            return InputOutcome::Ignored;
        }
        let a = self.active;
        let input_len = self.inputs[a].chars().count();
        if commit && input_len == 0 {
            return InputOutcome::Ignored;
        }
        if !commit {
            let max = if zen { 30 } else { self.words[a].chars().count() + 20 };
            if input_len >= max {
                return InputOutcome::Ignored;
            }
        }
        if self.state == SessionState::Ready {
            self.start(now);
        }
        let ms = self.ms(now);
        let correct = zen || self.words[a].chars().nth(input_len) == Some(ch);
        if correct {
            self.correct_inputs += 1;
        } else {
            self.incorrect_inputs += 1;
        }
        self.inputs[a].push(ch);
        self.log.push(ms, EventKind::Insert { word_index: a as u32, char_index: input_len as u32, ch, correct });
        if input_len == 0 && self.first_insert[a].is_none() {
            self.first_insert[a] = Some(ms);
        }
        if commit {
            return self.commit(ms);
        }
        if self.is_last_word() && self.inputs[a] == self.words[a] {
            self.finish_at(ms, EndReason::Completed);
            return InputOutcome::Finished;
        }
        InputOutcome::Inserted { correct }
    }

    /// Validation du mot actif (le séparateur vient d'être inséré).
    fn commit(&mut self, ms: f64) -> InputOutcome {
        let a = self.active;
        let zen = self.is_zen();
        let correct = zen || self.inputs[a] == self.words[a];
        // `computeBurst` : longueur saisie (séparateur compris) depuis la 1re lettre
        let len = self.inputs[a].chars().count() as f64;
        let burst = match self.first_insert[a] {
            Some(start) if ms > start => js_round(calculate_wpm(len, (ms - start) / 1000.0)),
            Some(_) => f64::INFINITY,
            None => 0.0,
        };
        self.last_burst = Some(burst);
        if self.is_last_word() {
            self.finish_at(ms, EndReason::Completed);
            return InputOutcome::Finished;
        }
        self.active += 1;
        if zen {
            self.push_word(String::new());
        } else {
            self.add_word();
        }
        if self.active >= self.words.len() {
            self.finish_at(ms, EndReason::Completed);
            return InputOutcome::Finished;
        }
        InputOutcome::Committed { correct, burst }
    }

    /// Mot précédent rouvrable : seulement s'il est faux (pas de freedom mode en v1).
    fn previous_editable_word(&self) -> Option<usize> {
        if self.active == 0 || self.is_zen() {
            return None;
        }
        let prev = self.active - 1;
        (self.inputs[prev] != self.words[prev]).then_some(prev)
    }

    pub fn backspace(&mut self, now: f64) -> bool {
        if self.state == SessionState::Running {
            self.tick(now);
        }
        if self.state != SessionState::Running {
            return false;
        }
        let ms = self.ms(now);
        let a = self.active;
        if self.inputs[a].pop().is_some() {
            self.log.push(ms, EventKind::DeleteChar { word_index: a as u32 });
            return true;
        }
        let Some(prev) = self.previous_editable_word() else {
            return false;
        };
        self.active = prev;
        self.inputs[prev].pop();
        self.log.push(ms, EventKind::DeleteChar { word_index: prev as u32 });
        true
    }

    /// Ctrl/Alt + Backspace.
    pub fn delete_word(&mut self, now: f64) -> bool {
        if self.state == SessionState::Running {
            self.tick(now);
        }
        if self.state != SessionState::Running {
            return false;
        }
        let ms = self.ms(now);
        let a = self.active;
        if !self.inputs[a].is_empty() {
            self.inputs[a].clear();
            self.log.push(ms, EventKind::DeleteWord { word_index: a as u32 });
            return true;
        }
        let Some(prev) = self.previous_editable_word() else {
            return false;
        };
        self.active = prev;
        self.inputs[prev].clear();
        self.log.push(ms, EventKind::DeleteWord { word_index: prev as u32 });
        true
    }

    /// Avance le timer jusqu'à `now`. Complété à la tâche 11.
    pub fn tick(&mut self, _now: f64) -> bool {
        false
    }

    fn finish_at(&mut self, ms: f64, reason: EndReason) {
        self.log.push(ms, EventKind::TimerEnd);
        self.state = SessionState::Finished;
        self.end_reason = Some(reason);
        if reason == EndReason::BailedOut {
            self.log.context.bailed_out = true;
        }
        if !self.is_zen() {
            self.log.context.target_words = self.words.clone();
        }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn spec(&self) -> &TestSpec {
        &self.spec
    }

    pub fn words(&self) -> &[String] {
        &self.words
    }

    pub fn word(&self, i: usize) -> &str {
        self.words.get(i).map_or("", String::as_str)
    }

    pub fn inputs(&self) -> &[String] {
        &self.inputs
    }

    pub fn input(&self, i: usize) -> &str {
        self.inputs.get(i).map_or("", String::as_str)
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Un mot est validé quand le caret l'a dépassé.
    pub fn is_committed(&self, i: usize) -> bool {
        i < self.active
            || (self.state == SessionState::Finished
                && i == self.active
                && self.inputs.get(i).is_some_and(|s| !s.is_empty()))
    }

    pub fn last_burst(&self) -> Option<f64> {
        self.last_burst
    }

    pub fn log(&self) -> &EventLog {
        &self.log
    }

    pub fn end_reason(&self) -> Option<EndReason> {
        self.end_reason
    }

    pub fn is_repeated(&self) -> bool {
        self.repeated
    }
}
```

Note : `if let ... && cond` (let chains) est stable depuis l'édition 2024 ; c'est la forme que demande clippy (`collapsible_if`).

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core --test session_input && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: 15 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): session de frappe, règles de saisie de Monkeytype"
```

---

### Task 11 : timer, fin de test, stats live, zen, bail out et repeat

**Files:**
- Modify: `crates/fasttype-core/src/session.rs`
- Test: `crates/fasttype-core/tests/session_flow.rs`

**Interfaces:**
- Produces, dans `TestSession` :
  - `tick(&mut self, now: f64) -> bool` (remplace le bouchon) ;
  - `next_tick_at(&self) -> Option<f64>` (horloge absolue du prochain tick) ;
  - `live_stats(&self) -> LiveStats` ;
  - `finish_zen(&mut self, now)` et `bail_out(&mut self, now)` ;
  - `into_repeat(self) -> Option<TestSession>`.
- `result()` est ajouté à la tâche 12.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/session_flow.rs` :
```rust
use fasttype_core::event::EventKind;
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{EndReason, InputOutcome, SessionState, TestSession};
use fasttype_core::sources::{RandomWords, SequenceWords};
use fasttype_core::spec::TestSpec;
use std::sync::Arc;

fn time_session(seconds: u32) -> TestSession {
    let words = Arc::new(["the", "cat", "sat", "mat"].iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let generator = WordGenerator::new(Box::new(RandomWords::new(words, "english", false, false, None)), "english", false, false);
    TestSession::new(TestSpec::time(seconds, "english", false, false), generator, Box::new(SplitMix64::new(5)))
}

fn words_session(words: &[&str]) -> TestSession {
    let seq = SequenceWords::new(words.iter().map(|s| s.to_string()).collect());
    let generator = WordGenerator::new(Box::new(seq), "english", false, false);
    TestSession::new(TestSpec::words(words.len() as u32, "english", false, false), generator, Box::new(SplitMix64::new(1)))
}

#[test]
fn time_test_ends_exactly_at_limit() {
    let mut s = time_session(15);
    let first = s.word(0).chars().next().unwrap();
    s.insert(first, 500.0);
    assert_eq!(s.next_tick_at(), Some(1500.0));
    assert!(s.tick(15_499.0));
    assert_eq!(s.state(), SessionState::Running);
    s.tick(15_500.0);
    assert_eq!(s.end_reason(), Some(EndReason::TimeUp));
    let last = s.log().events.last().unwrap();
    assert_eq!((last.ms, &last.kind), (15_000.0, &EventKind::TimerEnd));
    assert_eq!(s.next_tick_at(), None);
}

#[test]
fn input_after_time_limit_is_ignored() {
    let mut s = time_session(15);
    s.insert(s.word(0).chars().next().unwrap(), 0.0);
    assert_eq!(s.insert('x', 16_000.0), InputOutcome::Ignored);
    assert_eq!(s.state(), SessionState::Finished);
}

#[test]
fn infinite_time_never_ends_by_itself() {
    let mut s = time_session(0);
    s.insert(s.word(0).chars().next().unwrap(), 0.0);
    s.tick(10_000_000.0);
    assert_eq!(s.state(), SessionState::Running);
}

#[test]
fn live_stats_after_first_second() {
    let mut s = words_session(&["the", "cat", "sat"]);
    for (i, ch) in "the ".chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
    s.insert('x', 400.0); // erreur
    s.tick(1000.0);
    let live = s.live_stats();
    assert_eq!(live.seconds, 1);
    assert_eq!(live.wpm, 48.0); // 4 car. justes en 1 s
    assert_eq!(live.raw, 60.0); // 5 car. tapés
    assert_eq!(live.acc, 80.0);
}

#[test]
fn live_stats_before_start_are_neutral() {
    let s = words_session(&["the"]);
    let live = s.live_stats();
    assert_eq!((live.seconds, live.wpm, live.acc), (0, 0.0, 100.0));
}

#[test]
fn zen_grows_words_and_finishes_on_demand() {
    let generator = WordGenerator::empty();
    let mut s = TestSession::new(TestSpec::zen("english"), generator, Box::new(SplitMix64::new(1)));
    s.insert('h', 0.0);
    s.insert('i', 100.0);
    assert!(matches!(s.insert(' ', 200.0), InputOutcome::Committed { correct: true, .. }));
    assert_eq!(s.words().len(), 2);
    s.finish_zen(20_000.0);
    assert_eq!(s.end_reason(), Some(EndReason::ZenFinished));
    assert!(s.log().context.target_words.is_empty());
}

#[test]
fn bail_out_marks_log() {
    let mut s = time_session(0);
    s.insert(s.word(0).chars().next().unwrap(), 0.0);
    s.bail_out(5000.0);
    assert_eq!(s.end_reason(), Some(EndReason::BailedOut));
    assert!(s.log().context.bailed_out);
}

#[test]
fn bail_out_before_start_does_nothing() {
    let mut s = time_session(30);
    s.bail_out(100.0);
    assert_eq!(s.state(), SessionState::Ready);
}

#[test]
fn ticks_are_logged_before_later_inputs() {
    let mut s = words_session(&["abc", "d"]);
    s.insert('a', 0.0);
    s.insert('b', 2500.0);
    let ms: Vec<f64> = s.log().events.iter().map(|e| e.ms).collect();
    let mut sorted = ms.clone();
    sorted.sort_by(f64::total_cmp);
    assert_eq!(ms, sorted);
}

#[test]
fn repeat_keeps_words_and_is_flagged() {
    let mut s = words_session(&["ab", "cd"]);
    for (i, ch) in "ab cd".chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
    let words = s.words().to_vec();
    let r = s.into_repeat().unwrap();
    assert_eq!(r.words(), words.as_slice());
    assert!(r.is_repeated());
    assert_eq!(r.state(), SessionState::Ready);
}

#[test]
fn zen_cannot_be_repeated() {
    let s = TestSession::new(TestSpec::zen("english"), WordGenerator::empty(), Box::new(SplitMix64::new(1)));
    assert!(s.into_repeat().is_none());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test session_flow`
Expected: échec de compilation (`no method named next_tick_at`).

- [ ] **Step 3 : implémenter**

Dans `session.rs`, ajouter `use crate::chars::count_words;` aux imports, puis remplacer le bouchon `tick` par :
```rust
    /// Avance le timer jusqu'à `now` : un `TimerStep` par seconde écoulée, et
    /// fin du test à la limite de temps (`timerStep` + `checkIfTimeIsUp`).
    /// Le test se termine à la seconde pile, comme sur une grille idéale.
    pub fn tick(&mut self, now: f64) -> bool {
        let mut ticked = false;
        while self.state == SessionState::Running {
            let due = f64::from(self.next_tick) * 1000.0;
            if self.ms(now) < due {
                break;
            }
            ticked = true;
            self.log.push(due, EventKind::TimerStep { second: self.next_tick });
            if let Some(limit) = self.spec.time_limit
                && limit > 0
                && self.next_tick >= limit
            {
                self.finish_at(due, EndReason::TimeUp);
                break;
            }
            self.next_tick += 1;
        }
        ticked
    }

    /// Instant (horloge de l'appelant) du prochain tick, pour programmer le réveil.
    pub fn next_tick_at(&self) -> Option<f64> {
        (self.state == SessionState::Running).then(|| self.start_at + f64::from(self.next_tick) * 1000.0)
    }

    /// Stats live du dernier tick (`timerStep`) : wpm et raw arrondis avec
    /// crédit partiel du mot actif, précision tronquée (100 sans frappe).
    pub fn live_stats(&self) -> LiveStats {
        let seconds = self.next_tick.saturating_sub(1);
        let acc = match self.correct_inputs + self.incorrect_inputs {
            0 => 100.0,
            total => (f64::from(self.correct_inputs) / f64::from(total) * 100.0).floor(),
        };
        if self.words.is_empty() || seconds == 0 {
            return LiveStats { seconds, wpm: 0.0, raw: 0.0, acc };
        }
        let zen = self.is_zen();
        let last = self.active.min(self.words.len() - 1);
        let c = count_words(
            (0..=last).map(|i| {
                let input = self.inputs[i].as_str();
                (input, if zen { input } else { self.words[i].as_str() }, i == last)
            }),
            true,
        );
        let s = f64::from(seconds);
        LiveStats {
            seconds,
            wpm: js_round(calculate_wpm(f64::from(c.correct_word), s)),
            raw: js_round(calculate_wpm(f64::from(c.all_correct + c.incorrect + c.extra), s)),
            acc,
        }
    }

    /// Shift + Entrée en zen.
    pub fn finish_zen(&mut self, now: f64) {
        if self.is_zen() && self.state == SessionState::Running {
            let ms = self.ms(now);
            self.finish_at(ms, EndReason::ZenFinished);
        }
    }

    /// « Bail out » de la palette : fin immédiate, sans PB.
    pub fn bail_out(&mut self, now: f64) {
        if self.state != SessionState::Running {
            return;
        }
        self.tick(now);
        if self.state == SessionState::Running {
            let ms = self.ms(now);
            self.finish_at(ms, EndReason::BailedOut);
        }
    }

    /// `repeatTest` : mêmes mots, la génération reprend ensuite là où elle
    /// s'était arrêtée. Indisponible en zen.
    pub fn into_repeat(self) -> Option<TestSession> {
        if self.is_zen() {
            return None;
        }
        let mut s = Self::empty(self.spec, self.generator, self.rng);
        for w in self.words {
            s.push_word(w);
        }
        s.repeated = true;
        Some(s)
    }
```

Note : pour `live_stats_after_first_second`, le dernier mot compté est `last = active = 1` avec la saisie `"x"` et la cible `"cat "`. Le crédit partiel ne s'applique pas (`"cat "` ne commence pas par `"x"`). On obtient correct_word = 4 et raw = 4 + 1 incorrect = 5, soit 48 et 60 wpm.

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: tous les tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): timer, fin de test, stats live, zen, bail out et repeat"
```

---

### Task 12 : résultat, validité et clé de PB

**Files:**
- Create: `crates/fasttype-core/src/result.rs`
- Modify: `crates/fasttype-core/src/lib.rs` (ajouter `pub mod result;`), `crates/fasttype-core/src/session.rs` (méthode `result`)
- Test: `crates/fasttype-core/tests/result.rs`

**Interfaces:**
- Consumes : `stats::*`, `numbers::{calculate_wpm, round2, consistency}`, `TestSpec`, `EventLog`.
- Produces, dans `result` :
  - `ChartData { wpm: Vec<f64>, burst: Vec<f64>, err: Vec<u32> }` ;
  - `Invalid { TooShort, Afk, Repeated, Wpm, Raw, Accuracy }` ;
  - `TestResult { timestamp: u64, mode, mode2, language, punctuation, numbers, difficulty, lazy_mode, wpm, raw, acc, consistency, char_stats: [u32; 4], test_duration: f64, afk_duration: u32, afk_detected: bool, bailed_out: bool, quote_id: Option<u32>, quote_length: Option<u8>, chart: ChartData, invalid: Option<Invalid> }`, avec `pb_key() -> PbKey`, `is_saveable() -> bool` et `pb_eligible() -> bool` ;
  - `PbKey { mode, mode2, punctuation, numbers, language, difficulty, lazy_mode }` (`Hash`, `Eq`) ;
  - `build_result(log, spec, repeated: bool, timestamp: u64) -> TestResult` ;
  - `remove_language_size(&str) -> String`.
- Produces, dans `TestSession` : `result(&self, timestamp: u64) -> Option<TestResult>`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/result.rs` :
```rust
mod common;

use common::{LogBuilder, steady_time_test};
use fasttype_core::result::{Invalid, build_result, remove_language_size};
use fasttype_core::spec::{Mode, QuoteMeta, TestSpec};

fn time15() -> TestSpec {
    TestSpec::time(15, "english", false, false)
}

#[test]
fn steady_valid_test() {
    let r = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 5), &time15(), false, 42);
    assert_eq!(r.invalid, None);
    assert_eq!(r.wpm, 60.0);
    assert_eq!(r.raw, 60.0);
    assert_eq!(r.acc, 100.0);
    assert_eq!(r.consistency, 100.0);
    assert_eq!(r.char_stats, [75, 0, 0, 0]);
    assert_eq!(r.test_duration, 15.0);
    assert_eq!(r.chart.wpm.len(), 15);
    assert_eq!(r.timestamp, 42);
    assert!(r.is_saveable());
    assert!(r.pb_eligible());
}

#[test]
fn too_short_for_short_time_modes() {
    let r = build_result(&steady_time_test(10, 10, "abcd ", "abcd ", 5), &TestSpec::time(10, "english", false, false), false, 0);
    assert_eq!(r.invalid, Some(Invalid::TooShort));
}

#[test]
fn too_short_under_one_second() {
    let log = LogBuilder::new(Mode::Words, false, &["ab ", "cd"]).typ(0, "ab ").typ(1, "cd").end(400.0);
    let r = build_result(&log, &TestSpec::words(25, "english", false, false), false, 0);
    assert_eq!(r.invalid, Some(Invalid::TooShort));
}

#[test]
fn afk_detected_invalidates() {
    let r = build_result(&steady_time_test(15, 10, "abcd ", "abcd ", 5), &time15(), false, 0);
    assert!(r.afk_detected);
    assert_eq!(r.invalid, Some(Invalid::Afk));
}

#[test]
fn repeated_invalidates() {
    let r = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 5), &time15(), true, 0);
    assert_eq!(r.invalid, Some(Invalid::Repeated));
}

#[test]
fn inhuman_speed_invalidates() {
    let r = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 30), &time15(), false, 0);
    assert_eq!(r.wpm, 360.0);
    assert_eq!(r.invalid, Some(Invalid::Wpm));
}

#[test]
fn low_accuracy_invalidates() {
    let r = build_result(&steady_time_test(15, 15, "abcd ", "xxcd ", 5), &time15(), false, 0);
    assert_eq!(r.acc, 60.0);
    assert_eq!(r.invalid, Some(Invalid::Accuracy));
}

#[test]
fn pb_key_ignores_speed_but_not_settings() {
    let a = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 5), &time15(), false, 0);
    let b = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 6), &time15(), false, 0);
    assert_eq!(a.pb_key(), b.pb_key());
    let c = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 5), &TestSpec::time(15, "english", true, false), false, 0);
    assert_ne!(a.pb_key(), c.pb_key());
}

#[test]
fn quotes_drop_language_size_and_are_not_pb_eligible() {
    let meta = QuoteMeta { id: 7, group: 1, source: "x".into() };
    let r = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 5), &TestSpec::quote(meta, "english_1k"), false, 0);
    assert_eq!(r.language, "english");
    assert_eq!((r.quote_id, r.quote_length), (Some(7), Some(1)));
    assert!(!r.pb_eligible());
}

#[test]
fn language_size_suffix() {
    assert_eq!(remove_language_size("english_1k"), "english");
    assert_eq!(remove_language_size("french_600k"), "french");
    assert_eq!(remove_language_size("code_javascript"), "code_javascript");
    assert_eq!(remove_language_size("english"), "english");
}

#[test]
fn long_infinite_test_builds_result() {
    let mut spec = TestSpec::time(0, "english", false, false);
    spec.mode2 = "0".into();
    let mut log = steady_time_test(320, 320, "abcd ", "abcd ", 5);
    log.context.bailed_out = true;
    let r = build_result(&log, &spec, false, 0);
    // bail out : le temps mort final (< 7 s) est retiré → fin à 319,83 s → 319 secondes pleines
    assert_eq!(r.chart.wpm.len(), 319);
    assert!(r.wpm > 0.0);
    assert!(!r.pb_eligible()); // bail out
}

#[test]
fn result_serializes() {
    let r = build_result(&steady_time_test(15, 15, "abcd ", "abcd ", 5), &time15(), false, 1);
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"mode\":\"time\""));
    assert_eq!(serde_json::from_str::<fasttype_core::result::TestResult>(&json).unwrap(), r);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test result`
Expected: échec de compilation (`unresolved import fasttype_core::result`).

- [ ] **Step 3 : implémenter `result.rs`**

```rust
//! Résultat d'un test (`buildCompletedEvent`) et règles d'invalidation
//! (`finish`, test-logic.ts).

use crate::event::EventLog;
use crate::numbers::{calculate_wpm, consistency, round2};
use crate::spec::{CustomLimit, Difficulty, Mode, TestSpec};
use crate::stats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChartData {
    /// wpm cumulé à chaque seconde.
    pub wpm: Vec<f64>,
    /// « raw » de chaque seconde.
    pub burst: Vec<f64>,
    /// Erreurs de chaque seconde.
    pub err: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Invalid {
    TooShort,
    Afk,
    Repeated,
    Wpm,
    Raw,
    Accuracy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PbKey {
    pub mode: Mode,
    pub mode2: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub language: String,
    pub difficulty: Difficulty,
    pub lazy_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub timestamp: u64,
    pub mode: Mode,
    pub mode2: String,
    pub language: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub difficulty: Difficulty,
    pub lazy_mode: bool,
    pub wpm: f64,
    pub raw: f64,
    pub acc: f64,
    pub consistency: f64,
    /// `[correct, incorrect, extra, missed]`.
    pub char_stats: [u32; 4],
    /// Secondes.
    pub test_duration: f64,
    pub afk_duration: u32,
    pub afk_detected: bool,
    pub bailed_out: bool,
    pub quote_id: Option<u32>,
    pub quote_length: Option<u8>,
    pub chart: ChartData,
    pub invalid: Option<Invalid>,
}

impl TestResult {
    pub fn pb_key(&self) -> PbKey {
        PbKey {
            mode: self.mode,
            mode2: self.mode2.clone(),
            punctuation: self.punctuation,
            numbers: self.numbers,
            language: self.language.clone(),
            difficulty: self.difficulty,
            lazy_mode: self.lazy_mode,
        }
    }

    /// Un résultat invalide est affiché mais jamais enregistré.
    pub fn is_saveable(&self) -> bool {
        self.invalid.is_none()
    }

    /// `getPbEligibility` (v1) : enregistrable, pas une citation, pas un bail out.
    pub fn pb_eligible(&self) -> bool {
        self.is_saveable() && self.mode != Mode::Quote && !self.bailed_out
    }
}

/// `removeLanguageSize` : retire un suffixe de taille `_<n>k` (`english_1k` → `english`).
pub fn remove_language_size(language: &str) -> String {
    if let Some((base, size)) = language.rsplit_once('_')
        && let Some(digits) = size.strip_suffix('k')
        && digits.chars().all(|c| c.is_ascii_digit())
    {
        return base.to_string();
    }
    language.to_string()
}

pub fn build_result(log: &EventLog, spec: &TestSpec, repeated: bool, timestamp: u64) -> TestResult {
    let chars = stats::chars(log, false);
    let duration = stats::test_duration_ms(log) / 1000.0;
    let burst = stats::burst_history(log);
    let language =
        if spec.mode == Mode::Quote { remove_language_size(&spec.language) } else { spec.language.clone() };
    let mut r = TestResult {
        timestamp,
        mode: spec.mode,
        mode2: spec.mode2.clone(),
        language,
        punctuation: spec.punctuation,
        numbers: spec.numbers,
        difficulty: spec.difficulty,
        lazy_mode: spec.lazy_mode,
        wpm: round2(calculate_wpm(f64::from(chars.correct_word), duration)),
        raw: round2(calculate_wpm(f64::from(chars.all_correct + chars.incorrect + chars.extra), duration)),
        acc: round2(stats::accuracy(log, None).percentage),
        consistency: consistency(&burst),
        char_stats: [chars.correct_word, chars.incorrect, chars.extra, chars.missed],
        test_duration: duration,
        afk_duration: stats::afk_duration(log),
        afk_detected: stats::afk_detected(log),
        bailed_out: log.context.bailed_out,
        quote_id: spec.quote.as_ref().map(|q| q.id),
        quote_length: spec.quote.as_ref().map(|q| q.group),
        chart: ChartData { wpm: stats::wpm_history(log), burst, err: stats::error_count_history(log) },
        invalid: None,
    };
    r.invalid = invalid_reason(&r, spec, repeated);
    r
}

/// Ordre des contrôles de `finish`. Les bizarreries de Monkeytype sont
/// reproduites telles quelles :
/// - une limite custom en mots/sections < 10 (y compris 0 = infini) ou en temps < 15 rend le test « trop court » ;
/// - la limite wpm de 350 ne s'applique pas au mode words (seul words 10 a la sienne : 420).
fn invalid_reason(r: &TestResult, spec: &TestSpec, repeated: bool) -> Option<Invalid> {
    let mode2: Option<i64> = r.mode2.parse().ok();
    let d = r.test_duration;
    let too_short = d < 1.0
        || (r.mode == Mode::Time && mode2.is_some_and(|m| m > 0 && m < 15))
        || (r.mode == Mode::Time && mode2 == Some(0) && d < 15.0)
        || (r.mode == Mode::Words && mode2.is_some_and(|m| m > 0 && m < 10))
        || (r.mode == Mode::Words && mode2 == Some(0) && d < 15.0)
        || (r.mode == Mode::Custom
            && matches!(spec.custom_limit, Some(CustomLimit::Word(n) | CustomLimit::Section(n)) if n < 10))
        || (r.mode == Mode::Custom && matches!(spec.custom_limit, Some(CustomLimit::Time(n)) if n < 15))
        || (r.mode == Mode::Zen && d < 15.0);
    let words10 = r.mode == Mode::Words && r.mode2 == "10";
    let speed_invalid = |v: f64| v < 0.0 || (v > 350.0 && r.mode != Mode::Words && r.mode2 != "10") || (v > 420.0 && words10);
    if too_short {
        Some(Invalid::TooShort)
    } else if r.afk_detected {
        Some(Invalid::Afk)
    } else if repeated {
        Some(Invalid::Repeated)
    } else if speed_invalid(r.wpm) {
        Some(Invalid::Wpm)
    } else if speed_invalid(r.raw) {
        Some(Invalid::Raw)
    } else if r.acc < 75.0 || r.acc > 100.0 {
        Some(Invalid::Accuracy)
    } else {
        None
    }
}
```

Dans `session.rs`, ajouter `use crate::result::{TestResult, build_result};` et la méthode :
```rust
    /// Résultat du test terminé ; `timestamp` en ms Unix (fourni par l'appelant).
    pub fn result(&self, timestamp: u64) -> Option<TestResult> {
        (self.state == SessionState::Finished).then(|| build_result(&self.log, &self.spec, self.repeated, timestamp))
    }
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo test -p fasttype-core && cargo clippy -p fasttype-core --all-targets -- -D warnings && cargo fmt`
Expected: tous les tests PASS.

- [ ] **Step 5 : test de bout en bout avec la session**

Ajouter à `crates/fasttype-core/tests/session_flow.rs` :
```rust
#[test]
fn finished_session_produces_monkeytype_numbers() {
    let mut s = words_session(&["the", "cat"]);
    for (i, ch) in "the cat".chars().enumerate() {
        s.insert(ch, 1000.0 + i as f64 * 100.0);
    }
    let r = s.result(0).unwrap();
    assert_eq!(r.wpm, 140.0);
    assert_eq!(r.acc, 100.0);
    assert_eq!(r.char_stats, [7, 0, 0, 0]);
    assert_eq!(r.chart.wpm.len(), 1); // borne finale fractionnaire à 0,6 s
    assert_eq!(r.invalid, Some(fasttype_core::result::Invalid::TooShort));
}
```

Run: `cargo test -p fasttype-core --test session_flow`
Expected: PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): résultat, invalidations et clé de PB identiques à Monkeytype"
```

---

### Task 13 : propriétés et benchmarks

**Files:**
- Create: `crates/fasttype-core/tests/properties.rs`, `crates/fasttype-core/benches/session.rs`
- Modify: `crates/fasttype-core/Cargo.toml` (déclaration du benchmark)
- Test: les deux fichiers ci-dessus

**Interfaces:**
- Consumes : toute l'API publique de la crate.
- Produces : les mesures de référence `insert_10k_keystrokes` et `build_result_150s`.

- [ ] **Step 1 : écrire les propriétés**

`crates/fasttype-core/tests/properties.rs` :
```rust
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{SessionState, TestSession};
use fasttype_core::sources::RandomWords;
use fasttype_core::spec::TestSpec;
use proptest::prelude::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
enum Op {
    Char(char),
    Space,
    Backspace,
    DeleteWord,
    Wait(u16),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => prop::sample::select(vec!['t', 'h', 'e', 'c', 'a', 'x']).prop_map(Op::Char),
        2 => Just(Op::Space),
        1 => Just(Op::Backspace),
        1 => Just(Op::DeleteWord),
        2 => (1u16..1500).prop_map(Op::Wait),
    ]
}

fn session(seed: u64) -> TestSession {
    let words = Arc::new(["the", "cat", "hat", "tea", "ace"].iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let source = RandomWords::new(words, "english", false, false, Some(25));
    let generator = WordGenerator::new(Box::new(source), "english", false, false);
    TestSession::new(TestSpec::words(25, "english", false, false), generator, Box::new(SplitMix64::new(seed)))
}

proptest! {
    #[test]
    fn stats_stay_coherent(seed in any::<u64>(), ops in prop::collection::vec(op(), 1..300)) {
        let mut s = session(seed);
        let mut t = 0.0;
        for op in ops {
            match op {
                Op::Char(c) => { s.insert(c, t); }
                Op::Space => { s.insert(' ', t); }
                Op::Backspace => { s.backspace(t); }
                Op::DeleteWord => { s.delete_word(t); }
                Op::Wait(ms) => t += f64::from(ms),
            }
            t += 10.0;
            prop_assert!(s.active_index() < s.words().len().max(1));
        }
        if s.state() == SessionState::Running {
            s.bail_out(t);
        }
        if let Some(r) = s.result(0) {
            prop_assert!(r.wpm <= r.raw + 1e-9, "wpm {} > raw {}", r.wpm, r.raw);
            prop_assert!((0.0..=100.0).contains(&r.acc));
            prop_assert!((0.0..=100.0).contains(&r.consistency));
            let replayed = s.log().word_inputs(None);
            for (i, input) in &replayed {
                prop_assert_eq!(input, &s.inputs()[*i as usize]);
            }
        }
    }

    #[test]
    fn correct_words_never_reopen(seed in any::<u64>(), backspaces in 1usize..50) {
        let mut s = session(seed);
        let mut t = 0.0;
        for _ in 0..3 {
            let word: Vec<char> = s.word(s.active_index()).chars().collect();
            for ch in word {
                s.insert(ch, t);
                t += 50.0;
            }
        }
        let active = s.active_index();
        for _ in 0..backspaces {
            s.backspace(t);
            t += 10.0;
        }
        prop_assert_eq!(s.active_index(), active);
    }
}
```

- [ ] **Step 2 : lancer les propriétés**

Run: `cargo test -p fasttype-core --test properties`
Expected: PASS (256 cas par propriété). En cas d'échec, proptest affiche le cas minimal. Il s'agit alors d'un vrai bogue à corriger dans `session.rs` ou `stats.rs`, jamais en affaiblissant la propriété.

- [ ] **Step 3 : écrire le benchmark**

Ajouter à la fin de `crates/fasttype-core/Cargo.toml` (pas avant : Cargo refuse une cible de benchmark dont le fichier n'existe pas) :
```toml
[[bench]]
name = "session"
harness = false
```

`crates/fasttype-core/benches/session.rs` :
```rust
use criterion::{Criterion, criterion_group, criterion_main};
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::TestSession;
use fasttype_core::sources::RandomWords;
use fasttype_core::spec::TestSpec;
use std::hint::black_box;
use std::sync::Arc;

const ENGLISH: &[&str] = &[
    "the", "be", "of", "and", "a", "to", "in", "he", "have", "it", "that", "for", "they", "with", "as", "not", "on",
    "she", "at", "by",
];

fn session() -> TestSession {
    let words = Arc::new(ENGLISH.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let generator =
        WordGenerator::new(Box::new(RandomWords::new(words, "english", false, false, None)), "english", false, false);
    TestSession::new(TestSpec::time(0, "english", false, false), generator, Box::new(SplitMix64::new(42)))
}

/// Tape `count` caractères justes à 15 ms d'intervalle (~800 wpm), avec les
/// stats live à chaque tick, comme le fera l'interface.
fn type_chars(s: &mut TestSession, count: usize) {
    let mut t = 0.0;
    for _ in 0..count {
        let a = s.active_index();
        let typed = s.input(a).chars().count();
        let ch = s.word(a).chars().nth(typed).unwrap_or(' ');
        if s.tick(t) {
            black_box(s.live_stats());
        }
        s.insert(ch, t);
        t += 15.0;
    }
}

fn bench(c: &mut Criterion) {
    c.bench_function("insert_10k_keystrokes", |b| {
        b.iter(|| {
            let mut s = session();
            type_chars(&mut s, 10_000);
            black_box(s.active_index())
        })
    });
    let mut finished = session();
    type_chars(&mut finished, 10_000);
    finished.bail_out(200_000.0);
    c.bench_function("build_result_150s", |b| b.iter(|| black_box(finished.result(0))));
}

criterion_group!(benches, bench);
criterion_main!(benches);
```

- [ ] **Step 4 : lancer le benchmark et noter les chiffres**

Run: `cargo bench -p fasttype-core --bench session`
Expected : `insert_10k_keystrokes` sous **20 ms**, soit moins de 2 µs par frappe (stats live comprises) : c'est une petite fraction du budget de 2 ms de la spec. `build_result_150s` sous **5 ms** (calculé une seule fois, à la fin du test). Recopier les deux médianes dans le message de commit. Si un chiffre dépasse son seuil, l'investiguer avant de continuer (profilage avec `cargo flamegraph` ou recherche d'une boucle quadratique) ; ne jamais remonter le seuil.

- [ ] **Step 5 : vérification finale de la crate**

Run: `cargo fmt && cargo fmt --check && cargo test -p fasttype-core && cargo clippy -p fasttype-core --all-targets -- -D warnings`
Expected: tous les tests PASS, aucun avertissement, formatage stable.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-core
git commit -m "test(core): propriétés de cohérence et benchmarks de la session

insert_10k_keystrokes : <médiane> ; build_result_150s : <médiane>"
```
(remplacer `<médiane>` par les valeurs mesurées à l'étape 4)
