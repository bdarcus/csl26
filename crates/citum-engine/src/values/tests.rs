/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::*;
use crate::reference::Reference;
use crate::render::plain::PlainText;
use citum_schema::locale::{GeneralTerm, GrammaticalGender, Locale, TermForm};
use citum_schema::options::contributors::NameForm;
use citum_schema::options::*;
use citum_schema::reference::{
    ClassExtension, Collection, CollectionComponent, CollectionType, Contributor, ContributorEntry,
    ContributorGender, DateValue, FlatName, InputReference, Monograph, MonographComponentType,
    MonographType, StructuredName, Title, WorkRelation,
};
use citum_schema::template::DateVariable as TemplateDateVar;
use citum_schema::template::*;
use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference, StringOrNumber};
use rstest::rstest;

fn make_config() -> Config {
    Config {
        processing: Some(citum_schema::options::Processing::AuthorDate),
        contributors: Some(ContributorConfig {
            shorten: Some(ShortenListOptions {
                min: 3,
                use_first: 1,
                ..Default::default()
            }),
            and: Some(AndOptions::Symbol),
            display_as_sort: Some(DisplayAsSort::First),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn language_script_resolution_prefers_explicit_script_evidence() {
    for (language, expected) in [
        ("zh-Latn", "Latn"),
        ("ja-Hani", "Hani"),
        ("sr-Latn-RS", "Latn"),
        ("en-Cyrl", "Cyrl"),
        ("und-Arab", "Arab"),
    ] {
        assert_eq!(
            super::resolve_language_script(Some(language)).as_deref(),
            Some(expected),
            "{language}"
        );
    }
}

#[test]
fn language_identifier_parsing_normalizes_and_trims_shared_tag_forms() {
    let underscore = super::parse_language_identifier(Some(" tr_TR ")).unwrap();
    assert_eq!(underscore.language.as_str(), "tr");
    assert_eq!(
        underscore
            .region
            .map(|region| region.to_string())
            .as_deref(),
        Some("TR")
    );

    let suffixed = super::parse_language_identifier(Some("tr-TR-citum_override")).unwrap();
    assert_eq!(suffixed.language, underscore.language);
    assert_eq!(suffixed.region, underscore.region);

    assert_eq!(super::parse_language_identifier(Some("_")), None);
    assert_eq!(super::parse_language_identifier(Some("  ")), None);
}

#[test]
fn language_script_resolution_uses_likely_subtags_for_recognized_languages() {
    for (language, expected) in [
        ("en", "Latn"),
        ("en_US", "Latn"),
        ("sr_Latn_RS", "Latn"),
        ("tr-TR-citum_override", "Latn"),
        ("en-u-ca-gregory", "Latn"),
        ("ru-RU", "Cyrl"),
        ("ar", "Arab"),
        ("he", "Hebr"),
        ("el", "Grek"),
        ("hi", "Deva"),
        ("zh", "Hans"),
        ("zh-TW", "Hant"),
        ("ja", "Jpan"),
        ("ko", "Kore"),
    ] {
        assert_eq!(
            super::resolve_language_script(Some(language)).as_deref(),
            Some(expected),
            "{language}"
        );
    }
}

#[test]
fn language_script_resolution_requires_positive_language_evidence() {
    for language in [
        "",
        " ",
        "und",
        "mul",
        "zxx",
        "x-private",
        "x-Latn",
        "e",
        "1en",
        "123",
        "zz",
    ] {
        assert_eq!(
            super::resolve_language_script(Some(language)),
            None,
            "{language} must not resolve to a default script"
        );
    }
    assert_eq!(super::resolve_language_script(None), None);
}

#[test]
fn latin_script_adapter_matches_only_resolved_latin_script() {
    for language in ["en", "en-US", "zh-Latn", "ja-Latn"] {
        assert!(
            super::is_latin_script_language(Some(language)),
            "{language}"
        );
    }
    for language in ["zh", "zh-Hans", "ru-RU", "und", "zz"] {
        assert!(
            !super::is_latin_script_language(Some(language)),
            "{language} must not trigger Latin punctuation remapping"
        );
    }
    assert!(!super::is_latin_script_language(None));
}

#[test]
fn script_class_requires_positive_latin_or_cjk_evidence() {
    for language in ["zh", "zh-Hans", "zh-Hant", "ja", "ko", "yue-Bopo"] {
        assert_eq!(
            super::script_class(Some(language)),
            Some(super::ScriptClass::Cjk),
            "{language}"
        );
    }
    for language in ["en", "fr-Latn", "sr-Latn", "ru-RU"] {
        assert_eq!(
            super::script_class(Some(language)),
            Some(super::ScriptClass::Latin),
            "{language}"
        );
    }
    for language in ["ar", "he", "el", "", "und", "mul", "zxx"] {
        assert_eq!(super::script_class(Some(language)), None, "{language}");
    }
    assert_eq!(super::script_class(None), None);
}

#[test]
fn wrap_script_class_gates_evidence_override_on_the_declared_default() {
    // Latin default (unset `realization-default`, every existing style today):
    // no item language, regardless of script evidence, overrides it. This is
    // what keeps romanized citations of non-Latin sources (Chicago's
    // "romanized + original-script [translation]" mode) from picking up
    // full-width punctuation despite a CJK `language` field.
    for language in [None, Some("zh"), Some("ja"), Some("en")] {
        assert_eq!(
            super::wrap_script_class(language, super::ScriptClass::Latin),
            super::ScriptClass::Latin,
            "{language:?}"
        );
    }

    // CJK default (`realization-default: cjk`, style opted in): evidence
    // overrides in both directions; absence of evidence keeps the default.
    assert_eq!(
        super::wrap_script_class(Some("zh"), super::ScriptClass::Cjk),
        super::ScriptClass::Cjk
    );
    assert_eq!(
        super::wrap_script_class(Some("en"), super::ScriptClass::Cjk),
        super::ScriptClass::Latin
    );
    assert_eq!(
        super::wrap_script_class(Some("ru"), super::ScriptClass::Cjk),
        super::ScriptClass::Latin
    );
    assert_eq!(
        super::wrap_script_class(None, super::ScriptClass::Cjk),
        super::ScriptClass::Cjk
    );
}

/// A `make_config` variant declaring the APA role-label defaults bundle,
/// for tests that exercise other behavior through the editor suffix.
fn make_config_with_apa_role_defaults() -> Config {
    let mut config = make_config();
    if let Some(contributors) = &mut config.contributors {
        contributors.role = Some(RoleOptions {
            defaults: Some(RoleLabelDefaults::Apa),
            ..Default::default()
        });
    }
    config
}

fn make_locale() -> Locale {
    Locale::en_us()
}

fn make_embedded_english_locale() -> Locale {
    Locale::from_yaml_str(include_str!("../../../../locales/en-US.yaml"))
        .expect("english locale should parse")
}

fn make_embedded_chinese_locale() -> Locale {
    Locale::from_yaml_str(include_str!("../../../../locales/zh-CN.yaml"))
        .expect("Chinese locale should parse")
}

fn make_reference() -> Reference {
    Reference::from(LegacyReference {
        id: "kuhn1962".to_string(),
        ref_type: "book".to_string(),
        author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
        title: Some("The Structure of Scientific Revolutions".to_string()),
        issued: Some(DateVariable::year(1962)),
        publisher: Some("University of Chicago Press".to_string()),
        ..Default::default()
    })
}

fn make_spanish_gendered_locale() -> Locale {
    Locale::from_yaml_str(include_str!("../../../../locales/es-ES.yaml"))
        .expect("spanish locale should parse")
}

fn make_french_gendered_locale() -> Locale {
    Locale::from_yaml_str(include_str!("../../../../locales/fr-FR.yaml"))
        .expect("french locale should parse")
}

fn make_arabic_gendered_locale() -> Locale {
    Locale::from_yaml_str(include_str!("../../../../locales/ar-AR.yaml"))
        .expect("arabic locale should parse")
}

fn make_editor_reference(genders: &[ContributorGender]) -> Reference {
    let contributors = genders
        .iter()
        .enumerate()
        .map(|(idx, gender)| ContributorEntry {
            roles: citum_schema::reference::ContributorRole::Editor.into(),
            contributor: Contributor::StructuredName(StructuredName {
                family: format!("Editor{idx}").into(),
                given: format!("Nombre{idx}").into(),
                ..Default::default()
            }),
            gender: Some(*gender),
        })
        .collect();

    InputReference::Monograph(Box::new(Monograph {
        id: Some("editor-role-ref".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Obra".to_string())),
        contributors,
        issued: DateValue::new("2024".to_string()),
        ..Default::default()
    }))
}

fn make_custom_role_reference(
    role: citum_schema::reference::ContributorRole,
    genders: &[ContributorGender],
) -> Reference {
    let contributors = genders
        .iter()
        .enumerate()
        .map(|(idx, gender)| ContributorEntry {
            roles: role.clone().into(),
            contributor: Contributor::StructuredName(StructuredName {
                family: format!("Persona{idx}").into(),
                given: format!("Nombre{idx}").into(),
                ..Default::default()
            }),
            gender: Some(*gender),
        })
        .collect();

    InputReference::Monograph(Box::new(Monograph {
        id: Some("custom-role-ref".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Obra".to_string())),
        contributors,
        issued: DateValue::new("2024".to_string()),
        ..Default::default()
    }))
}

fn make_gendered_locator_locale() -> Locale {
    Locale::from_yaml_str(
        r#"
locale: es-ES
locators:
  volume:
    short:
      singular:
        masculine: tomo
        feminine: entrega
      plural:
        masculine: tomos
        feminine: entregas
"#,
    )
    .expect("gendered locator locale should parse")
}

/// Helper to create `NameFormatContext` for tests.
fn make_name_format_context<'a>(
    display_as_sort: Option<DisplayAsSort>,
    name_order: Option<&'a NameOrder>,
    initialize_with: Option<&'a String>,
    initialize_with_hyphen: Option<bool>,
    name_form: Option<NameForm>,
    demote_ndp: Option<&'a DemoteNonDroppingParticle>,
    sort_separator: Option<&'a String>,
) -> super::contributor::NameFormatContext<'a> {
    super::contributor::NameFormatContext {
        display_as_sort,
        name_order,
        total_names: 1,
        initialize_with,
        initialize_with_hyphen,
        name_form,
        demote_ndp,
        sort_separator,
        component_sort_separator: None,
        script_configs: None,
        integral_name_state: None,
        org_abbreviation_state: None,
        use_integral_short_name: true,
        short_name_display: None,
        subsequent_form: None,
        strip_periods: false,
    }
}

fn make_literal_name_with_short_name() -> FlatName {
    FlatName {
        literal: Some("World Health Organization".to_string()),
        short_name: Some("WHO".to_string()),
        ..Default::default()
    }
}

#[test]
fn given_literal_short_name_when_first_integral_mention_then_parenthetical_form_renders() {
    let mut ctx = make_name_format_context(None, None, None, None, None, None, None);
    ctx.org_abbreviation_state = Some(citum_schema::citation::IntegralNameState::First);
    ctx.short_name_display = Some(ShortNameDisplay::FullThenParenthetical);

    let result = super::contributor::format_single_name(
        &make_literal_name_with_short_name(),
        &ContributorForm::Long,
        0,
        &ctx,
        false,
    );

    assert_eq!(result, "World Health Organization (WHO)");
}

#[test]
fn given_literal_short_name_when_subsequent_integral_mention_then_short_form_renders() {
    let mut ctx = make_name_format_context(None, None, None, None, None, None, None);
    ctx.org_abbreviation_state = Some(citum_schema::citation::IntegralNameState::Subsequent);
    ctx.short_name_display = Some(ShortNameDisplay::FullThenParenthetical);

    let result = super::contributor::format_single_name(
        &make_literal_name_with_short_name(),
        &ContributorForm::Long,
        0,
        &ctx,
        false,
    );

    assert_eq!(result, "WHO");
}

#[test]
fn given_short_then_bracketed_option_when_first_integral_mention_then_bracketed_form_renders() {
    let mut ctx = make_name_format_context(None, None, None, None, None, None, None);
    ctx.org_abbreviation_state = Some(citum_schema::citation::IntegralNameState::First);
    ctx.short_name_display = Some(ShortNameDisplay::ShortThenBracketed);

    let result = super::contributor::format_single_name(
        &make_literal_name_with_short_name(),
        &ContributorForm::Long,
        0,
        &ctx,
        false,
    );

    assert_eq!(result, "WHO [World Health Organization]");
}

#[test]
fn given_non_integral_context_when_integral_name_state_is_set_then_literal_short_name_is_ignored() {
    let mut ctx = make_name_format_context(None, None, None, None, None, None, None);
    ctx.org_abbreviation_state = Some(citum_schema::citation::IntegralNameState::Subsequent);
    ctx.use_integral_short_name = false;
    ctx.short_name_display = Some(ShortNameDisplay::FullThenParenthetical);

    let result = super::contributor::format_single_name(
        &make_literal_name_with_short_name(),
        &ContributorForm::Long,
        0,
        &ctx,
        false,
    );

    assert_eq!(result, "World Health Organization");
}

/// Tests the behavior of `test_contributor_values`.
#[test]
fn test_contributor_values() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateContributor {
        fallback: None,
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        label: None,
        merge: None,
        name_order: None,
        name_form: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        gender: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "Kuhn");
}

/// `family-first-except-last` inverts every contributor except the last one
/// in the full list (CSL schema#134: Bioscene / AJFAND author-list style,
/// e.g. "Gardner MN, Halweil AO and JM Nono").
#[test]
fn test_family_first_except_last_inverts_all_but_last_name() {
    let mut config = make_config();
    if let Some(contributors) = &mut config.contributors {
        contributors.shorten = None;
        contributors.and = Some(AndOptions::Text);
    }
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_custom_role_reference(
        citum_schema::reference::ContributorRole::Author,
        &[
            ContributorGender::Common,
            ContributorGender::Common,
            ContributorGender::Common,
        ],
    );
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Long,
        name_order: Some(NameOrder::FamilyFirstExceptLast),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("author list should render");

    assert_eq!(
        values.value,
        "Persona0, Nombre0, Persona1, Nombre1, and Nombre2 Persona2"
    );
}

#[test]
fn test_spanish_role_label_uses_feminine_form_for_single_editor() {
    let config = make_config();
    let locale = make_spanish_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_editor_reference(&[ContributorGender::Feminine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("editor should render");

    assert_eq!(values.suffix, Some(", editora".to_string()));
}

#[test]
fn test_spanish_role_label_uses_plural_feminine_form_for_matching_group() {
    let config = make_config();
    let locale = make_spanish_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference =
        make_editor_reference(&[ContributorGender::Feminine, ContributorGender::Feminine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("editors should render");

    assert_eq!(values.suffix, Some(", editoras".to_string()));
}

#[test]
fn test_spanish_role_label_prefers_common_form_for_mixed_group() {
    let config = make_config();
    let locale = make_spanish_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference =
        make_editor_reference(&[ContributorGender::Feminine, ContributorGender::Masculine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("mixed editors should render");

    assert_eq!(values.suffix, Some(", equipo editorial".to_string()));
}

#[test]
fn test_spanish_role_label_omits_gendered_label_for_mixed_group_without_common_form() {
    let config = make_config();
    let locale = Locale::from_yaml_str(
        r#"
locale: es-ES
roles:
  editor:
    long:
      singular:
        masculine: editor
        feminine: editora
      plural:
        masculine: editores
        feminine: editoras
"#,
    )
    .expect("locale should parse");
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference =
        make_editor_reference(&[ContributorGender::Feminine, ContributorGender::Masculine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("editors should render");

    assert_eq!(values.suffix, None);
}

#[test]
fn test_french_role_label_uses_feminine_form_for_single_contributor() {
    let config = make_config();
    let locale = make_french_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_editor_reference(&[ContributorGender::Feminine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("editor should render");

    assert_eq!(values.suffix, Some(", éditrice".to_string()));
}

#[test]
fn test_arabic_role_label_uses_feminine_form_for_single_contributor() {
    let config = make_config();
    let locale = make_arabic_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_editor_reference(&[ContributorGender::Feminine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("editor should render");

    assert_eq!(values.suffix, Some(", مُحَرِّرَة".to_string()));
}

#[test]
fn test_french_role_label_falls_back_to_masculine_plural_for_mixed_group() {
    let config = make_config();
    let locale = make_french_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference =
        make_editor_reference(&[ContributorGender::Feminine, ContributorGender::Masculine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("mixed editors should render");

    assert_eq!(values.suffix, Some(", éditeurs".to_string()));
}

#[test]
fn test_arabic_role_label_falls_back_to_verbal_noun_for_mixed_group() {
    let config = make_config();
    let locale = make_arabic_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference =
        make_editor_reference(&[ContributorGender::Feminine, ContributorGender::Masculine]);
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("mixed editors should render");

    assert_eq!(values.suffix, Some(", تحقيق".to_string()));
}

#[test]
fn test_arabic_role_label_falls_back_to_roles_common_when_gender_missing() {
    let config = make_config();
    let locale = make_arabic_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };

    // Reference with no gender info
    let contributors = vec![ContributorEntry {
        roles: citum_schema::reference::ContributorRole::Editor.into(),
        contributor: Contributor::StructuredName(StructuredName {
            family: "Editor".into(),
            given: "Name".into(),
            ..Default::default()
        }),
        gender: None,
    }];

    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("no-gender-ref".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Obra".to_string())),
        contributors,
        issued: DateValue::new("2024".to_string()),
        ..Default::default()
    }));

    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("editor should render");

    // MF2 will return None because $gender is missing, should fall back to roles.editor.long.singular.common
    assert_eq!(values.suffix, Some(", تحقيق".to_string()));
}

#[test]
fn test_collection_editor_role_label_derives_gender_from_reference_data() {
    let config = make_config();
    let locale = make_spanish_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_custom_role_reference(
        citum_schema::reference::ContributorRole::Unknown("collection-editor".to_string()),
        &[ContributorGender::Feminine],
    );
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::CollectionEditor.into(),
        form: ContributorForm::Long,
        label: Some(RoleLabel {
            term: "collection-editor".to_string(),
            form: RoleLabelForm::Long,
            placement: LabelPlacement::Suffix,
            text_case: None,
            wrap: None,
            prefix: None,
            suffix: None,
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("collection editor should render");

    assert_eq!(values.suffix, Some(", directora".to_string()));
}

/// Tests the behavior of `test_date_values`.
#[test]
fn test_date_values() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        rendering: Default::default(),
        links: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "1962");
}

#[test]
fn test_message_component_renders_accessed_date_argument() {
    let config = make_config();
    let mut locale = make_locale();
    locale
        .messages
        .insert("pattern.accessed-date".into(), "accessed {$date}".into());
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "web-2021".to_string(),
        ref_type: "book".to_string(),
        title: Some("Web Resource".to_string()),
        issued: Some(DateVariable::year(2021)),
        ..Default::default()
    });
    let hints = ProcHints::default();
    let component = TemplateMessage {
        message: "pattern.accessed-date".into(),
        args: [(
            "date".into(),
            MessageArgSource::Date(TemplateDate {
                date: TemplateDateVar::Issued,
                form: DateForm::Year,
                fallback: None,
                suppress_note: None,
                suppress_disamb_suffix: None,
                rendering: Default::default(),
                links: None,
                custom: None,
            }),
        )]
        .into(),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "accessed 2021");
    assert!(values.pre_formatted);
}

#[test]
fn test_message_component_renders_in_container_argument_with_formatting() {
    let config = make_config();
    let mut locale = make_locale();
    locale
        .messages
        .insert("pattern.in-container".into(), "in {$container}".into());
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "chapter-1".to_string(),
        ref_type: "chapter".to_string(),
        container_title: Some("Book Title".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();
    let component = TemplateMessage {
        message: "pattern.in-container".into(),
        args: [(
            "container".into(),
            MessageArgSource::Title(TemplateTitle {
                title: TitleType::ParentMonograph,
                rendering: Rendering {
                    emph: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )]
        .into(),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "in _Book Title_");
    assert!(values.pre_formatted);
}

#[test]
fn style_owned_mf2_message_selects_reference_type_and_carrier() {
    let mut config = make_config();
    config.messages.insert(
        "gb-t-7714-type-code".into(),
        ".match {$type :select} {$carrier :select}\nwhen book OL {M/OL}\nwhen book * {M}\nwhen * * {Z}"
            .into(),
    );
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "online-book".to_string(),
        ref_type: "book".to_string(),
        url: Some("https://example.test/book".to_string()),
        ..Default::default()
    });
    let component = TemplateMessage {
        message: "gb-t-7714-type-code".into(),
        args: [
            (
                "type".into(),
                MessageArgSource::ReferenceType {
                    reference_type: MessageReferenceTypeSource::Key,
                },
            ),
            (
                "carrier".into(),
                MessageArgSource::Carrier {
                    carrier: MessageCarrierSource {
                        online: "OL".into(),
                        absent: "none".into(),
                    },
                },
            ),
        ]
        .into(),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &ProcHints::default(), &options)
        .expect("style-owned MF2 message should render");

    assert_eq!(values.value, "M/OL");
}

#[test]
fn test_message_component_renders_grouped_container_argument() {
    let config = make_config_with_apa_role_defaults();
    let mut locale = make_locale();
    locale
        .messages
        .insert("pattern.in-container".into(), "in {$container}".into());
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "chapter-1".to_string(),
        ref_type: "chapter".to_string(),
        editor: Some(vec![Name::new("Grimm", "Jacob")]),
        container_title: Some("Book Title".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();
    let component = TemplateMessage {
        message: "pattern.in-container".into(),
        args: [(
            "container".into(),
            MessageArgSource::Group(TemplateGroup {
                group: vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Editor.into(),
                        form: ContributorForm::Long,
                        name_order: Some(NameOrder::FamilyFirst),
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentMonograph,
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ],
                delimiter: Some(DelimiterPunctuation::Custom(" ".into())),
                ..Default::default()
            }),
        )]
        .into(),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "in Grimm, Jacob (ed.) _Book Title_");
    assert!(values.pre_formatted);
}

#[test]
fn test_message_component_reorders_locale_phrase_arguments() {
    let config = make_config();
    let mut locale = make_locale();
    locale.messages.insert(
        "pattern.accessed-date".into(),
        "{$date} <<accessed>>".into(),
    );
    locale
        .messages
        .insert("pattern.in-container".into(), "{$container} <<in>>".into());
    locale
        .messages
        .insert("pattern.cited-date".into(), "{$date} <<cited>>".into());
    locale
        .messages
        .insert("pattern.available-at".into(), "{$url} <<available>>".into());
    locale.messages.insert(
        "pattern.retrieved-from".into(),
        "{$url} <<retrieved>>".into(),
    );
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "localized-phrase".to_string(),
        ref_type: "chapter".to_string(),
        container_title: Some("Book Title".to_string()),
        issued: Some(DateVariable::year(2021)),
        url: Some("https://example.test/item".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let date_arg = MessageArgSource::Date(TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        rendering: Default::default(),
        links: None,
        custom: None,
    });
    let url_arg = MessageArgSource::Variable(TemplateVariable {
        variable: SimpleVariable::Url,
        ..Default::default()
    });

    let phrase = |message: &str, name: &str, arg: MessageArgSource| {
        TemplateMessage {
            message: message.into(),
            args: [(name.into(), arg)].into(),
            ..Default::default()
        }
        .values::<PlainText>(&reference, &hints, &options)
        .expect("message phrase should render")
        .value
    };

    assert_eq!(
        phrase("pattern.accessed-date", "date", date_arg.clone()),
        "2021 <<accessed>>"
    );
    assert_eq!(
        phrase(
            "pattern.in-container",
            "container",
            MessageArgSource::Title(TemplateTitle {
                title: TitleType::ParentMonograph,
                rendering: Rendering {
                    emph: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ),
        "_Book Title_ <<in>>"
    );
    assert_eq!(
        phrase("pattern.cited-date", "date", date_arg),
        "2021 <<cited>>"
    );
    assert_eq!(
        phrase("pattern.available-at", "url", url_arg.clone()),
        "https://example.test/item <<available>>"
    );
    assert_eq!(
        phrase("pattern.retrieved-from", "url", url_arg),
        "https://example.test/item <<retrieved>>"
    );
}

#[test]
fn test_message_component_resolves_term_backed_messages() {
    let config = make_config();
    let locale = make_embedded_english_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let cases = [
        ("term.no-date", Some(TermForm::Short), "n.d."),
        ("term.edition", Some(TermForm::Short), "ed."),
        ("term.cited", Some(TermForm::Long), "cited"),
    ];

    for (message, form, expected) in cases {
        let component = TemplateMessage {
            message: message.into(),
            form,
            ..Default::default()
        };
        let values = component
            .values::<PlainText>(&reference, &hints, &options)
            .unwrap();
        assert_eq!(values.value, expected, "{message}");
    }
}

#[test]
fn test_message_component_preserves_term_rendering_options() {
    let mut config = make_config();
    config.strip_periods = Some(true);
    let locale = make_embedded_english_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();
    let component = TemplateMessage {
        message: "term.ibid".into(),
        rendering: Rendering {
            strip_periods: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "ibid");
}

#[test]
fn test_message_component_renders_embedded_container_author_group() {
    let mut config = make_config();
    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        role_substitute: std::collections::HashMap::from([(
            "container-author".to_string(),
            vec!["editor".to_string()],
        )]),
        ..Default::default()
    }));
    let mut locale = make_locale();
    locale
        .messages
        .insert("pattern.in-container".into(), "in {$container}".into());
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = InputReference::CollectionComponent(Box::new(CollectionComponent {
        id: Some("chapter-1".into()),
        r#type: MonographComponentType::Chapter,
        title: Some(Title::Single("Chapter Title".to_string())),
        container: Some(WorkRelation::Embedded(Box::new(
            InputReference::Collection(Box::new(Collection {
                r#type: CollectionType::EditedBook,
                title: Some(Title::Single("Book Title".to_string())),
                editor: Some(Contributor::StructuredName(StructuredName {
                    given: "Jane".into(),
                    family: "Editor".into(),
                    ..Default::default()
                })),
                ..Default::default()
            })),
        ))),
        ..Default::default()
    }));
    let hints = ProcHints::default();
    let component = TemplateMessage {
        message: "pattern.in-container".into(),
        args: [(
            "container".into(),
            MessageArgSource::Group(TemplateGroup {
                group: vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::ContainerAuthor.into(),
                        form: ContributorForm::Long,
                        name_order: Some(NameOrder::GivenFirst),
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentMonograph,
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ],
                delimiter: Some(DelimiterPunctuation::Custom(", ".into())),
                ..Default::default()
            }),
        )]
        .into(),
        rendering: Rendering {
            text_case: Some(TextCase::CapitalizeFirst),
            ..Default::default()
        },
        custom: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "In Jane Editor, _Book Title_");
}

#[test]
fn test_message_component_renders_legacy_container_author_group() {
    let mut config = make_config();
    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        role_substitute: std::collections::HashMap::from([(
            "container-author".to_string(),
            vec!["editor".to_string()],
        )]),
        ..Default::default()
    }));
    let mut locale = make_locale();
    locale
        .messages
        .insert("pattern.in-container".into(), "in {$container}".into());
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "chapter-1".to_string(),
        ref_type: "chapter".to_string(),
        editor: Some(vec![Name::new("Editor", "Jane")]),
        container_title: Some("Book Title".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();
    let component = TemplateMessage {
        message: "pattern.in-container".into(),
        args: [(
            "container".into(),
            MessageArgSource::Group(TemplateGroup {
                group: vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::ContainerAuthor.into(),
                        form: ContributorForm::Long,
                        name_order: Some(NameOrder::GivenFirst),
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentMonograph,
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ],
                delimiter: Some(DelimiterPunctuation::Custom(", ".into())),
                ..Default::default()
            }),
        )]
        .into(),
        rendering: Rendering {
            text_case: Some(TextCase::CapitalizeFirst),
            ..Default::default()
        },
        custom: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "In Jane Editor, _Book Title_");
}

#[test]
fn test_year_month_day_dates_inline_disambiguation_suffix_on_year() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "dated-2018".to_string(),
        ref_type: "article-magazine".to_string(),
        issued: Some(DateVariable::full(2018, 7, 14)),
        ..Default::default()
    });
    let hints = ProcHints {
        disamb_condition: true,
        group_index: 3,
        group_length: 4,
        ..Default::default()
    };

    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::YearMonthDay,
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        rendering: Default::default(),
        links: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "2018c, July 14");
    assert_eq!(values.suffix, None);
}

/// Tests the behavior of `test_et_al`.
#[test]
fn test_et_al() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("LeCun", "Yann"),
            Name::new("Bengio", "Yoshua"),
            Name::new("Hinton", "Geoffrey"),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        fallback: None,
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        label: None,
        merge: None,
        name_order: None,
        name_form: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        gender: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "LeCun et al.");
}

/// Tests the behavior of `test_et_al_delimiter_never`.
#[test]
fn test_et_al_delimiter_never() {
    use citum_schema::options::DelimiterPrecedesLast;

    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 2,
            use_first: 1,
            ..Default::default()
        });
        contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Never);
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
        ..Default::default()
    });

    let component = TemplateContributor {
        fallback: None,
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        label: None,
        merge: None,
        name_order: None,
        name_form: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        gender: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // With "never", no comma before et al.
    assert_eq!(values.value, "Smith et al.");
}

#[test]
fn test_role_substitute_uses_custom_fallback_roles_without_silent_drop() {
    let mut config = make_config();
    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        role_substitute: std::collections::HashMap::from([(
            "editor".to_string(),
            vec!["compiler".to_string()],
        )]),
        ..Default::default()
    }));

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "compiler-fallback".to_string(),
        ref_type: "book".to_string(),
        title: Some("Compiled Work".to_string()),
        extra: std::collections::HashMap::from([(
            "compiler".to_string(),
            serde_json::json!([{ "family": "Compiler", "given": "Casey" }]),
        )]),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("custom compiler fallback should render");
    assert_eq!(values.value, "Compiler, Casey");
}

