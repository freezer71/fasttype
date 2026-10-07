# fasttype-store — plan d'implémentation (plan 3 sur 4)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tout ce que fasttype garde sur le disque : la config (les 94 clés de Monkeytype et leurs valeurs par défaut, en TOML), l'historique des résultats, les records personnels, les textes custom et les citations favorites.

**Architecture:**
- **Config pilotée par un schéma.** Une table `SCHEMA` décrit les 94 clés de Monkeytype : nom, groupe, libellé, type et valeurs permises, défaut. Elle a été générée depuis `packages/schemas/src/configs.ts`, `frontend/src/ts/constants/default-config.ts` et `frontend/src/ts/config/metadata.tsx`, au commit `574d819`.
- **Même schéma partout.** La validation, la lecture et l'écriture du TOML s'appuient sur ce schéma, ainsi que, plus tard, la palette de commandes de la TUI, comme sur le site.
- **Effets de bord.** Les `overrideConfig` de Monkeytype (choisir une durée passe en mode time, etc.) sont appliqués par `Config::set`.
- **Écritures sûres.** Chaque écriture est atomique (fichier temporaire puis `rename`) ou un ajout d'une seule ligne. Une erreur de lecture ne fait jamais planter : on revient aux défauts et on renvoie un avertissement, que la TUI affichera en notification.

**Tech Stack:** Rust 1.97 (édition 2024), `toml` 1.1.6, `serde` / `serde_json`, `fasttype-core` (`TestResult`, `PbKey`).

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§6.2, §7, §8, §9). Les plans 1 et 2 sont livrés sur `main`.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Commentaires en français, identifiants en anglais.
- `fasttype-store` dépend de `fasttype-core`, `serde`, `serde_json` et `toml`, jamais de `fasttype-data`. La validation des noms de langue, de thème, de layout et de police contre les catalogues revient à la TUI, qui applique le repli de §8.
- Aucune panique sur une donnée lue du disque. Une erreur d'E/S est renvoyée en `io::Result` ; un contenu illisible donne un avertissement et un repli.
- **Démarrage rapide (§7) :** `Store::open` lit seulement la config et les records personnels, jamais tout l'historique.
- Chemins XDG : `XDG_CONFIG_HOME`, sinon `~/.config`, puis `fasttype/`. `XDG_DATA_HOME`, sinon `~/.local/share`, puis `fasttype/`. Même règle sous macOS.
- Clés TOML en `snake_case` (`smooth_caret = "medium"`) ; en interne, les noms de Monkeytype en camelCase (`smoothCaret`).
- Les tests n'écrivent jamais dans le vrai `$HOME` : chaque test travaille dans son propre dossier temporaire.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace (somme de toutes les lignes `test result`), tous au vert.

## Review Focus

1. **Coupure pendant une écriture** (plantage, disque plein) : le fichier précédent reste intact ; aucun fichier n'est à moitié écrit. Tests dans les tâches 3 et 5 (`atomic_write_leaves_no_temp_file`, `pbs_are_written_atomically`).
2. **Ligne d'historique tronquée** par un plantage, puis nouveau résultat ajouté : le nouveau résultat reste lisible, au lieu d'être collé à la ligne cassée. Test dans la tâche 4 (`append_after_truncated_line_stays_readable`).
3. **Config modifiée à la main** : clé inconnue, valeur hors bornes, type faux ou TOML cassé. On garde les défauts concernés, on avertit, et un fichier cassé est mis de côté dans `config.toml.bak`. Tests dans les tâches 2 et 3.
4. **Nom de texte custom hostile** (`../x`, chemin absolu, nom vide) : refusé, aucune écriture hors du dossier. Test dans la tâche 6 (`hostile_names_are_refused`).
5. **Fichier de records corrompu** : `Store::open` le reconstruit depuis l'historique et le signale, sans planter. Test dans la tâche 7 (`corrupt_pbs_are_rebuilt_from_history`).

---

## Structure des fichiers

```
Cargo.toml                          + membre crates/fasttype-store
crates/fasttype-store/
├── Cargo.toml
├── src/lib.rs                      modules et réexports
├── src/schema.rs                   SCHEMA (94 clés), Kind, Group, validate, to_snake
├── src/config.rs                   Config : défauts, accès, set + overrides, TOML
├── src/paths.rs                    Paths (XDG)
├── src/fs.rs                       write_atomic, load_config, save_config
├── src/results.rs                  ResultLog (JSONL)
├── src/pbs.rs                      PersonalBests
├── src/texts.rs                    CustomTexts, FavoriteQuotes
├── src/store.rs                    Store (façade)
└── tests/
    ├── common/mod.rs               scratch(), result()
    ├── schema.rs  config.rs  files.rs  results.rs  pbs.rs  texts.rs  store.rs
```

---

### Task 1 : crate fasttype-store et schéma des 94 clés

**Files:**
- Modify: `Cargo.toml` (membre et dépendance de workspace)
- Create: `crates/fasttype-store/Cargo.toml`, `crates/fasttype-store/src/lib.rs`, `crates/fasttype-store/src/schema.rs`
- Test: `crates/fasttype-store/tests/schema.rs`

**Interfaces:**
- Produces, dans `fasttype_store::schema` :
  - `Group { Test, Behavior, Input, Sound, Caret, Appearance, Theme, HideElements, Hidden, Ads }`, avec `label(self) -> &'static str` ;
  - `Kind`, avec les variantes `Bool`, `Choice(&'static [&'static str])`, `Int { min: i64 }`, `Number { min: f64, max: Option<f64> }`, `Positive`, `MaxLineWidth`, `Name`, `BackgroundUrl`, `Numbers(usize)`, `NameList { min: usize, max: Option<usize> }`, `QuoteLengths`, `Colors(usize)` et `ChoiceList { values: &'static [&'static str], len: usize }` ;
  - `KeyDef { name: &'static str, group: Group, display: &'static str, kind: Kind, default: &'static str }` (le défaut est un littéral TOML), avec `default_value(&self) -> toml::Value` et `toml_key(&self) -> String` ;
  - `SCHEMA: &[KeyDef]` (94 entrées, dans l'ordre de `ConfigSchema`) ;
  - `key_def(name: &str) -> Option<&'static KeyDef>`, `key_def_by_toml(snake: &str) -> Option<&'static KeyDef>`, `to_snake(camel: &str) -> String` et `validate(kind: &Kind, value: &toml::Value) -> Result<(), String>`.

- [ ] **Step 1 : déclarer la crate**

Dans `Cargo.toml` (racine) :
- `members = ["crates/fasttype-core", "crates/fasttype-data", "crates/fasttype-store", "xtask"]` ;
- sous `[workspace.dependencies]`, ajouter `fasttype-store = { path = "crates/fasttype-store" }`.

`crates/fasttype-store/Cargo.toml` :
```toml
[package]
name = "fasttype-store"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
fasttype-core.workspace = true
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
```

`crates/fasttype-store/src/lib.rs` :
```rust
//! Persistance locale de fasttype : config, historique, records personnels,
//! textes custom et citations favorites.

pub mod schema;
```

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-store/tests/schema.rs` :
```rust
use fasttype_store::schema::{Group, Kind, SCHEMA, key_def, key_def_by_toml, to_snake, validate};
use std::collections::HashSet;
use toml::Value;

#[test]
fn has_the_94_monkeytype_keys_once() {
    assert_eq!(SCHEMA.len(), 94);
    let names: HashSet<_> = SCHEMA.iter().map(|d| d.name).collect();
    assert_eq!(names.len(), 94);
    let snakes: HashSet<_> = SCHEMA.iter().map(|d| d.toml_key()).collect();
    assert_eq!(snakes.len(), 94);
}

#[test]
fn every_default_is_valid() {
    for d in SCHEMA {
        let v = d.default_value();
        assert_eq!(validate(&d.kind, &v), Ok(()), "{} = {}", d.name, d.default);
    }
}

#[test]
fn defaults_match_monkeytype() {
    let get = |k: &str| key_def(k).unwrap().default_value();
    assert_eq!(get("mode").as_str(), Some("time"));
    assert_eq!(get("time").as_integer(), Some(30));
    assert_eq!(get("words").as_integer(), Some(50));
    assert_eq!(get("smoothCaret").as_str(), Some("medium"));
    assert_eq!(get("timerStyle").as_str(), Some("mini"));
    assert_eq!(get("theme").as_str(), Some("serika_dark"));
    assert_eq!(get("soundVolume").as_float(), Some(0.5));
    assert_eq!(get("customThemeColors").as_array().unwrap().len(), 10);
    assert_eq!(get("singleListCommandLine").as_str(), Some("on"));
    assert_eq!(get("quoteLength").as_array().unwrap()[0].as_integer(), Some(1));
}

#[test]
fn groups_and_labels() {
    assert_eq!(key_def("smoothCaret").unwrap().group, Group::Caret);
    assert_eq!(key_def("timerStyle").unwrap().display, "live progress style");
    assert_eq!(key_def("showPb").unwrap().group, Group::HideElements);
    assert_eq!(Group::HideElements.label(), "hide elements");
}

#[test]
fn snake_case_keys() {
    assert_eq!(to_snake("smoothCaret"), "smooth_caret");
    assert_eq!(to_snake("customLayoutfluid"), "custom_layoutfluid");
    assert_eq!(to_snake("minWpmCustomSpeed"), "min_wpm_custom_speed");
    assert_eq!(key_def_by_toml("quick_restart").unwrap().name, "quickRestart");
    assert!(key_def_by_toml("quickRestart").is_none());
}

fn v(src: &str) -> Value {
    toml::from_str::<toml::Table>(&format!("v = {src}")).unwrap().remove("v").unwrap()
}

#[test]
fn validation_follows_zod_schemas() {
    let ok = |k: &str, s: &str| validate(&key_def(k).unwrap().kind, &v(s)).is_ok();
    assert!(ok("smoothCaret", "\"fast\""));
    assert!(!ok("smoothCaret", "\"turbo\""));
    assert!(!ok("smoothCaret", "true"));
    assert!(ok("time", "0"));
    assert!(!ok("time", "-1"));
    assert!(!ok("time", "1.5"));
    assert!(ok("minAccCustom", "100"));
    assert!(ok("minAccCustom", "99.5"));
    assert!(!ok("minAccCustom", "101"));
    assert!(ok("tapeMargin", "10"));
    assert!(!ok("tapeMargin", "9"));
    assert!(ok("maxLineWidth", "0"));
    assert!(!ok("maxLineWidth", "10"));
    assert!(ok("maxLineWidth", "20"));
    assert!(!ok("fontSize", "0"));
    assert!(ok("quoteLength", "[-3, 0, 3]"));
    assert!(!ok("quoteLength", "[4]"));
    assert!(ok("customThemeColors", "[\"#fff\",\"#000000\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#abc\",\"#ABCDEF\"]"));
    assert!(!ok("customThemeColors", "[\"#fff\"]"));
    assert!(!ok("customPolyglot", "[\"english\"]"));
    assert!(ok("customPolyglot", "[\"english\", \"french\"]"));
    assert!(ok("accountChart", "[\"on\", \"off\", \"on\", \"on\"]"));
    assert!(!ok("accountChart", "[\"on\"]"));
    assert!(ok("customBackground", "\"\""));
    assert!(ok("customBackground", "\"https://example.com/cat.webp\""));
    assert!(!ok("customBackground", "\"ftp://example.com/cat.png\""));
    assert!(!ok("customBackground", "\"https://example.com/cat.svg\""));
    assert!(!ok("language", "\"\""));
    assert!(!ok("soundVolume", "nan"));
    assert!(matches!(key_def("customBackgroundFilter").unwrap().kind, Kind::Numbers(4)));
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test schema`
Expected: échec de compilation (`unresolved imports fasttype_store::schema::{...}`).

- [ ] **Step 4 : implémenter `schema.rs`**

```rust
//! Schéma des 94 clés de config de Monkeytype : `ConfigSchema`
//! (packages/schemas/src/configs.ts), valeurs par défaut (default-config.ts)
//! et libellés (`displayString` de config/metadata.tsx), au commit 574d819.

use toml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Test,
    Behavior,
    Input,
    Sound,
    Caret,
    Appearance,
    Theme,
    HideElements,
    Hidden,
    Ads,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Test => "test",
            Group::Behavior => "behavior",
            Group::Input => "input",
            Group::Sound => "sound",
            Group::Caret => "caret",
            Group::Appearance => "appearance",
            Group::Theme => "theme",
            Group::HideElements => "hide elements",
            Group::Hidden => "hidden",
            Group::Ads => "ads",
        }
    }
}

