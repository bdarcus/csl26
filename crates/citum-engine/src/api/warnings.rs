/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Input-compatibility warning scanners.
//!
//! Each function inspects already-loaded inputs (style, bibliography) for
//! constructs the engine tolerated but cannot act on — unknown reference
//! classes, fields captured by the forward-compat `unknown_fields`
//! catch-all, and unknown enum variants — and reports them as structured
//! [`Warning`]s. Adapters (CLI, WASM, FFI) present these; they must never
//! re-derive their own checks.

use crate::processor::Processor;
use crate::reference::Bibliography;
use citum_schema::locale::{GeneralTerm, TermForm};
use citum_schema::options::TermLocale;
use citum_schema::reference::{
    ClassExtension, CollectionType, ContributorRole as ReferenceRole, MonographComponentType,
    MonographType, ReferenceClass, SerialComponentType,
};
use citum_schema::template::ContributorRole as TemplateRole;

use super::{Warning, WarningLevel};

/// Scan the bibliography for tagged items whose effective language has no
/// loaded locale under `options.multilingual.term-locale: item`.
///
/// A no-op unless `term-locale: item` is active in the citation or
/// bibliography scope. Untagged items resolve to the style locale by design
/// (positive-evidence rule, `PER_ITEM_TERM_LOCALE.md` §3) and never warn;
/// only items that *do* carry a language with no matching loaded locale are
/// reported, so the silent style-locale fallback stays discoverable. Reuses
/// [`crate::processor::rendering::lookup_embedded_locale`] — the same
/// exact-tag → primary-language lookup the renderer itself uses — so this
/// scanner and the render-time fallback never diverge.
pub fn term_locale_fallback_warnings(processor: &Processor) -> Vec<Warning> {
    let term_locale_is_item = |config: &citum_schema::options::Config| {
        config
            .multilingual
            .as_ref()
            .is_some_and(|ml| ml.term_locale == TermLocale::Item)
    };
    let active = term_locale_is_item(&processor.get_citation_config())
        || term_locale_is_item(&processor.get_bibliography_config());
    if !active {
        return Vec::new();
    }

    processor
        .bibliography
        .iter()
        .filter_map(|(ref_id, reference)| {
            let language = crate::values::effective_item_language(reference)?;
            if crate::processor::rendering::lookup_embedded_locale(&language).is_some() {
                return None;
            }
            Some(Warning {
                level: WarningLevel::Warning,
                code: "term_locale_unavailable".to_string(),
                citation_id: None,
                ref_id: Some(ref_id.clone()),
                message: format!(
                    "Reference '{ref_id}' has language '{language}' but no loaded locale matches it; \
                     term-locale: item falls back to the style locale for this reference's terms."
                ),
            })
        })
        .collect()
}

/// Warn when the bibliography config declares a numeric or alphabetic
/// reference-marker mode with no `label-wrap` and no `label-separator`,
/// which renders the marker flush against the entry body (`1Smith`, not
/// `1. Smith` or `[1] Smith`).
///
/// `label-separator` defaults to empty by design, and several shipped
/// styles depend on a genuinely flush marker. A declared
/// `second-field-align` (see `docs/specs/SECOND_FIELD_ALIGN.md`) is the
/// structural, affirmative signal that flush is intentional, and suppresses
/// this warning outright — `royal-society-of-chemistry` is the case this
/// replaces: its in-file comment documented `second-field-align="flush"`
/// from its CSL source as the reason a bare flush marker was fine, and now
/// that is declarable instead of asserted in prose. For everything else this
/// stays advisory, not an error: it surfaces a configuration shape that is
/// easy to author unintentionally without asserting it is wrong (e.g.
/// `gb-t-7714-2025-base`'s bracketed-but-flush marker, which declares no
/// `second-field-align`). `author-date` mode is exempt: it never produces a
/// bibliography marker (see `marker.rs`), so wrap and separator are inert for
/// it. See `docs/specs/REFERENCE_MARKERS.md`.
pub fn bibliography_label_missing_separator_warnings(processor: &Processor) -> Vec<Warning> {
    let config = processor.get_bibliography_options();
    let produces_marker = matches!(
        config.label_mode,
        Some(
            citum_schema::options::BibliographyLabelMode::Numeric
                | citum_schema::options::BibliographyLabelMode::Alphabetic
        )
    );
    if !produces_marker
        || config.label_wrap.is_some()
        || config.label_separator.is_some()
        || config.second_field_align.is_some()
    {
        return Vec::new();
    }

    vec![Warning {
        level: WarningLevel::Warning,
        code: "bibliography_label_no_separator".to_string(),
        citation_id: None,
        ref_id: None,
        message: "Bibliography configuration declares a numeric or alphabetic label-mode with \
                  no label-wrap and no label-separator; the reference marker will render flush \
                  against the entry body (e.g. '1Smith' rather than '1. Smith'). If this is \
                  intentional (flush second-field-align), no action is needed."
            .to_string(),
    }]
}

