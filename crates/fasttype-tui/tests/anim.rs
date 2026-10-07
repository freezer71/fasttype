use fasttype_tui::anim::{Easing, FrameClock, Tween};

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn easings_match_anime_js() {
    assert!(close(Easing::Linear.apply(0.3), 0.3));
    // out(2) : 1 - (1 - t)²
    assert!(close(Easing::Out(2.0).apply(0.5), 0.75));
    // inOut(1.25) : symétrique autour de (0.5, 0.5)
    let e = Easing::InOut(1.25);
    assert!(close(e.apply(0.5), 0.5));
    assert!(close(e.apply(0.25), 0.5f64.powf(1.25) / 2.0));
    assert!(close(e.apply(0.25) + e.apply(0.75), 1.0));
    for e in [Easing::Linear, Easing::Out(2.0), Easing::InOut(1.25)] {
        assert!(close(e.apply(0.0), 0.0) && close(e.apply(1.0), 1.0));
        assert!(close(e.apply(-1.0), 0.0) && close(e.apply(2.0), 1.0));
    }
}

#[test]
fn tween_at_fixed_times() {
    let t = Tween {
        from: 0.0,
        to: 10.0,
        start: 100.0,
        duration: 100.0,
        easing: Easing::Linear,
    };
    assert_eq!(t.value(50.0), 0.0);
    assert_eq!(t.value(100.0), 0.0);
    assert_eq!(t.value(150.0), 5.0);
    assert_eq!(t.value(200.0), 10.0);
    assert_eq!(t.value(1e9), 10.0);
    assert!(t.is_running(150.0));
    assert!(!t.is_running(200.0));
}

#[test]
fn retarget_starts_from_the_shown_value() {
    let mut t = Tween::fixed(0.0);
    t.retarget(10.0, 0.0, 100.0, Easing::Linear);
    assert_eq!(t.value(50.0), 5.0);
    t.retarget(20.0, 50.0, 100.0, Easing::Linear);
    assert_eq!(t.value(50.0), 5.0, "aucun saut au reciblage");
    assert_eq!(t.value(100.0), 12.5);
    assert_eq!(t.value(150.0), 20.0);
    // même cible : l'animation continue sans repartir
    t.retarget(20.0, 120.0, 100.0, Easing::Linear);
    assert_eq!(t.start, 50.0);
}

#[test]
fn zero_duration_is_a_jump() {
    let mut t = Tween::fixed(3.0);
    t.retarget(7.0, 10.0, 0.0, Easing::InOut(1.25));
    assert_eq!(t.value(10.0), 7.0);
    assert!(!t.is_running(10.0));
}

#[test]
fn frame_clock_is_absolute() {
    let c = FrameClock::new(60, 0.0);
    assert!(close(c.next_after(0.0), 1000.0 / 60.0));
    assert!(close(c.next_after(16.0), 1000.0 / 60.0));
    // une image en retard ne décale pas les suivantes
    assert!(close(c.next_after(40.0), 3.0 * 1000.0 / 60.0));
    let c = FrameClock::new(120, 5.0);
    assert!(close(c.next_after(5.0), 5.0 + 1000.0 / 120.0));
}

#[test]
fn css_cubic_bezier_curves() {
    use fasttype_tui::anim::{CSS_EASE, TAILWIND_EASE};
    for e in [CSS_EASE, TAILWIND_EASE] {
        assert!(e.apply(0.0).abs() < 1e-6 && (e.apply(1.0) - 1.0).abs() < 1e-6);
    }
    // valeurs de référence (navigateur) : ease(0.5) ≈ 0.8024, Tailwind(0.5) ≈ 0.7756
    assert!((CSS_EASE.apply(0.5) - 0.8024).abs() < 1e-3);
    assert!((TAILWIND_EASE.apply(0.5) - 0.7756).abs() < 1e-3);
    // cubic-bezier(0, 0, 1, 1) est linéaire
    assert!((Easing::CubicBezier(0.0, 0.0, 1.0, 1.0).apply(0.3) - 0.3).abs() < 1e-6);
}