/// Forme des valeurs permises, traduite des schémas zod.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Bool,
    Choice(&'static [&'static str]),
    /// Entier ≥ `min`.
    Int { min: i64 },
    /// Nombre (entier ou flottant) dans `[min, max]`.
    Number { min: f64, max: Option<f64> },
    /// Nombre strictement positif.
    Positive,
    /// 0, ou un nombre entre 20 et 1000.
    MaxLineWidth,
    /// Nom non vide (langue, thème, layout, police) ; vérifié contre les catalogues par la TUI.
    Name,
    /// `""` ou URL http(s) d'image (png, gif, jpeg, jpg, webp), sans guillemets, ≤ 2048 caractères.
    BackgroundUrl,
    /// Exactement n nombres.
    Numbers(usize),
    NameList { min: usize, max: Option<usize> },
    /// Liste de longueurs de citation : -3 favoris, -2 recherche, 0 à 3.
    QuoteLengths,
    /// Exactement n couleurs `#rgb` ou `#rrggbb`.
    Colors(usize),
    /// Exactement `len` valeurs prises dans `values`.
    ChoiceList { values: &'static [&'static str], len: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyDef {
    /// Nom Monkeytype (camelCase).
    pub name: &'static str,
    pub group: Group,
    /// Libellé (`displayString`), utilisé par la palette : « Smooth caret... ».
    pub display: &'static str,
    pub kind: Kind,
    /// Valeur par défaut, en littéral TOML.
    pub default: &'static str,
}

impl KeyDef {
    pub fn default_value(&self) -> Value {
        parse_literal(self.default).unwrap_or_else(|| panic!("défaut invalide pour {} : {}", self.name, self.default))
    }

    pub fn toml_key(&self) -> String {
        to_snake(self.name)
    }
}

/// Lit un littéral TOML isolé (`"medium"`, `[1]`, `0.5`…).
pub(crate) fn parse_literal(src: &str) -> Option<Value> {
    toml::from_str::<toml::Table>(&format!("v = {src}")).ok()?.remove("v")
}

