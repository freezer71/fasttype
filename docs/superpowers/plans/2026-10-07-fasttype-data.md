# fasttype-data — plan d'implémentation (plan 2 sur 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Embarquer dans le binaire toutes les données de Monkeytype (446 langues, 87 fichiers de citations, 187 thèmes) et les exposer par une API simple. Le plan règle aussi le coréen, reporté au plan 1.

**Architecture:**
- Un outil de build, `xtask`, récupère les fichiers de Monkeytype au commit figé, les valide et les compresse en deux « packs » zstd, plus un `themes.json` et un `manifest.toml`. Ces fichiers sont rangés dans `assets/` et versionnés.
- La crate `fasttype-data` les embarque avec `include_bytes!`. Au démarrage, elle ne lit que l'index ; une langue n'est décompressée que lorsqu'on la demande.
- Le format des packs, l'analyse des thèmes et la validation des langues vivent dans `fasttype-data`, et `xtask` les réutilise. Ainsi, écriture et lecture partagent le même code.

**Tech Stack:** Rust 1.97 (édition 2024), `zstd` 0.14.0 (niveau 19, fenêtre `--long=27`), `sha2` 0.11.0, `toml` 1.1.6, `serde` / `serde_json`, `git` en ligne de commande (pour `xtask` seulement).

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§3, §6.1, §8, §9). Le plan 1 est livré (`fasttype-core` sur `main`).

**Mesures déjà faites** (commit Monkeytype `574d8193498f75d13e7296584c82b64cebe2efea`) :

| Contenu | Brut | Compressé (zstd -19, `--long=27`) |
|---|---|---|
| Langues | 135 Mo | 21 Mo |
| Citations | 7,2 Mo | 2 Mo |

- Dans les langues, le nom du fichier est toujours égal au champ `name`, et aucun mot n'est vide.
- Les 87 fichiers de citations ont tous 4 groupes, des identifiants uniques et aucun texte vide.
- Couleurs des thèmes : hexadécimal à 3, 4, 6 ou 8 chiffres. Les formats à 4 et 8 chiffres portent une transparence, par exemple `#1c82adc4`.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Commentaires en français, identifiants en anglais.
- À l'exécution, `fasttype-data` ne fait **aucune E/S** : ni disque ni réseau, uniquement les données embarquées. Seul `xtask` lit le disque et utilise `git`.
- Commit Monkeytype de référence : `574d8193498f75d13e7296584c82b64cebe2efea`.
- Le build de `fasttype` ne demande jamais Internet : `assets/` est versionné.
- Une donnée embarquée invalide renvoie une erreur (`DataError`), jamais une panique. La spec (§8) prévoit une notification et un repli sur `english`, gérés par la TUI.
- Chaque tâche se termine par `cargo fmt`, `cargo test --workspace` et `cargo clippy --workspace --all-targets -- -D warnings`, tous au vert. Les tâches 1 à 5 s'exécutent avant que `assets/` existe, et le code qui fait `include_bytes!` n'arrive qu'à la tâche 7.
- Pour chaque test, compter les tests réellement passés (total des lignes `test result`), et non la dernière ligne, qui est celle des doc-tests et vaut souvent 0.

## Review Focus

1. **Pack tronqué ou corrompu** (fichier coupé, octets modifiés) : on obtient `Err`, jamais de panique ni de lecture hors bornes. Tests dans la tâche 2 (`truncated_pack_is_rejected`, `corrupted_frame_is_an_error`).
2. **Couleurs à 3, 4 ou 8 chiffres et transparence** : expansion correcte et composition sur le fond. Tests dans la tâche 3 (`parses_all_hex_forms`, `alpha_is_composited_over_background`).
3. **Fichier source manquant ou invalide au moment du build** : `xtask` échoue avec un message qui nomme le fichier, et ne laisse pas de `assets/` à moitié écrit. Test dans la tâche 5 (`invalid_source_leaves_previous_assets_untouched`).
4. **Langue inconnue, ou langue sans citations** : on obtient `Err(UnknownLanguage)` ou `Ok(None)`, sans panique. Tests dans la tâche 7 (`unknown_language_is_an_error`, `language_without_quotes_returns_none`).
5. **Chargement concurrent de la même langue depuis plusieurs threads** (la TUI charge en arrière-plan) : aucune course ni double décompression visible, et le même `Arc` est rendu. Test dans la tâche 7 (`cache_is_shared_across_threads`).

---

## Structure des fichiers

```
Cargo.toml                          + membres fasttype-data, xtask ; dépendances ; opt-level des données en dev
.cargo/config.toml                  alias `cargo xtask`
.gitattributes                      assets binaires
assets/                             généré par xtask, versionné
├── languages.pack  quotes.pack  themes.json  manifest.toml
crates/fasttype-core/src/
├── chars.rs                        + décomposition Hangul, count_chars_for, count_words(korean)
├── event.rs                        + EventContext.korean
├── stats.rs  session.rs            utilisent le drapeau coréen
crates/fasttype-data/
├── Cargo.toml
├── src/lib.rs                      modules + DataError
├── src/pack.rs                     format pack : PackWriter, Pack, compress/decompress
├── src/themes.rs                   Rgba, Theme, parse_themes_ts
├── src/language.rs                 LanguageFile, Language, parse_language, parse_quotes
├── src/catalog.rs                  données embarquées : langues, citations, thèmes, LanguageCache
├── tests/pack.rs  tests/themes.rs  tests/language.rs  tests/catalog.rs
└── benches/catalog.rs
xtask/
├── Cargo.toml
├── src/lib.rs                      build_assets, fetch_source
├── src/main.rs                     `cargo xtask fetch-data`
└── tests/build.rs
```

---

### Task 1 : coréen dans le moteur (décomposition Hangul)

**Files:**
- Modify: `crates/fasttype-core/src/chars.rs`, `crates/fasttype-core/src/event.rs`, `crates/fasttype-core/src/stats.rs`, `crates/fasttype-core/src/session.rs`
- Modify (appelants de `count_words` et de `EventContext`) : `crates/fasttype-core/tests/chars.rs`, `crates/fasttype-core/tests/properties.rs`, `crates/fasttype-core/tests/common/mod.rs`, `crates/fasttype-core/tests/event.rs`
- Test: `crates/fasttype-core/tests/korean.rs`

**Interfaces:**
- Produces, dans `chars` :
  - `hangul_disassemble(s: &str) -> String` ;
  - `contains_korean(s: &str) -> bool` ;
  - `count_chars_for(input: &str, target: &str, credit_partial: bool, korean: bool) -> CharCounts` ;
  - nouvelle signature `count_words<'a, I>(words: I, credit_partial_last: bool, korean: bool) -> CharCounts`.
- Produces : `EventContext.korean: bool` (`#[serde(default)]`).
- Comportement Monkeytype à reproduire :
  - `koreanStatus` est activé si les mots générés au départ contiennent un caractère des plages `[가-힯]|[ᄀ-ᇿ]|[㄰-㆏]|[ꥠ-꥿]|[ힰ-퟿]` (test-logic.ts l.543) ;
  - la saisie et la cible sont alors décomposées en jamo (`Hangul.disassemble`) avant `countChars` (stats.ts l.438-445).

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-core/tests/korean.rs` :
```rust
use fasttype_core::chars::{CharCounts, contains_korean, count_chars_for, count_words, hangul_disassemble};
use fasttype_core::generator::WordGenerator;
use fasttype_core::rng::SplitMix64;
use fasttype_core::session::{SessionState, TestSession};
use fasttype_core::sources::SequenceWords;
use fasttype_core::spec::TestSpec;

#[test]
fn syllables_split_into_jamo_with_compounds() {
    assert_eq!(hangul_disassemble("값"), "ㄱㅏㅂㅅ"); // ㅄ → ㅂㅅ
    assert_eq!(hangul_disassemble("와"), "ㅇㅗㅏ"); // ㅘ → ㅗㅏ
    assert_eq!(hangul_disassemble("한국 "), "ㅎㅏㄴㄱㅜㄱ ");
    assert_eq!(hangul_disassemble("가a"), "ㄱㅏa");
    assert_eq!(hangul_disassemble("ㄲ"), "ㄲ"); // consonne double : une seule touche
    assert_eq!(hangul_disassemble("ㄳ"), "ㄱㅅ");
}

#[test]
fn detects_korean_ranges() {
    assert!(contains_korean("한국"));
    assert!(contains_korean("ㄱ"));
    assert!(!contains_korean("hello"));
    assert!(!contains_korean("日本"));
}

#[test]
fn korean_counts_jamo() {
    let c = count_chars_for("한국 ", "한국 ", false, true);
    assert_eq!(c, CharCounts { all_correct: 7, correct_word: 7, ..Default::default() });
    // syllabe incomplète en cours de frappe : préfixe en jamo, crédit partiel
    let partial = count_chars_for("하", "한 ", true, true);
    assert_eq!((partial.all_correct, partial.correct_word), (2, 2));
    assert_eq!(count_words([("한 ", "한 ", true)], false, true).correct_word, 4);
    assert_eq!(count_words([("한 ", "한 ", true)], false, false).correct_word, 2);
}

#[test]
fn korean_session_counts_jamo_in_result() {
    let seq = SequenceWords::new(vec!["한국".into(), "사람".into()]);
    let generator = WordGenerator::new(Box::new(seq), "korean", false, false);
    let mut s = TestSession::new(TestSpec::words(2, "korean", false, false), generator, Box::new(SplitMix64::new(1)));
    assert!(s.log().context.korean);
    for (i, ch) in "한국 사람".chars().enumerate() {
        s.insert(ch, i as f64 * 100.0);
    }
    assert_eq!(s.state(), SessionState::Finished);
    // 한국 + espace = 7 jamo ; 사람 = ㅅㅏㄹㅏㅁ = 5
    assert_eq!(s.result(0).unwrap().char_stats, [12, 0, 0, 0]);
}