/// Scan the bibliography for unknown reference classes and return compatibility warnings.
pub fn unknown_reference_class_warnings(bibliography: &Bibliography) -> Vec<Warning> {
    bibliography
        .iter()
        .filter_map(|(ref_id, reference)| {
            let ReferenceClass::Unknown(class) = reference.class() else {
                return None;
            };
            Some(Warning {
                level: WarningLevel::Warning,
                code: "unknown_reference_class".to_string(),
                citation_id: None,
                ref_id: Some(ref_id.clone()),
                message: format!(
                    "Reference '{ref_id}' uses unknown class '{class}'; rendering will use only fields this engine understands."
                ),
            })
        })
        .collect()
}

/// Scan the bibliography for fields captured by the forward-compat
/// `unknown_fields` catch-all and return per-reference warnings.
///
/// Unknown-class references are skipped here; they are already reported by
/// [`unknown_reference_class_warnings`].
pub fn unknown_reference_field_warnings(bibliography: &Bibliography) -> Vec<Warning> {
    bibliography
        .iter()
        .filter_map(|(ref_id, reference)| {
            let unknown = reference.unknown_fields()?;
            if unknown.is_empty() {
                return None;
            }
            let keys: Vec<&str> = unknown.keys().map(String::as_str).collect();
            Some(Warning {
                level: WarningLevel::Warning,
                code: "unknown_reference_field".to_string(),
                citation_id: None,
                ref_id: Some(ref_id.clone()),
                message: format!(
                    "Reference '{ref_id}' has unknown field(s): {}; these fields are ignored during rendering.",
                    keys.join(", ")
                ),
            })
        })
        .collect()
}

