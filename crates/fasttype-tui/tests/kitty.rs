use fasttype_tui::caret::{CaretFrame, CaretStyle};
use fasttype_tui::kitty::{
    CaretRenderer, CellPx, KittyCaret, base64, caret_image, caret_origin, place, probe_answer,
    transmit,
};

const CELL: CellPx = CellPx { w: 10, h: 24 };

fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.to_string())
    }
}

fn bar(x: f64, y: f64, opacity: f64) -> CaretFrame {
    CaretFrame {
        style: CaretStyle::Bar,
        x,
        y,
        width: 1.0,
        height: 1.0,
        opacity,
        moving: false,
    }
}

#[test]
fn base64_matches_rfc_4648() {
    assert_eq!(base64(b""), "");
    assert_eq!(base64(b"f"), "Zg==");
    assert_eq!(base64(b"fo"), "Zm8=");
    assert_eq!(base64(b"foo"), "Zm9v");
    assert_eq!(base64(b"foobar"), "Zm9vYmFy");
}

#[test]
fn cell_size_from_window() {
    assert_eq!(
        CellPx::from_window(100, 50, 1000, 1200),
        Some(CellPx { w: 10, h: 24 })
    );
    assert_eq!(CellPx::from_window(100, 50, 0, 0), None);
}

#[test]
fn renderer_detection() {
    let cell = Some(CELL);
    let none = env(&[]);
    assert_eq!(
        CaretRenderer::detect(&none, cell, true),
        CaretRenderer::Kitty(CELL),
        "le terminal a répondu OK à la sonde graphique"
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("TERM", "xterm-kitty")]), cell, false),
        CaretRenderer::Cell,
        "pas de réponse (Zellij, tmux, terminal imbriqué) : pas d'image"
    );
    assert_eq!(
        CaretRenderer::detect(&none, None, true),
        CaretRenderer::Cell,
        "taille des cases inconnue"
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("FASTTYPE_CARET", "kitty")]), cell, false),
        CaretRenderer::Kitty(CELL)
    );
    assert_eq!(
        CaretRenderer::detect(env(&[("FASTTYPE_CARET", "cell")]), cell, true),
        CaretRenderer::Cell
    );
}

#[test]
fn graphics_probe_answers() {
    assert_eq!(probe_answer(b""), None, "réponse pas encore arrivée");
    assert_eq!(probe_answer(b"\x1b_Gi=31;OK\x1b\\"), None, "attend DA1");
    assert_eq!(probe_answer(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;22c"), Some(true));
    assert_eq!(probe_answer(b"\x1b[?62c"), Some(false));
    assert_eq!(
        probe_answer(b"\x1b_Gi=31;EINVAL:bad\x1b\\\x1b[?1;2c"),
        Some(false)
    );
}

#[test]
fn unchanged_caret_is_not_sent_again() {
    let mut k = KittyCaret::new(CELL);
    let mut out = Vec::new();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), (1, 2, 3))
        .unwrap();
    out.clear();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), (1, 2, 3))
        .unwrap();
    assert!(out.is_empty(), "même image au même endroit : rien à écrire");
    k.invalidate();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), (1, 2, 3))
        .unwrap();
    assert!(!out.is_empty(), "après un effacement d'écran, on replace");
}

#[test]
fn transmit_is_chunked_at_4096() {
    let small = transmit(7, 1, 1, &[1, 2, 3, 4]);
    assert_eq!(small, b"\x1b_Ga=t,f=32,s=1,v=1,i=7,q=2,m=0;AQIDBA==\x1b\\");
    let big = String::from_utf8(transmit(8, 40, 40, &[0u8; 40 * 40 * 4])).unwrap();
    // 6400 octets → 8536 caractères base64 → 3 morceaux
    assert_eq!(big.matches("\x1b_G").count(), 3);
    assert!(big.starts_with("\x1b_Ga=t,f=32,s=40,v=40,i=8,q=2,m=1;"));
    assert_eq!(big.matches("\x1b_Gm=1;").count(), 1);
    assert_eq!(big.matches("\x1b_Gm=0;").count(), 1);
}

