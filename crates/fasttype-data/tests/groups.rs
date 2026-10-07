use fasttype_data::groups::{LanguageGroup, parse_language_groups_ts};

const SAMPLE: &str = r#"import { Language, LanguageSchema } from "@monkeytype/schemas/languages";

export const LanguageList: Language[] = LanguageSchema._def.values;

export const LanguageGroups: Record<string, Language[]> = {
  english: [
    "english",
    "english_1k",
  ],
  spanish: ["spanish", "spanish_1k"],
  code: ["code_python"],
};

export type LanguageGroupName = keyof typeof LanguageGroups;
"#;

fn group(name: &str, languages: &[&str]) -> LanguageGroup {
    LanguageGroup {
        name: name.into(),
        languages: languages.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn parses_groups_in_source_order() {
    assert_eq!(
        parse_language_groups_ts(SAMPLE).unwrap(),
        [
            group("english", &["english", "english_1k"]),
            group("spanish", &["spanish", "spanish_1k"]),
            group("code", &["code_python"])
        ]
    );
}

#[test]
fn unexpected_shape_is_an_error() {
    assert!(parse_language_groups_ts("export const X = {};").is_err());
    let bad = SAMPLE.replace("    \"english_1k\",", "    english_1k,");
    assert!(
        parse_language_groups_ts(&bad)
            .unwrap_err()
            .to_string()
            .contains("english_1k")
    );
    let unterminated = SAMPLE.replace("};\n\nexport type", "\nexport type");
    assert!(parse_language_groups_ts(&unterminated).is_err());
}