#[test]
fn typed_editorial_roles_resolve_through_substitution() {
    for (role_key, data_role) in [
        (
            "annotator",
            citum_schema::reference::ContributorRole::Annotator,
        ),
        (
            "commentator",
            citum_schema::reference::ContributorRole::Commentator,
        ),
        (
            "foreword-author",
            citum_schema::reference::ContributorRole::ForewordAuthor,
        ),
        (
            "introduction-author",
            citum_schema::reference::ContributorRole::IntroductionAuthor,
        ),
        (
            "afterword-author",
            citum_schema::reference::ContributorRole::AfterwordAuthor,
        ),
    ] {
        let mut config = make_config();
        config.substitute = Some(SubstituteConfig::Explicit(Substitute {
            role_substitute: std::collections::HashMap::from([(
                "editor".to_string(),
                vec![role_key.to_string()],
            )]),
            ..Default::default()
        }));

        let locale = make_locale();
        let options = RenderOptions {
            config: Arc::new(config),
            bibliography_config: None,
            locale: &locale,
            context: RenderContext::Bibliography,
            mode: citum_schema::citation::CitationMode::NonIntegral,
            suppress_author: false,
            locator_raw: None,
            ref_type: None,
            show_semantics: true,
            current_template_index: None,
            abbreviation_map: None,
        };
        let reference = make_custom_role_reference(data_role, &[ContributorGender::Common]);
        let hints = ProcHints::default();
        let component = TemplateContributor {
            contributor: ContributorRole::Editor.into(),
            form: ContributorForm::Long,
            ..Default::default()
        };

        let values = component
            .values::<PlainText>(&reference, &hints, &options)
            .unwrap_or_else(|| panic!("{role_key} should resolve through the substitution chain"));
        assert_eq!(values.value, "Persona0, Nombre0", "{role_key}");
    }
}

