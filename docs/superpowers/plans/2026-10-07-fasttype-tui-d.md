# fasttype-tui, partie D : la palette de commandes — plan d'implémentation (plan 4d)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Changer tous les réglages de la v1 sans éditer `config.toml`, comme sur Monkeytype : `esc` (ou `ctrl+shift+p`) ouvre la palette de commandes. On y trouve la recherche du site, des sous-groupes avec la valeur en cours cochée, la saisie de valeurs libres, l'aperçu des thèmes au survol, et les commandes Next test, Repeat test, Bail out, Clear all notifications et Quit.

**Architecture:**
- **`palette/` (logique pure)** :
  - `filter.rs` reprend `commandline/filter.ts` mot pour mot ;
  - `lists.rs` construit la liste racine depuis le schéma de la config, limitée aux réglages qui ont un effet en v1 ;
  - `state.rs` gère la pile de sous-groupes, la recherche, la commande active et la saisie libre validée. Il renvoie l'action choisie (`Outcome::Run`) sans toucher à la config.
- **`App`** ouvre la palette, lui passe les touches, et exécute les actions :
  - **Changer un réglage :** `Config::set`, puis `save_config`, et un restart si la clé touche le test.
  - **Commandes de test :** restart, repeat (`into_repeat`), bail out.
  - **Survol d'un thème :** aperçu de sa palette, puis retour au thème en cours si on annule.
- **`view/palette.rs`** dessine un voile noir à 50 % sur tout l'écran, puis une boîte en haut au centre. La saisie a un curseur, la ligne active est inversée et la valeur en cours porte une coche.
  - **Mots agrandis (plan 4c) :** la zone agrandie contourne la boîte (`ScaledText.hole`). Elle est assombrie elle aussi.
- **Clavier :**
  - flèches, `ctrl+j/k/n/p`, `tab` et `shift+tab` pour naviguer ;
  - `ctrl+shift+p` (avec le protocole clavier Kitty) ;
  - le collage arrive d'un bloc (bracketed paste), sans valider la saisie au premier saut de ligne.

**Tech Stack:** Rust 1.97 (édition 2024), `ratatui` 0.30.2, `crossterm` 0.29.0 (`EnableBracketedPaste`), `toml` 1.1.6 (valeurs de config), Python 3 pour le test pty.

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§5.3 « Palette de commandes », §4.4 bail out, §4.6 repeat). Les plans 4a à 4c sont livrés sur `main`.

**Code vérifié avant rédaction :** tout le code de ce plan a été assemblé et exécuté dans une copie de travail jetable.
- **Tests :** 368 tests du workspace au vert, et clippy ne signale rien.
- **Benchmark :** `key_and_frame_200x60` à 77 µs.
- **Test pty `--palette` :** Échap, puis « quit », puis Entrée ferme l'application proprement. Le collage est désactivé à la sortie.

**Comportements de référence (Monkeytype, commit 574d819, `commandline/`, `CommandlineModal.tsx`) :**
- **Ouverture :** `esc`, ou `tab` quand `quickRestart` vaut `esc` (`hotkeys.ts`), et toujours `ctrl/cmd+shift+p`. La même liste racine sert au test et au résultat ; les entrées Résultat (Next test, Repeat test) n'apparaissent que sur le résultat.
- **Recherche :**
  - la saisie est mise en minuscules, sans `>` initial, découpée en mots et sans ponctuation ;
  - chaque mot saisi doit être le début d'un mot du libellé ou d'un alias, et un mot du libellé ne sert qu'une fois ;
  - on garde les commandes qui apparient le plus de mots (seuil abaissé jusqu'à ce qu'une commande l'atteigne, 1 au minimum), puis le plus de caractères ;
  - l'ordre de la liste ne change pas, et une saisie vide montre tout.
- **Navigation :**
  - ↑, Ctrl+K et Ctrl+P montent ; ↓, Ctrl+J et Ctrl+N descendent ; Tab descend, Shift+Tab monte ; la liste boucle ;
  - Entrée ouvre un sous-groupe, entre en saisie libre, ou exécute puis ferme ;
  - Échap quitte la saisie, puis remonte d'un niveau, puis ferme ;
  - sans recherche, le curseur se place sur la valeur en cours.
- **Libellés :**
  - `display` du réglage avec une majuscule et « ... » (« Smooth caret... »). Exceptions : `timerColor` devient « Live progress color... », `showKeyTips` « Key tips... », `words` « Word count... » ;
  - valeurs `on`/`off`, ou la valeur avec `_` remplacé par une espace ; `off` en premier ;
  - time `15 30 60 120 custom...`, words `10 25 50 100 custom...`, quote length `all short medium long thicc` (coché si la config contient ces groupes).
- **Bail out** (`lists/bail-out.ts`) : proposé pendant un test zen, time ≥ 3600 s ou infini, words ≥ 5000 ou infini, ou custom long. Il ouvre « Are you sure... » avec « Nevermind » et « Yes, I am sure ».
- **Rendu :** la ligne active est inversée (`bg-text text-bg`), les autres sont en `sub`, et une coche est alignée devant la valeur en cours. Le fond est assombri.
- **Pied de page :** `tab + enter - restart   esc - command line`, adapté à `quickRestart` et masqué si `showKeyTips` est off.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Textes de l'interface en anglais, identiques à Monkeytype, y compris les messages de validation (« Must be a whole number »). Commentaires en français.
- La palette ne modifie ni la config ni le test : elle renvoie une `Action`, que `App` exécute.
- Un réglage choisi est enregistré tout de suite (`save_config`, écriture atomique). Une clé de test (mode, time, words, quoteLength, language, punctuation, numbers) relance un test, avec ses fondus.
- **Pendant que la palette est ouverte :**
  - aucune touche n'atteint le test, mais le test continue (le timer tourne, comme sur le site) ;
  - le caret est caché ;
  - le clignotement ne réveille plus la boucle.
- Le coût par frappe reste celui de 4b : benchmark sous 1 ms.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace, tous au vert.

**Choix assumés :**
- **Seuls les réglages qui ont un effet en v1** sont proposés (spec §5.3), soit une vingtaine. Les autres arrivent avec la v2.
- **Reporté au plan 4e :**
  - Change custom text, Search for quotes ;
  - Export et Import settings ;
  - les notifications d'erreur persistantes ;
  - le chargement des grosses langues en arrière-plan.
- **Pas de mode « liste unique »** (`>` ou `singleListCommandLine`) : le `>` initial est accepté et ignoré.
- **Pas de souris** (survol, clic) : le terminal n'est pas en mode souris.

## Review Focus

1. **Test qui se termine pendant que la palette est ouverte** (time 15). La palette doit rester utilisable et le résultat apparaître derrière. Test : `a_test_that_ends_behind_the_command_line_keeps_it_usable` (tâche 2).
2. **Aperçu d'un thème puis annulation.** Le thème d'origine doit revenir et la config rester inchangée. Choisir un thème doit l'appliquer et l'enregistrer. Tests : `hovering_a_theme_previews_it_and_escape_reverts`, `choosing_a_theme_applies_it` (tâche 2).
3. **Collage multiligne dans la recherche ou une saisie libre.** Les sauts de ligne deviennent des espaces, rien n'est validé au premier saut, et un collage palette fermée est ignoré. Tests : `pasted_text_goes_to_the_search` (tâche 2), `custom_values_are_typed_and_checked` (tâche 1).
4. **Mots agrandis sous la palette.** La boîte doit rester lisible et aucune lettre agrandie la recouvrir. Le caret doit être caché. Test : `scaled_words_make_room_for_the_command_line` (tâche 2). Pour la réécriture partielle, vérifier par lecture que `write_changes` contourne la boîte.
5. **Terminal minuscule, avec ou sans message d'erreur de saisie.** Aucune panique. Test : `command_line_fits_any_terminal_size` (tâche 2).

---

## Structure des fichiers

```
crates/fasttype-tui/
├── Cargo.toml                     + toml
├── src/palette/mod.rs             Action, AppAction, Command, Subgroup
├── src/palette/filter.rs          split_words, query_words, score, filter
├── src/palette/lists.rs           Context, root (réglages de la v1, Next/Repeat test, Bail out, Quit)
├── src/palette/state.rs           PaletteState, Outcome, InputMode, parse_input
├── src/input.rs                   + Key::{Up, Down, Palette}, Input::Paste ; Input n'est plus Copy
├── src/view/palette.rs            dim, palette_rect, render
├── src/view/notify.rs             + clear
├── src/view/test.rs               key_tips ; Chrome sans `tips`
├── src/sized.rs                   ScaledText.hole
├── src/app.rs                     ouverture, routage, exécution, aperçu des thèmes
├── src/terminal.rs                bracketed paste
├── tests/palette.rs               nouveau (logique pure)
├── tests/command_line.rs          nouveau (intégration)
└── tests/sized.rs                 hole: None
scripts/pty_smoke.py               + --palette, contrôle du bracketed paste
```

---

### Task 1 : logique de la palette

**Files:**
- Create: `crates/fasttype-tui/src/palette/mod.rs`, `crates/fasttype-tui/src/palette/filter.rs`, `crates/fasttype-tui/src/palette/lists.rs`, `crates/fasttype-tui/src/palette/state.rs`
- Modify: `crates/fasttype-tui/src/input.rs` (version complète), `crates/fasttype-tui/src/lib.rs` (`pub mod palette;` après `layout`), `crates/fasttype-tui/Cargo.toml` (`toml.workspace = true` après `unicode-width.workspace = true`)
- Test: `crates/fasttype-tui/tests/palette.rs`

**Interfaces:**
- Consumes : `fasttype_store::schema::{key_def, Kind}`, `Config::get`.
- Produces :
  - `palette::{Action { Set { key, value }, Open(Subgroup), Input { key }, App(AppAction), Close }, AppAction { NextTest, RepeatTest, BailOut, ClearNotifications, Quit }, Command, Subgroup}` ;
  - `palette::filter::{split_words, query_words, score, filter}` ;
  - `palette::lists::{Context { config, on_result, can_bail_out, languages, themes }, root}` ;
  - `palette::state::{PaletteState, Outcome { Stay, Close, Run(Action) }, InputMode, parse_input}`, avec `PaletteState::{open, key(Key, &Config), paste, title, query, input, shown, hovered}` ;
  - `input::Key::{Up, Down, Palette}` et `input::Input::Paste(String)`. `Input` perd `Copy` (il contient une `String`).

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/palette.rs` :
```rust
mod common;

use common::app;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fasttype_tui::input::{Input, Key, map_event, map_key};
use fasttype_tui::palette::filter::{filter, split_words};
use fasttype_tui::palette::lists::{Context, root};
use fasttype_tui::palette::state::{Outcome, PaletteState, parse_input};
use fasttype_tui::palette::{Action, AppAction, Subgroup};
use toml::Value;

fn words(labels: &[&str]) -> Vec<Vec<String>> {
    labels.iter().map(|l| split_words(l)).collect()
}

fn run(input: &str, labels: &[&str]) -> Vec<String> {
    let w = words(labels);
    let refs: Vec<Option<&[String]>> = w.iter().map(|v| Some(v.as_slice())).collect();
    filter(input, &refs)
        .into_iter()
        .map(|i| labels[i].to_string())
        .collect()
}

const LABELS: &[&str] = &[
    "Smooth caret...",
    "Caret style...",
    "Smooth line scroll...",
    "Theme...",
    "Timer style...",
];

#[test]
fn filter_keeps_the_best_word_prefix_matches_in_list_order() {
    assert_eq!(run("", LABELS).len(), 5, "saisie vide : tout");
    assert_eq!(run("sm car", LABELS), ["Smooth caret..."]);
    assert_eq!(run("caret", LABELS), ["Smooth caret...", "Caret style..."]);
    assert_eq!(run("t", LABELS), ["Theme...", "Timer style..."]);
    // un mot sans correspondance : on retombe sur les meilleures commandes
    assert_eq!(
        run("caret zzz", LABELS),
        ["Smooth caret...", "Caret style..."]
    );
    assert_eq!(run(">smooth  ca", LABELS), ["Smooth caret..."]);
    assert!(run("qqq", LABELS).is_empty());
}

#[test]
fn an_input_word_claims_a_single_label_word() {
    // « s s » : deux mots saisis, il faut deux mots du libellé commençant par s
    assert_eq!(run("s s", LABELS), ["Smooth line scroll..."]);
}

fn ctx_root(config: &fasttype_store::Config, on_result: bool, bail: bool) -> Subgroup {
    let languages = ["english", "english_1k", "french"];
    let themes = ["dracula", "serika_dark"];
    root(&Context {
        config,
        on_result,
        can_bail_out: bail,
        languages: &languages,
        themes: &themes,
    })
}

fn labels(g: &Subgroup) -> Vec<&str> {
    g.list.iter().map(|c| c.display.as_str()).collect()
}

fn sub<'a>(g: &'a Subgroup, label: &str) -> &'a Subgroup {
    match &g.list.iter().find(|c| c.display == label).unwrap().action {
        Action::Open(s) => s,
        other => panic!("{label} : {other:?}"),
    }
}

