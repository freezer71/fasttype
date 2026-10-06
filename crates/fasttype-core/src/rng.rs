//! Aléatoire injectable. Les formules reprennent `Math.random()` et les
//! utilitaires de Monkeytype (`randomIntFromRange`, `randomElementFromArray`,
//! `shuffle`).

/// Source de nombres aléatoires uniformes, comme `Math.random()`.
pub trait RandomSource: Send {
    /// Nombre uniforme dans [0, 1).
    fn next_f64(&mut self) -> f64;

    /// `Math.floor(random() * n)`, borné à `n - 1`. `n` doit être > 0.
    fn below(&mut self, n: usize) -> usize {
        assert!(n > 0, "below(0)");
        ((self.next_f64() * n as f64) as usize).min(n - 1)
    }

    /// `randomIntFromRange(min, max)` : bornes incluses.
    fn int_in(&mut self, min: u32, max: u32) -> u32 {
        min + self.below((max - min + 1) as usize) as u32
    }
}

/// Générateur SplitMix64 : rapide, reproductible avec une graine.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Graine tirée de l'heure système et d'une adresse de pile.
    pub fn from_entropy() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let addr = &nanos as *const u64 as u64;
        Self::new(nanos ^ addr.rotate_left(32))
    }
}

impl RandomSource for SplitMix64 {
    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Source scriptée pour les tests : renvoie les valeurs données, en boucle.
pub struct Scripted {
    values: Vec<f64>,
    consumed: usize,
}

impl Scripted {
    pub fn new(values: &[f64]) -> Self {
        Self {
            values: values.to_vec(),
            consumed: 0,
        }
    }

    /// Nombre de valeurs déjà tirées.
    pub fn consumed(&self) -> usize {
        self.consumed
    }
}

impl RandomSource for Scripted {
    fn next_f64(&mut self) -> f64 {
        assert!(
            !self.values.is_empty(),
            "Scripted : aucune valeur disponible"
        );
        let v = self.values[self.consumed % self.values.len()];
        self.consumed += 1;
        v
    }
}

/// Mélange de Fisher-Yates (`shuffle` de Monkeytype).
pub fn shuffle<T>(items: &mut [T], rng: &mut dyn RandomSource) {
    for i in (1..items.len()).rev() {
        let j = rng.below(i + 1);
        items.swap(i, j);
    }
}
