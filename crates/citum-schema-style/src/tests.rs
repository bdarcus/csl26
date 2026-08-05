/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]

use super::*;
use indexmap::IndexMap;
use rstest::rstest;

#[test]
fn test_style_minimal_deserialization() {
    let yaml = r#"
info:
  title: Test Style
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(style.info.title.as_ref().unwrap(), "Test Style");
}

#[test]
fn test_style_with_citation() {
    let yaml = r#"
info:
  title: Test
citation:
  template:
    - contributor: author
      form: short
    - date: issued
      form: year
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let citation = style.citation.unwrap();
    assert_eq!(citation.resolve_template().unwrap().len(), 2);
}

#[test]
fn supplementary_identifier_template_components_parse() {
    let component: TemplateComponent = serde_yaml::from_str(
        r#"
identifier: cstr
prefix: "CSTR: "
suffix: "."
"#,
    )
    .unwrap();

    let TemplateComponent::Identifier(identifier) = component else {
        panic!("expected an identifier component");
    };
    assert_eq!(identifier.identifier.as_str(), "cstr");
    assert_eq!(identifier.rendering.prefix.as_deref(), Some("CSTR: "));
}

#[test]
fn localized_template_type_variants_retain_the_matched_locale() {
    let citation: CitationSpec = serde_yaml::from_str(
        r#"
template:
  - title: primary
locales:
  - locale: [en]
    template:
      - term: et-al
  - default: true
    template:
      - title: primary
type-variants:
  book:
    - number: edition
"#,
    )
    .unwrap();

    let resolved = citation
        .resolve_localized_template_for_type("book", Some("en-US"))
        .unwrap();
    assert_eq!(resolved.locale.as_deref(), Some("en"));
    assert!(matches!(
        resolved.template.as_slice(),
        [TemplateComponent::Number(_)]
    ));
}

#[test]
fn test_style_with_options() {
    let yaml = r#"
info:
  title: APA
options:
  processing: author-date
  contributors:
    display-as-sort: first
    and: symbol
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let options = style.options.unwrap();
    assert_eq!(options.processing, Some(options::Processing::AuthorDate));
}

#[test]
fn test_resolve_for_position_ibid_falls_back_to_subsequent() {
    let citation = CitationSpec {
        suffix: Some("base".into()),
        subsequent: Some(Box::new(CitationSpec {
            suffix: Some("subseq".into()),
            ..Default::default()
        })),
        ..Default::default()
    };

    let resolved = citation
        .resolve_for_position(Some(&crate::citation::Position::Ibid))
        .into_owned();
    assert_eq!(resolved.suffix.as_deref(), Some("subseq"));

    let resolved_with_locator = citation
        .resolve_for_position(Some(&crate::citation::Position::IbidWithLocator))
        .into_owned();
    assert_eq!(resolved_with_locator.suffix.as_deref(), Some("subseq"));
}

#[test]
fn test_resolve_for_position_ibid_precedes_subsequent() {
    let citation = CitationSpec {
        suffix: Some("base".into()),
        subsequent: Some(Box::new(CitationSpec {
            suffix: Some("subseq".into()),
            ..Default::default()
        })),
        ibid: Some(Box::new(CitationSpec {
            suffix: Some("ibid".into()),
            ..Default::default()
        })),
        ..Default::default()
    };

    let resolved = citation
        .resolve_for_position(Some(&crate::citation::Position::Ibid))
        .into_owned();
    assert_eq!(resolved.suffix.as_deref(), Some("ibid"));

    let resolved_subsequent = citation
        .resolve_for_position(Some(&crate::citation::Position::Subsequent))
        .into_owned();
    assert_eq!(resolved_subsequent.suffix.as_deref(), Some("subseq"));
}

#[test]
fn test_resolve_for_position_merges_note_start_text_case() {
    let citation = CitationSpec {
        note_start_text_case: Some(NoteStartTextCase::Lowercase),
        ibid: Some(Box::new(CitationSpec {
            note_start_text_case: Some(NoteStartTextCase::CapitalizeFirst),
            ..Default::default()
        })),
        ..Default::default()
    };

    let resolved = citation
        .resolve_for_position(Some(&crate::citation::Position::Ibid))
        .into_owned();
    assert_eq!(
        resolved.note_start_text_case,
        Some(NoteStartTextCase::CapitalizeFirst)
    );

    let unresolved = citation.resolve_for_position(None).into_owned();
    assert_eq!(
        unresolved.note_start_text_case,
        Some(NoteStartTextCase::Lowercase)
    );
}

#[test]
fn test_citum_first_yaml() {
    // Test parsing the actual citum-first.yaml file structure
    let yaml = r#"
info:
  title: APA
options:
  substitute:
    contributor-role-form: short
    template:
      - editor
      - title
  processing: author-date
  contributors:
    display-as-sort: first
    and: symbol
citation:
  template:
    - contributor: author
      form: short
    - date: issued
      form: year
bibliography:
  template:
    - contributor: author
      form: long
    - date: issued
      form: year
      wrap: parentheses
    - title: primary
    - title: parent-monograph
      prefix: "In "
      emph: true
    - number: volume
    - variable: doi
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();

    // Verify info
    assert_eq!(style.info.title.as_ref().unwrap(), "APA");

    // Verify options
    let options = style.options.unwrap();
    assert_eq!(options.processing, Some(options::Processing::AuthorDate));
    assert!(options.substitute.is_some());

    // Verify citation
    let citation = style.citation.unwrap();
    let citation_template = citation.resolve_template().unwrap();
    assert_eq!(citation_template.len(), 2);

    // Verify bibliography
    let bib = style.bibliography.unwrap();
    let bib_template = bib.resolve_template().unwrap();
    assert_eq!(bib_template.len(), 6);

    // Verify flattened rendering worked
    match &bib_template[1] {
        template::TemplateComponent::Date(d) => {
            assert_eq!(
                d.rendering.wrap,
                Some(template::WrapConfig {
                    punctuation: template::WrapPunctuation::Parentheses,
                    inner_prefix: None,
                    inner_suffix: None,
                })
            );
        }
        _ => panic!("Expected Date"),
    }

    match &bib_template[3] {
        template::TemplateComponent::Title(t) => {
            assert_eq!(t.rendering.prefix.as_deref(), Some("In "));
            assert_eq!(t.rendering.emph, Some(true));
        }
        _ => panic!("Expected Title"),
    }
}

#[test]
fn substitute_parent_serial_and_group_conditions_parse() {
    let yaml = r#"
info:
  title: Conditional style
citation:
  options:
    substitute:
      template:
      - editor
      - parent-serial
      - title
  template:
  - group:
    - contributor: author
      form: short
    render-when:
      field-present: recipient
      field-absent: title
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let citation = style.citation.unwrap();
    let substitute = citation
        .options
        .as_ref()
        .unwrap()
        .substitute
        .as_ref()
        .unwrap()
        .resolve();

    assert_eq!(
        substitute.template,
        vec![
            options::SubstituteKey::Editor,
            options::SubstituteKey::ParentSerial,
            options::SubstituteKey::Title,
        ]
    );

    let template = citation.resolve_template().unwrap();
    let template::TemplateComponent::Group(group) = &template[0] else {
        panic!("expected group component");
    };
    let condition = group.render_when.as_ref().expect("condition should parse");
    assert_eq!(
        condition.field_present,
        Some(template::TemplateConditionField::Recipient)
    );
    assert_eq!(
        condition.field_absent,
        Some(template::TemplateConditionField::Title)
    );
}

