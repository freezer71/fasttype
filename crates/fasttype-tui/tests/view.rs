mod common;

use common::{app, render, row, screen_text, type_text, type_whole_test};

#[test]
fn test_screen_shows_header_words_and_tips() {
    let a = app("v-start", "");
    let (buf, caret) = render(&a, 80, 24);
    let text = screen_text(&buf);
    assert!(row(&buf, 1).contains("fasttype"));
    assert!(row(&buf, 1).contains("time 30 · english"));
    assert!(text.contains("tab + enter - restart"));
    let first = a.session().word(0).trim_end().to_string();
    let words_row = (0..24)
        .find(|&y| row(&buf, y).contains(&first))
        .expect("premier mot affiché");
    let x = row(&buf, words_row).find(&first).unwrap();
    assert_eq!(
        caret,
        Some((x as u16, words_row)),
        "caret avant la première lettre"
    );
}

#[test]
fn letters_are_colored_by_correctness() {
    let mut a = app("v-colors", "");
    let target: Vec<char> = a.session().word(0).chars().collect();
    let wrong = if target[1] == 'z' { 'y' } else { 'z' };
    type_text(&mut a, &format!("{}{}", target[0], wrong), 0.0, 100.0);
    let (buf, caret) = render(&a, 80, 24);
    let (cx, cy) = caret.unwrap();
    let p = *a.palette();
    assert_eq!(buf[(cx - 2, cy)].fg, p.text, "lettre juste");
    assert_eq!(buf[(cx - 1, cy)].fg, p.error, "lettre fausse");
    assert_eq!(
        buf[(cx - 1, cy)].symbol(),
        target[1].to_string(),
        "la lettre attendue est affichée"
    );
    assert_eq!(buf[(cx, cy)].fg, p.sub, "lettre à venir");
}

#[test]
fn focus_mode_hides_chrome_and_shows_timer() {
    let mut a = app("v-focus", "");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 100.0);
    let (buf, _) = render(&a, 80, 24);
    let text = screen_text(&buf);
    assert!(!text.contains("fasttype"));
    assert!(!text.contains("restart"));
    assert!(text.contains("30"), "timer mini");
}

#[test]
fn too_small_terminal_says_so() {
    let a = app("v-small", "");
    let (buf, caret) = render(&a, 30, 8);
    assert!(screen_text(&buf).contains("terminal too small"));
    assert_eq!(caret, None);
}

#[test]
fn result_screen_shows_speed_and_details() {
    let mut a = app("v-result", "mode = \"words\"\nwords = 10\n");
    type_whole_test(&mut a, 0.0, 300.0);
    let (buf, caret) = render(&a, 100, 24);
    let text = screen_text(&buf);
    assert!(text.contains("wpm"), "{text}");
    assert!(text.contains("acc"));
    assert!(text.contains("characters"));
    assert!(text.contains("test type words 10 english"));
    assert!(text.contains("new personal best"));
    assert!(text.contains("next test"));
    assert_eq!(caret, None);
}

#[test]
fn background_is_painted() {
    let a = app("v-bg", "");
    let (buf, _) = render(&a, 80, 24);
    assert_eq!(buf[(0, 0)].bg, a.palette().bg);
    assert_eq!(buf[(79, 23)].bg, a.palette().bg);
}

#[test]
fn resize_keeps_caret_after_the_typed_letters() {
    let mut a = app("v-resize", "");
    let word: String = a.session().word(0).trim_end().to_string();
    let typed: String = word.chars().take(2).collect();
    type_text(&mut a, &typed, 0.0, 100.0);
    for (w, h) in [(80, 24), (50, 12), (120, 40)] {
        let (buf, caret) = render(&a, w, h);
        let (cx, cy) = caret.expect("caret visible");
        let before: String = (cx - 2..cx)
            .map(|x| buf[(x, cy)].symbol().to_string())
            .collect();
        assert_eq!(before, typed, "{w}×{h}");
    }
}

#[test]
fn zero_sized_terminal_does_not_panic() {
    let a = app("v-zero", "");
    for (w, h) in [(100, 0), (0, 30), (0, 0), (39, 24), (80, 9)] {
        let (_, caret) = render(&a, w, h);
        assert_eq!(caret, None, "{w}×{h}");
    }
}