#[test]
fn non_korean_session_is_unflagged() {
    let seq = SequenceWords::new(vec!["the".into()]);
    let generator = WordGenerator::new(Box::new(seq), "english", false, false);
    let s = TestSession::new(TestSpec::words(1, "english", false, false), generator, Box::new(SplitMix64::new(1)));
    assert!(!s.log().context.korean);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-core --test korean`
Expected: échec de compilation (`unresolved imports ... contains_korean, count_chars_for, hangul_disassemble`).

- [ ] **Step 3 : implémenter la décomposition dans `chars.rs`**

Ajouter à la fin de `crates/fasttype-core/src/chars.rs` :
```rust
/// Initiales (jamo de compatibilité), dans l'ordre Unicode des syllabes.
const CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];
/// Voyelles médianes.
const JUNG: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ', 'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ',
    'ㅣ',
];
/// Finales (la première case : pas de finale).
const JONG: [Option<char>; 28] = [
    None,
    Some('ㄱ'),
    Some('ㄲ'),
    Some('ㄳ'),
    Some('ㄴ'),
    Some('ㄵ'),
    Some('ㄶ'),
    Some('ㄷ'),
    Some('ㄹ'),
    Some('ㄺ'),
    Some('ㄻ'),
    Some('ㄼ'),
    Some('ㄽ'),
    Some('ㄾ'),
    Some('ㄿ'),
    Some('ㅀ'),
    Some('ㅁ'),
    Some('ㅂ'),
    Some('ㅄ'),
    Some('ㅅ'),
    Some('ㅆ'),
    Some('ㅇ'),
    Some('ㅈ'),
    Some('ㅊ'),
    Some('ㅋ'),
    Some('ㅌ'),
    Some('ㅍ'),
    Some('ㅎ'),
];

/// Jamo composés tapés en deux touches (`COMPLEX_CONSONANTS` / `COMPLEX_VOWELS`
/// de hangul-js). Les consonnes doubles (ㄲ, ㄸ…) restent d'un seul tenant.
fn split_compound(c: char) -> Option<[char; 2]> {
    Some(match c {
        'ㄳ' => ['ㄱ', 'ㅅ'],
        'ㄵ' => ['ㄴ', 'ㅈ'],
        'ㄶ' => ['ㄴ', 'ㅎ'],
        'ㄺ' => ['ㄹ', 'ㄱ'],
        'ㄻ' => ['ㄹ', 'ㅁ'],
        'ㄼ' => ['ㄹ', 'ㅂ'],
        'ㄽ' => ['ㄹ', 'ㅅ'],
        'ㄾ' => ['ㄹ', 'ㅌ'],
        'ㄿ' => ['ㄹ', 'ㅍ'],
        'ㅀ' => ['ㄹ', 'ㅎ'],
        'ㅄ' => ['ㅂ', 'ㅅ'],
        'ㅘ' => ['ㅗ', 'ㅏ'],
        'ㅙ' => ['ㅗ', 'ㅐ'],
        'ㅚ' => ['ㅗ', 'ㅣ'],
        'ㅝ' => ['ㅜ', 'ㅓ'],
        'ㅞ' => ['ㅜ', 'ㅔ'],
        'ㅟ' => ['ㅜ', 'ㅣ'],
        'ㅢ' => ['ㅡ', 'ㅣ'],
        _ => return None,
    })
}

fn push_jamo(out: &mut String, c: char) {
    match split_compound(c) {
        Some([a, b]) => {
            out.push(a);
            out.push(b);
        }
        None => out.push(c),
    }
}

/// `Hangul.disassemble(s).join("")` : chaque syllabe devient ses jamo, et les
/// jamo composés sont scindés ; les autres caractères sont gardés tels quels.
pub fn hangul_disassemble(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        let code = c as u32;
        if (0xAC00..=0xD7A3).contains(&code) {
            let i = code - 0xAC00;
            push_jamo(&mut out, CHO[(i / 588) as usize]);
            push_jamo(&mut out, JUNG[((i % 588) / 28) as usize]);
            if let Some(t) = JONG[(i % 28) as usize] {
                push_jamo(&mut out, t);
            }
        } else {
            push_jamo(&mut out, c);
        }
    }
    out
}

/// Détection de `koreanStatus` (test-logic.ts).
pub fn contains_korean(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(c as u32, 0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xD7B0..=0xD7FF)
    })
}

/// `countChars`, précédé de la décomposition en jamo quand le test est coréen.
pub fn count_chars_for(input: &str, target: &str, credit_partial: bool, korean: bool) -> CharCounts {
    if korean {
        count_chars(&hangul_disassemble(input), &hangul_disassemble(target), credit_partial)
    } else {
        count_chars(input, target, credit_partial)
    }
}
```

Puis modifier `count_words` pour qu'il prenne le drapeau :
```rust
/// Additionne `count_chars` mot par mot et s'arrête après le dernier mot
/// (boucle de `getChars`). Seul le dernier mot peut recevoir le crédit partiel.
pub fn count_words<'a, I>(words: I, credit_partial_last: bool, korean: bool) -> CharCounts
where
    I: IntoIterator<Item = (&'a str, &'a str, bool)>,
{
    let mut total = CharCounts::default();
    for (input, target, last) in words {
        total += count_chars_for(input, target, last && credit_partial_last, korean);
        if last {
            break;
        }
    }
    total
}
```

- [ ] **Step 4 : brancher le drapeau dans le journal, les stats et la session**

1. `event.rs`, dans `EventContext`, ajouter après `bailed_out` :
```rust
    /// `koreanStatus` : les comptes se font en jamo.
    #[serde(default)]
    pub korean: bool,
```
2. `stats.rs` :
   - dans `chars`, remplacer l'appel par `count_words(..., partial, log.context.korean)` ;
   - dans `wpm_history`, remplacer les deux `count_chars(input, target, X)` par `count_chars_for(input, target, X, log.context.korean)` ;
   - adapter l'import en `use crate::chars::{CharCounts, count_chars_for, count_words};`.
3. `session.rs` :
   - dans `empty`, ajouter `korean: false` au littéral `EventContext { ... }` ;
   - dans `live_stats`, passer `self.log.context.korean` en troisième argument de `count_words` ;
   - ajouter la méthode privée :
```rust
    /// `koreanStatus` : activé si les mots générés au départ contiennent du coréen.
    fn detect_korean(&mut self) {
        self.log.context.korean = self.words.iter().any(|w| contains_korean(w));
    }
```
   - l'appeler à la fin de `new` (après `strip_last_separator_if_done()`, avant `s` ; jamais en zen, qui retourne plus tôt) et dans `into_repeat` (juste avant `s.repeated = true;`) ;
   - importer `contains_korean` : `use crate::chars::{contains_korean, count_words, normalize_typed};` en remplaçant les deux imports `crate::chars` existants.
4. Appelants dans les tests :
   - `tests/common/mod.rs` : ajouter `korean: false,` au littéral `EventContext` ;
   - `tests/event.rs` : ajouter `korean: false,` au littéral `EventContext` de `log()` ;
   - `tests/chars.rs` : les deux appels deviennent `count_words(words, true, false)` et `count_words(words, false, false)` ;
   - `tests/properties.rs` : dans `naive_wpm_history`, passer `log.context.korean` en troisième argument de `count_words`.

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-core 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy -p fasttype-core --all-targets -- -D warnings`
Expected: `passed 132 failed 0` (127 + 5 nouveaux), aucun avertissement.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-core
git commit -m "feat(core): comptes en jamo pour le coréen (koreanStatus de Monkeytype)"
```

---

### Task 2 : crate fasttype-data et format pack

**Files:**
- Modify: `Cargo.toml` (membre, dépendances, profil de dev)
- Create: `crates/fasttype-data/Cargo.toml`, `crates/fasttype-data/src/lib.rs`, `crates/fasttype-data/src/pack.rs`
- Test: `crates/fasttype-data/tests/pack.rs`

**Interfaces:**
- Produces, dans `fasttype_data::pack` :
  - `PackEntry { name: String, offset: u64, len: u64, raw_len: u64 }` ;
  - `PackError { BadMagic, Truncated, BadIndex(serde_json::Error), Io(std::io::Error), SizeMismatch { expected: u64, actual: u64 } }` (`Display`, `Error`) ;
  - `PackWriter::{new(), add(&mut self, name: &str, raw: &[u8]) -> std::io::Result<()>, finish(self) -> Vec<u8>}` ;
  - `Pack<'a>::{parse(bytes: &'a [u8]) -> Result<Pack<'a>, PackError>, entries(&self) -> &[PackEntry], get(&self, name) -> Option<&PackEntry>, decompress(&self, name) -> Result<Option<Vec<u8>>, PackError>}` ;
  - `compress(raw: &[u8]) -> std::io::Result<Vec<u8>>` et `decompress_frame(frame: &[u8], raw_len: u64) -> Result<Vec<u8>, PackError>`.

- [ ] **Step 1 : déclarer la crate**

