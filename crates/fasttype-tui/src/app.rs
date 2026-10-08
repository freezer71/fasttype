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
use fasttype_store::{Config, ConfigWarning, RecordOutcome, Store};
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

/// Citation choisie par « Search for quotes » : « langue id », dans le dossier de données.
pub const SELECTED_QUOTE_FILE: &str = "selected_quote.txt";

/// La citation choisie au dernier lancement, si elle vaut pour la langue en cours.
fn saved_quote(store: &Store) -> Option<u32> {
    let text = std::fs::read_to_string(store.paths.data_dir.join(SELECTED_QUOTE_FILE)).ok()?;
    let (language, id) = text.trim().split_once(' ')?;
    (language == store.config.str("language")).then(|| id.parse().ok())?
}

/// Citations d'une langue dans la palette : le début du texte, la source en alias.
fn quote_commands(file: &QuoteFile) -> Vec<Command> {
    file.quotes
        .iter()
        .map(|q| {
            let mut text: String = q.text.chars().take(64).collect();
            if q.text.chars().count() > 64 {
                text.push('…');
            }
            // la recherche porte sur toute la citation et sa source
            let alias = format!("{} {}", q.source, q.text);
            Command::new(text, Action::App(AppAction::SelectQuote(q.id))).alias(&alias)
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
        factory.selected_quote = saved_quote(&store);
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
                // gardée pour le prochain lancement, comme `selectedQuoteId` du site
                let language = self.store.config.str("language");
                let file = self.store.paths.data_dir.join(SELECTED_QUOTE_FILE);
                if let Err(e) =
                    fasttype_store::fs::write_atomic(&file, format!("{language} {id}").as_bytes())
                {
                    self.notifications.push(
                        format!("could not save the selected quote: {e}"),
                        Level::Error,
                        now,
                    );
                }
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
                // certains terminaux (Terminal.app, tmux par défaut) ignorent OSC 52
                let path = self.store.paths.config_file();
                self.notifications.push(
                    format!(
                        "Settings sent to the clipboard (OSC 52) - also in {}",
                        path.display()
                    ),
                    Level::Notice,
                    now,
                );
            }
            Action::App(AppAction::ImportSettings(text)) => {
                let (config, warnings) = Config::from_toml(&text);
                // texte illisible : on ne touche à rien (`applyConfigFromJson`)
                if let Some(ConfigWarning::Syntax(e)) = warnings
                    .iter()
                    .find(|w| matches!(w, ConfigWarning::Syntax(_)))
                {
                    let first = e.lines().next().unwrap_or_default();
                    self.notifications.push(
                        format!("Failed to import settings: {first}"),
                        Level::Error,
                        now,
                    );
                    return;
                }
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
        // nouveau test tout de suite (il apparaît en fondu) : on peut taper sans
        // attendre, contrairement au site qui ignore les touches 250 ms
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
        self.advance_transitions(at);
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

    /// Fait avancer les fondus jusqu'à `now`.
    fn advance_transitions(&mut self, now: f64) {
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
        let on_test = matches!(self.screen, Screen::Test);
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
                    quick_restart: c.str("quickRestart"),
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
        if style == CaretStyle::Off || s.state() == SessionState::Finished || s.words().is_empty() {
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
