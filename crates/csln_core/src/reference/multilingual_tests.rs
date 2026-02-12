/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#[cfg(test)]
mod tests {
    use crate::options::{Config, MultilingualMode};
    use crate::reference::contributor::MultilingualName;
    use crate::reference::types::Title;

    #[test]
    fn test_multilingual_title_deserialization() {
        let yaml = r#"
original: "战争与和平"
lang: "zh"
transliterations:
  zh-Latn-pinyin: "Zhànzhēng yǔ Hépíng"
translations:
  en: "War and Peace"
"#;
        let title: Title = serde_yaml::from_str(yaml).unwrap();
        if let Title::Multilingual(m) = title {
            assert_eq!(m.original, "战争与和平");
            assert_eq!(m.lang, Some("zh".to_string()));
            assert_eq!(m.translations.get("en").unwrap(), "War and Peace");
        } else {
            panic!("Expected Title::Multilingual");
        }
    }

    #[test]
    fn test_multilingual_contributor_deserialization() {
        let yaml = r#"
original:
  family: "Tolstoy"
  given: "Leo"
lang: "ru"
transliterations:
  Latn:
    family: "Tolstoy"
    given: "Leo"
"#;
        let name: MultilingualName = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(name.original.family.to_string(), "Tolstoy");
        assert_eq!(name.lang, Some("ru".to_string()));
        assert!(name.transliterations.contains_key("Latn"));
    }

    #[test]
    fn test_multilingual_style_options() {
        let yaml = r#"
multilingual:
  title-mode: "transliterated"
  name-mode: "combined"
  preferred-script: "Latn"
  scripts:
    cjk:
      use-native-ordering: true
      delimiter: ""
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let mlt = config.multilingual.unwrap();
        assert_eq!(mlt.title_mode, Some(MultilingualMode::Transliterated));
        assert_eq!(mlt.name_mode, Some(MultilingualMode::Combined));
        assert!(mlt.scripts.get("cjk").unwrap().use_native_ordering);
    }

    #[test]
    fn test_multiple_transliteration_methods() {
        let yaml = r#"
original: "東京"
lang: "ja"
transliterations:
  ja-Latn-hepburn: "Tōkyō"
  ja-Latn-kunrei: "Tôkyô"
translations:
  en: "Tokyo"
"#;
        let title: Title = serde_yaml::from_str(yaml).unwrap();
        if let Title::Multilingual(m) = title {
            assert_eq!(m.original, "東京");
            assert_eq!(m.transliterations.get("ja-Latn-hepburn").unwrap(), "Tōkyō");
            assert_eq!(m.transliterations.get("ja-Latn-kunrei").unwrap(), "Tôkyô");
        } else {
            panic!("Expected Title::Multilingual");
        }
    }
}