Dans `Cargo.toml` (racine) :
- `members = ["crates/fasttype-core", "crates/fasttype-data"]` ;
- sous `[workspace.dependencies]`, ajouter :
```toml
fasttype-core = { path = "crates/fasttype-core" }
fasttype-data = { path = "crates/fasttype-data" }
zstd = "0.14.0"
sha2 = "0.11.0"
toml = "1.1.6"
```
- à la fin du fichier, ajouter :
```toml
# Les tests décompressent et analysent 135 Mo de JSON : optimisés même en dev.
[profile.dev.package.fasttype-data]
opt-level = 2

[profile.dev.package."*"]
opt-level = 2
```

`crates/fasttype-data/Cargo.toml` :
```toml
[package]
name = "fasttype-data"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
fasttype-core.workspace = true
serde.workspace = true
serde_json.workspace = true
zstd.workspace = true
```

`crates/fasttype-data/src/lib.rs` :
```rust
//! Données de Monkeytype embarquées : langues, citations et thèmes.
//! Aucune E/S à l'exécution ; `xtask` produit les fichiers de `assets/`.

pub mod pack;
```

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-data/tests/pack.rs` :
```rust
use fasttype_data::pack::{Pack, PackError, PackWriter, compress, decompress_frame};

fn sample() -> Vec<u8> {
    let mut w = PackWriter::new();
    w.add("alpha", b"{\"words\":[\"a\"]}").unwrap();
    w.add("beta", "é".repeat(50_000).as_bytes()).unwrap();
    w.finish()
}

#[test]
fn roundtrip_keeps_names_and_bytes() {
    let bytes = sample();
    let pack = Pack::parse(&bytes).unwrap();
    let names: Vec<&str> = pack.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["alpha", "beta"]);
    assert_eq!(pack.decompress("alpha").unwrap().unwrap(), b"{\"words\":[\"a\"]}");
    assert_eq!(pack.decompress("beta").unwrap().unwrap(), "é".repeat(50_000).as_bytes());
    assert_eq!(pack.get("beta").unwrap().raw_len, 100_000);
    assert!(pack.get("beta").unwrap().len < 1_000, "compressé");
}

#[test]
fn unknown_entry_is_none() {
    let bytes = sample();
    assert!(Pack::parse(&bytes).unwrap().decompress("gamma").unwrap().is_none());
}

#[test]
fn duplicate_names_are_refused() {
    let mut w = PackWriter::new();
    w.add("a", b"1").unwrap();
    assert!(w.add("a", b"2").is_err());
}

#[test]
fn bad_magic_is_rejected() {
    assert!(matches!(Pack::parse(b"nope"), Err(PackError::BadMagic)));
}

#[test]
fn truncated_pack_is_rejected() {
    let bytes = sample();
    for cut in [6, 9, 20, bytes.len() - 1] {
        assert!(matches!(Pack::parse(&bytes[..cut]), Err(PackError::Truncated)), "coupé à {cut}");
    }
}

#[test]
fn corrupted_frame_is_an_error() {
    let mut bytes = sample();
    let n = bytes.len();
    for b in &mut bytes[n - 40..n - 10] {
        *b ^= 0xA5;
    }
    let pack = Pack::parse(&bytes).unwrap();
    assert!(pack.decompress("beta").is_err());
}

#[test]
fn frame_size_is_checked() {
    let frame = compress(b"hello").unwrap();
    assert_eq!(decompress_frame(&frame, 5).unwrap(), b"hello");
    assert!(matches!(decompress_frame(&frame, 6), Err(PackError::SizeMismatch { expected: 6, actual: 5 })));
}

#[test]
fn compression_is_deterministic() {
    assert_eq!(sample(), sample());
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-data --test pack`
Expected: échec de compilation (`unresolved imports fasttype_data::pack::{Pack, ...}`).

- [ ] **Step 4 : implémenter `pack.rs`**

```rust
//! Format « pack » : plusieurs fichiers compressés en zstd dans un seul blob,
//! embarqué tel quel dans le binaire.
//!
//! Disposition : `FTPK\x01`, longueur de l'index (u32 petit-boutiste), index
//! JSON (`[PackEntry]`), puis les trames zstd concaténées. Les offsets sont
//! relatifs au début des trames.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Read, Write};

const MAGIC: &[u8; 5] = b"FTPK\x01";
/// Fenêtre de 128 Mio (`zstd --long=27`) : utile pour les grosses listes de mots.
const WINDOW_LOG: u32 = 27;
const LEVEL: i32 = 19;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackEntry {
    pub name: String,
    pub offset: u64,
    pub len: u64,
    pub raw_len: u64,
}

#[derive(Debug)]
pub enum PackError {
    BadMagic,
    Truncated,
    BadIndex(serde_json::Error),
    Io(io::Error),
    SizeMismatch { expected: u64, actual: u64 },
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::BadMagic => write!(f, "pack : en-tête invalide"),
            PackError::Truncated => write!(f, "pack : données tronquées"),
            PackError::BadIndex(e) => write!(f, "pack : index illisible ({e})"),
            PackError::Io(e) => write!(f, "pack : décompression impossible ({e})"),
            PackError::SizeMismatch { expected, actual } => {
                write!(f, "pack : taille décompressée {actual} au lieu de {expected}")
            }
        }
    }
}

impl std::error::Error for PackError {}

impl From<io::Error> for PackError {
    fn from(e: io::Error) -> Self {
        PackError::Io(e)
    }
}

/// Compresse une trame (niveau 19, fenêtre longue, somme de contrôle pour
/// détecter une corruption). Sortie déterministe.
pub fn compress(raw: &[u8]) -> io::Result<Vec<u8>> {
    let mut enc = zstd::stream::Encoder::new(Vec::new(), LEVEL)?;
    enc.include_checksum(true)?;
    enc.window_log(WINDOW_LOG)?;
    enc.long_distance_matching(true)?;
    enc.write_all(raw)?;
    enc.finish()
}

/// Décompresse une trame et vérifie sa taille.
pub fn decompress_frame(frame: &[u8], raw_len: u64) -> Result<Vec<u8>, PackError> {
    let mut dec = zstd::stream::Decoder::new(frame)?;
    dec.window_log_max(WINDOW_LOG)?;
    let mut out = Vec::with_capacity(raw_len as usize);
    dec.read_to_end(&mut out)?;
    let actual = out.len() as u64;
    if actual != raw_len {
        return Err(PackError::SizeMismatch { expected: raw_len, actual });
    }
    Ok(out)
}

#[derive(Default)]
pub struct PackWriter {
    entries: Vec<PackEntry>,
    payload: Vec<u8>,
}

impl PackWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute un fichier ; les noms doivent être uniques. L'ordre d'ajout est
    /// conservé : l'appelant trie les noms pour un pack reproductible.
    pub fn add(&mut self, name: &str, raw: &[u8]) -> io::Result<()> {
        if self.entries.iter().any(|e| e.name == name) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("entrée en double : {name}")));
        }
        let frame = compress(raw)?;
        self.entries.push(PackEntry {
            name: name.to_string(),
            offset: self.payload.len() as u64,
            len: frame.len() as u64,
            raw_len: raw.len() as u64,
        });
        self.payload.extend_from_slice(&frame);
        Ok(())
    }

    pub fn finish(self) -> Vec<u8> {
        let index = serde_json::to_vec(&self.entries).expect("index sérialisable");
        let mut out = Vec::with_capacity(MAGIC.len() + 4 + index.len() + self.payload.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(index.len() as u32).to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&self.payload);
        out
    }
}

/// Vue en lecture seule d'un pack ; l'index est lu une fois, les trames à la demande.
pub struct Pack<'a> {
    entries: Vec<PackEntry>,
    payload: &'a [u8],
}