pub const SCHEMA: &[KeyDef] = &[
    KeyDef { name: "punctuation", group: Group::Test, display: "punctuation", kind: Kind::Bool, default: "false" },
    KeyDef { name: "numbers", group: Group::Test, display: "numbers", kind: Kind::Bool, default: "false" },
    KeyDef { name: "words", group: Group::Test, display: "word count", kind: Kind::Int { min: 0 }, default: "50" },
    KeyDef { name: "time", group: Group::Test, display: "time", kind: Kind::Int { min: 0 }, default: "30" },
    KeyDef { name: "mode", group: Group::Test, display: "mode", kind: Kind::Choice(&["time", "words", "quote", "custom", "zen"]), default: "\"time\"" },
    KeyDef { name: "quoteLength", group: Group::Test, display: "quote length", kind: Kind::QuoteLengths, default: "[1]" },
    KeyDef { name: "language", group: Group::Test, display: "language", kind: Kind::Name, default: "\"english\"" },
    KeyDef { name: "burstHeatmap", group: Group::Test, display: "word burst heatmap", kind: Kind::Bool, default: "false" },
    KeyDef { name: "difficulty", group: Group::Behavior, display: "difficulty", kind: Kind::Choice(&["normal", "expert", "master"]), default: "\"normal\"" },
    KeyDef { name: "quickRestart", group: Group::Behavior, display: "quick restart", kind: Kind::Choice(&["off", "esc", "tab", "enter"]), default: "\"off\"" },
    KeyDef { name: "repeatQuotes", group: Group::Behavior, display: "repeat quotes", kind: Kind::Choice(&["off", "typing"]), default: "\"off\"" },
    KeyDef { name: "resultSaving", group: Group::Behavior, display: "result saving", kind: Kind::Bool, default: "true" },
    KeyDef { name: "blindMode", group: Group::Behavior, display: "blind mode", kind: Kind::Bool, default: "false" },
    KeyDef { name: "alwaysShowWordsHistory", group: Group::Behavior, display: "always show words history", kind: Kind::Bool, default: "false" },
    KeyDef { name: "singleListCommandLine", group: Group::Behavior, display: "single list command line", kind: Kind::Choice(&["manual", "on"]), default: "\"on\"" },
    KeyDef { name: "minWpm", group: Group::Behavior, display: "min speed", kind: Kind::Choice(&["off", "custom"]), default: "\"off\"" },
    KeyDef { name: "minWpmCustomSpeed", group: Group::Behavior, display: "min speed custom", kind: Kind::Number { min: 0.0, max: None }, default: "100" },
    KeyDef { name: "minAcc", group: Group::Behavior, display: "min accuracy", kind: Kind::Choice(&["off", "custom"]), default: "\"off\"" },
    KeyDef { name: "minAccCustom", group: Group::Behavior, display: "min accuracy custom", kind: Kind::Number { min: 0.0, max: Some(100.0) }, default: "90" },
    KeyDef { name: "minBurst", group: Group::Behavior, display: "min word burst", kind: Kind::Choice(&["off", "fixed", "flex"]), default: "\"off\"" },
    KeyDef { name: "minBurstCustomSpeed", group: Group::Behavior, display: "min word burst custom speed", kind: Kind::Number { min: 0.0, max: None }, default: "100" },
    KeyDef { name: "britishEnglish", group: Group::Behavior, display: "british english", kind: Kind::Bool, default: "false" },
    KeyDef { name: "funbox", group: Group::Behavior, display: "funbox", kind: Kind::NameList { min: 0, max: Some(15) }, default: "[]" },
    KeyDef { name: "customLayoutfluid", group: Group::Behavior, display: "custom layoutfluid", kind: Kind::NameList { min: 2, max: Some(15) }, default: "[\"qwerty\", \"dvorak\", \"colemak\"]" },
    KeyDef { name: "customPolyglot", group: Group::Behavior, display: "polyglot languages", kind: Kind::NameList { min: 2, max: None }, default: "[\"english\", \"spanish\", \"french\", \"german\"]" },
    KeyDef { name: "freedomMode", group: Group::Input, display: "freedom mode", kind: Kind::Bool, default: "false" },
    KeyDef { name: "strictSpace", group: Group::Input, display: "strict space", kind: Kind::Bool, default: "false" },
    KeyDef { name: "oppositeShiftMode", group: Group::Input, display: "opposite shift mode", kind: Kind::Choice(&["off", "on", "keymap"]), default: "\"off\"" },
    KeyDef { name: "stopOnError", group: Group::Input, display: "stop on error", kind: Kind::Choice(&["off", "word", "letter"]), default: "\"off\"" },
    KeyDef { name: "deleteOnError", group: Group::Input, display: "delete on error", kind: Kind::Choice(&["off", "letter", "letter_hard", "word", "word_hard"]), default: "\"off\"" },
    KeyDef { name: "confidenceMode", group: Group::Input, display: "confidence mode", kind: Kind::Choice(&["off", "on", "max"]), default: "\"off\"" },
    KeyDef { name: "quickEnd", group: Group::Input, display: "quick end", kind: Kind::Bool, default: "false" },
    KeyDef { name: "indicateTypos", group: Group::Input, display: "indicate typos", kind: Kind::Choice(&["off", "below", "replace", "both"]), default: "\"off\"" },
    KeyDef { name: "compositionDisplay", group: Group::Input, display: "composition display", kind: Kind::Choice(&["off", "below", "replace"]), default: "\"replace\"" },
    KeyDef { name: "hideExtraLetters", group: Group::Input, display: "hide extra letters", kind: Kind::Bool, default: "false" },
    KeyDef { name: "lazyMode", group: Group::Input, display: "lazy mode", kind: Kind::Bool, default: "false" },
    KeyDef { name: "layout", group: Group::Input, display: "layout", kind: Kind::Name, default: "\"default\"" },
    KeyDef { name: "codeUnindentOnBackspace", group: Group::Input, display: "code unindent on backspace", kind: Kind::Bool, default: "false" },
    KeyDef { name: "soundVolume", group: Group::Sound, display: "sound volume", kind: Kind::Number { min: 0.0, max: Some(1.0) }, default: "0.5" },
    KeyDef { name: "playSoundOnClick", group: Group::Sound, display: "play sound on click", kind: Kind::Choice(&["off", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26"]), default: "\"off\"" },
    KeyDef { name: "playSoundOnError", group: Group::Sound, display: "play sound on error", kind: Kind::Choice(&["off", "1", "2", "3", "4"]), default: "\"off\"" },
    KeyDef { name: "playTimeWarning", group: Group::Sound, display: "play time warning", kind: Kind::Choice(&["off", "1", "3", "5", "10"]), default: "\"off\"" },
    KeyDef { name: "smoothCaret", group: Group::Caret, display: "smooth caret", kind: Kind::Choice(&["off", "slow", "medium", "fast"]), default: "\"medium\"" },
    KeyDef { name: "caretStyle", group: Group::Caret, display: "caret style", kind: Kind::Choice(&["off", "default", "block", "outline", "underline", "carrot", "banana", "monkey"]), default: "\"default\"" },
    KeyDef { name: "paceCaret", group: Group::Caret, display: "pace caret", kind: Kind::Choice(&["off", "average", "pb", "tagPb", "last", "custom", "daily"]), default: "\"off\"" },
    KeyDef { name: "paceCaretCustomSpeed", group: Group::Caret, display: "pace caret custom speed", kind: Kind::Number { min: 0.0, max: None }, default: "100" },
    KeyDef { name: "paceCaretStyle", group: Group::Caret, display: "pace caret style", kind: Kind::Choice(&["off", "default", "block", "outline", "underline", "carrot", "banana", "monkey"]), default: "\"default\"" },
    KeyDef { name: "repeatedPace", group: Group::Caret, display: "repeated pace", kind: Kind::Bool, default: "true" },
    KeyDef { name: "timerStyle", group: Group::Appearance, display: "live progress style", kind: Kind::Choice(&["off", "bar", "text", "mini", "flash_text", "flash_mini"]), default: "\"mini\"" },
    KeyDef { name: "liveSpeedStyle", group: Group::Appearance, display: "live speed style", kind: Kind::Choice(&["off", "text", "mini"]), default: "\"off\"" },
    KeyDef { name: "liveAccStyle", group: Group::Appearance, display: "live accuracy style", kind: Kind::Choice(&["off", "text", "mini"]), default: "\"off\"" },
    KeyDef { name: "liveBurstStyle", group: Group::Appearance, display: "live word burst style", kind: Kind::Choice(&["off", "text", "mini"]), default: "\"off\"" },
    KeyDef { name: "timerColor", group: Group::Appearance, display: "timer color", kind: Kind::Choice(&["black", "sub", "text", "main"]), default: "\"main\"" },
    KeyDef { name: "timerOpacity", group: Group::Appearance, display: "timer opacity", kind: Kind::Choice(&["0.25", "0.5", "0.75", "1"]), default: "\"1\"" },
    KeyDef { name: "highlightMode", group: Group::Appearance, display: "highlight mode", kind: Kind::Choice(&["off", "letter", "word", "next_word", "next_two_words", "next_three_words"]), default: "\"letter\"" },
    KeyDef { name: "typedEffect", group: Group::Appearance, display: "typed effect", kind: Kind::Choice(&["keep", "hide", "fade", "dots"]), default: "\"keep\"" },
    KeyDef { name: "tapeMode", group: Group::Appearance, display: "tape mode", kind: Kind::Choice(&["off", "letter", "word"]), default: "\"off\"" },
    KeyDef { name: "tapeMargin", group: Group::Appearance, display: "tape margin", kind: Kind::Number { min: 10.0, max: Some(90.0) }, default: "50" },
    KeyDef { name: "smoothLineScroll", group: Group::Appearance, display: "smooth line scroll", kind: Kind::Bool, default: "false" },
    KeyDef { name: "showAllLines", group: Group::Appearance, display: "show all lines", kind: Kind::Bool, default: "false" },
    KeyDef { name: "alwaysShowDecimalPlaces", group: Group::Appearance, display: "always show decimal places", kind: Kind::Bool, default: "false" },
    KeyDef { name: "typingSpeedUnit", group: Group::Appearance, display: "typing speed unit", kind: Kind::Choice(&["wpm", "cpm", "wps", "cps", "wph"]), default: "\"wpm\"" },
    KeyDef { name: "startGraphsAtZero", group: Group::Appearance, display: "start graphs at zero", kind: Kind::Bool, default: "true" },
    KeyDef { name: "maxLineWidth", group: Group::Appearance, display: "max line width", kind: Kind::MaxLineWidth, default: "0" },
    KeyDef { name: "fontSize", group: Group::Appearance, display: "font size", kind: Kind::Positive, default: "2" },
    KeyDef { name: "fontFamily", group: Group::Appearance, display: "font family", kind: Kind::Name, default: "\"Roboto_Mono\"" },
    KeyDef { name: "keymapMode", group: Group::Appearance, display: "keymap mode", kind: Kind::Choice(&["off", "static", "react", "next"]), default: "\"off\"" },
    KeyDef { name: "keymapLayout", group: Group::Appearance, display: "keymap layout", kind: Kind::Name, default: "\"overrideSync\"" },
    KeyDef { name: "keymapStyle", group: Group::Appearance, display: "keymap style", kind: Kind::Choice(&["staggered", "alice", "matrix", "split", "split_matrix", "steno", "steno_matrix"]), default: "\"staggered\"" },
    KeyDef { name: "keymapLegendStyle", group: Group::Appearance, display: "keymap legend style", kind: Kind::Choice(&["lowercase", "uppercase", "blank", "dynamic"]), default: "\"lowercase\"" },
    KeyDef { name: "keymapKeys", group: Group::Appearance, display: "keymap keys", kind: Kind::Choice(&["minimal", "minimal_numrow", "full"]), default: "\"minimal\"" },
    KeyDef { name: "keymapSize", group: Group::Appearance, display: "keymap size", kind: Kind::Number { min: 0.5, max: Some(3.5) }, default: "1" },
    KeyDef { name: "flipTestColors", group: Group::Theme, display: "flip test colors", kind: Kind::Bool, default: "false" },
    KeyDef { name: "colorfulMode", group: Group::Theme, display: "colorful mode", kind: Kind::Bool, default: "false" },
    KeyDef { name: "customBackground", group: Group::Theme, display: "custom background", kind: Kind::BackgroundUrl, default: "\"\"" },
    KeyDef { name: "customBackgroundSize", group: Group::Theme, display: "custom background size", kind: Kind::Choice(&["cover", "contain", "max"]), default: "\"cover\"" },
    KeyDef { name: "customBackgroundFilter", group: Group::Theme, display: "custom background filter", kind: Kind::Numbers(4), default: "[0, 1, 1, 1]" },
    KeyDef { name: "autoSwitchTheme", group: Group::Theme, display: "auto switch theme", kind: Kind::Bool, default: "false" },
    KeyDef { name: "themeLight", group: Group::Theme, display: "theme light", kind: Kind::Name, default: "\"serika\"" },
    KeyDef { name: "themeDark", group: Group::Theme, display: "theme dark", kind: Kind::Name, default: "\"serika_dark\"" },
    KeyDef { name: "randomTheme", group: Group::Theme, display: "random theme", kind: Kind::Choice(&["off", "on", "fav", "light", "dark", "custom", "auto"]), default: "\"off\"" },
    KeyDef { name: "favThemes", group: Group::Theme, display: "favorite themes", kind: Kind::NameList { min: 0, max: None }, default: "[]" },
    KeyDef { name: "theme", group: Group::Theme, display: "theme", kind: Kind::Name, default: "\"serika_dark\"" },
    KeyDef { name: "customTheme", group: Group::Theme, display: "custom theme", kind: Kind::Bool, default: "false" },
    KeyDef { name: "customThemeColors", group: Group::Theme, display: "custom theme colors", kind: Kind::Colors(10), default: "[\"#323437\", \"#e2b714\", \"#e2b714\", \"#646669\", \"#2c2e31\", \"#d1d0c5\", \"#ca4754\", \"#7e2a33\", \"#ca4754\", \"#7e2a33\"]" },
    KeyDef { name: "showKeyTips", group: Group::HideElements, display: "show key tips", kind: Kind::Bool, default: "true" },
    KeyDef { name: "showOutOfFocusWarning", group: Group::HideElements, display: "show out of focus warning", kind: Kind::Bool, default: "true" },
    KeyDef { name: "capsLockWarning", group: Group::HideElements, display: "caps lock warning", kind: Kind::Bool, default: "true" },
    KeyDef { name: "showAverage", group: Group::HideElements, display: "show average", kind: Kind::Choice(&["off", "speed", "acc", "both"]), default: "\"off\"" },
    KeyDef { name: "showPb", group: Group::HideElements, display: "show personal best", kind: Kind::Bool, default: "false" },
    KeyDef { name: "accountChart", group: Group::Hidden, display: "account chart", kind: Kind::ChoiceList { values: &["on", "off"], len: 4 }, default: "[\"on\", \"on\", \"on\", \"on\"]" },
    KeyDef { name: "monkey", group: Group::Hidden, display: "monkey", kind: Kind::Bool, default: "false" },
    KeyDef { name: "monkeyPowerLevel", group: Group::Hidden, display: "monkey power level", kind: Kind::Choice(&["off", "1", "2", "3", "4"]), default: "\"off\"" },
    KeyDef { name: "ads", group: Group::Ads, display: "ads", kind: Kind::Choice(&["off", "result", "on", "sellout"]), default: "\"result\"" },
];

pub fn key_def(name: &str) -> Option<&'static KeyDef> {
    SCHEMA.iter().find(|d| d.name == name)
}

pub fn key_def_by_toml(snake: &str) -> Option<&'static KeyDef> {
    SCHEMA.iter().find(|d| d.toml_key() == snake)
}

/// `smoothCaret` → `smooth_caret`.
pub fn to_snake(camel: &str) -> String {
    let mut out = String::with_capacity(camel.len() + 4);
    for c in camel.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

fn number(v: &Value) -> Option<f64> {
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).filter(|n| n.is_finite())
}

fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#').is_some_and(|h| (h.len() == 3 || h.len() == 6) && h.chars().all(|c| c.is_ascii_hexdigit()))
}

fn is_background_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && !s.contains(['`', '\'', '"'])
        && s.chars().count() <= 2048
        && [".png", ".gif", ".jpeg", ".jpg", ".webp"].iter().any(|ext| lower.find(ext).is_some_and(|i| i > 0))
}

