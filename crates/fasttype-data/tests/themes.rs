use fasttype_data::themes::{Rgba, Theme, parse_themes_ts};

const SAMPLE: &str = r##"import { z } from "zod";

export const themes: Record<ThemeName, Theme> = {
  serika_dark: {
    bg: "#323437",
    main: "#e2b714",
    caret: "#e2b714",
    sub: "#646669",
    subAlt: "#2c2e31",
    text: "#d1d0c5",
    error: "#ca4754",
    errorExtra: "#7e2a33",
    colorfulError: "#ca4754",
    colorfulErrorExtra: "#7e2a33",
  },
  "8008": {
    bg: "#333a45",
    caret: "#f44c7f",
    main: "#f44c7f",
    sub: "#1c82adc4",
    subAlt: "#2e343d",
    text: "#e9ecf0",
    error: "#da3333",
    errorExtra: "#791717",
    colorfulError: "#c5da33",
    colorfulErrorExtra: "#849224",
    hasCss: true,
  },
};

export type ThemeWithName = Theme & { name: ThemeName };
"##;

#[test]
fn parses_all_hex_forms() {
    assert_eq!(
        Rgba::parse_hex("#abc"),
        Some(Rgba {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc,
            a: 255
        })
    );
    assert_eq!(
        Rgba::parse_hex("#abcd"),
        Some(Rgba {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc,
            a: 0xdd
        })
    );
    assert_eq!(
        Rgba::parse_hex("#E2B714"),
        Some(Rgba {
            r: 0xe2,
            g: 0xb7,
            b: 0x14,
            a: 255
        })
    );
    assert_eq!(
        Rgba::parse_hex("#1c82adc4"),
        Some(Rgba {
            r: 0x1c,
            g: 0x82,
            b: 0xad,
            a: 0xc4
        })
    );
    for bad in ["#12", "e2b714", "#ggg", "#12345", ""] {
        assert_eq!(Rgba::parse_hex(bad), None, "{bad}");
    }
}

#[test]
fn hex_roundtrip() {
    assert_eq!(Rgba::parse_hex("#e2b714").unwrap().to_hex(), "#e2b714");
    assert_eq!(Rgba::parse_hex("#1c82adc4").unwrap().to_hex(), "#1c82adc4");
}

#[test]
fn alpha_is_composited_over_background() {
    let black = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    let half_red = Rgba {
        r: 255,
        g: 0,
        b: 0,
        a: 128,
    };
    assert_eq!(
        half_red.over(black),
        Rgba {
            r: 128,
            g: 0,
            b: 0,
            a: 255
        }
    );
    let opaque = Rgba::parse_hex("#e2b714").unwrap();
    assert_eq!(opaque.over(black), opaque);
}

#[test]
fn parses_themes_file_sorted_by_name() {
    let themes = parse_themes_ts(SAMPLE).unwrap();
    let names: Vec<&str> = themes.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["8008", "serika_dark"]);
    let serika = &themes[1];
    assert_eq!(serika.main.to_hex(), "#e2b714");
    assert_eq!(serika.sub_alt.to_hex(), "#2c2e31");
    assert!(!serika.has_css);
    assert!(themes[0].has_css);
    assert_eq!(themes[0].sub.a, 0xc4);
}

#[test]
fn unknown_field_is_an_error_with_line() {
    let src = SAMPLE.replace(
        "    subAlt: \"#2c2e31\",",
        "    subAlt: \"#2c2e31\",\n    glow: \"#ffffff\",",
    );
    let err = parse_themes_ts(&src).unwrap_err();
    assert!(err.message.contains("glow"), "{err}");
    assert!(err.line > 0);
}

#[test]
fn missing_color_is_an_error() {
    let src = SAMPLE.replacen("    caret: \"#e2b714\",\n", "", 1);
    assert!(parse_themes_ts(&src).unwrap_err().message.contains("caret"));
}

#[test]
fn missing_block_is_an_error() {
    assert!(parse_themes_ts("export const other = {};").is_err());
    let unterminated = SAMPLE.replace("};\n\nexport type", "\nexport type");
    assert!(parse_themes_ts(&unterminated).is_err());
}

#[test]
fn theme_json_roundtrip() {
    let themes = parse_themes_ts(SAMPLE).unwrap();
    let json = serde_json::to_string(&themes).unwrap();
    assert!(json.contains("\"subAlt\":\"#2c2e31\""), "{json}");
    assert_eq!(serde_json::from_str::<Vec<Theme>>(&json).unwrap(), themes);
}
