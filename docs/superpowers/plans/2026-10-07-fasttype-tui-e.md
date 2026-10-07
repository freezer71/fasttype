# fasttype-tui, partie E : commandes de la palette — plan d'implémentation (plan 4e)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Terminer la palette de commandes de la v1 :
- Change custom text, Search for quotes, Export settings et Import settings ;
- notifications d'erreur qui restent jusqu'à « Clear all notifications » ;
- grosses langues décompressées en arrière-plan, avec « loading... » sur le badge.

**Architecture:**
- **`SessionFactory`** :
  - partage son cache de langues avec un thread de préchargement (`preload`, `language_ready`) ;
  - garde le texte custom choisi et la citation choisie (`quoteLength = [-2]`).
- **La palette** distingue les saisies libres (`InputTarget` : clé de config, texte custom, import TOML) et porte des actions avec données (`AppAction::SelectQuote(id)`, `SetCustomText`, `ImportSettings`). La liste des citations, plusieurs milliers d'entrées, est construite seulement à la demande.
- **`App`** exécute ces actions :
  - texte custom enregistré dans `custom_texts/current.txt` et relu au démarrage ;
  - réglages exportés vers le presse-papiers par OSC 52, que la boucle écrit dans l'image ;
  - import par `Config::from_toml`, avec ses avertissements ;
  - changement de langue : test relancé quand la langue est prête.
- **Les notifications** d'erreur n'expirent plus (et ne réveillent jamais la boucle). Pendant la frappe, seules les erreurs restent visibles.

**Tech Stack:** Rust 1.97 (édition 2024), `ratatui` 0.30.2, `toml` 1.1.6, OSC 52 (presse-papiers du terminal), Python 3 pour le test pty.

**Spec:** `docs/superpowers/specs/2026-10-07-fasttype-v1-design.md` (§5.3 palette et notifications, §6.1 chargement en arrière-plan, §6.2 textes custom). Les plans 4a à 4d sont livrés sur `main`.

**Code vérifié avant rédaction :** tout le code de ce plan a été assemblé et exécuté dans une copie de travail jetable.
- **Tests :** 384 tests du workspace au vert, et clippy ne signale rien.
- **Test pty :** `--palette`, `--export` (le TOML arrive bien dans OSC 52) et `--perf` (p50 0,17 ms) passent.
- **Recherche dans les 6 488 citations anglaises :** environ 1,5 ms par frappe, rendu compris. L'ouverture de la liste coûte 27 ms, une fois.
- **Chargement en arrière-plan de `french_600k`** (634 000 mots) : le test démarre à la fin du chargement.

**Comportements de référence (Monkeytype, commit 574d819) :**
- **Search for quotes :** choisir une citation met `quoteLength` à `[-2]`, et cette citation revient à chaque restart.
- **Change custom text :** le mode passe à custom et le texte est gardé.
- **Notifications** (`states/notifications.ts`) :
  - les erreurs ont une durée 0, c'est-à-dire qu'elles restent jusqu'à ce qu'on les ferme ;
  - les notices disparaissent après 3 s ;
  - pendant la frappe, les notifications non importantes sont cachées.
- **Spec §6.1 :** les grosses langues sont décompressées dans un thread, avec « loading… » sur le badge.

## Global Constraints

- Rust 1.97, édition 2024, licence `GPL-3.0-only`. Textes de l'interface en anglais. Commentaires en français.
- `App` ne fait aucune E/S terminal : le presse-papiers passe par `take_clipboard`, que la boucle écrit en OSC 52 dans la même sortie synchronisée.
- **Le chargement d'une langue ne bloque jamais la boucle.** Pendant le chargement, une image est prévue à chaque échéance d'animation, pour voir la fin. Le test en cours reste affiché.
- **Aucune échéance infinie dans `next_deadline` :** `Duration::from_secs_f64(inf)` paniquerait.
- Le coût par frappe reste celui de 4b : benchmark sous 1 ms.
- Chaque tâche se termine par `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` et le total des tests du workspace, tous au vert.

**Choix assumés :**
- **Texte custom simple :** les mots du texte, répétés, sans les options de limite du site (sections, temps, délimiteur `|`), qui viendront en v2. Il est enregistré sous le nom `current`.
- **Export :** le terminal copie le TOML dans le presse-papiers (OSC 52), là où le site affiche le JSON à copier. Certains terminaux ignorent OSC 52 ; le fichier `config.toml` reste la référence.
- **Import :** du TOML (le format de `config.toml`), pas le JSON du site.
- **Recherche de citations :** dans la palette, avec sa recherche par débuts de mots, et non dans la fenêtre dédiée du site.

## Review Focus

1. **Une langue qui se charge pendant qu'on tape ou qu'on rouvre la palette.** Le test en cours ne doit pas être perdu avant la fin du chargement, et rien ne doit bloquer. Test : `a_new_language_loads_in_the_background` (tâche 2).
2. **Erreur persistante.** Aucun réveil de la boucle, et elle reste visible pendant la frappe. Test : `errors_stay_and_notices_hide_while_typing` (tâche 2).
3. **Import d'un TOML invalide ou partiel.** Les clés valides doivent s'appliquer et les autres être signalées, sans écran cassé. Test : `export_copies_the_settings_and_import_applies_them` (tâche 2), où l'import est partiel ; les clés invalides passent par `Config::from_toml`, déjà testé dans `fasttype-store`.
4. **Citation choisie puis changement de langue ou de longueur.** `quoteLength` n'est plus `[-2]`, donc le tirage reprend au hasard. Test : `a_selected_quote_is_used_with_quote_length_minus_two` (tâche 1).
5. **Texte custom vide ou fait d'espaces.** Il doit être refusé dans la palette, et la fabrique doit retomber sur le texte du site. Tests : `custom_text_and_settings_entries` et `custom_mode_uses_the_chosen_text` (tâches 1 et 2).