#[test]
fn root_list_follows_the_site_order() {
    let a = app("pal-root", "");
    let r = ctx_root(&a.store.config, false, false);
    let l = labels(&r);
    assert_eq!(
        &l[..7],
        [
            "Punctuation...",
            "Numbers...",
            "Mode...",
            "Time...",
            "Word count...",
            "Quote length...",
            "Language..."
        ]
    );
    assert!(l.contains(&"Smooth caret..."));
    assert!(l.contains(&"Live progress color..."));
    assert!(l.contains(&"Key tips..."));
    assert!(!l.contains(&"Next test"), "pas sur l'écran de test");
    assert!(!l.contains(&"Bail out..."));
    assert_eq!(l.last(), Some(&"Quit"));
    let r = ctx_root(&a.store.config, true, true);
    assert_eq!(&labels(&r)[..2], ["Next test", "Repeat test"]);
    assert!(labels(&r).contains(&"Bail out..."));
}

#[test]
fn subgroups_list_values_with_the_current_one_checked() {
    let a = app("pal-sub", "smooth_caret = \"fast\"\n");
    let r = ctx_root(&a.store.config, false, false);
    let caret = sub(&r, "Smooth caret...");
    assert_eq!(labels(caret), ["off", "slow", "medium", "fast"]);
    assert!(caret.list[3].active && !caret.list[2].active);
    let time = sub(&r, "Time...");
    assert_eq!(labels(time), ["15", "30", "60", "120", "custom..."]);
    assert!(time.list[1].active, "30 par défaut");
    assert_eq!(time.list[4].action, Action::Input { key: "time" });
    let punct = sub(&r, "Punctuation...");
    assert_eq!(labels(punct), ["off", "on"]);
    let lang = sub(&r, "Language...");
    assert_eq!(labels(lang), ["english", "english 1k", "french"]);
    let theme = sub(&r, "Theme...");
    assert_eq!(theme.list[0].preview.as_deref(), Some("dracula"));
    assert!(theme.list[1].active);
    let quote = sub(&r, "Quote length...");
    assert_eq!(labels(quote), ["all", "short", "medium", "long", "thicc"]);
    assert!(quote.list[2].active, "medium par défaut");
    // les nombres libres s'éditent directement
    let font = r.list.iter().find(|c| c.display == "Font size...").unwrap();
    assert_eq!(font.action, Action::Input { key: "fontSize" });
}

#[test]
fn navigation_search_and_subgroups() {
    let a = app("pal-nav", "");
    let c = &a.store.config;
    let mut p = PaletteState::open(ctx_root(c, false, false));
    assert_eq!(p.title(), "Search...");
    for ch in "time".chars() {
        assert_eq!(p.key(Key::Char(ch), c), Outcome::Stay);
    }
    assert_eq!(p.hovered().unwrap().display, "Time...");
    p.key(Key::Enter, c);
    assert_eq!(p.title(), "Time");
    assert_eq!(p.query(), "", "la recherche repart à vide");
    assert_eq!(
        p.hovered().unwrap().display,
        "30",
        "curseur sur la valeur en cours"
    );
    p.key(Key::Down, c);
    assert_eq!(p.hovered().unwrap().display, "60");
    p.key(Key::Up, c);
    p.key(Key::Up, c);
    assert_eq!(p.hovered().unwrap().display, "15");
    p.key(Key::Up, c);
    assert_eq!(p.hovered().unwrap().display, "custom...", "on boucle");
    p.key(Key::Tab, c);
    assert_eq!(p.hovered().unwrap().display, "15", "Tab descend");
    assert_eq!(
        p.key(Key::Esc, c),
        Outcome::Stay,
        "Échap remonte d'un niveau"
    );
    assert_eq!(p.title(), "Search...");
    assert_eq!(p.key(Key::Esc, c), Outcome::Close);
    // choisir une valeur
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "time".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    p.key(Key::Down, c);
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::Set {
            key: "time",
            value: Value::Integer(60)
        })
    );
}

#[test]
fn custom_values_are_typed_and_checked() {
    let a = app("pal-input", "");
    let c = &a.store.config;
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "time".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    // depuis « 30 » : « 15 », puis on boucle sur « custom... »
    p.key(Key::Up, c);
    p.key(Key::Up, c);
    p.key(Key::Enter, c);
    let m = p.input().expect("mode saisie");
    assert_eq!(m.title, "custom...");
    assert_eq!(m.text, "30", "valeur en cours");
    p.key(Key::Backspace, c);
    p.key(Key::Backspace, c);
    p.key(Key::Char('x'), c);
    assert_eq!(p.key(Key::Enter, c), Outcome::Stay);
    assert_eq!(
        p.input().unwrap().error.as_deref(),
        Some("Must be a whole number")
    );
    p.key(Key::Backspace, c);
    p.paste("45");
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::Set {
            key: "time",
            value: Value::Integer(45)
        })
    );
    // Échap quitte la saisie, pas la palette
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "font".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    assert_eq!(p.input().unwrap().text, "2");
    assert_eq!(p.key(Key::Esc, c), Outcome::Stay);
    assert!(p.input().is_none());
}

#[test]
fn parse_input_messages() {
    assert_eq!(parse_input("time", "45"), Ok(Value::Integer(45)));
    assert_eq!(parse_input("time", " 0 "), Ok(Value::Integer(0)));
    assert_eq!(parse_input("time", "-1"), Err("Must be at least 0".into()));
    assert_eq!(parse_input("fontSize", "1.5"), Ok(Value::Float(1.5)));
    assert_eq!(
        parse_input("fontSize", "0"),
        Err("Must be greater than 0".into())
    );
    assert_eq!(parse_input("maxLineWidth", "0"), Ok(Value::Integer(0)));
    assert_eq!(parse_input("maxLineWidth", "80"), Ok(Value::Integer(80)));
    assert_eq!(
        parse_input("maxLineWidth", "10"),
        Err("Must be 0, or between 20 and 1000".into())
    );
    assert_eq!(
        parse_input("fontSize", "big"),
        Err("Must be a number".into())
    );
}

#[test]
fn bail_out_asks_for_confirmation() {
    let a = app("pal-bail", "");
    let c = &a.store.config;
    let r = ctx_root(c, false, true);
    let bail = sub(&r, "Bail out...");
    assert_eq!(bail.title, "Are you sure...");
    assert_eq!(labels(bail), ["Nevermind", "Yes, I am sure"]);
    assert_eq!(bail.list[0].action, Action::Close);
    assert_eq!(bail.list[1].action, Action::App(AppAction::BailOut));
}

fn press(code: KeyCode, mods: KeyModifiers) -> Option<Key> {
    map_key(&KeyEvent::new_with_kind(code, mods, KeyEventKind::Press)).map(|(k, _)| k)
}

