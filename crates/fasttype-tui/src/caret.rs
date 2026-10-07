//! Caret de Monkeytype (`elements/caret.ts`, `caret.scss`) : position en cases
//! fractionnaires, glissement `inOut(1.25)`, clignotement d'une seconde.

use crate::anim::{CARET_EASE, CSS_EASE, Tween};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretStyle {
    Off,
    /// `default` : barre fine avant la lettre.
    Bar,
    /// Pavé derrière la lettre.
    Block,
    /// Contour de la lettre.
    Outline,
    /// Trait sous la lettre.
    Underline,
}

impl CaretStyle {
    /// `caretStyle` ; les carets en image (carrot, banana, monkey) deviennent une barre.
    pub fn from_config(name: &str) -> Self {
        match name {
            "off" => CaretStyle::Off,
            "block" => CaretStyle::Block,
            "outline" => CaretStyle::Outline,
            "underline" => CaretStyle::Underline,
            _ => CaretStyle::Bar,
        }
    }

    /// Block, outline et underline prennent la largeur de la lettre visée.
    pub fn is_full_width(self) -> bool {
        matches!(
            self,
            CaretStyle::Block | CaretStyle::Outline | CaretStyle::Underline
        )
    }
}

/// `smoothCaret` → durée du glissement (ms).
pub fn smooth_caret_ms(setting: &str) -> f64 {
    match setting {
        "slow" => 150.0,
        "medium" => 100.0,
        "fast" => 85.0,
        _ => 0.0,
    }
}

/// Où le caret doit aller : bord gauche de la lettre visée (colonne, ligne, en
/// cases de l'écran) et largeur de cette lettre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretTarget {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    /// Hauteur de la lettre en lignes (taille du texte, `fontSize`).
    pub height: f64,
}

/// Ce qu'il faut dessiner à un instant donné.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretFrame {
    pub style: CaretStyle,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    /// Encore en mouvement : l'image suivante sera différente.
    pub moving: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Caret {
    x: Tween,
    y: Tween,
    width: Tween,
    height: f64,
    /// Début du cycle de clignotement ; `None` : caret plein (on tape).
    blink_since: Option<f64>,
}

impl Default for Caret {
    fn default() -> Self {
        Caret {
            x: Tween::fixed(0.0),
            y: Tween::fixed(0.0),
            width: Tween::fixed(1.0),
            height: 1.0,
            blink_since: Some(0.0),
        }
    }
}

impl Caret {
    /// `resetPosition` : place le caret sans animation.
    pub fn jump(&mut self, t: CaretTarget) {
        self.x.jump(t.x);
        self.y.jump(t.y);
        self.width.jump(t.width);
        self.height = t.height;
    }

    /// `goTo` : glisse vers la cible en `duration` ms (0 : saut), en repartant
    /// de la position affichée si une animation est en cours.
    pub fn go_to(&mut self, t: CaretTarget, now: f64, duration: f64) {
        self.x.retarget(t.x, now, duration, CARET_EASE);
        self.y.retarget(t.y, now, duration, CARET_EASE);
        self.width.retarget(t.width, now, duration, CARET_EASE);
        self.height = t.height;
    }

    pub fn target(&self) -> CaretTarget {
        CaretTarget {
            x: self.x.to,
            y: self.y.to,
            width: self.width.to,
            height: self.height,
        }
    }

    pub fn start_blinking(&mut self, now: f64) {
        if self.blink_since.is_none() {
            self.blink_since = Some(now);
        }
    }

    /// Chaque frappe rend le caret plein (`stopBlinking`).
    pub fn stop_blinking(&mut self) {
        self.blink_since = None;
    }

    /// Début du cycle de clignotement en cours.
    pub fn blink_since(&self) -> Option<f64> {
        self.blink_since
    }

    pub fn is_blinking(&self) -> bool {
        self.blink_since.is_some()
    }

    /// Opacité du clignotement : `caretFlashSmooth` (0 → 1 → 0 en 1 s, courbe
    /// `ease` entre les étapes) ou `caretFlashHard` (plein 50 %, éteint 50 %).
    pub fn opacity(&self, now: f64, smooth: bool) -> f64 {
        let Some(since) = self.blink_since else {
            return 1.0;
        };
        let phase = (now - since).rem_euclid(1000.0) / 1000.0;
        if !smooth {
            return if phase < 0.5 { 1.0 } else { 0.0 };
        }
        if phase < 0.5 {
            CSS_EASE.apply(phase * 2.0)
        } else {
            1.0 - CSS_EASE.apply((phase - 0.5) * 2.0)
        }
    }

    pub fn is_moving(&self, now: f64) -> bool {
        self.x.is_running(now) || self.y.is_running(now) || self.width.is_running(now)
    }

    pub fn frame(&self, style: CaretStyle, now: f64, smooth_blink: bool) -> CaretFrame {
        CaretFrame {
            style,
            x: self.x.value(now),
            y: self.y.value(now),
            width: self.width.value(now),
            height: self.height,
            opacity: self.opacity(now, smooth_blink),
            moving: self.is_moving(now),
        }
    }
}

/// Part de la case (`col`, `row`) couverte par le rectangle du caret
/// `[x, x + width) × [y, y + height)` : sert à teinter le fond des lettres pendant
/// le glissement du caret bloc (rendu demi-case et plus fin).
pub fn coverage(f: &CaretFrame, col: u16, row: u16) -> f64 {
    coverage_box(f, col, row, 1)
}

/// Part d'un bloc de `size × size` cases (une lettre agrandie) couverte par le caret.
pub fn coverage_box(f: &CaretFrame, col: u16, row: u16, size: u16) -> f64 {
    let s = f64::from(size.max(1));
    let overlap = |a0: f64, a1: f64, b0: f64| (a1.min(b0 + s) - a0.max(b0)).max(0.0) / s;
    overlap(f.x, f.x + f.width, f64::from(col)) * overlap(f.y, f.y + f.height, f64::from(row))
}