/// Vérifie une valeur contre son type ; le message dit ce qui est attendu.
pub fn validate(kind: &Kind, v: &Value) -> Result<(), String> {
    let fail = |what: String| Err(what);
    match *kind {
        Kind::Bool => v.as_bool().map(|_| ()).ok_or_else(|| "true ou false attendu".into()),
        Kind::Choice(values) => match v.as_str() {
            Some(s) if values.contains(&s) => Ok(()),
            _ => fail(format!("une valeur parmi {} attendue", values.join(", "))),
        },
        Kind::Int { min } => match v.as_integer() {
            Some(n) if n >= min => Ok(()),
            _ => fail(format!("un entier ≥ {min} attendu")),
        },
        Kind::Number { min, max } => match number(v) {
            Some(n) if n >= min && max.is_none_or(|m| n <= m) => Ok(()),
            _ => fail(match max {
                Some(m) => format!("un nombre entre {min} et {m} attendu"),
                None => format!("un nombre ≥ {min} attendu"),
            }),
        },
        Kind::Positive => match number(v) {
            Some(n) if n > 0.0 => Ok(()),
            _ => fail("un nombre strictement positif attendu".into()),
        },
        Kind::MaxLineWidth => match number(v) {
            Some(n) if n == 0.0 || (20.0..=1000.0).contains(&n) => Ok(()),
            _ => fail("0, ou un nombre entre 20 et 1000 attendu".into()),
        },
        Kind::Name => match v.as_str() {
            Some(s) if !s.trim().is_empty() => Ok(()),
            _ => fail("un nom non vide attendu".into()),
        },
        Kind::BackgroundUrl => match v.as_str() {
            Some(s) if s.is_empty() || is_background_url(s) => Ok(()),
            _ => fail("\"\" ou une URL http(s) d'image png, gif, jpeg, jpg ou webp attendue".into()),
        },
        Kind::Numbers(n) => match v.as_array() {
            Some(a) if a.len() == n && a.iter().all(|x| number(x).is_some()) => Ok(()),
            _ => fail(format!("{n} nombres attendus")),
        },
        Kind::NameList { min, max } => match v.as_array() {
            Some(a)
                if a.len() >= min
                    && max.is_none_or(|m| a.len() <= m)
                    && a.iter().all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty())) =>
            {
                Ok(())
            }
            _ => fail(match max {
                Some(m) => format!("une liste de {min} à {m} noms attendue"),
                None => format!("une liste d'au moins {min} noms attendue"),
            }),
        },
        Kind::QuoteLengths => match v.as_array() {
            Some(a) if a.iter().all(|x| x.as_integer().is_some_and(|n| [-3, -2, 0, 1, 2, 3].contains(&n))) => Ok(()),
            _ => fail("une liste de valeurs parmi -3, -2, 0, 1, 2, 3 attendue".into()),
        },
        Kind::Colors(n) => match v.as_array() {
            Some(a) if a.len() == n && a.iter().all(|x| x.as_str().is_some_and(is_hex_color)) => Ok(()),
            _ => fail(format!("{n} couleurs #rgb ou #rrggbb attendues")),
        },
        Kind::ChoiceList { values, len } => match v.as_array() {
            Some(a) if a.len() == len && a.iter().all(|x| x.as_str().is_some_and(|s| values.contains(&s))) => Ok(()),
            _ => fail(format!("{len} valeurs parmi {} attendues", values.join(", "))),
        },
    }
}
```

Note : `default_value` panique si un littéral par défaut est invalide. C'est une erreur de programmation dans la table elle-même, pas une donnée lue du disque, et le test `every_default_is_valid` l'exclut.

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store --test schema && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 6 tests PASS, aucun avertissement.

- [ ] **Step 6 : commit**

```bash
git add Cargo.toml Cargo.lock crates/fasttype-store
git commit -m "feat(store): schéma des 94 clés de config de Monkeytype"
```

---

### Task 2 : Config (accès, changements, TOML)

**Files:**
- Create: `crates/fasttype-store/src/config.rs`
- Modify: `crates/fasttype-store/src/lib.rs` (`pub mod config;` et réexport `pub use config::{Config, ConfigError, ConfigWarning};`)
- Test: `crates/fasttype-store/tests/config.rs`

**Interfaces:**
- Consumes : `SCHEMA`, `key_def`, `key_def_by_toml`, `validate`, `KeyDef::default_value`.
- Produces, dans `fasttype_store::config` :
  - `ConfigError { UnknownKey(String), Invalid { key: String, reason: String } }` (`Display`) ;
  - `ConfigWarning { Syntax(String), UnknownKey(String), Invalid { key: String, reason: String } }` (`Display`, en français) ;
  - `Config`, avec :
    - `defaults()` et `reset(&mut self)` ;
    - `get(&self, key) -> Option<&toml::Value>`, `bool(&self, key) -> bool`, `str(&self, key) -> &str`, `int(&self, key) -> i64`, `float(&self, key) -> f64`, `str_list(&self, key) -> Vec<&str>` et `int_list(&self, key) -> Vec<i64>` ;
    - `set(&mut self, key: &str, value: toml::Value) -> Result<Vec<&'static str>, ConfigError>` : clés réellement changées, effets de bord compris ;
    - `from_toml(src: &str) -> (Config, Vec<ConfigWarning>)` et `to_toml(&self) -> String`.
- Effets de bord de `set` (les `overrideConfig` de metadata.tsx, et seulement eux) :

| Clé changée | Condition | Effet |
|---|---|---|
| `words` | mode ≠ words | mode = words |
| `time` | mode ≠ time | mode = time |
| `quoteLength` | mode ≠ quote | mode = quote |
| `mode` | valeur custom, quote ou zen | numbers = false, punctuation = false |
| `minWpmCustomSpeed` | — | minWpm = custom |
| `minAccCustom` | — | minAcc = custom |
| `paceCaretCustomSpeed` | — | paceCaret = custom |
| `freedomMode` | true | confidenceMode = off |
| `stopOnError` | ≠ off | confidenceMode = off, deleteOnError = off |
| `deleteOnError` | ≠ off | confidenceMode = off, stopOnError = off |
| `confidenceMode` | ≠ off | freedomMode = false, stopOnError = off, deleteOnError = off |
| `liveSpeedStyle`, `liveAccStyle` | text | monkey = false |
| `tapeMode` | ≠ off | showAllLines = false |
| `keymapLayout`, `keymapStyle`, `keymapLegendStyle`, `keymapKeys`, `keymapSize` | keymapMode = off | keymapMode = static |
| `theme` | — | customTheme = false |
| `monkey` | true | liveSpeedStyle et liveAccStyle : text → mini |

  La lecture d'un fichier n'applique **pas** ces effets : comme pour Monkeytype, une config enregistrée est reprise telle quelle.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-store/tests/config.rs` :
```rust
use fasttype_store::{Config, ConfigError, ConfigWarning};
use toml::Value;

fn s(x: &str) -> Value {
    Value::String(x.into())
}

#[test]
fn defaults_are_readable_through_typed_getters() {
    let c = Config::defaults();
    assert_eq!(c.str("mode"), "time");
    assert_eq!(c.int("time"), 30);
    assert_eq!(c.int("words"), 50);
    assert!(!c.bool("punctuation"));
    assert!(c.bool("resultSaving"));
    assert_eq!(c.float("soundVolume"), 0.5);
    assert_eq!(c.float("fontSize"), 2.0);
    assert_eq!(c.int_list("quoteLength"), [1]);
    assert_eq!(c.str_list("customPolyglot"), ["english", "spanish", "french", "german"]);
    assert_eq!(c.str("theme"), "serika_dark");
    assert_eq!(c.str("doesNotExist"), "");
}

#[test]
fn choosing_a_duration_switches_to_time_mode() {
    let mut c = Config::defaults();
    c.set("mode", s("words")).unwrap();
    let changed = c.set("time", Value::Integer(60)).unwrap();
    assert_eq!(changed, ["time", "mode"]);
    assert_eq!((c.str("mode"), c.int("time")), ("time", 60));
    assert_eq!(c.set("time", Value::Integer(60)).unwrap(), Vec::<&str>::new(), "rien ne change");
}

#[test]
fn quote_mode_turns_off_punctuation_and_numbers() {
    let mut c = Config::defaults();
    c.set("punctuation", Value::Boolean(true)).unwrap();
    c.set("numbers", Value::Boolean(true)).unwrap();
    let changed = c.set("mode", s("quote")).unwrap();
    assert_eq!(changed, ["mode", "numbers", "punctuation"]);
    assert!(!c.bool("punctuation") && !c.bool("numbers"));
}

#[test]
fn error_handling_modes_exclude_each_other() {
    let mut c = Config::defaults();
    c.set("freedomMode", Value::Boolean(true)).unwrap();
    c.set("confidenceMode", s("on")).unwrap();
    assert!(!c.bool("freedomMode"));
    c.set("stopOnError", s("word")).unwrap();
    assert_eq!(c.str("confidenceMode"), "off");
    c.set("deleteOnError", s("letter")).unwrap();
    assert_eq!(c.str("stopOnError"), "off");
}

#[test]
fn monkey_downgrades_text_live_stats() {
    let mut c = Config::defaults();
    c.set("liveSpeedStyle", s("text")).unwrap();
    c.set("monkey", Value::Boolean(true)).unwrap();
    assert_eq!(c.str("liveSpeedStyle"), "mini");
    c.set("liveAccStyle", s("text")).unwrap();
    assert!(!c.bool("monkey"));
}

#[test]
fn other_overrides() {
    let mut c = Config::defaults();
    c.set("customTheme", Value::Boolean(true)).unwrap();
    c.set("theme", s("nord")).unwrap();
    assert!(!c.bool("customTheme"));
    c.set("keymapSize", Value::Float(1.5)).unwrap();
    assert_eq!(c.str("keymapMode"), "static");
    c.set("showAllLines", Value::Boolean(true)).unwrap();
    c.set("tapeMode", s("word")).unwrap();
    assert!(!c.bool("showAllLines"));
    c.set("minAccCustom", Value::Integer(95)).unwrap();
    assert_eq!(c.str("minAcc"), "custom");
}

#[test]
fn invalid_set_is_refused_and_changes_nothing() {
    let mut c = Config::defaults();
    assert!(matches!(c.set("smoothCaret", s("turbo")), Err(ConfigError::Invalid { .. })));
    assert!(matches!(c.set("nope", s("x")), Err(ConfigError::UnknownKey(_))));
    assert_eq!(c, Config::defaults());
}

#[test]
fn toml_roundtrip_is_lossless() {
    let mut c = Config::defaults();
    c.set("smoothCaret", s("fast")).unwrap();
    c.set("customBackgroundFilter", toml::from_str::<toml::Table>("v = [2, 0.5, 1, 1]").unwrap()["v"].clone()).unwrap();
    c.set("theme", s("serika \"quoted\"")).unwrap();
    let text = c.to_toml();
    assert!(text.contains("smooth_caret = \"fast\""), "{text}");
    assert!(text.contains("# caret"), "{text}");
    let (back, warnings) = Config::from_toml(&text);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(back, c);
}

#[test]
fn hand_edited_file_keeps_valid_keys_and_warns_on_the_rest() {
    let (c, warnings) = Config::from_toml("smooth_caret = \"slow\"\nturbo = true\ntime = -5\nmode = \"words\"\n");
    assert_eq!(c.str("smoothCaret"), "slow");
    assert_eq!(c.int("time"), 30, "valeur invalide remplacée par le défaut");
    assert_eq!(c.str("mode"), "words", "aucun effet de bord à la lecture");
    assert_eq!(warnings.len(), 2);
    assert!(warnings.contains(&ConfigWarning::UnknownKey("turbo".into())));
    assert!(warnings.iter().any(|w| matches!(w, ConfigWarning::Invalid { key, .. } if key == "time")));
    assert!(warnings.iter().all(|w| !w.to_string().is_empty()));
}

#[test]
fn broken_toml_gives_defaults_and_one_warning() {
    let (c, warnings) = Config::from_toml("smooth_caret = \"slow");
    assert_eq!(c, Config::defaults());
    assert!(matches!(warnings.as_slice(), [ConfigWarning::Syntax(_)]));
}

#[test]
fn reset_restores_defaults() {
    let mut c = Config::defaults();
    c.set("words", Value::Integer(10)).unwrap();
    c.reset();
    assert_eq!(c, Config::defaults());
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test config`
Expected: échec de compilation (`unresolved imports fasttype_store::Config, ...`).

- [ ] **Step 3 : implémenter `config.rs`**

```rust
//! Config de fasttype : les 94 clés de Monkeytype, validées par `SCHEMA`.
//! `set` applique les effets de bord (`overrideConfig`) ; la lecture d'un
//! fichier ne les applique pas.

use crate::schema::{Group, SCHEMA, key_def, key_def_by_toml, validate};
use std::fmt;
use toml::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    UnknownKey(String),
    Invalid { key: String, reason: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::UnknownKey(k) => write!(f, "réglage inconnu : {k}"),
            ConfigError::Invalid { key, reason } => write!(f, "{key} : {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    Syntax(String),
    UnknownKey(String),
    Invalid { key: String, reason: String },
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigWarning::Syntax(e) => write!(f, "config.toml illisible ({e}) : réglages par défaut utilisés"),
            ConfigWarning::UnknownKey(k) => write!(f, "config.toml : clé inconnue « {k} » ignorée"),
            ConfigWarning::Invalid { key, reason } => {
                write!(f, "config.toml : « {key} » invalide ({reason}), valeur par défaut utilisée")
            }
        }
    }
}