impl<'a> Pack<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, PackError> {
        let rest = bytes.strip_prefix(MAGIC.as_slice()).ok_or(PackError::BadMagic)?;
        let (len_bytes, rest) = rest.split_first_chunk::<4>().ok_or(PackError::Truncated)?;
        let index_len = u32::from_le_bytes(*len_bytes) as usize;
        if rest.len() < index_len {
            return Err(PackError::Truncated);
        }
        let (index, payload) = rest.split_at(index_len);
        let entries: Vec<PackEntry> = serde_json::from_slice(index).map_err(PackError::BadIndex)?;
        for e in &entries {
            let end = e.offset.checked_add(e.len).ok_or(PackError::Truncated)?;
            if end > payload.len() as u64 {
                return Err(PackError::Truncated);
            }
        }
        Ok(Self { entries, payload })
    }

    pub fn entries(&self) -> &[PackEntry] {
        &self.entries
    }

    pub fn get(&self, name: &str) -> Option<&PackEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// `Ok(None)` si l'entrée n'existe pas.
    pub fn decompress(&self, name: &str) -> Result<Option<Vec<u8>>, PackError> {
        let Some(e) = self.get(name) else {
            return Ok(None);
        };
        let frame = &self.payload[e.offset as usize..(e.offset + e.len) as usize];
        decompress_frame(frame, e.raw_len).map(Some)
    }
}
```

Note pour `truncated_pack_is_rejected` :
- une coupe à 6 ou 9 octets tombe dans la longueur de l'index (`Truncated`) ;
- une coupe à 20 octets tombe dans l'index. Si `rest.len() < index_len`, on obtient `Truncated` avant toute analyse JSON. L'index fait plus de 20 octets, donc c'est bien ce cas ;
- une coupe au dernier octet tombe dans les trames : la vérification des bornes renvoie `Truncated`.

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-data --test pack && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 8 tests PASS, aucun avertissement.

- [ ] **Step 6 : commit**

```bash
git add Cargo.toml Cargo.lock crates/fasttype-data
git commit -m "feat(data): crate fasttype-data et format pack zstd"
```

---

### Task 3 : thèmes (couleurs et lecture de themes.ts)

**Files:**
- Create: `crates/fasttype-data/src/themes.rs`
- Modify: `crates/fasttype-data/src/lib.rs` (ajouter `pub mod themes;`)
- Test: `crates/fasttype-data/tests/themes.rs`

**Interfaces:**
- Produces, dans `fasttype_data::themes` :
  - `Rgba { r: u8, g: u8, b: u8, a: u8 }` (`Copy`, serde au format `"#rrggbb"` ou `"#rrggbbaa"`), avec `parse_hex(&str) -> Option<Rgba>`, `to_hex(self) -> String` et `over(self, bg: Rgba) -> Rgba` (résultat opaque) ;
  - `Theme { name: String, bg, main, caret, sub, sub_alt, text, error, error_extra, colorful_error, colorful_error_extra: Rgba, has_css: bool }` (serde en camelCase) ;
  - `ThemeParseError { line: usize, message: String }` et `parse_themes_ts(src: &str) -> Result<Vec<Theme>, ThemeParseError>` (triés par nom, comme `ThemesList`).

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-data/tests/themes.rs` :
```rust
use fasttype_data::themes::{Rgba, Theme, parse_themes_ts};

const SAMPLE: &str = r##"import { z } from "zod";

export const themes: Record<ThemeName, Theme> = {
  serika_dark: {
    bg: "#323437",
    main: "#e2b714",
    caret: "#e2b714",
    sub: "#646669",
    subAlt: "#2c2e31",
    text: "#d1d0c5",
    error: "#ca4754",
    errorExtra: "#7e2a33",
    colorfulError: "#ca4754",
    colorfulErrorExtra: "#7e2a33",
  },
  "8008": {
    bg: "#333a45",
    caret: "#f44c7f",
    main: "#f44c7f",
    sub: "#1c82adc4",
    subAlt: "#2e343d",
    text: "#e9ecf0",
    error: "#da3333",
    errorExtra: "#791717",
    colorfulError: "#c5da33",
    colorfulErrorExtra: "#849224",
    hasCss: true,
  },
};

export type ThemeWithName = Theme & { name: ThemeName };
"##;

#[test]
fn parses_all_hex_forms() {
    assert_eq!(Rgba::parse_hex("#abc"), Some(Rgba { r: 0xaa, g: 0xbb, b: 0xcc, a: 255 }));
    assert_eq!(Rgba::parse_hex("#abcd"), Some(Rgba { r: 0xaa, g: 0xbb, b: 0xcc, a: 0xdd }));
    assert_eq!(Rgba::parse_hex("#E2B714"), Some(Rgba { r: 0xe2, g: 0xb7, b: 0x14, a: 255 }));
    assert_eq!(Rgba::parse_hex("#1c82adc4"), Some(Rgba { r: 0x1c, g: 0x82, b: 0xad, a: 0xc4 }));
    for bad in ["#12", "e2b714", "#ggg", "#12345", ""] {
        assert_eq!(Rgba::parse_hex(bad), None, "{bad}");
    }
}

#[test]
fn hex_roundtrip() {
    assert_eq!(Rgba::parse_hex("#e2b714").unwrap().to_hex(), "#e2b714");
    assert_eq!(Rgba::parse_hex("#1c82adc4").unwrap().to_hex(), "#1c82adc4");
}

#[test]
fn alpha_is_composited_over_background() {
    let black = Rgba { r: 0, g: 0, b: 0, a: 255 };
    let half_red = Rgba { r: 255, g: 0, b: 0, a: 128 };
    assert_eq!(half_red.over(black), Rgba { r: 128, g: 0, b: 0, a: 255 });
    let opaque = Rgba::parse_hex("#e2b714").unwrap();
    assert_eq!(opaque.over(black), opaque);
}

#[test]
fn parses_themes_file_sorted_by_name() {
    let themes = parse_themes_ts(SAMPLE).unwrap();
    let names: Vec<&str> = themes.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["8008", "serika_dark"]);
    let serika = &themes[1];
    assert_eq!(serika.main.to_hex(), "#e2b714");
    assert_eq!(serika.sub_alt.to_hex(), "#2c2e31");
    assert!(!serika.has_css);
    assert!(themes[0].has_css);
    assert_eq!(themes[0].sub.a, 0xc4);
}

#[test]
fn unknown_field_is_an_error_with_line() {
    let src = SAMPLE.replace("    subAlt: \"#2c2e31\",", "    subAlt: \"#2c2e31\",\n    glow: \"#ffffff\",");
    let err = parse_themes_ts(&src).unwrap_err();
    assert!(err.message.contains("glow"), "{err}");
    assert!(err.line > 0);
}

#[test]
fn missing_color_is_an_error() {
    let src = SAMPLE.replacen("    caret: \"#e2b714\",\n", "", 1);
    assert!(parse_themes_ts(&src).unwrap_err().message.contains("caret"));
}

#[test]
fn missing_block_is_an_error() {
    assert!(parse_themes_ts("export const other = {};").is_err());
    let unterminated = SAMPLE.replace("};\n\nexport type", "\nexport type");
    assert!(parse_themes_ts(&unterminated).is_err());
}

#[test]
fn theme_json_roundtrip() {
    let themes = parse_themes_ts(SAMPLE).unwrap();
    let json = serde_json::to_string(&themes).unwrap();
    assert!(json.contains("\"subAlt\":\"#2c2e31\""), "{json}");
    assert_eq!(serde_json::from_str::<Vec<Theme>>(&json).unwrap(), themes);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-data --test themes`
Expected: échec de compilation (`unresolved import fasttype_data::themes`).

- [ ] **Step 3 : implémenter `themes.rs`**

```rust
//! Thèmes de Monkeytype (`frontend/src/ts/constants/themes.ts`) : couleurs
//! et lecture stricte du fichier source par `xtask`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// `#rgb`, `#rgba`, `#rrggbb` ou `#rrggbbaa` (schéma `hexColorSchema`).
    pub fn parse_hex(s: &str) -> Option<Self> {
        let hex = s.strip_prefix('#')?;
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let digit = |i: usize| u8::from_str_radix(&hex[i..i + 1], 16).ok().map(|d| d * 17);
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        match hex.len() {
            3 => Some(Self { r: digit(0)?, g: digit(1)?, b: digit(2)?, a: 255 }),
            4 => Some(Self { r: digit(0)?, g: digit(1)?, b: digit(2)?, a: digit(3)? }),
            6 => Some(Self { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: 255 }),
            8 => Some(Self { r: byte(0)?, g: byte(2)?, b: byte(4)?, a: byte(6)? }),
            _ => None,
        }
    }

    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Couleur opaque obtenue en posant `self` sur `bg` (le terminal n'a pas de transparence).
    pub fn over(self, bg: Rgba) -> Rgba {
        let a = f64::from(self.a) / 255.0;
        let mix = |fg: u8, bg: u8| (a * f64::from(fg) + (1.0 - a) * f64::from(bg)).round() as u8;
        Rgba { r: mix(self.r, bg.r), g: mix(self.g, bg.g), b: mix(self.b, bg.b), a: 255 }
    }
}