#[test]
fn test_style_custom_fields() {
    let yaml = r#"
version: "1.1"
info:
  title: Custom Fields Test
custom:
  my-extension: true
  author-tool: "StyleAuthor v2.0"
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(style.version, SchemaVersion::parse("1.1").unwrap());
    let custom = style.custom.as_ref().unwrap();
    assert_eq!(
        custom.get("my-extension").unwrap(),
        &serde_json::Value::Bool(true)
    );
    assert_eq!(
        custom.get("author-tool").unwrap(),
        &serde_json::Value::String("StyleAuthor v2.0".to_string())
    );

    // Round-trip test
    let round_tripped = serde_yaml::to_string(&style).unwrap();
    assert!(
        round_tripped.contains("version: 1.1")
            || round_tripped.contains("version: \"1.1\"")
            || round_tripped.contains("version: '1.1'")
    );
    assert!(round_tripped.contains("my-extension: true"));
    assert!(round_tripped.contains("author-tool:"));
}

#[test]
fn test_style_with_template_ref() {
    let yaml = r#"
info:
  title: Preset Test
citation:
  template-ref: apa
bibliography:
  template-ref: vancouver
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();

    // Test Citation template reference (APA)
    let citation = style.citation.unwrap();
    assert!(citation.template_ref.is_some());
    assert!(citation.template.is_none());

    let citation_template = citation.resolve_template().unwrap();
    assert_eq!(citation_template.len(), 3); // APA citation is (Author, Year, Locator)

    // precise check for APA structure
    match &citation_template[0] {
        template::TemplateComponent::Contributor(c) => {
            assert_eq!(c.contributor, template::ContributorRole::Author);
        }
        _ => panic!("Expected Contributor"),
    }

    // Test Bibliography template extension (Vancouver)
    let bib = style.bibliography.unwrap();
    let bib_template = bib.resolve_template().unwrap();
    // Vancouver bib has roughly 8 components
    assert!(bib_template.len() >= 5);

    // The entry leads with the author: its reference marker is declared, not
    // authored, so it is not a template component.
    match &bib_template[0] {
        template::TemplateComponent::Contributor(c) => {
            assert_eq!(c.contributor, template::ContributorRole::Author);
        }
        other => panic!("Expected Contributor, got {other:?}"),
    }
}

#[test]
fn test_template_overrides_template_ref_precedence() {
    let yaml = r#"
info:
  title: Override Test
citation:
  template-ref: apa
  template:
    - variable: doi
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let citation = style.citation.unwrap();

    // Should have both
    assert!(citation.template_ref.is_some());
    assert!(citation.template.is_some());

    // Template should win
    let resolved = citation.resolve_template().unwrap();
    assert_eq!(resolved.len(), 1);
    match &resolved[0] {
        template::TemplateComponent::Variable(v) => {
            assert_eq!(v.variable, template::SimpleVariable::Doi);
        }
        _ => panic!("Expected Variable"),
    }
}

#[test]
fn cid_and_integrity_fields_round_trip() {
    let yaml = r#"
extends: https://hub.citum.org/styles/apa-7th.yaml
extends-pin: bafkreigh2akiscaildc6mzfo4qtbk3rjmxa3ofkpzxqzd2cs6ftxvtsqfa
info:
  title: Pinned APA Variant
  citum-version: ">=0.38.0"
"#;
    let style: Style = Style::from_yaml_str(yaml).expect("yaml parses");
    assert!(style.extends.is_some(), "extends preserved");
    assert_eq!(
        style.extends_pin.as_deref(),
        Some("bafkreigh2akiscaildc6mzfo4qtbk3rjmxa3ofkpzxqzd2cs6ftxvtsqfa")
    );
    assert_eq!(style.info.citum_version.as_deref(), Some(">=0.38.0"));

    let serialized = serde_yaml::to_string(&style).expect("serializes");
    assert!(
        serialized.contains("extends-pin: bafkrei"),
        "extends-pin field name preserved on serialization: {serialized}"
    );
    assert!(
        serialized.contains("citum-version:"),
        "citum-version field name preserved on serialization: {serialized}"
    );
}

#[test]
fn citum_version_too_high_rejects_resolution() {
    let yaml = r#"
info:
  title: From the Future
  citum-version: ">=999.0.0"
"#;
    let style = Style::from_yaml_str(yaml).unwrap();
    let err = style.try_into_resolved().expect_err("must reject");
    assert!(
        matches!(err, ResolutionError::VersionMismatch { .. }),
        "expected VersionMismatch, got {err:?}"
    );
}

#[test]
fn citum_version_satisfied_resolves_normally() {
    let yaml = r#"
info:
  title: Satisfied
  citum-version: ">=0.0.1"
"#;
    let style = Style::from_yaml_str(yaml).unwrap();
    let resolved = style.try_into_resolved().expect("must resolve");
    assert_eq!(resolved.info.title.as_deref(), Some("Satisfied"));
}

#[test]
fn extends_pin_on_builtin_base_is_rejected() {
    let yaml = r#"
extends: apa-7th
extends-pin: bafkreigh2akiscaildc6mzfo4qtbk3rjmxa3ofkpzxqzd2cs6ftxvtsqfa
info:
  title: Bad Pin Target
"#;
    let style = Style::from_yaml_str(yaml).unwrap();
    let err = style.try_into_resolved().expect_err("must reject");
    assert!(
        matches!(err, ResolutionError::UriResolutionFailed { .. }),
        "expected UriResolutionFailed for builtin pin, got {err:?}"
    );
}

#[test]
fn cid_extends_uri_round_trips_as_uri_variant() {
    let yaml = r#"
extends: cid:bafkreigh2akiscaildc6mzfo4qtbk3rjmxa3ofkpzxqzd2cs6ftxvtsqfa
info:
  title: CID-extended Style
"#;
    let style: Style = Style::from_yaml_str(yaml).expect("yaml parses");
    let extends = style.extends.expect("extends present");
    assert!(extends.is_cid(), "cid: prefix detected as CID URI");
    assert_eq!(
        extends.key(),
        "cid:bafkreigh2akiscaildc6mzfo4qtbk3rjmxa3ofkpzxqzd2cs6ftxvtsqfa"
    );
}

#[test]
fn old_section_extends_key_is_captured_for_forward_compat() {
    let yaml = r#"
info:
  title: Old Section Template Reuse Key
citation:
  extends: apa
"#;
    let style: Style = serde_yaml::from_str(yaml).expect("parse succeeds");
    let citation = style.citation.expect("citation section present");
    assert!(
        citation.unknown_fields.contains_key("extends"),
        "removed `extends` key should land in unknown_fields for SoftDegrade detection, \
             got: {:?}",
        citation.unknown_fields,
    );
}

#[test]
fn test_citation_localized_templates() {
    let yaml = r#"
info:
  title: Localized Citation
citation:
  template:
    - variable: note
  locales:
    - locale: [de]
      template:
        - variable: publisher
    - default: true
      template:
        - variable: doi
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let citation = style.citation.unwrap();

    assert_eq!(
        citation
            .resolve_template_for_language(Some("de-AT"))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        citation
            .resolve_template_for_language(Some("fr"))
            .unwrap()
            .len(),
        1
    );
    match &citation.resolve_template_for_language(Some("de")).unwrap()[0] {
        template::TemplateComponent::Variable(v) => {
            assert_eq!(v.variable, template::SimpleVariable::Publisher);
        }
        _ => panic!("Expected Variable"),
    }
    match &citation.resolve_template_for_language(Some("fr")).unwrap()[0] {
        template::TemplateComponent::Variable(v) => {
            assert_eq!(v.variable, template::SimpleVariable::Doi);
        }
        _ => panic!("Expected Variable"),
    }
}

