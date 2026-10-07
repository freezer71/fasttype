mod common;

use common::app;
use fasttype_tui::view::config_bar::{bar_groups, bar_width};

#[test]
fn config_bar_follows_the_mode() {
    let a = app("bar-time", "");
    let g = bar_groups(&a.store.config, false);
    let texts: Vec<Vec<(&str, bool)>> = g
        .iter()
        .map(|grp| grp.iter().map(|i| (i.text.as_str(), i.active)).collect())
        .collect();
    assert_eq!(texts[0], [("@ punctuation", false), ("# numbers", false)]);
    assert_eq!(
        texts[1],
        [
            ("time", true),
            ("words", false),
            ("quote", false),
            ("zen", false),
            ("custom", false)
        ]
    );
    assert_eq!(
        texts[2],
        [
            ("15", false),
            ("30", true),
            ("60", false),
            ("120", false),
            ("custom", false)
        ]
    );
    assert_eq!(bar_width(&g), 92);
    let compact = bar_groups(&a.store.config, true);
    assert_eq!(compact[0][0].text, "@");
    assert_eq!(bar_width(&compact), 72);

    let a = app("bar-quote", "mode = \"quote\"\npunctuation = true\n");
    let g = bar_groups(&a.store.config, false);
    assert!(!g[0][0].enabled && !g[0][0].active, "désactivé en quote");
    let lengths: Vec<_> = g[2].iter().map(|i| (i.text.as_str(), i.active)).collect();
    assert_eq!(
        lengths,
        [
            ("all", false),
            ("short", false),
            ("medium", true),
            ("long", false),
            ("thicc", false)
        ]
    );

    let a = app("bar-zen", "mode = \"zen\"\n");
    let g = bar_groups(&a.store.config, false);
    assert_eq!(g.len(), 1, "en zen : seulement les modes");

    let a = app("bar-words", "mode = \"words\"\nwords = 42\n");
    let g = bar_groups(&a.store.config, false);
    assert!(
        g[2].last().unwrap().active,
        "valeur hors préréglages : custom"
    );
}