impl Serialize for Rgba {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgba {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Rgba::parse_hex(&s).ok_or_else(|| serde::de::Error::custom(format!("couleur invalide : {s}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub name: String,
    pub bg: Rgba,
    pub main: Rgba,
    pub caret: Rgba,
    pub sub: Rgba,
    pub sub_alt: Rgba,
    pub text: Rgba,
    pub error: Rgba,
    pub error_extra: Rgba,
    pub colorful_error: Rgba,
    pub colorful_error_extra: Rgba,
    /// Le thème a un CSS décoratif sur le site (non transposable en terminal).
    #[serde(default)]
    pub has_css: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ThemeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "themes.ts, ligne {} : {}", self.line, self.message)
    }
}

impl std::error::Error for ThemeParseError {}

const START: &str = "export const themes: Record<ThemeName, Theme> = {";
const COLORS: [&str; 10] =
    ["bg", "main", "caret", "sub", "subAlt", "text", "error", "errorExtra", "colorfulError", "colorfulErrorExtra"];

fn err(line: usize, message: impl Into<String>) -> ThemeParseError {
    ThemeParseError { line, message: message.into() }
}

fn build(name: String, mut fields: BTreeMap<String, String>, line: usize) -> Result<Theme, ThemeParseError> {
    let mut color = |key: &str| -> Result<Rgba, ThemeParseError> {
        let v = fields.remove(key).ok_or_else(|| err(line, format!("{name} : couleur `{key}` manquante")))?;
        Rgba::parse_hex(&v).ok_or_else(|| err(line, format!("{name} : couleur `{key}` invalide ({v})")))
    };
    let [bg, main, caret, sub, sub_alt, text, error, error_extra, colorful_error, colorful_error_extra] =
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(|i| color(COLORS[i]));
    let theme = Theme {
        name: name.clone(),
        bg: bg?,
        main: main?,
        caret: caret?,
        sub: sub?,
        sub_alt: sub_alt?,
        text: text?,
        error: error?,
        error_extra: error_extra?,
        colorful_error: colorful_error?,
        colorful_error_extra: colorful_error_extra?,
        has_css: match fields.remove("hasCss").as_deref() {
            None | Some("false") => false,
            Some("true") => true,
            Some(other) => return Err(err(line, format!("{name} : hasCss invalide ({other})"))),
        },
    };
    if let Some(unknown) = fields.keys().next() {
        return Err(err(line, format!("{name} : champ inconnu `{unknown}`")));
    }
    Ok(theme)
}

/// Lit l'objet `themes` de `themes.ts`, ligne par ligne. Échoue à la moindre
/// forme inattendue plutôt que de deviner.
pub fn parse_themes_ts(src: &str) -> Result<Vec<Theme>, ThemeParseError> {
    let start = src.find(START).ok_or_else(|| err(0, "objet `themes` introuvable"))?;
    let first_line = src[..start].lines().count() + 1;
    let mut themes = Vec::new();
    let mut current: Option<(String, BTreeMap<String, String>, usize)> = None;
    for (i, raw) in src[start..].lines().enumerate().skip(1) {
        let line_no = first_line + i;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, mut fields, start_line)) = current.take() else {
            if line == "};" {
                themes.sort_by(|a: &Theme, b: &Theme| a.name.cmp(&b.name));
                return Ok(themes);
            }
            let key = line.strip_suffix(": {").ok_or_else(|| err(line_no, format!("entrée de thème attendue : {line}")))?;
            let key = key.trim().trim_matches('"');
            current = Some((key.to_string(), BTreeMap::new(), line_no));
            continue;
        };
        if line == "}," || line == "}" {
            themes.push(build(name, fields, start_line)?);
            continue;
        }
        let body = line.strip_suffix(',').unwrap_or(line);
        let (key, value) = body.split_once(':').ok_or_else(|| err(line_no, format!("champ attendu : {line}")))?;
        let value = value.trim();
        let value = match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            Some(s) => s.to_string(),
            None if value == "true" || value == "false" => value.to_string(),
            None => return Err(err(line_no, format!("valeur inattendue : {value}"))),
        };
        if fields.insert(key.trim().to_string(), value).is_some() {
            return Err(err(line_no, format!("{name} : champ `{}` en double", key.trim())));
        }
        current = Some((name, fields, start_line));
    }
    Err(err(first_line, "fin de l'objet `themes` introuvable"))
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-data --test themes && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 8 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-data
git commit -m "feat(data): couleurs des thèmes et lecture stricte de themes.ts"
```

---

### Task 4 : langues et citations (lecture et validation)

**Files:**
- Create: `crates/fasttype-data/src/language.rs`
- Modify: `crates/fasttype-data/src/lib.rs` (modules `language` et type `DataError`)
- Test: `crates/fasttype-data/tests/language.rs`

**Interfaces:**
- Consumes : `fasttype_core::quote::QuoteFile`.
- Produces, dans `fasttype_data` :
  - `DataError { UnknownLanguage(String), Corrupt(String), Json { name: String, message: String }, Invalid { name: String, reason: String } }` (`Display`, `Error`, `Clone`, `PartialEq`).
- Produces, dans `fasttype_data::language` :
  - `LanguageFile { name, words: Vec<String>, right_to_left, no_lazy_mode, ordered_by_frequency, original_punctuation: bool, bcp47: Option<String>, additional_accents: Vec<(String, String)> }` ;
  - `Language { name, words: Arc<Vec<String>>, right_to_left, no_lazy_mode, ordered_by_frequency, original_punctuation, bcp47, additional_accents }`, avec `From<LanguageFile>` ;
  - `parse_language(expected_name: &str, json: &[u8]) -> Result<LanguageFile, DataError>` ;
  - `parse_quotes(expected_name: &str, json: &[u8]) -> Result<QuoteFile, DataError>`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-data/tests/language.rs` :
```rust
use fasttype_data::DataError;
use fasttype_data::language::{Language, parse_language, parse_quotes};

const EN: &[u8] = br#"{"name":"english","orderedByFrequency":true,"noLazyMode":true,"_comment":"x","preferredFont":"Roboto","words":["the","be","of"]}"#;

#[test]
fn parses_language_and_ignores_unused_fields() {
    let file = parse_language("english", EN).unwrap();
    assert_eq!(file.words, ["the", "be", "of"]);
    assert!(file.ordered_by_frequency && file.no_lazy_mode && !file.right_to_left);
    let lang = Language::from(file);
    assert_eq!(lang.words.len(), 3);
}

#[test]
fn parses_additional_accents() {
    let json = br#"{"name":"vietnamese","additionalAccents":[["áà","a"],["đ","d"]],"words":["a"]}"#;
    let file = parse_language("vietnamese", json).unwrap();
    assert_eq!(file.additional_accents, [("áà".to_string(), "a".to_string()), ("đ".to_string(), "d".to_string())]);
}

#[test]
fn name_must_match_file() {
    assert!(matches!(parse_language("french", EN), Err(DataError::Invalid { .. })));
}

#[test]
fn empty_or_blank_words_are_invalid() {
    let empty = br#"{"name":"x","words":[]}"#;
    assert!(matches!(parse_language("x", empty), Err(DataError::Invalid { .. })));
    let blank = br#"{"name":"x","words":["a","  "]}"#;
    let e = parse_language("x", blank).unwrap_err();
    assert!(e.to_string().contains("mot vide"), "{e}");
}

#[test]
fn broken_json_names_the_file() {
    let e = parse_language("english", b"{").unwrap_err();
    assert!(matches!(&e, DataError::Json { name, .. } if name == "english"));
}

#[test]
fn quotes_are_validated() {
    let ok = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"hi there","source":"a","length":8,"id":1}]}"#;
    assert_eq!(parse_quotes("english", ok).unwrap().quotes.len(), 1);
    let dup = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"a","source":"a","length":1,"id":1},{"text":"b","source":"b","length":1,"id":1}]}"#;
    assert!(parse_quotes("english", dup).unwrap_err().to_string().contains("en double"));
    let blank = br#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],
        "quotes":[{"text":"  ","source":"a","length":2,"id":1}]}"#;
    assert!(parse_quotes("english", blank).is_err());
    assert!(parse_quotes("french", ok).is_err(), "langue du fichier différente");
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-data --test language`
Expected: échec de compilation (`unresolved import fasttype_data::DataError`).

- [ ] **Step 3 : implémenter**

Remplacer `crates/fasttype-data/src/lib.rs` par :
```rust
//! Données de Monkeytype embarquées : langues, citations et thèmes.
//! Aucune E/S à l'exécution ; `xtask` produit les fichiers de `assets/`.

use std::fmt;

pub mod language;
pub mod pack;
pub mod themes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    UnknownLanguage(String),
    /// Données embarquées illisibles (pack ou index).
    Corrupt(String),
    Json { name: String, message: String },
    Invalid { name: String, reason: String },
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::UnknownLanguage(name) => write!(f, "langue inconnue : {name}"),
            DataError::Corrupt(why) => write!(f, "données embarquées illisibles : {why}"),
            DataError::Json { name, message } => write!(f, "{name} : JSON invalide ({message})"),
            DataError::Invalid { name, reason } => write!(f, "{name} : {reason}"),
        }
    }
}

impl std::error::Error for DataError {}
```

`crates/fasttype-data/src/language.rs` :
```rust
//! Listes de mots (`frontend/static/languages/<nom>.json`) et fichiers de
//! citations (`frontend/static/quotes/<langue>.json`), avec leur validation.

use crate::DataError;
use fasttype_core::quote::QuoteFile;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

/// Schéma `LanguageObjectSchema` ; les champs d'affichage du site sont ignorés.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageFile {
    pub name: String,
    pub words: Vec<String>,
    #[serde(default)]
    pub right_to_left: bool,
    #[serde(default)]
    pub no_lazy_mode: bool,
    #[serde(default)]
    pub ordered_by_frequency: bool,
    #[serde(default)]
    pub original_punctuation: bool,
    #[serde(default)]
    pub bcp47: Option<String>,
    /// `[caractères accentués, remplacement]` pour le lazy mode.
    #[serde(default)]
    pub additional_accents: Vec<(String, String)>,
}

/// Langue chargée ; la liste de mots est partagée sans copie.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Language {
    pub name: String,
    pub words: Arc<Vec<String>>,
    pub right_to_left: bool,
    pub no_lazy_mode: bool,
    pub ordered_by_frequency: bool,
    pub original_punctuation: bool,
    pub bcp47: Option<String>,
    pub additional_accents: Vec<(String, String)>,
}

impl From<LanguageFile> for Language {
    fn from(f: LanguageFile) -> Self {
        Self {
            name: f.name,
            words: Arc::new(f.words),
            right_to_left: f.right_to_left,
            no_lazy_mode: f.no_lazy_mode,
            ordered_by_frequency: f.ordered_by_frequency,
            original_punctuation: f.original_punctuation,
            bcp47: f.bcp47,
            additional_accents: f.additional_accents,
        }
    }
}

fn invalid(name: &str, reason: impl Into<String>) -> DataError {
    DataError::Invalid { name: name.to_string(), reason: reason.into() }
}

fn json_error(name: &str, e: serde_json::Error) -> DataError {
    DataError::Json { name: name.to_string(), message: e.to_string() }
}

/// Lit et valide une liste de mots : nom égal au fichier, au moins un mot,
/// aucun mot vide.
pub fn parse_language(expected_name: &str, json: &[u8]) -> Result<LanguageFile, DataError> {
    let file: LanguageFile = serde_json::from_slice(json).map_err(|e| json_error(expected_name, e))?;
    if file.name != expected_name {
        return Err(invalid(expected_name, format!("le champ name vaut « {} »", file.name)));
    }
    if file.words.is_empty() {
        return Err(invalid(expected_name, "aucun mot"));
    }
    if let Some(i) = file.words.iter().position(|w| w.trim().is_empty()) {
        return Err(invalid(expected_name, format!("mot vide à l'index {i}")));
    }
    Ok(file)
}

/// Lit et valide un fichier de citations : langue égale au fichier, quatre
/// groupes, textes non vides, identifiants uniques.
pub fn parse_quotes(expected_name: &str, json: &[u8]) -> Result<QuoteFile, DataError> {
    let file = QuoteFile::from_json(json).map_err(|e| json_error(expected_name, e))?;
    if file.language != expected_name {
        return Err(invalid(expected_name, format!("le champ language vaut « {} »", file.language)));
    }
    if file.groups.len() != 4 {
        return Err(invalid(expected_name, format!("{} groupes au lieu de 4", file.groups.len())));
    }
    let mut ids = HashSet::new();
    for q in &file.quotes {
        if q.text.trim().is_empty() {
            return Err(invalid(expected_name, format!("citation {} vide", q.id)));
        }
        if !ids.insert(q.id) {
            return Err(invalid(expected_name, format!("identifiant {} en double", q.id)));
        }
    }
    Ok(file)
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-data && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous les tests de `fasttype-data` PASS (8 + 8 + 6).

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-data
git commit -m "feat(data): lecture et validation des langues et des citations"
```