/// Tests the behavior of `test_et_al_delimiter_always`.
#[test]
fn test_et_al_delimiter_always() {
    use citum_schema::options::DelimiterPrecedesLast;

    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 2,
            use_first: 1,
            ..Default::default()
        });
        contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Always);
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
        ..Default::default()
    });

    let component = TemplateContributor {
        fallback: None,
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        label: None,
        merge: None,
        name_order: None,
        name_form: None,
        delimiter: None,
        sort_separator: None,
        shorten: None,
        and: None,
        rendering: Default::default(),
        links: None,
        gender: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // With "always", comma before et al.
    assert_eq!(values.value, "Smith, et al.");
}

/// Tests the behavior of `test_demote_non_dropping_particle`.
#[test]
fn test_demote_non_dropping_particle() {
    use citum_schema::options::DemoteNonDroppingParticle;

    // Name: Ludwig van Beethoven
    let name = FlatName {
        family: Some("Beethoven".to_string()),
        given: Some("Ludwig".to_string()),
        non_dropping_particle: Some("van".to_string()),
        ..Default::default()
    };

    // Case 1: Never demote (default CSL behavior for display)
    // Inverted: "van Beethoven, Ludwig"
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        None,
        None,
        None,
        Some(&DemoteNonDroppingParticle::Never),
        None,
    );
    let res_never = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(res_never, "van Beethoven, Ludwig");

    // Case 2: Display-and-sort (demote)
    // Inverted: "Beethoven, Ludwig van"
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        None,
        None,
        None,
        Some(&DemoteNonDroppingParticle::DisplayAndSort),
        None,
    );
    let res_demote = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(res_demote, "Beethoven, Ludwig van");

    // Case 3: Sort-only (same as Never for display)
    // Inverted: "van Beethoven, Ludwig"
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        None,
        None,
        None,
        Some(&DemoteNonDroppingParticle::SortOnly),
        None,
    );
    let res_sort_only =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(res_sort_only, "van Beethoven, Ludwig");

    // Case 4: Not inverted (should be same for all)
    // "Ludwig van Beethoven"
    let ctx = make_name_format_context(
        Some(DisplayAsSort::None),
        None,
        None,
        None,
        None,
        Some(&DemoteNonDroppingParticle::DisplayAndSort),
        None,
    );
    let res_straight =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(res_straight, "Ludwig van Beethoven");
}

/// Tests the behavior of `test_initialize_with_variants_for_multi_part_given_names`.
#[test]
fn test_initialize_with_variants_for_multi_part_given_names() {
    let name = FlatName {
        family: Some("Kuhn".to_string()),
        given: Some("Thomas Samuel".to_string()),
        ..Default::default()
    };

    let init_compact = String::new();
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_compact),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let compact = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(compact, "Kuhn, TS");

    let init_space = " ".to_string();
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_space),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let space = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(space, "Kuhn, T S");

    let init_dot = ".".to_string();
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_dot),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let dot = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(dot, "Kuhn, T.S.");

    let init_dot_space = ". ".to_string();
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_dot_space),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let dot_space = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(dot_space, "Kuhn, T. S.");
}

/// Tests the behavior of `test_initialize_with_hyphen_guard`.
#[test]
fn test_initialize_with_hyphen_guard() {
    let name = FlatName {
        family: Some("Kuhn".to_string()),
        given: Some("Jean-Paul".to_string()),
        ..Default::default()
    };
    let init_dot = ".".to_string();

    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_dot),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let hyphen_default =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(hyphen_default, "Kuhn, J.-P.");

    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_dot),
        Some(false),
        Some(NameForm::Initials),
        None,
        None,
    );
    let hyphen_disabled =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(hyphen_disabled, "Kuhn, J.");
}

/// Tests `NameForm` variants: Full, `FamilyOnly`, Initials, and backward-compat defaulting.
#[test]
fn test_name_form_variants() {
    use citum_schema::options::contributors::NameForm;

    let name = FlatName {
        family: Some("Smith".to_string()),
        given: Some("John David".to_string()),
        ..Default::default()
    };

    // Full: render complete given names
    let ctx = make_name_format_context(None, None, None, None, Some(NameForm::Full), None, None);
    let full = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(full, "John David Smith");

    // FamilyOnly: suppress given names entirely
    let ctx = make_name_format_context(
        None,
        None,
        None,
        None,
        Some(NameForm::FamilyOnly),
        None,
        None,
    );
    let family_only =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(family_only, "Smith");

    // Initials with explicit initialize_with
    let init_str = ". ".to_string();
    let ctx = make_name_format_context(
        None,
        None,
        Some(&init_str),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let initials = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(initials, "J. D. Smith");

    // Initials with defaulted initialize_with (None → ". ")
    let ctx =
        make_name_format_context(None, None, None, None, Some(NameForm::Initials), None, None);
    let initials_default =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(initials_default, "J. D. Smith");

    // Semantic split: name_form=None + initialize_with=Some → Full (not Initials).
    // initialize_with only controls the separator, not the form activation.
    // The migrator is responsible for co-emitting name_form: Initials with initialize_with.
    let ctx = make_name_format_context(None, None, Some(&init_str), None, None, None, None);
    let semantic_split =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(semantic_split, "John David Smith");
}

/// Tests that Initials + defaulted separator produces correct hyphenated output.
#[test]
fn test_name_form_initials_hyphen_default_separator() {
    use citum_schema::options::contributors::NameForm;

    let name = FlatName {
        family: Some("Sartre".to_string()),
        given: Some("Jean-Paul".to_string()),
        ..Default::default()
    };

    // Default separator ". " should produce "J.-P." not "J. -P."
    let ctx =
        make_name_format_context(None, None, None, None, Some(NameForm::Initials), None, None);
    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(result, "J.-P. Sartre");
}

/// Tests the behavior of `test_template_list_suppression`.
#[test]
fn test_template_list_suppression() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let component = TemplateGroup {
        group: vec![
            TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Doi,
                ..Default::default()
            }),
            TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Url,
                ..Default::default()
            }),
        ],
        delimiter: Some(DelimiterPunctuation::Comma),
        ..Default::default()
    };

    let values = component.values::<PlainText>(&reference, &hints, &options);
    assert!(values.is_none());
}

/// Tests the behavior of `test_et_al_use_last`.
#[test]
fn test_et_al_use_last() {
    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 3,
            use_first: 1,
            use_last: Some(1),
            ..Default::default()
        });
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("LeCun", "Yann"),
            Name::new("Bengio", "Yoshua"),
            Name::new("Hinton", "Geoffrey"),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // first name (LeCun) + ellipsis + last name (Hinton)
    assert_eq!(values.value, "LeCun … Hinton");
}

/// Tests the behavior of `test_et_al_use_last_overlap`.
#[test]
fn test_et_al_use_last_overlap() {
    // Edge case: use_first + use_last >= names.len() should show all names
    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.shorten = Some(ShortenListOptions {
            min: 3,
            use_first: 2,
            use_last: Some(2),
            ..Default::default()
        });
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "overlap".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("Alpha", "A."),
            Name::new("Beta", "B."),
            Name::new("Gamma", "C."),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // use_first(2) + use_last(2) = 4 >= 3 names, so show first 2 + ellipsis + last 1
    // (skip=max(2, 3-2)=2, so last 1 name). Two names are shown before the
    // ellipsis, so the configured delimiter (default ", ") precedes it, as
    // citeproc-js does whenever more than one name is shown before the
    // ellipsis.
    assert_eq!(values.value, "Alpha & Beta, … Gamma");
}

/// When et-al-use-last shows more than one leading name, the configured delimiter precedes the ellipsis (citeproc-js behavior).
#[test]
fn test_et_al_use_last_multiple_first_names_delimiter() {
    // citeproc-js: "Smith, J., Jones, K., … Zebra, Z." — the name delimiter
    // precedes the ellipsis when more than one name is shown before it.
    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.and = None;
        contributors.shorten = Some(ShortenListOptions {
            min: 4,
            use_first: 2,
            use_last: Some(1),
            ..Default::default()
        });
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi-first".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("Smith", "John"),
            Name::new("Jones", "Karen"),
            Name::new("Alpha", "Amy"),
            Name::new("Zebra", "Zoe"),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "Smith, Jones, … Zebra");
}

/// Uses the configured name delimiter (not a hardcoded ", ") before "et al." when a delimiter is required.
#[test]
fn test_et_al_uses_configured_delimiter() {
    // citeproc-js: a "; "-delimited name list places "; " (not a hardcoded
    // ", ") before "et al." when more than one name is shown.
    use citum_schema::options::DelimiterPrecedesLast;

    let mut config = make_config();
    if let Some(ref mut contributors) = config.contributors {
        contributors.delimiter = Some("; ".into());
        contributors.shorten = Some(ShortenListOptions {
            min: 3,
            use_first: 2,
            ..Default::default()
        });
        contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Contextual);
    }

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "multi".to_string(),
        ref_type: "article-journal".to_string(),
        author: Some(vec![
            Name::new("Smith", "John"),
            Name::new("Jones", "Jane"),
            Name::new("Zebra", "Zoe"),
        ]),
        ..Default::default()
    });

    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Short,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "Smith; Jones; et al.");
}