---

## Structure des fichiers

```
crates/fasttype-data/src/catalog.rs        LanguageCache::is_ready
crates/fasttype-tui/
├── src/session_factory.rs                 Arc<LanguageCache>, preload, language_ready, quotes (public), custom_text, selected_quote
├── src/palette/mod.rs                     InputTarget ; AppAction avec données
├── src/palette/lists.rs                   Change custom text, Search for quotes, Import/Export settings ; Context.custom_text
├── src/palette/state.rs                   saisie selon InputTarget ; push ; collage TOML gardé tel quel
├── src/view/notify.rs                     erreurs persistantes ; render(…, typing)
├── src/app.rs                             nouvelles actions, chargement en arrière-plan, presse-papiers
├── src/runner.rs                          OSC 52
├── tests/{factory,palette}.rs             compléments
└── tests/commands.rs                      nouveau
scripts/pty_smoke.py                       + --export
```

---

### Task 1 : langues en arrière-plan, texte custom et citation choisie dans la fabrique

**Files:**
- Modify: `crates/fasttype-data/src/catalog.rs`, `crates/fasttype-tui/src/session_factory.rs` (version complète)
- Test: `crates/fasttype-data/tests/catalog.rs`, `crates/fasttype-tui/tests/factory.rs`

**Interfaces:**
- Produces :
  - `LanguageCache::is_ready(&self, name) -> bool`, sans charger ni attendre ;
  - `SessionFactory::{language_ready, preload(name) -> JoinHandle<()>, quotes(language) -> Option<Arc<QuoteFile>>}` ;
  - les champs publics `custom_text: Option<String>` et `selected_quote: Option<u32>`.
- Comportement de `build` :
  - en custom, le texte choisi est utilisé, ou celui du site s'il est vide ;
  - en quote, la citation choisie est utilisée si `quoteLength` contient -2.

- [ ] **Step 1 : écrire les tests qui échouent**

Ajouter à la fin de `crates/fasttype-data/tests/catalog.rs` :
```rust
#[test]
fn cache_tells_whether_a_language_is_ready_without_loading_it() {
    let cache = LanguageCache::new();
    assert!(!cache.is_ready("french"));
    assert!(!cache.is_ready("french"), "la question ne charge rien");
    cache.get("french").unwrap();
    assert!(cache.is_ready("french"));
    assert!(!cache.is_ready("english"));
}
```

Ajouter à la fin de `crates/fasttype-tui/tests/factory.rs` :
```rust
#[test]
fn custom_mode_uses_the_chosen_text() {
    let mut f = SessionFactory::new();
    f.custom_text = Some("alpha beta gamma".into());
    let built = f.build(&config("mode = \"custom\"\n"), 7);
    let words: Vec<&str> = built.session.words().iter().map(|w| w.trim_end()).collect();
    assert_eq!(&words[..3], ["alpha", "beta", "gamma"]);
    // un texte vide garde celui du site
    f.custom_text = Some("   ".into());
    let built = f.build(&config("mode = \"custom\"\n"), 7);
    assert_ne!(built.session.word(0).trim_end(), "");
}

#[test]
fn a_selected_quote_is_used_with_quote_length_minus_two() {
    let mut f = SessionFactory::new();
    let file = f.quotes("english").expect("citations anglaises");
    let id = file.quotes[123].id;
    f.selected_quote = Some(id);
    for seed in [1, 2, 3] {
        let built = f.build(&config("mode = \"quote\"\nquote_length = [-2]\n"), seed);
        assert_eq!(built.session.spec().quote.as_ref().unwrap().id, id);
    }
    // sans -2, la citation est tirée au hasard dans les groupes
    let built = f.build(&config("mode = \"quote\"\nquote_length = [0]\n"), 5);
    assert_eq!(built.session.spec().quote.as_ref().unwrap().group, 0);
}

#[test]
fn languages_can_be_preloaded_in_the_background() {
    let f = SessionFactory::new();
    assert!(!f.language_ready("german"));
    f.preload("german").join().unwrap();
    assert!(f.language_ready("german"));
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-data --test catalog; cargo test -p fasttype-tui --test factory`
Expected: échecs de compilation (`no method named is_ready`, `no field custom_text`).

- [ ] **Step 3 : implémenter**

Dans `crates/fasttype-data/src/catalog.rs`, ajouter à `impl LanguageCache`, après `get` :
```rust
    /// La langue est déjà chargée (sans attendre ni la charger).
    pub fn is_ready(&self, name: &str) -> bool {
        let loaded = self.loaded.lock().unwrap_or_else(|p| p.into_inner());
        loaded.get(name).is_some_and(|slot| slot.get().is_some())
    }
```

