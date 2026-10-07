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
            ConfigWarning::Syntax(e) => write!(
                f,
                "config.toml illisible ({e}) : réglages par défaut utilisés"
            ),
            ConfigWarning::UnknownKey(k) => write!(f, "config.toml : clé inconnue « {k} » ignorée"),
            ConfigWarning::Invalid { key, reason } => {
                write!(
                    f,
                    "config.toml : « {key} » invalide ({reason}), valeur par défaut utilisée"
                )
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
        Self {
            values: SCHEMA.iter().map(|d| d.default_value()).collect(),
        }
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
        self.get(key)
            .and_then(|v| v.as_integer().or_else(|| v.as_float().map(|f| f as i64)))
            .unwrap_or(0)
    }

    pub fn float(&self, key: &str) -> f64 {
        self.get(key)
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
            .unwrap_or(0.0)
    }

    pub fn str_list(&self, key: &str) -> Vec<&str> {
        self.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    pub fn int_list(&self, key: &str) -> Vec<i64> {
        self.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_integer).collect())
            .unwrap_or_default()
    }

    /// Change une clé et applique les effets de bord de Monkeytype. Renvoie les
    /// clés réellement modifiées, dans l'ordre (la clé demandée d'abord).
    pub fn set(&mut self, key: &str, value: Value) -> Result<Vec<&'static str>, ConfigError> {
        let def = key_def(key).ok_or_else(|| ConfigError::UnknownKey(key.to_string()))?;
        validate(&def.kind, &value).map_err(|reason| ConfigError::Invalid {
            key: key.to_string(),
            reason,
        })?;
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
            "freedomMode" if self.bool("freedomMode") => {
                self.force_str("confidenceMode", "off", changed)
            }
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
            "liveSpeedStyle" | "liveAccStyle" if self.str(key) == "text" => {
                self.force("monkey", off, changed)
            }
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