/// Tests the behavior of `test_title_hyperlink`.
#[test]
fn test_title_hyperlink() {
    use citum_schema::options::LinksConfig;

    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "kuhn1962".to_string(),
        title: Some("The Structure of Scientific Revolutions".to_string()),
        doi: Some("10.1001/example".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        links: Some(LinksConfig {
            doi: Some(true),
            target: Some(LinkTarget::Doi),
            anchor: Some(LinkAnchor::Title),
            ..Default::default()
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(
        values.url,
        Some("https://doi.org/10.1001/example".to_string())
    );
}

/// Tests the behavior of `test_title_hyperlink_url_fallback`.
#[test]
fn test_title_hyperlink_url_fallback() {
    use citum_schema::options::LinksConfig;

    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    // Reference with URL but no DOI
    let reference = Reference::from(LegacyReference {
        id: "web2024".to_string(),
        title: Some("A Web Resource".to_string()),
        url: Some("https://example.com/resource".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        links: Some(LinksConfig {
            doi: Some(true),
            url: Some(true),
            target: Some(LinkTarget::UrlOrDoi),
            anchor: Some(LinkAnchor::Title),
            ..Default::default()
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // Falls back to URL when DOI is absent
    assert_eq!(values.url, Some("https://example.com/resource".to_string()));
}

/// Tests the behavior of `test_title_values_smarten_leading_single_quotes`.
#[test]
fn test_title_values_smarten_leading_single_quotes() {
    // Upstream provenance: CSL fixture `flipflop_LeadingSingleQuote`.
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "parmenides".to_string(),
        ref_type: "book".to_string(),
        title: Some("'Parmenides' 132c-133a and the development of Plato's thought".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(
        values.value,
        "‘Parmenides’ 132c-133a and the development of Plato’s thought"
    );
}

/// Tests the behavior of `test_title_values_smarten_starting_apostrophe`.
#[test]
fn test_title_values_smarten_starting_apostrophe() {
    // Upstream provenance: CSL fixture `flipflop_StartingApostrophe`.
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "etfa-09".to_string(),
        ref_type: "book".to_string(),
        title: Some(
            "IEEE Conference on Emerging Technologies and Factory Automation (ETFA '09)"
                .to_string(),
        ),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(
        values.value,
        "IEEE Conference on Emerging Technologies and Factory Automation (ETFA ’09)"
    );
}

/// Tests the behavior of `test_title_values_smarten_french_apostrophes`.
#[test]
fn test_title_values_smarten_french_apostrophes() {
    // Upstream provenance: adapted from CSL fixture `flipflop_Apostrophes`.
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "egypt".to_string(),
        ref_type: "book".to_string(),
        title: Some(
            "Supplément aux annales du Service des Antiquités de l'Egypte, Cahier".to_string(),
        ),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(
        values.value,
        "Supplément aux annales du Service des Antiquités de l’Egypte, Cahier"
    );
}

/// Tests straight double quotes being smartened in plain title values.
#[test]
fn test_title_values_smarten_double_quotes() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "quoted-title".to_string(),
        ref_type: "book".to_string(),
        title: Some("The \"Parmenides\" dialogue".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "The “Parmenides” dialogue");
}

/// Tests the behavior of mixed outer single and inner double title quotes.
#[test]
fn test_title_values_flip_flop_outer_single_inner_double_quotes() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "flip-flop-single-double".to_string(),
        ref_type: "book".to_string(),
        title: Some("'Some Title \"with something\"'".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "‘Some Title “with something”’");
}

/// Tests the behavior of mixed outer double and inner single title quotes.
#[test]
fn test_title_values_flip_flop_outer_double_inner_single_quotes() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "flip-flop-double-single".to_string(),
        ref_type: "book".to_string(),
        title: Some("\"Some title 'with something'\"".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "“Some title ‘with something’”");
}

/// Tests the behavior of preserving ambiguous double quotes in title values.
#[test]
fn test_title_values_preserve_ambiguous_double_quotes() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "record-title".to_string(),
        ref_type: "book".to_string(),
        title: Some("The 12\" record".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "The 12\" record");
}

/// Tests the behavior of djot-marked title values.
#[test]
fn test_title_values_render_djot_markup_as_preformatted() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "djot-title".to_string(),
        ref_type: "book".to_string(),
        title: Some("_Homo sapiens_ and *modern* world".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "_Homo sapiens_ and **modern** world");
    assert!(values.pre_formatted);
}

/// Tests the behavior of djot-marked title smart apostrophes.
#[test]
fn test_title_values_smarten_djot_text_leaves() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "djot-apostrophe".to_string(),
        ref_type: "book".to_string(),
        title: Some("_Plato's dialogue_".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "_Plato’s dialogue_");
    assert!(values.pre_formatted);
}

/// Tests the behavior of djot-marked title smart double quotes.
#[test]
fn test_title_values_smarten_djot_double_quotes() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "djot-double-quotes".to_string(),
        ref_type: "book".to_string(),
        title: Some("_\"Parmenides\" dialogue_".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "_“Parmenides” dialogue_");
    assert!(values.pre_formatted);
}

/// Tests the behavior of inline title links suppressing whole-title autolinks.
#[test]
fn test_title_values_inline_link_suppresses_outer_title_link() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "djot-link".to_string(),
        ref_type: "book".to_string(),
        title: Some("[Linked title](https://example.com)".to_string()),
        doi: Some("10.1001/test".to_string()),
        ..Default::default()
    });

    let component = TemplateTitle {
        title: TitleType::Primary,
        links: Some(LinksConfig {
            doi: Some(true),
            target: Some(LinkTarget::Doi),
            anchor: Some(LinkAnchor::Title),
            ..Default::default()
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render");
    assert_eq!(values.value, "Linked title");
    assert!(values.pre_formatted);
    assert_eq!(values.url, None);
}

/// Tests the behavior of `test_variable_hyperlink`.
#[test]
fn test_variable_hyperlink() {
    use citum_schema::options::LinksConfig;

    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "pub2024".to_string(),
        publisher: Some("MIT Press".to_string()),
        doi: Some("10.1234/pub".to_string()),
        ..Default::default()
    });

    let component = TemplateVariable {
        variable: SimpleVariable::Publisher,
        links: Some(LinksConfig {
            doi: Some(true),
            target: Some(LinkTarget::Doi),
            anchor: Some(LinkAnchor::Component),
            ..Default::default()
        }),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "MIT Press");
    assert_eq!(values.url, Some("https://doi.org/10.1234/pub".to_string()));
}

/// Tests that `strip_protocol` is a no-op when unset (default behavior unchanged).
#[test]
fn test_resolve_url_strip_protocol_unset() {
    let reference = Reference::from(LegacyReference {
        id: "web2024".to_string(),
        url: Some("https://example.com/resource".to_string()),
        ..Default::default()
    });

    let links = LinksConfig {
        target: Some(LinkTarget::Url),
        ..Default::default()
    };

    let url = resolve_url(&links, &reference);
    assert_eq!(url, Some("https://example.com/resource".to_string()));
}

/// Tests that `strip_protocol: Some(true)` strips an `https://` prefix.
#[test]
fn test_resolve_url_strip_protocol_https() {
    let reference = Reference::from(LegacyReference {
        id: "web2024".to_string(),
        url: Some("https://example.com/resource".to_string()),
        ..Default::default()
    });

    let links = LinksConfig {
        target: Some(LinkTarget::Url),
        strip_protocol: Some(true),
        ..Default::default()
    };

    let url = resolve_url(&links, &reference);
    assert_eq!(url, Some("example.com/resource".to_string()));
}

/// Tests that `strip_protocol: Some(true)` strips an `http://` prefix.
#[test]
fn test_resolve_url_strip_protocol_http() {
    let reference = Reference::from(LegacyReference {
        id: "web2024".to_string(),
        url: Some("http://example.com/resource".to_string()),
        ..Default::default()
    });

    let links = LinksConfig {
        target: Some(LinkTarget::Url),
        strip_protocol: Some(true),
        ..Default::default()
    };

    let url = resolve_url(&links, &reference);
    assert_eq!(url, Some("example.com/resource".to_string()));
}

/// Tests that `strip_protocol: Some(false)` is an explicit no-op.
#[test]
fn test_resolve_url_strip_protocol_explicit_false() {
    let reference = Reference::from(LegacyReference {
        id: "web2024".to_string(),
        url: Some("https://example.com/resource".to_string()),
        ..Default::default()
    });

    let links = LinksConfig {
        target: Some(LinkTarget::Url),
        strip_protocol: Some(false),
        ..Default::default()
    };

    let url = resolve_url(&links, &reference);
    assert_eq!(url, Some("https://example.com/resource".to_string()));
}

#[test]
fn test_report_number_variable_uses_report_number_accessor() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "report-1".to_string(),
        ref_type: "report".to_string(),
        title: Some("Report".to_string()),
        issued: Some(DateVariable::year(2024)),
        number: Some("TR-7".to_string()),
        ..Default::default()
    });

    let number_component = TemplateNumber {
        number: NumberVariable::ReportNumber,
        ..Default::default()
    };
    let variable_component = TemplateVariable {
        variable: SimpleVariable::ReportNumber,
        ..Default::default()
    };

    assert_eq!(
        number_component
            .values::<PlainText>(&reference, &hints, &options)
            .expect("report number should render")
            .value,
        "TR-7"
    );
    assert_eq!(
        variable_component
            .values::<PlainText>(&reference, &hints, &options)
            .expect("report variable should render")
            .value,
        "TR-7"
    );
}

#[test]
fn template_number_ordinal_form_uses_the_active_locale_message() {
    let config = make_config();
    let english_locale = make_embedded_english_locale();
    let chinese_locale = make_embedded_chinese_locale();
    let reference = Reference::from(LegacyReference {
        id: "edition-ordinal".to_string(),
        ref_type: "book".to_string(),
        title: Some("Edition test".to_string()),
        issued: Some(DateVariable::year(2024)),
        edition: Some(StringOrNumber::String("2".to_string())),
        ..Default::default()
    });
    let number = TemplateNumber {
        number: NumberVariable::Edition,
        form: Some(NumberForm::Ordinal),
        ..Default::default()
    };

    for (locale, expected) in [(&english_locale, "2nd"), (&chinese_locale, "第2")] {
        let options = RenderOptions {
            config: Arc::new(config.clone()),
            bibliography_config: None,
            locale,
            context: RenderContext::Bibliography,
            mode: citum_schema::citation::CitationMode::NonIntegral,
            suppress_author: false,
            locator_raw: None,
            ref_type: None,
            show_semantics: true,
            current_template_index: None,
            abbreviation_map: None,
        };

        assert_eq!(
            number
                .values::<PlainText>(&reference, &ProcHints::default(), &options)
                .expect("numeric edition should render")
                .value,
            expected
        );
    }
}

#[test]
fn template_number_localizes_digits_for_the_active_locale() {
    let config = make_config();
    let locale = make_arabic_gendered_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Citation,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "localized-number".to_string(),
        ref_type: "report".to_string(),
        page: Some("12-34".to_string()),
        number: Some("TR-7".to_string()),
        ..Default::default()
    });

    let pages = TemplateNumber {
        number: NumberVariable::Pages,
        ..Default::default()
    };
    let report_number = TemplateNumber {
        number: NumberVariable::ReportNumber,
        ..Default::default()
    };

    assert_eq!(
        pages
            .values::<PlainText>(&reference, &ProcHints::default(), &options)
            .expect("page range should render")
            .value,
        "١٢–٣٤"
    );
    assert_eq!(
        report_number
            .values::<PlainText>(&reference, &ProcHints::default(), &options)
            .expect("report number should render")
            .value,
        "TR-٧"
    );
}

#[test]
fn monograph_metadata_variables_render_their_accessors() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let reference = InputReference::Monograph(Box::new(Monograph {
        r#type: MonographType::Document,
        genre: Some("map".to_string()),
        volume_title: Some("History of Science".to_string()),
        scale: Some("1:25000".to_string()),
        version: Some("2".to_string()),
        ..Default::default()
    }));

    for (variable, expected) in [
        (SimpleVariable::VolumeTitle, "History of Science"),
        (SimpleVariable::Scale, "1:25000"),
        (SimpleVariable::Version, "2"),
    ] {
        let component = TemplateVariable {
            variable,
            ..Default::default()
        };
        assert_eq!(
            component
                .values::<PlainText>(&reference, &hints, &options)
                .expect("monograph metadata should render")
                .value,
            expected
        );
    }
}

#[test]
fn test_number_variable_excludes_report_number_accessor() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "report-1".to_string(),
        ref_type: "report".to_string(),
        title: Some("Report".to_string()),
        issued: Some(DateVariable::year(2024)),
        number: Some("TR-7".to_string()),
        ..Default::default()
    });

    let number_component = TemplateNumber {
        number: NumberVariable::Number,
        ..Default::default()
    };
    let variable_component = TemplateVariable {
        variable: SimpleVariable::Number,
        ..Default::default()
    };

    assert!(
        number_component
            .values::<PlainText>(&reference, &hints, &options)
            .is_none()
    );
    assert!(
        variable_component
            .values::<PlainText>(&reference, &hints, &options)
            .is_none()
    );
}

#[test]
fn test_custom_number_variable_renders_from_custom_numbering_kind() {
    let config = make_config();
    let locale = citum_schema::Locale::from_yaml_str(
        r#"
locale: en-US
locators:
  reel:
    short:
      singular: "reel"
      plural: "reels"
"#,
    )
    .expect("custom locale should parse");
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference: Reference = serde_json::from_str(
        r#"{
            "class": "monograph",
            "type": "book",
            "title": "Film",
            "issued": "2024",
            "numbering": [
                { "type": "reel", "value": "3" }
            ]
        }"#,
    )
    .expect("reference should parse");

    let unlabeled = TemplateNumber {
        number: NumberVariable::Custom("reel".to_string()),
        ..Default::default()
    };
    let labeled = TemplateNumber {
        number: NumberVariable::Custom("reel".to_string()),
        label_form: Some(citum_schema::template::LabelForm::Short),
        ..Default::default()
    };

    assert_eq!(
        unlabeled
            .values::<PlainText>(&reference, &hints, &options)
            .expect("custom number should render")
            .value,
        "3"
    );
    let labeled_values = labeled
        .values::<PlainText>(&reference, &hints, &options)
        .expect("custom labeled number should render");
    assert_eq!(labeled_values.value, "3");
    assert_eq!(labeled_values.prefix, Some("reel ".to_string()));
}

