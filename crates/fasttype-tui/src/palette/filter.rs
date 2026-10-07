//! Recherche de la palette, comme `commandline/filter.ts` : chaque mot saisi
//! doit être le début d'un mot du libellé ou d'un alias (un mot du libellé
//! ne sert qu'une fois). On garde les commandes qui apparient le plus de
//! mots, puis le plus de caractères. L'ordre de la liste ne change pas.

/// Retire la ponctuation ASCII, comme `stripPunctuation`.
fn strip_punctuation(s: &str) -> String {
    s.chars().filter(|c| !c.is_ascii_punctuation()).collect()
}

/// Mots en minuscules, sans ponctuation (`splitWords`).
pub fn split_words(s: &str) -> Vec<String> {
    s.to_lowercase().split(' ').map(strip_punctuation).collect()
}

/// Mots de la saisie : sans `>` initial, sans mots vides.
pub fn query_words(input: &str) -> Vec<String> {
    let s = input.trim_start_matches('>').to_lowercase();
    s.trim()
        .split(' ')
        .map(strip_punctuation)
        .filter(|w| !w.is_empty())
        .collect()
}

/// (mots appariés, caractères appariés) d'une commande dont les mots du
/// libellé et des alias sont `words`.
pub fn score(query: &[String], words: &[String]) -> (usize, usize) {
    let mut claimed: Vec<Option<usize>> = vec![None; words.len()];
    let mut strength = 0;
    for (qi, q) in query.iter().enumerate() {
        for (wi, w) in words.iter().enumerate() {
            if w.starts_with(q.as_str()) && claimed[wi].is_none() && !claimed.contains(&Some(qi)) {
                claimed[wi] = Some(qi);
                strength += q.len();
            }
        }
    }
    (claimed.iter().filter(|c| c.is_some()).count(), strength)
}

/// Indices des commandes à montrer. `words[i]` : mots du libellé et des
/// alias de la commande `i` ; `None` si elle n'est pas disponible.
pub fn filter(input: &str, words: &[Option<&[String]>]) -> Vec<usize> {
    let query = query_words(input);
    if query.is_empty() {
        return (0..words.len()).filter(|&i| words[i].is_some()).collect();
    }
    let scores: Vec<Option<(usize, usize)>> =
        words.iter().map(|w| w.map(|w| score(&query, w))).collect();
    let max_strength = scores.iter().flatten().map(|s| s.1).max().unwrap_or(0);
    let mut min_count = query.len();
    while min_count > 0 && !scores.iter().flatten().any(|s| s.0 >= min_count) {
        min_count -= 1;
    }
    let min_count = min_count.max(1);
    (0..words.len())
        .filter(|&i| scores[i].is_some_and(|(c, s)| c >= min_count && s >= max_strength))
        .collect()
}
