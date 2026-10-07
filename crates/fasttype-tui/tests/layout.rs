use fasttype_tui::layout::{extras, layout_words, letters, prefix_cells, word_cells};

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn letters_and_extras() {
    assert_eq!(letters("hello "), "hello");
    assert_eq!(letters("line\n"), "line");
    assert_eq!(extras("ab ", "abcd"), "cd");
    assert_eq!(extras("ab ", "abcd "), "cd");
    assert_eq!(extras("ab ", "a"), "");
}

#[test]
fn widths_count_cells_not_bytes() {
    assert_eq!(word_cells("café ", ""), 4);
    assert_eq!(word_cells("日本 ", ""), 4);
    assert_eq!(
        word_cells("ab ", "abxx"),
        4,
        "les lettres en trop élargissent le mot"
    );
    assert_eq!(prefix_cells("日本 ", "日", 1), 2);
    assert_eq!(prefix_cells("ab ", "abxx", 3), 3);
}

#[test]
fn words_wrap_without_splitting() {
    let words = s(&["aa ", "bb ", "cc ", "dd"]);
    let l = layout_words(&words, &s(&["", "", "", ""]), 0, 5);
    let lines: Vec<Vec<usize>> = l
        .lines
        .iter()
        .map(|line| line.iter().map(|b| b.index).collect())
        .collect();
    assert_eq!(lines, [vec![0, 1], vec![2, 3]]);
    assert_eq!(l.lines[0][1].x, 3);
}

#[test]
fn caret_follows_input_and_extras() {
    let words = s(&["aa ", "bb ", "cc"]);
    let l = layout_words(&words, &s(&["aa ", "b", ""]), 1, 20);
    assert_eq!(l.caret, (0, 4));
    let l = layout_words(&words, &s(&["aa ", "bbxx", ""]), 1, 20);
    assert_eq!(l.caret, (0, 7));
    assert_eq!(
        l.lines[0][2].x, 8,
        "le mot suivant est poussé par les lettres en trop"
    );
}

#[test]
fn newline_word_ends_the_line() {
    let words = s(&["a\n", "b "]);
    let l = layout_words(&words, &s(&["", ""]), 0, 20);
    assert_eq!(l.lines.len(), 2);
}

#[test]
fn window_keeps_caret_on_second_line() {
    let words = s(&["aa ", "bb ", "cc ", "dd ", "ee"]);
    let inputs = s(&["aa ", "bb ", "cc ", "dd ", ""]);
    // largeur 2 : un mot par ligne
    let l = layout_words(&words, &inputs, 0, 2);
    assert_eq!(l.first_visible(), 0);
    let l = layout_words(&words, &inputs, 1, 2);
    assert_eq!(
        l.first_visible(),
        0,
        "le premier saut de ligne ne fait pas défiler"
    );
    let l = layout_words(&words, &inputs, 3, 2);
    assert_eq!(l.first_visible(), 2);
}

#[test]
fn word_wider_than_line_gets_its_own_line() {
    let words = s(&["a ", "abcdefghij ", "b"]);
    let l = layout_words(&words, &s(&["", "", ""]), 1, 4);
    let lines: Vec<Vec<usize>> = l
        .lines
        .iter()
        .map(|line| line.iter().map(|b| b.index).collect())
        .collect();
    assert_eq!(lines, [vec![0], vec![1], vec![2]]);
    assert_eq!(l.caret, (1, 0));
}
