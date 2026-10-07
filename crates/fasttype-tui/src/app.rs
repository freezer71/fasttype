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
        // En zen, Entrée insère un saut de ligne et Shift+Entrée termine le test :
        // ni l'une ni l'autre ne relance.
        let zen = self.session.spec().mode == Mode::Zen && matches!(self.screen, Screen::Test);
        let has_newlines = zen || self.session.words().iter().any(|w| w.contains('\n'));
        let quick = self.store.config.str("quickRestart");
        let is_quick = match (quick, key) {
            ("tab", Key::Tab | Key::BackTab) | ("esc", Key::Esc) => true,
            ("enter", Key::Enter) => !has_newlines,
            ("enter", Key::ShiftEnter) => !zen,
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
        if area.is_empty() {
            return;
        }
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
