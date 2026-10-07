use fasttype_tui::perf::{LatencyStats, Perf};

#[test]
fn percentiles_by_nearest_rank() {
    let mut s = LatencyStats::default();
    assert_eq!(s.percentile(50.0), None);
    for v in 1..=100 {
        s.record(f64::from(v));
    }
    assert_eq!(s.percentile(50.0), Some(50.0));
    assert_eq!(s.percentile(99.0), Some(99.0));
    assert_eq!(s.percentile(100.0), Some(100.0));
}

#[test]
fn keeps_only_recent_samples() {
    let mut s = LatencyStats::default();
    for _ in 0..1024 {
        s.record(100.0);
    }
    for _ in 0..1024 {
        s.record(1.0);
    }
    assert_eq!(s.count(), 1024);
    assert_eq!(s.percentile(99.0), Some(1.0));
}

#[test]
fn summary_mentions_latency() {
    let mut p = Perf::new(true);
    p.input_to_flush.record(0.5);
    assert!(p.summary().contains("p99 0.50"), "{}", p.summary());
}
