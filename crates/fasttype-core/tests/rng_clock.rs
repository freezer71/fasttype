use fasttype_core::clock::{Clock, ManualClock};
use fasttype_core::rng::{RandomSource, Scripted, SplitMix64, shuffle};

#[test]
fn splitmix_is_reproducible_and_in_unit_interval() {
    let mut a = SplitMix64::new(7);
    let mut b = SplitMix64::new(7);
    for _ in 0..1000 {
        let x = a.next_f64();
        assert_eq!(x, b.next_f64());
        assert!((0.0..1.0).contains(&x));
    }
}

#[test]
fn below_and_int_in_follow_monkeytype_formulas() {
    // Math.floor(random() * n) et randomIntFromRange(min, max)
    let mut r = Scripted::new(&[0.0, 0.999, 0.5]);
    assert_eq!(r.below(4), 0);
    assert_eq!(r.below(4), 3);
    assert_eq!(r.int_in(1, 9), 5); // 1 + floor(0.5 × 9)
    assert_eq!(r.consumed(), 3);
}

#[test]
fn scripted_cycles_through_values() {
    let mut r = Scripted::new(&[0.25]);
    assert_eq!(r.next_f64(), 0.25);
    assert_eq!(r.next_f64(), 0.25);
}

#[test]
#[should_panic(expected = "Scripted")]
fn scripted_without_values_panics_on_use() {
    Scripted::new(&[]).next_f64();
}

#[test]
fn shuffle_is_a_permutation() {
    let mut items: Vec<u32> = (0..20).collect();
    shuffle(&mut items, &mut SplitMix64::new(3));
    let mut sorted = items.clone();
    sorted.sort();
    assert_eq!(sorted, (0..20).collect::<Vec<_>>());
}

#[test]
fn manual_clock_moves_only_when_told() {
    let c = ManualClock::new(100.0);
    assert_eq!(c.now_ms(), 100.0);
    c.advance(50.0);
    assert_eq!(c.now_ms(), 150.0);
    c.set(10.0);
    assert_eq!(c.now_ms(), 10.0);
}