#[test]
fn test_custom_number_variable_normalizes_manual_custom_key() {
    let config = make_config();
    let locale = Locale::from_yaml_str(
        r#"
locale: en-US
locators:
  reel:
    short:
      singular: "reel"
      plural: "reels"
"#,
    )
    .expect("custom locale should parse");
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };

    let reference: Reference = serde_json::from_str(
        r#"{
            "class": "monograph",
            "type": "book",
            "title": "Film",
            "issued": "2024",
            "numbering": [
                { "type": "reel", "value": "3" }
            ]
        }"#,
    )
    .expect("reference should parse");

    let number = TemplateNumber {
        number: NumberVariable::Custom("Reel".to_string()),
        label_form: Some(citum_schema::template::LabelForm::Short),
        ..Default::default()
    };

    let values = number
        .values::<PlainText>(&reference, &ProcHints::default(), &options)
        .expect("custom number should render");

    assert_eq!(values.value, "3");
    assert_eq!(values.prefix, Some("reel ".to_string()));
}

#[test]
fn test_template_number_gender_overrides_locator_label_resolution() {
    let config = make_config();
    let locale = make_gendered_locator_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "book-1".to_string(),
        ref_type: "book".to_string(),
        title: Some("Libro".to_string()),
        volume: Some(csl_legacy::csl_json::StringOrNumber::String(
            "1".to_string(),
        )),
        issued: Some(DateVariable::year(2024)),
        ..Default::default()
    });

    let masculine = TemplateNumber {
        number: NumberVariable::Volume,
        label_form: Some(citum_schema::template::LabelForm::Short),
        gender: Some(GrammaticalGender::Masculine),
        ..Default::default()
    };
    let feminine = TemplateNumber {
        number: NumberVariable::Volume,
        label_form: Some(citum_schema::template::LabelForm::Short),
        gender: Some(GrammaticalGender::Feminine),
        ..Default::default()
    };

    let masculine_values = masculine
        .values::<PlainText>(&reference, &hints, &options)
        .expect("masculine volume should render");
    let feminine_values = feminine
        .values::<PlainText>(&reference, &hints, &options)
        .expect("feminine volume should render");

    assert_eq!(masculine_values.prefix, Some("tomo ".to_string()));
    assert_eq!(feminine_values.prefix, Some("entrega ".to_string()));
}

#[test]
fn test_role_label_preset_applies_to_translator_component() {
    let mut config = make_config();
    let locale = make_locale();
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "translator-test".to_string(),
        ref_type: "book".to_string(),
        translator: Some(vec![Name::new("Muller", "Anna")]),
        ..Default::default()
    });

    if let Some(ref mut contributors) = config.contributors {
        contributors.role = Some(RoleOptions {
            preset: Some(RoleLabelPreset::LongSuffix),
            ..Default::default()
        });
    }

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let component = TemplateContributor {
        contributor: ContributorRole::Translator.into(),
        form: ContributorForm::Long,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "Muller, Anna");
    assert_eq!(values.suffix, Some(", translator".to_string()));
}

#[test]
fn test_translator_substitute_uses_locale_aware_role_label() {
    let mut config = make_config();
    let locale = make_locale();
    let hints = ProcHints::default();

    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        contributor_role_form: Some("long".to_string()),
        contributor_role_case: None,
        template: vec![SubstituteKey::Translator],
        overrides: std::collections::HashMap::new(),
        role_substitute: std::collections::HashMap::new(),
        title_quote: None,
        unknown_fields: Default::default(),
    }));

    let reference = Reference::from(LegacyReference {
        id: "translator-substitute".to_string(),
        ref_type: "book".to_string(),
        translator: Some(vec![Name::new("Muller", "Anna")]),
        ..Default::default()
    });

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Long,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "Muller, Anna");
    assert_eq!(values.suffix, Some(", translator".to_string()));
    assert_eq!(
        values.substituted_key,
        Some("contributor:Translator".to_string())
    );
}

#[test]
fn test_editor_substitute_suppresses_verb_prefix_role_label() {
    let mut config = make_config();
    let locale = make_locale();
    let hints = ProcHints::default();

    config.substitute = Some(SubstituteConfig::Preset(
        citum_schema::presets::SubstitutePreset::Standard,
    ));

    if let Some(ref mut contributors) = config.contributors {
        contributors.role = Some(RoleOptions {
            preset: Some(RoleLabelPreset::VerbPrefix),
            ..Default::default()
        });
    }

    let reference = Reference::from(LegacyReference {
        id: "editor-substitute".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Grimm", "Jacob")]),
        ..Default::default()
    });

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Long,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "Grimm, Jacob");
    assert_eq!(values.prefix, None);
    assert_eq!(values.suffix, None);
    assert_eq!(
        values.substituted_key,
        Some("contributor:Editor".to_string())
    );
}

#[test]
fn test_editor_component_keeps_verb_prefix_role_label() {
    let mut config = make_config();
    let locale = make_locale();
    let hints = ProcHints::default();

    if let Some(ref mut contributors) = config.contributors {
        contributors.role = Some(RoleOptions {
            preset: Some(RoleLabelPreset::VerbPrefix),
            ..Default::default()
        });
    }

    let reference = Reference::from(LegacyReference {
        id: "editor-component".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Grimm", "Jacob")]),
        ..Default::default()
    });

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "Grimm, Jacob");
    assert_eq!(values.prefix, Some("edited by ".to_string()));
    assert_eq!(values.suffix, None);
}

#[test]
fn test_role_substitute_normalizes_primary_role_lookup_keys() {
    let mut config = make_config();
    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        role_substitute: std::collections::HashMap::from([(
            "container_author".to_string(),
            vec!["Editor".to_string()],
        )]),
        ..Default::default()
    }));

    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "normalized-role-key".to_string(),
        ref_type: "chapter".to_string(),
        title: Some("Collected Work".to_string()),
        editor: Some(vec![Name::new("Editor", "Avery")]),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let component = TemplateContributor {
        contributor: ContributorRole::ContainerAuthor.into(),
        form: ContributorForm::Long,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "Editor, Avery");
    assert_eq!(values.suffix, None);
}

#[test]
fn test_role_specific_name_order_applies_in_substitute_path() {
    let mut config = make_config();
    let locale = make_locale();
    let hints = ProcHints::default();

    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        contributor_role_form: Some("short".to_string()),
        contributor_role_case: None,
        template: vec![SubstituteKey::Translator],
        overrides: std::collections::HashMap::new(),
        role_substitute: std::collections::HashMap::new(),
        title_quote: None,
        unknown_fields: Default::default(),
    }));

    if let Some(ref mut contributors) = config.contributors {
        contributors.role = Some(RoleOptions {
            roles: Some({
                let mut roles = std::collections::HashMap::new();
                roles.insert(
                    "translator".to_string(),
                    RoleRendering {
                        name_order: Some(NameOrder::GivenFirst),
                        ..Default::default()
                    },
                );
                roles
            }),
            ..Default::default()
        });
    }

    let reference = Reference::from(LegacyReference {
        id: "translator-name-order".to_string(),
        ref_type: "book".to_string(),
        translator: Some(vec![Name::new("Muller", "Anna")]),
        ..Default::default()
    });

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Long,
        links: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "Anna Muller");
    // CSL reference locale (scripts/locales-en-US.xml) has translator short
    // form "trans." (lowercase, no plural variant) — the base engine locale
    // now single-sources from the YAML asset which matches this; styles
    // that need a capitalized label (e.g. APA) apply `text-case:
    // capitalize-first` at the style level instead.
    assert_eq!(values.suffix, Some(" (trans.)".to_string()));
}

/// Tests the behavior of `test_term_values`.
#[test]
fn test_term_values() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateTerm {
        term: GeneralTerm::In,
        form: Some(TermForm::Long),
        custom: None,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "in");
}

/// Tests the behavior of `test_template_list_term_suppression`.
#[test]
fn test_template_list_term_suppression() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    // Reference with no editor
    let reference = make_reference();
    let hints = ProcHints::default();

    let component = TemplateGroup {
        group: vec![
            TemplateComponent::Term(TemplateTerm {
                term: GeneralTerm::In,
                custom: None,
                ..Default::default()
            }),
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Editor.into(),
                ..Default::default()
            }),
        ],
        delimiter: Some(DelimiterPunctuation::Space),
        ..Default::default()
    };

    let values = component.values::<PlainText>(&reference, &hints, &options);
    // Should be None because only the term "In" would render, and it's suppressed if no content-bearing items are present
    assert!(values.is_none());
}

/// Tests the behavior of `test_date_fallback`.
#[test]
fn test_date_fallback() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    // Reference with NO issued date
    let reference = Reference::from(LegacyReference {
        id: "no-date".to_string(),
        ref_type: "book".to_string(),
        author: Some(vec![Name::new("Aristotle", "Ancient")]),
        title: Some("Poetics".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: Some(vec![TemplateComponent::Term(TemplateTerm {
            term: GeneralTerm::NoDate,
            form: Some(TermForm::Short),
            ..Default::default()
        })]),
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "n.d.");
}

/// Given an explicit empty fallback, a missing issued date is omitted instead of
/// using the implicit locale no-date term.
#[test]
fn given_empty_date_fallback_when_issued_missing_then_date_is_omitted() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "no-date".to_string(),
        ref_type: "book".to_string(),
        title: Some("Poetics".to_string()),
        ..Default::default()
    });
    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: Some(Vec::new()),
        ..Default::default()
    };

    let values = component.values::<PlainText>(&reference, &ProcHints::default(), &options);

    assert!(values.is_none());
}

/// Given an explicit fallback whose components are also missing, a missing issued
/// date is omitted instead of continuing to the implicit locale no-date term.
#[test]
fn given_exhausted_date_fallback_when_issued_missing_then_date_is_omitted() {
    let config = make_config();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let reference = Reference::from(LegacyReference {
        id: "no-date-or-doi".to_string(),
        ref_type: "book".to_string(),
        title: Some("Poetics".to_string()),
        ..Default::default()
    });
    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: Some(vec![TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Doi,
            ..Default::default()
        })]),
        ..Default::default()
    };

    let values = component.values::<PlainText>(&reference, &ProcHints::default(), &options);

    assert!(values.is_none());
}

#[test]
fn test_author_fallback_renders_anonymous_term_when_substitute_chain_is_empty() {
    // csl26-6eak: GB/T 7714's `佚名` (anonymous-author) placeholder. The
    // reference has no author, no editor/title/translator for the default
    // substitute chain to promote, so `TemplateContributor.fallback`
    // (mirroring `TemplateDate.fallback`) is consulted as the last resort.
    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("no-author-1947".into()),
        r#type: MonographType::Book,
        issued: DateValue {
            value: "1947".to_string(),
            note: None,
        },
        ..Default::default()
    }));
    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Long,
        fallback: Some(vec![TemplateComponent::Message(TemplateMessage {
            message: "term.anonymous".to_string(),
            form: Some(TermForm::Short),
            ..Default::default()
        })]),
        ..Default::default()
    };
    let config = Config {
        substitute: Some(citum_schema::options::SubstituteConfig::Explicit(
            citum_schema::options::Substitute {
                template: Vec::new(),
                ..Default::default()
            },
        )),
        ..Default::default()
    };
    let locale = make_embedded_chinese_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };

    let values = component
        .values::<PlainText>(&reference, &ProcHints::default(), &options)
        .expect("anonymous-term fallback should render when the substitute chain is empty");

    assert_eq!(values.value, "佚名");
}

/// Tests the behavior of `test_strip_periods_global_config`.
#[test]
fn test_strip_periods_global_config() {
    let mut config = make_config_with_apa_role_defaults();
    config.strip_periods = Some(true);
    let locale = make_locale();
    let reference = Reference::from(LegacyReference {
        id: "editor1".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "John")]),
        title: Some("A Book".to_string()),
        issued: Some(DateVariable::year(2020)),
        publisher: Some("Publisher".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // Should have "(ed)" instead of "(ed.)" due to strip_periods
    assert!(values.suffix.is_some());
    assert_eq!(values.suffix.as_ref().unwrap(), " (ed)");
}

/// Tests the behavior of `test_strip_periods_component_override`.
#[test]
fn test_strip_periods_component_override() {
    let mut config = make_config_with_apa_role_defaults();
    config.strip_periods = Some(false); // Global is false
    let locale = make_locale();
    let reference = Reference::from(LegacyReference {
        id: "editor1".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "John")]),
        title: Some("A Book".to_string()),
        issued: Some(DateVariable::year(2020)),
        publisher: Some("Publisher".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };

    // Component overrides global setting
    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        rendering: Rendering {
            strip_periods: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // Should strip periods because component overrides global
    assert!(values.suffix.is_some());
    assert_eq!(values.suffix.as_ref().unwrap(), " (ed)");
}

/// Tests the behavior of `test_strip_periods_no_strip_by_default`.
#[test]
fn test_strip_periods_no_strip_by_default() {
    let config = make_config_with_apa_role_defaults();
    let locale = make_locale();
    let reference = Reference::from(LegacyReference {
        id: "editor1".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "John")]),
        title: Some("A Book".to_string()),
        issued: Some(DateVariable::year(2020)),
        publisher: Some("Publisher".to_string()),
        ..Default::default()
    });
    let hints = ProcHints::default();

    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };

    let component = TemplateContributor {
        contributor: ContributorRole::Editor.into(),
        form: ContributorForm::Long,
        ..Default::default()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // Should preserve periods by default
    assert!(values.suffix.is_some());
    assert_eq!(values.suffix.as_ref().unwrap(), " (ed.)");
}

/// Tests the behavior of `test_strip_trailing_periods`.
#[test]
fn test_strip_trailing_periods() {
    assert_eq!(strip_trailing_periods("test."), "test");
    assert_eq!(strip_trailing_periods("test"), "test");
    assert_eq!(strip_trailing_periods("Ph.D."), "Ph.D");
    assert_eq!(strip_trailing_periods("A.B.C."), "A.B.C");
    assert_eq!(strip_trailing_periods("..."), "");
}