/// Valeurs alignées sur `SCHEMA` (même ordre, même longueur).
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    values: Vec<Value>,
}

impl Default for Config {
    fn default() -> Self {
        Self::defaults()
    }
}

fn index(key: &str) -> Option<usize> {
    SCHEMA.iter().position(|d| d.name == key)
}

/// Littéral TOML d'une valeur validée. Les chaînes passent par l'échappement
/// JSON, qui est un sous-ensemble valide des chaînes TOML.
fn literal(v: &Value) -> String {
    match v {
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => format!("{f:?}"),
        Value::Boolean(b) => b.to_string(),
        Value::Array(a) => format!("[{}]", a.iter().map(literal).collect::<Vec<_>>().join(", ")),
        other => other.to_string(),
    }
}

impl Config {
    pub fn defaults() -> Self {
        Self { values: SCHEMA.iter().map(|d| d.default_value()).collect() }
    }

    /// « Reset settings » de la danger zone.
    pub fn reset(&mut self) {
        *self = Self::defaults();
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        index(key).map(|i| &self.values[i])
    }

    pub fn bool(&self, key: &str) -> bool {
        self.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn str(&self, key: &str) -> &str {
        self.get(key).and_then(Value::as_str).unwrap_or("")
    }

    pub fn int(&self, key: &str) -> i64 {
        self.get(key).and_then(|v| v.as_integer().or_else(|| v.as_float().map(|f| f as i64))).unwrap_or(0)
    }

    pub fn float(&self, key: &str) -> f64 {
        self.get(key).and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(0.0)
    }

    pub fn str_list(&self, key: &str) -> Vec<&str> {
        self.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
    }

    pub fn int_list(&self, key: &str) -> Vec<i64> {
        self.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_integer).collect()).unwrap_or_default()
    }

    /// Change une clé et applique les effets de bord de Monkeytype. Renvoie les
    /// clés réellement modifiées, dans l'ordre (la clé demandée d'abord).
    pub fn set(&mut self, key: &str, value: Value) -> Result<Vec<&'static str>, ConfigError> {
        let def = key_def(key).ok_or_else(|| ConfigError::UnknownKey(key.to_string()))?;
        validate(&def.kind, &value).map_err(|reason| ConfigError::Invalid { key: key.to_string(), reason })?;
        let mut changed = Vec::new();
        self.force(def.name, value, &mut changed);
        self.apply_overrides(def.name, &mut changed);
        Ok(changed)
    }

    /// Affecte sans validation (valeurs internes) et note la clé si elle change.
    fn force(&mut self, key: &'static str, value: Value, changed: &mut Vec<&'static str>) {
        let i = index(key).expect("clé interne présente dans SCHEMA");
        if self.values[i] != value {
            self.values[i] = value;
            changed.push(SCHEMA[i].name);
        }
    }

    fn force_str(&mut self, key: &'static str, value: &str, changed: &mut Vec<&'static str>) {
        self.force(key, Value::String(value.to_string()), changed);
    }

    /// `overrideConfig` de config/metadata.tsx.
    fn apply_overrides(&mut self, key: &'static str, changed: &mut Vec<&'static str>) {
        let off = Value::Boolean(false);
        match key {
            "words" => self.force_str("mode", "words", changed),
            "time" => self.force_str("mode", "time", changed),
            "quoteLength" => self.force_str("mode", "quote", changed),
            "mode" if matches!(self.str("mode"), "custom" | "quote" | "zen") => {
                self.force("numbers", off.clone(), changed);
                self.force("punctuation", off, changed);
            }
            "minWpmCustomSpeed" => self.force_str("minWpm", "custom", changed),
            "minAccCustom" => self.force_str("minAcc", "custom", changed),
            "paceCaretCustomSpeed" => self.force_str("paceCaret", "custom", changed),
            "freedomMode" if self.bool("freedomMode") => self.force_str("confidenceMode", "off", changed),
            "stopOnError" if self.str("stopOnError") != "off" => {
                self.force_str("confidenceMode", "off", changed);
                self.force_str("deleteOnError", "off", changed);
            }
            "deleteOnError" if self.str("deleteOnError") != "off" => {
                self.force_str("confidenceMode", "off", changed);
                self.force_str("stopOnError", "off", changed);
            }
            "confidenceMode" if self.str("confidenceMode") != "off" => {
                self.force("freedomMode", off, changed);
                self.force_str("stopOnError", "off", changed);
                self.force_str("deleteOnError", "off", changed);
            }
            "liveSpeedStyle" | "liveAccStyle" if self.str(key) == "text" => self.force("monkey", off, changed),
            "tapeMode" if self.str("tapeMode") != "off" => self.force("showAllLines", off, changed),
            "keymapLayout" | "keymapStyle" | "keymapLegendStyle" | "keymapKeys" | "keymapSize"
                if self.str("keymapMode") == "off" =>
            {
                self.force_str("keymapMode", "static", changed)
            }
            "theme" => self.force("customTheme", off, changed),
            "monkey" if self.bool("monkey") => {
                for k in ["liveSpeedStyle", "liveAccStyle"] {
                    if self.str(k) == "text" {
                        self.force_str(k, "mini", changed);
                    }
                }
            }
            _ => {}
        }
    }

    /// Lit un fichier TOML. Ne échoue jamais : une clé inconnue ou invalide
    /// est ignorée (défaut gardé) et signalée.
    pub fn from_toml(src: &str) -> (Config, Vec<ConfigWarning>) {
        let mut config = Config::defaults();
        let table = match toml::from_str::<toml::Table>(src) {
            Ok(t) => t,
            Err(e) => return (config, vec![ConfigWarning::Syntax(e.message().to_string())]),
        };
        let mut warnings = Vec::new();
        for (key, value) in table {
            match key_def_by_toml(&key) {
                None => warnings.push(ConfigWarning::UnknownKey(key)),
                Some(def) => match validate(&def.kind, &value) {
                    Ok(()) => {
                        let i = index(def.name).expect("clé de SCHEMA");
                        config.values[i] = value;
                    }
                    Err(reason) => warnings.push(ConfigWarning::Invalid { key, reason }),
                },
            }
        }
        (config, warnings)
    }

    /// Fichier complet, groupé comme la page settings de Monkeytype.
    pub fn to_toml(&self) -> String {
        let mut out = String::from(
            "# Réglages de fasttype (mêmes clés que Monkeytype, en snake_case).\n\
             # Une clé inconnue ou invalide est ignorée au démarrage.\n",
        );
        let mut group: Option<Group> = None;
        for (def, value) in SCHEMA.iter().zip(&self.values) {
            if group != Some(def.group) {
                group = Some(def.group);
                out.push_str(&format!("\n# {}\n", def.group.label()));
            }
            out.push_str(&format!("{} = {}\n", def.toml_key(), literal(value)));
        }
        out
    }
}
```

Dans `lib.rs` :
```rust
pub mod config;
pub mod schema;

pub use config::{Config, ConfigError, ConfigWarning};
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 6 + 11 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-store
git commit -m "feat(store): config validée, effets de bord de Monkeytype et TOML"
```

---

### Task 3 : chemins XDG, écriture atomique, fichier de config

**Files:**
- Create: `crates/fasttype-store/src/paths.rs`, `crates/fasttype-store/src/fs.rs`, `crates/fasttype-store/tests/common/mod.rs`
- Modify: `crates/fasttype-store/src/lib.rs`
- Test: `crates/fasttype-store/tests/files.rs`

**Interfaces:**
- Produces :
  - `Paths { config_dir: PathBuf, data_dir: PathBuf }`, avec :
    - `from_env(get: impl Fn(&str) -> Option<String>) -> Option<Paths>` et `from_system() -> Option<Paths>` ;
    - `config_file()`, `results_file()`, `pbs_file()`, `custom_texts_dir()` et `favorites_file()` ;
  - `fs::write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()>` ;
  - `fs::load_config(path: &Path) -> (Config, Vec<String>)` : avertissements en français ; un TOML cassé est déplacé vers `config.toml.bak` ;
  - `fs::save_config(path: &Path, config: &Config) -> io::Result<()>` ;
  - dans `tests/common` : `scratch(name) -> PathBuf` et `result(mode2: &str, wpm: f64, timestamp: u64) -> TestResult`.

- [ ] **Step 1 : outil de test partagé**

`crates/fasttype-store/tests/common/mod.rs` :
```rust
#![allow(dead_code)]

use fasttype_core::result::{ChartData, TestResult};
use fasttype_core::spec::{Difficulty, Mode};
use std::fs;
use std::path::PathBuf;

/// Dossier temporaire propre au test : jamais le vrai `$HOME`.
pub fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fasttype-store-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Résultat valide de test time en anglais.
pub fn result(mode2: &str, wpm: f64, timestamp: u64) -> TestResult {
    TestResult {
        timestamp,
        mode: Mode::Time,
        mode2: mode2.into(),
        language: "english".into(),
        punctuation: false,
        numbers: false,
        difficulty: Difficulty::Normal,
        lazy_mode: false,
        wpm,
        raw: wpm + 5.0,
        acc: 97.5,
        consistency: 80.0,
        char_stats: [100, 2, 0, 1],
        test_duration: 30.0,
        afk_duration: 0,
        afk_detected: false,
        bailed_out: false,
        quote_id: None,
        quote_length: None,
        chart: ChartData { wpm: vec![wpm; 3], burst: vec![wpm; 3], err: vec![0; 3] },
        invalid: None,
    }
}
```

- [ ] **Step 2 : écrire les tests qui échouent**

`crates/fasttype-store/tests/files.rs` :
```rust
mod common;

use common::scratch;
use fasttype_store::Config;
use fasttype_store::fs::{load_config, save_config, write_atomic};
use fasttype_store::paths::Paths;
use std::collections::HashMap;
use std::path::PathBuf;

fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |k| map.get(k).cloned()
}

#[test]
fn xdg_paths_with_home_fallback() {
    let p = Paths::from_env(env(&[("HOME", "/home/a")])).unwrap();
    assert_eq!(p.config_dir, PathBuf::from("/home/a/.config/fasttype"));
    assert_eq!(p.data_dir, PathBuf::from("/home/a/.local/share/fasttype"));
    assert_eq!(p.config_file(), PathBuf::from("/home/a/.config/fasttype/config.toml"));
    assert_eq!(p.results_file(), PathBuf::from("/home/a/.local/share/fasttype/results.jsonl"));
    assert_eq!(p.pbs_file(), PathBuf::from("/home/a/.local/share/fasttype/personal_bests.json"));

    let x = Paths::from_env(env(&[("HOME", "/home/a"), ("XDG_CONFIG_HOME", "/cfg"), ("XDG_DATA_HOME", "/data")])).unwrap();
    assert_eq!(x.config_dir, PathBuf::from("/cfg/fasttype"));
    assert_eq!(x.data_dir, PathBuf::from("/data/fasttype"));

    // XDG relatif ou vide : ignoré, comme le veut la spécification XDG
    let rel = Paths::from_env(env(&[("HOME", "/home/a"), ("XDG_CONFIG_HOME", "cfg"), ("XDG_DATA_HOME", "")])).unwrap();
    assert_eq!(rel.config_dir, PathBuf::from("/home/a/.config/fasttype"));
    assert_eq!(rel.data_dir, PathBuf::from("/home/a/.local/share/fasttype"));

    assert!(Paths::from_env(env(&[])).is_none());
}

