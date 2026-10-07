use fasttype_core::result::ChartData;
use fasttype_data::{DEFAULT_THEME, theme};
use fasttype_tui::theme::{ColorMode, Palette};
use fasttype_tui::view::big;
use fasttype_tui::view::chart::{ChartView, sample, smooth_with_value_window, y_range};
use fasttype_tui::view::live::seconds_to_string;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

fn palette() -> Palette {
    Palette::from_theme(theme(DEFAULT_THEME).unwrap(), ColorMode::TrueColor)
}

#[test]
fn seconds_like_monkeytype() {
    assert_eq!(seconds_to_string(30), "30");
    assert_eq!(seconds_to_string(5), "5");
    assert_eq!(seconds_to_string(60), "01:00");
    assert_eq!(seconds_to_string(75), "01:15");
    assert_eq!(seconds_to_string(605), "10:05");
    assert_eq!(seconds_to_string(3605), "01:00:05");
}

#[test]
fn value_window_smoothing() {
    // le pic isolé n'est pas moyenné avec ses voisins trop éloignés
    let v = [10.0, 12.0, 100.0, 14.0];
    let s = smooth_with_value_window(&v, 1, 25.0);
    assert_eq!(s[0], 11.0);
    assert_eq!(s[1], 11.0);
    assert_eq!(s[2], 100.0);
    assert_eq!(s[3], 14.0);
}

#[test]
fn left_axis_range() {
    assert_eq!(y_range(&[&[42.0, 87.0], &[95.0]], true), (0.0, 100.0));
    assert_eq!(y_range(&[&[42.0, 87.0]], false), (40.0, 90.0));
    assert_eq!(y_range(&[&[]], true), (0.0, 10.0));
}

#[test]
fn spline_passes_through_the_points() {
    let v = [10.0, 30.0, 20.0, 40.0];
    for (i, &x) in v.iter().enumerate() {
        assert!((sample(&v, i as f64) - x).abs() < 1e-9);
    }
    let mid = sample(&v, 1.5);
    assert!(mid > 20.0 && mid < 30.0);
}

#[test]
fn chart_draws_series_errors_and_labels() {
    let p = palette();
    let chart = ChartData {
        wpm: vec![40.0, 60.0, 80.0, 90.0],
        raw: vec![50.0, 70.0, 85.0, 95.0],
        burst: vec![50.0, 80.0, 100.0, 95.0],
        err: vec![0, 2, 0, 1],
    };
    let area = Rect::new(0, 0, 60, 12);
    let mut buf = Buffer::empty(area);
    ChartView {
        chart: &chart,
        palette: &p,
        factor: 1.0,
        unit: "wpm",
        start_at_zero: true,
        duration: 4.0,
    }
    .render(&mut buf, area);
    let cells: Vec<_> = buf.content().iter().collect();
    assert!(
        cells.iter().any(|c| c
            .symbol()
            .starts_with(|ch| ('\u{2801}'..='\u{28ff}').contains(&ch))
            && c.fg == p.main),
        "wpm en main"
    );
    assert!(
        cells.iter().any(|c| c.symbol() == "×" && c.fg == p.error),
        "croix d'erreurs"
    );
    let text: String = cells.iter().map(|c| c.symbol()).collect();
    assert!(text.contains("100"), "max de l'axe arrondi à la dizaine");
    assert!(text.contains("errors"));
    assert!(text.contains("wpm"));
}

#[test]
fn fractional_last_second_is_labelled() {
    let p = palette();
    let chart = ChartData {
        wpm: vec![40.0, 60.0, 62.0],
        raw: vec![40.0, 60.0, 62.0],
        burst: vec![40.0, 60.0, 62.0],
        err: vec![0, 0, 0],
    };
    let area = Rect::new(0, 0, 60, 10);
    let mut buf = Buffer::empty(area);
    ChartView {
        chart: &chart,
        palette: &p,
        factor: 1.0,
        unit: "wpm",
        start_at_zero: true,
        duration: 2.75,
    }
    .render(&mut buf, area);
    let text: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(text.contains("2.75"), "{text}");
}

#[test]
fn big_digits() {
    assert_eq!(big::width("8"), 3);
    assert_eq!(big::width("100%"), 15);
    assert!(big::supported("01:15"));
    assert!(!big::supported("Infinite"));
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 3));
    big::draw(&mut buf, 0, 0, "7", Style::default());
    assert_eq!(buf[(0, 0)].symbol(), "▀");
    assert_eq!(buf[(2, 2)].symbol(), "▀");
    // hors de l'écran : rien, sans panique
    big::draw(&mut buf, 6, 2, "88", Style::default());
}