#[test]
fn test_bibliography_localized_templates() {
    let yaml = r#"
info:
  title: Localized Bibliography
bibliography:
  template:
    - variable: note
  locales:
    - locale: [ja, zh]
      template:
        - title: primary
    - default: true
      template:
        - contributor: author
          form: long
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let bibliography = style.bibliography.unwrap();

    match &bibliography
        .resolve_template_for_language(Some("ja-JP"))
        .unwrap()[0]
    {
        template::TemplateComponent::Title(_) => {}
        _ => panic!("Expected Title"),
    }
    match &bibliography
        .resolve_template_for_language(Some("en-US"))
        .unwrap()[0]
    {
        template::TemplateComponent::Contributor(_) => {}
        _ => panic!("Expected Contributor"),
    }
}

#[test]
fn bibliography_localized_type_variant_takes_precedence() {
    let yaml = r#"
info:
  title: Localized bibliography type variants
bibliography:
  template:
    - title: primary
  locales:
    - locale: [en]
      template:
        - title: primary
      type-variants:
        book:
          - number: edition
  type-variants:
    book:
      - variable: publisher
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let bibliography = style.bibliography.unwrap();

    assert!(matches!(
        bibliography.resolve_template_for_type("book", Some("en-US")),
        Some(template) if matches!(template.first(), Some(template::TemplateComponent::Number(_)))
    ));
    assert!(matches!(
        bibliography.resolve_template_for_type("book", Some("zh-CN")),
        Some(template) if matches!(template.first(), Some(template::TemplateComponent::Variable(_)))
    ));
}

#[test]
fn test_bibliography_with_groups() {
    let yaml = r#"
info:
  title: Grouped Bibliography Test
bibliography:
  template:
    - contributor: author
      form: long
  groups:
    - id: vietnamese
      heading:
        localized:
          vi: "Tài liệu tiếng Việt"
          en-US: "Vietnamese Sources"
      selector:
        field:
          language: vi
      sort:
        template:
          - key: author
            sort-order: given-family
    - id: other
      selector:
        not:
          field:
            language: vi
"#;
    let style: Style = serde_yaml::from_str(yaml).unwrap();
    let bib = style.bibliography.unwrap();

    assert!(bib.groups.is_some());
    let groups = bib.groups.unwrap();
    assert_eq!(groups.len(), 2);

    // First group
    assert_eq!(groups[0].id, "vietnamese");
    match groups[0].heading.as_ref().unwrap() {
        grouping::GroupHeading::Localized { localized } => {
            assert_eq!(localized.get("vi").unwrap(), "Tài liệu tiếng Việt");
        }
        _ => panic!("expected localized heading"),
    }
    assert!(groups[0].sort.is_some());

    // Second group (fallback with negation)
    assert_eq!(groups[1].id, "other");
    assert!(groups[1].heading.is_none());
    assert!(groups[1].selector.not.is_some());
}

#[test]
fn validate_type_name_accepts_known_types() {
    assert!(template::validate_type_name("article-journal"));
    assert!(template::validate_type_name("legal-case"));
    assert!(template::validate_type_name("all"));
    assert!(template::validate_type_name("default"));
}

#[test]
fn validate_type_name_normalizes_underscores() {
    assert!(template::validate_type_name("article_journal"));
    assert!(template::validate_type_name("legal_case"));
}

#[test]
fn validate_type_name_rejects_unknown() {
    assert!(!template::validate_type_name("article-journall"));
    assert!(!template::validate_type_name("unknown-type"));
    assert!(!template::validate_type_name(""));
}

#[test]
fn style_validate_emits_warning_for_unknown_type_in_bib_type_variants() {
    let mut type_variants = IndexMap::new();
    type_variants.insert(
        template::TypeSelector::Single("typo-type".to_string()),
        TemplateVariant::Full(vec![]),
    );

    let style = Style {
        bibliography: Some(BibliographySpec {
            type_variants: Some(type_variants),
            ..Default::default()
        }),
        ..Default::default()
    };

    let warnings = style.validate();
    assert_eq!(warnings.len(), 1);
    match &warnings[0] {
        SchemaWarning::UnknownTypeName { name, location } => {
            assert_eq!(name, "typo-type");
            assert_eq!(location, "bibliography.type-variants");
        }
    }
}

#[test]
fn style_validate_emits_warning_for_unknown_title_mapping_type() {
    let style: Style = serde_yaml::from_str(
        r#"
info:
  title: Future type mapping
options:
  titles:
    type-mapping:
      dance-performance: component
"#,
    )
    .unwrap();

    assert_eq!(
        style.validate(),
        vec![SchemaWarning::UnknownTypeName {
            name: "dance-performance".to_string(),
            location: "options.titles.type-mapping".to_string(),
        }]
    );
}

#[test]
fn style_validate_accepts_normalized_known_title_mapping_type() {
    let style: Style = serde_yaml::from_str(
        r#"
info:
  title: Known type mapping
options:
  titles:
    type-mapping:
      personal_communication: component
"#,
    )
    .unwrap();

    assert!(style.validate().is_empty());
}

#[test]
fn style_validate_no_warnings_for_valid_style() {
    let mut type_variants = IndexMap::new();
    type_variants.insert(
        template::TypeSelector::Single("legal-case".to_string()),
        TemplateVariant::Full(vec![]),
    );

    let style = Style {
        bibliography: Some(BibliographySpec {
            type_variants: Some(type_variants),
            ..Default::default()
        }),
        ..Default::default()
    };

    let warnings = style.validate();
    assert!(warnings.is_empty());
}

#[test]
fn null_type_variants_override_clears_preset_type_variants() {
    let child_yaml = r#"
extends: chicago-notes-18th
citation:
  type-variants: ~
  template:
  - contributor: author
    form: short
"#;
    let style = Style::from_yaml_str(child_yaml).expect("parses");
    let resolved = style.into_resolved();
    let citation = resolved.citation.unwrap();
    assert!(
        citation.type_variants.is_none(),
        "type_variants should be None after null override, got: {:?}",
        citation.type_variants.as_ref().map(|tv| tv.keys().count())
    );
}

#[test]
fn template_v3_diff_resolves_to_full_type_variant() {
    let yaml = r#"
bibliography:
  template:
  - contributor: author
    form: long
  - title: primary
  - variable: publisher
  - variable: url
  type-variants:
    article-journal:
      modify:
      - match: { variable: publisher }
        suppress: true
      remove:
      - match: { variable: url }
      add:
      - after: { title: primary }
        component: { title: parent-serial, emph: true }
"#;
    let resolved = Style::from_yaml_str(yaml)
        .expect("style should parse")
        .try_into_resolved()
        .expect("diff should resolve");
    let variants = resolved
        .bibliography
        .expect("bibliography should exist")
        .type_variants
        .expect("variants should exist");
    let template = variants
        .get(&TypeSelector::Single("article-journal".to_string()))
        .and_then(TemplateVariant::as_template)
        .expect("variant should resolve to a full template");

    assert_eq!(template.len(), 4);
    assert!(matches!(
        &template[2],
        TemplateComponent::Title(title)
            if title.title == template::TitleType::ParentSerial
                && title.rendering.emph == Some(true)
    ));
    assert!(template.iter().any(|component| matches!(
        component,
        TemplateComponent::Variable(variable)
            if variable.variable == template::SimpleVariable::Publisher
                && variable.rendering.suppress == Some(true)
    )));
}