/// Tests the behavior of `test_should_strip_periods_precedence`.
#[test]
fn test_should_strip_periods_precedence() {
    let config = Config {
        strip_periods: Some(true),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };

    // Component override takes precedence
    let rendering_override_true = Rendering {
        strip_periods: Some(true),
        ..Default::default()
    };
    assert!(should_strip_periods(&rendering_override_true, &options));

    let rendering_override_false = Rendering {
        strip_periods: Some(false),
        ..Default::default()
    };
    assert!(!should_strip_periods(&rendering_override_false, &options));

    // Falls back to config when component has None
    let rendering_default = Rendering::default();
    assert!(should_strip_periods(&rendering_default, &options));

    // Defaults to false when both are None
    let config_none = Config::default();
    let options_none = RenderOptions {
        config: Arc::new(config_none),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    assert!(!should_strip_periods(&rendering_default, &options_none));
}

/// Tests the behavior of `test_sort_separator_space`.
#[test]
fn test_sort_separator_space() {
    use citum_schema::options::DisplayAsSort;

    // Test sort_separator directly via format_single_name with inverted display
    let name = FlatName {
        family: Some("Smith".to_string()),
        given: Some("John".to_string()),
        ..Default::default()
    };

    // Test with space separator: should produce "Smith J" (no comma)
    let sep_space = " ".to_string();
    let init_empty = String::new();
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_empty),
        None,
        Some(NameForm::Initials),
        None,
        Some(&sep_space),
    );
    let result_space =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(result_space, "Smith J");

    // Test with default (no sort_separator set): should produce "Smith, J" (with comma)
    let ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        Some(&init_empty),
        None,
        Some(NameForm::Initials),
        None,
        None,
    );
    let result_default =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(result_default, "Smith, J");
}

/// citeproc-js places the sort-separator before a generational suffix in
/// inverted name order ("Smith, J., Jr."), not a plain space
/// ("Smith, J. Jr."). No prior test populated `FlatName.suffix` for an
/// inverted name.
#[test]
fn test_suffix_uses_sort_separator_not_space() {
    let name = FlatName {
        family: Some("Smith".to_string()),
        given: Some("J.".to_string()),
        suffix: Some("Jr.".into()),
        ..Default::default()
    };

    let ctx =
        make_name_format_context(Some(DisplayAsSort::All), None, None, None, None, None, None);
    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);
    assert_eq!(result, "Smith, J., Jr.");
}

#[test]
fn katakana_names_use_script_delimiter_in_original_order() {
    let name = FlatName {
        family: Some("ジャクソン".to_string()),
        given: Some("マイケル".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "katakana".to_string(),
        ScriptConfig {
            delimiter: Some("・".into()),
            ..Default::default()
        },
    )]);
    let mut ctx =
        make_name_format_context(None, None, None, None, Some(NameForm::Full), None, None);
    ctx.script_configs = Some(&scripts);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    assert_eq!(result, "マイケル・ジャクソン");
}

#[test]
fn katakana_names_use_script_sort_separator_when_inverted() {
    let name = FlatName {
        family: Some("ジャクソン".to_string()),
        given: Some("マイケル".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "katakana".to_string(),
        ScriptConfig {
            delimiter: Some("・".into()),
            sort_separator: Some("、".to_string()),
            ..Default::default()
        },
    )]);
    let mut ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        None,
        None,
        Some(NameForm::Full),
        None,
        None,
    );
    ctx.script_configs = Some(&scripts);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    assert_eq!(result, "ジャクソン、マイケル");
}

/// Reproduces the forum use-case verbatim:
///   マイケル・ジャクソン  (original order)
///   ジャクソン、マイケル  (inverted order)
/// Both outputs come from the same style config with one `katakana` entry.
#[test]
fn katakana_name_renders_correctly_in_both_orders() {
    let name = FlatName {
        family: Some("ジャクソン".to_string()),
        given: Some("マイケル".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "katakana".to_string(),
        ScriptConfig {
            delimiter: Some("・".into()),
            sort_separator: Some("、".to_string()),
            ..Default::default()
        },
    )]);

    let mut ctx_original =
        make_name_format_context(None, None, None, None, Some(NameForm::Full), None, None);
    ctx_original.script_configs = Some(&scripts);
    let original =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx_original, false);

    let mut ctx_inverted = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        None,
        None,
        Some(NameForm::Full),
        None,
        None,
    );
    ctx_inverted.script_configs = Some(&scripts);
    let inverted =
        contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx_inverted, false);

    assert_eq!(original, "マイケル・ジャクソン");
    assert_eq!(inverted, "ジャクソン、マイケル");
}

#[test]
fn native_cjk_names_use_native_ordering_without_space() {
    let name = FlatName {
        family: Some("北川".to_string()),
        given: Some("善太郎".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "cjk".to_string(),
        ScriptConfig {
            use_native_ordering: true,
            delimiter: Some(String::new()),
            ..Default::default()
        },
    )]);
    let mut ctx =
        make_name_format_context(None, None, None, None, Some(NameForm::Full), None, None);
    ctx.script_configs = Some(&scripts);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    assert_eq!(result, "北川善太郎");
}

#[test]
fn latin_names_ignore_unmatched_script_separators() {
    let name = FlatName {
        family: Some("Jackson".to_string()),
        given: Some("Michael".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "katakana".to_string(),
        ScriptConfig {
            delimiter: Some("・".into()),
            sort_separator: Some("、".to_string()),
            ..Default::default()
        },
    )]);
    let mut ctx =
        make_name_format_context(None, None, None, None, Some(NameForm::Full), None, None);
    ctx.script_configs = Some(&scripts);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    assert_eq!(result, "Michael Jackson");
}

#[test]
fn component_sort_separator_overrides_script_sort_separator() {
    let name = FlatName {
        family: Some("ジャクソン".to_string()),
        given: Some("マイケル".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "katakana".to_string(),
        ScriptConfig {
            sort_separator: Some("、".to_string()),
            ..Default::default()
        },
    )]);
    let component_separator = " / ".to_string();
    let mut ctx = make_name_format_context(
        Some(DisplayAsSort::All),
        None,
        None,
        None,
        Some(NameForm::Full),
        None,
        None,
    );
    ctx.script_configs = Some(&scripts);
    ctx.component_sort_separator = Some(&component_separator);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    assert_eq!(result, "ジャクソン / マイケル");
}

#[test]
fn mixed_kana_names_match_kana_config() {
    // Family in Hiragana, given in Katakana — mixed kana should resolve to "kana" key.
    let name = FlatName {
        family: Some("やまだ".to_string()), // hiragana
        given: Some("タロウ".to_string()),  // katakana
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "kana".to_string(),
        ScriptConfig {
            delimiter: Some("・".into()),
            ..Default::default()
        },
    )]);
    let mut ctx =
        make_name_format_context(None, None, None, None, Some(NameForm::Full), None, None);
    ctx.script_configs = Some(&scripts);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    assert_eq!(result, "タロウ・やまだ");
}

#[test]
fn explicit_name_order_overrides_use_native_ordering() {
    let name = FlatName {
        family: Some("北川".to_string()),
        given: Some("善太郎".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "cjk".to_string(),
        ScriptConfig {
            use_native_ordering: true,
            delimiter: Some(String::new()),
            ..Default::default()
        },
    )]);
    let order = NameOrder::GivenFirst;
    let mut ctx = make_name_format_context(
        None,
        Some(&order),
        None,
        None,
        Some(NameForm::Full),
        None,
        None,
    );
    ctx.script_configs = Some(&scripts);

    let result = contributor::format_single_name(&name, &ContributorForm::Long, 0, &ctx, false);

    // Explicit given-first order wins over script use-native-ordering.
    assert_eq!(result, "善太郎北川");
}

#[test]
fn native_ordering_applies_when_display_as_sort_is_set() {
    // display-as-sort: first inverts only the first name (index 0).
    // For a subsequent name (index 1) that is not inverted, use-native-ordering must
    // still apply — the global display_as_sort setting must not suppress it.
    let name = FlatName {
        family: Some("北川".to_string()),
        given: Some("善太郎".to_string()),
        ..Default::default()
    };
    let scripts = std::collections::HashMap::from([(
        "cjk".to_string(),
        ScriptConfig {
            use_native_ordering: true,
            delimiter: Some(String::new()),
            ..Default::default()
        },
    )]);
    let mut ctx = make_name_format_context(
        Some(DisplayAsSort::First),
        None,
        None,
        None,
        Some(NameForm::Full),
        None,
        None,
    );
    ctx.script_configs = Some(&scripts);

    // Index 1 — not inverted by display-as-sort:first; native ordering must apply.
    let result = contributor::format_single_name(&name, &ContributorForm::Long, 1, &ctx, false);

    assert_eq!(result, "北川善太郎");
}

/// Tests the behavior of `preferred_transliteration_exact_match`.
#[test]
fn preferred_transliteration_exact_match() {
    use citum_schema::reference::types::{MultilingualComplex, MultilingualString};
    use std::collections::HashMap;

    let s = MultilingualString::Complex(MultilingualComplex {
        original: "战争".to_string(),
        lang: None,
        sort_as: None,
        transliterations: vec![
            ("zh-Latn-wadegile".to_string(), "Chan-cheng".to_string()),
            ("zh-Latn-pinyin".to_string(), "Zhànzhēng".to_string()),
        ]
        .into_iter()
        .collect(),
        translations: HashMap::new(),
    });
    let result = super::resolve_multilingual_string(
        &s,
        Some(&citum_schema::options::MultilingualMode::Transliterated),
        Some(&["zh-Latn-wadegile".to_string()]),
        None,
        "en",
    );
    assert_eq!(result, "Chan-cheng");
}

/// Tests the behavior of `preferred_transliteration_substring_match`.
#[test]
fn preferred_transliteration_substring_match() {
    use citum_schema::reference::types::{MultilingualComplex, MultilingualString};
    use std::collections::HashMap;

    let s = MultilingualString::Complex(MultilingualComplex {
        original: "战争".to_string(),
        lang: None,
        sort_as: None,
        transliterations: vec![("zh-Latn-pinyin".to_string(), "Zhànzhēng".to_string())]
            .into_iter()
            .collect(),
        translations: HashMap::new(),
    });
    let result = super::resolve_multilingual_string(
        &s,
        Some(&citum_schema::options::MultilingualMode::Transliterated),
        Some(&["zh-Latn".to_string()]),
        None,
        "en",
    );
    assert_eq!(result, "Zhànzhēng");
}

/// Tests the behavior of `preferred_transliteration_fallback_to_preferred_script`.
#[test]
fn preferred_transliteration_fallback_to_preferred_script() {
    use citum_schema::reference::types::{MultilingualComplex, MultilingualString};
    use std::collections::HashMap;

    let s = MultilingualString::Complex(MultilingualComplex {
        original: "战争".to_string(),
        lang: None,
        sort_as: None,
        transliterations: vec![("zh-Latn-pinyin".to_string(), "Zhànzhēng".to_string())]
            .into_iter()
            .collect(),
        translations: HashMap::new(),
    });
    let script = "Latn".to_string();
    let result = super::resolve_multilingual_string(
        &s,
        Some(&citum_schema::options::MultilingualMode::Transliterated),
        None,
        Some(&script),
        "en",
    );
    assert_eq!(result, "Zhànzhēng");
}

/// Tests the behavior of `preferred_transliteration_fallback_to_original`.
#[test]
fn preferred_transliteration_fallback_to_original() {
    use citum_schema::reference::types::{MultilingualComplex, MultilingualString};
    use std::collections::HashMap;

    let s = MultilingualString::Complex(MultilingualComplex {
        original: "战争".to_string(),
        lang: None,
        sort_as: None,
        transliterations: HashMap::new(),
        translations: HashMap::new(),
    });
    let result = super::resolve_multilingual_string(
        &s,
        Some(&citum_schema::options::MultilingualMode::Transliterated),
        None,
        None,
        "en",
    );
    assert_eq!(result, "战争");
}

/// Tests `int_to_letter` edge case: zero input returns None.
///
/// Verifies that `int_to_letter` correctly rejects invalid input (0).
#[test]
fn test_int_to_letter_zero_edge_case() {
    assert_eq!(int_to_letter(0), None);
}

/// Tests `int_to_letter` for large numbers (triple letters and beyond).
///
/// Verifies that `int_to_letter` correctly converts 27→aa, 52→az, 53→ba,
/// demonstrating base-26 conversion with wrapping.
#[test]
fn test_int_to_letter_large_number() {
    assert_eq!(int_to_letter(27), Some("aa".to_string()));
    assert_eq!(int_to_letter(52), Some("az".to_string()));
    assert_eq!(int_to_letter(53), Some("ba".to_string()));
}

/// Tests locator label selection with different value formats.
///
/// Verifies that `check_plural` correctly identifies plural locators:
/// ranges (hyphens), lists (commas), and conjunctions (ampersands).
#[test]
fn test_locator_label_selection_comprehensive() {
    use citum_schema::citation::LocatorType;

    // Single value: not plural
    assert!(!crate::values::number::check_plural(
        "15",
        &LocatorType::Page
    ));

    // Range with hyphen: plural
    assert!(crate::values::number::check_plural(
        "15-20",
        &LocatorType::Page
    ));

    // List with comma: plural
    assert!(crate::values::number::check_plural(
        "15, 18",
        &LocatorType::Page
    ));

    // Conjunction with ampersand: plural
    assert!(crate::values::number::check_plural(
        "15 & 18",
        &LocatorType::Page
    ));
}

// ── Title text-case tests ──────────────────────────────────────────────

fn make_config_with_titles(titles: citum_schema::options::TitlesConfig) -> Config {
    Config {
        titles: Some(titles),
        ..Default::default()
    }
}

