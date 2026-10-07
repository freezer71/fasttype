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
            3 => Some(Self {
                r: digit(0)?,
                g: digit(1)?,
                b: digit(2)?,
                a: 255,
            }),
            4 => Some(Self {
                r: digit(0)?,
                g: digit(1)?,
                b: digit(2)?,
                a: digit(3)?,
            }),
            6 => Some(Self {
                r: byte(0)?,
                g: byte(2)?,
                b: byte(4)?,
                a: 255,
            }),
            8 => Some(Self {
                r: byte(0)?,
                g: byte(2)?,
                b: byte(4)?,
                a: byte(6)?,
            }),
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
        Rgba {
            r: mix(self.r, bg.r),
            g: mix(self.g, bg.g),
            b: mix(self.b, bg.b),
            a: 255,
        }
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
        Rgba::parse_hex(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("couleur invalide : {s}")))
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
const COLORS: [&str; 10] = [
    "bg",
    "main",
    "caret",
    "sub",
    "subAlt",
    "text",
    "error",
    "errorExtra",
    "colorfulError",
    "colorfulErrorExtra",
];

fn err(line: usize, message: impl Into<String>) -> ThemeParseError {
    ThemeParseError {
        line,
        message: message.into(),
    }
}

fn build(
    name: String,
    mut fields: BTreeMap<String, String>,
    line: usize,
) -> Result<Theme, ThemeParseError> {
    let mut color = |key: &str| -> Result<Rgba, ThemeParseError> {
        let v = fields
            .remove(key)
            .ok_or_else(|| err(line, format!("{name} : couleur `{key}` manquante")))?;
        Rgba::parse_hex(&v)
            .ok_or_else(|| err(line, format!("{name} : couleur `{key}` invalide ({v})")))
    };
    let [
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
    ] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(|i| color(COLORS[i]));
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
    let start = src
        .find(START)
        .ok_or_else(|| err(0, "objet `themes` introuvable"))?;
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
            let key = line
                .strip_suffix(": {")
                .ok_or_else(|| err(line_no, format!("entrée de thème attendue : {line}")))?;
            let key = key.trim().trim_matches('"');
            current = Some((key.to_string(), BTreeMap::new(), line_no));
            continue;
        };
        if line == "}," || line == "}" {
            themes.push(build(name, fields, start_line)?);
            continue;
        }
        let body = line.strip_suffix(',').unwrap_or(line);
        let (key, value) = body
            .split_once(':')
            .ok_or_else(|| err(line_no, format!("champ attendu : {line}")))?;
        let value = value.trim();
        let value = match value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            Some(s) => s.to_string(),
            None if value == "true" || value == "false" => value.to_string(),
            None => return Err(err(line_no, format!("valeur inattendue : {value}"))),
        };
        if fields.insert(key.trim().to_string(), value).is_some() {
            return Err(err(
                line_no,
                format!("{name} : champ `{}` en double", key.trim()),
            ));
        }
        current = Some((name, fields, start_line));
    }
    Err(err(first_line, "fin de l'objet `themes` introuvable"))
}