---

### Task 5 : xtask (construction des assets)

**Files:**
- Modify: `Cargo.toml` (membre `xtask`)
- Create: `.cargo/config.toml`, `xtask/Cargo.toml`, `xtask/src/lib.rs`, `xtask/src/main.rs`
- Test: `xtask/tests/build.rs`

**Interfaces:**
- Consumes : `fasttype_data::{pack::PackWriter, themes::parse_themes_ts, language::{parse_language, parse_quotes}}`.
- Produces :
  - `xtask::Summary { languages: usize, quotes: usize, themes: usize }` ;
  - `xtask::Manifest { source: String, rev: String, languages: usize, quotes: usize, themes: usize, files: Vec<FileHash> }` et `xtask::FileHash { path: String, sha256: String }` (serde, TOML) ;
  - `xtask::build_assets(source: &Path, out: &Path, rev: &str) -> Result<Summary, String>` ;
  - `xtask::fetch_source(rev: &str, dest: &Path) -> Result<(), String>`.
- Le binaire `xtask` s'appelle avec `cargo xtask fetch-data [--rev SHA] [--source DIR] [--out DIR]`.
- Fichiers produits dans `out/` : `languages.pack`, `quotes.pack`, `themes.json` et `manifest.toml`. Ils sont d'abord écrits dans `out.tmp/`, puis renommés : l'ancien `out/` reste intact en cas d'échec.

- [ ] **Step 1 : déclarer la crate et l'alias**

- `Cargo.toml` : `members = ["crates/fasttype-core", "crates/fasttype-data", "xtask"]`.
- `.cargo/config.toml` :
```toml
[alias]
xtask = "run --package xtask --release --"
```
- `xtask/Cargo.toml` :
```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true
publish = false

[dependencies]
fasttype-data.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
toml.workspace = true
```

- [ ] **Step 2 : écrire les tests qui échouent**

