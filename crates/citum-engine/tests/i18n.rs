/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in test, benchmark, and example code."
)]

/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

mod common;
use common::*;

use citum_engine::Processor;
use citum_engine::values::{
    effective_field_language, effective_item_language, resolve_multilingual_string,
};
use citum_schema::{
    BibliographySpec, CitationSpec, LocalizedTemplateSpec, Style, StyleBase, StyleInfo,
    options::{
        Config, ContributorConfig, MultilingualConfig, MultilingualMode, Processing,
        RoleLabelPreset, RoleOptions, RoleRendering, TitleRendering,
    },
    reference::contributor::{Contributor, MultilingualName, StructuredName},
    reference::types::{
        Collection, CollectionComponent, MultilingualComplex, MultilingualString, Title,
    },
    reference::{
        DateValue, InputReference, Monograph, MonographType, Serial, SerialComponent,
        SerialComponentType, SerialType, WorkRelation,
    },
    template::{
        ContributorForm, ContributorRole, DateForm, NameOrder, Rendering, TemplateContributor,
        TemplateDate, TemplateTitle, TitleType,
    },
};
use rstest::rstest;
use std::collections::HashMap;

#[rstest]
#[case(
    "en-US",
    Some("tr-TR"),
    "istanbul ızmir",
    "uppercase",
    "İSTANBUL IZMİR"
)]
#[case("az-Latn-AZ", None, "İSTANBUL IĞDIR", "lowercase", "istanbul ığdır")]
fn given_item_language_when_title_case_is_explicit_then_mapping_is_locale_tailored(
    #[case] item_language: &str,
    #[case] field_language: Option<&str>,
    #[case] title: &str,
    #[case] text_case: &str,
    #[case] expected: &str,
) {
    let style: Style = serde_yaml::from_str(&format!(
        r#"
info:
  title: Locale-tailored title casing
bibliography:
  template:
    - title: primary
      text-case: {text_case}
"#,
    ))
    .unwrap();
    let mut reference_json = serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "item",
        "title": title,
        "language": item_language
    });
    if let Some(field_language) = field_language {
        reference_json["field-languages"] = serde_json::json!({ "title": field_language });
    }
    let reference: InputReference = serde_json::from_value(reference_json).unwrap();
    let processor = Processor::new(
        style,
        indexmap::indexmap! { "item".to_string() => reference },
    );

    assert_eq!(processor.render_bibliography(), expected);
}

#[rstest]
#[case("- variable: publisher-place", "en-US", "tr-TR", "ISTANBUL")]
#[case("- variable: publisher-place", "tr_TR", "en-US", "İSTANBUL")]
#[case("- number: edition", "en-US", "tr-TR", "ISTANBUL")]
#[case("- number: edition", "tr_TR", "en-US", "İSTANBUL")]
fn given_title_and_entry_languages_when_casing_non_title_fields_then_entry_language_wins(
    #[case] component: &str,
    #[case] item_language: &str,
    #[case] title_language: &str,
    #[case] expected: &str,
) {
    let style: Style = serde_yaml::from_str(&format!(
        r#"
info:
  title: Field-scoped locale casing
bibliography:
  template:
    {component}
      text-case: uppercase
"#,
    ))
    .unwrap();
    let reference: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "item",
        "title": "istanbul",
        "publisher": { "name": "publisher", "place": "istanbul" },
        "edition": "istanbul",
        "language": item_language,
        "field-languages": { "title": title_language }
    }))
    .unwrap();
    let processor = Processor::new(
        style,
        indexmap::indexmap! { "item".to_string() => reference },
    );

    assert_eq!(processor.render_bibliography(), expected);
}

#[test]
fn turkish_title_case_preserves_djot_nocase_spans() {
    let style: Style = serde_yaml::from_str(
        r#"
info:
  title: Locale-tailored protected title casing
bibliography:
  template:
    - title: primary
      text-case: uppercase
"#,
    )
    .unwrap();
    let reference: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "item",
        "title": "istanbul [izmir]{.nocase}",
        "language": "tr-TR"
    }))
    .unwrap();
    let processor = Processor::new(
        style,
        indexmap::indexmap! { "item".to_string() => reference },
    );

    assert_eq!(processor.render_bibliography(), "İSTANBUL izmir");
}

#[rstest]
#[case("- term: in\n      text-case: uppercase", "İÇİNDE")]
#[case("- message: custom.case-test\n      text-case: uppercase", "İSTANBUL")]
fn given_turkish_locale_when_locale_text_is_uppercased_then_mapping_is_locale_tailored(
    #[case] component: &str,
    #[case] expected: &str,
) {
    let style: Style = serde_yaml::from_str(&format!(
        r#"
info:
  title: Locale-tailored locale casing
options:
  messages:
    custom.case-test: istanbul
bibliography:
  template:
    {component}
"#,
    ))
    .unwrap();
    let reference: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "item",
        "title": "unused"
    }))
    .unwrap();
    let tr_yaml = std::str::from_utf8(
        citum_schema::embedded::get_locale_bytes("tr-TR").expect("tr-TR must be embedded"),
    )
    .unwrap();
    let processor = Processor::with_locale(
        style,
        indexmap::indexmap! { "item".to_string() => reference },
        citum_schema::Locale::from_yaml_str(tr_yaml).unwrap(),
    );

    assert_eq!(processor.render_bibliography(), expected);
}

#[test]
fn localized_citation_layouts_select_their_term_locale() {
    let style: Style = serde_yaml::from_str(
        r#"
info:
  title: Localized terms
citation:
  locales:
    - locale: [en]
      template:
        - term: et-al
    - default: true
      template:
        - term: et-al
"#,
    )
    .unwrap();
    let mut bibliography = indexmap::IndexMap::new();
    for (id, language) in [("english", "en-US"), ("chinese", "zh-CN")] {
        let reference: InputReference = serde_json::from_value(serde_json::json!({
            "class": "monograph",
            "type": "book",
            "id": id,
            "title": id,
            "language": language
        }))
        .unwrap();
        bibliography.insert(id.to_string(), reference);
    }
    let zh_yaml = std::str::from_utf8(
        citum_schema::embedded::get_locale_bytes("zh-CN").expect("zh-CN must be embedded"),
    )
    .unwrap();
    let processor = Processor::with_locale(
        style,
        bibliography,
        citum_schema::Locale::from_yaml_str(zh_yaml).unwrap(),
    );

    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("english"))
            .unwrap(),
        "et al."
    );
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("chinese"))
            .unwrap(),
        "等"
    );
}

#[test]
fn localized_author_date_grouping_uses_the_selected_term_locale() {
    let style: Style = serde_yaml::from_str(
        r#"
info:
  title: Localized grouped contributors
options:
  processing: author-date
citation:
  locales:
    - locale: [en]
      template:
        - contributor: author
          form: short
          shorten: { min: 2, use-first: 1, and-others: et-al }
    - default: true
      template:
        - contributor: author
          form: short
          shorten: { min: 2, use-first: 1, and-others: et-al }
"#,
    )
    .unwrap();
    let reference: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "english",
        "title": "English item",
        "language": "en-US",
        "author": [
            { "family": "Smith", "given": "Alice" },
            { "family": "Jones", "given": "Bob" }
        ]
    }))
    .unwrap();
    let zh_yaml = std::str::from_utf8(
        citum_schema::embedded::get_locale_bytes("zh-CN").expect("zh-CN must be embedded"),
    )
    .unwrap();
    let processor = Processor::with_locale(
        style,
        indexmap::indexmap! { "english".to_string() => reference },
        citum_schema::Locale::from_yaml_str(zh_yaml).unwrap(),
    );

    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("english"))
            .unwrap(),
        "Smith et al."
    );
}

