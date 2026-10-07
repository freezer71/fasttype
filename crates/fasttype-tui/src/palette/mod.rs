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
