mod common;

use common::{app, col, render, row, screen_text, settle, type_text, type_whole_test};

#[test]
fn test_screen_shows_header_words_and_tips() {
    let mut a = app("v-start", "");
    let (buf, caret) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(row(&buf, 1).contains("fasttype"));
    // barre de config compacte à 80 colonnes, sous le logo
    assert!(row(&buf, 3).contains("time"), "{text}");
    assert!(row(&buf, 3).contains("30"));
    assert!(text.contains("english"), "badge de langue");
    assert!(text.contains("tab + enter - restart"));
    let first = a.session().word(0).trim_end().to_string();
    let words_row = (0..24)
        .find(|&y| row(&buf, y).contains(&first))
        .expect("premier mot affiché");
    let x = col(&buf, words_row, &first).unwrap();
    assert_eq!(
        caret,
        Some((x, words_row)),
        "caret avant la première lettre"
    );
}

#[test]
fn letters_are_colored_by_correctness() {
    let mut a = app("v-colors", "");
    let target: Vec<char> = a.session().word(0).chars().collect();
    let wrong = if target[1] == 'z' { 'y' } else { 'z' };
    type_text(&mut a, &format!("{}{}", target[0], wrong), 0.0, 100.0);
    let (buf, caret) = render(&mut a, 80, 24);
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
    // le focus mode s'installe en 125 ms (logo : 250 ms)
    a.tick(300.0);
    let (buf, _) = render(&mut a, 80, 24);
    let text = screen_text(&buf);
    assert!(!text.contains("restart"));
    assert!(!text.contains("time 30"));
    assert!(text.contains("30"), "timer mini");
    // le logo reste, en couleur sub
    let x = row(&buf, 1).find("fasttype").unwrap() as u16;
    assert_eq!(buf[(x, 1)].fg, a.palette().sub);
}

#[test]
fn focus_mode_fades_the_chrome() {
    let mut a = app("v-focus-fade", "");
    let first: String = a.session().word(0).chars().take(1).collect();
    type_text(&mut a, &first, 0.0, 100.0);
    a.tick(40.0);
    let (buf, _) = render(&mut a, 80, 24);
    let x = col(&buf, 3, "words").expect("encore visible");
    let fg = buf[(x, 3)].fg;
    assert_ne!(fg, a.palette().sub, "en cours de fondu");
    assert_ne!(fg, a.palette().bg);
}

#[test]
fn too_small_terminal_says_so() {
    let mut a = app("v-small", "");
    let (buf, caret) = render(&mut a, 30, 8);
    assert!(screen_text(&buf).contains("terminal too small"));
    assert_eq!(caret, None);
}

#[test]
fn result_screen_shows_speed_and_details() {
    let mut a = app("v-result", "mode = \"words\"\nwords = 10\n");
    let end = type_whole_test(&mut a, 0.0, 300.0);
    settle(&mut a, end);
    let (buf, caret) = render(&mut a, 100, 24);
    let text = screen_text(&buf);
    assert!(text.contains("wpm"), "{text}");
    assert!(text.contains("acc"));
    assert!(text.contains("characters"));
    assert!(text.contains("test type words 10 english"));
    assert!(text.contains('♛'), "couronne du nouveau record");
    assert!(
        text.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)),
        "graphique en braille"
    );
    assert!(text.contains("next test"));
    assert_eq!(caret, None);
}

#[test]
fn background_is_painted() {
    let mut a = app("v-bg", "");
    let (buf, _) = render(&mut a, 80, 24);
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
        let (buf, caret) = render(&mut a, w, h);
        let (cx, cy) = caret.expect("caret visible");
        let before: String = (cx - 2..cx)
            .map(|x| buf[(x, cy)].symbol().to_string())
            .collect();
        assert_eq!(before, typed, "{w}×{h}");
    }
}

#[test]
fn zero_sized_terminal_does_not_panic() {
    let mut a = app("v-zero", "");
    for (w, h) in [(100, 0), (0, 30), (0, 0), (39, 24), (80, 9)] {
        let (_, caret) = render(&mut a, w, h);
        assert_eq!(caret, None, "{w}×{h}");
    }
}
