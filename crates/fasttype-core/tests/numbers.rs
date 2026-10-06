use fasttype_core::numbers::*;

#[test]
fn wpm_is_chars_over_five_per_minute() {
    assert_eq!(calculate_wpm(50.0, 6.0), 100.0);
    assert_eq!(calculate_wpm(250.0, 60.0), 50.0);
}

#[test]
fn wpm_is_zero_without_duration() {
    assert_eq!(calculate_wpm(10.0, 0.0), 0.0);
    assert_eq!(calculate_wpm(10.0, -1.0), 0.0);
}

#[test]
fn js_round_rounds_half_towards_positive_infinity() {
    assert_eq!(js_round(2.5), 3.0);
    assert_eq!(js_round(-2.5), -2.0);
    assert_eq!(js_round(2.4), 2.0);
    assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
}

#[test]
fn round2_keeps_two_decimals() {
    assert_eq!(round2(12.345678), 12.35);
    assert_eq!(round2(2.0), 2.0);
    assert_eq!(round2(99.999), 100.0);
}

#[test]
fn mean_and_population_std_dev() {
    let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
    assert_eq!(mean(&xs), 5.0);
    assert_eq!(std_dev(&xs), 2.0);
    assert_eq!(mean(&[]), 0.0);
    assert_eq!(std_dev(&[]), 0.0);
}

#[test]
fn kogasa_is_100_without_variation() {
    assert_eq!(kogasa(0.0), 100.0);
    assert!(kogasa(0.5) < 100.0);
}

#[test]
fn consistency_matches_monkeytype() {
    assert_eq!(consistency(&[100.0, 100.0, 100.0]), 100.0);
    // cov = 20 / 100 = 0.2 → kogasa ≈ 80.0002 → 80
    assert_eq!(consistency(&[80.0, 120.0]), 80.0);
    // NaN (0 / 0) et liste vide → 0
    assert_eq!(consistency(&[]), 0.0);
    assert_eq!(consistency(&[0.0, 0.0]), 0.0);
}

#[test]
fn whorf_lowers_threshold_for_long_words() {
    assert_eq!(whorf(100, 3), 100);
    assert_eq!(whorf(100, 5), 88); // floor(100 × 1.03^-4) = 88
    assert_eq!(whorf(100, 1), 100); // plafonné à la vitesse
}
