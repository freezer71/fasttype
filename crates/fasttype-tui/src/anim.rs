//! Moteur d'animation : courbes d'anime.js v4 et interpolations reciblables.
//! Tout se calcule à un instant `now` donné (ms) : testable sans horloge réelle.

/// Courbes d'anime.js v4 (`eases.out(p)`, `eases.inOut(p)`) et courbes CSS
/// (`cubic-bezier`), pour les transitions que le site confie au CSS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    /// `out(p)` : `1 - (1 - t)^p`.
    Out(f64),
    /// `inOut(p)` : `(2t)^p / 2` puis `1 - (2 - 2t)^p / 2`.
    InOut(f64),
    /// `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier(f64, f64, f64, f64),
}

/// Courbe par défaut d'anime.js v4 (fondus, défilement des lignes).
pub const OUT2: Easing = Easing::Out(2.0);
/// Courbe du caret (`elements/caret.ts`).
pub const CARET_EASE: Easing = Easing::InOut(1.25);
/// `ease` du CSS : clignotement du caret.
pub const CSS_EASE: Easing = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0);
/// Courbe des transitions Tailwind : focus mode.
pub const TAILWIND_EASE: Easing = Easing::CubicBezier(0.4, 0.0, 0.2, 1.0);

/// Point d'une courbe de Bézier cubique de (0, 0) à (1, 1), coordonnée par coordonnée.
fn bezier(a: f64, b: f64, s: f64) -> f64 {
    let u = 1.0 - s;
    3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
}

/// Valeur `y` de la courbe au point d'abscisse `x` : `s` trouvé par dichotomie
/// (`x(s)` est croissante quand `x1` et `x2` sont dans [0, 1]).
fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    if x <= 0.0 || x >= 1.0 {
        return x.clamp(0.0, 1.0);
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if bezier(x1, x2, mid) < x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    bezier(y1, y2, (lo + hi) / 2.0)
}

impl Easing {
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::Out(p) => 1.0 - (1.0 - t).powf(p),
            Easing::InOut(p) => {
                if t < 0.5 {
                    (2.0 * t).powf(p) / 2.0
                } else {
                    1.0 - (2.0 - 2.0 * t).powf(p) / 2.0
                }
            }
            Easing::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1, y1, x2, y2, t),
        }
    }
}

/// Interpolation d'une valeur de `from` vers `to` entre `start` et `start + duration`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    pub from: f64,
    pub to: f64,
    pub start: f64,
    pub duration: f64,
    pub easing: Easing,
}

impl Tween {
    /// Valeur fixe, sans animation.
    pub fn fixed(value: f64) -> Self {
        Tween {
            from: value,
            to: value,
            start: 0.0,
            duration: 0.0,
            easing: Easing::Linear,
        }
    }

    pub fn value(&self, now: f64) -> f64 {
        if self.duration <= 0.0 || now >= self.start + self.duration {
            return self.to;
        }
        if now <= self.start {
            return self.from;
        }
        let k = self.easing.apply((now - self.start) / self.duration);
        self.from + (self.to - self.from) * k
    }

    pub fn is_running(&self, now: f64) -> bool {
        self.duration > 0.0 && now < self.start + self.duration && self.from != self.to
    }

    /// Nouvelle cible : repart de la valeur affichée à `now` (jamais de saut).
    /// Sans effet si la cible ne change pas.
    pub fn retarget(&mut self, to: f64, now: f64, duration: f64, easing: Easing) {
        if to == self.to {
            return;
        }
        self.from = self.value(now);
        self.to = to;
        self.start = now;
        self.duration = duration;
        self.easing = easing;
    }

    /// Place la valeur sans animation.
    pub fn jump(&mut self, to: f64) {
        *self = Tween::fixed(to);
    }
}

/// Cadence des images d'animation : un planning absolu (`origin + k × période`),
/// sans dérive même si une image prend du retard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameClock {
    pub period: f64,
    pub origin: f64,
}

impl FrameClock {
    pub fn new(fps: u32, origin: f64) -> Self {
        FrameClock {
            period: 1000.0 / f64::from(fps.max(1)),
            origin,
        }
    }

    /// Première échéance d'image strictement après `now`.
    pub fn next_after(&self, now: f64) -> f64 {
        let k = ((now - self.origin) / self.period).floor() + 1.0;
        self.origin + k * self.period
    }
}