#[test]
fn supplementary_identifiers_render_and_missing_values_suppress() {
    let style: Style = serde_yaml::from_str(
        r#"
info:
  title: Supplementary identifier
bibliography:
  template:
    - identifier: cstr
      prefix: "CSTR: "
      suffix: "."
"#,
    )
    .unwrap();

    let with_identifier: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "present",
        "title": "Present",
        "identifiers": { "cstr": "32012.36.1001024.2023.0328" }
    }))
    .unwrap();
    let without_identifier: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "missing",
        "title": "Missing"
    }))
    .unwrap();

    let present = Processor::new(
        style.clone(),
        indexmap::IndexMap::from([("present".to_string(), with_identifier)]),
    );
    assert_eq!(
        present.render_bibliography(),
        "CSTR: 32012.36.1001024.2023.0328."
    );

    let missing = Processor::new(
        style,
        indexmap::IndexMap::from([("missing".to_string(), without_identifier)]),
    );
    assert_eq!(missing.render_bibliography(), "");
}

// --- Helper Functions ---

fn build_ml_style(name_mode: MultilingualMode, preferred_script: Option<String>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Multilingual Test".to_string()),
            id: Some("ml-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            multilingual: Some(MultilingualConfig {
                name_mode: Some(name_mode),
                preferred_script,
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Short),
                    citum_schema::tc_date!(Issued, Year),
                ]
                .into(),
            ),
            delimiter: Some(", ".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_german_translator_role_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Chicago German Translator Test".to_string()),
            id: Some("chicago-de-translator".into()),
            default_locale: Some("de-DE".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            locale_override: Some("de-DE-chicago".to_string()),
            contributors: Some(ContributorConfig {
                role: Some(RoleOptions {
                    roles: Some(HashMap::from([(
                        "translator".to_string(),
                        RoleRendering {
                            preset: Some(RoleLabelPreset::VerbPrefix),
                            ..Default::default()
                        },
                    )])),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::template::TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author.into(),
                        form: ContributorForm::Long,
                        ..Default::default()
                    }),
                    citum_schema::template::TemplateComponent::Date(TemplateDate {
                        date: citum_schema::template::DateVariable::Issued,
                        form: DateForm::Year,
                        rendering: Rendering {
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    citum_schema::template::TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        rendering: Rendering {
                            emph: Some(true),
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    citum_schema::template::TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Translator.into(),
                        form: ContributorForm::Long,
                        name_order: Some(NameOrder::GivenFirst),
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_german_translator_reference() -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("ref1".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Das Buch".to_string())),
        container: None,
        author: Some(Contributor::StructuredName(StructuredName {
            given: "Autor".into(),
            family: "Name".into(),
            ..Default::default()
        })),
        editor: None,
        translator: Some(Contributor::StructuredName(StructuredName {
            given: "Ubersetzer".into(),
            family: "Name".into(),
            ..Default::default()
        })),
        issued: DateValue::new("2024".to_string()),
        language: Some("de".into()),
        ..Default::default()
    }))
}

// --- Multilingual Resolution Tests ---

fn given_a_simple_string_when_resolved_then_the_original_text_is_returned() {
    let simple = MultilingualString::Simple("Hello".to_string());
    let result = resolve_multilingual_string(&simple, None, None, None, "en");
    assert_eq!(result, "Hello");
}

fn given_primary_mode_when_resolving_a_multilingual_title_then_the_original_script_is_returned() {
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
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
            map.insert("en".into(), "War and Peace".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Primary),
        None,
        None,
        "en",
    );

    assert_eq!(result, "战争与和平");
}

fn given_an_exact_transliteration_match_when_resolving_then_that_transliteration_is_used() {
    let complex = MultilingualComplex {
        original: "東京".to_string(),
        lang: Some("ja".into()),
        sort_as: None,
        transliterations: {
            let mut map = HashMap::new();
            map.insert("ja-Latn-hepburn".to_string(), "Tōkyō".to_string());
            map.insert("ja-Latn-kunrei".to_string(), "Tôkyô".to_string());
            map
        },
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "Tokyo".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    // Exact match for hepburn
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Transliterated),
        Some(&["ja-Latn-hepburn".to_string()]),
        None,
        "en",
    );
    assert_eq!(result, "Tōkyō");
}

fn given_a_transliteration_prefix_match_when_resolving_then_the_matching_transliteration_is_used() {
    let complex = MultilingualComplex {
        original: "東京".to_string(),
        lang: Some("ja".into()),
        sort_as: None,
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
        Some(&["ja-Latn".to_string()]),
        None,
        "en",
    );
    assert_eq!(result, "Tōkyō");
}

fn given_no_transliteration_when_transliterated_mode_is_requested_then_the_original_text_is_used() {
    let complex = MultilingualComplex {
        original: "东京".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
        transliterations: HashMap::new(), // No transliterations available
        translations: HashMap::new(),
    };

    let ml_string = MultilingualString::Complex(complex);

    // Should fallback to original
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Transliterated),
        None,
        Some(&"Latn".to_string()),
        "en",
    );
    assert_eq!(result, "东京");
}

fn given_translated_mode_when_resolving_then_the_requested_locale_translation_is_used() {
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
        transliterations: HashMap::new(),
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "War and Peace".to_string());
            map.insert("fr".into(), "Guerre et Paix".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    // English translation
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Translated),
        None,
        None,
        "en",
    );
    assert_eq!(result, "War and Peace");

    // French translation
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Translated),
        None,
        None,
        "fr",
    );
    assert_eq!(result, "Guerre et Paix");
}

fn given_translated_mode_with_region_locale_when_resolving_then_the_base_language_translation_is_used()
 {
    let complex = MultilingualComplex {
        original: "引用の社会的機能".to_string(),
        lang: Some("ja".into()),
        sort_as: None,
        transliterations: HashMap::new(),
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "The Social Function of Citation".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Translated),
        None,
        None,
        "en-US",
    );

    assert_eq!(result, "The Social Function of Citation");
}

fn given_combined_mode_when_transliteration_and_translation_exist_then_both_are_combined() {
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
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
            map.insert("en".into(), "War and Peace".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Combined),
        Some(&["zh-Latn-pinyin".to_string()]),
        None,
        "en",
    );

    assert_eq!(result, "Zhànzhēng yǔ Hépíng [War and Peace]");
}

