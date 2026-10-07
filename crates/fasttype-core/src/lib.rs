//! Moteur de frappe de fasttype : génération des mots, session, journal
//! d'événements et statistiques. Logique pure, sans aucune E/S.

pub mod chars;
pub mod clock;
pub mod event;
pub mod generator;
pub mod numbers;
pub mod punctuation;
pub mod quote;
pub mod rng;
pub mod sources;
pub mod spec;
pub mod stats;