fn title_value_with_config(title_str: &str, ref_type: &str, config: &Config) -> String {
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config.clone()),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let reference = Reference::from(LegacyReference {
        id: "tc".to_string(),
        ref_type: ref_type.to_string(),
        title: Some(title_str.to_string()),
        ..Default::default()
    });
    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };
    component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap()
        .value
}

fn structured_title_value_with_config(
    title: citum_schema::reference::types::StructuredTitle,
    config: &Config,
    language: Option<&str>,
    form: Option<TitleForm>,
) -> String {
    let locale = make_locale();
    structured_title_value_with_config_and_locale(title, config, &locale, language, form)
}

fn structured_title_value_with_config_and_locale(
    title: citum_schema::reference::types::StructuredTitle,
    config: &Config,
    locale: &Locale,
    language: Option<&str>,
    form: Option<TitleForm>,
) -> String {
    let options = RenderOptions {
        config: Arc::new(config.clone()),
        bibliography_config: None,
        locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let mut reference = Reference::from(LegacyReference {
        id: "structured-delimiter".to_string(),
        ref_type: "book".to_string(),
        title: Some("placeholder".to_string()),
        language: language.map(Into::into),
        ..Default::default()
    });
    if let ClassExtension::Monograph(monograph) = reference.extension_mut() {
        monograph.title = Some(Title::Structured(title));
    }

    let component = TemplateTitle {
        title: TitleType::Primary,
        form,
        ..Default::default()
    };
    component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap()
        .value
}

#[test]
fn test_text_case_sentence_apa_basic() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result =
        title_value_with_config("The Structure of Scientific Revolutions", "book", &config);
    assert_eq!(result, "The structure of scientific revolutions");
}

#[test]
fn test_text_case_sentence_nlm_basic() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceNlm),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result =
        title_value_with_config("The Structure of Scientific Revolutions", "book", &config);
    assert_eq!(result, "The structure of scientific revolutions");
}

#[test]
fn test_text_case_title_case() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::Title),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = title_value_with_config("the quick brown fox", "book", &config);
    assert_eq!(result, "The Quick Brown Fox");
}

#[test]
fn test_text_case_as_is() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::AsIs),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result =
        title_value_with_config("The Structure of Scientific Revolutions", "book", &config);
    assert_eq!(result, "The Structure of Scientific Revolutions");
}

#[test]
fn test_text_case_nocase_protection_in_djot() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    // [mRNA]{.nocase} should be preserved even under sentence case
    let result = title_value_with_config(
        "The Role of [mRNA]{.nocase} in Modern Science",
        "book",
        &config,
    );
    assert_eq!(result, "The role of mRNA in modern science");
}

#[test]
fn test_text_case_nocase_nested_in_emphasis() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = title_value_with_config(
        "_Homo Sapiens_ and [DNA]{.nocase} Replication",
        "book",
        &config,
    );
    // Emphasis content gets sentence case (first word capitalized), DNA preserved
    assert_eq!(result, "_Homo sapiens_ and DNA replication");
}

#[test]
fn test_text_case_leading_nocase_advances_state() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    // Leading .nocase span: the first-word state should still advance,
    // so "replication" after the span is NOT capitalized.
    let result = title_value_with_config(
        "[DNA]{.nocase} Replication in Modern Science",
        "book",
        &config,
    );
    assert_eq!(result, "DNA replication in modern science");
}

/// As [`substitute_title_value`], but lets the caller supply the monograph
/// title-category rendering and the style's `substitute.title-quote` mode
/// directly, so tests can exercise category `emph`/`strong`/`small-caps`
/// (csl26-0dca) together with the div-011 `by-category` quoting mode.
fn substitute_title_value_with_rendering(
    title_str: &str,
    context: RenderContext,
    monograph_rendering: citum_schema::options::titles::TitleRendering,
    title_quote: Option<citum_schema::options::SubstituteTitleQuoteMode>,
) -> String {
    let mut config = make_config_with_titles(citum_schema::options::TitlesConfig {
        monograph: Some(monograph_rendering),
        ..Default::default()
    });
    config.substitute = Some(SubstituteConfig::Explicit(Substitute {
        title_quote,
        ..Default::default()
    }));
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let reference = InputReference::Monograph(Box::new(Monograph {
        r#type: MonographType::Book,
        title: Some(Title::Single(title_str.to_string())),
        ..Default::default()
    }));
    let component = TemplateContributor {
        contributor: ContributorRole::Author.into(),
        form: ContributorForm::Long,
        ..Default::default()
    };
    component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("author-substitute title should render")
        .value
}

/// Renders the `author` contributor role for an author-less monograph under
/// `text-case: sentence-apa`, forcing the default `editor -> title ->
/// translator` substitute chain to promote the title into the author slot
/// (csl26-d3kj). Built natively (no `csl_legacy` round-trip) so this
/// exercises the same construction as [`sentence_apa_monograph_title_value`],
/// isolating the substitute path's Djot handling from the direct title path.
fn substitute_title_value(title_str: &str, context: RenderContext) -> String {
    substitute_title_value_with_rendering(
        title_str,
        context,
        citum_schema::options::titles::TitleRendering {
            text_case: Some(citum_schema::options::titles::TextCase::SentenceApa),
            ..Default::default()
        },
        None,
    )
}

#[rstest]
#[case::nocase_span_markup_stripped_and_case_protected(
    "The Role of [mRNA]{.nocase} in Modern Science",
    "The role of mRNA in modern science"
)]
#[case::emphasis_markup_rendered_not_leaked_verbatim(
    "_Homo Sapiens_ and [DNA]{.nocase} Replication",
    "_Homo sapiens_ and DNA replication"
)]
#[case::plain_text_without_markup_still_case_transformed(
    "The Role of DNA in Modern Science",
    "The role of DNA in modern science"
)]
fn given_author_less_reference_with_djot_title_when_author_substituted_then_matches_direct_title_path(
    #[case] input: &str,
    #[case] expected: &str,
) {
    // csl26-d3kj: previously the substitute path applied case to the raw
    // string and skipped Djot rendering entirely, leaking `[...]{.nocase}`
    // verbatim. These expectations are identical to what
    // `TemplateTitle::values` produces for the same string outside the
    // substitute chain (see `test_text_case_nocase_protection_in_djot` and
    // `test_text_case_nocase_nested_in_emphasis`).
    assert_eq!(
        substitute_title_value(input, RenderContext::Bibliography),
        expected
    );
}

#[test]
fn given_author_less_reference_in_citation_context_when_title_substituted_then_quoted_without_markup_leak()
 {
    let result = substitute_title_value("[Library of Congress]{.nocase}", RenderContext::Citation);
    assert_eq!(result, "“Library of Congress”");
}

// ── Substituted-title category emphasis tests (csl26-0dca) ────────────────
//
// A title promoted into the author slot by the substitute chain renders
// through this contributor component, not `TemplateTitle`, so it never
// reached `render_component_detailed_with_format_and_renderer`'s
// `titles:`-category emph/strong/small-caps application. See div-011 for
// the either/or contract with quoting that these tests pin down.

#[rstest]
#[case::emph(
    citum_schema::options::titles::TitleRendering {
        emph: Some(true),
        ..Default::default()
    },
    "_Some Book Title_"
)]
#[case::strong(
    citum_schema::options::titles::TitleRendering {
        strong: Some(true),
        ..Default::default()
    },
    "**Some Book Title**"
)]
#[case::small_caps(
    citum_schema::options::titles::TitleRendering {
        small_caps: Some(true),
        ..Default::default()
    },
    "SOME BOOK TITLE"
)]
fn given_category_emphasis_when_title_substituted_in_bibliography_then_title_is_emphasized(
    #[case] rendering: citum_schema::options::titles::TitleRendering,
    #[case] expected: &str,
) {
    // csl26-0dca: bibliography context never quotes a substituted title, so
    // category emphasis always applies there (matching the non-substitute
    // `title:` component path).
    assert_eq!(
        substitute_title_value_with_rendering(
            "Some Book Title",
            RenderContext::Bibliography,
            rendering,
            None,
        ),
        expected
    );
}

#[test]
fn given_category_emphasis_when_title_substituted_in_citation_then_default_quoting_wins_without_emphasis()
 {
    // csl26-0dca / div-011: the historical default (`title-quote` unset,
    // i.e. `Always`) must stay byte-identical — quoting and category
    // emphasis are either/or, and the default keeps the quote.
    let result = substitute_title_value_with_rendering(
        "Some Book Title",
        RenderContext::Citation,
        citum_schema::options::titles::TitleRendering {
            emph: Some(true),
            ..Default::default()
        },
        None,
    );
    assert_eq!(result, "“Some Book Title”");
}

#[test]
fn given_by_category_title_quote_when_title_substituted_in_citation_then_title_is_emphasized_not_quoted()
 {
    // div-011's `by-category` mode defers quoting to the `titles:` category
    // rendering; with no `quote: true` on that category, the substituted
    // title is unquoted and — per csl26-0dca — now picks up the category's
    // `emph`, completing the "italicize instead of quoting" contract the
    // divergence register documents.
    let result = substitute_title_value_with_rendering(
        "Some Book Title",
        RenderContext::Citation,
        citum_schema::options::titles::TitleRendering {
            emph: Some(true),
            ..Default::default()
        },
        Some(citum_schema::options::SubstituteTitleQuoteMode::ByCategory),
    );
    assert_eq!(result, "_Some Book Title_");
}

#[test]
fn given_djot_markup_and_category_emphasis_when_title_substituted_then_case_and_emphasis_compose() {
    // csl26-0dca composes with csl26-d3kj: Djot nocase-protection, text-case,
    // and category emphasis must all apply together, in that order (Djot
    // rendering happens first, category emphasis wraps the result).
    let result = substitute_title_value_with_rendering(
        "The Role of [mRNA]{.nocase} in Modern Science",
        RenderContext::Bibliography,
        citum_schema::options::titles::TitleRendering {
            text_case: Some(citum_schema::options::titles::TextCase::SentenceApa),
            emph: Some(true),
            ..Default::default()
        },
        None,
    );
    assert_eq!(result, "_The role of mRNA in modern science_");
}

/// Renders a monograph's primary title under `text-case: sentence-apa`,
/// built natively (no `csl_legacy` round-trip) so markup-bearing and
/// plain-text titles are exercised through identical construction.
fn sentence_apa_monograph_title_value(title_str: &str) -> String {
    let config = make_config_with_titles(citum_schema::options::TitlesConfig {
        monograph: Some(citum_schema::options::titles::TitleRendering {
            text_case: Some(citum_schema::options::titles::TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let reference = InputReference::Monograph(Box::new(Monograph {
        r#type: MonographType::Book,
        title: Some(Title::Single(title_str.to_string())),
        ..Default::default()
    }));
    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };
    component
        .values::<PlainText>(&reference, &hints, &options)
        .expect("title value should render")
        .value
}

#[rstest]
#[case::acronym_after_emphasis(
    "_The Effect_ of NIPS on Cognition",
    "_The effect_ of NIPS on cognition"
)]
#[case::acronym_inside_strong("Studies in *AI* and Law", "Studies in **AI** and law")]
fn given_markup_bearing_title_with_acronym_when_sentence_case_then_acronym_preserved(
    #[case] input: &str,
    #[case] expected: &str,
) {
    assert_eq!(sentence_apa_monograph_title_value(input), expected);
}

#[test]
fn test_text_case_punctuation_only_leaf_preserves_first_word_capital() {
    // A markup boundary immediately around a punctuation character (e.g. an
    // em dash inside its own span) splits it into its own Djot text leaf.
    // That leaf carries no letters, so it must not consume the pending
    // "capitalize the title's first word" request meant for the real first
    // word that follows.
    let result = sentence_apa_monograph_title_value("[—]{.emphasis-marker} Overview of Cognition");
    assert_eq!(result, "— Overview of cognition");
}

#[test]
fn test_text_case_plain_and_markup_paths_agree_on_acronym_preservation() {
    // Same words, same casing decisions, whether or not the title happens to
    // carry a Djot emphasis span (regression: markup-path sentence case used
    // to flat-lowercase every non-first leaf, destroying NIPS/AI regardless
    // of internal capitalization, while the plain-text path preserved them).
    let plain = sentence_apa_monograph_title_value("The Effect of NIPS on Cognition");
    let markup = sentence_apa_monograph_title_value("_The Effect_ of NIPS on Cognition");
    assert_eq!(plain, "The effect of NIPS on cognition");
    assert_eq!(markup, "_The effect_ of NIPS on cognition");
}

#[test]
fn test_text_case_structured_title_sentence_apa() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle, Title};

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    // Build a native Citum structured title reference
    let mut reference = Reference::from(LegacyReference {
        id: "structured".to_string(),
        ref_type: "book".to_string(),
        title: Some("placeholder".to_string()),
        ..Default::default()
    });
    // Replace with structured title
    if let ClassExtension::Monograph(m) = reference.extension_mut() {
        m.title = Some(Title::Structured(StructuredTitle {
            full: None,
            main: "Understanding Citation Systems".to_string(),
            sub: Subtitle::Vector(vec![
                "History and Practice".to_string(),
                "A Comparative View".to_string(),
            ]),
        }));
    }

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };
    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // APA: first word of main + each subtitle capitalized
    assert_eq!(
        values.value,
        "Understanding citation systems: History and practice; A comparative view"
    );
}

#[test]
fn test_structured_title_default_delimiters_use_locale_grammar_options() {
    use citum_schema::reference::types::{StructuredTitle, Subtitle};

    let result = structured_title_value_with_config(
        StructuredTitle {
            full: None,
            main: "Main title".to_string(),
            sub: Subtitle::Vector(vec!["Subtitle one".to_string(), "Subtitle two".to_string()]),
        },
        &Config::default(),
        None,
        None,
    );

    assert_eq!(result, "Main title: Subtitle one; Subtitle two");
}