#[test]
fn placement_moves_the_cursor_then_places() {
    let mut out = Vec::new();
    place(&mut out, 1101, 4, 2, 3, 0).unwrap();
    assert_eq!(
        out,
        b"\x1b[3;5H\x1b_Ga=p,i=1101,p=1,X=3,Y=0,C=1,z=1,q=2\x1b\\"
    );
}

#[test]
fn images_have_monkeytype_proportions() {
    let rgb = (226, 183, 20);
    let (w, h, px) = caret_image(CaretStyle::Bar, CELL, 1, 1, rgb, 255);
    assert_eq!((w, h), (2, 24), "0,1em de large, toute la hauteur");
    assert_eq!(px.len(), (w * h * 4) as usize);
    assert_eq!(&px[..4], &[226, 183, 20, 255]);
    let (w, h, _) = caret_image(CaretStyle::Underline, CELL, 2, 1, rgb, 255);
    assert_eq!((w, h), (20, 2), "largeur de la lettre (CJK : 2 cases)");
    let (w, h, px) = caret_image(CaretStyle::Outline, CELL, 1, 1, rgb, 255);
    assert_eq!((w, h), (10, 24));
    let alpha = |x: u32, y: u32| px[((y * w + x) * 4 + 3) as usize];
    assert_eq!(alpha(0, 5), 255, "bord");
    assert_eq!(alpha(5, 12), 0, "intérieur transparent");
}

#[test]
fn bar_is_centred_on_the_letter_edge() {
    // x = 3.5 cases → 35 px, moins la moitié de la barre (1 px)
    assert_eq!(caret_origin(&bar(3.5, 2.0, 1.0), CELL), (34, 48));
    assert_eq!(
        caret_origin(&bar(0.0, 0.0, 1.0), CELL),
        (0, 0),
        "jamais négatif"
    );
    let u = CaretFrame {
        style: CaretStyle::Underline,
        ..bar(1.0, 1.0, 1.0)
    };
    assert_eq!(caret_origin(&u, CELL), (10, 46), "sous la lettre");
}

#[test]
fn kitty_caret_sends_each_image_once_and_moves_it() {
    let mut k = KittyCaret::new(CELL);
    let rgb = (1, 2, 3);
    let mut out = Vec::new();
    k.draw(&mut out, Some(bar(3.5, 2.0, 1.0)), rgb).unwrap();
    let first = String::from_utf8(out.clone()).unwrap();
    assert!(first.contains("a=t"), "transmission");
    assert!(first.ends_with("\x1b[3;4H\x1b_Ga=p,i=11117,p=1,X=4,Y=0,C=1,z=1,q=2\x1b\\"));
    out.clear();
    k.draw(&mut out, Some(bar(3.6, 2.0, 1.0)), rgb).unwrap();
    let second = String::from_utf8(out.clone()).unwrap();
    assert!(!second.contains("a=t"), "image déjà transmise");
    assert!(second.contains("X=5"));
    out.clear();
    // clignotement : une autre opacité est une autre image ; l'ancienne est retirée
    k.draw(&mut out, Some(bar(3.6, 2.0, 0.5)), rgb).unwrap();
    let third = String::from_utf8(out.clone()).unwrap();
    assert!(third.starts_with("\x1b_Ga=t"));
    assert!(third.contains("\x1b_Ga=d,d=i,i=11117,q=2\x1b\\"));
    out.clear();
    k.draw(&mut out, None, rgb).unwrap();
    assert_eq!(out, b"\x1b_Ga=d,d=i,i=11109,q=2\x1b\\");
    out.clear();
    // changement de thème : tout est supprimé puis retransmis
    k.draw(&mut out, Some(bar(1.0, 1.0, 1.0)), (9, 9, 9))
        .unwrap();
    let fourth = String::from_utf8(out).unwrap();
    assert!(fourth.starts_with("\x1b_Ga=d,d=A,q=2\x1b\\\x1b_Ga=t"));
}

#[test]
fn block_and_off_are_not_images() {
    let mut k = KittyCaret::new(CELL);
    let mut out = Vec::new();
    let block = CaretFrame {
        style: CaretStyle::Block,
        ..bar(1.0, 1.0, 1.0)
    };
    k.draw(&mut out, Some(block), (1, 2, 3)).unwrap();
    assert!(out.is_empty());
}