#[test]
fn template_v3_modify_can_set_number_label_form() {
    let yaml = r#"
bibliography:
  template:
  - number: pages
  type-variants:
    chapter:
      modify:
      - match: { number: pages }
        label-form: short
"#;
    let resolved = Style::from_yaml_str(yaml)
        .expect("style should parse")
        .try_into_resolved()
        .expect("diff should resolve");
    let variants = resolved
        .bibliography
        .expect("bibliography should exist")
        .type_variants
        .expect("variants should exist");
    let template = variants
        .get(&TypeSelector::Single("chapter".to_string()))
        .and_then(TemplateVariant::as_template)
        .expect("variant should resolve to a full template");

    assert!(matches!(
        &template[0],
        TemplateComponent::Number(number)
            if number.number == template::NumberVariable::Pages
                && number.label_form == Some(template::LabelForm::Short)
    ));
}

#[test]
fn template_v3_add_with_missing_anchor_returns_resolution_error() {
    let yaml = r#"
bibliography:
  template:
  - title: primary
  type-variants:
    book:
      add:
      - after: { variable: publisher }
        component: { date: issued, form: year }
"#;
    let err = Style::from_yaml_str(yaml)
        .expect("style should parse")
        .try_into_resolved()
        .expect_err("missing add anchor should reject the diff");

    assert!(matches!(
        err,
        ResolutionError::TemplateVariantAnchorNotFound { location }
            if location == "bibliography.type-variants[book]"
    ));
}

#[test]
fn template_v3_nested_citation_diff_can_use_outer_template() {
    let style = Style {
        citation: Some(CitationSpec {
            template: Some(vec![
                TemplateComponent::Contributor(template::TemplateContributor {
                    contributor: template::ContributorRole::Author.into(),
                    ..Default::default()
                }),
                TemplateComponent::Title(template::TemplateTitle {
                    title: template::TitleType::Primary,
                    ..Default::default()
                }),
            ]),
            integral: Some(Box::new(CitationSpec {
                type_variants: Some(indexmap::IndexMap::from([(
                    TypeSelector::Single("personal_communication".to_string()),
                    TemplateVariant::Diff(TemplateVariantDiff {
                        remove: vec![TemplateRemoveOperation {
                            match_selector: TemplateComponentSelector {
                                fields: std::collections::BTreeMap::from([(
                                    "title".to_string(),
                                    serde_json::json!("primary"),
                                )]),
                            },
                        }],
                        ..Default::default()
                    }),
                )])),
                ..Default::default()
            })),
            ..Default::default()
        }),
        ..Default::default()
    };
    let resolved = style
        .try_into_resolved()
        .expect("nested diff should resolve against outer citation template");
    let variants = resolved
        .citation
        .expect("citation should exist")
        .integral
        .expect("integral citation should exist")
        .type_variants
        .expect("variants should exist");
    let template = variants
        .get(&TypeSelector::Single("personal_communication".to_string()))
        .and_then(TemplateVariant::as_template)
        .expect("variant should resolve");

    assert_eq!(template.len(), 1);
    assert!(matches!(template[0], TemplateComponent::Contributor(_)));
}

#[test]
fn template_v3_overlay_variant_defaults_to_inherited_variant() {
    #[derive(Clone)]
    struct FakeResolver {
        style: Style,
    }

    impl citum_resolver_api::StyleResolver for FakeResolver {
        type Style = Style;
        type Locale = Locale;

        fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
            if uri == "parent-style" {
                Ok(self.style.clone())
            } else {
                Err(ResolverError::StyleNotFound(std::borrow::Cow::Owned(
                    uri.to_string(),
                )))
            }
        }

        fn resolve_locale(&self, id: &str) -> Result<Self::Locale, ResolverError> {
            Err(ResolverError::LocaleNotFound(std::borrow::Cow::Owned(
                id.to_string(),
            )))
        }
    }

    let parent = Style::from_yaml_str(
        r#"
bibliography:
  template:
  - title: primary
  type-variants:
    book:
    - title: primary
      emph: true
"#,
    )
    .expect("parent should parse");
    let child = Style::from_yaml_str(
        r#"
extends: parent-style
bibliography:
  type-variants:
    book:
      modify:
      - match: { title: primary }
        quote: true
"#,
    )
    .expect("child should parse");

    let resolved = child
        .try_into_resolved_with(Some(&FakeResolver { style: parent }))
        .expect("child diff should resolve");
    let template = resolved
        .bibliography
        .expect("bibliography should exist")
        .type_variants
        .expect("variants should exist")
        .get(&TypeSelector::Single("book".to_string()))
        .and_then(TemplateVariant::as_template)
        .expect("book variant should resolve")
        .to_vec();

    assert!(matches!(
        &template[0],
        TemplateComponent::Title(title)
            if title.rendering.emph == Some(true)
                && title.rendering.quote == Some(true)
    ));
}

#[test]
fn citation_options_parse_valid_citation_fields() {
    let yaml = r#"
citation:
  options:
    contributors:
      shorten: {min: 3, use-first: 1}
    links:
      doi: true
"#;

    let style = Style::from_yaml_str(yaml).expect("citation options should parse");
    let options = style
        .citation
        .and_then(|citation| citation.options)
        .expect("citation options should exist");
    assert!(options.contributors.is_some());
    assert_eq!(options.links.and_then(|links| links.doi), Some(true));
}

#[test]
fn citation_options_capture_bibliography_only_fields_for_forward_compat() {
    let yaml = r#"
citation:
  options:
    entry-suffix: "."
"#;

    // Misplaced keys land in `unknown_fields` (SoftDegrade contract); strict-mode
    // surfaces them via `citum check --strict`.
    let style = Style::from_yaml_str(yaml).expect("citation options must tolerate unknown keys");
    let options = style
        .citation
        .and_then(|c| c.options)
        .expect("citation.options must exist");
    assert!(options.unknown_fields.contains_key("entry-suffix"));
}

#[test]
fn bibliography_options_parse_valid_bibliography_fields() {
    let yaml = r#"
bibliography:
  options:
    entry-suffix: "."
    separator: ", "
"#;

    let style = Style::from_yaml_str(yaml).expect("bibliography options should parse");
    let options = style
        .bibliography
        .and_then(|bibliography| bibliography.options)
        .expect("bibliography options should exist");
    assert_eq!(options.entry_suffix.as_deref(), Some("."));
    assert_eq!(options.separator.as_deref(), Some(", "));
}

#[test]
fn bibliography_options_capture_citation_only_fields_for_forward_compat() {
    let yaml = r#"
bibliography:
  options:
    locators:
      form: short
"#;

    let style =
        Style::from_yaml_str(yaml).expect("bibliography options must tolerate unknown keys");
    let options = style
        .bibliography
        .and_then(|b| b.options)
        .expect("bibliography.options must exist");
    assert!(options.unknown_fields.contains_key("locators"));
}

#[test]
fn top_level_options_capture_unknown_fields() {
    let yaml = r#"
options:
  bibliography:
    entry-suffix: "."
"#;

    // Unknown keys under `options:` are captured for forward-compat rather than rejected.
    let style = Style::from_yaml_str(yaml).expect("unknown top-level option key must be tolerated");
    assert!(!style.options.unwrap().unknown_fields.is_empty());
}

#[test]
fn profile_rejects_top_level_templates() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
templates:
  foo:
    - title: primary
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile template override must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "templates"
    ));
}

#[test]
fn profile_rejects_citation_template_override() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
citation:
  template:
    - title: primary
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile citation template override must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "citation.template"
    ));
}

#[test]
fn profile_rejects_bibliography_type_variants_override() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
bibliography:
  type-variants:
    default:
      - title: primary
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile bibliography type variants must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "bibliography.type-variants"
    ));
}