fn given_combined_mode_with_preferred_script_and_region_locale_then_transliteration_and_base_translation_are_combined()
 {
    let complex = MultilingualComplex {
        original: "引用の社会的機能".to_string(),
        lang: Some("ja".into()),
        sort_as: None,
        transliterations: {
            let mut map = HashMap::new();
            map.insert(
                "ja-Latn".to_string(),
                "In'yo no shakaiteki kino".to_string(),
            );
            map
        },
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "The Social Function of Citation".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Combined),
        None,
        Some(&"Latn".to_string()),
        "en-US",
    );

    assert_eq!(
        result,
        "In'yo no shakaiteki kino [The Social Function of Citation]"
    );
}

fn given_combined_mode_without_transliteration_when_resolving_then_original_and_translation_are_combined()
 {
    let complex = MultilingualComplex {
        original: "东京".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
        transliterations: HashMap::new(),
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "Tokyo".to_string());
            map
        },
    };

    let ml_string = MultilingualString::Complex(complex);

    // No transliteration, should use original + translation
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&MultilingualMode::Combined),
        None,
        Some(&"Latn".to_string()),
        "en",
    );

    assert_eq!(result, "东京 [Tokyo]");
}

fn given_a_simple_structured_name_when_resolved_then_the_name_parts_are_preserved() {
    let name = Contributor::StructuredName(StructuredName {
        given: MultilingualString::Simple("John".to_string()),
        family: MultilingualString::Simple("Smith".to_string()),
        suffix: None,
        dropping_particle: None,
        non_dropping_particle: None,
    });

    let result = citum_engine::values::resolve_multilingual_name(&name, None, None, None, "en");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("John".to_string()));
    assert_eq!(result[0].family, Some("Smith".to_string()));
}

fn given_a_multilingual_name_with_requested_script_when_resolved_then_the_transliterated_name_is_used()
 {
    let name = Contributor::Multilingual(MultilingualName {
        original: StructuredName {
            given: MultilingualString::Simple("Лев".to_string()),
            family: MultilingualString::Simple("Толстой".to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
        lang: Some("ru".into()),
        sort_as: None,
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

    let result = citum_engine::values::resolve_multilingual_name(
        &name,
        Some(&MultilingualMode::Transliterated),
        Some(&["Latn".to_string()]),
        None,
        "en",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("Leo".to_string()));
    assert_eq!(result[0].family, Some("Tolstoy".to_string()));
}

fn given_a_multilingual_name_with_a_script_prefix_match_when_resolved_then_the_matching_transliteration_is_used()
 {
    let name = Contributor::Multilingual(MultilingualName {
        original: StructuredName {
            given: MultilingualString::Simple("Лев".to_string()),
            family: MultilingualString::Simple("Толстой".to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
        lang: Some("ru".into()),
        sort_as: None,
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
    let result = citum_engine::values::resolve_multilingual_name(
        &name,
        Some(&MultilingualMode::Transliterated),
        Some(&["Latn".to_string()]),
        None,
        "en",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("Lev".to_string()));
    assert_eq!(result[0].family, Some("Tolstoi".to_string()));
}

fn given_a_multilingual_name_without_transliterations_when_resolved_then_the_original_name_is_used()
{
    let name = Contributor::Multilingual(MultilingualName {
        original: StructuredName {
            given: MultilingualString::Simple("Лев".to_string()),
            family: MultilingualString::Simple("Толстой".to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
        lang: Some("ru".into()),
        sort_as: None,
        transliterations: HashMap::new(),
        translations: HashMap::new(),
    });

    // No transliterations available, should use original
    let result = citum_engine::values::resolve_multilingual_name(
        &name,
        Some(&MultilingualMode::Transliterated),
        Some(&["Latn".to_string()]),
        None,
        "en",
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].given, Some("Лев".to_string()));
    assert_eq!(result[0].family, Some("Толстой".to_string()));
}

fn given_multilingual_yaml_options_when_deserialized_then_the_config_keeps_script_preferences() {
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
    assert_eq!(cjk_config.delimiter, Some(String::new()));
}

// --- Multilingual Rendering Tests ---

#[rstest]
#[case::primary(MultilingualMode::Primary, None, "東京, 2020")]
#[case::transliterated(
    MultilingualMode::Transliterated,
    Some("Latn".to_string()),
    "Tokyo, 2020"
)]
#[case::combined(
    MultilingualMode::Combined,
    Some("Latn".to_string()),
    "Tokyo, 2020"
)]
#[rstest]
fn given_a_multilingual_author_when_rendering_a_citation_then_the_selected_name_mode_controls_the_output(
    #[case] mode: MultilingualMode,
    #[case] preferred_script: Option<String>,
    #[case] expected: &str,
) {
    announce_behavior(&format!(
        "A multilingual citation should render author names according to {:?} mode{}.",
        mode,
        preferred_script
            .as_deref()
            .map(|script| format!(" with preferred script {script}"))
            .unwrap_or_default()
    ));

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_multilingual_book(common::MultilingualBookParams {
            id: "item1",
            original_family: "東京",
            original_given: "太郎",
            lang: "ja",
            translit_script: "ja-Latn",
            translit_family: "Tokyo",
            translit_given: "Taro",
            year: 2020,
            title: "Title",
        }),
    );

    // Given a multilingual citation style and a single Japanese reference.
    let style = build_ml_style(mode, preferred_script);
    let processor = Processor::new(style, bib);

    // When the citation is rendered.
    let rendered = processor
        .process_citation(&citum_schema::cite!("item1"))
        .unwrap();

    // Then the selected name mode controls the visible author text.
    assert_eq!(rendered, expected);
}

fn given_translated_numeric_integral_citations_when_rendered_then_the_translated_name_is_used_as_the_anchor()
 {
    let mut style = build_ml_style(MultilingualMode::Translated, None);
    style.options.as_mut().unwrap().processing = Some(Processing::Numeric);
    style.citation.as_mut().unwrap().template =
        Some(vec![citum_schema::tc_contributor!(Author, Short)].into());

    let mut bib = indexmap::IndexMap::new();
    let mut translations = HashMap::new();
    translations.insert(
        "en-US".into(),
        StructuredName {
            family: MultilingualString::Simple("Tolstoy".to_string()),
            given: MultilingualString::Simple("Leo".to_string()),
            ..Default::default()
        },
    );

    bib.insert(
        "item1".to_string(),
        citum_schema::reference::InputReference::Monograph(Box::new(
            citum_schema::reference::Monograph {
                short_title: None,
                id: Some("item1".into()),
                r#type: citum_schema::reference::MonographType::Book,
                title: Some(citum_schema::reference::Title::Single(
                    "War and Peace".to_string(),
                )),
                container: None,
                author: Some(Contributor::Multilingual(MultilingualName {
                    original: StructuredName {
                        family: MultilingualString::Simple("Толстой".to_string()),
                        given: MultilingualString::Simple("Лев".to_string()),
                        ..Default::default()
                    },
                    lang: Some("ru".into()),
                    sort_as: None,
                    transliterations: HashMap::new(),
                    translations,
                })),
                editor: None,
                translator: None,
                issued: citum_schema::reference::DateValue::new("1869".to_string()),
                ..Default::default()
            },
        )),
    );

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!(
                "item1",
                mode = citum_schema::citation::CitationMode::Integral
            ))
            .unwrap(),
        "Tolstoy [1]"
    );
}

fn given_field_language_overrides_when_resolving_the_effective_field_language_then_the_field_override_wins()
 {
    let reference = InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("item1".into()),
        r#type: MonographType::Book,
        title: Some(Title::Multilingual(MultilingualComplex {
            original: "Titel".to_string(),
            lang: Some("de".into()),
            sort_as: None,
            transliterations: HashMap::new(),
            translations: HashMap::new(),
        })),
        container: None,
        author: None,
        editor: None,
        translator: None,
        issued: DateValue::new("2024".to_string()),
        language: Some("fr".into()),
        field_languages: HashMap::from([("title".to_string(), "en".into())]),
        ..Default::default()
    }));

    assert_eq!(
        effective_field_language(&reference, "title", reference.title().as_ref()),
        Some("en".to_string())
    );
}

