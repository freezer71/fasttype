//! Horloge injectable : réelle dans l'application, manuelle dans les tests.

use std::cell::Cell;
use std::time::Instant;

pub trait Clock {
    /// Millisecondes écoulées depuis une origine fixe (monotone).
    fn now_ms(&self) -> f64;
}

pub struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> f64 {
        self.origin.elapsed().as_secs_f64() * 1000.0
    }
}

pub struct ManualClock {
    ms: Cell<f64>,
}

impl ManualClock {
    pub fn new(ms: f64) -> Self {
        Self { ms: Cell::new(ms) }
    }

    pub fn set(&self, ms: f64) {
        self.ms.set(ms);
    }

    pub fn advance(&self, ms: f64) {
        self.ms.set(self.ms.get() + ms);
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> f64 {
        self.ms.get()
    }
}