#[test]
fn atomic_write_leaves_no_temp_file() {
    let dir = scratch("atomic");
    let file = dir.join("sub/f.txt");
    write_atomic(&file, b"one").unwrap();
    write_atomic(&file, b"two").unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"two");
    let names: Vec<_> = std::fs::read_dir(dir.join("sub")).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names, ["f.txt"]);
}

#[test]
fn missing_config_gives_defaults_silently() {
    let dir = scratch("noconfig");
    let (c, warnings) = load_config(&dir.join("config.toml"));
    assert_eq!(c, Config::defaults());
    assert!(warnings.is_empty());
}

#[test]
fn saved_config_loads_back() {
    let dir = scratch("roundtrip");
    let path = dir.join("config.toml");
    let mut c = Config::defaults();
    c.set("smoothCaret", toml::Value::String("slow".into())).unwrap();
    save_config(&path, &c).unwrap();
    let (back, warnings) = load_config(&path);
    assert_eq!(back, c);
    assert!(warnings.is_empty());
}

#[test]
fn broken_config_is_moved_aside() {
    let dir = scratch("broken");
    let path = dir.join("config.toml");
    std::fs::write(&path, "smooth_caret = \"slow").unwrap();
    let (c, warnings) = load_config(&path);
    assert_eq!(c, Config::defaults());
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings[1].contains("config.toml.bak"));
    assert_eq!(std::fs::read_to_string(dir.join("config.toml.bak")).unwrap(), "smooth_caret = \"slow");
    assert!(!path.exists());
}

#[test]
fn partially_invalid_config_is_kept_in_place() {
    let dir = scratch("partial");
    let path = dir.join("config.toml");
    std::fs::write(&path, "smooth_caret = \"slow\"\nturbo = 1\n").unwrap();
    let (c, warnings) = load_config(&path);
    assert_eq!(c.str("smoothCaret"), "slow");
    assert_eq!(warnings.len(), 1);
    assert!(path.exists());
}
```

- [ ] **Step 3 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test files`
Expected: échec de compilation (`unresolved import fasttype_store::fs`).

- [ ] **Step 4 : implémenter**

`crates/fasttype-store/src/paths.rs` :
```rust
//! Emplacements XDG des fichiers de fasttype.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

/// Variable XDG utilisable : définie, non vide et absolue.
fn xdg(get: &impl Fn(&str) -> Option<String>, var: &str) -> Option<PathBuf> {
    get(var).filter(|v| !v.is_empty()).map(PathBuf::from).filter(|p| p.is_absolute())
}

impl Paths {
    /// `get` lit une variable d'environnement (injectée pour les tests).
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Option<Paths> {
        let home = get("HOME").filter(|h| !h.is_empty()).map(PathBuf::from);
        let config = xdg(&get, "XDG_CONFIG_HOME").or_else(|| home.as_ref().map(|h| h.join(".config")))?;
        let data = xdg(&get, "XDG_DATA_HOME").or_else(|| home.as_ref().map(|h| h.join(".local/share")))?;
        Some(Paths { config_dir: config.join("fasttype"), data_dir: data.join("fasttype") })
    }

    pub fn from_system() -> Option<Paths> {
        Self::from_env(|k| std::env::var(k).ok())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn results_file(&self) -> PathBuf {
        self.data_dir.join("results.jsonl")
    }

    pub fn pbs_file(&self) -> PathBuf {
        self.data_dir.join("personal_bests.json")
    }

    pub fn custom_texts_dir(&self) -> PathBuf {
        self.data_dir.join("custom_texts")
    }

    pub fn favorites_file(&self) -> PathBuf {
        self.data_dir.join("favorite_quotes.json")
    }
}

/// Dossier parent existant ou à créer (`.` pour un chemin nu).
pub(crate) fn parent_of(path: &Path) -> &Path {
    path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."))
}
```

`crates/fasttype-store/src/fs.rs` :
```rust
//! Écritures sûres et fichier de config.

use crate::config::Config;
use crate::paths::parent_of;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

/// Écrit dans un fichier temporaire voisin, le synchronise, puis le renomme :
/// le fichier visé est soit l'ancien, soit le nouveau, jamais un mélange.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = parent_of(path);
    fs::create_dir_all(dir)?;
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chemin sans nom de fichier"))?;
    let tmp = dir.join(format!(".{}.tmp", name.to_string_lossy()));
    let result = (|| {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Charge la config. Fichier absent : défauts sans avertissement. TOML cassé :
/// défauts, et le fichier est déplacé vers `config.toml.bak` pour ne rien perdre.
pub fn load_config(path: &Path) -> (Config, Vec<String>) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return (Config::defaults(), Vec::new()),
        Err(e) => return (Config::defaults(), vec![format!("{} illisible ({e}) : réglages par défaut", path.display())]),
    };
    let (config, warnings) = Config::from_toml(&src);
    let mut messages: Vec<String> = warnings.iter().map(ToString::to_string).collect();
    if matches!(warnings.first(), Some(crate::ConfigWarning::Syntax(_))) {
        let bak = path.with_extension("toml.bak");
        match fs::rename(path, &bak) {
            Ok(()) => messages.push(format!("l'ancien fichier est conservé dans {}", bak.display())),
            Err(e) => messages.push(format!("impossible de mettre de côté {} ({e})", path.display())),
        }
    }
    (config, messages)
}

pub fn save_config(path: &Path, config: &Config) -> io::Result<()> {
    write_atomic(path, config.to_toml().as_bytes())
}
```

Dans `lib.rs`, ajouter `pub mod fs;` et `pub mod paths;` (ordre alphabétique).

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous les tests de `fasttype-store` PASS (6 + 11 + 6).

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-store
git commit -m "feat(store): chemins XDG, écriture atomique et fichier de config"
```

---

### Task 4 : historique des résultats (JSONL)

**Files:**
- Create: `crates/fasttype-store/src/results.rs`
- Modify: `crates/fasttype-store/src/lib.rs` (`pub mod results;`), `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§4.5)
- Test: `crates/fasttype-store/tests/results.rs`

**Interfaces:**
- Consumes : `fasttype_core::result::TestResult` (serde).
- Produces : `ResultLog::{new(path: PathBuf), append(&self, r: &TestResult) -> io::Result<()>, load(&self) -> io::Result<LoadedResults>}` et `LoadedResults { results: Vec<TestResult>, skipped_lines: usize }`.
- Décision sur le graphique : Monkeytype remplace `chartData` par `"toolong"` à l'enregistrement d'un test de plus de 122 s, une limite de son serveur. En local, la série complète est gardée : on ne perd rien, et le fichier n'enfle que d'environ 50 Ko pour une heure de frappe. La spec §4.5 est alignée dans cette tâche.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-store/tests/results.rs` :
```rust
mod common;

use common::{result, scratch};
use fasttype_store::results::ResultLog;
use std::io::Write;

#[test]
fn appends_and_loads_in_order() {
    let dir = scratch("results");
    let log = ResultLog::new(dir.join("results.jsonl"));
    log.append(&result("30", 80.0, 1)).unwrap();
    log.append(&result("60", 90.0, 2)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(loaded.skipped_lines, 0);
    assert_eq!(loaded.results, [result("30", 80.0, 1), result("60", 90.0, 2)]);
    let text = std::fs::read_to_string(dir.join("results.jsonl")).unwrap();
    assert_eq!(text.lines().count(), 2);
}

#[test]
fn missing_file_is_empty_history() {
    let dir = scratch("noresults");
    let loaded = ResultLog::new(dir.join("results.jsonl")).load().unwrap();
    assert!(loaded.results.is_empty());
}

#[test]
fn corrupt_line_is_skipped_and_counted() {
    let dir = scratch("corrupt");
    let path = dir.join("results.jsonl");
    let log = ResultLog::new(path.clone());
    log.append(&result("30", 80.0, 1)).unwrap();
    std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"{not json}\n\n").unwrap();
    log.append(&result("30", 81.0, 2)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(loaded.results.len(), 2);
    assert_eq!(loaded.skipped_lines, 1, "les lignes vides ne comptent pas");
}

#[test]
fn append_after_truncated_line_stays_readable() {
    let dir = scratch("truncated");
    let path = dir.join("results.jsonl");
    let log = ResultLog::new(path.clone());
    log.append(&result("30", 80.0, 1)).unwrap();
    // plantage au milieu d'une écriture : ligne sans fin
    std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"{\"timestamp\":2,\"mo").unwrap();
    log.append(&result("30", 82.0, 3)).unwrap();
    let loaded = log.load().unwrap();
    assert_eq!(loaded.results.iter().map(|r| r.timestamp).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(loaded.skipped_lines, 1);
}

#[test]
fn long_chart_is_kept_whole() {
    let dir = scratch("longchart");
    let log = ResultLog::new(dir.join("results.jsonl"));
    let mut r = result("0", 70.0, 1);
    r.chart.wpm = vec![70.0; 3600];
    log.append(&r).unwrap();
    assert_eq!(log.load().unwrap().results[0].chart.wpm.len(), 3600);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test results`
Expected: échec de compilation (`unresolved import fasttype_store::results`).

- [ ] **Step 3 : implémenter `results.rs`**

```rust
//! Historique des résultats : un `TestResult` JSON par ligne, ajouté en une
//! seule écriture à la fin du test.

use fasttype_core::result::TestResult;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedResults {
    pub results: Vec<TestResult>,
    /// Lignes illisibles ignorées (écriture interrompue, édition à la main).
    pub skipped_lines: usize,
}

pub struct ResultLog {
    path: PathBuf,
}

/// Vrai si le fichier est vide ou finit par un saut de ligne.
fn ends_with_newline(file: &mut File) -> io::Result<bool> {
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(true);
    }
    file.seek(SeekFrom::Start(len - 1))?;
    let mut last = [0u8; 1];
    file.read_exact(&mut last)?;
    Ok(last[0] == b'\n')
}

impl ResultLog {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Ajoute une ligne. Si la dernière ligne a été tronquée par un plantage,
    /// on repart sur une nouvelle ligne pour ne pas coller les deux.
    pub fn append(&self, r: &TestResult) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut file = OpenOptions::new().create(true).read(true).append(true).open(&self.path)?;
        let mut line = if ends_with_newline(&mut file)? { String::new() } else { String::from("\n") };
        line.push_str(&serde_json::to_string(r).map_err(io::Error::other)?);
        line.push('\n');
        file.write_all(line.as_bytes())
    }

    pub fn load(&self) -> io::Result<LoadedResults> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        };
        let mut results = Vec::new();
        let mut skipped_lines = 0;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            match serde_json::from_str(line) {
                Ok(r) => results.push(r),
                Err(_) => skipped_lines += 1,
            }
        }
        Ok(LoadedResults { results, skipped_lines })
    }
}
```

- [ ] **Step 4 : aligner la spec**

Dans `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md`, §4.5, remplacer « Séries : wpm cumulé, burst par seconde et erreurs par seconde, 122 points maximum. » par :
« Séries : wpm cumulé, burst par seconde et erreurs par seconde. Monkeytype ne garde pas le graphique des tests de plus de 122 s (limite de son serveur) ; fasttype garde la série complète en local. »

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store --test results && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 5 tests PASS.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-store docs/superpowers/specs
git commit -m "feat(store): historique des résultats en JSONL, robuste aux lignes tronquées"
```