fn given_no_item_language_when_resolving_the_effective_item_language_then_the_multilingual_title_language_is_used()
 {
    let reference = InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("item1".into()),
        r#type: MonographType::Book,
        title: Some(Title::Multilingual(MultilingualComplex {
            original: "東京".to_string(),
            lang: Some("ja".into()),
            sort_as: None,
            transliterations: HashMap::new(),
            translations: HashMap::new(),
        })),
        container: None,
        author: None,
        editor: None,
        translator: None,
        issued: DateValue::new("2024".to_string()),
        ..Default::default()
    }));

    assert_eq!(effective_item_language(&reference), Some("ja".to_string()));
}

#[allow(
    clippy::too_many_lines,
    reason = "test functions naturally exceed 100 lines"
)]
fn given_localized_citation_templates_when_the_item_language_matches_then_the_locale_specific_template_is_selected()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Localized Citation".to_string()),
            ..Default::default()
        },
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_variable!(Note)].into()),
            locales: Some(vec![
                LocalizedTemplateSpec {
                    locale: Some(vec!["de".to_string()]),
                    default: None,
                    template: vec![citum_schema::tc_variable!(Publisher)],
                    type_variants: None,
                    unknown_fields: Default::default(),
                },
                LocalizedTemplateSpec {
                    locale: None,
                    default: Some(true),
                    template: vec![citum_schema::tc_variable!(Note)],
                    type_variants: None,
                    unknown_fields: Default::default(),
                },
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "de-item".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            short_title: None,
            id: Some("de-item".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Titel".to_string())),
            container: None,
            author: None,
            editor: None,
            translator: None,
            issued: DateValue::new("2024".to_string()),
            publisher: Some(citum_schema::reference::Publisher {
                name: citum_schema::reference::MultilingualString::Simple("Verlag".to_string()),
                place: None,
            }),
            url: None,
            accessed: None,
            language: Some("de-AT".into()),
            field_languages: HashMap::new(),
            note: Some(citum_schema::reference::RichText::Plain(
                "fallback".to_string(),
            )),
            isbn: None,
            doi: None,
            numbering: Default::default(),
            genre: None,
            medium: None,
            archive_info: None,
            eprint: None,
            archive: None,
            archive_location: None,
            keywords: None,
            original: None,
            ads_bibcode: None,
            ..Default::default()
        })),
    );
    bibliography.insert(
        "fr-item".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            short_title: None,
            id: Some("fr-item".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Titre".to_string())),
            container: None,
            author: None,
            editor: None,
            translator: None,
            issued: DateValue::new("2024".to_string()),
            publisher: Some(citum_schema::reference::Publisher {
                name: citum_schema::reference::MultilingualString::Simple("Editeur".to_string()),
                place: None,
            }),
            url: None,
            accessed: None,
            language: Some("fr".into()),
            field_languages: HashMap::new(),
            note: Some(citum_schema::reference::RichText::Plain(
                "fallback".to_string(),
            )),
            isbn: None,
            doi: None,
            numbering: Default::default(),
            genre: None,
            medium: None,
            archive_info: None,
            eprint: None,
            archive: None,
            archive_location: None,
            keywords: None,
            original: None,
            ads_bibcode: None,
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bibliography);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("de-item"))
            .unwrap(),
        "Verlag"
    );
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("fr-item"))
            .unwrap(),
        "fallback"
    );
}

fn given_localized_bibliography_templates_when_only_the_multilingual_title_has_a_language_then_that_language_still_selects_the_template()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Localized Bibliography".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(vec![citum_schema::tc_variable!(Note)].into()),
            locales: Some(vec![
                LocalizedTemplateSpec {
                    locale: Some(vec!["ja".to_string()]),
                    default: None,
                    template: vec![citum_schema::tc_title!(Primary)],
                    type_variants: None,
                    unknown_fields: Default::default(),
                },
                LocalizedTemplateSpec {
                    locale: None,
                    default: Some(true),
                    template: vec![citum_schema::tc_variable!(Note)],
                    type_variants: None,
                    unknown_fields: Default::default(),
                },
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            short_title: None,
            id: Some("item1".into()),
            r#type: MonographType::Book,
            title: Some(Title::Multilingual(MultilingualComplex {
                original: "東京".to_string(),
                lang: Some("ja".into()),
                sort_as: None,
                transliterations: HashMap::new(),
                translations: HashMap::new(),
            })),
            container: None,
            author: None,
            editor: None,
            translator: None,
            issued: DateValue::new("2024".to_string()),
            publisher: None,
            url: None,
            accessed: None,
            language: None,
            field_languages: HashMap::new(),
            note: Some(citum_schema::reference::RichText::Plain(
                "fallback".to_string(),
            )),
            isbn: None,
            doi: None,
            numbering: Default::default(),
            genre: None,
            medium: None,
            archive_info: None,
            eprint: None,
            archive: None,
            archive_location: None,
            keywords: None,
            original: None,
            ads_bibcode: None,
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bibliography);
    assert_eq!(processor.render_bibliography(), "東京");
}

