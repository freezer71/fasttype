use fasttype_tui::caret::{Caret, CaretFrame, CaretStyle, CaretTarget, coverage, smooth_caret_ms};

fn at(x: f64, y: f64) -> CaretTarget {
    CaretTarget { x, y, width: 1.0 }
}

#[test]
fn smooth_caret_durations() {
    assert_eq!(smooth_caret_ms("off"), 0.0);
    assert_eq!(smooth_caret_ms("slow"), 150.0);
    assert_eq!(smooth_caret_ms("medium"), 100.0);
    assert_eq!(smooth_caret_ms("fast"), 85.0);
}

#[test]
fn styles_from_config() {
    assert_eq!(CaretStyle::from_config("default"), CaretStyle::Bar);
    assert_eq!(CaretStyle::from_config("carrot"), CaretStyle::Bar);
    assert_eq!(CaretStyle::from_config("block"), CaretStyle::Block);
    assert_eq!(CaretStyle::from_config("off"), CaretStyle::Off);
    assert!(CaretStyle::Underline.is_full_width());
    assert!(!CaretStyle::Bar.is_full_width());
}

#[test]
fn caret_glides_with_in_out_curve() {
    let mut c = Caret::default();
    c.jump(at(10.0, 5.0));
    c.go_to(at(11.0, 5.0), 0.0, 100.0);
    let f = |t| c.frame(CaretStyle::Bar, t, true);
    assert_eq!(f(0.0).x, 10.0);
    assert_eq!(
        f(50.0).x,
        10.5,
        "inOut est symétrique : mi-chemin à mi-temps"
    );
    assert!(f(25.0).x < 10.25, "départ lent");
    assert_eq!(f(150.0).x, 11.0);
    assert!(f(50.0).moving && !f(100.0).moving);
}

#[test]
fn retarget_mid_glide_never_jumps_back() {
    let mut c = Caret::default();
    c.jump(at(0.0, 0.0));
    c.go_to(at(1.0, 0.0), 0.0, 100.0);
    let shown = c.frame(CaretStyle::Bar, 50.0, true).x;
    c.go_to(at(2.0, 0.0), 50.0, 100.0);
    assert_eq!(c.frame(CaretStyle::Bar, 50.0, true).x, shown);
    let mut last = shown;
    for t in (50..=150).step_by(5) {
        let x = c.frame(CaretStyle::Bar, f64::from(t), true).x;
        assert!(x >= last, "le caret ne recule jamais");
        last = x;
    }
    assert_eq!(last, 2.0);
}

#[test]
fn smooth_off_is_a_jump() {
    let mut c = Caret::default();
    c.go_to(at(7.0, 2.0), 0.0, 0.0);
    assert_eq!(c.frame(CaretStyle::Bar, 0.0, false).x, 7.0);
}

#[test]
fn blink_is_smooth_then_solid_once_typing() {
    let mut c = Caret::default();
    c.stop_blinking();
    c.start_blinking(1000.0);
    assert_eq!(c.opacity(1000.0, true), 0.0);
    assert!((c.opacity(1500.0, true) - 1.0).abs() < 1e-9);
    assert!(
        (c.opacity(1250.0, true) - 0.8024).abs() < 1e-3,
        "courbe ease"
    );
    assert_eq!(c.opacity(2000.0, true), 0.0, "période d'une seconde");
    // clignotement franc quand smoothCaret est off
    assert_eq!(c.opacity(1200.0, false), 1.0);
    assert_eq!(c.opacity(1700.0, false), 0.0);
    c.stop_blinking();
    assert_eq!(c.opacity(1700.0, true), 1.0);
}

#[test]
fn coverage_splits_a_block_between_two_cells() {
    let f = CaretFrame {
        style: CaretStyle::Block,
        x: 3.25,
        y: 2.0,
        width: 1.0,
        opacity: 1.0,
        moving: true,
    };
    assert_eq!(coverage(&f, 3, 2), 0.75);
    assert_eq!(coverage(&f, 4, 2), 0.25);
    assert_eq!(coverage(&f, 5, 2), 0.0);
    assert_eq!(coverage(&f, 3, 1), 0.0);
    let wide = CaretFrame {
        width: 2.0,
        x: 3.0,
        ..f
    };
    assert_eq!(coverage(&wide, 4, 2), 1.0);
}