---

### Task 5 : records personnels

**Files:**
- Create: `crates/fasttype-store/src/pbs.rs`
- Modify: `crates/fasttype-store/src/lib.rs` (`pub mod pbs;`)
- Test: `crates/fasttype-store/tests/pbs.rs`

**Interfaces:**
- Consumes : `TestResult::{pb_key, pb_eligible}`, `PbKey`, `write_atomic`.
- Produces :
  - `PbEntry { wpm: f64, raw: f64, acc: f64, consistency: f64, timestamp: u64 }` ;
  - `PbOutcome { NewBest { previous: Option<f64> }, NotBest { best: f64 }, Ineligible }` ;
  - `PersonalBests`, avec :
    - `get(&self, key: &PbKey) -> Option<&PbEntry>` et `len(&self) -> usize` ;
    - `update(&mut self, r: &TestResult) -> PbOutcome` et `rebuild<'a>(results: impl IntoIterator<Item = &'a TestResult>) -> Self` ;
    - `load(path: &Path) -> io::Result<Self>` (fichier absent : vide ; contenu illisible : `ErrorKind::InvalidData`) et `save(&self, path: &Path) -> io::Result<()>`.
- Règles (`result-pb.ts` et `backend/src/utils/pb.ts`) :
  - seul un résultat `pb_eligible()` compte : valide, hors citation, sans bail out ;
  - le record n'est remplacé que si le wpm est **strictement** supérieur ;
  - un record par `PbKey`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-store/tests/pbs.rs` :
```rust
mod common;

use common::{result, scratch};
use fasttype_core::result::Invalid;
use fasttype_core::spec::Mode;
use fasttype_store::pbs::{PbOutcome, PersonalBests};

#[test]
fn only_strictly_faster_results_beat_the_record() {
    let mut pbs = PersonalBests::default();
    assert_eq!(pbs.update(&result("30", 80.0, 1)), PbOutcome::NewBest { previous: None });
    assert_eq!(pbs.update(&result("30", 80.0, 2)), PbOutcome::NotBest { best: 80.0 });
    assert_eq!(pbs.update(&result("30", 85.5, 3)), PbOutcome::NewBest { previous: Some(80.0) });
    let key = result("30", 0.0, 0).pb_key();
    assert_eq!(pbs.get(&key).unwrap().timestamp, 3);
}

#[test]
fn each_configuration_has_its_own_record() {
    let mut pbs = PersonalBests::default();
    pbs.update(&result("30", 80.0, 1));
    pbs.update(&result("60", 70.0, 2));
    let mut punct = result("30", 60.0, 3);
    punct.punctuation = true;
    assert_eq!(pbs.update(&punct), PbOutcome::NewBest { previous: None });
    assert_eq!(pbs.len(), 3);
}

#[test]
fn ineligible_results_are_ignored() {
    let mut pbs = PersonalBests::default();
    let mut invalid = result("30", 200.0, 1);
    invalid.invalid = Some(Invalid::Afk);
    let mut bailed = result("30", 200.0, 2);
    bailed.bailed_out = true;
    let mut quote = result("7", 200.0, 3);
    quote.mode = Mode::Quote;
    for r in [invalid, bailed, quote] {
        assert_eq!(pbs.update(&r), PbOutcome::Ineligible);
    }
    assert_eq!(pbs.len(), 0);
}

#[test]
fn rebuild_matches_incremental_updates() {
    let history = [result("30", 80.0, 1), result("30", 90.0, 2), result("60", 70.0, 3), result("30", 85.0, 4)];
    let mut incremental = PersonalBests::default();
    for r in &history {
        incremental.update(r);
    }
    assert_eq!(PersonalBests::rebuild(&history), incremental);
}

