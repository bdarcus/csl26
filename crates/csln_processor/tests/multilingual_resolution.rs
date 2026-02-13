/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Integration tests for multilingual string and name resolution.
//!
//! Tests the BCP 47 fallback logic and mode-based rendering.

use csln_core::options::{Config, MultilingualMode};
use csln_core::reference::contributor::{Contributor, MultilingualName, StructuredName};
use csln_core::reference::types::{MultilingualComplex, MultilingualString};
use csln_processor::values::resolve_multilingual_string;
use std::collections::HashMap;

#[test]
fn test_resolve_simple_string() {
    let simple = MultilingualString::Simple("Hello".to_string());
    let result = resolve_multilingual_string(&simple, None, None, "en");
    assert_eq!(result, "Hello");
}

#[test]
fn test_resolve_primary_mode() {
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".to_string()),
        transliterations: {
            let mut map = HashMap::new();
            map.insert(
                "zh-Latn-pinyin".to_string(),
                "Zhànzhēng yǔ Hépíng".to_string(),
            );
            map
        },
        translations: {
            let mut map = HashMap::new();
            map.insert("en".to_string(), "War and Peace".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);
    let result =
        resolve_multilingual_string(&ml_string, Some(&MultilingualMode::Primary), None, "en");

    assert_eq!(result, "战争与和平");
}

#[test]
fn test_resolve_transliterated_exact_match() {
    let complex = MultilingualComplex {
        original: "東京".to_string(),
        lang: Some("ja".to_string()),
        transliterations: {
            let mut map = HashMap::new();
            map.insert("ja-Latn-hepburn".to_string(), "Tōkyō".to_string());
            map.insert("ja-Latn-kunrei".to_string(), "Tôkyô".to_string());
            map
        },
        translations: {
            let mut map = HashMap::new();
            map.insert("en".to_string(), "Tokyo".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    // Exact match for hepburn
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Transliterated),
        Some(&"ja-Latn-hepburn".to_string()),
        "en",
    );
    assert_eq!(result, "Tōkyō");
}

#[test]
fn test_resolve_transliterated_prefix_match() {
    let complex = MultilingualComplex {
        original: "東京".to_string(),
        lang: Some("ja".to_string()),
        transliterations: {
            let mut map = HashMap::new();
            map.insert("ja-Latn-hepburn".to_string(), "Tōkyō".to_string());
            map
        },
        translations: HashMap::new(),
    };

    let ml_string = MultilingualString::Complex(complex);

    // Prefix match: "ja-Latn" should match "ja-Latn-hepburn"
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Transliterated),
        Some(&"ja-Latn".to_string()),
        "en",
    );
    assert_eq!(result, "Tōkyō");
}

#[test]
fn test_resolve_transliterated_fallback_to_original() {
    let complex = MultilingualComplex {
        original: "东京".to_string(),
        lang: Some("zh".to_string()),
        transliterations: HashMap::new(), // No transliterations available
        translations: HashMap::new(),
    };

    let ml_string = MultilingualString::Complex(complex);

    // Should fallback to original
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Transliterated),
        Some(&"Latn".to_string()),
        "en",
    );
    assert_eq!(result, "东京");
}