#[test]
fn test_structured_title_locale_grammar_options_provide_fallback_delimiters() {
    use citum_schema::reference::types::{StructuredTitle, Subtitle};

    let mut locale = make_locale();
    locale.grammar_options.title_subtitle_delimiter = " — ".to_string();
    locale.grammar_options.subtitle_delimiter = " / ".to_string();

    let result = structured_title_value_with_config_and_locale(
        StructuredTitle {
            full: None,
            main: "Main title".to_string(),
            sub: Subtitle::Vector(vec!["Subtitle one".to_string(), "Subtitle two".to_string()]),
        },
        &Config::default(),
        &locale,
        None,
        None,
    );

    assert_eq!(result, "Main title — Subtitle one / Subtitle two");
}

#[test]
fn test_structured_title_primary_delimiter_separates_main_from_subtitle() {
    use citum_schema::options::titles::{TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle};

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            primary_delimiter: Some(". ".into()),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = structured_title_value_with_config(
        StructuredTitle {
            full: None,
            main: "Main title".to_string(),
            sub: Subtitle::String("Subtitle".to_string()),
        },
        &config,
        None,
        None,
    );

    assert_eq!(result, "Main title. Subtitle");
}

#[test]
fn test_structured_title_subtitle_delimiter_separates_subtitle_parts() {
    use citum_schema::options::titles::{TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle};

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            primary_delimiter: Some(": ".into()),
            subtitle_delimiter: Some(". ".into()),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = structured_title_value_with_config(
        StructuredTitle {
            full: None,
            main: "Main title".to_string(),
            sub: Subtitle::Vector(vec!["Subtitle one".to_string(), "Subtitle two".to_string()]),
        },
        &config,
        None,
        None,
    );

    assert_eq!(result, "Main title: Subtitle one. Subtitle two");
}

#[test]
fn test_structured_title_default_title_options_apply_delimiters_to_all_categories() {
    use citum_schema::options::titles::{TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle};

    let config = make_config_with_titles(TitlesConfig {
        default: Some(TitleRendering {
            primary_delimiter: Some(". ".into()),
            subtitle_delimiter: Some(" / ".into()),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = structured_title_value_with_config(
        StructuredTitle {
            full: None,
            main: "Main title".to_string(),
            sub: Subtitle::Vector(vec!["Subtitle one".to_string(), "Subtitle two".to_string()]),
        },
        &config,
        None,
        None,
    );

    assert_eq!(result, "Main title. Subtitle one / Subtitle two");
}

#[test]
fn test_structured_title_locale_override_can_change_delimiters() {
    use citum_schema::options::titles::{TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle};
    use std::collections::HashMap;

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            primary_delimiter: Some(": ".into()),
            subtitle_delimiter: Some(": ".into()),
            locale_overrides: Some(HashMap::from([(
                "de".to_string(),
                TitleRendering {
                    primary_delimiter: Some(". ".into()),
                    subtitle_delimiter: Some("; ".into()),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = structured_title_value_with_config(
        StructuredTitle {
            full: None,
            main: "Haupttitel".to_string(),
            sub: Subtitle::Vector(vec![
                "Untertitel eins".to_string(),
                "Untertitel zwei".to_string(),
            ]),
        },
        &config,
        Some("de-DE"),
        None,
    );

    assert_eq!(result, "Haupttitel. Untertitel eins; Untertitel zwei");
}

#[test]
fn test_structured_title_form_short_ignores_configured_delimiters() {
    use citum_schema::options::titles::{TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle};

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            primary_delimiter: Some(". ".into()),
            subtitle_delimiter: Some("; ".into()),
            ..Default::default()
        }),
        ..Default::default()
    });
    let result = structured_title_value_with_config(
        StructuredTitle {
            full: None,
            main: "Main title".to_string(),
            sub: Subtitle::Vector(vec!["Subtitle one".to_string(), "Subtitle two".to_string()]),
        },
        &config,
        None,
        Some(TitleForm::Short),
    );

    assert_eq!(result, "Main title");
}

#[test]
fn test_text_case_structured_title_sentence_nlm() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};
    use citum_schema::reference::types::{StructuredTitle, Subtitle, Title};

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceNlm),
            ..Default::default()
        }),
        ..Default::default()
    });
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let mut reference = Reference::from(LegacyReference {
        id: "structured".to_string(),
        ref_type: "book".to_string(),
        title: Some("placeholder".to_string()),
        ..Default::default()
    });
    if let ClassExtension::Monograph(m) = reference.extension_mut() {
        m.title = Some(Title::Structured(StructuredTitle {
            full: None,
            main: "Understanding Citation Systems".to_string(),
            sub: Subtitle::String("History and Practice".to_string()),
        }));
    }

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };
    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // NLM: main sentence-cased, subtitle lowercased only
    assert_eq!(
        values.value,
        "Understanding citation systems: history and practice"
    );
}

#[test]
fn test_text_case_non_english_falls_back_to_as_is() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};

    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::SentenceApa),
            ..Default::default()
        }),
        ..Default::default()
    });
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let reference = Reference::from(LegacyReference {
        id: "german".to_string(),
        ref_type: "book".to_string(),
        title: Some("Die Geschichte der Molekularbiologie".to_string()),
        language: Some("de".into()),
        ..Default::default()
    });
    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };
    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    // German: should be as-is (no English sentence case applied)
    assert_eq!(values.value, "Die Geschichte der Molekularbiologie");
}

#[test]
fn test_text_case_template_level_override() {
    use citum_schema::options::titles::{TextCase, TitleRendering, TitlesConfig};

    // Global config says as-is
    let config = make_config_with_titles(TitlesConfig {
        monograph: Some(TitleRendering {
            text_case: Some(TextCase::AsIs),
            ..Default::default()
        }),
        ..Default::default()
    });
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let reference = Reference::from(LegacyReference {
        id: "override".to_string(),
        ref_type: "book".to_string(),
        title: Some("The Quick Brown Fox".to_string()),
        ..Default::default()
    });
    // Template-level rendering overrides to lowercase
    let component = TemplateTitle {
        title: TitleType::Primary,
        rendering: Rendering {
            text_case: Some(TextCase::Lowercase),
            ..Default::default()
        },
        ..Default::default()
    };
    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(values.value, "the quick brown fox");
}

#[test]
fn test_text_case_no_config_means_no_transform() {
    let config = Config::default();
    let result = title_value_with_config("The Quick Brown Fox", "book", &config);
    // No text-case configured: title rendered as-is (just smart quotes)
    assert_eq!(result, "The Quick Brown Fox");
}

/// `form: short` on a structured title returns only the main part, not the
/// full compound string.
#[test]
fn test_structured_title_form_short_returns_main_only() {
    use citum_schema::reference::types::{StructuredTitle, Subtitle, Title};

    let config = Config::default();
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: true,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let mut reference = Reference::from(LegacyReference {
        id: "treaty".to_string(),
        ref_type: "treaty".to_string(),
        title: Some("placeholder".to_string()),
        ..Default::default()
    });
    if let ClassExtension::Treaty(m) = reference.extension_mut() {
        m.title = Some(Title::Structured(StructuredTitle {
            full: None,
            main: "Homeland Security Act of 2002".to_string(),
            sub: Subtitle::Vector(vec![
                "Hearings on H.R. 5005".to_string(),
                "Day 3".to_string(),
            ]),
        }));
    }

    // Without form: short — full compound title
    let full_component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };
    let full = full_component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(
        full.value,
        "Homeland Security Act of 2002: Hearings on H.R. 5005; Day 3"
    );

    // With form: short — main only, subtitles suppressed
    let short_component = TemplateTitle {
        title: TitleType::Primary,
        form: Some(TitleForm::Short),
        ..Default::default()
    };
    let short = short_component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();
    assert_eq!(short.value, "Homeland Security Act of 2002");
}

#[test]
fn given_abbreviation_map_when_title_matches_then_abbreviation_returned() {
    let reference = Reference::from(LegacyReference {
        id: "test-journal".to_string(),
        ref_type: "journal-article".to_string(),
        title: Some("Estates Gazette".to_string()),
        container_title: Some("Estates Gazette".to_string()),
        ..Default::default()
    });

    let mut abbrev_map = std::collections::HashMap::new();
    abbrev_map.insert("Estates Gazette".to_string(), "EG".to_string());

    let config = Config {
        ..Default::default()
    };

    let abbrev_map_obj = crate::api::AbbreviationMap(abbrev_map);
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &make_locale(),
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: Some(&abbrev_map_obj),
    };

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let hints = ProcHints {
        group_length: 1,
        ..Default::default()
    };

    let result = component.values::<PlainText>(&reference, &hints, &options);
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "EG");
}

#[test]
fn given_abbreviation_map_when_title_not_in_map_then_original_returned() {
    let reference = Reference::from(LegacyReference {
        id: "test-journal".to_string(),
        ref_type: "journal-article".to_string(),
        title: Some("Some Other Journal".to_string()),
        container_title: Some("Some Other Journal".to_string()),
        ..Default::default()
    });

    let mut abbrev_map = std::collections::HashMap::new();
    abbrev_map.insert("Estates Gazette".to_string(), "EG".to_string());

    let config = Config {
        ..Default::default()
    };

    let abbrev_map_obj = crate::api::AbbreviationMap(abbrev_map);
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &make_locale(),
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: Some(&abbrev_map_obj),
    };

    let component = TemplateTitle {
        title: TitleType::Primary,
        ..Default::default()
    };

    let hints = ProcHints {
        group_length: 1,
        ..Default::default()
    };

    let result = component.values::<PlainText>(&reference, &hints, &options);
    assert!(result.is_some());
    assert_eq!(result.unwrap().value, "Some Other Journal");
}

fn make_annotated_date_reference(note: Option<&str>, language: Option<&str>) -> Reference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some("gbt-regnal-1947".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Test Work".to_string())),
        issued: DateValue {
            value: "1947".to_string(),
            note: note.map(str::to_string),
        },
        language: language.map(Into::into),
        ..Default::default()
    }))
}

fn make_issued_year_component() -> TemplateDate {
    TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        rendering: Default::default(),
        links: None,
        custom: None,
    }
}

#[test]
fn test_date_note_appends_full_width_wrap_for_cjk_script_item() {
    // Full-width delimiters require the style to opt in via
    // `multilingual.realization-default: cjk` (PUNCTUATION_REALIZATION.md
    // §5) — matching how the embedded GB/T styles are configured. Without
    // that opt-in, per-item CJK language evidence alone does not force
    // full-width wrap punctuation (see `wrap_script_class`).
    let reference = make_annotated_date_reference(Some("民国三十六年"), Some("zh"));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        multilingual: Some(MultilingualConfig {
            realization_default: RealizationDefault::Cjk,
            ..Default::default()
        }),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let values = make_issued_year_component()
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947（民国三十六年）");
}

#[test]
fn test_date_note_appends_half_width_wrap_for_latin_script_item() {
    let reference = make_annotated_date_reference(Some("民国三十六年"), Some("en"));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let values = make_issued_year_component()
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947(民国三十六年)");
}

#[test]
fn test_date_note_hidden_when_style_has_no_note_wrap() {
    let reference = make_annotated_date_reference(Some("民国三十六年"), Some("zh"));
    let config = Config {
        dates: Some(DateConfig::default()),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let values = make_issued_year_component()
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947");
}

#[test]
fn test_date_note_hidden_when_component_suppresses_it() {
    // csl26-gl0n: a component with `suppress-note: true` never wraps its
    // note, even when the section's `note-wrap` is configured and the input
    // carries one — the escape hatch for a style that legitimately renders
    // the same date variable twice per item (e.g. GB/T author-date's short
    // front-matter year plus a full-precision date later in the body).
    let reference = make_annotated_date_reference(Some("民国三十六年"), Some("zh"));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let component = TemplateDate {
        suppress_note: Some(true),
        ..make_issued_year_component()
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947");
}

#[test]
fn test_date_note_hidden_when_input_has_no_note() {
    let reference = make_annotated_date_reference(None, Some("zh"));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let values = make_issued_year_component()
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947");
}

#[test]
fn test_date_note_follows_year_suffix_disambiguator() {
    let reference = make_annotated_date_reference(Some("民国三十六年"), Some("en"));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        ..make_config()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints {
        disamb_condition: true,
        group_index: 1,
        group_length: 2,
        ..Default::default()
    };

    let values = make_issued_year_component()
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947a(民国三十六年)");
}

#[test]
fn test_date_note_wraps_after_a_closed_interval() {
    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("kangxi-1705".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Test Work".to_string())),
        issued: DateValue {
            value: "1705/1706".to_string(),
            note: Some("康熙四十四年至四十五年".to_string()),
        },
        language: Some("zh".into()),
        ..Default::default()
    }));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        multilingual: Some(MultilingualConfig {
            realization_default: RealizationDefault::Cjk,
            ..Default::default()
        }),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();
    let component = TemplateDate {
        date: TemplateDateVar::Issued,
        form: DateForm::Year,
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        rendering: Default::default(),
        links: None,
        custom: None,
    };

    let values = component
        .values::<PlainText>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1705–1706（康熙四十四年至四十五年）");
}

#[test]
fn test_date_note_escapes_through_the_active_output_format() {
    use crate::render::html::Html;

    let reference = make_annotated_date_reference(Some("Minguo <36> & co"), Some("en"));
    let config = Config {
        dates: Some(DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let locale = make_locale();
    let options = RenderOptions {
        config: Arc::new(config),
        bibliography_config: None,
        locale: &locale,
        context: RenderContext::Bibliography,
        mode: citum_schema::citation::CitationMode::NonIntegral,
        suppress_author: false,
        locator_raw: None,
        ref_type: None,
        show_semantics: false,
        current_template_index: None,
        abbreviation_map: None,
    };
    let hints = ProcHints::default();

    let values = make_issued_year_component()
        .values::<Html>(&reference, &hints, &options)
        .unwrap();

    assert_eq!(values.value, "1947(Minguo &lt;36&gt; &amp; co)");
}
