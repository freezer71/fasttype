//! Persistance locale de fasttype : config, historique, records personnels,
//! textes custom et citations favorites.

pub mod config;
pub mod fs;
pub mod paths;
pub mod results;
pub mod schema;

pub use config::{Config, ConfigError, ConfigWarning};