#[test]
fn test_resolve_translated_mode() {
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".to_string()),
        transliterations: HashMap::new(),
        translations: {
            let mut map = HashMap::new();
            map.insert("en".to_string(), "War and Peace".to_string());
            map.insert("fr".to_string(), "Guerre et Paix".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    // English translation
    let result =
        resolve_multilingual_string(&ml_string, Some(&MultilingualMode::Translated), None, "en");
    assert_eq!(result, "War and Peace");

    // French translation
    let result =
        resolve_multilingual_string(&ml_string, Some(&MultilingualMode::Translated), None, "fr");
    assert_eq!(result, "Guerre et Paix");
}

#[test]
fn test_resolve_combined_mode() {
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".to_string()),
        transliterations: {
            let mut map = HashMap::new();
            map.insert(
                "zh-Latn-pinyin".to_string(),
                "Zhànzhēng yǔ Hépíng".to_string(),
            );
            map
        },
        translations: {
            let mut map = HashMap::new();
            map.insert("en".to_string(), "War and Peace".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Combined),
        Some(&"zh-Latn-pinyin".to_string()),
        "en",
    );

    assert_eq!(result, "Zhànzhēng yǔ Hépíng [War and Peace]");
}

#[test]
fn test_resolve_combined_fallback() {
    let complex = MultilingualComplex {
        original: "东京".to_string(),
        lang: Some("zh".to_string()),
        transliterations: HashMap::new(),
        translations: {
            let mut map = HashMap::new();
            map.insert("en".to_string(), "Tokyo".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    // No transliteration, should use original + translation
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Combined),
        Some(&"Latn".to_string()),
        "en",
    );

    assert_eq!(result, "东京 [Tokyo]");
}

#[test]
fn test_resolve_multilingual_name_simple() {
    let name = Contributor::StructuredName(StructuredName {
        given: MultilingualString::Simple("John".to_string()),
        family: MultilingualString::Simple("Smith".to_string()),
        suffix: None,
        dropping_particle: None,
        non_dropping_particle: None,
    });

    let result = csln_processor::values::resolve_multilingual_name(&name, None, None, "en");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("John".to_string()));
    assert_eq!(result[0].family, Some("Smith".to_string()));
}

#[test]
fn test_resolve_multilingual_name_transliterated() {
    let name = Contributor::Multilingual(MultilingualName {
        original: StructuredName {
            given: MultilingualString::Simple("Лев".to_string()),
            family: MultilingualString::Simple("Толстой".to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
        lang: Some("ru".to_string()),
        transliterations: {
            let mut map = HashMap::new();
            map.insert(
                "Latn".to_string(),
                StructuredName {
                    given: MultilingualString::Simple("Leo".to_string()),
                    family: MultilingualString::Simple("Tolstoy".to_string()),
                    suffix: None,
                    dropping_particle: None,
                    non_dropping_particle: None,
                },
            );
            map
        },
        translations: HashMap::new(),
    });

    let result = csln_processor::values::resolve_multilingual_name(
        &name,
        Some(&MultilingualMode::Transliterated),
        Some(&"Latn".to_string()),
        "en",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("Leo".to_string()));
    assert_eq!(result[0].family, Some("Tolstoy".to_string()));
}

#[test]
fn test_resolve_multilingual_name_prefix_match() {
    let name = Contributor::Multilingual(MultilingualName {
        original: StructuredName {
            given: MultilingualString::Simple("Лев".to_string()),
            family: MultilingualString::Simple("Толстой".to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
        lang: Some("ru".to_string()),
        transliterations: {
            let mut map = HashMap::new();
            map.insert(
                "ru-Latn-alalc97".to_string(),
                StructuredName {
                    given: MultilingualString::Simple("Lev".to_string()),
                    family: MultilingualString::Simple("Tolstoi".to_string()),
                    suffix: None,
                    dropping_particle: None,
                    non_dropping_particle: None,
                },
            );
            map
        },
        translations: HashMap::new(),
    });

    // Prefix "Latn" should match "ru-Latn-alalc97"
    let result = csln_processor::values::resolve_multilingual_name(
        &name,
        Some(&MultilingualMode::Transliterated),
        Some(&"Latn".to_string()),
        "en",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("Lev".to_string()));
    assert_eq!(result[0].family, Some("Tolstoi".to_string()));
}

#[test]
fn test_resolve_multilingual_name_fallback_to_original() {
    let name = Contributor::Multilingual(MultilingualName {
        original: StructuredName {
            given: MultilingualString::Simple("Лев".to_string()),
            family: MultilingualString::Simple("Толстой".to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
        lang: Some("ru".to_string()),
        transliterations: HashMap::new(),
        translations: HashMap::new(),
    });

    // No transliterations available, should use original
    let result = csln_processor::values::resolve_multilingual_name(
        &name,
        Some(&MultilingualMode::Transliterated),
        Some(&"Latn".to_string()),
        "en",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("Лев".to_string()));
    assert_eq!(result[0].family, Some("Толстой".to_string()));
}

#[test]
fn test_multilingual_config_deserialization() {
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
    assert_eq!(mlt.preferred_script, Some("Latn".to_string()));

    let cjk_config = mlt.scripts.get("cjk").unwrap();
    assert!(cjk_config.use_native_ordering);
    assert_eq!(cjk_config.delimiter, Some("".to_string()));
}