`crates/fasttype-tui/src/session_factory.rs` (version complète) :
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
    /// Partagé avec le thread qui précharge une langue choisie dans la palette.
    languages: Arc<LanguageCache>,
    quotes: HashMap<String, Option<Arc<QuoteFile>>>,
    /// Texte du mode custom (« Change custom text ») ; sinon celui du site.
    pub custom_text: Option<String>,
    /// Citation choisie par « Search for quotes » (`quoteLength = [-2]`).
    pub selected_quote: Option<u32>,
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

    /// La langue est déjà décompressée : un test peut être créé sans attendre.
    pub fn language_ready(&self, name: &str) -> bool {
        self.languages.is_ready(name)
    }

    /// Décompresse une langue dans un thread (les plus grosses prennent des
    /// dizaines de millisecondes) ; le prochain `build` la trouvera en cache.
    pub fn preload(&self, name: &str) -> std::thread::JoinHandle<()> {
        let cache = Arc::clone(&self.languages);
        let name = name.to_string();
        std::thread::spawn(move || {
            let _ = cache.get(&name);
        })
    }

    /// Citations d'une langue (décompressées une fois, puis gardées).
    pub fn quotes(&mut self, language: &str) -> Option<Arc<QuoteFile>> {
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
            Err(_) => {
                warning = Some(format!(
                    "language {requested} not found - using {DEFAULT_LANGUAGE}"
                ));
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
                let text = self
                    .custom_text
                    .as_deref()
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or(DEFAULT_CUSTOM_TEXT);
                let limit = CustomLimit::Word(text.split_whitespace().count() as u32);
                let source = CustomWords::new(text, CustomMode::Repeat, limit, false);
                (
                    TestSpec::custom(limit, &language, punctuation, numbers),
                    WordGenerator::new(Box::new(source), &language, punctuation, numbers),
                )
            }
            "quote" => {
                let lengths = config.int_list("quoteLength");
                let selected = self.selected_quote.filter(|_| lengths.contains(&-2));
                let picked = self.quotes(&language).and_then(|file| {
                    let chosen = selected.and_then(|id| file.quotes.iter().find(|q| q.id == id));
                    let q = match chosen {
                        Some(q) => q,
                        None => pick_quote(&file, &lengths, &mut rng)?,
                    };
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

Run: `cargo fmt && cargo test -p fasttype-data --test catalog && cargo test -p fasttype-tui --test factory && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (7 dans `factory`).

- [ ] **Step 5 : commit**

```bash
git add crates/fasttype-data crates/fasttype-tui
git commit -m "feat(tui): langues préchargées en arrière-plan, texte custom et citation choisie"
```

---

### Task 2 : commandes de la palette, notifications persistantes, presse-papiers

**Files:**
- Modify: `crates/fasttype-tui/src/palette/mod.rs`, `crates/fasttype-tui/src/palette/lists.rs`, `crates/fasttype-tui/src/palette/state.rs`, `crates/fasttype-tui/src/view/notify.rs`, `crates/fasttype-tui/src/app.rs`, `crates/fasttype-tui/src/runner.rs` (versions complètes)
- Modify: `crates/fasttype-tui/tests/palette.rs` (version complète), `scripts/pty_smoke.py` (version complète)
- Test: `crates/fasttype-tui/tests/commands.rs`

**Interfaces:**
- Consumes : tâche 1.
- Produces :
  - `palette::InputTarget { Config(&'static str), CustomText(String), ImportSettings }` ;
  - `Action::Input(InputTarget)`, au lieu de `Action::Input { key }` ;
  - `AppAction::{SearchQuotes, SelectQuote(u32), SetCustomText(String), ExportSettings, ImportSettings(String)}`. `AppAction` n'est plus `Copy` ;
  - `lists::Context.custom_text` ;
  - `PaletteState::push(Subgroup)` ;
  - `Notifications::render(…, typing: bool)` ;
  - `App::{take_clipboard, loading}` et `app::CURRENT_CUSTOM_TEXT`.

- [ ] **Step 1 : écrire les tests qui échouent**

`crates/fasttype-tui/tests/palette.rs` (version complète) :
```rust
mod common;

use common::app;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use fasttype_tui::input::{Input, Key, map_event, map_key};
use fasttype_tui::palette::filter::{filter, split_words};
use fasttype_tui::palette::lists::{Context, root};
use fasttype_tui::palette::state::{Outcome, PaletteState, parse_input};
use fasttype_tui::palette::{Action, AppAction, InputTarget, Subgroup};
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
        custom_text: "hello world",
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
    assert_eq!(
        time.list[4].action,
        Action::Input(InputTarget::Config("time"))
    );
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
    assert_eq!(font.action, Action::Input(InputTarget::Config("fontSize")));
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

#[test]
fn typing_replaces_the_prefilled_value() {
    let a = app("pal-replace", "");
    let c = &a.store.config;
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "time".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    p.key(Key::Up, c);
    p.key(Key::Up, c);
    p.key(Key::Enter, c);
    assert_eq!(p.input().unwrap().text, "30");
    assert!(
        p.input().unwrap().selected,
        "valeur pré-remplie sélectionnée"
    );
    p.key(Key::Char('4'), c);
    p.key(Key::Char('5'), c);
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::Set {
            key: "time",
            value: Value::Integer(45)
        }),
        "comme sur le site : la frappe remplace la valeur"
    );
}

#[test]
fn aliases_from_the_site() {
    let a = app("pal-alias", "");
    let c = &a.store.config;
    let find = |q: &str, on_result: bool| -> Vec<String> {
        let mut p = PaletteState::open(ctx_root(c, on_result, false));
        q.chars().for_each(|ch| {
            p.key(Key::Char(ch), c);
        });
        p.shown().0.iter().map(|c| c.display.clone()).collect()
    };
    assert_eq!(find("words", false), ["Word count..."]);
    assert_eq!(
        find("quotes", false),
        ["Quote length...", "Search for quotes"]
    );
    assert!(find("wpm", false).contains(&"Live speed style...".to_string()));
    assert!(find("timer", false).contains(&"Live progress style...".to_string()));
    assert!(find("page", false).contains(&"Max line width...".to_string()));
    assert!(find("restart", true).contains(&"Next test".to_string()));
    assert!(find("opacity", false).contains(&"Live progress opacity...".to_string()));
}

#[test]
fn custom_text_and_settings_entries() {
    let a = app("pal-extra", "");
    let c = &a.store.config;
    let r = ctx_root(c, false, false);
    let l = labels(&r);
    for want in [
        "Change custom text",
        "Search for quotes",
        "Import settings",
        "Export settings",
    ] {
        assert!(l.contains(&want), "{want}");
    }
    // le texte custom part du texte en cours ; vide : refusé
    let mut p = PaletteState::open(r);
    "custom text".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    assert_eq!(p.input().unwrap().text, "hello world");
    for _ in 0..11 {
        p.key(Key::Backspace, c);
    }
    assert_eq!(p.key(Key::Enter, c), Outcome::Stay);
    assert_eq!(
        p.input().unwrap().error.as_deref(),
        Some("Must not be empty")
    );
    p.paste("one\ntwo");
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::App(AppAction::SetCustomText("one two".into())))
    );
    // l'import garde les sauts de ligne du TOML collé
    let mut p = PaletteState::open(ctx_root(c, false, false));
    "import".chars().for_each(|ch| {
        p.key(Key::Char(ch), c);
    });
    p.key(Key::Enter, c);
    p.paste("time = 60\ntheme = \"dracula\"\n");
    assert_eq!(
        p.key(Key::Enter, c),
        Outcome::Run(Action::App(AppAction::ImportSettings(
            "time = 60\ntheme = \"dracula\"\n".into()
        )))
    );
}
```

`crates/fasttype-tui/tests/commands.rs` :
```rust
mod common;

use common::{app, press, render, screen_text, settle, type_text};
use fasttype_core::spec::Mode;
use fasttype_tui::app::{App, CURRENT_CUSTOM_TEXT};
use fasttype_tui::input::{Input, Key};
use fasttype_tui::theme::ColorMode;

fn command(a: &mut App, query: &str, at: f64) {
    a.handle(press(Key::Esc, at));
    for c in query.chars() {
        a.handle(press(Key::Char(c), at));
    }
    a.handle(press(Key::Enter, at));
}

#[test]
fn change_custom_text_starts_a_custom_test_and_is_kept() {
    let mut a = app("ex-custom", "");
    command(&mut a, "custom text", 0.0);
    // la saisie part du texte en cours : on le remplace
    for _ in 0..400 {
        a.handle(press(Key::Backspace, 0.0));
    }
    a.handle(Input::Paste("the quick brown fox".into()));
    a.handle(press(Key::Enter, 0.0));
    settle(&mut a, 0.0);
    assert_eq!(a.store.config.str("mode"), "custom");
    assert_eq!(a.session().spec().mode, Mode::Custom);
    assert_eq!(a.session().word(0).trim_end(), "the");
    assert_eq!(
        a.store.custom_texts.load(CURRENT_CUSTOM_TEXT).unwrap(),
        "the quick brown fox"
    );
    // au prochain lancement, le texte est relu
    let reopened = App::new(
        fasttype_store::Store::open(common::paths(
            &std::env::temp_dir().join(format!("fasttype-tui-{}-ex-custom", std::process::id())),
        )),
        ColorMode::TrueColor,
        0.0,
        1,
    );
    assert_eq!(reopened.session().word(0).trim_end(), "the");
}

#[test]
fn search_for_quotes_picks_that_quote_every_time() {
    let mut a = app("ex-quote", "");
    command(&mut a, "search quotes", 0.0);
    let p = a.command_line().expect("liste des citations");
    assert_eq!(p.title(), "Search for quotes");
    let (shown, _) = p.shown();
    assert!(shown.len() > 1000, "toutes les citations : {}", shown.len());
    let wanted = shown[42].display.clone();
    // chercher les deux premiers mots de la citation
    let words: Vec<&str> = wanted.split(' ').take(3).collect();
    for c in words.join(" ").chars() {
        a.handle(press(Key::Char(c), 0.0));
    }
    let id = {
        let (shown, active) = a.command_line().unwrap().shown();
        assert!(!shown.is_empty());
        match &shown[active].action {
            fasttype_tui::palette::Action::App(fasttype_tui::palette::AppAction::SelectQuote(
                id,
            )) => *id,
            other => panic!("{other:?}"),
        }
    };
    a.handle(press(Key::Enter, 0.0));
    let t = settle(&mut a, 0.0);
    assert_eq!(a.store.config.int_list("quoteLength"), [-2]);
    assert_eq!(a.session().spec().quote.as_ref().unwrap().id, id);
    // un restart garde la même citation
    a.handle(press(Key::Tab, t));
    a.handle(press(Key::Enter, t));
    settle(&mut a, t);
    assert_eq!(a.session().spec().quote.as_ref().unwrap().id, id);
}

#[test]
fn export_copies_the_settings_and_import_applies_them() {
    let mut a = app("ex-export", "time = 60\n");
    command(&mut a, "export", 0.0);
    let toml = a.take_clipboard().expect("réglages à copier");
    assert!(toml.contains("time = 60"), "{toml}");
    assert!(a.take_clipboard().is_none(), "une seule fois");
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text == "Settings copied to clipboard")
    );
    let original = *a.palette();
    command(&mut a, "import", 0.0);
    a.handle(Input::Paste("time = 15\ntheme = \"dracula\"\n".into()));
    a.handle(press(Key::Enter, 0.0));
    settle(&mut a, 0.0);
    assert_eq!(a.store.config.int("time"), 15);
    assert_eq!(a.store.config.str("theme"), "dracula");
    assert_ne!(*a.palette(), original, "le thème importé est appliqué");
    assert_eq!(a.session().spec().time_limit, Some(15));
}

#[test]
fn a_new_language_loads_in_the_background() {
    let mut a = app("ex-lang", "");
    command(&mut a, "language", 0.0);
    for c in "french 600k".chars() {
        a.handle(press(Key::Char(c), 0.0));
    }
    a.handle(press(Key::Enter, 0.0));
    assert_eq!(a.loading(), Some("french_600k"));
    assert!(
        a.next_deadline().is_some(),
        "on surveille la fin du chargement"
    );
    let (buf, _) = render(&mut a, 100, 30);
    assert!(screen_text(&buf).contains("loading..."));
    let mut t = 0.0;
    while a.loading().is_some() && t < 10_000.0 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        t += 5.0;
        a.tick(t);
    }
    assert!(a.loading().is_none(), "chargé");
    settle(&mut a, t);
    assert_eq!(a.session().spec().language, "french_600k");
}

#[test]
fn errors_stay_and_notices_hide_while_typing() {
    let mut a = app("ex-notif", "language = \"klingon_9000k\"\n");
    a.tick(100_000.0);
    assert!(
        a.notifications
            .items()
            .iter()
            .any(|n| n.text.contains("klingon")),
        "l'erreur reste jusqu'à ce qu'on l'efface"
    );
    assert_eq!(
        a.next_deadline(),
        None,
        "pas de réveil pour une erreur sans fin"
    );
    a.notifications.push(
        "Quick restart disabled in long tests. Use shift + tab.",
        fasttype_tui::view::notify::Level::Notice,
        100_000.0,
    );
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 100_000.0, 10.0);
    let (buf, _) = render(&mut a, 120, 30);
    let text = screen_text(&buf);
    assert!(text.contains("klingon"), "l'erreur reste visible");
    assert!(
        !text.contains("Quick restart"),
        "la notice se cache pendant la frappe"
    );
}
```

- [ ] **Step 2 : lancer les tests pour vérifier qu'ils échouent**

Run: `cargo test -p fasttype-tui --test palette --test commands`
Expected: échec de compilation (`unresolved import InputTarget`, `no method named take_clipboard`).

- [ ] **Step 3 : implémenter la palette et les notifications**

`crates/fasttype-tui/src/palette/mod.rs` (version complète) :
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
    /// Demande une valeur libre (« custom... », texte custom, import).
    Input(InputTarget),
    /// Action propre à l'application.
    App(AppAction),
    /// Ne fait rien et ferme la palette (« Nevermind »).
    Close,
}

/// Ce que remplit une saisie libre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputTarget {
    /// Une clé de config (nombre libre).
    Config(&'static str),
    /// Le texte du mode custom ; la saisie part du texte en cours.
    CustomText(String),
    /// Des réglages au format TOML (collés).
    ImportSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    NextTest,
    RepeatTest,
    BailOut,
    ClearNotifications,
    Quit,
    /// Ouvre la liste des citations de la langue (construite à la demande).
    SearchQuotes,
    /// Lance la citation choisie.
    SelectQuote(u32),
    SetCustomText(String),
    /// Copie les réglages (TOML) dans le presse-papiers.
    ExportSettings,
    ImportSettings(String),
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

`crates/fasttype-tui/src/palette/lists.rs` (version complète) :
```rust
//! Liste racine de la palette (`commandline/lists.ts`), limitée aux réglages
//! qui ont un effet dans fasttype v1. Libellés : `display` du schéma avec une
//! majuscule et « ... » ; dans un sous-groupe, `on`/`off` ou la valeur, `off`
//! en premier.

use super::{Action, AppAction, Command, InputTarget, Subgroup};
use fasttype_store::Config;
use fasttype_store::schema::{Kind, key_def};
use toml::Value;

/// État de l'application utile aux listes.
pub struct Context<'a> {
    pub config: &'a Config,
    /// Texte du mode custom en cours (point de départ de « Change custom text »).
    pub custom_text: &'a str,
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
        "timerOpacity" => "live progress opacity",
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
    Command::new("custom...", Action::Input(InputTarget::Config(key)))
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

/// Alias de recherche du site (`commandline-metadata.ts`).
fn alias(key: &str) -> &'static str {
    match key {
        "words" => "words",
        "quoteLength" => "quotes",
        "liveSpeedStyle" | "liveAccStyle" | "liveBurstStyle" => "wpm",
        "timerStyle" => "timer",
        "timerColor" | "timerOpacity" => "timer speed wpm burst acc",
        "maxLineWidth" => "page",
        _ => "",
    }
}

/// Commande d'une clé : sous-groupe de ses valeurs, ou saisie directe pour
/// les nombres libres (`fontSize`, `maxLineWidth`).
fn key_command(key: &'static str, ctx: &Context) -> Command {
    let title = label(key);
    let list = values(key, ctx);
    if list.is_empty() {
        return Command::new(title, Action::Input(InputTarget::Config(key))).alias(alias(key));
    }
    Command::new(
        title.clone(),
        Action::Open(Subgroup {
            title: title.trim_end_matches("...").to_string(),
            list,
        }),
    )
    .alias(alias(key))
}

pub fn root(ctx: &Context) -> Subgroup {
    let mut list = Vec::new();
    if ctx.on_result {
        list.push(
            Command::new("Next test", Action::App(AppAction::NextTest))
                .alias("restart start begin type test typing"),
        );
        list.push(Command::new(
            "Repeat test",
            Action::App(AppAction::RepeatTest),
        ));
    }
    for group in [TEST, BEHAVIOR, CARET, APPEARANCE, THEME, SHOW_HIDE] {
        list.extend(group.iter().map(|k| key_command(k, ctx)));
        if group == TEST {
            list.push(Command::new(
                "Change custom text",
                Action::Input(InputTarget::CustomText(ctx.custom_text.to_string())),
            ));
            list.push(Command::new(
                "Search for quotes",
                Action::App(AppAction::SearchQuotes),
            ));
        }
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
    list.push(Command::new(
        "Import settings",
        Action::Input(InputTarget::ImportSettings),
    ));
    list.push(Command::new(
        "Export settings",
        Action::App(AppAction::ExportSettings),
    ));
    list.push(Command::new("Quit", Action::App(AppAction::Quit)).alias("exit close"));
    Subgroup {
        title: "Search...".into(),
        list,
    }
}
```

`crates/fasttype-tui/src/palette/state.rs` (version complète) :
```rust
//! Palette ouverte (`CommandlineModal.tsx`) : pile de sous-groupes, recherche,
//! commande active, saisie libre. Ne touche ni à la config ni au test : elle
//! renvoie l'action choisie à `App`.

use super::filter::filter;
use super::{Action, AppAction, Command, InputTarget, Subgroup};
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
    pub target: InputTarget,
    pub title: String,
    pub text: String,
    /// Message sous la saisie quand la valeur est refusée.
    pub error: Option<String>,
    /// La valeur pré-remplie est sélectionnée : la première frappe la remplace
    /// (`setSelectionRange(0, len)` du site).
    pub selected: bool,
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

    /// Ouvre un sous-groupe construit à la demande (« Search for quotes »).
    pub fn push(&mut self, group: Subgroup) {
        self.stack.push(group);
        self.query.clear();
        self.refresh();
    }

    /// Texte collé (bracketed paste). Les sauts de ligne deviennent des
    /// espaces, sauf dans un import de réglages (TOML).
    pub fn paste(&mut self, text: &str) {
        let flat: String = text
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        match &mut self.input {
            Some(m) => {
                if std::mem::take(&mut m.selected) {
                    m.text.clear();
                }
                if m.target == InputTarget::ImportSettings {
                    m.text.push_str(text);
                } else {
                    m.text.push_str(&flat);
                }
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
                    Action::Input(target) => {
                        let text = match &target {
                            InputTarget::Config(key) => current_text(config, key),
                            InputTarget::CustomText(current) => current.clone(),
                            InputTarget::ImportSettings => String::new(),
                        };
                        self.input = Some(InputMode {
                            target,
                            title: cmd.display.clone(),
                            text,
                            error: None,
                            selected: true,
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
        // texte sélectionné : une lettre le remplace, un effacement le vide
        if m.selected && matches!(key, Key::Char(_) | Key::Backspace | Key::DeleteWord) {
            m.text.clear();
        }
        m.selected = false;
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
            Key::Enter | Key::ShiftEnter => {
                let text = m.text.trim();
                let done = match &m.target {
                    InputTarget::Config(key) => {
                        parse_input(key, &m.text).map(|value| Action::Set { key, value })
                    }
                    _ if text.is_empty() => Err("Must not be empty".to_string()),
                    InputTarget::CustomText(_) => {
                        Ok(Action::App(AppAction::SetCustomText(text.to_string())))
                    }
                    InputTarget::ImportSettings => {
                        Ok(Action::App(AppAction::ImportSettings(m.text.clone())))
                    }
                };
                match done {
                    Ok(action) => Outcome::Run(action),
                    Err(e) => {
                        m.error = Some(e);
                        Outcome::Stay
                    }
                }
            }
            _ => Outcome::Stay,
        }
    }
}
```

`crates/fasttype-tui/src/view/notify.rs` (version complète) :
```rust
//! Notifications en pile en haut à droite (`states/notifications.ts`) : 3 s,
//! et les erreurs restent jusqu'à « Clear all notifications ». Pendant la
//! frappe, seules les erreurs restent visibles.

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
            Level::Error => f64::INFINITY,
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

    /// « Clear all notifications ».
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn expire(&mut self, now: f64) {
        self.items.retain(|n| n.until > now);
    }

    pub fn next_expiry(&self) -> Option<f64> {
        self.items
            .iter()
            .map(|n| n.until)
            .filter(|t| t.is_finite())
            .min_by(f64::total_cmp)
    }

    pub fn items(&self) -> &[Notification] {
        &self.items
    }

    /// Dessine la pile ; `typing` : seules les erreurs restent visibles.
    pub fn render(&self, buf: &mut Buffer, area: Rect, palette: &Palette, typing: bool) {
        let shown = self
            .items
            .iter()
            .filter(|n| !typing || n.level == Level::Error);
        for (i, n) in shown.enumerate() {
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

- [ ] **Step 4 : implémenter l'application et la boucle**

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
use crate::palette::{Action, AppAction, Command, Subgroup, lists, lists::Context};
use crate::perf::Perf;
use crate::session_factory::DEFAULT_CUSTOM_TEXT;
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
use fasttype_core::quote::QuoteFile;
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
use toml::Value;
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

/// Nom du texte custom en cours dans `custom_texts/`.
pub const CURRENT_CUSTOM_TEXT: &str = "current";

/// Citations d'une langue dans la palette : le début du texte, la source en alias.
fn quote_commands(file: &QuoteFile) -> Vec<Command> {
    file.quotes
        .iter()
        .map(|q| {
            let mut text: String = q.text.chars().take(64).collect();
            if q.text.chars().count() > 64 {
                text.push('…');
            }
            Command::new(text, Action::App(AppAction::SelectQuote(q.id))).alias(&q.source)
        })
        .collect()
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
    /// Langue décompressée dans un thread ; le test suivant démarre à la fin.
    loading: Option<(String, std::thread::JoinHandle<()>)>,
    /// Réglages exportés, à copier dans le presse-papiers par la boucle.
    clipboard: Option<String>,
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
        factory.custom_text = store.custom_texts.load(CURRENT_CUSTOM_TEXT).ok();
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
            loading: None,
            clipboard: None,
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
        let custom_text = self
            .factory
            .custom_text
            .as_deref()
            .unwrap_or(DEFAULT_CUSTOM_TEXT);
        let ctx = Context {
            config: &self.store.config,
            custom_text,
            on_result: matches!(self.screen, Screen::Result(_)),
            can_bail_out: running && can_bail_out(spec),
            languages: &languages,
            themes: &themes,
        };
        self.command_line = Some(PaletteState::open(lists::root(&ctx)));
        // la palette prend le focus : un Tab d'avant ne relance plus avec Entrée
        self.restart_armed = None;
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
            Outcome::Run(Action::App(AppAction::SearchQuotes)) => {
                // liste construite à la demande : des milliers de citations
                let language = self.store.config.str("language").to_string();
                let list = self.factory.quotes(&language).map(|f| quote_commands(&f));
                match (list, &mut self.command_line) {
                    (Some(list), Some(p)) if !list.is_empty() => p.push(Subgroup {
                        title: "Search for quotes".into(),
                        list,
                    }),
                    _ => {
                        self.notifications.push(
                            format!("no quotes for {language}"),
                            Level::Notice,
                            now,
                        );
                        self.close_palette();
                    }
                }
            }
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

    fn save_settings(&mut self, now: f64) {
        if let Err(e) = self.store.save_config() {
            self.notifications.push(
                format!("could not save the settings: {e}"),
                Level::Error,
                now,
            );
        }
    }

    /// Relance un test, après avoir décompressé la langue dans un thread si
    /// elle n'est pas encore en mémoire (« loading... » sur le badge).
    fn restart_when_ready(&mut self, now: f64) {
        let language = self.store.config.str("language").to_string();
        if fasttype_data::language_names().is_ok_and(|n| n.contains(&language.as_str()))
            && !self.factory.language_ready(&language)
        {
            let handle = self.factory.preload(&language);
            self.loading = Some((language, handle));
            return;
        }
        self.try_restart(now, true);
    }

    /// Réglages à copier dans le presse-papiers (OSC 52), une seule fois.
    pub fn take_clipboard(&mut self) -> Option<String> {
        self.clipboard.take()
    }

    /// Langue en cours de chargement.
    pub fn loading(&self) -> Option<&str> {
        self.loading.as_ref().map(|(l, _)| l.as_str())
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
                    // comme `overrideConfig` du site, même si la valeur ne change pas
                    if key == "theme" {
                        let _ = self.store.config.set("customTheme", Value::Boolean(false));
                    }
                    self.save_settings(now);
                    // `afterExec: restart` : même si la valeur ne change pas
                    if RESTART_KEYS.contains(&key)
                        || changed.iter().any(|k| RESTART_KEYS.contains(k))
                    {
                        self.restart_when_ready(now);
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
            Action::App(AppAction::SearchQuotes) => {}
            Action::App(AppAction::SelectQuote(id)) => {
                // `quoteLength = [-2]` : la citation choisie, à chaque restart
                self.factory.selected_quote = Some(id);
                let lengths = Value::Array(vec![Value::Integer(-2)]);
                if self.store.config.set("quoteLength", lengths).is_ok() {
                    self.save_settings(now);
                }
                self.try_restart(now, true);
            }
            Action::App(AppAction::SetCustomText(text)) => {
                if let Err(e) = self.store.custom_texts.save(CURRENT_CUSTOM_TEXT, &text) {
                    self.notifications.push(
                        format!("could not save the custom text: {e}"),
                        Level::Error,
                        now,
                    );
                }
                self.factory.custom_text = Some(text);
                if self
                    .store
                    .config
                    .set("mode", Value::String("custom".into()))
                    .is_ok()
                {
                    self.save_settings(now);
                }
                self.try_restart(now, true);
            }
            Action::App(AppAction::ExportSettings) => {
                self.clipboard = Some(self.store.config.to_toml());
                self.notifications
                    .push("Settings copied to clipboard", Level::Notice, now);
            }
            Action::App(AppAction::ImportSettings(text)) => {
                let (config, warnings) = Config::from_toml(&text);
                for w in warnings {
                    self.notifications.push(w.to_string(), Level::Error, now);
                }
                self.store.config = config;
                self.save_settings(now);
                self.notifications
                    .push("Settings imported", Level::Notice, now);
                self.restart_when_ready(now);
            }
            Action::Open(_) | Action::Input(_) | Action::Close => {}
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
        if self.loading.as_ref().is_some_and(|(_, h)| h.is_finished()) {
            self.loading = None;
            self.try_restart(now, true);
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
        // une langue se charge : on regarde à chaque image si c'est fini
        self.loading.is_some()
            || self.transition.is_some()
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
        let typing = matches!(self.screen, Screen::Test)
            && self.session.state() == SessionState::Running
            && self.command_line.is_none();
        self.notifications.render(buf, area, &self.palette, typing);
        if let Some(p) = &self.command_line {
            // pendant l'aperçu d'un thème, pas de voile : ses vraies couleurs
            let veil = self.saved_palette.is_none();
            cursor = Some(palette_view::render(buf, area, p, &self.palette, veil));
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
                if veil {
                    st.bg = palette_view::dim_color(st.bg);
                    for c in &mut st.cells {
                        c.style = palette_view::dim_style(c.style);
                    }
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
            let name = match &self.loading {
                Some(_) => "loading...".to_string(),
                None => self.session.spec().language.replace('_', " "),
            };
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
use crate::sized::ScaledText;
use crate::terminal::{
    FrameWriter, TerminalGuard, graphics_supported, mark_kitty_images, queue_caret_look,
    text_sizing_supported,
};
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
    /// Mots agrandis déjà à l'écran : rien à réécrire s'ils n'ont pas changé.
    scaled: Option<ScaledText>,
}

impl Output {
    /// Dessine une image et l'envoie en une écriture, encadrée par la sortie
    /// synchronisée (mode 2026) : le terminal ne montre jamais d'image partielle.
    fn present(&mut self, app: &mut App, perf: &Perf) -> io::Result<()> {
        let size = self.term.size()?;
        if size != self.size {
            self.size = size;
            // ratatui efface l'écran : les mots agrandis sont à réécrire
            self.scaled = None;
            // la taille des cases a pu changer (zoom) : images à refaire
            if let Some(k) = &mut self.kitty {
                match cell_px() {
                    Some(cell) => {
                        self.term.backend_mut().write_all(DELETE_ALL)?;
                        *k = KittyCaret::new(cell);
                    }
                    // l'écran va être effacé : replacer l'image au prochain dessin
                    None => k.invalidate(),
                }
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
        let scaled = app.scaled_text();
        if scaled != self.scaled.as_ref() {
            if let Some(t) = scaled {
                // seules les lettres qui ont changé : quelques dizaines d'octets par frappe
                t.write_changes(self.scaled.as_ref(), self.term.backend_mut())?;
            }
            self.scaled = scaled.cloned();
        }
        if let Some(k) = &mut self.kitty {
            k.draw(
                self.term.backend_mut(),
                app.caret_frame(),
                app.palette().caret_rgb,
            )?;
        }
        if let Some(text) = app.take_clipboard() {
            // OSC 52 : le terminal copie le texte dans le presse-papiers
            write!(
                self.term.backend_mut(),
                "\x1b]52;c;{}\x07",
                crate::kitty::base64(text.as_bytes())
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
    let renderer =
        CaretRenderer::detect(|k| std::env::var(k).ok(), cell_px(), graphics_supported());
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
        scaled: None,
    };
    let mut app = App::new(store, color_mode, clock.now_ms(), seed());
    app.set_caret_renderer(renderer);
    app.set_text_sizing(text_sizing_supported());
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

- [ ] **Step 5 : lancer les tests**

Run: `cargo fmt && cargo test -p fasttype-tui && cargo clippy --workspace --all-targets -- -D warnings`
Expected: tous PASS (12 dans `palette`, 5 dans `commands`).

- [ ] **Step 6 : test pty et benchmark**

`scripts/pty_smoke.py` (version complète) :
```python
"""Lance fasttype dans un pseudo-terminal, tape un test custom complet, quitte.
Usage : python3 -I scripts/pty_smoke.py <binaire> <dossier HOME temporaire> [--perf] [--kitty] [--sized] [--silent] [--palette] [--export] [--sigterm]
  --kitty    le terminal répond OK à la sonde graphique Kitty (cases de 10 × 20 pixels)
  --sized    le terminal agrandit le texte (OSC 66, Kitty ≥ 0.40)
  --silent   le terminal ne répond à aucune sonde (ni position, ni DA1) : chaque
             sonde doit abandonner au bout de 300 ms
  --palette  quitte par la palette de commandes (Échap, « quit », Entrée)
  --export   exporte les réglages depuis la palette (OSC 52), puis Ctrl+C
  --sigterm  quitte par SIGTERM au lieu de Ctrl+C"""
import fcntl, os, pty, re, select, signal, struct, sys, termios, time

binary, home = sys.argv[1], sys.argv[2]
flags = sys.argv[3:]
perf, kitty, sigterm = "--perf" in flags, "--kitty" in flags, "--sigterm" in flags
sized, silent = "--sized" in flags, "--silent" in flags
palette, export = "--palette" in flags, "--export" in flags
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
if export:
    os.write(fd, b"\x1b")
    pump(0.3)
    for ch in "export":
        os.write(fd, ch.encode())
        pump(0.05)
    os.write(fd, b"\r")
    pump(0.3)
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
if export:
    import base64
    found = re.search(rb"\x1b\]52;c;([A-Za-z0-9+/=]+)\x07", bytes(out))
    copied = base64.b64decode(found.group(1)).decode() if found else ""
    print("settings copied (OSC 52):", 'mode = "custom"' in copied)
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
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --export
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --palette
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --perf
rm -rf target/pty-home && mkdir -p target/pty-home && python3 -I scripts/pty_smoke.py target/release/fasttype target/pty-home --kitty --sized --perf
```
Expected : pour chacun, `exit 0` et tous les contrôles à `True`. En plus :
- avec `--export` : `settings copied (OSC 52): True` ;
- p50 sous 0,5 ms.

Run: `cargo bench -p fasttype-tui --bench frame 2>&1 | grep -A1 -E "^(frame|key)"`
Expected : les trois benchmarks sous 1 ms.

- [ ] **Step 7 : vérification finale et essai à la main**

Run: `cargo fmt --check && cargo test --workspace 2>&1 | grep "test result" | awk '{s+=$4; f+=$6} END {print "passed", s, "failed", f}' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `failed 0`, aucun avertissement.

Essai à la main :
- `esc`, « search quotes », taper quelques mots d'une citation, Entrée : la citation doit être lancée ;
- `esc`, « export », puis coller dans un éditeur : le TOML doit y être ;
- `esc`, « language », « french 600k » : « loading... » doit apparaître sur le badge, puis le test démarre.

Le noter dans le ledger, et signaler à l'utilisateur que l'essai visuel lui revient.

- [ ] **Step 8 : commit**

```bash
git add crates/fasttype-tui scripts
git commit -m "feat(tui): texte custom, recherche de citations, export/import des réglages, erreurs persistantes"
```
