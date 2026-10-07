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
