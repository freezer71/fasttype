//! Persistance locale de fasttype : config, historique, records personnels,
//! textes custom et citations favorites.

pub mod config;
pub mod fs;
pub mod paths;
pub mod pbs;
pub mod results;
pub mod schema;
mod store;
pub mod texts;

pub use config::{Config, ConfigError, ConfigWarning};
pub use store::{RecordOutcome, Store};
