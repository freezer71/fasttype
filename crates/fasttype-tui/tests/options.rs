use fasttype_tui::runner::parse_options;

#[test]
fn command_line_options() {
    let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let o = parse_options(&args(&[])).unwrap();
    assert!(!o.perf && o.fps == 60);
    let o = parse_options(&args(&["--fps", "144", "--perf"])).unwrap();
    assert!(o.perf && o.fps == 144);
    assert!(parse_options(&args(&["--fps", "30"])).is_err());
    assert!(parse_options(&args(&["--fps"])).is_err());
    assert!(parse_options(&args(&["--nope"])).is_err());
}