#[test]
fn palette_keys_are_mapped() {
    let ctrl = KeyModifiers::CONTROL;
    assert_eq!(press(KeyCode::Up, KeyModifiers::NONE), Some(Key::Up));
    assert_eq!(press(KeyCode::Down, KeyModifiers::NONE), Some(Key::Down));
    for c in ['k', 'p'] {
        assert_eq!(press(KeyCode::Char(c), ctrl), Some(Key::Up));
    }
    for c in ['j', 'n'] {
        assert_eq!(press(KeyCode::Char(c), ctrl), Some(Key::Down));
    }
    assert_eq!(
        press(KeyCode::Char('p'), ctrl | KeyModifiers::SHIFT),
        Some(Key::Palette)
    );
    assert_eq!(
        press(KeyCode::Char('P'), ctrl | KeyModifiers::SHIFT),
        Some(Key::Palette)
    );
    assert_eq!(
        map_event(Event::Paste("a\nb".into()), 0.0),
        Some(Input::Paste("a\nb".into()))
    );
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test palette`
Expected: échec de compilation (`unresolved import fasttype_tui::palette`, `no variant Up`).

- [ ] **Step 3 : implémenter**

`crates/fasttype-tui/src/palette/mod.rs` :
```rust
//! Palette de commandes (`commandline/`) : listes construites depuis le
//! schéma de la config, recherche du site, navigation dans les sous-groupes,
//! saisie de valeurs libres. Logique pure : `App` exécute les actions.

pub mod filter;
pub mod lists;
pub mod state;

use toml::Value;

/// Ce que fait une commande.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Change une clé de config.
    Set { key: &'static str, value: Value },
    /// Ouvre un sous-groupe.
    Open(Subgroup),
    /// Demande une valeur libre pour une clé (« custom... »).
    Input { key: &'static str },
    /// Action propre à l'application.
    App(AppAction),
    /// Ne fait rien et ferme la palette (« Nevermind »).
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    NextTest,
    RepeatTest,
    BailOut,
    ClearNotifications,
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub display: String,
    pub alias: String,
    pub action: Action,
    /// Valeur en cours : coche devant le libellé.
    pub active: bool,
    /// Commande qui a une valeur de config (coche visible ou réservée).
    pub checkable: bool,
    /// Thème montré au survol (aperçu).
    pub preview: Option<String>,
    /// Mots du libellé et des alias, pour la recherche.
    pub words: Vec<String>,
}

impl Command {
    pub fn new(display: impl Into<String>, action: Action) -> Self {
        let display = display.into();
        Command {
            words: filter::split_words(&display),
            display,
            alias: String::new(),
            action,
            active: false,
            checkable: false,
            preview: None,
        }
    }

    pub fn alias(mut self, alias: &str) -> Self {
        self.alias = alias.to_string();
        self.words = filter::split_words(&self.display);
        self.words.extend(filter::split_words(alias));
        self
    }

    pub fn checked(mut self, active: bool) -> Self {
        self.checkable = true;
        self.active = active;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subgroup {
    pub title: String,
    pub list: Vec<Command>,
}
```

`crates/fasttype-tui/src/palette/filter.rs` :
```rust
//! Recherche de la palette, comme `commandline/filter.ts` : chaque mot saisi
//! doit être le début d'un mot du libellé ou d'un alias (un mot du libellé
//! ne sert qu'une fois). On garde les commandes qui apparient le plus de
//! mots, puis le plus de caractères. L'ordre de la liste ne change pas.

/// Retire la ponctuation ASCII, comme `stripPunctuation`.
fn strip_punctuation(s: &str) -> String {
    s.chars().filter(|c| !c.is_ascii_punctuation()).collect()
}

/// Mots en minuscules, sans ponctuation (`splitWords`).
pub fn split_words(s: &str) -> Vec<String> {
    s.to_lowercase().split(' ').map(strip_punctuation).collect()
}

/// Mots de la saisie : sans `>` initial, sans mots vides.
pub fn query_words(input: &str) -> Vec<String> {
    let s = input.trim_start_matches('>').to_lowercase();
    s.trim()
        .split(' ')
        .map(strip_punctuation)
        .filter(|w| !w.is_empty())
        .collect()
}

/// (mots appariés, caractères appariés) d'une commande dont les mots du
/// libellé et des alias sont `words`.
pub fn score(query: &[String], words: &[String]) -> (usize, usize) {
    let mut claimed: Vec<Option<usize>> = vec![None; words.len()];
    let mut strength = 0;
    for (qi, q) in query.iter().enumerate() {
        for (wi, w) in words.iter().enumerate() {
            if w.starts_with(q.as_str()) && claimed[wi].is_none() && !claimed.contains(&Some(qi)) {
                claimed[wi] = Some(qi);
                strength += q.len();
            }
        }
    }
    (claimed.iter().filter(|c| c.is_some()).count(), strength)
}

/// Indices des commandes à montrer. `words[i]` : mots du libellé et des
/// alias de la commande `i` ; `None` si elle n'est pas disponible.
pub fn filter(input: &str, words: &[Option<&[String]>]) -> Vec<usize> {
    let query = query_words(input);
    if query.is_empty() {
        return (0..words.len()).filter(|&i| words[i].is_some()).collect();
    }
    let scores: Vec<Option<(usize, usize)>> =
        words.iter().map(|w| w.map(|w| score(&query, w))).collect();
    let max_strength = scores.iter().flatten().map(|s| s.1).max().unwrap_or(0);
    let mut min_count = query.len();
    while min_count > 0 && !scores.iter().flatten().any(|s| s.0 >= min_count) {
        min_count -= 1;
    }
    let min_count = min_count.max(1);
    (0..words.len())
        .filter(|&i| scores[i].is_some_and(|(c, s)| c >= min_count && s >= max_strength))
        .collect()
}
```

`crates/fasttype-tui/src/palette/lists.rs` :
```rust
//! Liste racine de la palette (`commandline/lists.ts`), limitée aux réglages
//! qui ont un effet dans fasttype v1. Libellés : `display` du schéma avec une
//! majuscule et « ... » ; dans un sous-groupe, `on`/`off` ou la valeur, `off`
//! en premier.

use super::{Action, AppAction, Command, Subgroup};
use fasttype_store::Config;
use fasttype_store::schema::{Kind, key_def};
use toml::Value;

/// État de l'application utile aux listes.
pub struct Context<'a> {
    pub config: &'a Config,
    /// L'écran de résultat est affiché (entrées Next test, Repeat test).
    pub on_result: bool,
    /// Un test long ou zen est en cours (`canBailOut`).
    pub can_bail_out: bool,
    pub languages: &'a [&'a str],
    pub themes: &'a [&'a str],
}

/// Réglages de la v1, par catégorie, dans l'ordre du site.
const TEST: &[&str] = &[
    "punctuation",
    "numbers",
    "mode",
    "time",
    "words",
    "quoteLength",
    "language",
];
const BEHAVIOR: &[&str] = &["quickRestart"];
const CARET: &[&str] = &["smoothCaret", "caretStyle"];
const APPEARANCE: &[&str] = &[
    "timerStyle",
    "liveSpeedStyle",
    "liveAccStyle",
    "liveBurstStyle",
    "timerColor",
    "timerOpacity",
    "smoothLineScroll",
    "typingSpeedUnit",
    "alwaysShowDecimalPlaces",
    "startGraphsAtZero",
    "maxLineWidth",
    "fontSize",
];
const THEME: &[&str] = &["theme", "flipTestColors", "colorfulMode"];
const SHOW_HIDE: &[&str] = &["showKeyTips"];

/// « Smooth caret... » ; quelques libellés du site diffèrent du schéma.
fn label(key: &str) -> String {
    let display = match key {
        "timerColor" => "live progress color",
        "showKeyTips" => "key tips",
        "words" => "word count",
        other => key_def(other).map_or(other, |d| d.display),
    };
    let mut c = display.chars();
    match c.next() {
        Some(f) => format!("{}{}...", f.to_uppercase(), c.as_str()),
        None => String::new(),
    }
}

fn value_label(v: &Value) -> String {
    match v {
        Value::Boolean(true) => "on".into(),
        Value::Boolean(false) => "off".into(),
        Value::String(s) => s.replace('_', " "),
        other => other.to_string(),
    }
}

fn set(key: &'static str, value: Value, config: &Config) -> Command {
    let active = config.get(key) == Some(&value);
    Command::new(value_label(&value), Action::Set { key, value }).checked(active)
}

fn custom(key: &'static str) -> Command {
    Command::new("custom...", Action::Input { key })
}

/// Sous-groupe d'une clé : ses valeurs (`off` en premier) et, pour les
/// nombres, une saisie libre.
fn values(key: &'static str, ctx: &Context) -> Vec<Command> {
    let c = ctx.config;
    let def = key_def(key).expect("clé de la v1 présente dans le schéma");
    let ints = |list: &[i64]| -> Vec<Command> {
        let mut v: Vec<Command> = list
            .iter()
            .map(|n| set(key, Value::Integer(*n), c))
            .collect();
        v.push(custom(key));
        v
    };
    let mut list = match (key, &def.kind) {
        ("time", _) => ints(&[15, 30, 60, 120]),
        ("words", _) => ints(&[10, 25, 50, 100]),
        ("quoteLength", _) => {
            let current = c.int_list("quoteLength");
            [
                ("all", vec![0, 1, 2, 3]),
                ("short", vec![0]),
                ("medium", vec![1]),
                ("long", vec![2]),
                ("thicc", vec![3]),
            ]
            .into_iter()
            .map(|(name, groups)| {
                // `configValueMode: include` : coché si la config contient ces groupes
                let active = groups.iter().all(|g| current.contains(g));
                let value = Value::Array(groups.into_iter().map(Value::Integer).collect());
                Command::new(name, Action::Set { key, value }).checked(active)
            })
            .collect()
        }
        ("language", _) => ctx
            .languages
            .iter()
            .map(|n| set(key, Value::String(n.to_string()), c))
            .collect(),
        ("theme", _) => ctx
            .themes
            .iter()
            .map(|n| {
                let mut cmd = set(key, Value::String(n.to_string()), c);
                cmd.preview = Some(n.to_string());
                cmd
            })
            .collect(),
        (_, Kind::Bool) => [false, true]
            .into_iter()
            .map(|b| set(key, Value::Boolean(b), c))
            .collect(),
        (_, Kind::Choice(choices)) => choices
            .iter()
            .map(|s| set(key, Value::String(s.to_string()), c))
            .collect(),
        _ => Vec::new(),
    };
    // `off` et `false` en premier, l'ordre des autres ne change pas
    list.sort_by_key(|cmd| {
        !matches!(
            &cmd.action,
            Action::Set {
                value: Value::Boolean(false),
                ..
            }
        ) && !matches!(&cmd.action, Action::Set { value: Value::String(s), .. } if s == "off")
    });
    list
}

/// Commande d'une clé : sous-groupe de ses valeurs, ou saisie directe pour
/// les nombres libres (`fontSize`, `maxLineWidth`).
fn key_command(key: &'static str, ctx: &Context) -> Command {
    let title = label(key);
    let list = values(key, ctx);
    if list.is_empty() {
        return Command::new(title, Action::Input { key });
    }
    Command::new(
        title.clone(),
        Action::Open(Subgroup {
            title: title.trim_end_matches("...").to_string(),
            list,
        }),
    )
}

pub fn root(ctx: &Context) -> Subgroup {
    let mut list = Vec::new();
    if ctx.on_result {
        list.push(Command::new("Next test", Action::App(AppAction::NextTest)));
        list.push(Command::new(
            "Repeat test",
            Action::App(AppAction::RepeatTest),
        ));
    }
    for group in [TEST, BEHAVIOR, CARET, APPEARANCE, THEME, SHOW_HIDE] {
        list.extend(group.iter().map(|k| key_command(k, ctx)));
        if group == TEST && ctx.can_bail_out {
            list.push(Command::new(
                "Bail out...",
                Action::Open(Subgroup {
                    title: "Are you sure...".into(),
                    list: vec![
                        Command::new("Nevermind", Action::Close),
                        Command::new("Yes, I am sure", Action::App(AppAction::BailOut)),
                    ],
                }),
            ));
        }
    }
    list.push(
        Command::new(
            "Clear all notifications",
            Action::App(AppAction::ClearNotifications),
        )
        .alias("dismiss"),
    );
    list.push(Command::new("Quit", Action::App(AppAction::Quit)).alias("exit close"));
    Subgroup {
        title: "Search...".into(),
        list,
    }
}
```

`crates/fasttype-tui/src/palette/state.rs` :
```rust
//! Palette ouverte (`CommandlineModal.tsx`) : pile de sous-groupes, recherche,
//! commande active, saisie libre. Ne touche ni à la config ni au test : elle
//! renvoie l'action choisie à `App`.

use super::filter::filter;
use super::{Action, Command, Subgroup};
use crate::input::Key;
use fasttype_store::Config;
use fasttype_store::schema::{Kind, key_def};
use toml::Value;

/// Réponse de la palette à une touche.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Rester ouverte.
    Stay,
    /// Fermer sans rien faire.
    Close,
    /// Exécuter l'action, puis fermer.
    Run(Action),
}

/// Saisie d'une valeur libre (« custom... »).
#[derive(Debug, Clone, PartialEq)]
pub struct InputMode {
    pub key: &'static str,
    pub title: String,
    pub text: String,
    /// Message sous la saisie quand la valeur est refusée.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteState {
    stack: Vec<Subgroup>,
    query: String,
    /// Indices des commandes montrées (filtrées), dans l'ordre de la liste.
    shown: Vec<usize>,
    /// Position de la commande active dans `shown`.
    active: usize,
    input: Option<InputMode>,
}