#[test]
fn pbs_are_written_atomically_and_load_back() {
    let dir = scratch("pbs");
    let path = dir.join("personal_bests.json");
    let mut pbs = PersonalBests::default();
    pbs.update(&result("30", 80.0, 1));
    pbs.update(&result("60", 70.0, 2));
    pbs.save(&path).unwrap();
    assert_eq!(PersonalBests::load(&path).unwrap(), pbs);
    assert!(!dir.join(".personal_bests.json.tmp").exists());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn missing_and_corrupt_files() {
    let dir = scratch("pbs-bad");
    let path = dir.join("personal_bests.json");
    assert_eq!(PersonalBests::load(&path).unwrap(), PersonalBests::default());
    std::fs::write(&path, "[{\"broken\"").unwrap();
    assert_eq!(PersonalBests::load(&path).unwrap_err().kind(), std::io::ErrorKind::InvalidData);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test pbs`
Expected: échec de compilation (`unresolved import fasttype_store::pbs`).

- [ ] **Step 3 : implémenter `pbs.rs`**

```rust
//! Records personnels : le meilleur wpm par configuration (`PbKey`).
//! Cache reconstructible depuis l'historique (`fasttype --rebuild-pbs`).

use crate::fs::write_atomic;
use fasttype_core::result::{PbKey, TestResult};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PbEntry {
    pub wpm: f64,
    pub raw: f64,
    pub acc: f64,
    pub consistency: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PbOutcome {
    NewBest { previous: Option<f64> },
    NotBest { best: f64 },
    Ineligible,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PbRecord {
    key: PbKey,
    best: PbEntry,
}

/// Peu d'entrées (une par configuration jouée) : une simple liste suffit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PersonalBests {
    records: Vec<PbRecord>,
}

impl PersonalBests {
    pub fn get(&self, key: &PbKey) -> Option<&PbEntry> {
        self.records.iter().find(|r| &r.key == key).map(|r| &r.best)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn update(&mut self, r: &TestResult) -> PbOutcome {
        if !r.pb_eligible() {
            return PbOutcome::Ineligible;
        }
        let entry = PbEntry { wpm: r.wpm, raw: r.raw, acc: r.acc, consistency: r.consistency, timestamp: r.timestamp };
        let key = r.pb_key();
        match self.records.iter_mut().find(|rec| rec.key == key) {
            None => {
                self.records.push(PbRecord { key, best: entry });
                PbOutcome::NewBest { previous: None }
            }
            Some(rec) if r.wpm > rec.best.wpm => {
                let previous = rec.best.wpm;
                rec.best = entry;
                PbOutcome::NewBest { previous: Some(previous) }
            }
            Some(rec) => PbOutcome::NotBest { best: rec.best.wpm },
        }
    }

    pub fn rebuild<'a>(results: impl IntoIterator<Item = &'a TestResult>) -> Self {
        let mut pbs = Self::default();
        for r in results {
            pbs.update(r);
        }
        pbs
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        write_atomic(path, &json)
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store --test pbs && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 6 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-store
git commit -m "feat(store): records personnels par configuration"
```

---

### Task 6 : textes custom et citations favorites

**Files:**
- Create: `crates/fasttype-store/src/texts.rs`
- Modify: `crates/fasttype-store/src/lib.rs` (`pub mod texts;`)
- Test: `crates/fasttype-store/tests/texts.rs`

**Interfaces:**
- Produces :
  - `CustomTexts::{new(dir: PathBuf), list(&self) -> io::Result<Vec<String>>, load(&self, name) -> io::Result<String>, save(&self, name, text) -> io::Result<()>, delete(&self, name) -> io::Result<()>}` ;
  - `FavoriteQuote { language: String, id: u32 }` ;
  - `FavoriteQuotes::{new(path: PathBuf), load(&self) -> io::Result<Vec<FavoriteQuote>>, toggle(&self, language, id) -> io::Result<bool>}` (renvoie `true` si la citation est désormais favorite).
- Noms de texte acceptés : 1 à 64 caractères parmi lettres, chiffres, espace, `_`, `-` et `.`, sans `.` initial. Tout autre nom donne `ErrorKind::InvalidInput`. Chaque texte est stocké dans `<dir>/<nom>.txt`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-store/tests/texts.rs` :
```rust
mod common;

use common::scratch;
use fasttype_store::texts::{CustomTexts, FavoriteQuote, FavoriteQuotes};
use std::io::ErrorKind;

#[test]
fn custom_texts_roundtrip() {
    let dir = scratch("texts");
    let texts = CustomTexts::new(dir.join("custom_texts"));
    assert!(texts.list().unwrap().is_empty());
    texts.save("lorem", "lorem ipsum dolor").unwrap();
    texts.save("Mon texte 2", "été à Paris").unwrap();
    assert_eq!(texts.list().unwrap(), ["Mon texte 2", "lorem"]);
    assert_eq!(texts.load("Mon texte 2").unwrap(), "été à Paris");
    texts.delete("lorem").unwrap();
    assert_eq!(texts.list().unwrap(), ["Mon texte 2"]);
    assert_eq!(texts.load("lorem").unwrap_err().kind(), ErrorKind::NotFound);
}

#[test]
fn hostile_names_are_refused() {
    let dir = scratch("hostile");
    let texts = CustomTexts::new(dir.join("custom_texts"));
    for name in ["", "../x", "a/b", "/etc/passwd", ".hidden", "..", &"x".repeat(65)] {
        assert_eq!(texts.save(name, "x").unwrap_err().kind(), ErrorKind::InvalidInput, "{name:?}");
    }
    assert!(!dir.join("x.txt").exists());
}

#[test]
fn favorites_toggle_and_persist() {
    let dir = scratch("favs");
    let favs = FavoriteQuotes::new(dir.join("favorite_quotes.json"));
    assert!(favs.load().unwrap().is_empty());
    assert!(favs.toggle("english", 12).unwrap());
    assert!(favs.toggle("french", 3).unwrap());
    assert_eq!(
        favs.load().unwrap(),
        [FavoriteQuote { language: "english".into(), id: 12 }, FavoriteQuote { language: "french".into(), id: 3 }]
    );
    assert!(!favs.toggle("english", 12).unwrap());
    assert_eq!(favs.load().unwrap().len(), 1);
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test texts`
Expected: échec de compilation (`unresolved import fasttype_store::texts`).

- [ ] **Step 3 : implémenter `texts.rs`**

```rust
//! Textes custom enregistrés et citations favorites.

use crate::fs::write_atomic;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

fn valid_name(name: &str) -> bool {
    let len = name.chars().count();
    (1..=64).contains(&len)
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '_' | '-' | '.'))
}

pub struct CustomTexts {
    dir: PathBuf,
}

impl CustomTexts {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn file(&self, name: &str) -> io::Result<PathBuf> {
        if !valid_name(name) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("nom de texte invalide : {name:?}")));
        }
        Ok(self.dir.join(format!("{name}.txt")))
    }

    /// Noms des textes enregistrés, triés.
    pub fn list(&self) -> io::Result<Vec<String>> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut names = Vec::new();
        for entry in entries {
            let file = entry?.file_name().to_string_lossy().to_string();
            if let Some(name) = file.strip_suffix(".txt").filter(|n| valid_name(n)) {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn load(&self, name: &str) -> io::Result<String> {
        fs::read_to_string(self.file(name)?)
    }

    pub fn save(&self, name: &str, text: &str) -> io::Result<()> {
        write_atomic(&self.file(name)?, text.as_bytes())
    }

    pub fn delete(&self, name: &str) -> io::Result<()> {
        fs::remove_file(self.file(name)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteQuote {
    pub language: String,
    pub id: u32,
}

pub struct FavoriteQuotes {
    path: PathBuf,
}

impl FavoriteQuotes {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> io::Result<Vec<FavoriteQuote>> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Ajoute ou retire la citation ; renvoie `true` si elle est désormais favorite.
    pub fn toggle(&self, language: &str, id: u32) -> io::Result<bool> {
        let mut favs = self.load()?;
        let quote = FavoriteQuote { language: language.to_string(), id };
        let now_favorite = match favs.iter().position(|f| *f == quote) {
            Some(i) => {
                favs.remove(i);
                false
            }
            None => {
                favs.push(quote);
                true
            }
        };
        let json = serde_json::to_vec_pretty(&favs).map_err(io::Error::other)?;
        write_atomic(&self.path, &json)?;
        Ok(now_favorite)
    }
}
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store --test texts && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 3 tests PASS.

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-store
git commit -m "feat(store): textes custom et citations favorites"
```

---

### Task 7 : façade Store

**Files:**
- Create: `crates/fasttype-store/src/store.rs`
- Modify: `crates/fasttype-store/src/lib.rs` (`mod store;` et réexport `pub use store::{RecordOutcome, Store};`)
- Test: `crates/fasttype-store/tests/store.rs`

**Interfaces:**
- Consumes : `Paths`, `load_config`, `save_config`, `ResultLog`, `PersonalBests`, `CustomTexts`, `FavoriteQuotes`.
- Produces :
  - `RecordOutcome { Saved(PbOutcome), SavingDisabled, Invalid }` ;
  - `Store`, avec :
    - les champs publics `paths: Paths`, `config: Config`, `warnings: Vec<String>`, `custom_texts: CustomTexts` et `favorites: FavoriteQuotes` ;
    - `open(paths: Paths) -> Store` (ne échoue jamais) ;
    - `save_config(&self) -> io::Result<()>` ;
    - `record(&mut self, r: &TestResult) -> io::Result<RecordOutcome>` ;
    - `personal_best(&self, key: &PbKey) -> Option<&PbEntry>` ;
    - `history(&self) -> io::Result<LoadedResults>` et `rebuild_pbs(&mut self) -> io::Result<LoadedResults>`.
- `open` lit la config et les records, jamais l'historique (§7). Si les records sont illisibles, ils sont reconstruits depuis l'historique, et un avertissement l'annonce.
- `record` : si `resultSaving` vaut false, on renvoie `SavingDisabled` sans rien écrire. Si le résultat n'est pas enregistrable, on renvoie `Invalid`. Sinon, on ajoute à l'historique, on met à jour les records, on les enregistre seulement s'ils ont changé, et on renvoie `Saved(outcome)`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-store/tests/store.rs` :
```rust
mod common;

use common::{result, scratch};
use fasttype_core::result::Invalid;
use fasttype_store::paths::Paths;
use fasttype_store::pbs::PbOutcome;
use fasttype_store::{RecordOutcome, Store};

fn paths(dir: &std::path::Path) -> Paths {
    Paths { config_dir: dir.join("config"), data_dir: dir.join("data") }
}

#[test]
fn opens_on_an_empty_home() {
    let dir = scratch("store-empty");
    let store = Store::open(paths(&dir));
    assert!(store.warnings.is_empty(), "{:?}", store.warnings);
    assert_eq!(store.config.str("mode"), "time");
    assert!(!dir.join("data").exists(), "ouvrir n'écrit rien");
}

#[test]
fn records_results_and_personal_bests() {
    let dir = scratch("store-record");
    let mut store = Store::open(paths(&dir));
    assert_eq!(store.record(&result("30", 80.0, 1)).unwrap(), RecordOutcome::Saved(PbOutcome::NewBest { previous: None }));
    assert_eq!(store.record(&result("30", 70.0, 2)).unwrap(), RecordOutcome::Saved(PbOutcome::NotBest { best: 80.0 }));
    assert_eq!(store.history().unwrap().results.len(), 2);

    let reopened = Store::open(paths(&dir));
    assert_eq!(reopened.personal_best(&result("30", 0.0, 0).pb_key()).unwrap().wpm, 80.0);
}

#[test]
fn saving_disabled_and_invalid_results_write_nothing() {
    let dir = scratch("store-nosave");
    let mut store = Store::open(paths(&dir));
    let mut invalid = result("30", 80.0, 1);
    invalid.invalid = Some(Invalid::TooShort);
    assert_eq!(store.record(&invalid).unwrap(), RecordOutcome::Invalid);
    store.config.set("resultSaving", toml::Value::Boolean(false)).unwrap();
    assert_eq!(store.record(&result("30", 80.0, 2)).unwrap(), RecordOutcome::SavingDisabled);
    assert!(store.history().unwrap().results.is_empty());
}

#[test]
fn config_changes_persist() {
    let dir = scratch("store-config");
    let mut store = Store::open(paths(&dir));
    store.config.set("theme", toml::Value::String("nord".into())).unwrap();
    store.save_config().unwrap();
    assert_eq!(Store::open(paths(&dir)).config.str("theme"), "nord");
}

#[test]
fn corrupt_pbs_are_rebuilt_from_history() {
    let dir = scratch("store-rebuild");
    let mut store = Store::open(paths(&dir));
    store.record(&result("30", 80.0, 1)).unwrap();
    store.record(&result("30", 90.0, 2)).unwrap();
    std::fs::write(paths(&dir).pbs_file(), "garbage").unwrap();

    let reopened = Store::open(paths(&dir));
    assert_eq!(reopened.warnings.len(), 1, "{:?}", reopened.warnings);
    assert_eq!(reopened.personal_best(&result("30", 0.0, 0).pb_key()).unwrap().wpm, 90.0);
    let saved = fasttype_store::pbs::PersonalBests::load(&paths(&dir).pbs_file()).unwrap();
    assert_eq!(saved.len(), 1, "le cache reconstruit est réécrit");
}

#[test]
fn rebuild_pbs_reports_history() {
    let dir = scratch("store-rebuild-cmd");
    let mut store = Store::open(paths(&dir));
    store.record(&result("30", 80.0, 1)).unwrap();
    let report = store.rebuild_pbs().unwrap();
    assert_eq!((report.results.len(), report.skipped_lines), (1, 0));
}

#[test]
fn config_warnings_surface_in_store() {
    let dir = scratch("store-warn");
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::write(dir.join("config/config.toml"), "turbo = true\n").unwrap();
    let store = Store::open(paths(&dir));
    assert_eq!(store.warnings.len(), 1);
    assert!(store.warnings[0].contains("turbo"));
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-store --test store`
Expected: échec de compilation (`unresolved imports fasttype_store::RecordOutcome, fasttype_store::Store`).

- [ ] **Step 3 : implémenter `store.rs`**

```rust
//! Point d'entrée de la persistance pour la TUI.

use crate::config::Config;
use crate::fs::{load_config, save_config};
use crate::paths::Paths;
use crate::pbs::{PbEntry, PbOutcome, PersonalBests};
use crate::results::{LoadedResults, ResultLog};
use crate::texts::{CustomTexts, FavoriteQuotes};
use fasttype_core::result::{PbKey, TestResult};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RecordOutcome {
    Saved(PbOutcome),
    /// Réglage `resultSaving` désactivé.
    SavingDisabled,
    /// Résultat invalide : affiché, jamais enregistré.
    Invalid,
}

pub struct Store {
    pub paths: Paths,
    pub config: Config,
    /// Messages pour les notifications de la TUI.
    pub warnings: Vec<String>,
    pub custom_texts: CustomTexts,
    pub favorites: FavoriteQuotes,
    results: ResultLog,
    pbs: PersonalBests,
}

impl Store {
    /// Lit la config et les records (jamais tout l'historique). Ne échoue jamais.
    pub fn open(paths: Paths) -> Store {
        let (config, mut warnings) = load_config(&paths.config_file());
        let results = ResultLog::new(paths.results_file());
        let pbs = match PersonalBests::load(&paths.pbs_file()) {
            Ok(pbs) => pbs,
            Err(e) => {
                let rebuilt = results.load().map(|h| PersonalBests::rebuild(&h.results)).unwrap_or_default();
                let saved = rebuilt.save(&paths.pbs_file());
                warnings.push(format!(
                    "records personnels illisibles ({e}) : reconstruits depuis l'historique{}",
                    if saved.is_err() { ", sans pouvoir les réécrire" } else { "" }
                ));
                rebuilt
            }
        };
        Store {
            custom_texts: CustomTexts::new(paths.custom_texts_dir()),
            favorites: FavoriteQuotes::new(paths.favorites_file()),
            paths,
            config,
            warnings,
            results,
            pbs,
        }
    }

    pub fn save_config(&self) -> io::Result<()> {
        save_config(&self.paths.config_file(), &self.config)
    }

    /// Enregistre un résultat terminé et met à jour les records.
    pub fn record(&mut self, r: &TestResult) -> io::Result<RecordOutcome> {
        if !self.config.bool("resultSaving") {
            return Ok(RecordOutcome::SavingDisabled);
        }
        if !r.is_saveable() {
            return Ok(RecordOutcome::Invalid);
        }
        self.results.append(r)?;
        let outcome = self.pbs.update(r);
        if matches!(outcome, PbOutcome::NewBest { .. }) {
            self.pbs.save(&self.paths.pbs_file())?;
        }
        Ok(RecordOutcome::Saved(outcome))
    }

    pub fn personal_best(&self, key: &PbKey) -> Option<&PbEntry> {
        self.pbs.get(key)
    }

    /// Tout l'historique (lecture complète : à faire hors du fil de l'interface).
    pub fn history(&self) -> io::Result<LoadedResults> {
        self.results.load()
    }

    /// `fasttype --rebuild-pbs` : recalcule et réécrit les records.
    pub fn rebuild_pbs(&mut self) -> io::Result<LoadedResults> {
        let history = self.results.load()?;
        self.pbs = PersonalBests::rebuild(&history.results);
        self.pbs.save(&self.paths.pbs_file())?;
        Ok(history)
    }
}
```

`lib.rs` final :
```rust
//! Persistance locale de fasttype : config, historique, records personnels,
//! textes custom et citations favorites.

pub mod config;
pub mod fs;
pub mod paths;
pub mod pbs;
pub mod results;
pub mod schema;
mod store;
pub mod texts;

pub use config::{Config, ConfigError, ConfigWarning};
pub use store::{RecordOutcome, Store};
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-store --test store && cargo clippy --workspace --all-targets -- -D warnings`
Expected: 7 tests PASS.

- [ ] **Step 5 : vérification finale du workspace**

Run: `cargo fmt && cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, avec le total de passed relevé dans le ledger, aucun avertissement.

- [ ] **Step 6 : commit**

```bash
git add crates/fasttype-store
git commit -m "feat(store): façade Store (config, historique, records, textes)"
```