#[test]
fn profile_rejects_null_template_clear() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
bibliography:
  template: ~
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile null template clear must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "bibliography.template"
    ));
}

#[test]
fn profile_rejects_bibliography_template_ref_override() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
bibliography:
  template-ref: apa
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile bibliography template-ref override must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "bibliography.template-ref"
    ));
}

#[test]
fn profile_rejects_citation_localized_template_override() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
citation:
  locales:
    - locale: [fr-FR]
      template:
        - title: primary
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile citation localized template override must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "citation.locales"
    ));
}

#[test]
fn profile_rejects_citation_subsequent_template_override() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
citation:
  subsequent:
    template:
      - title: primary
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile citation subsequent template override must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "citation.subsequent.template"
    ));
}

#[test]
fn profile_rejects_citation_ibid_type_variants_override() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
citation:
  ibid:
    type-variants:
      default:
        - title: primary
"#;
    let err = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect_err("profile citation ibid type variants override must fail");
    assert!(matches!(
        err,
        ResolutionError::InvalidProfileOverride { location } if location == "citation.ibid.type-variants"
    ));
}

#[test]
fn profile_allows_normal_options() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
options:
  page-range-format: expanded
"#;
    let resolved = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect("profile wrappers should accept normal options");
    assert_eq!(
        resolved
            .options
            .as_ref()
            .and_then(|options| options.page_range_format.clone()),
        Some(options::PageRangeFormat::Expanded)
    );
}

#[test]
fn profile_rejects_removed_options_profile_surface() {
    let yaml = r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
options:
  profile:
    citation-label-wrap: brackets
"#;
    let err = Style::from_yaml_str(yaml).expect_err("legacy profile surface must fail");
    assert!(err.to_string().contains("`options.profile` was removed"));
}