`xtask/tests/build.rs` :
```rust
use fasttype_data::pack::Pack;
use std::fs;
use std::path::{Path, PathBuf};
use xtask::{Manifest, build_assets};

const THEMES_TS: &str = r##"export const themes: Record<ThemeName, Theme> = {
  serika_dark: {
    bg: "#323437",
    main: "#e2b714",
    caret: "#e2b714",
    sub: "#646669",
    subAlt: "#2c2e31",
    text: "#d1d0c5",
    error: "#ca4754",
    errorExtra: "#7e2a33",
    colorfulError: "#ca4754",
    colorfulErrorExtra: "#7e2a33",
  },
};
"##;

/// Répertoire temporaire propre au test (pas de dépendance `tempfile`).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fasttype-xtask-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fixture(root: &Path) {
    let st = root.join("frontend/static");
    write(&st.join("languages/english.json"), r#"{"name":"english","words":["the","be","of"]}"#);
    write(&st.join("languages/korean.json"), r#"{"name":"korean","words":["한국","사람"]}"#);
    write(
        &st.join("quotes/english.json"),
        r#"{"language":"english","groups":[[0,100],[101,300],[301,600],[601,9999]],"quotes":[{"text":"hi","source":"s","length":2,"id":1}]}"#,
    );
    write(&root.join("frontend/src/ts/constants/themes.ts"), THEMES_TS);
    write(&root.join("LICENSE"), "GPL-3.0 (fixture)\n");
}

#[test]
fn builds_packs_themes_and_manifest() {
    let dir = scratch("ok");
    fixture(&dir.join("src"));
    let out = dir.join("assets");
    let summary = build_assets(&dir.join("src"), &out, "abc123").unwrap();
    assert_eq!((summary.languages, summary.quotes, summary.themes), (2, 1, 1));

    let langs = fs::read(out.join("languages.pack")).unwrap();
    let pack = Pack::parse(&langs).unwrap();
    let names: Vec<&str> = pack.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["english", "korean"]);
    assert!(String::from_utf8(pack.decompress("korean").unwrap().unwrap()).unwrap().contains("한국"));

    let themes: serde_json::Value = serde_json::from_slice(&fs::read(out.join("themes.json")).unwrap()).unwrap();
    assert_eq!(themes[0]["name"], "serika_dark");

    let manifest: Manifest = toml::from_str(&fs::read_to_string(out.join("manifest.toml")).unwrap()).unwrap();
    assert_eq!(manifest.rev, "abc123");
    assert_eq!(manifest.files.len(), 4);
    assert!(manifest.files.iter().all(|f| f.sha256.len() == 64));
    assert!(!dir.join("assets.tmp").exists());
}

#[test]
fn build_is_reproducible() {
    let dir = scratch("repro");
    fixture(&dir.join("src"));
    build_assets(&dir.join("src"), &dir.join("a"), "r").unwrap();
    build_assets(&dir.join("src"), &dir.join("b"), "r").unwrap();
    for f in ["languages.pack", "quotes.pack", "themes.json", "manifest.toml"] {
        assert_eq!(fs::read(dir.join("a").join(f)).unwrap(), fs::read(dir.join("b").join(f)).unwrap(), "{f}");
    }
}

#[test]
fn invalid_source_leaves_previous_assets_untouched() {
    let dir = scratch("bad");
    fixture(&dir.join("src"));
    let out = dir.join("assets");
    build_assets(&dir.join("src"), &out, "r").unwrap();
    let before = fs::read(out.join("languages.pack")).unwrap();

    write(&dir.join("src/frontend/static/languages/broken.json"), r#"{"name":"other","words":["a"]}"#);
    let err = build_assets(&dir.join("src"), &out, "r").unwrap_err();
    assert!(err.contains("broken"), "{err}");
    assert_eq!(fs::read(out.join("languages.pack")).unwrap(), before);
    assert!(!dir.join("assets.tmp").exists());
}

#[test]
fn missing_themes_file_is_reported() {
    let dir = scratch("nothemes");
    fixture(&dir.join("src"));
    fs::remove_file(dir.join("src/frontend/src/ts/constants/themes.ts")).unwrap();
    let err = build_assets(&dir.join("src"), &dir.join("assets"), "r").unwrap_err();
    assert!(err.contains("themes.ts"), "{err}");
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p xtask --test build`
Expected: échec de compilation (`unresolved import xtask::Manifest`, ou crate `xtask` introuvable tant que `src/lib.rs` n'existe pas).

- [ ] **Step 4 : implémenter `xtask/src/lib.rs`**

```rust
//! Construction de `assets/` à partir du dépôt Monkeytype, à un commit figé.

use fasttype_data::language::{parse_language, parse_quotes};
use fasttype_data::pack::PackWriter;
use fasttype_data::themes::parse_themes_ts;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const UPSTREAM: &str = "https://github.com/monkeytypegame/monkeytype";
const LANGUAGES_DIR: &str = "frontend/static/languages";
const QUOTES_DIR: &str = "frontend/static/quotes";
const THEMES_TS: &str = "frontend/src/ts/constants/themes.ts";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub languages: usize,
    pub quotes: usize,
    pub themes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub source: String,
    pub rev: String,
    pub languages: usize,
    pub quotes: usize,
    pub themes: usize,
    /// Empreinte de chaque fichier produit dans `assets/`.
    pub files: Vec<FileHash>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Fichiers `*.json` d'un dossier, triés par nom (pack reproductible).
fn json_files(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|x| x == "json") {
            let stem = path.file_stem().and_then(|s| s.to_str()).ok_or("nom de fichier non UTF-8")?.to_string();
            files.push((stem, path));
        }
    }
    files.sort();
    Ok(files)
}

/// Lit, valide et compresse tous les fichiers d'un dossier dans un pack.
fn build_pack(
    dir: &Path,
    validate: impl Fn(&str, &[u8]) -> Result<(), String>,
) -> Result<(Vec<u8>, usize), String> {
    let mut writer = PackWriter::new();
    let files = json_files(dir)?;
    for (name, path) in &files {
        let bytes = fs::read(path).map_err(|e| format!("{} : {e}", path.display()))?;
        validate(name, &bytes).map_err(|e| format!("{} : {e}", path.display()))?;
        writer.add(name, &bytes).map_err(|e| e.to_string())?;
    }
    Ok((writer.finish(), files.len()))
}

/// Construit `out/` depuis une copie de Monkeytype. Tout est écrit dans
/// `out.tmp/` puis renommé : en cas d'erreur, `out/` n'est pas touché.
pub fn build_assets(source: &Path, out: &Path, rev: &str) -> Result<Summary, String> {
    let (languages, n_languages) = build_pack(&source.join(LANGUAGES_DIR), |name, bytes| {
        parse_language(name, bytes).map(|_| ()).map_err(|e| e.to_string())
    })?;
    let (quotes, n_quotes) = build_pack(&source.join(QUOTES_DIR), |name, bytes| {
        parse_quotes(name, bytes).map(|_| ()).map_err(|e| e.to_string())
    })?;
    let themes_path = source.join(THEMES_TS);
    let themes_src =
        fs::read_to_string(&themes_path).map_err(|e| format!("{} : {e}", themes_path.display()))?;
    let themes = parse_themes_ts(&themes_src).map_err(|e| e.to_string())?;
    let mut themes_json = serde_json::to_vec_pretty(&themes).map_err(|e| e.to_string())?;
    themes_json.push(b'\n');

    let outputs: [(&str, &[u8]); 3] =
        [("languages.pack", &languages), ("quotes.pack", &quotes), ("themes.json", &themes_json)];
    let manifest = Manifest {
        source: UPSTREAM.to_string(),
        rev: rev.to_string(),
        languages: n_languages,
        quotes: n_quotes,
        themes: themes.len(),
        files: outputs
            .iter()
            .map(|(path, bytes)| FileHash { path: path.to_string(), sha256: sha256_hex(bytes) })
            .chain(std::iter::once(FileHash { path: "LICENSE (amont)".into(), sha256: license_hash(source) }))
            .collect(),
    };
    let manifest_toml = toml::to_string(&manifest).map_err(|e| e.to_string())?;

    let tmp = out.with_extension("tmp");
    let _ = fs::remove_dir_all(&tmp);
    let write_all = || -> std::io::Result<()> {
        fs::create_dir_all(&tmp)?;
        for (name, bytes) in outputs {
            fs::write(tmp.join(name), bytes)?;
        }
        fs::write(tmp.join("manifest.toml"), &manifest_toml)
    };
    if let Err(e) = write_all() {
        let _ = fs::remove_dir_all(&tmp);
        return Err(format!("écriture de {} : {e}", tmp.display()));
    }
    if out.exists() {
        fs::remove_dir_all(out).map_err(|e| format!("{} : {e}", out.display()))?;
    }
    fs::rename(&tmp, out).map_err(|e| format!("{} : {e}", out.display()))?;
    Ok(Summary { languages: n_languages, quotes: n_quotes, themes: themes.len() })
}

/// Empreinte du texte de licence amont (vide si absent, ex. dans les tests).
fn license_hash(source: &Path) -> String {
    fs::read(source.join("LICENSE")).map(|b| sha256_hex(&b)).unwrap_or_default()
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git introuvable : {e}"))?;
    if !out.status.success() {
        return Err(format!("git {} : {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Récupère uniquement les fichiers utiles de Monkeytype, au commit `rev`
/// (clone partiel et sparse checkout). Ne refait rien si `dest` y est déjà.
pub fn fetch_source(rev: &str, dest: &Path) -> Result<(), String> {
    if dest.join(".git").exists() && git(dest, &["rev-parse", "HEAD"]).is_ok_and(|h| h == rev) {
        return Ok(());
    }
    let _ = fs::remove_dir_all(dest);
    fs::create_dir_all(dest).map_err(|e| format!("{} : {e}", dest.display()))?;
    git(dest, &["init", "--quiet"])?;
    git(dest, &["remote", "add", "origin", &format!("{UPSTREAM}.git")])?;
    git(
        dest,
        &["sparse-checkout", "set", "--no-cone", "/LICENSE", "/frontend/static/languages/", "/frontend/static/quotes/", "/frontend/src/ts/constants/themes.ts"],
    )?;
    git(dest, &["fetch", "--quiet", "--depth", "1", "--filter=blob:none", "origin", rev])?;
    git(dest, &["checkout", "--quiet", "--detach", "FETCH_HEAD"])?;
    let head = git(dest, &["rev-parse", "HEAD"])?;
    if head != rev {
        return Err(format!("commit obtenu {head} au lieu de {rev}"));
    }
    Ok(())
}
```

- [ ] **Step 5 : implémenter `xtask/src/main.rs`**

```rust
//! `cargo xtask fetch-data [--rev SHA] [--source DIR] [--out DIR]`

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use xtask::{build_assets, fetch_source};

/// Commit de référence de Monkeytype (voir la spec et NOTICE).
const DEFAULT_REV: &str = "574d8193498f75d13e7296584c82b64cebe2efea";

fn usage() -> ExitCode {
    eprintln!("usage : cargo xtask fetch-data [--rev SHA] [--source DOSSIER] [--out DOSSIER]");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("fetch-data") {
        return usage();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("racine du workspace").to_path_buf();
    let mut rev = DEFAULT_REV.to_string();
    let mut source: Option<PathBuf> = None;
    let mut out = root.join("assets");
    let mut it = args[1..].iter();
    while let Some(flag) = it.next() {
        let Some(value) = it.next() else {
            return usage();
        };
        match flag.as_str() {
            "--rev" => rev = value.clone(),
            "--source" => source = Some(PathBuf::from(value)),
            "--out" => out = PathBuf::from(value),
            _ => return usage(),
        }
    }
    let source = match source {
        Some(s) => s,
        None => {
            let dest = root.join("target").join(format!("monkeytype-{rev}"));
            eprintln!("récupération de Monkeytype @ {rev}…");
            if let Err(e) = fetch_source(&rev, &dest) {
                eprintln!("erreur : {e}");
                return ExitCode::FAILURE;
            }
            dest
        }
    };
    match build_assets(&source, &out, &rev) {
        Ok(s) => {
            println!("{} langues, {} fichiers de citations, {} thèmes → {}", s.languages, s.quotes, s.themes, out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 6 : lancer les tests**

Run: `cargo fmt && cargo test -p xtask && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 4 tests PASS. (`fetch_source` n'est pas testé unitairement, car il faut le réseau ; il sert à la tâche 6.)

- [ ] **Step 7 : commit**

```bash
git add Cargo.toml Cargo.lock .cargo xtask
git commit -m "feat(xtask): construction reproductible des assets depuis Monkeytype"
```

---

### Task 6 : générer et versionner les vraies données

**Files:**
- Create: `assets/languages.pack`, `assets/quotes.pack`, `assets/themes.json`, `assets/manifest.toml`, `.gitattributes`

**Interfaces:**
- Produces : `assets/` complet. La tâche 7 l'embarque.

- [ ] **Step 1 : générer**

Run: `cargo xtask fetch-data`
Expected: `446 langues, 87 fichiers de citations, 187 thèmes → …/assets`.

Si GitHub est injoignable, une copie locale au même commit peut servir : `cargo xtask fetch-data --source <copie de monkeytype>`. Vérifier d'abord que `git -C <copie> rev-parse HEAD` affiche `574d8193498f75d13e7296584c82b64cebe2efea`. Noter dans le ledger la source utilisée.

- [ ] **Step 2 : vérifier tailles et contenu**

Run: `du -sh assets/* && head -12 assets/manifest.toml && grep -c '"name"' assets/themes.json`
Expected :
- `languages.pack` vers 21 Mo et `quotes.pack` vers 2 Mo, à 15 % près des mesures préalables ;
- le manifeste donne `rev = "574d8193498f75d13e7296584c82b64cebe2efea"`, `languages = 446`, `quotes = 87` et `themes = 187` ;
- `grep -c` affiche 187.

- [ ] **Step 3 : vérifier la reproductibilité**

Run: `cp -r assets target/assets-check && cargo xtask fetch-data && for f in assets/*; do cmp "$f" "target/assets-check/$(basename $f)" || echo "DIFF $f"; done; rm -rf target/assets-check`
Expected: aucune ligne `DIFF`.

- [ ] **Step 4 : déclarer les packs comme binaires et committer**

`.gitattributes` :
```
assets/*.pack binary
```

```bash
git add .gitattributes assets
git commit -m "data: assets Monkeytype @ 574d819 (446 langues, 87 citations, 187 thèmes)"
```

---

### Task 7 : catalogue embarqué (API publique)

**Files:**
- Create: `crates/fasttype-data/src/catalog.rs`, `crates/fasttype-data/benches/catalog.rs`
- Modify: `crates/fasttype-data/src/lib.rs` (module `catalog` et réexports), `crates/fasttype-data/Cargo.toml` (bench)
- Test: `crates/fasttype-data/tests/catalog.rs`

**Interfaces:**
- Consumes : `Pack`, `parse_language`, `parse_quotes`, `Theme`, `fasttype_core::result::remove_language_size`.
- Produces, réexportés à la racine de `fasttype_data` :
  - `language_names() -> Result<Vec<&'static str>, DataError>` (triés) ;
  - `load_language(name: &str) -> Result<Language, DataError>` ;
  - `quote_file_names() -> Result<Vec<&'static str>, DataError>` ;
  - `quotes_for(language: &str) -> Result<Option<QuoteFile>, DataError>` ;
  - `themes() -> Result<&'static [Theme], DataError>` et `theme(name: &str) -> Option<&'static Theme>` ;
  - `DEFAULT_LANGUAGE = "english"` et `DEFAULT_THEME = "serika_dark"` ;
  - `LanguageCache::{new(), get(&self, name: &str) -> Result<Arc<Language>, DataError>}` (`Send + Sync`).

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-data/tests/catalog.rs` :
```rust
use fasttype_core::chars::contains_korean;
use fasttype_core::result::remove_language_size;
use fasttype_data::{
    DEFAULT_LANGUAGE, DEFAULT_THEME, DataError, LanguageCache, language_names, load_language, quote_file_names,
    quotes_for, theme, themes,
};
use std::sync::Arc;

#[test]
fn counts_match_monkeytype() {
    assert_eq!(language_names().unwrap().len(), 446);
    assert_eq!(quote_file_names().unwrap().len(), 87);
    assert_eq!(themes().unwrap().len(), 187);
}

#[test]
fn default_language_and_theme_exist() {
    let english = load_language(DEFAULT_LANGUAGE).unwrap();
    assert_eq!(english.words.len(), 200);
    assert_eq!(english.words[0], "the");
    let serika = theme(DEFAULT_THEME).unwrap();
    assert_eq!(serika.bg.to_hex(), "#323437");
    assert_eq!(serika.main.to_hex(), "#e2b714");
    assert_eq!(serika.error.to_hex(), "#ca4754");
}

#[test]
fn unknown_language_is_an_error() {
    assert_eq!(load_language("klingon_9000k"), Err(DataError::UnknownLanguage("klingon_9000k".into())));
    assert!(theme("does_not_exist").is_none());
}

#[test]
fn quotes_follow_language_without_size() {
    let q = quotes_for("english_1k").unwrap().unwrap();
    assert_eq!(q.language, "english");
    assert!(q.quotes.len() > 6000);
}

#[test]
fn language_without_quotes_returns_none() {
    let quote_names = quote_file_names().unwrap();
    let without = language_names()
        .unwrap()
        .into_iter()
        .find(|n| !quote_names.contains(&remove_language_size(n).as_str()))
        .expect("au moins une langue sans citations");
    assert_eq!(quotes_for(without).unwrap(), None);
}

#[test]
fn every_language_and_quote_file_loads() {
    for name in language_names().unwrap() {
        let lang = load_language(name).unwrap_or_else(|e| panic!("{e}"));
        assert!(!lang.words.is_empty(), "{name}");
    }
    for name in quote_file_names().unwrap() {
        assert!(quotes_for(name).unwrap().is_some(), "{name}");
    }
    assert!(load_language("korean").unwrap().words.iter().any(|w| contains_korean(w)));
    assert_eq!(load_language("english_450k").unwrap().words.len() > 400_000, true);
}

#[test]
fn cache_is_shared_across_threads() {
    let cache = Arc::new(LanguageCache::new());
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let cache = Arc::clone(&cache);
            std::thread::spawn(move || cache.get("english_1k").unwrap())
        })
        .collect();
    let langs: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(langs.windows(2).all(|w| Arc::ptr_eq(&w[0], &w[1])));
    assert!(matches!(cache.get("nope"), Err(DataError::UnknownLanguage(_))));
}
```

Note : `quotes_for(language)` renvoie `Option<QuoteFile>`, et la comparaison `== None` demande `QuoteFile: PartialEq`, ce qui est déjà le cas dans `fasttype-core`.

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-data --test catalog`
Expected: échec de compilation (`unresolved imports fasttype_data::DEFAULT_LANGUAGE, ...`).

