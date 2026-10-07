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
        return Command::new(title, Action::Input { key }).alias(alias(key));
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
