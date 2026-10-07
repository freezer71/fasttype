//! Placement des mots en lignes, comme le retour à la ligne du site : un mot
//! ne se coupe pas, les lettres en trop élargissent le mot, et le caret suit la
//! saisie. Les largeurs sont en cases de terminal (CJK = 2 cases).

use unicode_width::UnicodeWidthChar;

/// Largeur d'affichage d'un caractère (0 pour un caractère de contrôle).
pub fn char_width(c: char) -> u16 {
    c.width().unwrap_or(0) as u16
}

/// Lettres affichées d'un mot cible : sans le séparateur final ni le saut de ligne.
pub fn letters(target: &str) -> &str {
    target
        .strip_suffix(' ')
        .or_else(|| target.strip_suffix('\n'))
        .unwrap_or(target)
}

/// Lettres de saisie au-delà de la cible (hors séparateur) : affichées en « extra ».
pub fn extras<'a>(target: &str, input: &'a str) -> &'a str {
    let n = letters(target).chars().count();
    let typed = letters(input);
    typed.char_indices().nth(n).map_or("", |(i, _)| &typed[i..])
}

/// Largeur d'un mot à l'écran : lettres cibles puis lettres en trop.
pub fn word_cells(target: &str, input: &str) -> u16 {
    letters(target)
        .chars()
        .chain(extras(target, input).chars())
        .map(char_width)
        .sum()
}

/// Largeur des `n` premières positions saisies : la cible tant qu'il y en a, puis la saisie.
pub fn prefix_cells(target: &str, input: &str, n: usize) -> u16 {
    letters(target)
        .chars()
        .chain(extras(target, input).chars())
        .take(n)
        .map(char_width)
        .sum()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordBox {
    pub index: usize,
    pub x: u16,
    pub width: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub lines: Vec<Vec<WordBox>>,
    /// Ligne et colonne du caret.
    pub caret: (usize, u16),
}

impl Layout {
    /// Première ligne visible : le caret reste sur la deuxième ligne une fois le
    /// premier saut passé (Monkeytype retire la ligne du haut en arrivant à la 3e).
    pub fn first_visible(&self) -> usize {
        self.caret.0.saturating_sub(1)
    }
}

/// Place tous les mots sur des lignes de `width` cases.
pub fn layout_words(words: &[String], inputs: &[String], active: usize, width: u16) -> Layout {
    layout_window(words, inputs, active, width, 0, usize::MAX)
}

/// Place les mots à partir du mot `start` (début d'une ligne), et s'arrête
/// `lines_after` lignes complètes après celle du caret : le coût d'une image
/// ne dépend pas de la longueur du test. Les numéros de ligne partent de `start`.
pub fn layout_window(
    words: &[String],
    inputs: &[String],
    active: usize,
    width: u16,
    start: usize,
    lines_after: usize,
) -> Layout {
    let mut lines: Vec<Vec<WordBox>> = vec![Vec::new()];
    let mut x: u16 = 0;
    let mut caret = None;
    for (index, target) in words.iter().enumerate().skip(start) {
        let input = inputs.get(index).map_or("", String::as_str);
        let w = word_cells(target, input);
        if x > 0 && x.saturating_add(w) > width {
            if caret.is_some_and(|(l, _)| lines.len() - l > lines_after) {
                break;
            }
            lines.push(Vec::new());
            x = 0;
        }
        let line = lines.len() - 1;
        lines[line].push(WordBox { index, x, width: w });
        if index == active {
            let typed = letters(input).chars().count();
            caret = Some((line, x + prefix_cells(target, input, typed)));
        }
        x = x.saturating_add(w + 1);
        if target.ends_with('\n') || input.ends_with('\n') {
            if caret.is_some_and(|(l, _)| lines.len() - l > lines_after) {
                break;
            }
            lines.push(Vec::new());
            x = 0;
        }
    }
    if lines.last().is_some_and(Vec::is_empty) && lines.len() > 1 {
        lines.pop();
    }
    Layout {
        lines,
        caret: caret.unwrap_or((0, 0)),
    }
}