- [ ] **Step 3 : implémenter `catalog.rs`**

```rust
//! Données embarquées dans le binaire. Seul l'index des packs est lu au
//! premier accès ; chaque langue est décompressée à la demande.

use crate::DataError;
use crate::language::{Language, parse_language, parse_quotes};
use crate::pack::Pack;
use crate::themes::Theme;
use fasttype_core::quote::QuoteFile;
use fasttype_core::result::remove_language_size;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

static LANGUAGES_PACK: &[u8] = include_bytes!("../../../assets/languages.pack");
static QUOTES_PACK: &[u8] = include_bytes!("../../../assets/quotes.pack");
static THEMES_JSON: &str = include_str!("../../../assets/themes.json");

pub const DEFAULT_LANGUAGE: &str = "english";
pub const DEFAULT_THEME: &str = "serika_dark";

fn parsed_pack(cell: &'static OnceLock<Result<Pack<'static>, String>>, bytes: &'static [u8]) -> Result<&'static Pack<'static>, DataError> {
    cell.get_or_init(|| Pack::parse(bytes).map_err(|e| e.to_string()))
        .as_ref()
        .map_err(|e| DataError::Corrupt(e.clone()))
}

fn languages_pack() -> Result<&'static Pack<'static>, DataError> {
    static CELL: OnceLock<Result<Pack<'static>, String>> = OnceLock::new();
    parsed_pack(&CELL, LANGUAGES_PACK)
}

fn quotes_pack() -> Result<&'static Pack<'static>, DataError> {
    static CELL: OnceLock<Result<Pack<'static>, String>> = OnceLock::new();
    parsed_pack(&CELL, QUOTES_PACK)
}

fn names(pack: &'static Pack<'static>) -> Vec<&'static str> {
    pack.entries().iter().map(|e| e.name.as_str()).collect()
}

/// Noms de toutes les langues, triés.
pub fn language_names() -> Result<Vec<&'static str>, DataError> {
    languages_pack().map(names)
}

/// Noms des fichiers de citations (langues sans suffixe de taille), triés.
pub fn quote_file_names() -> Result<Vec<&'static str>, DataError> {
    quotes_pack().map(names)
}

/// Décompresse et valide une langue. Pour les grosses listes (jusqu'à 12 Mo),
/// l'appelant fait cet appel hors du fil de l'interface.
pub fn load_language(name: &str) -> Result<Language, DataError> {
    let bytes = languages_pack()?
        .decompress(name)
        .map_err(|e| DataError::Corrupt(format!("{name} : {e}")))?
        .ok_or_else(|| DataError::UnknownLanguage(name.to_string()))?;
    parse_language(name, &bytes).map(Language::from)
}

/// Citations de la langue (`english_1k` → `english`) ; `None` s'il n'y en a pas.
pub fn quotes_for(language: &str) -> Result<Option<QuoteFile>, DataError> {
    let base = remove_language_size(language);
    match quotes_pack()?.decompress(&base).map_err(|e| DataError::Corrupt(format!("{base} : {e}")))? {
        Some(bytes) => parse_quotes(&base, &bytes).map(Some),
        None => Ok(None),
    }
}

/// Les 187 thèmes, triés par nom.
pub fn themes() -> Result<&'static [Theme], DataError> {
    static CELL: OnceLock<Result<Vec<Theme>, String>> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(THEMES_JSON).map_err(|e| e.to_string()))
        .as_deref()
        .map_err(|e| DataError::Corrupt(format!("themes.json : {e}")))
}

pub fn theme(name: &str) -> Option<&'static Theme> {
    themes().ok()?.iter().find(|t| t.name == name)
}

/// Langues déjà chargées, partagées entre threads. Une langue n'est décompressée
/// qu'une fois, même si plusieurs threads la demandent en même temps.
#[derive(Default)]
pub struct LanguageCache {
    loaded: Mutex<HashMap<String, Arc<OnceLock<Result<Arc<Language>, DataError>>>>>,
}

impl LanguageCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Result<Arc<Language>, DataError> {
        // Le verrou ne couvre que la table : la décompression se fait hors verrou.
        let slot = {
            let mut loaded = self.loaded.lock().unwrap_or_else(|p| p.into_inner());
            Arc::clone(loaded.entry(name.to_string()).or_default())
        };
        slot.get_or_init(|| load_language(name).map(Arc::new)).clone()
    }
}
```

Remplacer la ligne `pub mod language;` de `lib.rs` par :
```rust
mod catalog;
pub mod language;
```
et ajouter à la fin de `lib.rs` :
```rust
pub use catalog::{
    DEFAULT_LANGUAGE, DEFAULT_THEME, LanguageCache, language_names, load_language, quote_file_names, quotes_for, theme,
    themes,
};
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-data --test catalog`
Expected: 7 tests PASS. `every_language_and_quote_file_loads` décompresse 135 Mo : il prend quelques secondes grâce à `opt-level = 2` en dev. S'il dépasse 60 s, vérifier que la section `[profile.dev.package.fasttype-data]` de la tâche 2 est bien en place.

- [ ] **Step 5 : benchmark du chargement**

Ajouter à `crates/fasttype-data/Cargo.toml` :
```toml
[dev-dependencies]
criterion.workspace = true

[[bench]]
name = "catalog"
harness = false
```

`crates/fasttype-data/benches/catalog.rs` :
```rust
use criterion::{Criterion, criterion_group, criterion_main};
use fasttype_data::pack::Pack;
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/languages.pack")).unwrap();
    c.bench_function("parse_language_index", |b| b.iter(|| black_box(Pack::parse(&bytes).unwrap().entries().len())));
    c.bench_function("load_english", |b| b.iter(|| black_box(fasttype_data::load_language("english").unwrap())));
    let mut slow = c.benchmark_group("big");
    slow.sample_size(10);
    slow.bench_function("load_english_450k", |b| {
        b.iter(|| black_box(fasttype_data::load_language("english_450k").unwrap()))
    });
    slow.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
```

Run: `cargo bench -p fasttype-data --bench catalog 2>&1 | grep -E "time:"`
Expected :
- `parse_language_index` sous 1 ms : c'est le seul travail fait au démarrage, la spec visant moins de 50 ms en tout ;
- `load_english` sous 1 ms ;
- `load_english_450k` sous 300 ms, d'où un chargement en arrière-plan dans la TUI.

Recopier les médianes dans le message de commit. Si un seuil est dépassé, chercher la cause (profilage) au lieu de remonter le seuil.

- [ ] **Step 6 : vérification finale du workspace**

Run: `cargo fmt && cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, avec le total de passed relevé dans le ledger, aucun avertissement.

- [ ] **Step 7 : commit**

```bash
git add crates/fasttype-data
git commit -m "feat(data): catalogue embarqué des langues, citations et thèmes

parse_language_index : <médiane> ; load_english : <médiane> ; load_english_450k : <médiane>"
```
