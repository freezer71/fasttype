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
    Int {
        min: i64,
    },
    /// Nombre (entier ou flottant) dans `[min, max]`.
    Number {
        min: f64,
        max: Option<f64>,
    },
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
    NameList {
        min: usize,
        max: Option<usize>,
    },
    /// Liste de longueurs de citation : -3 favoris, -2 recherche, 0 à 3.
    QuoteLengths,
    /// Exactement n couleurs `#rgb` ou `#rrggbb`.
    Colors(usize),
    /// Exactement `len` valeurs prises dans `values`.
    ChoiceList {
        values: &'static [&'static str],
        len: usize,
    },
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
        parse_literal(self.default)
            .unwrap_or_else(|| panic!("défaut invalide pour {} : {}", self.name, self.default))
    }

    pub fn toml_key(&self) -> String {
        to_snake(self.name)
    }
}

/// Lit un littéral TOML isolé (`"medium"`, `[1]`, `0.5`…).
pub(crate) fn parse_literal(src: &str) -> Option<Value> {
    toml::from_str::<toml::Table>(&format!("v = {src}"))
        .ok()?
        .remove("v")
}

pub const SCHEMA: &[KeyDef] = &[
    KeyDef {
        name: "punctuation",
        group: Group::Test,
        display: "punctuation",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "numbers",
        group: Group::Test,
        display: "numbers",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "words",
        group: Group::Test,
        display: "word count",
        kind: Kind::Int { min: 0 },
        default: "50",
    },
    KeyDef {
        name: "time",
        group: Group::Test,
        display: "time",
        kind: Kind::Int { min: 0 },
        default: "30",
    },
    KeyDef {
        name: "mode",
        group: Group::Test,
        display: "mode",
        kind: Kind::Choice(&["time", "words", "quote", "custom", "zen"]),
        default: "\"time\"",
    },
    KeyDef {
        name: "quoteLength",
        group: Group::Test,
        display: "quote length",
        kind: Kind::QuoteLengths,
        default: "[1]",
    },
    KeyDef {
        name: "language",
        group: Group::Test,
        display: "language",
        kind: Kind::Name,
        default: "\"english\"",
    },
    KeyDef {
        name: "burstHeatmap",
        group: Group::Test,
        display: "word burst heatmap",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "difficulty",
        group: Group::Behavior,
        display: "difficulty",
        kind: Kind::Choice(&["normal", "expert", "master"]),
        default: "\"normal\"",
    },
    KeyDef {
        name: "quickRestart",
        group: Group::Behavior,
        display: "quick restart",
        kind: Kind::Choice(&["off", "esc", "tab", "enter"]),
        // écart voulu au site (« off » : tab puis enter) : Tab seul relance
        default: "\"tab\"",
    },
    KeyDef {
        name: "repeatQuotes",
        group: Group::Behavior,
        display: "repeat quotes",
        kind: Kind::Choice(&["off", "typing"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "resultSaving",
        group: Group::Behavior,
        display: "result saving",
        kind: Kind::Bool,
        default: "true",
    },
    KeyDef {
        name: "blindMode",
        group: Group::Behavior,
        display: "blind mode",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "alwaysShowWordsHistory",
        group: Group::Behavior,
        display: "always show words history",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "singleListCommandLine",
        group: Group::Behavior,
        display: "single list command line",
        kind: Kind::Choice(&["manual", "on"]),
        default: "\"on\"",
    },
    KeyDef {
        name: "minWpm",
        group: Group::Behavior,
        display: "min speed",
        kind: Kind::Choice(&["off", "custom"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "minWpmCustomSpeed",
        group: Group::Behavior,
        display: "min speed custom",
        kind: Kind::Number {
            min: 0.0,
            max: None,
        },
        default: "100",
    },
    KeyDef {
        name: "minAcc",
        group: Group::Behavior,
        display: "min accuracy",
        kind: Kind::Choice(&["off", "custom"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "minAccCustom",
        group: Group::Behavior,
        display: "min accuracy custom",
        kind: Kind::Number {
            min: 0.0,
            max: Some(100.0),
        },
        default: "90",
    },
    KeyDef {
        name: "minBurst",
        group: Group::Behavior,
        display: "min word burst",
        kind: Kind::Choice(&["off", "fixed", "flex"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "minBurstCustomSpeed",
        group: Group::Behavior,
        display: "min word burst custom speed",
        kind: Kind::Number {
            min: 0.0,
            max: None,
        },
        default: "100",
    },
    KeyDef {
        name: "britishEnglish",
        group: Group::Behavior,
        display: "british english",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "funbox",
        group: Group::Behavior,
        display: "funbox",
        kind: Kind::NameList {
            min: 0,
            max: Some(15),
        },
        default: "[]",
    },
    KeyDef {
        name: "customLayoutfluid",
        group: Group::Behavior,
        display: "custom layoutfluid",
        kind: Kind::NameList {
            min: 2,
            max: Some(15),
        },
        default: "[\"qwerty\", \"dvorak\", \"colemak\"]",
    },
    KeyDef {
        name: "customPolyglot",
        group: Group::Behavior,
        display: "polyglot languages",
        kind: Kind::NameList { min: 2, max: None },
        default: "[\"english\", \"spanish\", \"french\", \"german\"]",
    },
    KeyDef {
        name: "freedomMode",
        group: Group::Input,
        display: "freedom mode",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "strictSpace",
        group: Group::Input,
        display: "strict space",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "oppositeShiftMode",
        group: Group::Input,
        display: "opposite shift mode",
        kind: Kind::Choice(&["off", "on", "keymap"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "stopOnError",
        group: Group::Input,
        display: "stop on error",
        kind: Kind::Choice(&["off", "word", "letter"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "deleteOnError",
        group: Group::Input,
        display: "delete on error",
        kind: Kind::Choice(&["off", "letter", "letter_hard", "word", "word_hard"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "confidenceMode",
        group: Group::Input,
        display: "confidence mode",
        kind: Kind::Choice(&["off", "on", "max"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "quickEnd",
        group: Group::Input,
        display: "quick end",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "indicateTypos",
        group: Group::Input,
        display: "indicate typos",
        kind: Kind::Choice(&["off", "below", "replace", "both"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "compositionDisplay",
        group: Group::Input,
        display: "composition display",
        kind: Kind::Choice(&["off", "below", "replace"]),
        default: "\"replace\"",
    },
    KeyDef {
        name: "hideExtraLetters",
        group: Group::Input,
        display: "hide extra letters",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "lazyMode",
        group: Group::Input,
        display: "lazy mode",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "layout",
        group: Group::Input,
        display: "layout",
        kind: Kind::Name,
        default: "\"default\"",
    },
    KeyDef {
        name: "codeUnindentOnBackspace",
        group: Group::Input,
        display: "code unindent on backspace",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "soundVolume",
        group: Group::Sound,
        display: "sound volume",
        kind: Kind::Number {
            min: 0.0,
            max: Some(1.0),
        },
        default: "0.5",
    },
    KeyDef {
        name: "playSoundOnClick",
        group: Group::Sound,
        display: "play sound on click",
        kind: Kind::Choice(&[
            "off", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
            "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26",
        ]),
        default: "\"off\"",
    },
    KeyDef {
        name: "playSoundOnError",
        group: Group::Sound,
        display: "play sound on error",
        kind: Kind::Choice(&["off", "1", "2", "3", "4"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "playTimeWarning",
        group: Group::Sound,
        display: "play time warning",
        kind: Kind::Choice(&["off", "1", "3", "5", "10"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "smoothCaret",
        group: Group::Caret,
        display: "smooth caret",
        kind: Kind::Choice(&["off", "slow", "medium", "fast"]),
        default: "\"medium\"",
    },
    KeyDef {
        name: "caretStyle",
        group: Group::Caret,
        display: "caret style",
        kind: Kind::Choice(&[
            "off",
            "default",
            "block",
            "outline",
            "underline",
            "carrot",
            "banana",
            "monkey",
        ]),
        default: "\"default\"",
    },
    KeyDef {
        name: "paceCaret",
        group: Group::Caret,
        display: "pace caret",
        kind: Kind::Choice(&["off", "average", "pb", "tagPb", "last", "custom", "daily"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "paceCaretCustomSpeed",
        group: Group::Caret,
        display: "pace caret custom speed",
        kind: Kind::Number {
            min: 0.0,
            max: None,
        },
        default: "100",
    },
    KeyDef {
        name: "paceCaretStyle",
        group: Group::Caret,
        display: "pace caret style",
        kind: Kind::Choice(&[
            "off",
            "default",
            "block",
            "outline",
            "underline",
            "carrot",
            "banana",
            "monkey",
        ]),
        default: "\"default\"",
    },
    KeyDef {
        name: "repeatedPace",
        group: Group::Caret,
        display: "repeated pace",
        kind: Kind::Bool,
        default: "true",
    },
    KeyDef {
        name: "timerStyle",
        group: Group::Appearance,
        display: "live progress style",
        kind: Kind::Choice(&["off", "bar", "text", "mini", "flash_text", "flash_mini"]),
        default: "\"mini\"",
    },
    KeyDef {
        name: "liveSpeedStyle",
        group: Group::Appearance,
        display: "live speed style",
        kind: Kind::Choice(&["off", "text", "mini"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "liveAccStyle",
        group: Group::Appearance,
        display: "live accuracy style",
        kind: Kind::Choice(&["off", "text", "mini"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "liveBurstStyle",
        group: Group::Appearance,
        display: "live word burst style",
        kind: Kind::Choice(&["off", "text", "mini"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "timerColor",
        group: Group::Appearance,
        display: "timer color",
        kind: Kind::Choice(&["black", "sub", "text", "main"]),
        default: "\"main\"",
    },
    KeyDef {
        name: "timerOpacity",
        group: Group::Appearance,
        display: "timer opacity",
        kind: Kind::Choice(&["0.25", "0.5", "0.75", "1"]),
        default: "\"1\"",
    },
    KeyDef {
        name: "highlightMode",
        group: Group::Appearance,
        display: "highlight mode",
        kind: Kind::Choice(&[
            "off",
            "letter",
            "word",
            "next_word",
            "next_two_words",
            "next_three_words",
        ]),
        default: "\"letter\"",
    },
    KeyDef {
        name: "typedEffect",
        group: Group::Appearance,
        display: "typed effect",
        kind: Kind::Choice(&["keep", "hide", "fade", "dots"]),
        default: "\"keep\"",
    },
    KeyDef {
        name: "tapeMode",
        group: Group::Appearance,
        display: "tape mode",
        kind: Kind::Choice(&["off", "letter", "word"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "tapeMargin",
        group: Group::Appearance,
        display: "tape margin",
        kind: Kind::Number {
            min: 10.0,
            max: Some(90.0),
        },
        default: "50",
    },
    KeyDef {
        name: "smoothLineScroll",
        group: Group::Appearance,
        display: "smooth line scroll",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "showAllLines",
        group: Group::Appearance,
        display: "show all lines",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "alwaysShowDecimalPlaces",
        group: Group::Appearance,
        display: "always show decimal places",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "typingSpeedUnit",
        group: Group::Appearance,
        display: "typing speed unit",
        kind: Kind::Choice(&["wpm", "cpm", "wps", "cps", "wph"]),
        default: "\"wpm\"",
    },
    KeyDef {
        name: "startGraphsAtZero",
        group: Group::Appearance,
        display: "start graphs at zero",
        kind: Kind::Bool,
        default: "true",
    },
    KeyDef {
        name: "maxLineWidth",
        group: Group::Appearance,
        display: "max line width",
        kind: Kind::MaxLineWidth,
        default: "0",
    },
    KeyDef {
        name: "fontSize",
        group: Group::Appearance,
        display: "font size",
        kind: Kind::Positive,
        default: "2",
    },
    KeyDef {
        name: "fontFamily",
        group: Group::Appearance,
        display: "font family",
        kind: Kind::Name,
        default: "\"Roboto_Mono\"",
    },
    KeyDef {
        name: "keymapMode",
        group: Group::Appearance,
        display: "keymap mode",
        kind: Kind::Choice(&["off", "static", "react", "next"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "keymapLayout",
        group: Group::Appearance,
        display: "keymap layout",
        kind: Kind::Name,
        default: "\"overrideSync\"",
    },
    KeyDef {
        name: "keymapStyle",
        group: Group::Appearance,
        display: "keymap style",
        kind: Kind::Choice(&[
            "staggered",
            "alice",
            "matrix",
            "split",
            "split_matrix",
            "steno",
            "steno_matrix",
        ]),
        default: "\"staggered\"",
    },
    KeyDef {
        name: "keymapLegendStyle",
        group: Group::Appearance,
        display: "keymap legend style",
        kind: Kind::Choice(&["lowercase", "uppercase", "blank", "dynamic"]),
        default: "\"lowercase\"",
    },
    KeyDef {
        name: "keymapKeys",
        group: Group::Appearance,
        display: "keymap keys",
        kind: Kind::Choice(&["minimal", "minimal_numrow", "full"]),
        default: "\"minimal\"",
    },
    KeyDef {
        name: "keymapSize",
        group: Group::Appearance,
        display: "keymap size",
        kind: Kind::Number {
            min: 0.5,
            max: Some(3.5),
        },
        default: "1",
    },
    KeyDef {
        name: "flipTestColors",
        group: Group::Theme,
        display: "flip test colors",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "colorfulMode",
        group: Group::Theme,
        display: "colorful mode",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "customBackground",
        group: Group::Theme,
        display: "custom background",
        kind: Kind::BackgroundUrl,
        default: "\"\"",
    },
    KeyDef {
        name: "customBackgroundSize",
        group: Group::Theme,
        display: "custom background size",
        kind: Kind::Choice(&["cover", "contain", "max"]),
        default: "\"cover\"",
    },
    KeyDef {
        name: "customBackgroundFilter",
        group: Group::Theme,
        display: "custom background filter",
        kind: Kind::Numbers(4),
        default: "[0, 1, 1, 1]",
    },
    KeyDef {
        name: "autoSwitchTheme",
        group: Group::Theme,
        display: "auto switch theme",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "themeLight",
        group: Group::Theme,
        display: "theme light",
        kind: Kind::Name,
        default: "\"serika\"",
    },
    KeyDef {
        name: "themeDark",
        group: Group::Theme,
        display: "theme dark",
        kind: Kind::Name,
        default: "\"serika_dark\"",
    },
    KeyDef {
        name: "randomTheme",
        group: Group::Theme,
        display: "random theme",
        kind: Kind::Choice(&["off", "on", "fav", "light", "dark", "custom", "auto"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "favThemes",
        group: Group::Theme,
        display: "favorite themes",
        kind: Kind::NameList { min: 0, max: None },
        default: "[]",
    },
    KeyDef {
        name: "theme",
        group: Group::Theme,
        display: "theme",
        kind: Kind::Name,
        default: "\"serika_dark\"",
    },
    KeyDef {
        name: "customTheme",
        group: Group::Theme,
        display: "custom theme",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "customThemeColors",
        group: Group::Theme,
        display: "custom theme colors",
        kind: Kind::Colors(10),
        default: "[\"#323437\", \"#e2b714\", \"#e2b714\", \"#646669\", \"#2c2e31\", \"#d1d0c5\", \"#ca4754\", \"#7e2a33\", \"#ca4754\", \"#7e2a33\"]",
    },
    KeyDef {
        name: "showKeyTips",
        group: Group::HideElements,
        display: "show key tips",
        kind: Kind::Bool,
        default: "true",
    },
    KeyDef {
        name: "showOutOfFocusWarning",
        group: Group::HideElements,
        display: "show out of focus warning",
        kind: Kind::Bool,
        default: "true",
    },
    KeyDef {
        name: "capsLockWarning",
        group: Group::HideElements,
        display: "caps lock warning",
        kind: Kind::Bool,
        default: "true",
    },
    KeyDef {
        name: "showAverage",
        group: Group::HideElements,
        display: "show average",
        kind: Kind::Choice(&["off", "speed", "acc", "both"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "showPb",
        group: Group::HideElements,
        display: "show personal best",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "accountChart",
        group: Group::Hidden,
        display: "account chart",
        kind: Kind::ChoiceList {
            values: &["on", "off"],
            len: 4,
        },
        default: "[\"on\", \"on\", \"on\", \"on\"]",
    },
    KeyDef {
        name: "monkey",
        group: Group::Hidden,
        display: "monkey",
        kind: Kind::Bool,
        default: "false",
    },
    KeyDef {
        name: "monkeyPowerLevel",
        group: Group::Hidden,
        display: "monkey power level",
        kind: Kind::Choice(&["off", "1", "2", "3", "4"]),
        default: "\"off\"",
    },
    KeyDef {
        name: "ads",
        group: Group::Ads,
        display: "ads",
        kind: Kind::Choice(&["off", "result", "on", "sellout"]),
        default: "\"result\"",
    },
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
    v.as_float()
        .or_else(|| v.as_integer().map(|i| i as f64))
        .filter(|n| n.is_finite())
}

fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#')
        .is_some_and(|h| (h.len() == 3 || h.len() == 6) && h.chars().all(|c| c.is_ascii_hexdigit()))
}

fn is_background_url(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && !s.contains(['`', '\'', '"'])
        && s.chars().count() <= 2048
        && [".png", ".gif", ".jpeg", ".jpg", ".webp"]
            .iter()
            .any(|ext| lower.find(ext).is_some_and(|i| i > 0))
}

/// Vérifie une valeur contre son type ; le message dit ce qui est attendu.
pub fn validate(kind: &Kind, v: &Value) -> Result<(), String> {
    let fail = |what: String| Err(what);
    match *kind {
        Kind::Bool => v
            .as_bool()
            .map(|_| ())
            .ok_or_else(|| "true ou false attendu".into()),
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
            _ => {
                fail("\"\" ou une URL http(s) d'image png, gif, jpeg, jpg ou webp attendue".into())
            }
        },
        Kind::Numbers(n) => match v.as_array() {
            Some(a) if a.len() == n && a.iter().all(|x| number(x).is_some()) => Ok(()),
            _ => fail(format!("{n} nombres attendus")),
        },
        Kind::NameList { min, max } => match v.as_array() {
            Some(a)
                if a.len() >= min
                    && max.is_none_or(|m| a.len() <= m)
                    && a.iter()
                        .all(|x| x.as_str().is_some_and(|s| !s.trim().is_empty())) =>
            {
                Ok(())
            }
            _ => fail(match max {
                Some(m) => format!("une liste de {min} à {m} noms attendue"),
                None => format!("une liste d'au moins {min} noms attendue"),
            }),
        },
        Kind::QuoteLengths => match v.as_array() {
            Some(a)
                if a.iter().all(|x| {
                    x.as_integer()
                        .is_some_and(|n| [-3, -2, 0, 1, 2, 3].contains(&n))
                }) =>
            {
                Ok(())
            }
            _ => fail("une liste de valeurs parmi -3, -2, 0, 1, 2, 3 attendue".into()),
        },
        Kind::Colors(n) => match v.as_array() {
            Some(a) if a.len() == n && a.iter().all(|x| x.as_str().is_some_and(is_hex_color)) => {
                Ok(())
            }
            _ => fail(format!("{n} couleurs #rgb ou #rrggbb attendues")),
        },
        Kind::ChoiceList { values, len } => match v.as_array() {
            Some(a)
                if a.len() == len
                    && a.iter()
                        .all(|x| x.as_str().is_some_and(|s| values.contains(&s))) =>
            {
                Ok(())
            }
            _ => fail(format!(
                "{len} valeurs parmi {} attendues",
                values.join(", ")
            )),
        },
    }
}
