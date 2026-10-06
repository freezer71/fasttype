//! Utilitaires numériques repris de Monkeytype :
//! `frontend/src/ts/utils/numbers.ts` (calculateWpm),
//! `packages/util/src/numbers.ts` (roundTo2, mean, stdDev, kogasa),
//! `frontend/src/ts/utils/misc.ts` (whorf).

/// `calculateWpm` : un « mot » vaut 5 caractères.
pub fn calculate_wpm(chars: f64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return 0.0;
    }
    chars / 5.0 / (seconds / 60.0)
}

/// `Math.round` de JavaScript : les demis sont arrondis vers +∞.
pub fn js_round(x: f64) -> f64 {
    if x.is_infinite() || x.is_nan() {
        return x;
    }
    (x + 0.5).floor()
}

/// `roundTo2` : `Math.round((x + Number.EPSILON) * 100) / 100`.
pub fn round2(x: f64) -> f64 {
    js_round((x + f64::EPSILON) * 100.0) / 100.0
}

/// Moyenne ; 0 pour une liste vide (comme le `try/catch` de Monkeytype).
pub fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Écart-type de **population** (division par n) ; 0 pour une liste vide.
pub fn std_dev(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    let m = mean(xs);
    (xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64).sqrt()
}

/// `kogasa(cov) = 100 × (1 − tanh(cov + cov³/3 + cov⁵/5))`.
pub fn kogasa(cov: f64) -> f64 {
    100.0 * (1.0 - (cov + cov.powi(3) / 3.0 + cov.powi(5) / 5.0).tanh())
}

/// Consistency d'une série : `roundTo2(kogasa(stdDev / mean))`, 0 si NaN ou 0
/// (`if (!consistency || isNaN(consistency)) consistency = 0`).
pub fn consistency(xs: &[f64]) -> f64 {
    let c = round2(kogasa(std_dev(xs) / mean(xs)));
    if c.is_nan() { 0.0 } else { c }
}

/// Seuil « flex » du min burst : `min(speed, floor(speed × 1.03^(−2 × (len − 3))))`.
pub fn whorf(speed: u32, word_len: usize) -> u32 {
    let flex = (f64::from(speed) * 1.03_f64.powf(-2.0 * (word_len as f64 - 3.0))).floor();
    speed.min(flex as u32)
}