/// Valeur d'une clé, à éditer dans la saisie libre.
fn current_text(config: &Config, key: &str) -> String {
    match config.get(key) {
        Some(Value::Integer(n)) => n.to_string(),
        Some(Value::Float(f)) if f.fract() == 0.0 => format!("{f:.0}"),
        Some(Value::Float(f)) => f.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Convertit et vérifie une saisie libre (messages en anglais, comme le site).
pub fn parse_input(key: &str, text: &str) -> Result<Value, String> {
    let def = key_def(key).ok_or_else(|| format!("Unknown setting {key}"))?;
    let t = text.trim();
    let number = || t.parse::<f64>().ok().filter(|n| n.is_finite());
    let as_value = |n: f64| {
        if n.fract() == 0.0 {
            Value::Integer(n as i64)
        } else {
            Value::Float(n)
        }
    };
    match def.kind {
        Kind::Int { min } => match t.parse::<i64>() {
            Ok(n) if n >= min => Ok(Value::Integer(n)),
            Ok(_) => Err(format!("Must be at least {min}")),
            Err(_) => Err("Must be a whole number".into()),
        },
        Kind::Positive => match number() {
            Some(n) if n > 0.0 => Ok(Value::Float(n)),
            Some(_) => Err("Must be greater than 0".into()),
            None => Err("Must be a number".into()),
        },
        Kind::MaxLineWidth => match number() {
            Some(n) if n == 0.0 || (20.0..=1000.0).contains(&n) => Ok(as_value(n)),
            Some(_) => Err("Must be 0, or between 20 and 1000".into()),
            None => Err("Must be a number".into()),
        },
        Kind::Number { min, max } => match number() {
            Some(n) if n >= min && max.is_none_or(|m| n <= m) => Ok(as_value(n)),
            Some(_) => Err(match max {
                Some(m) => format!("Must be between {min} and {m}"),
                None => format!("Must be at least {min}"),
            }),
            None => Err("Must be a number".into()),
        },
        _ if t.is_empty() => Err("Must not be empty".into()),
        _ => Ok(Value::String(t.to_string())),
    }
}

impl PaletteState {
    pub fn open(root: Subgroup) -> Self {
        let mut p = PaletteState {
            stack: vec![root],
            query: String::new(),
            shown: Vec::new(),
            active: 0,
            input: None,
        };
        p.refresh();
        p
    }

    fn group(&self) -> &Subgroup {
        self.stack.last().expect("la pile n'est jamais vide")
    }

    /// Refait la liste montrée. Sans recherche, la commande active est la
    /// valeur en cours ; avec une recherche, la première.
    fn refresh(&mut self) {
        let words: Vec<Option<&[String]>> = self
            .group()
            .list
            .iter()
            .map(|c| Some(c.words.as_slice()))
            .collect();
        self.shown = filter(&self.query, &words);
        self.active = if self.query.trim().is_empty() {
            let list = &self.group().list;
            self.shown.iter().position(|&i| list[i].active).unwrap_or(0)
        } else {
            0
        };
    }

    pub fn title(&self) -> &str {
        &self.group().title
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn input(&self) -> Option<&InputMode> {
        self.input.as_ref()
    }

    /// Commandes montrées et position de l'active.
    pub fn shown(&self) -> (Vec<&Command>, usize) {
        let list = &self.group().list;
        (self.shown.iter().map(|&i| &list[i]).collect(), self.active)
    }

    /// Commande sous le curseur (pour l'aperçu des thèmes).
    pub fn hovered(&self) -> Option<&Command> {
        if self.input.is_some() {
            return None;
        }
        self.shown.get(self.active).map(|&i| &self.group().list[i])
    }

    fn step(&mut self, delta: isize) {
        let n = self.shown.len() as isize;
        if n > 0 {
            self.active = (self.active as isize + delta).rem_euclid(n) as usize;
        }
    }

    /// Texte collé (bracketed paste) : ajouté à la saisie en cours.
    pub fn paste(&mut self, text: &str) {
        let flat: String = text
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        match &mut self.input {
            Some(m) => {
                m.text.push_str(&flat);
                m.error = None;
            }
            None => {
                self.query.push_str(&flat);
                self.refresh();
            }
        }
    }

    pub fn key(&mut self, key: Key, config: &Config) -> Outcome {
        if self.input.is_some() {
            return self.input_key(key);
        }
        match key {
            Key::Esc => {
                if self.stack.len() > 1 {
                    self.stack.pop();
                    self.query.clear();
                    self.refresh();
                    Outcome::Stay
                } else {
                    Outcome::Close
                }
            }
            Key::Up | Key::BackTab => {
                self.step(-1);
                Outcome::Stay
            }
            Key::Down | Key::Tab => {
                self.step(1);
                Outcome::Stay
            }
            Key::Char(c) => {
                self.query.push(c);
                self.refresh();
                Outcome::Stay
            }
            Key::Backspace => {
                self.query.pop();
                self.refresh();
                Outcome::Stay
            }
            Key::DeleteWord => {
                let kept = self.query.trim_end().rfind(' ').map_or(0, |i| i + 1);
                self.query.truncate(kept);
                self.refresh();
                Outcome::Stay
            }
            Key::Enter | Key::ShiftEnter => {
                let Some(cmd) = self.hovered().cloned() else {
                    return Outcome::Stay;
                };
                match cmd.action {
                    Action::Open(group) => {
                        self.stack.push(group);
                        self.query.clear();
                        self.refresh();
                        Outcome::Stay
                    }
                    Action::Input { key } => {
                        self.input = Some(InputMode {
                            key,
                            title: cmd.display.clone(),
                            text: current_text(config, key),
                            error: None,
                        });
                        Outcome::Stay
                    }
                    Action::Close => Outcome::Close,
                    action => Outcome::Run(action),
                }
            }
            _ => Outcome::Stay,
        }
    }

    fn input_key(&mut self, key: Key) -> Outcome {
        let m = self.input.as_mut().expect("mode saisie");
        match key {
            Key::Esc => {
                self.input = None;
                Outcome::Stay
            }
            Key::Char(c) => {
                m.text.push(c);
                m.error = None;
                Outcome::Stay
            }
            Key::Backspace => {
                m.text.pop();
                m.error = None;
                Outcome::Stay
            }
            Key::DeleteWord => {
                let kept = m.text.trim_end().rfind(' ').map_or(0, |i| i + 1);
                m.text.truncate(kept);
                m.error = None;
                Outcome::Stay
            }
            Key::Enter | Key::ShiftEnter => match parse_input(m.key, &m.text) {
                Ok(value) => Outcome::Run(Action::Set { key: m.key, value }),
                Err(e) => {
                    m.error = Some(e);
                    Outcome::Stay
                }
            },
            _ => Outcome::Stay,
        }
    }
}
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
```

- [ ] **Step 4 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (9 dans `palette`).

- [ ] **Step 5 : commit**

```bash
git add Cargo.lock crates/fasttype-tui
git commit -m "feat(tui): logique de la palette de commandes (recherche du site, listes, navigation, saisie libre)"
```

---

### Task 2 : palette à l'écran et dans l'application

**Files:**
- Create: `crates/fasttype-tui/src/view/palette.rs`
- Modify: `crates/fasttype-tui/src/view/mod.rs` (`pub mod palette;` après `notify`), `crates/fasttype-tui/src/view/notify.rs` (`clear`)
- Modify: `crates/fasttype-tui/src/view/test.rs`, `crates/fasttype-tui/src/sized.rs`, `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/terminal.rs` (versions complètes)
- Modify: `crates/fasttype-tui/tests/sized.rs` (version complète), `scripts/pty_smoke.py` (version complète)
- Test: `crates/fasttype-tui/tests/command_line.rs`

**Interfaces:**
- Consumes : tâche 1.
- Produces :
  - `view::palette::{dim, dim_color, dim_style, palette_rect, render, MAX_ROWS}` ;
  - `Notifications::clear` ;
  - `view::test::key_tips(&Config) -> Vec<(String, bool)>` ; `Chrome` perd le champ `tips` ;
  - `ScaledText.hole: Option<Rect>` (`write` et `write_changes` contournent cette zone) ;
  - `App::command_line() -> Option<&PaletteState>`.
- Comportement :
  - `Notifications::clear` ajoute à `view/notify.rs`, avant `expire`, la méthode
    ```rust
    /// « Clear all notifications ».
    pub fn clear(&mut self) {
        self.items.clear();
    }
    ```
  - le bracketed paste est activé à l'entrée en plein écran et désactivé par `restore` ;
  - le pied de page indique la touche de la palette.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/command_line.rs` :
```rust
mod common;

use common::{app, press, render, row, screen_text, settle, type_text, type_whole_test};
use fasttype_core::session::SessionState;
use fasttype_tui::app::{App, Screen};
use fasttype_tui::input::{Input, Key};

fn type_query(a: &mut App, text: &str, at: f64) {
    for c in text.chars() {
        a.handle(press(Key::Char(c), at));
    }
}

#[test]
fn escape_opens_and_closes_the_command_line() {
    let mut a = app("cl-open", "");
    a.handle(press(Key::Esc, 0.0));
    assert!(a.command_line().is_some());
    // les lettres vont à la recherche, pas au test
    type_query(&mut a, "caret", 10.0);
    assert_eq!(a.session().state(), SessionState::Ready);
    assert_eq!(a.command_line().unwrap().query(), "caret");
    a.handle(press(Key::Esc, 20.0));
    assert!(a.command_line().is_none());
    a.handle(press(Key::Palette, 30.0));
    assert!(a.command_line().is_some(), "Ctrl+Shift+P aussi");
}

#[test]
fn with_quick_restart_on_escape_tab_opens_it() {
    let mut a = app("cl-tab", "quick_restart = \"esc\"\n");
    a.handle(press(Key::Tab, 0.0));
    assert!(a.command_line().is_some());
    a.handle(press(Key::Esc, 10.0));
    assert!(a.command_line().is_none());
    a.handle(press(Key::Esc, 20.0));
    assert!(a.command_line().is_none(), "Échap relance");
}

#[test]
fn choosing_a_value_saves_it_and_restarts() {
    let mut a = app("cl-set", "");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "time", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Down, 0.0));
    a.handle(press(Key::Enter, 0.0));
    assert!(a.command_line().is_none());
    assert_eq!(a.store.config.int("time"), 60);
    let dir = std::env::temp_dir().join(format!("fasttype-tui-{}-cl-set", std::process::id()));
    let saved = std::fs::read_to_string(dir.join("config/config.toml")).unwrap();
    assert!(saved.contains("time = 60"), "{saved}");
    settle(&mut a, 0.0);
    assert_eq!(
        a.session().spec().time_limit,
        Some(60),
        "nouveau test de 60 s"
    );
}

#[test]
fn hovering_a_theme_previews_it_and_escape_reverts() {
    let mut a = app("cl-theme", "");
    let original = *a.palette();
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "theme", 0.0);
    a.handle(press(Key::Enter, 0.0));
    // le curseur part du thème en cours ; on descend sur le suivant
    a.handle(press(Key::Down, 0.0));
    let hovered = a.command_line().unwrap().hovered().unwrap().display.clone();
    assert_ne!(*a.palette(), original, "aperçu de {hovered}");
    a.handle(press(Key::Esc, 0.0));
    a.handle(press(Key::Esc, 0.0));
    assert!(a.command_line().is_none());
    assert_eq!(*a.palette(), original, "retour au thème en cours");
    assert_eq!(a.store.config.str("theme"), "serika_dark");
}

#[test]
fn choosing_a_theme_applies_it() {
    let mut a = app("cl-theme-set", "");
    let original = *a.palette();
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "theme", 0.0);
    a.handle(press(Key::Enter, 0.0));
    type_query(&mut a, "dracula", 0.0);
    a.handle(press(Key::Enter, 0.0));
    assert_eq!(a.store.config.str("theme"), "dracula");
    assert_ne!(*a.palette(), original);
}

#[test]
fn repeat_test_replays_the_same_words() {
    let mut a = app("cl-repeat", "mode = \"words\"\nwords = 10\n");
    let words = a.session().words().to_vec();
    let end = type_whole_test(&mut a, 0.0, 20.0);
    let t = settle(&mut a, end);
    a.handle(press(Key::Esc, t));
    assert_eq!(
        a.command_line().unwrap().hovered().unwrap().display,
        "Next test"
    );
    a.handle(press(Key::Down, t));
    a.handle(press(Key::Enter, t));
    settle(&mut a, t);
    assert!(matches!(a.screen(), Screen::Test));
    assert_eq!(a.session().words(), words.as_slice());
    assert!(a.session().is_repeated());
}

#[test]
fn bail_out_ends_a_zen_test() {
    let mut a = app("cl-bail", "mode = \"zen\"\n");
    type_text(&mut a, "abc ", 0.0, 50.0);
    a.handle(press(Key::Esc, 300.0));
    type_query(&mut a, "bail", 300.0);
    a.handle(press(Key::Enter, 300.0));
    a.handle(press(Key::Down, 300.0));
    a.handle(press(Key::Enter, 300.0));
    match a.screen() {
        Screen::Result(info) => assert!(info.result.bailed_out),
        _ => panic!("écran de résultat attendu"),
    }
}

#[test]
fn clear_all_notifications() {
    let mut a = app("cl-clear", "language = \"klingon_9000k\"\n");
    assert!(!a.notifications.items().is_empty());
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "dismiss", 0.0);
    a.handle(press(Key::Enter, 0.0));
    assert!(a.notifications.items().is_empty());
}

#[test]
fn quit_command() {
    let mut a = app("cl-quit", "");
    a.handle(press(Key::Esc, 0.0));
    type_query(&mut a, "quit", 0.0);
    a.handle(press(Key::Enter, 0.0));
    assert!(a.quit);
}

#[test]
fn pasted_text_goes_to_the_search() {
    let mut a = app("cl-paste", "");
    a.handle(Input::Paste("smooth caret".into()));
    assert!(
        a.command_line().is_none(),
        "palette fermée : collage ignoré"
    );
    a.handle(press(Key::Esc, 0.0));
    a.handle(Input::Paste("smooth\ncaret".into()));
    assert_eq!(a.command_line().unwrap().query(), "smooth caret");
}

#[test]
fn command_line_is_drawn_over_the_test() {
    let mut a = app("cl-draw", "");
    a.handle(press(Key::Esc, 0.0));
    let (buf, cursor) = render(&mut a, 100, 30);
    let text = screen_text(&buf);
    assert!(text.contains("Search..."), "{text}");
    assert!(text.contains("Punctuation..."));
    let (x, y) = cursor.expect("curseur dans la saisie");
    assert!(row(&buf, y).contains("Search..."));
    assert!(x > 0);
    // la ligne active est inversée
    let line = (0..30u16)
        .find(|&yy| row(&buf, yy).contains("Punctuation..."))
        .unwrap();
    let col = row(&buf, line).find("Punctuation").unwrap() as u16;
    assert_eq!(buf[(col, line)].bg, a.palette().text);
    // le fond est assombri
    assert_ne!(buf[(0, 0)].bg, a.palette().bg);
}

#[test]
fn key_tips_follow_quick_restart() {
    let mut a = app("cl-tips", "");
    let (buf, _) = render(&mut a, 100, 30);
    assert!(screen_text(&buf).contains("tab + enter - restart   esc - command line"));
    let mut a = app("cl-tips-esc", "quick_restart = \"esc\"\n");
    let (buf, _) = render(&mut a, 100, 30);
    assert!(screen_text(&buf).contains("esc - restart   tab - command line"));
    let mut a = app("cl-tips-off", "show_key_tips = false\n");
    let (buf, _) = render(&mut a, 100, 30);
    assert!(!screen_text(&buf).contains("restart"));
}

#[test]
fn scaled_words_make_room_for_the_command_line() {
    let mut a = app("cl-scaled", "");
    a.set_text_sizing(true);
    render(&mut a, 120, 30);
    a.handle(press(Key::Esc, 0.0));
    render(&mut a, 120, 30);
    let t = a.scaled_text().expect("mots agrandis").clone();
    let hole = t.hole.expect("la palette recouvre une partie de la zone");
    assert!(hole.intersects(t.region));
    assert!(a.caret_frame().is_none(), "caret caché pendant la palette");
}

#[test]
fn command_line_fits_any_terminal_size() {
    let mut a = app("cl-small", "");
    a.handle(press(Key::Esc, 0.0));
    for (w, h) in [(40, 10), (41, 11), (60, 12), (200, 60), (0, 0), (10, 3)] {
        render(&mut a, w, h);
    }
    // une saisie libre avec une erreur, au plus petit
    type_query(&mut a, "font", 0.0);
    a.handle(press(Key::Enter, 0.0));
    a.handle(press(Key::Char('x'), 0.0));
    a.handle(press(Key::Enter, 0.0));
    for (w, h) in [(40, 10), (100, 30)] {
        let (buf, _) = render(&mut a, w, h);
        if w == 100 {
            assert!(screen_text(&buf).contains("Must be a number"));
        }
    }
}

#[test]
fn a_test_that_ends_behind_the_command_line_keeps_it_usable() {
    let mut a = app("cl-ends", "time = 15\n");
    type_text(&mut a, "x", 0.0, 50.0);
    a.handle(press(Key::Esc, 100.0));
    a.tick(15_000.0);
    assert!(
        matches!(a.screen(), Screen::Result(_)),
        "le test continue derrière"
    );
    assert!(a.command_line().is_some());
    render(&mut a, 100, 30);
    a.handle(press(Key::Esc, 15_100.0));
    assert!(a.command_line().is_none());
}
```

`crates/fasttype-tui/tests/sized.rs` (version complète : `hole: None` dans les `ScaledText`) :
```rust
use fasttype_tui::sized::{ScaledCell, ScaledText, probe_answer, scale_for};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

#[test]
fn text_sizing_probe_answers() {
    assert_eq!(probe_answer(b""), None);
    assert_eq!(
        probe_answer(b"\x1b[1;1R"),
        None,
        "attend la seconde position"
    );
    assert_eq!(probe_answer(b"\x1b[1;1R\x1b[1;3R"), Some(true));
    assert_eq!(
        probe_answer(b"\x1b[1;1R\x1b[1;2R"),
        Some(false),
        "un espace normal"
    );
    assert_eq!(probe_answer(b"\x1b[1;1R\x1b[1;1R"), Some(false), "ignoré");
}

#[test]
fn font_size_to_scale() {
    assert_eq!(scale_for(2.0), 2, "défaut du site : mots à 2rem");
    assert_eq!(scale_for(1.0), 1);
    assert_eq!(scale_for(1.5), 2);
    assert_eq!(scale_for(1.25), 1);
    assert_eq!(scale_for(10.0), 4);
    assert_eq!(scale_for(f64::NAN), 1);
}

#[test]
fn scaled_text_clears_the_region_then_writes_each_letter() {
    let t = ScaledText {
        scale: 2,
        region: Rect::new(4, 10, 3, 2),
        bg: Color::Rgb(1, 2, 3),
        cells: vec![ScaledCell {
            x: 4,
            y: 10,
            ch: 'a',
            style: Style::default()
                .fg(Color::Rgb(9, 9, 9))
                .bg(Color::Rgb(1, 2, 3)),
        }],
        hole: None,
    };
    let mut out = Vec::new();
    t.write(&mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    // le curseur est sauvé puis rendu (DECSC/DECRC) : le caret du terminal reste en place
    assert_eq!(
        s,
        "\x1b7\x1b[0;48;2;1;2;3m\x1b[11;5H   \x1b[12;5H   \x1b[11;5H\x1b[0;38;2;9;9;9;48;2;1;2;3m\x1b]66;s=2;a\x07\x1b[0m\x1b8"
    );
}

fn cell(x: u16, ch: char, fg: u8) -> ScaledCell {
    ScaledCell {
        x,
        y: 10,
        ch,
        style: Style::default().fg(Color::Rgb(fg, fg, fg)),
    }
}

fn text(cells: Vec<ScaledCell>) -> ScaledText {
    ScaledText {
        scale: 2,
        region: Rect::new(0, 10, 20, 2),
        bg: Color::Rgb(1, 2, 3),
        cells,
        hole: None,
    }
}

#[test]
fn only_changed_letters_are_rewritten() {
    let before = text(vec![cell(0, 'a', 9), cell(2, 'b', 9), cell(4, 'c', 9)]);
    // une lettre change de couleur, une disparaît
    let after = text(vec![cell(0, 'a', 9), cell(2, 'b', 7)]);
    let mut out = Vec::new();
    after.write_changes(Some(&before), &mut out).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert!(s.starts_with("\x1b7") && s.ends_with("\x1b8"));
    assert!(!s.contains("s=2;a"), "lettre inchangée : rien à écrire");
    assert!(s.contains("\x1b[11;3H\x1b[0;38;2;7;7;7m\x1b]66;s=2;b\x07"));
    assert!(
        s.contains("\x1b[11;5H  \x1b[12;5H  "),
        "le bloc de « c » est effacé"
    );
    assert!(s.len() < 120, "pas toute la zone : {}", s.len());
    // sans image précédente, ou si la zone a changé : tout est réécrit
    let mut full = Vec::new();
    after.write_changes(None, &mut full).unwrap();
    let mut direct = Vec::new();
    after.write(&mut direct).unwrap();
    assert_eq!(full, direct);
    let moved = ScaledText {
        region: Rect::new(0, 12, 20, 2),
        ..after.clone()
    };
    let mut out = Vec::new();
    moved.write_changes(Some(&after), &mut out).unwrap();
    assert!(
        String::from_utf8(out).unwrap().contains("\x1b[13;1H"),
        "zone effacée"
    );
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test command_line --test sized`
Expected: échec de compilation (`no method named command_line`, `struct ScaledText has no field named hole`).

- [ ] **Step 3 : implémenter les vues**

Ajouter `pub mod palette;` après `pub mod notify;` dans `crates/fasttype-tui/src/view/mod.rs`, et la méthode `clear` dans `crates/fasttype-tui/src/view/notify.rs` (voir Interfaces).

`crates/fasttype-tui/src/view/palette.rs` :
```rust
//! Palette de commandes à l'écran (`CommandlineModal.tsx`) : fond assombri,
//! boîte en haut au centre, saisie puis liste ; la ligne active est inversée
//! (`bg-text text-bg`), les autres en `sub`, une coche devant la valeur en cours.

use crate::palette::state::PaletteState;
use crate::theme::Palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Lignes de liste visibles au plus.
pub const MAX_ROWS: u16 = 12;

/// Couleur vue derrière le voile noir à 50 % de la palette (truecolor).
pub fn dim_color(c: Color) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb(r / 2, g / 2, b / 2),
        other => other,
    }
}

pub fn dim_style(s: Style) -> Style {
    Style {
        fg: s.fg.map(dim_color),
        bg: s.bg.map(dim_color),
        underline_color: s.underline_color.map(dim_color),
        ..s
    }
}

/// Assombrit l'écran derrière la palette.
pub fn dim(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            let (fg, bg) = (dim_color(cell.fg), dim_color(cell.bg));
            cell.set_fg(fg).set_bg(bg);
        }
    }
}

/// Place de la boîte : en haut au centre, à la hauteur de son contenu.
pub fn palette_rect(area: Rect, state: &PaletteState) -> Rect {
    let width = area.width.saturating_sub(4).min(72);
    let error = state.input().is_some_and(|m| m.error.is_some());
    let rows = if state.input().is_some() {
        u16::from(error)
    } else {
        (state.shown().0.len() as u16).min(MAX_ROWS)
    };
    let top = area.y + (area.height / 8).max(1);
    let height = (2 + rows + 1).min(area.bottom().saturating_sub(top));
    Rect {
        x: area.x + (area.width - width) / 2,
        y: top,
        width,
        height,
    }
}

/// Coupe un texte à `width` cases.
fn fit(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width {
            break;
        }
        used += w;
        out.push(c);
    }
    out
}

/// Dessine la palette ; renvoie la position du curseur de saisie.
pub fn render(buf: &mut Buffer, area: Rect, state: &PaletteState, p: &Palette) -> (u16, u16) {
    dim(buf, area);
    let r = palette_rect(area, state);
    let back = Style::default().bg(p.bg);
    for y in r.top()..r.bottom() {
        buf.set_string(r.x, y, " ".repeat(usize::from(r.width)), back);
    }
    let inner = usize::from(r.width.saturating_sub(4));
    // saisie : la recherche, ou la valeur libre en cours
    let (placeholder, text) = match state.input() {
        Some(m) => (m.title.as_str(), m.text.as_str()),
        None => (state.title(), state.query()),
    };
    let y = r.y + 1;
    buf.set_string(r.x + 2, y, "›", back.fg(p.sub));
    let shown = if text.is_empty() {
        buf.set_string(
            r.x + 4,
            y,
            fit(placeholder, inner.saturating_sub(2)),
            back.fg(p.sub),
        );
        String::new()
    } else {
        // la fin de la saisie reste visible
        let room = inner.saturating_sub(3);
        let skip = text.width().saturating_sub(room);
        let visible: String = text.chars().skip(skip).collect();
        buf.set_string(r.x + 4, y, &visible, back.fg(p.text));
        visible
    };
    let cursor = (r.x + 4 + shown.width() as u16, y);
    if let Some(m) = state.input() {
        if let Some(e) = &m.error {
            buf.set_string(
                r.x + 2,
                y + 1,
                fit(&format!("⚠ {e}"), inner),
                back.fg(p.error),
            );
        }
        return cursor;
    }
    let (list, active) = state.shown();
    let rows = usize::from(r.height.saturating_sub(3));
    let start = active
        .saturating_sub(rows / 2)
        .min(list.len().saturating_sub(rows));
    for (k, cmd) in list.iter().enumerate().skip(start).take(rows) {
        let y = r.y + 2 + (k - start) as u16;
        let style = if k == active {
            Style::default().bg(p.text).fg(p.bg)
        } else {
            back.fg(p.sub)
        };
        let check = if cmd.checkable && cmd.active {
            "✓"
        } else {
            " "
        };
        let line = fit(&format!(" {check} {}", cmd.display), inner + 2);
        let pad = (inner + 2).saturating_sub(line.width());
        buf.set_string(r.x + 1, y, format!("{line}{}", " ".repeat(pad)), style);
    }
    cursor
}
```

`crates/fasttype-tui/src/view/test.rs` (version complète) :
```rust
//! Écran de test : lignes de mots visibles, en-tête et raccourcis.

use crate::layout::{Layout, char_width, extras, letters};
use crate::sized::{ScaledCell, ScaledText};
use crate::theme::Palette;
use crate::view::centered_segments;
use crate::view::config_bar::{bar_layout, render_bar};
use crate::view::live::WordsBox;
use fasttype_core::session::TestSession;
use fasttype_store::Config;
use ratatui::buffer::{Buffer, CellDiffOption};
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

/// Place de la zone de mots : centrée, 3 lignes (2 en zen) de lettres à
/// l'échelle `scale`. Largeur : `maxLineWidth` lettres (0 = automatique :
/// 100 cases de l'écran au plus). L'échelle baisse si la zone ne tient pas.
pub fn words_box(area: Rect, max_line_width: u16, zen: bool, scale: u16) -> WordsBox {
    let lines = if zen { 2 } else { 3 };
    let mut scale = scale.max(1);
    // la place des mots, plus l'en-tête, les stats et les raccourcis
    while scale > 1
        && (area.width.saturating_sub(8) / scale < 20 || lines * scale + 8 > area.height)
    {
        scale -= 1;
    }
    let screen = if max_line_width >= 20 {
        max_line_width
            .saturating_mul(scale)
            .min(area.width.saturating_sub(2))
    } else {
        area.width.saturating_sub(8).min(100)
    };
    let width = screen / scale;
    WordsBox {
        left: area.x + area.width.saturating_sub(width * scale) / 2,
        top: area.y + area.height.saturating_sub(lines * scale) / 2,
        width,
        lines,
        scale,
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
    /// Dessine les mots. À l'échelle 1, dans le tampon ; au-delà, la zone est
    /// réservée (`skip`) et les lettres sont renvoyées pour l'écriture OSC 66.
    pub fn render(&self, buf: &mut Buffer) -> Option<ScaledText> {
        let w = self.words;
        let shown = usize::from(w.lines);
        let mut letters = Vec::new();
        for (row, line) in self.layout.lines.iter().take(shown).enumerate() {
            let palette = match self.last_line {
                Some(ref p) if row + 1 == shown => p,
                _ => self.palette,
            };
            for b in line {
                self.word_letters(palette, b.x, row as u16, b.index, &mut letters);
            }
        }
        let area = buf.area;
        if w.scale <= 1 {
            for (col, row, ch, style) in letters {
                let (x, y) = (w.left + col, w.top + row);
                if x < area.right() && y < area.bottom() {
                    buf[(x, y)].set_char(ch).set_style(style);
                }
            }
            return None;
        }
        let region = w.rect().intersection(area);
        for y in region.top()..region.bottom() {
            for x in region.left()..region.right() {
                buf[(x, y)].set_diff_option(CellDiffOption::Skip);
            }
        }
        let cells = letters
            .into_iter()
            .map(|(col, row, ch, style)| ScaledCell {
                x: w.left + col * w.scale,
                y: w.top + row * w.scale,
                ch,
                style,
            })
            .filter(|c| c.x + w.scale <= region.right() && c.y + w.scale <= region.bottom())
            .collect();
        Some(ScaledText {
            scale: w.scale,
            region,
            bg: self.palette.bg,
            cells,
            hole: None,
        })
    }

    /// Lettres d'un mot : (colonne, ligne, caractère, style), en lettres.
    fn word_letters(
        &self,
        p: &Palette,
        x0: u16,
        row: u16,
        index: usize,
        out: &mut Vec<(u16, u16, char, Style)>,
    ) {
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
        let mut x = x0;
        let mut put = |c: char, style: Style| {
            out.push((x, row, c, style));
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

/// En-tête (logo et barre de config) et raccourcis du bas, avec leurs
/// opacités du focus mode.
pub struct Chrome<'a> {
    pub palette: &'a Palette,
    /// Couleur du logo : `main`, qui passe à `sub` en focus mode.
    pub logo: Color,
    pub config: &'a Config,
    /// Opacité de la barre de config et des raccourcis.
    pub opacity: f64,
    /// Haut de la zone de mots : la barre n'y descend jamais (une ligne libre au-dessus).
    pub words_top: Option<u16>,
}

/// Raccourcis du pied de page selon `quickRestart` : touches de restart, puis
/// touche de la palette (Tab quand Échap relance). Chaque segment : (texte, touche ?).
pub fn key_tips(config: &Config) -> Vec<(String, bool)> {
    let quick = config.str("quickRestart");
    let restart: &[&str] = match quick {
        "tab" => &["tab"],
        "esc" => &["esc"],
        "enter" => &["enter"],
        _ => &["tab", "enter"],
    };
    let mut tips = Vec::new();
    for (i, k) in restart.iter().enumerate() {
        if i > 0 {
            tips.push((" + ".to_string(), false));
        }
        tips.push((k.to_string(), true));
    }
    tips.push((" - restart   ".to_string(), false));
    tips.push((
        (if quick == "esc" { "tab" } else { "esc" }).to_string(),
        true,
    ));
    tips.push((" - command line".to_string(), false));
    tips
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
        if let Some((y, bar)) = bar_layout(self.config, area)
            && self.words_top.is_none_or(|top| y + 1 < top)
        {
            render_bar(buf, area, y, &bar, &p);
        }
        let key = Style::default().fg(p.sub_alt).bg(p.sub);
        let text = Style::default().fg(p.sub);
        if self.config.bool("showKeyTips") {
            let tips: Vec<(String, Style)> = key_tips(self.config)
                .into_iter()
                .map(|(t, is_key)| (t, if is_key { key } else { text }))
                .collect();
            centered_segments(buf, area, area.bottom().saturating_sub(2), &tips);
        }
    }
}
```

`crates/fasttype-tui/src/sized.rs` (version complète) :
```rust
//! Texte agrandi avec le protocole de taille du texte de Kitty (OSC 66,
//! Kitty ≥ 0.40) : les mots du test s'affichent à `fontSize` fois la taille
//! du reste de l'interface, comme sur le site (mots à 2rem, interface à 1rem).
//!
//! Chaque lettre agrandie occupe un bloc de `scale × scale` cases. La zone des
//! mots est marquée `skip` dans le tampon ratatui, qui n'y écrit donc jamais :
//! elle est effacée puis réécrite ici, d'un bloc, à chaque changement.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::io::{self, Write};

/// Sonde : position du curseur, un espace en double taille, position du
/// curseur. Un terminal qui gère OSC 66 avance de 2 cases au lieu d'une.
pub const PROBE: &[u8] = b"\r\x1b[6n\x1b]66;s=2; \x07\x1b[6n";

/// Lit la réponse à `PROBE` : `None` tant que les deux positions ne sont pas
/// arrivées, puis `Some(true)` si le curseur a avancé de 2 cases.
pub fn probe_answer(bytes: &[u8]) -> Option<bool> {
    let text = String::from_utf8_lossy(bytes);
    let mut cols = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find("\x1b[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('R') else {
            break;
        };
        if let Some((_, col)) = after[..end].split_once(';')
            && let Ok(c) = col.parse::<u32>()
        {
            cols.push(c);
        }
        rest = &after[end + 1..];
    }
    match cols[..] {
        [a, b, ..] => Some(b == a + 2),
        _ => None,
    }
}

/// Échelle du texte des mots : `fontSize` arrondi, de 1 à 4.
pub fn scale_for(font_size: f64) -> u16 {
    if font_size.is_finite() {
        font_size.round().clamp(1.0, 4.0) as u16
    } else {
        1
    }
}

/// Une lettre agrandie, coin haut-gauche (`x`, `y`) en cases de l'écran.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledCell {
    pub x: u16,
    pub y: u16,
    pub ch: char,
    pub style: Style,
}

/// La zone des mots agrandis : à effacer puis réécrire d'un bloc.
#[derive(Debug, Clone, PartialEq)]
pub struct ScaledText {
    pub scale: u16,
    pub region: Rect,
    /// Fond de la zone (couleur `bg` du thème).
    pub bg: Color,
    pub cells: Vec<ScaledCell>,
    /// Partie de la zone recouverte par la palette : ni effacée ni écrite.
    pub hole: Option<Rect>,
}

fn color(out: &mut Vec<u8>, base: u8, c: Color) {
    let _ = match c {
        Color::Rgb(r, g, b) => write!(out, ";{base};2;{r};{g};{b}"),
        Color::Indexed(i) => write!(out, ";{base};5;{i}"),
        _ => Ok(()),
    };
}

/// SGR complet d'un style (repart de zéro : `0`).
fn sgr(out: &mut Vec<u8>, style: &Style) {
    out.extend_from_slice(b"\x1b[0");
    if let Some(fg) = style.fg {
        color(out, 38, fg);
    }
    if let Some(bg) = style.bg {
        color(out, 48, bg);
    }
    if style.add_modifier.contains(Modifier::BOLD) {
        out.extend_from_slice(b";1");
    }
    if style.add_modifier.contains(Modifier::UNDERLINED) {
        out.extend_from_slice(b";4");
        if let Some(u) = style.underline_color {
            color(out, 58, u);
        }
    }
    out.push(b'm');
}

impl ScaledText {
    /// Efface la zone (écraser la case haut-gauche d'une lettre agrandie
    /// l'efface en entier) puis écrit chaque lettre. Le curseur est sauvé puis
    /// rendu (DECSC/DECRC) : le caret du terminal reste où ratatui l'a mis.
    pub fn write(&self, out: &mut impl Write) -> io::Result<()> {
        let mut buf = Vec::with_capacity(64 * self.cells.len() + 8 * self.region.area() as usize);
        buf.extend_from_slice(b"\x1b7");
        sgr(&mut buf, &Style::default().bg(self.bg));
        let r = self.region;
        let hole = self
            .hole
            .map(|h| h.intersection(r))
            .filter(|h| !h.is_empty());
        for y in r.top()..r.bottom() {
            // segments de la ligne hors de la palette
            let spans = match hole {
                Some(h) if y >= h.top() && y < h.bottom() => {
                    vec![(r.left(), h.left()), (h.right(), r.right())]
                }
                _ => vec![(r.left(), r.right())],
            };
            for (a, b) in spans.into_iter().filter(|(a, b)| a < b) {
                let blank = " ".repeat(usize::from(b - a));
                let _ = write!(buf, "\x1b[{};{}H{blank}", y + 1, a + 1);
            }
        }
        for c in &self.cells {
            let block = Rect::new(c.x, c.y, self.scale, self.scale);
            if hole.is_some_and(|h| h.intersects(block)) {
                continue;
            }
            put(&mut buf, self.scale, c);
        }
        buf.extend_from_slice(b"\x1b[0m\x1b8");
        out.write_all(&buf)
    }

    /// N'écrit que ce qui a changé depuis `prev` : une lettre nouvelle ou
    /// modifiée remplace l'ancienne (même case haut-gauche), une lettre partie
    /// est effacée. Tout est réécrit si la zone, l'échelle, le fond ou la
    /// partie recouverte par la palette changent.
    pub fn write_changes(&self, prev: Option<&ScaledText>, out: &mut impl Write) -> io::Result<()> {
        let Some(prev) = prev.filter(|p| {
            p.region == self.region
                && p.scale == self.scale
                && p.bg == self.bg
                && p.hole == self.hole
        }) else {
            return self.write(out);
        };
        let old: std::collections::HashMap<(u16, u16), &ScaledCell> =
            prev.cells.iter().map(|c| ((c.x, c.y), c)).collect();
        let new: std::collections::HashSet<(u16, u16)> =
            self.cells.iter().map(|c| (c.x, c.y)).collect();
        let hole = self.hole;
        let covered = |c: &ScaledCell| {
            hole.is_some_and(|h| h.intersects(Rect::new(c.x, c.y, self.scale, self.scale)))
        };
        let mut buf = Vec::new();
        buf.extend_from_slice(b"\x1b7");
        // lettres parties : effacer leur bloc
        let blank = " ".repeat(usize::from(self.scale));
        let mut cleared = false;
        for c in prev
            .cells
            .iter()
            .filter(|c| !new.contains(&(c.x, c.y)) && !covered(c))
        {
            if !cleared {
                sgr(&mut buf, &Style::default().bg(self.bg));
                cleared = true;
            }
            for dy in 0..self.scale {
                let _ = write!(buf, "\x1b[{};{}H{blank}", c.y + dy + 1, c.x + 1);
            }
        }
        for c in &self.cells {
            if !covered(c)
                && old
                    .get(&(c.x, c.y))
                    .is_none_or(|o| o.ch != c.ch || o.style != c.style)
            {
                put(&mut buf, self.scale, c);
            }
        }
        buf.extend_from_slice(b"\x1b[0m\x1b8");
        out.write_all(&buf)
    }
}

/// Une lettre agrandie : position, couleurs, séquence OSC 66.
fn put(buf: &mut Vec<u8>, scale: u16, c: &ScaledCell) {
    let _ = write!(buf, "\x1b[{};{}H", c.y + 1, c.x + 1);
    sgr(buf, &c.style);
    let _ = write!(buf, "\x1b]66;s={scale};{}\x07", c.ch);
}
```

- [ ] **Step 4 : implémenter l'application et le terminal**

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
use crate::caret::{
    Caret, CaretFrame, CaretStyle, CaretTarget, coverage, coverage_box, smooth_caret_ms,
};
use crate::input::{Input, Key, Phase};
use crate::kitty::CaretRenderer;
use crate::layout::{Layout, char_width, layout_window, letters};
use crate::palette::state::{Outcome, PaletteState};
use crate::palette::{Action, AppAction, lists, lists::Context};
use crate::perf::Perf;
use crate::session_factory::SessionFactory;
use crate::sized::{ScaledCell, ScaledText, scale_for};
use crate::theme::{ColorMode, Palette, Rgb, mix};
use crate::view::config_bar::bar_layout;
use crate::view::live::{LiveItem, LiveStats, Style3, WordsBox, render_bar, seconds_to_string};
use crate::view::notify::{Level, Notifications};
use crate::view::palette as palette_view;
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
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;
use ratatui::style::Style;
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

/// Réglages qui changent le test : le changer relance un test (`afterExec: restart`).
const RESTART_KEYS: &[&str] = &[
    "mode",
    "time",
    "words",
    "quoteLength",
    "language",
    "punctuation",
    "numbers",
];

/// `canBailOut` (lists/bail-out.ts) : tests longs, infinis ou zen.
fn can_bail_out(spec: &fasttype_core::spec::TestSpec) -> bool {
    use fasttype_core::spec::CustomLimit;
    let big = |n: u32, threshold: u32| n == 0 || n >= threshold;
    match (spec.mode, spec.custom_limit) {
        (Mode::Zen, _) => true,
        (Mode::Time, _) => big(spec.time_limit.unwrap_or(0), 3600),
        (Mode::Words, _) => big(spec.mode2.parse().unwrap_or(0), 5000),
        (Mode::Custom, Some(CustomLimit::Time(s))) => big(s, 3600),
        (Mode::Custom, Some(CustomLimit::Word(n) | CustomLimit::Section(n))) => big(n, 5000),
        _ => false,
    }
}

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
    /// Instant de la dernière touche prise en compte : le caret glisse depuis là.
    input_at: f64,
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
    /// Le terminal gère OSC 66 : les mots s'affichent à `fontSize` fois la taille.
    text_sizing: bool,
    /// Mots agrandis du dernier dessin, à écrire par la boucle.
    scaled: Option<ScaledText>,
    /// Zone des mots agrandis de l'image précédente : à redessiner en entier
    /// quand elle disparaît (ratatui n'y avait rien écrit).
    last_scaled_region: Option<Rect>,
    /// Haut de la zone de mots du dernier dessin (la barre de config reste au-dessus).
    words_top: Option<u16>,
    /// Palette de commandes ouverte.
    command_line: Option<PaletteState>,
    /// Palette du thème en cours, gardée pendant l'aperçu d'un autre thème.
    saved_palette: Option<Palette>,
    color_mode: ColorMode,
    /// Le prochain restart rejoue le même texte (« Repeat test »).
    repeat_next: bool,
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
            input_at: now,
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
            text_sizing: false,
            scaled: None,
            last_scaled_region: None,
            words_top: None,
            command_line: None,
            saved_palette: None,
            color_mode,
            repeat_next: false,
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

    /// Le terminal sait agrandir le texte (OSC 66, détecté au démarrage).
    pub fn set_text_sizing(&mut self, supported: bool) {
        self.text_sizing = supported;
    }

    /// Mots agrandis du dernier `draw`, à écrire après le dessin ratatui.
    pub fn scaled_text(&self) -> Option<&ScaledText> {
        self.scaled.as_ref()
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

    /// La palette est ouverte.
    pub fn command_line(&self) -> Option<&PaletteState> {
        self.command_line.as_ref()
    }

    /// Ouvre la palette sur la liste racine.
    fn open_palette(&mut self) {
        let languages = fasttype_data::language_names().unwrap_or_default();
        let themes: Vec<&str> = fasttype_data::themes()
            .map(|t| t.iter().map(|t| t.name.as_str()).collect())
            .unwrap_or_default();
        let spec = self.session.spec();
        let running =
            matches!(self.screen, Screen::Test) && self.session.state() == SessionState::Running;
        let ctx = Context {
            config: &self.store.config,
            on_result: matches!(self.screen, Screen::Result(_)),
            can_bail_out: running && can_bail_out(spec),
            languages: &languages,
            themes: &themes,
        };
        self.command_line = Some(PaletteState::open(lists::root(&ctx)));
    }

    fn close_palette(&mut self) {
        self.command_line = None;
        self.preview_theme(None);
    }

    fn palette_key(&mut self, key: Key, now: f64) {
        let Some(p) = &mut self.command_line else {
            return;
        };
        match p.key(key, &self.store.config) {
            Outcome::Stay => {
                let hovered = p.hovered().and_then(|c| c.preview.clone());
                self.preview_theme(hovered.as_deref());
            }
            Outcome::Close => self.close_palette(),
            Outcome::Run(action) => {
                self.command_line = None;
                self.saved_palette = None;
                self.run(action, now);
                // un thème survolé mais pas choisi : retour au thème de la config
                self.palette =
                    Palette::from_theme(&resolve_theme(&self.store.config).0, self.color_mode);
            }
        }
    }

    /// Aperçu d'un thème au survol ; `None` remet le thème en cours.
    fn preview_theme(&mut self, name: Option<&str>) {
        match name.and_then(theme) {
            Some(t) => {
                if self.saved_palette.is_none() {
                    self.saved_palette = Some(self.palette);
                }
                self.palette = Palette::from_theme(t, self.color_mode);
            }
            None => {
                if let Some(p) = self.saved_palette.take() {
                    self.palette = p;
                }
            }
        }
    }

    /// Exécute une commande de la palette.
    fn run(&mut self, action: Action, now: f64) {
        match action {
            Action::Set { key, value } => match self.store.config.set(key, value) {
                Ok(changed) => {
                    if let Err(e) = self.store.save_config() {
                        self.notifications.push(
                            format!("could not save the settings: {e}"),
                            Level::Error,
                            now,
                        );
                    }
                    if changed.iter().any(|k| RESTART_KEYS.contains(k)) {
                        self.try_restart(now, true);
                    }
                }
                Err(_) => {
                    self.notifications
                        .push(format!("invalid value for {key}"), Level::Error, now)
                }
            },
            Action::App(AppAction::NextTest) => self.try_restart(now, true),
            Action::App(AppAction::RepeatTest) => {
                self.repeat_next = true;
                self.try_restart(now, true);
            }
            Action::App(AppAction::BailOut) => {
                self.session.bail_out(now);
                self.check_finished(now);
            }
            Action::App(AppAction::ClearNotifications) => self.notifications.clear(),
            Action::App(AppAction::Quit) => self.quit = true,
            Action::Open(_) | Action::Input { .. } | Action::Close => {}
        }
        self.update_motion(now);
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
        let previous = std::mem::replace(&mut self.session, built.session);
        if std::mem::take(&mut self.repeat_next)
            && let Some(again) = previous.into_repeat()
        {
            self.session = again;
        }
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
        if let Input::Paste(text) = &input {
            if let Some(p) = &mut self.command_line {
                p.paste(text);
            }
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
        // le restart dure ses deux fondus (`isTestRestarting`) : les touches sont ignorées
        self.advance_transitions(at);
        if matches!(
            self.transition,
            Some(Transition::Restart { .. } | Transition::FadeIn { .. })
        ) {
            return;
        }
        if self.command_line.is_some() {
            self.palette_key(key, at);
            return;
        }
        let quick = self.store.config.str("quickRestart");
        // la palette s'ouvre avec Échap, ou Tab quand Échap relance (`hotkeys.ts`)
        let opens = match key {
            Key::Palette => true,
            Key::Esc => quick != "esc",
            Key::Tab => quick == "esc",
            _ => false,
        };
        if opens {
            self.open_palette();
            return;
        }
        self.input_at = at;
        // En zen, Entrée insère un saut de ligne et Shift+Entrée termine le test :
        // ni l'une ni l'autre ne relance.
        let zen = self.session.spec().mode == Mode::Zen && matches!(self.screen, Screen::Test);
        let has_newlines = zen || self.session.has_newlines();
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

    /// Fait avancer les fondus jusqu'à `now` : le restart a lieu à la fin du
    /// fondu de sortie, même si aucun tick n'est tombé pile à ce moment.
    fn advance_transitions(&mut self, now: f64) {
        if let Some(Transition::Restart { start }) = self.transition
            && now >= start + FADE_MS
        {
            self.restart(start + FADE_MS);
        }
        match self.transition {
            Some(Transition::ToResult { start }) if now >= start + 2.0 * FADE_MS => {
                self.transition = None;
            }
            Some(Transition::FadeIn { start }) if now >= start + FADE_MS => {
                self.transition = None;
            }
            _ => {}
        }
    }

    pub fn tick(&mut self, now: f64) {
        self.now = self.now.max(now);
        let now = self.now;
        self.advance_transitions(now);
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

    /// Prochain palier d'opacité du clignotement, s'il se dessine dans
    /// l'image (caret Kitty ou bloc) : 16 paliers par demi-période, soit 32
    /// réveils par seconde au plus au lieu d'une image à chaque 1/60 s.
    fn next_blink_step(&self, now: f64) -> Option<f64> {
        let drawn = matches!(self.screen, Screen::Test)
            && self.transition.is_none()
            && self.command_line.is_none()
            && (matches!(self.renderer, CaretRenderer::Kitty(_))
                || self.caret_style() == CaretStyle::Block);
        let since = self.caret.blink_since().filter(|_| drawn)?;
        let step = 1000.0 / f64::from(2 * crate::kitty::LEVELS);
        Some(since + ((now - since) / step).floor() * step + step)
    }

    /// Une image change-t-elle sans frappe (hors clignotement) ?
    fn animating(&self, now: f64) -> bool {
        self.transition.is_some()
            || self.caret.is_moving(now)
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
        let blink = self.next_blink_step(self.now);
        [tick, self.notifications.next_expiry(), frame, blink]
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
        self.scaled = None;
        self.words_top = None;
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
                    quote_source: self
                        .session
                        .spec()
                        .quote
                        .as_ref()
                        .map(|q| q.source.as_str()),
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
            config: &self.store.config,
            opacity: chrome_opacity,
            words_top: self.words_top,
        }
        .render(buf, area);
        self.notifications.render(buf, area, &self.palette);
        if let Some(p) = &self.command_line {
            cursor = Some(palette_view::render(buf, area, p, &self.palette));
            // la boîte recouvre les mots agrandis : ratatui la redessine toujours,
            // et la zone agrandie la contourne
            let boxed = palette_view::palette_rect(area, p);
            for y in boxed.top()..boxed.bottom() {
                for x in boxed.left()..boxed.right() {
                    buf[(x, y)].set_diff_option(CellDiffOption::AlwaysUpdate);
                }
            }
            if let Some(st) = &mut self.scaled {
                st.hole = Some(boxed);
                st.bg = palette_view::dim_color(st.bg);
                for c in &mut st.cells {
                    c.style = palette_view::dim_style(c.style);
                }
            }
            self.caret_frame = None;
        }
        if let Some(p) = perf {
            buf.set_string(
                area.x + 1,
                area.bottom() - 1,
                p.summary(),
                Style::default().fg(self.palette.sub),
            );
        }
        // l'ancienne zone agrandie qui n'est plus réservée : tout y réécrire,
        // ce qui efface aussi les lettres agrandies restées à l'écran
        let region = self.scaled.as_ref().map(|s| s.region);
        if let Some(old) = self.last_scaled_region
            && region != Some(old)
        {
            let keep = region.unwrap_or_default();
            let old = old.intersection(area);
            for y in old.top()..old.bottom() {
                for x in old.left()..old.right() {
                    if !keep.contains(ratatui::layout::Position { x, y }) {
                        buf[(x, y)].set_diff_option(CellDiffOption::AlwaysUpdate);
                    }
                }
            }
        }
        self.last_scaled_region = region;
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
        let scale = if self.text_sizing {
            scale_for(self.store.config.float("fontSize"))
        } else {
            1
        };
        let words = words_box(area, max_line_width, zen, scale);
        let layout = self.visible_lines(words, now);
        let c = &self.store.config;
        let last_line = self
            .line_fade
            .map(|start| content.faded(OUT2.apply((now - start) / FADE_MS)));
        self.scaled = WordsView {
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

        // badge de langue au-dessus des mots, effacé en focus mode comme la barre
        self.words_top = Some(words.top);
        let badge = self.chrome.value(now) * opacity;
        let badge_row = words.top.saturating_sub(2);
        // jamais sur la ligne du logo ni sous la barre de config
        let bar_row = bar_layout(&self.store.config, area)
            .map(|(y, _)| y)
            .filter(|y| y + 1 < words.top);
        if badge > 0.0 && badge_row > area.y + 1 && bar_row.is_none_or(|b| b < badge_row) {
            let name = self.session.spec().language.replace('_', " ");
            let x = area.x + area.width.saturating_sub(name.width() as u16) / 2;
            let fg = self.palette.over_bg(self.palette.rgb.sub, badge);
            buf.set_string(x, badge_row, &name, Style::default().fg(fg));
        }
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
        let k = f64::from(words.scale);
        let target = CaretTarget {
            x: f64::from(words.left) + f64::from(col) * k,
            y: f64::from(words.top) + line as f64 * k,
            width: f64::from(width) * k,
            height: k,
        };
        if self.caret_reset {
            self.caret.jump(target);
            self.caret_reset = false;
        } else if target != self.caret.target() {
            // le curseur du terminal ne montre pas de position intermédiaire :
            // un glissement n'y ajouterait que du retard
            let ms = if self.renderer == CaretRenderer::Cell && style != CaretStyle::Block {
                0.0
            } else {
                smooth_caret_ms(self.store.config.str("smoothCaret"))
            };
            self.caret.go_to(target, self.input_at.min(now), ms);
        }
        let smooth_blink = self.store.config.str("smoothCaret") != "off";
        let mut f = self.caret.frame(style, now, smooth_blink);
        f.opacity *= opacity;
        match style {
            CaretStyle::Block if self.scaled.is_some() => {
                // lettres agrandies : on teinte le fond de chaque lettre couverte
                let caret = self.palette.rgb.caret;
                let st = self.scaled.as_mut()?;
                let s = st.scale;
                let col0 = ((f.x - f64::from(words.left)) / f64::from(s))
                    .floor()
                    .max(0.0) as u16;
                let row0 = ((f.y - f64::from(words.top)) / f64::from(s))
                    .floor()
                    .max(0.0) as u16;
                for row in row0..=row0 + 1 {
                    for col in col0..=col0 + (f.width / f64::from(s)).ceil() as u16 {
                        let (x, y) = (words.left + col * s, words.top + row * s);
                        let k = coverage_box(&f, x, y, s) * f.opacity;
                        if k <= 0.0 || x + s > st.region.right() || y + s > st.region.bottom() {
                            continue;
                        }
                        let bg = content.over_bg(caret, k);
                        match st.cells.iter_mut().find(|c| c.x == x && c.y == y) {
                            Some(c) => c.style = c.style.bg(bg),
                            None => st.cells.push(ScaledCell {
                                x,
                                y,
                                ch: ' ',
                                style: Style::default().bg(bg),
                            }),
                        }
                    }
                }
                None
            }
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

`crates/fasttype-tui/src/terminal.rs` (version complète) :
```rust
//! Entrée et sortie du mode plein écran, restauration garantie (y compris en
//! cas de panique), et tampon qui envoie chaque image en une seule écriture.

use crate::app::{CaretLook, CaretShape};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{
    DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
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
static GRAPHICS: AtomicBool = AtomicBool::new(false);

/// Le terminal a répondu OK à la sonde graphique Kitty au démarrage.
pub fn graphics_supported() -> bool {
    GRAPHICS.load(Ordering::SeqCst)
}

/// Des images Kitty (caret) seront affichées : `restore` les supprimera.
pub fn mark_kitty_images() {
    KITTY_IMAGES.store(true, Ordering::SeqCst);
}

static HOOK: Once = Once::new();
static TEXT_SIZING: AtomicBool = AtomicBool::new(false);

/// Le terminal a montré au démarrage qu'il agrandit le texte (OSC 66).
pub fn text_sizing_supported() -> bool {
    TEXT_SIZING.load(Ordering::SeqCst)
}

/// Envoie une sonde et lit la réponse directement sur l'entrée, avant que le
/// thread clavier ne la lise : 300 ms au plus (les terminaux répondent bien avant).
#[cfg(unix)]
fn probe(
    out: &mut impl Write,
    query: &[u8],
    answer: fn(&[u8]) -> Option<bool>,
) -> io::Result<bool> {
    use std::time::{Duration, Instant};
    out.write_all(query)?;
    out.flush()?;
    let deadline = Instant::now() + Duration::from_millis(300);
    let mut got = Vec::new();
    loop {
        if let Some(ok) = answer(&got) {
            return Ok(ok);
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(false);
        }
        let mut fds = libc::pollfd {
            fd: libc::STDIN_FILENO,
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY : un seul descripteur valide, tampon local de taille connue.
        let ready = unsafe { libc::poll(&mut fds, 1, left.as_millis() as libc::c_int) };
        if ready <= 0 {
            return Ok(false);
        }
        let mut buf = [0u8; 256];
        let n = unsafe { libc::read(libc::STDIN_FILENO, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            return Ok(false);
        }
        got.extend_from_slice(&buf[..n as usize]);
    }
}

#[cfg(not(unix))]
fn probe(
    _out: &mut impl Write,
    _query: &[u8],
    _answer: fn(&[u8]) -> Option<bool>,
) -> io::Result<bool> {
    Ok(false)
}

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
        DisableBracketedPaste,
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
        execute!(out, EnterAlternateScreen, EnableBracketedPaste)?;
        // avant toute autre lecture de l'entrée (protocole clavier, thread clavier)
        let graphics = probe(&mut out, crate::kitty::PROBE, crate::kitty::probe_answer)?;
        GRAPHICS.store(graphics, Ordering::SeqCst);
        let sizing = probe(&mut out, crate::sized::PROBE, crate::sized::probe_answer)?;
        TEXT_SIZING.store(sizing, Ordering::SeqCst);
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

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (15 dans `command_line`).

- [ ] **Step 6 : test pty et benchmark**

`scripts/pty_smoke.py` (version complète) :
```python
"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf] [--kitty] [--sized] [--silent] [--palette] [--sigterm]
  --kitty    le terminal répond OK à la sonde graphique Kitty (cases de 10 × 20 pixels)
  --sized    le terminal agrandit le texte (OSC 66, Kitty ≥ 0.40)
  --silent   le terminal ne répond à aucune sonde (ni position, ni DA1) : chaque
             sonde doit abandonner au bout de 300 ms
  --palette  quitte par la palette de commandes (Échap, « quit », Entrée)
  --sigterm  quitte par SIGTERM au lieu de Ctrl+C"""
import fcntl, os, pty, re, select, signal, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
flags = sys.argv[3:]
perf, kitty, sigterm = "--perf" in flags, "--kitty" in flags, "--sigterm" in flags
sized, silent = "--sized" in flags, "--silent" in flags
palette = "--palette" in flags
probe_at, keyboard_at = None, None
os.makedirs(f"{home}/.config/fasttype", exist_ok=True)
with open(f"{home}/.config/fasttype/config.toml", "w") as f:
    f.write('mode = "custom"\n')

pid, fd = pty.fork()
if pid == 0:
    os.environ.update({"HOME": home, "TERM": "xterm-256color", "COLORTERM": "truecolor"})
    os.environ.pop("FASTTYPE_CARET", None)
    os.environ.pop("XDG_CONFIG_HOME", None)
    os.environ.pop("XDG_DATA_HOME", None)
    os.execv(binary, [binary] + (["--perf"] if perf else []))

# 30 lignes × 100 colonnes ; 1000 × 600 pixels → cases de 10 × 20
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 100, 1000, 600))
out = bytearray()

def pump(seconds):
    global probe_at, keyboard_at
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
        if probe_at is None and b"a=q" in data:
            probe_at = time.time()
        if keyboard_at is None and b"\x1b[?u" in data:
            keyboard_at = time.time()
        if silent:
            # seule la requête du protocole clavier (crossterm) reçoit sa réponse DA1
            if b"\x1b[?u" in data:
                os.write(fd, b"\x1b[?62c")
            continue
        # sonde graphique : OK seulement si l'on joue un terminal Kitty
        if kitty and b"a=q" in data:
            os.write(fd, b"\x1b_Gi=31;OK\x1b\\")
        # réponses d'un terminal sans protocole clavier Kitty
        cpr = data.count(b"\x1b[6n")
        if sized and cpr == 2 and b"\x1b]66;" in data:
            os.write(fd, b"\x1b[1;1R\x1b[1;3R")  # l'espace agrandi a avancé de 2 cases
        elif cpr:
            os.write(fd, b"\x1b[1;1R" * cpr)
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
elif palette:
    os.write(fd, b"\x1b")
    pump(0.3)
    for ch in "quit":
        os.write(fd, ch.encode())
        pump(0.05)
    pump(0.2)
    palette_seen = "Quit" in plain(bytes(out))
    os.write(fd, b"\r")
else:
    os.write(fd, b"\x03")  # Ctrl+C
pump(1.0)
_, status = os.waitpid(pid, 0)
print("exit", os.waitstatus_to_exitcode(status))
print("result screen:", "test type custom english" in screen)
print("alt screen left:", b"\x1b[?1049l" in out)
print("sync output used:", b"\x1b[?2026h" in out and b"\x1b[?2026l" in out)
print("cursor color reset:", b"\x1b]112\x07" in out)
print("bracketed paste reset:", b"\x1b[?2004l" in out)
if kitty:
    print("kitty caret placed:", b"\x1b_Ga=p," in out)
    print("kitty images deleted:", out.rstrip().find(b"\x1b_Ga=d,d=A,q=2\x1b\\") > out.find(b"\x1b_Ga=p,"))
if palette:
    print("command line shown:", palette_seen)
if silent:
    wait = (keyboard_at - probe_at) * 1000 if probe_at and keyboard_at else None
    print("probes gave up after (ms):", round(wait) if wait else None)
if sized:
    print("scaled words written:", b"\x1b]66;s=2;" in out)
if perf:
    found = re.findall(r"key→flush p50 ([0-9.]+) p99 ([0-9.]+) ms · frame p99 ([0-9.]+) ms · n (\d+)", bytes(out).decode("utf-8", "replace"))
    print("perf (p50, p99, frame p99, n):", found[-1] if found else None)
```

Lancer chaque mode **séparément**, avec ses options en arguments distincts :
```bash
cargo build --release -p fasttype-tui
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --palette
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --perf
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --kitty --sized --perf
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --silent
```
Expected : pour chacun, `exit 0`, `result screen: True`, `alt screen left: True`, `sync output used: True`, `cursor color reset: True` et `bracketed paste reset: True`. En plus :
- avec `--palette` : `command line shown: True` ;
- avec `--kitty --sized` : les trois lignes Kitty et OSC 66 à `True` ;
- avec `--silent` : `probes gave up after (ms):` vers 600 ;
- p50 sous 0,5 ms.

Run: `cargo bench -p fasttype-tui --bench frame 2>&1 | grep -A1 -E "^(frame|key)"`
Expected : les trois benchmarks sous 1 ms (mesurés vers 77 µs).

- [ ] **Step 7 : vérification finale et essai à la main**

Run: `cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, aucun avertissement.

Essai à la main, dans un vrai terminal :
- `esc`, puis taper « theme » et Entrée ; parcourir les thèmes avec ↓ et regarder l'aperçu, puis `esc` deux fois ; le thème d'origine doit revenir ;
- choisir « Time... » puis « 60 » ; un test de 60 s doit démarrer.

Les tests automatiques ne jugent pas l'aspect : le noter dans le ledger, et signaler à l'utilisateur que l'essai visuel lui revient.

- [ ] **Step 8 : commit**

```bash
git add crates/fasttype-tui scripts
git commit -m "feat(tui): palette de commandes à l'écran (esc, aperçu des thèmes, Next/Repeat test, Bail out, Quit)"
```