/// Scan the style and bibliography for unknown enum variants and term keys.
///
/// Returns a list of structured compatibility warnings for encounter of
/// unknown variants that were captured via the tolerant-enum mechanism.
pub fn unknown_enum_warnings(processor: &Processor) -> Vec<Warning> {
    let mut warnings = Vec::new();

    // 1. Scan bibliography
    for (ref_id, reference) in &processor.bibliography {
        match reference.extension() {
            ClassExtension::Monograph(r) => {
                if let MonographType::Unknown(s) = &r.r#type {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: Some(ref_id.clone()),
                        message: format!("Reference '{ref_id}' uses unknown monograph type '{s}'; rendering will use default monograph formatting."),
                    });
                }
            }
            ClassExtension::Collection(r) => {
                if let CollectionType::Unknown(s) = &r.r#type {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: Some(ref_id.clone()),
                        message: format!("Reference '{ref_id}' uses unknown collection type '{s}'; rendering will use default collection formatting."),
                    });
                }
            }
            ClassExtension::CollectionComponent(r) => {
                if let MonographComponentType::Unknown(s) = &r.r#type {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: Some(ref_id.clone()),
                        message: format!("Reference '{ref_id}' uses unknown monograph component type '{s}'; rendering will use default chapter formatting."),
                    });
                }
            }
            ClassExtension::SerialComponent(r) => {
                if let SerialComponentType::Unknown(s) = &r.r#type {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: Some(ref_id.clone()),
                        message: format!("Reference '{ref_id}' uses unknown serial component type '{s}'; rendering will use default article formatting."),
                    });
                }
            }
            _ => {}
        }

        for contributor in reference.all_contributor_entries() {
            for role in contributor.roles.as_slice() {
                if let ReferenceRole::Unknown(s) = role {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: Some(ref_id.clone()),
                        message: format!("Reference '{ref_id}' uses unknown contributor role '{s}'; this role may be ignored during rendering."),
                    });
                }
            }
        }
    }

    // 2. Scan Style
    if let Some(templates) = &processor.style.templates {
        for (name, template) in templates {
            scan_template_for_unknowns(template, &format!("template '{name}'"), &mut warnings);
        }
    }
    if let Some(citation) = &processor.style.citation {
        scan_citation_spec_for_unknowns(citation, "citation layout", &mut warnings);
    }
    if let Some(bib) = &processor.style.bibliography {
        if let Some(template) = &bib.template {
            scan_template_variant_for_unknowns(template, "bibliography layout", &mut warnings);
        }
        if let Some(type_variants) = &bib.type_variants {
            for variant in type_variants.values() {
                if let Some(template) = variant.as_template() {
                    scan_template_for_unknowns(template, "bibliography layout", &mut warnings);
                }
            }
        }
        if let Some(locales) = &bib.locales {
            for locale_spec in locales {
                scan_template_for_unknowns(
                    &locale_spec.template,
                    "bibliography layout",
                    &mut warnings,
                );
            }
        }
    }
    scan_bibliography_sort_for_citation_number(processor, &mut warnings);

    warnings
}

/// Warn about bibliography sort configurations that resolve to nothing.
///
/// Two distinct shapes, both surfaced under the same `code`:
///
/// 1. An *explicit* config-level (custom-processing) sort lists
///    `citation-number` as one of several keys. `Sort::group_sort` drops the
///    key rather than mapping it, so that key contributes nothing to
///    bibliography ordering while any other listed keys still apply — a
///    silent partial no-op the style author almost certainly did not intend.
/// 2. The *effective* bibliography sort — [`Processor::resolved_bibliography_sort`],
///    which accounts for a style-level `bibliography.sort` override and
///    processing-family preset defaults, not just the config-level step —
///    resolves to an empty group-sort template because it is (or inherits)
///    the `citation-number` preset. That preset is the documented way to say
///    "no bibliography sort" for numeric styles, but a non-numeric family
///    (author-date/note/label) that resolves to it — often by silently
///    inheriting `bibliography.sort: citation-number` from a numeric base,
///    rather than declaring its own sort — has no group-sort equivalent and
///    renders in registry order with no warning otherwise. Guarded to run
///    only when case 1 did not already fire, since both read the same
///    config-level step.
fn scan_bibliography_sort_for_citation_number(processor: &Processor, warnings: &mut Vec<Warning>) {
    let mut config_level_explicit_warned = false;

    if let Some(citum_schema::options::SortEntry::Explicit(sort)) = processor
        .get_bibliography_config()
        .processing
        .as_ref()
        .map(citum_schema::options::Processing::config)
        .and_then(|config| config.sort)
    {
        let uses_citation_number = sort
            .template
            .iter()
            .any(|spec| matches!(spec.key, citum_schema::options::SortKey::CitationNumber));

        if uses_citation_number {
            config_level_explicit_warned = true;
            warnings.push(Warning {
                level: WarningLevel::Warning,
                code: "citation_number_sort_not_supported".to_string(),
                citation_id: None,
                ref_id: None,
                message: "Style bibliography configuration lists 'citation-number' as an explicit \
                          sort key; it is not supported and is ignored for bibliography ordering."
                    .to_string(),
            });
        }
    }

    if config_level_explicit_warned {
        return;
    }

    let is_numeric_family = matches!(
        processor.get_bibliography_config().processing,
        Some(citum_schema::options::Processing::Numeric)
    );
    if is_numeric_family {
        return;
    }

    if let Some((group_sort, is_config_level)) = processor.resolved_bibliography_sort()
        && !is_config_level
        && group_sort.template.is_empty()
    {
        warnings.push(Warning {
            level: WarningLevel::Warning,
            code: "citation_number_sort_not_supported".to_string(),
            citation_id: None,
            ref_id: None,
            message: "Style bibliography sort resolves to the 'citation-number' preset, which has \
                      no group-sort equivalent outside numeric processing; the bibliography will \
                      render in registry order instead. This is frequently inherited unintentionally \
                      from a numeric base style rather than declared explicitly."
                .to_string(),
        });
    }
}