fn given_mixed_language_titles_when_rendering_the_bibliography_then_field_languages_drive_the_title_formatting_overrides()
 {
    let style = Style {
        info: StyleInfo {
            title: Some("Mixed Language Titles".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            titles: Some(citum_schema::options::TitlesConfig {
                component: Some(TitleRendering {
                    quote: Some(true),
                    locale_overrides: Some(HashMap::from([(
                        "de".to_string(),
                        TitleRendering {
                            quote: Some(false),
                            emph: Some(true),
                            ..Default::default()
                        },
                    )])),
                    ..Default::default()
                }),
                container_monograph: Some(TitleRendering {
                    emph: Some(true),
                    locale_overrides: Some(HashMap::from([(
                        "en".to_string(),
                        TitleRendering {
                            emph: Some(false),
                            quote: Some(true),
                            ..Default::default()
                        },
                    )])),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(citum_schema::TemplateVariant::Full(vec![
                citum_schema::tc_title!(Primary),
                citum_schema::tc_title!(ParentMonograph),
            ])),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::CollectionComponent(Box::new(CollectionComponent {
        id: Some("chapter-1".into()),
        r#type: citum_schema::reference::MonographComponentType::Chapter,
        title: Some(Title::Single("English Article".to_string())),
        author: None,
        translator: None,
        issued: DateValue::new("2024".to_string()),
        container: Some(WorkRelation::Embedded(Box::new(
            InputReference::Collection(Box::new(Collection {
                short_title: None,
                id: None,
                r#type: citum_schema::reference::CollectionType::EditedBook,
                title: Some(Title::Single("Deutscher Sammelband".to_string())),
                container: None,
                editor: None,
                translator: None,
                issued: DateValue::new("2024".to_string()),
                publisher: None,
                numbering: Vec::new(),
                url: None,
                accessed: None,
                language: Some("de".into()),
                field_languages: HashMap::new(),
                note: None,
                isbn: None,
                keywords: None,
                ..Default::default()
            })),
        ))),
        numbering: Vec::new(),
        pages: None,
        url: None,
        accessed: None,
        language: Some("de".into()),
        field_languages: HashMap::from([
            ("title".to_string(), "en".into()),
            ("parent-monograph.title".to_string(), "de".into()),
        ]),
        note: None,
        doi: None,
        genre: None,
        medium: None,
        archive_info: None,
        eprint: None,
        keywords: None,
        original: None,
        ..Default::default()
    }));

    let bibliography = indexmap::IndexMap::from([("chapter-1".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);

    // Default processor locale is en-US, whose `punctuation-in-quote: true`
    // grammar option now supplies the style default (csl26-8e75) and moves
    // the trailing period inside the closing quote, matching citeproc-js.
    assert_eq!(
        processor.render_bibliography(),
        "“English Article.” _Deutscher Sammelband_"
    );
}

fn given_apa_multilingual_title_mode_when_rendering_a_japanese_article_then_titles_are_romanized() {
    let style = Style {
        info: StyleInfo {
            title: Some("APA Multilingual Title Test".to_string()),
            default_locale: Some("en-US".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            multilingual: Some(MultilingualConfig {
                title_mode: Some(MultilingualMode::Combined),
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::template::TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        rendering: Rendering {
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    citum_schema::template::TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentSerial,
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::SerialComponent(Box::new(SerialComponent {
        id: Some("tanaka_yuki2019".into()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Multilingual(MultilingualComplex {
            original: "引用の社会的機能：学術知識の構築における参照実践".to_string(),
            lang: Some("ja".into()),
            sort_as: None,
            transliterations: HashMap::from([(
                "ja-Latn".to_string(),
                "In'yo no shakaiteki kino: Gakujutsu chishiki no kochiku ni okeru sansho jissen"
                    .to_string(),
            )]),
            translations: HashMap::from([(
                "en".into(),
                "The Social Function of Citation: Reference Practices in the Construction of Academic Knowledge"
                    .to_string(),
            )]),
        })),
        issued: DateValue::new("2019".to_string()),
        container: Some(WorkRelation::Embedded(Box::new(InputReference::Serial(
            Box::new(Serial {
                r#type: SerialType::AcademicJournal,
                title: Some(Title::Multilingual(MultilingualComplex {
                    original: "科学社会学研究".to_string(),
                    lang: Some("ja".into()),
                    sort_as: None,
                    transliterations: HashMap::from([(
                        "ja-Latn".to_string(),
                        "Kagaku Shakai-gaku Kenkyu".to_string(),
                    )]),
                    translations: HashMap::new(),
                })),
                ..Default::default()
            }),
        )))),
        language: Some("ja".into()),
        ..Default::default()
    }));

    let bibliography = indexmap::IndexMap::from([("tanaka_yuki2019".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography();

    assert_eq!(
        rendered,
        "In’yo no shakaiteki kino: Gakujutsu chishiki no kochiku ni okeru sansho jissen [The Social Function of Citation: Reference Practices in the Construction of Academic Knowledge]. Kagaku Shakai-gaku Kenkyu"
    );
}

// ── End-to-end: Chicago + MLA multilingual rendering ─────────────────────────

/// Pattern mode: 3-way Japanese title (romanized + original + [translation]).
///
/// Verifies that `Transliterated` → `Original` → `Translated(brackets)` segment order
/// produces `romanized 原文 [translation]` and that the smart-quote transform is applied
/// to the transliterated text (ASCII `'` → U+2019 RIGHT SINGLE QUOTATION MARK).
fn chicago_pattern_renders_three_way_japanese_title() {
    use citum_schema::options::{
        MultilingualMode, MultilingualSegment, MultilingualView, SegmentWrap,
    };
    let style = Style {
        info: StyleInfo {
            title: Some("Chicago Pattern Test".to_string()),
            default_locale: Some("en-US".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            multilingual: Some(MultilingualConfig {
                title_mode: Some(MultilingualMode::Pattern(vec![
                    MultilingualSegment {
                        view: MultilingualView::Transliterated,
                        wrap: SegmentWrap::None,
                    },
                    MultilingualSegment {
                        view: MultilingualView::OriginalScript,
                        wrap: SegmentWrap::None,
                    },
                    MultilingualSegment {
                        view: MultilingualView::Translated,
                        wrap: SegmentWrap::Brackets,
                    },
                ])),
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![citum_schema::template::TemplateComponent::Title(
                    TemplateTitle {
                        title: TitleType::Primary,
                        rendering: Rendering {
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::SerialComponent(Box::new(SerialComponent {
        id: Some("suzuki2020".into()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Multilingual(MultilingualComplex {
            original: "引用の社会的機能".to_string(),
            lang: Some("ja".into()),
            sort_as: None,
            transliterations: HashMap::from([(
                "ja-Latn".to_string(),
                "In'yo no shakaiteki kino".to_string(),
            )]),
            translations: HashMap::from([(
                "en".into(),
                "The Social Function of Citation".to_string(),
            )]),
        })),
        issued: DateValue::new("2020".to_string()),
        language: Some("ja".into()),
        ..Default::default()
    }));

    let bibliography = indexmap::IndexMap::from([("suzuki2020".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography();

    // Smart-quote: ASCII `'` between alphabetic chars → U+2019 (curly apostrophe).
    assert_eq!(
        rendered,
        "In\u{2019}yo no shakaiteki kino 引用の社会的機能 [The Social Function of Citation]. "
    );
}

/// Pattern mode: 2-segment Chinese title (original + [translation]), no romanization.
///
/// Verifies that a `Original` → `Translated(brackets)` pattern renders `原文 [translation]`
/// and that no romanized form leaks through (transliteration data is present in the
/// fixture but the pattern does not request it).
fn mla_pattern_renders_original_and_translation_chinese_title() {
    use citum_schema::options::{
        MultilingualMode, MultilingualSegment, MultilingualView, SegmentWrap,
    };
    let style = Style {
        info: StyleInfo {
            title: Some("MLA Pattern Test".to_string()),
            default_locale: Some("en-US".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            multilingual: Some(MultilingualConfig {
                title_mode: Some(MultilingualMode::Pattern(vec![
                    MultilingualSegment {
                        view: MultilingualView::OriginalScript,
                        wrap: SegmentWrap::None,
                    },
                    MultilingualSegment {
                        view: MultilingualView::Translated,
                        wrap: SegmentWrap::Brackets,
                    },
                ])),
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![citum_schema::template::TemplateComponent::Title(
                    TemplateTitle {
                        title: TitleType::Primary,
                        rendering: Rendering {
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::SerialComponent(Box::new(SerialComponent {
        id: Some("wang2021".into()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Multilingual(MultilingualComplex {
            original: "战争与和平".to_string(),
            lang: Some("zh".into()),
            sort_as: None,
            transliterations: HashMap::from([(
                "zh-Latn-pinyin".to_string(),
                "Zhànzhēng yǔ Hépíng".to_string(),
            )]),
            translations: HashMap::from([("en".into(), "War and Peace".to_string())]),
        })),
        issued: DateValue::new("2021".to_string()),
        language: Some("zh".into()),
        ..Default::default()
    }));

    let bibliography = indexmap::IndexMap::from([("wang2021".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography();

    // No romanized segment in this pattern — the transliteration is present in the
    // fixture data but must not appear in the output.
    assert_eq!(rendered, "战争与和平 [War and Peace]. ");
}

// ── Pattern mode helpers ──────────────────────────────────────────────────────

fn given_mla_pattern_mode_when_original_and_translation_exist_then_original_precedes_bracketed_translation()
 {
    use citum_schema::options::{
        MultilingualMode, MultilingualSegment, MultilingualView, SegmentWrap,
    };
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
        transliterations: HashMap::new(), // MLA may omit transliteration
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "War and Peace".to_string());
            map
        },
    };
    let ml_string = MultilingualString::Complex(complex);
    let mode = MultilingualMode::Pattern(vec![
        MultilingualSegment {
            view: MultilingualView::OriginalScript,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::Translated,
            wrap: SegmentWrap::Brackets,
        },
    ]);
    let result = resolve_multilingual_string(&ml_string, Some(&mode), None, None, "en");
    assert_eq!(result, "战争与和平 [War and Peace]");
}

fn given_chicago_pattern_mode_when_all_three_views_exist_then_romanized_original_translation_are_joined()
 {
    use citum_schema::options::{
        MultilingualMode, MultilingualSegment, MultilingualView, SegmentWrap,
    };
    let complex = MultilingualComplex {
        original: "战争与和平".to_string(),
        lang: Some("zh".into()),
        sort_as: None,
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
            map.insert("en".into(), "War and Peace".to_string());
            map
        },
    };
    let ml_string = MultilingualString::Complex(complex);
    let mode = MultilingualMode::Pattern(vec![
        MultilingualSegment {
            view: MultilingualView::Transliterated,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::OriginalScript,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::Translated,
            wrap: SegmentWrap::Brackets,
        },
    ]);
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&mode),
        Some(&["zh-Latn-pinyin".to_string()]),
        None,
        "en",
    );
    assert_eq!(result, "Zhànzhēng yǔ Hépíng 战争与和平 [War and Peace]");
}

fn given_pattern_mode_when_transliteration_equals_original_then_duplicate_is_suppressed() {
    use citum_schema::options::{
        MultilingualMode, MultilingualSegment, MultilingualView, SegmentWrap,
    };
    // When the transliteration falls back to or happens to match the original,
    // the dedup logic should suppress the second occurrence.
    let complex = MultilingualComplex {
        original: "Tokyo".to_string(),
        lang: Some("ja".into()),
        sort_as: None,
        transliterations: {
            let mut map = HashMap::new();
            map.insert("ja-Latn".to_string(), "Tokyo".to_string()); // same as original
            map
        },
        translations: {
            let mut map = HashMap::new();
            map.insert("en".into(), "Tokyo (East Capital)".to_string());
            map
        },
    };
    let ml_string = MultilingualString::Complex(complex);
    let mode = MultilingualMode::Pattern(vec![
        MultilingualSegment {
            view: MultilingualView::Transliterated,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::OriginalScript,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::Translated,
            wrap: SegmentWrap::Brackets,
        },
    ]);
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&mode),
        Some(&["ja-Latn".to_string()]),
        None,
        "en",
    );
    // "Tokyo" appears once (dedup suppresses original); translation is still shown
    assert_eq!(result, "Tokyo [Tokyo (East Capital)]");
}

fn given_pattern_mode_when_translation_is_missing_then_missing_segment_is_skipped() {
    use citum_schema::options::{
        MultilingualMode, MultilingualSegment, MultilingualView, SegmentWrap,
    };
    let complex = MultilingualComplex {
        original: "引用の社会的機能".to_string(),
        lang: Some("ja".into()),
        sort_as: None,
        transliterations: {
            let mut map = HashMap::new();
            map.insert(
                "ja-Latn".to_string(),
                "In'yo no shakaiteki kino".to_string(),
            );
            map
        },
        translations: HashMap::new(), // no English translation available
    };
    let ml_string = MultilingualString::Complex(complex);
    let mode = MultilingualMode::Pattern(vec![
        MultilingualSegment {
            view: MultilingualView::Transliterated,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::OriginalScript,
            wrap: SegmentWrap::None,
        },
        MultilingualSegment {
            view: MultilingualView::Translated,
            wrap: SegmentWrap::Brackets,
        },
    ]);
    let result = resolve_multilingual_string(
        &ml_string,
        Some(&mode),
        None,
        Some(&"Latn".to_string()),
        "en",
    );
    // Translation segment is absent → only romanized + original
    assert_eq!(result, "In'yo no shakaiteki kino 引用の社会的機能");
}

mod string_resolution {
    use super::announce_behavior;

    #[test]
    fn simple_strings_return_the_original_text() {
        announce_behavior(
            "A simple multilingual string should resolve to its original text unchanged.",
        );
        super::given_a_simple_string_when_resolved_then_the_original_text_is_returned();
    }

    #[test]
    fn primary_mode_uses_the_original_script() {
        announce_behavior(
            "Primary multilingual mode should keep the original script instead of transliterating or translating.",
        );
        super::given_primary_mode_when_resolving_a_multilingual_title_then_the_original_script_is_returned();
    }

    #[test]
    fn exact_transliteration_matches_are_used() {
        announce_behavior(
            "An exact transliteration script match should use that transliterated value.",
        );
        super::given_an_exact_transliteration_match_when_resolving_then_that_transliteration_is_used();
    }

    #[test]
    fn transliteration_prefix_matches_are_used() {
        announce_behavior(
            "A transliteration script prefix should match the more specific transliteration variant.",
        );
        super::given_a_transliteration_prefix_match_when_resolving_then_the_matching_transliteration_is_used();
    }

    #[test]
    fn transliterated_mode_falls_back_to_the_original_text() {
        announce_behavior(
            "If no transliteration is available, transliterated mode should fall back to the original text.",
        );
        super::given_no_transliteration_when_transliterated_mode_is_requested_then_the_original_text_is_used();
    }

    #[test]
    fn translated_mode_uses_the_requested_locale_translation() {
        announce_behavior(
            "Translated mode should select the translation that matches the requested locale.",
        );
        super::given_translated_mode_when_resolving_then_the_requested_locale_translation_is_used();
    }

    #[test]
    fn translated_mode_falls_back_to_base_language_for_region_locale() {
        announce_behavior(
            "Translated mode should use a base-language translation when the style locale has a region subtag.",
        );
        super::given_translated_mode_with_region_locale_when_resolving_then_the_base_language_translation_is_used();
    }

    #[test]
    fn combined_mode_uses_transliteration_and_translation_when_both_exist() {
        announce_behavior(
            "Combined mode should join the transliteration with the locale translation when both exist.",
        );
        super::given_combined_mode_when_transliteration_and_translation_exist_then_both_are_combined();
    }

    #[test]
    fn combined_mode_uses_preferred_script_and_base_language_translation() {
        announce_behavior(
            "Combined mode should join a preferred-script transliteration with a base-language translation.",
        );
        super::given_combined_mode_with_preferred_script_and_region_locale_then_transliteration_and_base_translation_are_combined();
    }

    #[test]
    fn combined_mode_falls_back_to_original_plus_translation() {
        announce_behavior(
            "Combined mode should fall back to original text plus translation when no transliteration exists.",
        );
        super::given_combined_mode_without_transliteration_when_resolving_then_original_and_translation_are_combined();
    }

    #[test]
    fn mla_pattern_mode_shows_original_then_bracketed_translation() {
        announce_behavior(
            "MLA Pattern mode should render the original CJK title followed by the English translation in brackets.",
        );
        super::given_mla_pattern_mode_when_original_and_translation_exist_then_original_precedes_bracketed_translation();
    }

    #[test]
    fn chicago_pattern_mode_shows_three_way_romanized_original_translation() {
        announce_behavior(
            "Chicago Pattern mode should render romanized + original CJK + [English translation] in that order.",
        );
        super::given_chicago_pattern_mode_when_all_three_views_exist_then_romanized_original_translation_are_joined();
    }

    #[test]
    fn pattern_mode_deduplicates_segments_with_identical_text() {
        announce_behavior(
            "Pattern mode should suppress a segment whose text is identical to the immediately preceding segment.",
        );
        super::given_pattern_mode_when_transliteration_equals_original_then_duplicate_is_suppressed(
        );
    }

    #[test]
    fn pattern_mode_skips_missing_segment_without_error() {
        announce_behavior(
            "Pattern mode should silently skip segments for which no text is available.",
        );
        super::given_pattern_mode_when_translation_is_missing_then_missing_segment_is_skipped();
    }
}

mod name_resolution {
    use super::announce_behavior;

    #[test]
    fn simple_structured_names_keep_their_name_parts() {
        announce_behavior(
            "A plain structured name should resolve without changing its family or given parts.",
        );
        super::given_a_simple_structured_name_when_resolved_then_the_name_parts_are_preserved();
    }

    #[test]
    fn requested_scripts_use_the_matching_transliterated_name() {
        announce_behavior(
            "Requested scripts should select the matching transliterated contributor name.",
        );
        super::given_a_multilingual_name_with_requested_script_when_resolved_then_the_transliterated_name_is_used();
    }

    #[test]
    fn script_prefixes_match_transliterated_names() {
        announce_behavior(
            "Script-prefix matching should work for multilingual contributor transliterations as well.",
        );
        super::given_a_multilingual_name_with_a_script_prefix_match_when_resolved_then_the_matching_transliteration_is_used();
    }

    #[test]
    fn missing_transliterations_fall_back_to_the_original_name() {
        announce_behavior(
            "Contributor names without transliterations should fall back to the original-script name.",
        );
        super::given_a_multilingual_name_without_transliterations_when_resolved_then_the_original_name_is_used();
    }
}

mod multilingual_rendering {
    use super::announce_behavior;

    #[test]
    fn translated_numeric_integral_citations_use_the_translated_anchor_name() {
        announce_behavior(
            "Translated numeric integral citations should use the translated author name as the narrative anchor.",
        );
        super::given_translated_numeric_integral_citations_when_rendered_then_the_translated_name_is_used_as_the_anchor();
    }

    #[test]
    fn field_language_overrides_win_for_effective_field_language() {
        announce_behavior(
            "A field-specific language override should win when computing the effective field language.",
        );
        super::given_field_language_overrides_when_resolving_the_effective_field_language_then_the_field_override_wins();
    }

    #[test]
    fn multilingual_title_languages_become_the_effective_item_language() {
        announce_behavior(
            "If an item has no top-level language, the multilingual title language should become the effective item language.",
        );
        super::given_no_item_language_when_resolving_the_effective_item_language_then_the_multilingual_title_language_is_used();
    }

    #[test]
    fn localized_citation_templates_follow_the_item_language() {
        announce_behavior(
            "Localized citation templates should follow the resolved item language when selecting a template.",
        );
        super::given_localized_citation_templates_when_the_item_language_matches_then_the_locale_specific_template_is_selected();
    }

    #[test]
    fn localized_bibliography_templates_can_follow_the_multilingual_title_language() {
        announce_behavior(
            "A multilingual title language should be enough to select a localized bibliography template.",
        );
        super::given_localized_bibliography_templates_when_only_the_multilingual_title_has_a_language_then_that_language_still_selects_the_template();
    }

    #[test]
    fn field_languages_drive_mixed_language_title_formatting() {
        announce_behavior(
            "Mixed-language titles should format each field according to its field-language override.",
        );
        super::given_mixed_language_titles_when_rendering_the_bibliography_then_field_languages_drive_the_title_formatting_overrides();
    }

    #[test]
    fn apa_multilingual_title_mode_romanizes_japanese_article_and_journal_titles() {
        announce_behavior(
            "APA combined title mode should romanize Japanese article and journal titles while adding the article-title translation.",
        );
        super::given_apa_multilingual_title_mode_when_rendering_a_japanese_article_then_titles_are_romanized();
    }

    #[test]
    fn chicago_pattern_renders_three_way_japanese_title() {
        announce_behavior(
            "Pattern mode [Transliterated, Original, Translated(brackets)] renders \
             'romanized original [translation]' for a Japanese title, with smart-quote applied to the transliteration.",
        );
        super::chicago_pattern_renders_three_way_japanese_title();
    }

    #[test]
    fn mla_pattern_renders_original_and_translation_chinese_title() {
        announce_behavior(
            "Pattern mode [Original, Translated(brackets)] renders 'original [translation]' \
             for a Chinese title; the transliteration present in the fixture must not appear.",
        );
        super::mla_pattern_renders_original_and_translation_chinese_title();
    }
}

mod config {
    use super::announce_behavior;

    #[test]
    fn multilingual_yaml_options_keep_script_preferences() {
        announce_behavior(
            "Deserializing multilingual YAML options should preserve script preferences and mode settings.",
        );
        super::given_multilingual_yaml_options_when_deserialized_then_the_config_keeps_script_preferences();
    }
}

#[test]
fn chicago_german_override_localizes_editor_verb() {
    let style = Style {
        info: StyleInfo {
            title: Some("Chicago German Test".to_string()),
            id: Some("chicago-de".into()),
            default_locale: Some("de-DE".to_string()),
            ..Default::default()
        },
        extends: Some(StyleBase::ChicagoAuthorDate18th.into()),
        options: Some(Config {
            locale_override: Some("de-DE-chicago".to_string()),
            contributors: Some(ContributorConfig {
                role: Some(RoleOptions {
                    roles: Some(HashMap::from([(
                        "translator".to_string(),
                        RoleRendering {
                            preset: Some(RoleLabelPreset::VerbPrefix),
                            ..Default::default()
                        },
                    )])),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Resolve it so the engine sees the Chicago templates
    let style = style.into_resolved();

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        InputReference::CollectionComponent(Box::new(CollectionComponent {
            id: Some("ref1".into()),
            r#type: citum_schema::reference::MonographComponentType::Chapter,
            title: Some(Title::Single("Kapitel".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                given: "Autor".into(),
                family: "Name".into(),
                ..Default::default()
            })),
            translator: None,
            issued: DateValue::new("2024".to_string()),
            container: Some(WorkRelation::Embedded(Box::new(
                InputReference::Collection(Box::new(Collection {
                    short_title: None,
                    id: None,
                    r#type: citum_schema::reference::CollectionType::EditedBook,
                    title: Some(Title::Single("Sammelband".to_string())),
                    container: None,
                    editor: Some(Contributor::StructuredName(StructuredName {
                        given: "Editor".into(),
                        family: "Name".into(),
                        ..Default::default()
                    })),
                    translator: None,
                    issued: DateValue::new("2024".to_string()),
                    publisher: None,
                    numbering: Vec::new(),
                    url: None,
                    accessed: None,
                    language: Some("de".into()),
                    field_languages: HashMap::new(),
                    note: None,
                    isbn: None,
                    ..Default::default()
                })),
            ))),
            numbering: Vec::new(),
            pages: None,
            url: None,
            accessed: None,
            language: Some("de".into()),
            field_languages: HashMap::new(),
            note: None,
            doi: None,
            genre: None,
            medium: None,
            archive_info: None,
            eprint: None,
            keywords: None,
            original: None,
            ..Default::default()
        })),
    );

    // Explicitly load the de-DE-chicago locale override.
    // In a real application, the loader would merge this with the base de-DE locale.
    // For this test, the override contains the role.editor.verb we want to verify.
    let de_de_chicago_bytes = citum_schema::embedded::get_locale_override_bytes("de-DE-chicago")
        .expect("de-DE-chicago bytes not found");
    let locale: citum_schema::locale::Locale =
        serde_yaml::from_slice(de_de_chicago_bytes).expect("failed to parse de-DE-chicago");

    let processor = Processor::with_locale(style, bib, locale);
    let output = processor.render_bibliography();

    assert_eq!(
        output,
        "Name, Autor. 2024. \u{201c}Kapitel.\u{201d} _Sammelband_, hg. von Editor Name."
    );
}

#[test]
fn german_override_localizes_translator_verb_when_role_preset_requests_it() {
    let style = make_german_translator_role_style();
    let mut bib = indexmap::IndexMap::new();
    bib.insert("ref1".to_string(), make_german_translator_reference());

    let de_de_chicago_bytes = citum_schema::embedded::get_locale_override_bytes("de-DE-chicago")
        .expect("de-DE-chicago bytes not found");
    let locale: citum_schema::locale::Locale =
        serde_yaml::from_slice(de_de_chicago_bytes).expect("failed to parse de-DE-chicago");

    let processor = Processor::with_locale(style, bib, locale);
    let output = processor.render_bibliography();

    assert_eq!(
        output,
        "Autor Name. 2024. _Das Buch_. übers. von Ubersetzer Name"
    );
}

#[test]
fn archive_hierarchy_labels_resolved_from_locales() {
    use citum_schema::locale::{ArchiveHierarchyField, Locale};

    // Test en-US
    let en_us_bytes =
        citum_schema::embedded::get_locale_bytes("en-US").expect("en-US locale bytes not found");
    let en_us = Locale::from_yaml_str(std::str::from_utf8(en_us_bytes).expect("invalid UTF-8"))
        .expect("failed to parse en-US locale");

    assert_eq!(
        en_us.resolved_archive_term(ArchiveHierarchyField::Collection),
        Some("collection".to_string()),
        "en-US collection label"
    );
    assert_eq!(
        en_us.resolved_archive_term(ArchiveHierarchyField::Series),
        Some("series".to_string()),
        "en-US series label"
    );
    assert_eq!(
        en_us.resolved_archive_term(ArchiveHierarchyField::Box),
        Some("box".to_string()),
        "en-US box label (singular)"
    );
    assert_eq!(
        en_us.resolved_archive_term(ArchiveHierarchyField::Folder),
        Some("folder".to_string()),
        "en-US folder label (singular)"
    );
    assert_eq!(
        en_us.resolved_archive_term(ArchiveHierarchyField::Item),
        Some("item".to_string()),
        "en-US item label (singular)"
    );

    // Test fr-FR
    let fr_fr_bytes =
        citum_schema::embedded::get_locale_bytes("fr-FR").expect("fr-FR locale bytes not found");
    let fr_fr = Locale::from_yaml_str(std::str::from_utf8(fr_fr_bytes).expect("invalid UTF-8"))
        .expect("failed to parse fr-FR locale");

    assert_eq!(
        fr_fr.resolved_archive_term(ArchiveHierarchyField::Collection),
        Some("fonds".to_string()),
        "fr-FR collection label"
    );
    assert_eq!(
        fr_fr.resolved_archive_term(ArchiveHierarchyField::Series),
        Some("série".to_string()),
        "fr-FR series label"
    );
    assert_eq!(
        fr_fr.resolved_archive_term(ArchiveHierarchyField::Box),
        Some("Boîte".to_string()),
        "fr-FR box label (singular)"
    );
    assert_eq!(
        fr_fr.resolved_archive_term(ArchiveHierarchyField::Folder),
        Some("Dossier".to_string()),
        "fr-FR folder label (singular)"
    );
    assert_eq!(
        fr_fr.resolved_archive_term(ArchiveHierarchyField::Item),
        Some("Pièce".to_string()),
        "fr-FR item label (singular)"
    );

    // Test de-DE
    let de_de_bytes =
        citum_schema::embedded::get_locale_bytes("de-DE").expect("de-DE locale bytes not found");
    let de_de = Locale::from_yaml_str(std::str::from_utf8(de_de_bytes).expect("invalid UTF-8"))
        .expect("failed to parse de-DE locale");

    assert_eq!(
        de_de.resolved_archive_term(ArchiveHierarchyField::Collection),
        Some("Sammlung".to_string()),
        "de-DE collection label"
    );
    assert_eq!(
        de_de.resolved_archive_term(ArchiveHierarchyField::Series),
        Some("Serie".to_string()),
        "de-DE series label"
    );
    assert_eq!(
        de_de.resolved_archive_term(ArchiveHierarchyField::Box),
        Some("Mappe".to_string()),
        "de-DE box label (singular)"
    );
    assert_eq!(
        de_de.resolved_archive_term(ArchiveHierarchyField::Folder),
        Some("Akte".to_string()),
        "de-DE folder label (singular)"
    );
    assert_eq!(
        de_de.resolved_archive_term(ArchiveHierarchyField::Item),
        Some("Dokument".to_string()),
        "de-DE item label (singular)"
    );
}