#[test]
fn profile_resolution_leaves_hidden_core_templates_intact() {
    let base = StyleBase::ElsevierHarvardCore.base();
    let wrapper = Style::from_yaml_str(
        r#"
info:
  id: elsevier-harvard
extends: elsevier-harvard-core
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("wrapper should resolve");

    assert_eq!(
        wrapper
            .citation
            .as_ref()
            .and_then(|citation| citation.resolve_template()),
        base.citation
            .as_ref()
            .and_then(|citation| citation.resolve_template())
    );
    assert_eq!(
        wrapper
            .bibliography
            .as_ref()
            .and_then(|bib| bib.resolve_template()),
        base.bibliography
            .as_ref()
            .and_then(|bib| bib.resolve_template())
    );
}

#[test]
fn scoped_options_apply_to_profile_wrappers() {
    let resolved = Style::from_yaml_str(
        r#"
info:
  id: elsevier-vancouver
extends: elsevier-vancouver-core
citation:
  options:
    label-wrap: brackets
    group-delimiter: comma
bibliography:
  options:
    title-terminator: comma
    repeated-author-rendering: dash
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("scoped wrapper config should resolve");

    assert_eq!(
        resolved
            .citation
            .as_ref()
            .and_then(|citation| citation.wrap.clone()),
        None
    );
    assert_eq!(
        resolved
            .citation
            .as_ref()
            .and_then(|citation| citation.options.as_ref())
            .and_then(|options| options.label_wrap),
        Some(options::LabelWrap::Brackets)
    );
    assert_eq!(
        resolved
            .citation
            .as_ref()
            .and_then(|citation| citation.multi_cite_delimiter.clone())
            .as_deref(),
        Some(", ")
    );
    assert_eq!(
        resolved
            .bibliography
            .as_ref()
            .and_then(|bib| bib.options.as_ref())
            .and_then(|options| options.subsequent_author_substitute.clone())
            .as_deref(),
        Some("———")
    );
}

#[test]
fn options_contributors_replaces_profile_contributor_slot() {
    let resolved = Style::from_yaml_str(
        r#"
info:
  id: springer-basic-author-date
extends: springer-basic-author-date-core
options:
  contributors: springer
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("top-level contributor preset should resolve");

    let contributors = resolved
        .options
        .as_ref()
        .and_then(|options| options.contributors.as_ref())
        .expect("resolved style should include contributor config");
    assert_eq!(contributors.name_form, Some(options::NameForm::Initials));
    assert_eq!(
        contributors.demote_non_dropping_particle,
        Some(options::DemoteNonDroppingParticle::Never)
    );
}

#[test]
fn citation_superscript_wrap_applies_vertical_align() {
    let resolved = Style::from_yaml_str(
        r#"
info:
  id: elsevier-vancouver
extends: elsevier-vancouver-core
citation:
  options:
    label-wrap: superscript
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("superscript citation wrap should resolve");

    assert_eq!(
        resolved
            .citation
            .as_ref()
            .and_then(|citation| citation.options.as_ref())
            .and_then(|options| options.label_wrap),
        Some(options::LabelWrap::Superscript)
    );
}

/// Reference markers are processor-owned: a style declares `label-mode` and
/// the processor materializes the component. Authoring one directly is
/// rejected at parse time, so a marker never has two spellings and never
/// degrades silently into a `Custom` variable that renders nothing.
/// See `docs/specs/REFERENCE_MARKERS.md`.
#[rstest]
#[case::numeric_marker("citation-number")]
#[case::alphabetic_marker("citation-label")]
fn test_authoring_a_reference_marker_is_rejected(#[case] variable: &str) {
    // given a style whose citation template names a marker variable
    let yaml = format!(
        r#"
citation:
  template:
    - number: {variable}
"#
    );

    // when it is parsed
    let result = Style::from_yaml_str(&yaml);

    // then it is rejected rather than accepted as an unknown variable
    assert!(
        result.is_err(),
        "authoring `{variable}` should be rejected, got: {:?}",
        result.map(|style| style.citation)
    );
}

#[test]
fn citation_label_mode_round_trips_and_is_inherited() {
    let parent = Style::from_yaml_str(
        r#"
info: { id: numeric-label-parent, title: Numeric Label Parent }
options:
  processing: numeric
citation:
  options:
    label-mode: numeric
    label-wrap: brackets
  template: []
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("numeric citation label mode should resolve");
    let citation_options = parent
        .citation
        .as_ref()
        .and_then(|citation| citation.options.as_ref())
        .expect("citation options should be present");
    assert_eq!(
        citation_options.label_mode,
        Some(options::CitationLabelMode::Numeric)
    );
    let serialized: serde_yaml::Value = serde_yaml::from_str(
        &serde_yaml::to_string(&parent).expect("resolved style should serialize"),
    )
    .expect("serialized style should remain valid YAML");
    assert_eq!(
        serialized["citation"]["options"]["label-mode"].as_str(),
        Some("numeric")
    );

    let mut child = parent.clone();
    child
        .citation
        .as_mut()
        .and_then(|citation| citation.options.as_mut())
        .expect("citation options should remain present")
        .label_mode = Some(options::CitationLabelMode::None);
    let resolved_child = child
        .try_into_resolved()
        .expect("explicit none should resolve");
    assert_eq!(
        resolved_child
            .citation
            .as_ref()
            .and_then(|citation| citation.options.as_ref())
            .and_then(|options| options.label_mode),
        Some(options::CitationLabelMode::None)
    );
}

#[test]
fn bibliography_rejects_superscript_label_wrap_at_parse_time() {
    let yaml = r#"
bibliography:
  options:
    label-wrap: superscript
"#;
    let err = Style::from_yaml_str(yaml).expect_err("bibliography superscript wrap must fail");
    assert!(err.to_string().contains("unknown variant `superscript`"));
}

#[test]
fn standalone_styles_can_use_scoped_options() {
    let resolved = Style::from_yaml_str(
        r#"
citation:
  template-ref: numeric-citation
  options:
    label-wrap: superscript
    group-delimiter: comma
bibliography:
  template-ref: vancouver
  options:
    label-mode: numeric
    title-terminator: comma
    repeated-author-rendering: dash-with-space
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("standalone scoped options should resolve");

    assert_eq!(
        resolved
            .citation
            .as_ref()
            .and_then(|citation| citation.multi_cite_delimiter.as_deref()),
        Some(", ")
    );
    assert_eq!(
        resolved
            .citation
            .as_ref()
            .and_then(|citation| citation.options.as_ref())
            .and_then(|options| options.label_wrap),
        Some(options::LabelWrap::Superscript)
    );
    assert_eq!(
        resolved
            .bibliography
            .as_ref()
            .and_then(|bib| bib.options.as_ref())
            .and_then(|options| options.subsequent_author_substitute.as_deref()),
        Some("——— ")
    );
}

#[test]
fn non_registry_extends_styles_do_not_use_profile_contract() {
    let yaml = r#"
info:
  id: local-custom-profile
extends: elsevier-vancouver-core
citation:
  template:
    - number: volume
"#;
    let resolved = Style::from_yaml_str(yaml)
        .unwrap()
        .try_into_resolved()
        .expect("non-registry extends styles should retain merge semantics");
    assert!(resolved.citation.is_some());
}

#[test]
fn uri_extends_file_resolves_yaml() {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let dir = std::env::temp_dir().join(format!("citum_test_{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    let parent_path = dir.join("parent.yaml");
    let mut f = std::fs::File::create(&parent_path).unwrap();
    f.write_all(b"info:\n  title: Parent\ncitation:\n  template: []\n")
        .unwrap();
    let child_yaml = format!(
        "info:\n  title: Child\nextends: \"file://{}\"\n",
        parent_path.display()
    );
    let child = Style::from_yaml_str(&child_yaml).unwrap();
    let resolved = child.try_into_resolved().unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        resolved.citation.is_some(),
        "should inherit citation from file-backed parent"
    );
}

#[test]
fn uri_extends_missing_file_returns_error() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let dir = std::env::temp_dir().join(format!("citum_test_{nanos}_missing"));
    std::fs::create_dir_all(&dir).unwrap();
    let missing = dir.join("does_not_exist.yaml");
    let yaml = format!(
        "info:\n  title: Child\nextends: \"file://{}\"\n",
        missing.display()
    );
    let child = Style::from_yaml_str(&yaml).unwrap();
    let err = child.try_into_resolved().unwrap_err();
    std::fs::remove_dir_all(&dir).ok();
    assert!(
        matches!(err, ResolutionError::UriResolutionFailed { .. }),
        "expected UriResolutionFailed, got {err:?}"
    );
}

#[test]
fn uri_extends_unsupported_scheme_returns_error() {
    let yaml = "info:\n  title: Child\nextends: \"https://example.com/style.yaml\"\n";
    let child = Style::from_yaml_str(yaml).unwrap();
    let err = child.try_into_resolved().unwrap_err();
    assert!(
        matches!(err, ResolutionError::UriResolutionFailed { .. }),
        "expected UriResolutionFailed for unsupported scheme, got {err:?}"
    );
}

#[test]
fn style_loader_reports_components_typo_in_add_operation() {
    let yaml = r#"
bibliography:
  template:
  - title: primary
  type-variants:
    chapter:
      add:
      - after: { title: primary }
        components:
          variable: doi
"#;
    let err = Style::from_yaml_str(yaml).expect_err("components typo should fail");
    let message = err.to_string();

    assert!(
        message.contains("bibliography.type-variants.chapter.add[0]"),
        "message should include precise operation path: {message}"
    );
    assert!(
        message.contains("unknown property \"components\" in TemplateAddOperation"),
        "message should name the rejected property and operation type: {message}"
    );
    assert!(
        message.contains("did you mean \"component\""),
        "message should suggest the singular field: {message}"
    );
}

#[test]
fn style_loader_reports_invalid_nested_component_body_path() {
    let yaml = r#"
bibliography:
  template:
  - title: primary
  type-variants:
    chapter:
      add:
      - after: { title: primary }
        component:
          components:
          - variable: doi
"#;
    let err = Style::from_yaml_str(yaml).expect_err("nested component body should fail");
    let message = err.to_string();

    assert!(
        message.contains("bibliography.type-variants.chapter.add[0].component"),
        "message should include precise nested component path: {message}"
    );
    assert!(
        message.contains("unknown template component property \"components\""),
        "message should name the invalid component property: {message}"
    );
    assert!(
        message.contains("component must contain exactly one of contributor/date/title/number/identifier/variable/message/group/term/type-label"),
        "message should enumerate valid component kinds: {message}"
    );
}

#[test]
fn style_loader_reports_empty_render_when() {
    let yaml = r#"
bibliography:
  template:
  - group:
    - variable: doi
    render-when: {}
"#;
    let err = Style::from_yaml_str(yaml).expect_err("empty render-when should fail");
    let message = err.to_string();

    assert!(
        message.contains("must set field-present or field-absent"),
        "message should reject an unconditional render-when: {message}"
    );
}

#[test]
fn style_loader_reports_contradictory_render_when() {
    let yaml = r#"
bibliography:
  template:
  - group:
    - variable: doi
    render-when:
      field-present: title
      field-absent: title
"#;
    let err = Style::from_yaml_str(yaml).expect_err("contradictory render-when should fail");
    let message = err.to_string();

    assert!(
        message.contains("must not be the same field"),
        "message should reject field-present and field-absent naming the same field: {message}"
    );
}

#[test]
fn style_loader_reports_unknown_template_component_key() {
    let yaml = r#"
bibliography:
  template:
  - components:
    - variable: doi
"#;
    let err = Style::from_yaml_str(yaml).expect_err("unknown component key should fail");
    let message = err.to_string();

    assert!(
        message.contains("bibliography.template[0]"),
        "message should include template item path: {message}"
    );
    assert!(
        message.contains("component must contain exactly one of contributor/date/title/number/identifier/variable/message/group/term/type-label"),
        "message should enumerate valid component kinds: {message}"
    );
}

#[test]
fn style_loader_accepts_valid_recursive_template_surfaces() {
    let yaml = r#"
templates:
  fallback-title:
  - title: primary
citation:
  template:
  - group:
    - contributor: author
      form: short
    - date: issued
      form: year
      fallback:
      - variable: doi
  locales:
  - locale: [en-US]
    template:
    - title: primary
  integral:
    template:
    - contributor: author
      form: short
bibliography:
  template:
  - title: primary
  type-variants:
    chapter:
    - title: parent-monograph
    article-journal:
      add:
      - after: { title: primary }
        component: { title: parent-serial, emph: true }
  groups:
  - id: cited
    selector:
      cited: visible
    template:
    - variable: doi
"#;
    Style::from_yaml_str(yaml).expect("valid recursive template surfaces should parse");
}

#[test]
fn chicago_notes_templates_keep_english_prose_in_locale_messages() {
    let yaml = include_str!("../embedded/styles/chicago-notes-18th.yaml");
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
    let mut violations = Vec::new();

    collect_english_affix_literals(&value, "$", &mut violations);

    assert!(
        violations.is_empty(),
        "move natural-language template affixes into locale messages: {violations:#?}"
    );
}

fn collect_english_affix_literals(
    value: &serde_yaml::Value,
    path: &str,
    violations: &mut Vec<String>,
) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, child) in mapping {
                let key_text = key.as_str().unwrap_or("<non-string-key>");
                let child_path = format!("{path}.{key_text}");
                if is_template_affix_key(key_text)
                    && let Some(text) = child.as_str()
                    && text.chars().any(char::is_alphabetic)
                    && !is_allowed_non_prose_affix(key_text, text)
                {
                    violations.push(format!("{child_path}: {text:?}"));
                }
                collect_english_affix_literals(child, &child_path, violations);
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for (index, child) in sequence.iter().enumerate() {
                collect_english_affix_literals(child, &format!("{path}[{index}]"), violations);
            }
        }
        _ => {}
    }
}

fn is_template_affix_key(key: &str) -> bool {
    matches!(key, "prefix" | "suffix" | "delimiter" | "text")
}

fn is_allowed_non_prose_affix(key: &str, text: &str) -> bool {
    (key == "prefix" && text.contains("https://doi.org/")) || (key == "delimiter" && text == "none")
}

#[test]
fn contributor_role_scalar_and_list_round_trip_without_shape_loss() {
    for yaml in [
        "contributor: author\nform: long\n",
        "contributor: [writer, director]\nform: long\n",
    ] {
        let component: template::TemplateContributor = serde_yaml::from_str(yaml).unwrap();
        let serialized = serde_yaml::to_string(&component).unwrap();
        let reparsed: template::TemplateContributor = serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(component, reparsed);
    }
}

#[test]
fn role_label_wrap_round_trips_without_affix_substitution() {
    let yaml = r#"
contributor: writer
form: long
label:
  term: writer
  form: long
  placement: suffix
  wrap: parentheses
"#;
    let component: template::TemplateContributor = serde_yaml::from_str(yaml).unwrap();
    let serialized = serde_yaml::to_string(&component).unwrap();
    let reparsed: template::TemplateContributor = serde_yaml::from_str(&serialized).unwrap();

    assert_eq!(component, reparsed);
    assert!(matches!(
        component.label.and_then(|label| label.wrap).as_deref(),
        Some(&template::WrapConfig {
            punctuation: template::WrapPunctuation::Parentheses,
            inner_prefix: None,
            inner_suffix: None,
        })
    ));
}

#[test]
fn contributor_list_defaults_merge_behavior_when_block_is_absent() {
    let component: template::TemplateContributor =
        serde_yaml::from_str("contributor: [writer, director]\nform: long\n").unwrap();

    assert!(component.contributor.is_multiple());
    assert_eq!(component.merge, None);
    assert_eq!(
        template::ContributorMerge::default().order,
        template::ContributorMergeOrder::Document
    );
    assert!(template::ContributorMerge::default().combine_same_person);
}

#[test]
fn contributor_list_semantic_validation_rejects_invalid_combinations() {
    let invalid_components = [
        "- contributor: author\n  merge: {}",
        "- contributor: [writer]",
        "- contributor: [writer, writer]",
        "- contributor: [writer, director]\n  label: {term: writer}",
        "- contributor: [writer, director]\n  merge:\n    roles:\n      producer: {labels: none}",
    ];
    for template in invalid_components {
        let yaml = format!(
            "info:\n  title: Invalid contributor test\nbibliography:\n  template:\n{}\n",
            template
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert!(
            Style::from_yaml_str(&yaml).is_err(),
            "accepted invalid template:\n{template}"
        );
    }
}

#[test]
fn contributor_list_resource_validation_reports_authored_field_paths() {
    fn style_with_contributor(contributor: &str) -> Style {
        Style::from_yaml_str(&format!(
            "info:\n  title: Contributor validation test\nbibliography:\n  template:\n    - contributor: {contributor}\n      form: long\n"
        ))
        .expect("base contributor style should parse")
    }

    fn contributor_mut(style: &mut Style) -> &mut template::TemplateContributor {
        let component = style
            .bibliography
            .as_mut()
            .and_then(|bibliography| bibliography.template.as_mut())
            .and_then(|template| template.first_mut())
            .expect("test style should contain a bibliography component");
        let template::TemplateComponent::Contributor(contributor) = component else {
            panic!("test component should be a contributor");
        };
        contributor
    }

    let mut scalar = style_with_contributor("author");
    contributor_mut(&mut scalar).merge = Some(template::ContributorMerge::default());
    assert_eq!(
        scalar.validate_resource_limits(),
        Err("bibliography.template.merge is valid only for a contributor role list".into())
    );

    let mut list_with_label = style_with_contributor("[writer, director]");
    contributor_mut(&mut list_with_label).label =
        Some(serde_yaml::from_str("term: writer").expect("test role label should deserialize"));
    assert_eq!(
        list_with_label.validate_resource_limits(),
        Err("bibliography.template.label is valid only for a single role".into())
    );

    let mut list_with_undeclared_override = style_with_contributor("[writer, director]");
    let mut merge = template::ContributorMerge::default();
    merge.roles.insert(
        template::ContributorRole::Producer,
        template::ContributorMergeRole::default(),
    );
    contributor_mut(&mut list_with_undeclared_override).merge = Some(merge);
    assert_eq!(
        list_with_undeclared_override.validate_resource_limits(),
        Err("bibliography.template.merge.roles contains undeclared role producer".into())
    );
}

#[test]
fn producer_is_a_known_style_contributor_role() {
    let component: template::TemplateContributor =
        serde_yaml::from_str("contributor: producer\nform: long\n").unwrap();

    assert_eq!(
        component.contributor.as_single(),
        Some(&template::ContributorRole::Producer)
    );
}

#[test]
fn en_us_resolves_combined_roles_alias_and_connector() {
    let mut locale = locale::Locale::en_us();
    let writer_director = [
        template::ContributorRole::Writer,
        template::ContributorRole::Director,
    ];
    assert_eq!(
        locale.resolved_role_combination_term(
            &writer_director,
            false,
            &locale::TermForm::Long,
            None,
        ),
        Some("writer & director".to_string())
    );
    assert_eq!(
        locale.resolved_role_combination_term(
            &writer_director,
            true,
            &locale::TermForm::Long,
            None,
        ),
        Some("writers & directors".to_string())
    );
    assert_eq!(
        locale
            .messages
            .get("term.role-conjunction")
            .map(String::as_str),
        Some(" & ")
    );
    locale
        .messages
        .insert("term.role-conjunction".to_string(), " + ".to_string());
    assert_eq!(locale.role_conjunction(), " + ");
    locale.messages.remove("term.role-conjunction");
    assert_eq!(locale.role_conjunction(), " & ");
    assert!(locale.role_combinations.contains_key("editor-translator"));
    assert!(!locale.role_combinations.contains_key("editortranslator"));
    assert_eq!(
        locale.resolved_role_combination_term(
            &[
                template::ContributorRole::Translator,
                template::ContributorRole::Editor,
            ],
            false,
            &locale::TermForm::Short,
            None,
        ),
        Some("ed. & trans.".to_string())
    );
}

#[test]
fn invalid_merge_label_mode_is_rejected_in_unused_type_variant() {
    let yaml = r#"
info: {id: invalid-merge-mode, title: Invalid merge mode}
bibliography:
  type-variants:
    episode:
      - contributor: [writer, director]
        merge:
          labels: combined
  template:
    - contributor: author
"#;

    Style::from_yaml_str(yaml).expect_err("invalid label mode must fail style loading");
}

#[test]
fn substitution_candidate_role_lists_require_distinct_roles() {
    let yaml = r#"
info: {id: invalid-substitution-roles, title: Invalid substitution roles}
options:
  substitute:
    overrides:
      episode:
        - contributor: [writer, writer]
bibliography:
  template:
    - contributor: author
"#;

    Style::from_yaml_str(yaml).expect_err("duplicate substitution roles must fail style loading");
}

#[cfg(feature = "schema")]
#[test]
fn contributor_list_schema_exposes_scalar_list_and_merge_shapes() {
    let schema = serde_json::to_value(schemars::schema_for!(template::TemplateContributor))
        .expect("contributor schema should serialize");

    assert_eq!(
        schema.pointer("/$defs/ContributorRoles/anyOf/0/$ref"),
        Some(&serde_json::json!("#/$defs/ContributorRole"))
    );
    assert_eq!(
        schema.pointer("/$defs/ContributorRoles/anyOf/1/minItems"),
        Some(&serde_json::json!(2))
    );
    assert_eq!(
        schema.pointer("/properties/merge/anyOf/0/$ref"),
        Some(&serde_json::json!("#/$defs/ContributorMerge"))
    );
    assert_eq!(
        schema.pointer("/$defs/ContributorMerge/properties/combine-same-person/default"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        schema.pointer("/$defs/RoleLabel/properties/wrap/anyOf/0/$ref"),
        Some(&serde_json::json!("#/$defs/WrapConfig"))
    );
}

#[cfg(feature = "schema")]
#[test]
fn substitute_schema_exposes_scalar_and_contributor_candidates() {
    let schema = serde_json::to_value(schemars::schema_for!(options::SubstituteKey))
        .expect("substitution candidate schema should serialize");

    assert_eq!(
        schema.pointer("/anyOf/0/$ref"),
        Some(&serde_json::json!("#/$defs/SubstituteField"))
    );
    assert_eq!(
        schema.pointer("/anyOf/1/$ref"),
        Some(&serde_json::json!("#/$defs/SubstituteContributor"))
    );
}

#[test]
fn punctuation_mark_mapping_is_semantic_but_scalar_name_is_literal() {
    let semantic: template::DelimiterPunctuation =
        serde_yaml::from_str("{ mark: comma }").expect("semantic mark should parse");
    let literal: template::DelimiterPunctuation =
        serde_yaml::from_str("comma").expect("literal punctuation should parse");

    assert_eq!(semantic, template::DelimiterPunctuation::Comma);
    assert_eq!(
        literal,
        template::DelimiterPunctuation::Custom("comma".to_string())
    );
    assert_eq!(
        serde_yaml::to_string(&semantic).expect("semantic mark should serialize"),
        "mark: comma\n"
    );
    assert_eq!(
        serde_yaml::to_string(&literal).expect("literal punctuation should serialize"),
        "comma\n"
    );
}

#[test]
fn multilingual_script_realization_overrides_parse_as_partial_tables() {
    let yaml = r#"
info: { id: realization-overrides, title: Realization Overrides }
options:
  multilingual:
    scripts:
      cjk:
        realization:
          comma: "、"
          parentheses: ["〔", "〕"]
bibliography:
  template:
    - title: primary
"#;
    let style: Style = serde_yaml::from_str(yaml).expect("realization overrides should parse");
    let realization = style
        .options
        .as_ref()
        .and_then(|options| options.multilingual.as_ref())
        .and_then(|options| options.scripts.get("cjk"))
        .and_then(|script| script.realization.as_ref())
        .expect("CJK realization table should be present");

    assert_eq!(realization.comma.as_deref(), Some("、"));
    assert_eq!(
        realization.parentheses.as_ref(),
        Some(&["〔".to_string(), "〕".to_string()])
    );
    assert_eq!(realization.colon, None);
}

#[test]
fn numeric_label_mode_does_not_mutate_authored_template() {
    let resolved = Style::from_yaml_str(
        r#"
info: { id: numeric-flush-wrap, title: Numeric Flush Wrap }
bibliography:
  options:
    label-mode: numeric
  template:
    - contributor: author
      form: long
    - title: primary
"#,
    )
    .unwrap()
    .try_into_resolved()
    .expect("numeric label-mode should resolve");

    let template = resolved
        .bibliography
        .as_ref()
        .and_then(|bib| bib.template.as_ref())
        .expect("bibliography template should be present");

    assert_eq!(template.len(), 2);
    assert!(matches!(
        &template[0],
        template::TemplateComponent::Contributor(contributor)
            if contributor.contributor == template::ContributorRoles::Single(template::ContributorRole::Author)
    ));
    assert!(matches!(
        &template[1],
        template::TemplateComponent::Title(_)
    ));
}

#[test]
fn numeric_label_mode_is_idempotent_across_inheritance_levels() {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let dir = std::env::temp_dir().join(format!("citum_numeric_idempotent_{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    let parent_path = dir.join("parent.yaml");
    std::fs::File::create(&parent_path)
        .unwrap()
        .write_all(
            br#"
info: { id: numeric-idempotent-parent, title: Numeric Idempotent Parent }
bibliography:
  options:
    label-mode: numeric
  template:
    - contributor: author
      form: long
"#,
        )
        .unwrap();

    let child_yaml = format!(
        "info: {{ id: numeric-idempotent-child, title: Numeric Idempotent Child }}\nextends: \"file://{}\"\n",
        parent_path.display()
    );
    let resolved = Style::from_yaml_str(&child_yaml)
        .unwrap()
        .try_into_resolved()
        .expect("child inheriting numeric label-mode should resolve");
    std::fs::remove_dir_all(&dir).ok();

    let template = resolved
        .bibliography
        .as_ref()
        .and_then(|bib| bib.template.as_ref())
        .expect("bibliography template should be present");

    // The marker is never a template component, so inheritance can only carry
    // the declared mode. See docs/specs/REFERENCE_MARKERS.md.
    assert!(
        !template.is_empty(),
        "the inherited bibliography template should survive, got {template:?}"
    );
    assert_eq!(
        resolved
            .bibliography
            .as_ref()
            .and_then(|bib| bib.options.as_ref())
            .and_then(|options| options.label_mode),
        Some(options::BibliographyLabelMode::Numeric),
        "numeric label mode must remain runtime-owned across inheritance levels"
    );
}

#[test]
fn author_date_label_mode_strips_label_inherited_from_a_numeric_parent() {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let dir = std::env::temp_dir().join(format!("citum_author_date_strip_{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    let parent_path = dir.join("parent.yaml");
    std::fs::File::create(&parent_path)
        .unwrap()
        .write_all(
            br#"
info: { id: author-date-strip-parent, title: Author Date Strip Parent }
bibliography:
  options:
    label-mode: numeric
  template:
    - contributor: author
      form: long
    - title: primary
"#,
        )
        .unwrap();

    let child_yaml = format!(
        "info: {{ id: author-date-strip-child, title: Author Date Strip Child }}\nextends: \"file://{}\"\nbibliography:\n  options:\n    label-mode: author-date\n",
        parent_path.display()
    );
    let resolved = Style::from_yaml_str(&child_yaml)
        .unwrap()
        .try_into_resolved()
        .expect("author-date override should resolve");
    std::fs::remove_dir_all(&dir).ok();

    let template = resolved
        .bibliography
        .as_ref()
        .and_then(|bib| bib.template.as_ref())
        .expect("bibliography template should be present");

    assert_eq!(
        resolved
            .bibliography
            .as_ref()
            .and_then(|bib| bib.options.as_ref())
            .and_then(|options| options.label_mode),
        Some(options::BibliographyLabelMode::AuthorDate),
        "author-date label-mode must win over the numeric parent's, got {template:?}"
    );
    assert!(matches!(
        &template[0],
        template::TemplateComponent::Contributor(contributor)
            if contributor.contributor == template::ContributorRoles::Single(template::ContributorRole::Author)
    ));
}