/// Recursively scan a [`citum_schema::CitationSpec`] and its mode/position
/// sub-specs (`integral`, `non-integral`, `subsequent`, `ibid`), plus its
/// `type-variants` and per-locale templates, for unknown enum variants.
///
/// The top-level `unknown_enum_warnings` scan previously inspected only the
/// main citation template, missing unknown terms/roles/date-forms nested in
/// sub-specs, type-variants, and localized templates.
fn scan_citation_spec_for_unknowns(
    spec: &citum_schema::CitationSpec,
    location: &str,
    warnings: &mut Vec<Warning>,
) {
    if let Some(template) = &spec.template {
        scan_template_variant_for_unknowns(template, location, warnings);
    }
    if let Some(type_variants) = &spec.type_variants {
        for variant in type_variants.values() {
            if let Some(template) = variant.as_template() {
                scan_template_for_unknowns(template, location, warnings);
            }
        }
    }
    if let Some(locales) = &spec.locales {
        for locale_spec in locales {
            scan_template_for_unknowns(&locale_spec.template, location, warnings);
        }
    }

    if let Some(child) = &spec.integral {
        scan_citation_spec_for_unknowns(child, &format!("{location} (integral)"), warnings);
    }
    if let Some(child) = &spec.non_integral {
        scan_citation_spec_for_unknowns(child, &format!("{location} (non-integral)"), warnings);
    }
    if let Some(child) = &spec.subsequent {
        scan_citation_spec_for_unknowns(child, &format!("{location} (subsequent)"), warnings);
    }
    if let Some(child) = &spec.ibid {
        scan_citation_spec_for_unknowns(child, &format!("{location} (ibid)"), warnings);
    }
}

fn scan_template_variant_for_unknowns(
    variant: &citum_schema::template::TemplateVariant,
    location: &str,
    warnings: &mut Vec<Warning>,
) {
    match variant {
        citum_schema::template::TemplateVariant::Full(template) => {
            scan_template_for_unknowns(template, location, warnings);
        }
        citum_schema::template::TemplateVariant::Diff(diff) => {
            for add in &diff.add {
                scan_template_for_unknowns(
                    std::slice::from_ref(&add.component),
                    location,
                    warnings,
                );
            }
        }
    }
}

