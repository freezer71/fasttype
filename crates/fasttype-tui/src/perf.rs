//! Mesures de fluidité pour `fasttype --perf` : latence touche → écran et
//! temps de calcul d'une image, sur les 1024 derniers échantillons.

const CAPACITY: usize = 1024;

#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    samples: Vec<f64>,
    next: usize,
}

impl LatencyStats {
    pub fn record(&mut self, ms: f64) {
        if self.samples.len() < CAPACITY {
            self.samples.push(ms);
        } else {
            self.samples[self.next] = ms;
        }
        self.next = (self.next + 1) % CAPACITY;
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }

    /// Percentile `p` (0 à 100), par rang le plus proche.
    pub fn percentile(&self, p: f64) -> Option<f64> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(f64::total_cmp);
        let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
        Some(sorted[rank.min(sorted.len()) - 1])
    }
}

#[derive(Debug, Clone, Default)]
pub struct Perf {
    pub enabled: bool,
    pub input_to_flush: LatencyStats,
    pub frame: LatencyStats,
}

impl Perf {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ..Self::default()
        }
    }

    /// Ligne affichée en surimpression.
    pub fn summary(&self) -> String {
        let fmt = |v: Option<f64>| v.map_or("-".to_string(), |v| format!("{v:.2}"));
        format!(
            "key→flush p50 {} p99 {} ms · frame p99 {} ms · n {}",
            fmt(self.input_to_flush.percentile(50.0)),
            fmt(self.input_to_flush.percentile(99.0)),
            fmt(self.frame.percentile(99.0)),
            self.input_to_flush.count()
        )
    }
}