fn scan_template_for_unknowns(
    components: &[citum_schema::template::TemplateComponent],
    location: &str,
    warnings: &mut Vec<Warning>,
) {
    use citum_schema::template::TemplateComponent;
    for component in components {
        match component {
            TemplateComponent::Term(t) => {
                if let GeneralTerm::Unknown(s) = &t.term {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: None,
                        message: format!("Style {location} uses unknown locale term key '{s}'; this term may render as empty."),
                    });
                }
                if let Some(TermForm::Unknown(s)) = &t.form {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: None,
                        message: format!("Style {location} uses unknown term form '{s}'; falling back to long form."),
                    });
                }
            }
            TemplateComponent::Contributor(c) => {
                for role in c.contributor.as_slice() {
                    if let TemplateRole::Unknown(s) = role {
                        warnings.push(Warning {
                            level: WarningLevel::Warning,
                            code: "unknown_enum_variant".to_string(),
                            citation_id: None,
                            ref_id: None,
                            message: format!("Style {location} uses unknown contributor role '{s}'; this role may be ignored."),
                        });
                    }
                }
                if let Some(label) = &c.label {
                    let term = label.term.as_str();
                    if !crate::values::contributor::labels::RECOGNIZED_LABEL_TERMS.contains(&term) {
                        warnings.push(Warning {
                            level: WarningLevel::Warning,
                            code: "unknown_role_label_term".to_string(),
                            citation_id: None,
                            ref_id: None,
                            message: format!("Style {location} uses unrecognized role-label term '{term}'; falling back to the contributor's own role term instead of the requested one."),
                        });
                    }
                }
            }
            TemplateComponent::Date(d) => {
                if let citum_schema::template::DateForm::Unknown(s) = &d.form {
                    warnings.push(Warning {
                        level: WarningLevel::Warning,
                        code: "unknown_enum_variant".to_string(),
                        citation_id: None,
                        ref_id: None,
                        message: format!("Style {location} uses unknown date form '{s}'; falling back to year only."),
                    });
                }
            }
            TemplateComponent::Group(g) => {
                scan_template_for_unknowns(&g.group, location, warnings);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests")]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn bibliography_label_missing_separator_warnings_reports_bare_numeric_label_mode() {
        let yaml = "info:\n  title: Test\nbibliography:\n  options:\n    label-mode: numeric\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = bibliography_label_missing_separator_warnings(&processor);
        assert!(
            warnings
                .iter()
                .any(|w| w.code == "bibliography_label_no_separator"),
            "expected a warning for label-mode with no wrap and no separator, got: {warnings:?}"
        );
    }

    #[test]
    fn bibliography_label_missing_separator_warnings_silent_with_label_wrap() {
        let yaml = "info:\n  title: Test\nbibliography:\n  options:\n    label-mode: numeric\n    label-wrap: period\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = bibliography_label_missing_separator_warnings(&processor);
        assert!(
            !warnings
                .iter()
                .any(|w| w.code == "bibliography_label_no_separator"),
            "did not expect a warning once label-wrap is declared, got: {warnings:?}"
        );
    }

    #[test]
    fn bibliography_label_missing_separator_warnings_silent_with_label_separator() {
        let yaml = "info:\n  title: Test\nbibliography:\n  options:\n    label-mode: numeric\n    label-separator: ' '\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = bibliography_label_missing_separator_warnings(&processor);
        assert!(
            !warnings
                .iter()
                .any(|w| w.code == "bibliography_label_no_separator"),
            "did not expect a warning once label-separator is declared, got: {warnings:?}"
        );
    }

    #[test]
    fn bibliography_label_missing_separator_warnings_silent_for_author_date_mode() {
        let yaml = "info:\n  title: Test\nbibliography:\n  options:\n    label-mode: author-date\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = bibliography_label_missing_separator_warnings(&processor);
        assert!(
            !warnings
                .iter()
                .any(|w| w.code == "bibliography_label_no_separator"),
            "author-date mode never produces a bibliography marker, so wrap/separator are \
             inert; did not expect a warning, got: {warnings:?}"
        );
    }

    #[test]
    fn bibliography_label_missing_separator_warnings_silent_with_second_field_align() {
        let yaml = "info:\n  title: Test\nbibliography:\n  options:\n    label-mode: numeric\n    second-field-align: flush\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = bibliography_label_missing_separator_warnings(&processor);
        assert!(
            !warnings
                .iter()
                .any(|w| w.code == "bibliography_label_no_separator"),
            "a declared second-field-align is the affirmative signal that flush is \
             intentional; did not expect a warning, got: {warnings:?}"
        );
    }

    #[test]
    fn bibliography_label_missing_separator_warnings_silent_with_no_label_mode() {
        let yaml = "info:\n  title: Test\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = bibliography_label_missing_separator_warnings(&processor);
        assert!(
            warnings.is_empty(),
            "did not expect a warning with no bibliography options at all, got: {warnings:?}"
        );
    }

    #[test]
    fn unknown_enum_warnings_reports_unknown_term_in_integral_sub_spec() {
        let yaml = "info:\n  title: Test\ncitation:\n  integral:\n    template:\n      - term: not-a-real-term\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = unknown_enum_warnings(&processor);
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("not-a-real-term") && w.message.contains("(integral)")),
            "expected a warning for the unknown term in citation.integral.template, got: {warnings:?}"
        );
    }

    #[test]
    fn unknown_enum_warnings_reports_unknown_role_label_term() {
        let yaml = "info:\n  title: Test\nbibliography:\n  template:\n    - contributor: editor\n      form: long\n      label: {term: not-a-real-role}\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = unknown_enum_warnings(&processor);
        assert!(
            warnings
                .iter()
                .any(|w| w.code == "unknown_role_label_term"
                    && w.message.contains("not-a-real-role")),
            "expected a warning for the unrecognized role-label term, got: {warnings:?}"
        );
    }

    #[test]
    fn unknown_enum_warnings_does_not_flag_recognized_role_label_terms() {
        let yaml = "info:\n  title: Test\nbibliography:\n  template:\n    - contributor: editor\n      form: long\n      label: {term: editor}\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = unknown_enum_warnings(&processor);
        assert!(
            !warnings.iter().any(|w| w.code == "unknown_role_label_term"),
            "did not expect a warning for a recognized role-label term, got: {warnings:?}"
        );
    }

    #[test]
    fn unknown_enum_warnings_does_not_flag_director_role_label_term() {
        let yaml = "info:\n  title: Test\nbibliography:\n  template:\n    - contributor: director\n      form: long\n      label: {term: director, form: short}\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = unknown_enum_warnings(&processor);
        assert!(
            !warnings.iter().any(|w| w.code == "unknown_role_label_term"),
            "did not expect a warning for the director role-label term \
             (chicago-author-date-18th's song type-variant relies on this), \
             got: {warnings:?}"
        );
    }

    /// given a style whose bibliography sort resolves to the `citation-number`
    /// preset (directly, or by inheriting a numeric base's default), when
    /// scanning for compat warnings, then a non-numeric processing family
    /// warns while a numeric family stays silent — `citation-number` has no
    /// group-sort equivalent outside numeric processing (registry order by
    /// definition), so it is only a legitimate "no bibliography sort" idiom
    /// for numeric styles.
    #[rstest]
    #[case::author_date_with_own_citation_number_sort(
        "options:\n  processing: author-date-full\nbibliography:\n  sort: citation-number\n",
        true
    )]
    #[case::note_family_with_citation_number_sort(
        "options:\n  processing: note\nbibliography:\n  sort: citation-number\n",
        true
    )]
    #[case::numeric_family_with_citation_number_sort(
        "options:\n  processing: numeric\nbibliography:\n  sort: citation-number\n",
        false
    )]
    #[case::author_date_family_with_no_bibliography_sort_declared(
        "options:\n  processing: author-date-full\n",
        false
    )]
    #[case::author_date_family_with_own_non_citation_number_sort(
        "options:\n  processing: author-date-full\nbibliography:\n  sort: author-date-title\n",
        false
    )]
    fn scan_bibliography_sort_for_citation_number_is_family_aware(
        #[case] yaml_body: &str,
        #[case] expect_warning: bool,
    ) {
        let yaml = format!("info:\n  title: Test\n{yaml_body}");
        let style = citum_schema::Style::from_yaml_str(&yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = unknown_enum_warnings(&processor);
        let fired = warnings
            .iter()
            .any(|w| w.code == "citation_number_sort_not_supported");

        assert_eq!(
            fired, expect_warning,
            "case yaml:\n{yaml}\nwarnings: {warnings:?}"
        );
    }

    #[test]
    fn unknown_enum_warnings_reports_unknown_term_in_type_variants() {
        let yaml = "info:\n  title: Test\ncitation:\n  type-variants:\n    book:\n      - term: not-a-real-term-2\n";
        let style = citum_schema::Style::from_yaml_str(yaml).unwrap();
        let processor = Processor::new(style, Bibliography::new());

        let warnings = unknown_enum_warnings(&processor);
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("not-a-real-term-2")),
            "expected a warning for the unknown term in citation.type-variants, got: {warnings:?}"
        );
    }
}
