/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Final standalone style assembly for `citum-migrate`.

use crate::{
    bib_postprocess::{
        is_inferred_bib_source, merge_inferred_type_templates, postprocess_bibliography_templates,
    },
    citation_validate,
};
use citum_migrate::{
    analysis,
    compilation::XmlCompilationOutput as XmlFallback,
    error::MigrateError,
    fixups::{
        ensure_numeric_locator_citation_component, ensure_personal_communication_omitted,
        gate_web_only_url_accessed, move_group_wrap_to_citation_items,
        normalize_author_date_locator_citation_component,
        normalize_wrapped_numeric_locator_citation_component,
    },
    passes::suppression::strip_inert_suppressed_placeholders,
    template_resolver,
};
use citum_schema::{
    BibliographySpec, CitationCollapse, CitationSpec, Style, StyleInfo,
    template::{TemplateComponent, TemplateVariant, TypeSelector, WrapPunctuation},
};
use std::path::Path;

type TypeTemplateMap = indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>;
type TypeVariantMap = indexmap::IndexMap<TypeSelector, TemplateVariant>;

/// Borrowed pipeline state needed to assemble a standalone style variant.
pub(crate) struct StandaloneAssembly<'a> {
    /// Parsed legacy CSL style.
    pub(crate) legacy_style: &'a csl_legacy::model::Style,
    /// Resolved hand-authored, inferred, or XML-fallback template sources.
    pub(crate) resolved: &'a template_resolver::ResolvedTemplates,
    /// XML-compiled fallback candidate used when resolved templates are absent.
    pub(crate) xml_fallback: &'a Option<XmlFallback>,
    /// Extracted global migration options.
    pub(crate) options: &'a citum_schema::options::Config,
    /// Extracted bibliography-specific options.
    pub(crate) bibliography_options: &'a Option<citum_schema::BibliographyOptions>,
    /// Citation contributor overrides extracted from legacy CSL.
    pub(crate) citation_contributor_overrides: &'a Option<citum_schema::options::ContributorConfig>,
    /// Bibliography contributor overrides extracted from legacy CSL.
    pub(crate) bibliography_contributor_overrides:
        &'a Option<citum_schema::options::ContributorConfig>,
}

/// Template-source suppression switches used for measured candidate selection.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TemplateSourceSelection {
    /// Force bibliography assembly through XML fallback.
    pub(crate) suppress_inferred_bibliography: bool,
    /// Force citation assembly through XML fallback.
    pub(crate) suppress_inferred_citation: bool,
}

/// All compiled template and option data needed to build the final Style.
struct CompiledOutput {
    options: citum_schema::options::Config,
    citation_contributor_overrides: Option<citum_schema::options::ContributorConfig>,
    bibliography_options: Option<citum_schema::BibliographyOptions>,
    bibliography_contributor_overrides: Option<citum_schema::options::ContributorConfig>,
    new_cit: Vec<TemplateComponent>,
    citation_locales: Option<Vec<citum_schema::LocalizedTemplateSpec>>,
    new_bib: Vec<TemplateComponent>,
    bibliography_locales: Option<Vec<citum_schema::LocalizedTemplateSpec>>,
    type_templates: Option<TypeTemplateMap>,
    citation_wrap: Option<WrapPunctuation>,
    citation_item_wrap: Option<citum_schema::options::LabelWrap>,
    citation_prefix: Option<String>,
    citation_suffix: Option<String>,
    citation_delimiter: Option<String>,
    citation_subsequent_override: Option<Vec<TemplateComponent>>,
    citation_ibid_override: Option<Vec<TemplateComponent>>,
}

impl StandaloneAssembly<'_> {
    /// Assemble a standalone style using the requested source suppression.
    pub(crate) fn assemble_with_selection(
        &self,
        selection: TemplateSourceSelection,
    ) -> Result<Style, MigrateError> {
        let masked;
        let resolved =
            if selection.suppress_inferred_bibliography || selection.suppress_inferred_citation {
                masked = template_resolver::ResolvedTemplates {
                    bibliography: if selection.suppress_inferred_bibliography {
                        None
                    } else {
                        self.resolved.bibliography.clone()
                    },
                    citation: if selection.suppress_inferred_citation {
                        None
                    } else {
                        self.resolved.citation.clone()
                    },
                };
                &masked
            } else {
                self.resolved
            };

        let mut bibliography_options = self.bibliography_options.clone();
        let (new_bib, mut type_templates, inferred_bib_source) =
            select_and_process_bibliography_template(
                resolved,
                self.xml_fallback,
                self.legacy_style,
            )?;

        let (mut new_cit, citation_subsequent_override, citation_ibid_override) =
            select_citation_template(
                resolved,
                self.xml_fallback,
                inferred_bib_source,
                self.legacy_style,
                &mut type_templates,
            )?;

        override_bibliography_options_if_inferred(
            resolved,
            self.legacy_style,
            &mut bibliography_options,
        );

        let (
            citation_wrap,
            citation_item_wrap,
            citation_prefix,
            citation_suffix,
            citation_delimiter,
        ) = resolve_citation_metadata(resolved, self.legacy_style, self.options, &mut new_cit);

        Ok(build_final_style(
            self.legacy_style,
            CompiledOutput {
                options: self.options.clone(),
                citation_contributor_overrides: self.citation_contributor_overrides.clone(),
                bibliography_options,
                bibliography_contributor_overrides: self.bibliography_contributor_overrides.clone(),
                new_cit,
                citation_locales: self
                    .xml_fallback
                    .as_ref()
                    .and_then(|output| output.citation_locales.clone()),
                new_bib,
                bibliography_locales: self
                    .xml_fallback
                    .as_ref()
                    .and_then(|output| output.bibliography_locales.clone()),
                type_templates,
                citation_wrap,
                citation_item_wrap,
                citation_prefix,
                citation_suffix,
                citation_delimiter,
                citation_subsequent_override,
                citation_ibid_override,
            },
        ))
    }
}

/// Replace the current citation template when a measured candidate renders
/// closer to citeproc-js.
///
/// The inferrer's confidence score is self-referential and can rate templates
/// highly that render badly; this settles the choice empirically. Any scoring
/// failure keeps the inferred status quo.
pub(crate) fn apply_measured_citation_selection(
    current: Style,
    assembly: &StandaloneAssembly<'_>,
    source_selection: TemplateSourceSelection,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<
    (
        Style,
        bool,
        Option<citum_migrate::evidence::CitationSelectionEvidence>,
    ),
    MigrateError,
> {
    let Some(out) = assembly.xml_fallback.as_ref() else {
        return Ok((current, false, None));
    };
    if out.citation.is_empty() {
        return Ok((current, false, None));
    }

    let alternative = assembly.assemble_with_selection(TemplateSourceSelection {
        suppress_inferred_citation: true,
        ..source_selection
    })?;
    Ok(
        match citum_migrate::synthesis::synthesize_citation(
            &current,
            &alternative,
            style_name,
            style_xml,
            workspace_root,
        ) {
            Ok(selection) => apply_citation_selection_result(current, style_name, selection),
            Err(err) => measured_selection_unavailable(current, style_name, "citation", err),
        },
    )
}

/// Replace the current bibliography template when a measured candidate renders
/// closer to citeproc-js.
pub(crate) fn apply_measured_bibliography_selection(
    current: Style,
    assembly: &StandaloneAssembly<'_>,
    source_selection: TemplateSourceSelection,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<
    (
        Style,
        bool,
        Option<citum_migrate::evidence::BibliographySelectionEvidence>,
    ),
    MigrateError,
> {
    let Some(out) = assembly.xml_fallback.as_ref() else {
        return Ok((current, false, None));
    };
    if out.bibliography.is_empty() {
        return Ok((current, false, None));
    }

    let bibliography_source = assembly.assemble_with_selection(TemplateSourceSelection {
        suppress_inferred_bibliography: true,
        ..source_selection
    })?;
    let alternative = style_with_bibliography_from(current.clone(), bibliography_source);
    Ok(
        match citum_migrate::synthesis::synthesize_bibliography(
            &current,
            &alternative,
            style_name,
            style_xml,
            workspace_root,
        ) {
            Ok(selection) => apply_bibliography_selection_result(current, style_name, selection),
            Err(err) => measured_selection_unavailable(current, style_name, "bibliography", err),
        },
    )
}

/// Resolve migrated bibliography sort only when it differs from the processing default.
pub(crate) fn resolve_migrated_bibliography_sort(
    processing: Option<&citum_schema::options::Processing>,
    legacy_sort: Option<&csl_legacy::model::Sort>,
) -> Option<citum_schema::grouping::GroupSortEntry> {
    let extracted_entry = legacy_sort.and_then(
        citum_migrate::options_extractor::bibliography::extract_group_sort_from_bibliography,
    )?;
    let extracted = extracted_entry.resolve();

    if bibliography_sort_matches_processing_default(processing, &extracted) {
        None
    } else {
        Some(extracted_entry)
    }
}

/// Assembles the final Citum Style from compiled output and legacy metadata.
fn build_final_style(legacy_style: &csl_legacy::model::Style, mut c: CompiledOutput) -> Style {
    // The template compiler drops marker variables, so the regime decides which
    // marker the style declares. See docs/specs/REFERENCE_MARKERS.md.
    let declared_label_mode = match c.options.processing {
        Some(citum_schema::options::Processing::Numeric) => {
            Some(citum_schema::options::CitationLabelMode::Numeric)
        }
        Some(citum_schema::options::Processing::Label(_)) => {
            Some(citum_schema::options::CitationLabelMode::Alphabetic)
        }
        _ => None,
    };
    let declares_marker = declared_label_mode.is_some();

    let citation_scope_options = if declares_marker || c.citation_contributor_overrides.is_some() {
        Some(citum_schema::CitationOptions {
            label_mode: declared_label_mode,
            item_wrap: c.citation_item_wrap,
            contributors: c.citation_contributor_overrides,
            ..Default::default()
        })
    } else {
        None
    };
    let mut bibliography_scope_options = c.bibliography_options.take().unwrap_or_default();
    if let Some(contributors) = c.bibliography_contributor_overrides.take() {
        bibliography_scope_options.contributors = Some(contributors);
    }
    let bibliography_scope_options = (bibliography_scope_options
        != citum_schema::BibliographyOptions::default())
    .then_some(bibliography_scope_options);
    let bibliography_sort = resolve_migrated_bibliography_sort(
        c.options.processing.as_ref(),
        legacy_style
            .bibliography
            .as_ref()
            .and_then(|bib| bib.sort.as_ref()),
    );
    let (new_bib, type_variants) = finalize_bibliography_variants(c.new_bib, c.type_templates);

    // [PRUNING] Prune redundant citation modes (e.g. ibid/subsequent if they match base).
    let subsequent = c
        .citation_subsequent_override
        .filter(|t| t != &c.new_cit)
        .map(|t| {
            Box::new(CitationSpec {
                template: Some(t.into()),
                ..Default::default()
            })
        });

    let ibid = c
        .citation_ibid_override
        .filter(|t| t != &c.new_cit)
        .map(|t| {
            Box::new(CitationSpec {
                template: Some(t.into()),
                ..Default::default()
            })
        });

    Style {
        info: StyleInfo {
            title: Some(legacy_style.info.title.clone()),
            id: Some(legacy_style.info.id.clone()),
            default_locale: legacy_style.default_locale.clone(),
            ..Default::default()
        },
        templates: None,
        options: Some(c.options),
        citation: Some(CitationSpec {
            options: citation_scope_options,
            template_ref: None,
            template: Some(c.new_cit.into()),
            locales: c.citation_locales,
            collapse: extract_citation_collapse(&legacy_style.citation),
            wrap: c.citation_wrap.map(Into::into),
            prefix: c.citation_prefix.map(Into::into),
            suffix: c.citation_suffix.map(Into::into),
            delimiter: c.citation_delimiter.map(Into::into),
            multi_cite_delimiter: legacy_style
                .citation
                .layout
                .delimiter
                .clone()
                .map(Into::into),
            subsequent,
            ibid,
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: bibliography_scope_options,
            template_ref: None,
            template: Some(new_bib.into()),
            locales: c.bibliography_locales,
            type_variants,
            sort: bibliography_sort,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn select_and_process_bibliography_template(
    resolved: &template_resolver::ResolvedTemplates,
    xml_fallback: &Option<XmlFallback>,
    legacy_style: &csl_legacy::model::Style,
) -> Result<(Vec<TemplateComponent>, Option<TypeTemplateMap>, bool), MigrateError> {
    let (mut new_bib, mut type_templates, inferred_bib_source) =
        if let Some(ref resolved_bib) = resolved.bibliography {
            let inferred_bib = is_inferred_bib_source(&resolved_bib.source);
            let merged_type_templates = if inferred_bib {
                xml_fallback
                    .as_ref()
                    .and_then(|out| merge_inferred_type_templates(out, &resolved_bib.template))
            } else {
                None
            };
            (
                resolved_bib.template.clone(),
                merged_type_templates,
                inferred_bib,
            )
        } else {
            let Some(out) = xml_fallback.as_ref() else {
                return Err(MigrateError::Render(
                    "XML fallback must exist when bibliography is unresolved".to_string(),
                ));
            };
            (out.bibliography.clone(), out.type_templates.clone(), false)
        };

    // Phase 1: semantic fixups operate only on full concrete templates.
    postprocess_bibliography_templates(&mut new_bib, &mut type_templates, legacy_style);

    Ok((new_bib, type_templates, inferred_bib_source))
}

fn select_citation_template(
    resolved: &template_resolver::ResolvedTemplates,
    xml_fallback: &Option<XmlFallback>,
    inferred_bib_source: bool,
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<TypeTemplateMap>,
) -> Result<
    (
        Vec<TemplateComponent>,
        Option<Vec<TemplateComponent>>,
        Option<Vec<TemplateComponent>>,
    ),
    MigrateError,
> {
    let mut citation_subsequent_override: Option<Vec<TemplateComponent>> = None;
    let mut citation_ibid_override: Option<Vec<TemplateComponent>> = None;
    let new_cit = if let Some(ref resolved_cit) = resolved.citation {
        // Inferred templates only capture the first-position form. For note
        // styles, the subsequent/ibid short forms exist solely in the XML
        // pipeline's position extraction — attach them so the inferred first
        // form keeps its repeat behavior. Hand-authored templates are left
        // alone: their repeat shape is a deliberate authoring decision.
        if legacy_style.class == "note"
            && citation_validate::is_inferred_source(&resolved_cit.source)
            && let Some(out) = xml_fallback.as_ref()
        {
            citation_subsequent_override = out.citation_overrides.subsequent.clone();
            citation_ibid_override = out.citation_overrides.ibid.clone();
        }
        resolved_cit.template.clone()
    } else {
        let Some(out) = xml_fallback.as_ref() else {
            return Err(MigrateError::Render(
                "XML fallback must exist when citation is unresolved".to_string(),
            ));
        };
        citation_subsequent_override = out.citation_overrides.subsequent.clone();
        citation_ibid_override = out.citation_overrides.ibid.clone();
        out.citation.clone()
    };

    if inferred_bib_source {
        ensure_personal_communication_omitted(legacy_style, &new_cit, type_templates);
    }

    Ok((
        new_cit,
        citation_subsequent_override,
        citation_ibid_override,
    ))
}

#[allow(clippy::cognitive_complexity, reason = "macro-heavy output code")]
fn override_bibliography_options_if_inferred(
    resolved: &template_resolver::ResolvedTemplates,
    legacy_style: &csl_legacy::model::Style,
    options: &mut Option<citum_schema::BibliographyOptions>,
) {
    if let Some(ref resolved_bib) = resolved.bibliography {
        let allow_bib_punctuation_override =
            !(legacy_style.class == "note" && is_inferred_bib_source(&resolved_bib.source));

        if allow_bib_punctuation_override {
            if let Some(ref delim) = resolved_bib.delimiter {
                tracing::debug!("  Overriding bibliography separator: {delim:?}");
                let bib_cfg = options.get_or_insert_with(Default::default);
                bib_cfg.separator = Some(delim.clone().into());
            }

            if let Some(ref suffix) = resolved_bib.entry_suffix {
                tracing::debug!("  Overriding bibliography entry suffix: {suffix:?}");
                let bib_cfg = options.get_or_insert_with(Default::default);
                bib_cfg.entry_suffix = Some(suffix.clone().into());
            }
        } else {
            tracing::debug!(
                "  Skipping inferred bibliography separator/entry-suffix override for note style."
            );
        }
    }
}

fn resolve_citation_metadata(
    resolved: &template_resolver::ResolvedTemplates,
    legacy_style: &csl_legacy::model::Style,
    options: &citum_schema::options::Config,
    new_cit: &mut Vec<TemplateComponent>,
) -> (
    Option<citum_schema::template::WrapPunctuation>,
    Option<citum_schema::options::LabelWrap>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let mut citation_item_wrap = None;
    let (mut citation_wrap, mut citation_prefix, mut citation_suffix) =
        analysis::citation::infer_citation_wrapping(&legacy_style.citation.layout);
    let mut citation_delimiter = analysis::citation::extract_citation_delimiter(
        &legacy_style.citation.layout,
        &legacy_style.macros,
    );

    if let Some(ref resolved_cit) = resolved.citation {
        if let Some(ref wrap) = resolved_cit.wrap {
            citation_wrap = Some(wrap.clone());
            citation_prefix = None;
            citation_suffix = None;
        }
        if let Some(ref delim) = resolved_cit.delimiter {
            citation_delimiter = Some(delim.clone());
        }
    }

    if matches!(
        options.processing,
        Some(citum_schema::options::Processing::Numeric)
    ) {
        ensure_numeric_locator_citation_component(&legacy_style.citation.layout, new_cit);
        normalize_wrapped_numeric_locator_citation_component(
            &legacy_style.citation.layout,
            new_cit,
            &mut citation_delimiter,
        );
        citation_item_wrap =
            move_group_wrap_to_citation_items(&legacy_style.citation.layout, &mut citation_wrap);
    } else if legacy_style.class == "in-text" {
        normalize_author_date_locator_citation_component(
            &legacy_style.citation.layout,
            &legacy_style.macros,
            new_cit,
        );
    }

    (
        citation_wrap,
        citation_item_wrap,
        citation_prefix,
        citation_suffix,
        citation_delimiter,
    )
}

fn finalize_bibliography_variants(
    mut new_bib: Vec<TemplateComponent>,
    mut type_templates: Option<TypeTemplateMap>,
) -> (Vec<TemplateComponent>, Option<TypeVariantMap>) {
    // Phase 1: final semantic fixups still see full templates. This render-time
    // citeproc-js gate is applied here after measured selection and before any
    // diff encoding, so the cleaned base and type templates stay coherent.
    {
        let mut empty = TypeTemplateMap::new();
        let templates = type_templates.as_mut().unwrap_or(&mut empty);
        gate_web_only_url_accessed(&mut new_bib, templates);
    }
    strip_inert_suppressed_placeholders(&mut new_bib);
    if let Some(type_templates) = type_templates.as_mut() {
        for template in type_templates.values_mut() {
            strip_inert_suppressed_placeholders(template);
        }
    }

    let type_variants = type_templates.map(|type_templates| {
        type_templates
            .into_iter()
            .map(|(selector, template)| (selector, TemplateVariant::Full(template)))
            .collect()
    });

    (new_bib, type_variants)
}

fn apply_citation_selection_result(
    current: Style,
    style_name: &str,
    selection: citum_migrate::measured_citation::MeasuredCitationSelection,
) -> (
    Style,
    bool,
    Option<citum_migrate::evidence::CitationSelectionEvidence>,
) {
    let evidence = citum_migrate::evidence::CitationSelectionEvidence::from(&selection);
    if selection.use_xml {
        tracing::debug!(
            "Measured citation selection for {style_name}: XML candidate wins ({} vs {} passes over {} items); replacing inferred citation template.",
            selection.xml_passes,
            selection.inferred_passes,
            selection.items
        );
        return (selection.selected_style, true, Some(evidence));
    }
    if selection.selected_candidate != "inferred" {
        tracing::debug!(
            "Measured citation selection for {style_name}: {} candidate wins ({} vs {} current passes over {} items; {} synthesis rounds: {:?}).",
            selection.selected_candidate,
            selection.selected_passes,
            selection.inferred_passes,
            selection.items,
            selection.synthesis_rounds,
            selection.accepted_mutations
        );
        return (selection.selected_style, false, Some(evidence));
    }
    tracing::debug!(
        "Measured citation selection for {style_name}: keeping inferred citation template ({} vs {} passes over {} items).",
        selection.inferred_passes,
        selection.xml_passes,
        selection.items
    );
    (current, false, Some(evidence))
}

fn apply_bibliography_selection_result(
    current: Style,
    style_name: &str,
    selection: citum_migrate::measured_citation::MeasuredBibliographySelection,
) -> (
    Style,
    bool,
    Option<citum_migrate::evidence::BibliographySelectionEvidence>,
) {
    let evidence = citum_migrate::evidence::BibliographySelectionEvidence::from(&selection);
    if selection.use_xml {
        tracing::debug!(
            "Measured bibliography selection for {style_name}: XML candidate wins ({} vs {} current passes over {} items); replacing current bibliography template.",
            selection.xml_passes,
            selection.inferred_passes,
            selection.items
        );
        return (selection.selected_style, true, Some(evidence));
    }
    if selection.selected_candidate != "inferred" {
        tracing::debug!(
            "Measured bibliography selection for {style_name}: {} candidate wins (family={}, section={}, types={:?}; {} vs {} current passes over {} items; {} synthesis rounds: {:?}).",
            selection.selected_candidate,
            selection.selected_family.as_deref().unwrap_or("unknown"),
            selection.selected_section.as_deref().unwrap_or("unknown"),
            selection.selected_affected_types,
            selection.selected_passes,
            selection.inferred_passes,
            selection.items,
            selection.synthesis_rounds,
            selection.accepted_mutations
        );
        return (selection.selected_style, false, Some(evidence));
    }
    tracing::debug!(
        "Measured bibliography selection for {style_name}: keeping inferred bibliography template ({} vs {} passes over {} items).",
        selection.inferred_passes,
        selection.xml_passes,
        selection.items
    );
    (current, false, Some(evidence))
}

fn style_with_bibliography_from(mut current: Style, bibliography_source: Style) -> Style {
    current.bibliography = bibliography_source.bibliography;
    current
}

fn measured_selection_unavailable<T>(
    current: Style,
    style_name: &str,
    section: &str,
    err: MigrateError,
) -> (Style, bool, Option<T>) {
    tracing::debug!("Measured {section} selection unavailable for {style_name}: {err}");
    (current, false, None)
}

fn extract_citation_collapse(citation: &csl_legacy::model::Citation) -> Option<CitationCollapse> {
    match citation.collapse.as_deref() {
        Some("citation-number") => Some(CitationCollapse::CitationNumber),
        _ => None,
    }
}

fn bibliography_sort_matches_processing_default(
    processing: Option<&citum_schema::options::Processing>,
    sort: &citum_schema::grouping::GroupSort,
) -> bool {
    processing
        .and_then(citum_schema::options::Processing::default_bibliography_sort)
        .is_some_and(|preset| preset.group_sort() == *sort)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use citum_schema::{
        BibliographySpec, CitationSpec,
        options::{
            AndOptions, ContributorConfig, DisplayAsSort, NameForm, TextCase, TitleRendering,
            TitlesConfig,
        },
        template::{
            ContributorForm, ContributorRole, NameOrder, Rendering, SimpleVariable,
            TemplateContributor, TemplateTitle, TemplateVariable, TitleType,
        },
    };
    use csl_legacy::model::{
        Citation, Info, Layout, Sort as LegacySort, SortKey as LegacySortKey, Style as LegacyStyle,
    };

    #[test]
    fn bibliography_candidate_preserves_current_citation_section() {
        let current = Style {
            citation: Some(CitationSpec {
                delimiter: Some(String::new().into()),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(
                    vec![TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Url,
                        ..TemplateVariable::default()
                    })]
                    .into(),
                ),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };
        let bibliography_source = Style {
            citation: Some(CitationSpec {
                delimiter: Some(", ".into()),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(
                    vec![TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Doi,
                        ..TemplateVariable::default()
                    })]
                    .into(),
                ),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let alternative =
            style_with_bibliography_from(current.clone(), bibliography_source.clone());

        assert_eq!(
            serde_json::to_value(&alternative.citation).expect("citation should serialize"),
            serde_json::to_value(&current.citation).expect("citation should serialize")
        );
        assert_eq!(
            serde_json::to_value(&alternative.bibliography).expect("bibliography should serialize"),
            serde_json::to_value(&bibliography_source.bibliography)
                .expect("bibliography should serialize")
        );
    }

    #[test]
    fn suppresses_author_date_default_bibliography_sort() {
        let sort = resolve_migrated_bibliography_sort(
            Some(&citum_schema::options::Processing::AuthorDate),
            Some(&legacy_sort(&["author", "issued", "title"])),
        );

        assert_eq!(sort, None);
    }

    #[test]
    fn suppresses_note_default_bibliography_sort() {
        let sort = resolve_migrated_bibliography_sort(
            Some(&citum_schema::options::Processing::Note),
            Some(&legacy_sort(&["author", "title", "issued"])),
        );

        assert_eq!(sort, None);
    }

    #[test]
    fn preserves_numeric_bibliography_sort() {
        let sort = resolve_migrated_bibliography_sort(
            Some(&citum_schema::options::Processing::Numeric),
            Some(&legacy_sort(&["author", "issued", "title"])),
        );

        assert_eq!(
            sort,
            Some(citum_schema::grouping::GroupSortEntry::Preset(
                citum_schema::presets::SortPreset::AuthorDateTitle
            ))
        );
    }

    #[test]
    fn preserves_note_family_exceptions() {
        let sort = resolve_migrated_bibliography_sort(
            Some(&citum_schema::options::Processing::Note),
            Some(&legacy_sort(&["author", "issued", "title"])),
        );

        assert!(sort.is_some());
    }

    #[test]
    fn maps_legacy_citation_number_collapse() {
        let mut style = minimal_legacy_style();
        style.citation.collapse = Some("citation-number".to_string());

        let migrated = build_final_style(
            &style,
            CompiledOutput {
                options: citum_schema::options::Config::default(),
                bibliography_options: None,
                citation_contributor_overrides: None,
                bibliography_contributor_overrides: None,
                new_cit: vec![],
                citation_locales: None,
                new_bib: vec![],
                bibliography_locales: None,
                type_templates: None,
                citation_wrap: None,
                citation_item_wrap: None,
                citation_prefix: None,
                citation_suffix: None,
                citation_delimiter: None,
                citation_subsequent_override: None,
                citation_ibid_override: None,
            },
        );

        assert_eq!(
            migrated
                .citation
                .as_ref()
                .and_then(|citation| citation.collapse.clone()),
            Some(CitationCollapse::CitationNumber)
        );
    }

    #[test]
    fn assembly_preserves_localized_template_branches() {
        let localized = citum_schema::LocalizedTemplateSpec {
            locale: Some(vec!["zh-CN".to_string()]),
            default: None,
            template: vec![TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Publisher,
                ..TemplateVariable::default()
            })],
            type_variants: None,
            unknown_fields: Default::default(),
        };
        let migrated = build_final_style(
            &minimal_legacy_style(),
            CompiledOutput {
                options: citum_schema::options::Config::default(),
                citation_contributor_overrides: None,
                bibliography_options: None,
                bibliography_contributor_overrides: None,
                new_cit: Vec::new(),
                citation_locales: Some(vec![localized.clone()]),
                new_bib: Vec::new(),
                bibliography_locales: Some(vec![localized]),
                type_templates: None,
                citation_wrap: None,
                citation_item_wrap: None,
                citation_prefix: None,
                citation_suffix: None,
                citation_delimiter: None,
                citation_subsequent_override: None,
                citation_ibid_override: None,
            },
        );

        assert_eq!(
            migrated
                .citation
                .as_ref()
                .and_then(|citation| citation.locales.as_ref())
                .and_then(|locales| locales.first())
                .and_then(|localized| localized.locale.as_ref()),
            Some(&vec!["zh-CN".to_string()])
        );
        assert_eq!(
            migrated
                .bibliography
                .as_ref()
                .and_then(|bibliography| bibliography.locales.as_ref())
                .and_then(|locales| locales.first())
                .and_then(|localized| localized.locale.as_ref()),
            Some(&vec!["zh-CN".to_string()])
        );
    }

    #[test]
    fn assembly_preserves_explicit_template_fields_for_sqi_refinement() {
        let migrated = build_final_style(&minimal_legacy_style(), explicit_sqi_boundary_output());

        let citation_template = migrated
            .citation
            .as_ref()
            .and_then(|citation| citation.template.as_ref())
            .and_then(TemplateVariant::as_template)
            .expect("citation template should be present");
        let TemplateComponent::Contributor(author) = citation_template
            .first()
            .expect("first citation component should be present")
        else {
            panic!("first citation component should remain a contributor");
        };
        assert_eq!(author.and, Some(AndOptions::Text));
        assert_eq!(author.name_form, Some(NameForm::Initials));
        assert_eq!(author.name_order, Some(NameOrder::FamilyFirstOnly));
        let TemplateComponent::Title(title) = citation_template
            .get(1)
            .expect("second citation component should be present")
        else {
            panic!("second citation component should remain a title");
        };
        assert_eq!(title.rendering.text_case, Some(TextCase::SentenceApa));

        let book_variant = migrated
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.type_variants.as_ref())
            .and_then(|variants| variants.get(&TypeSelector::Single("book".to_string())))
            .expect("book variant should be present");
        let TemplateVariant::Full(book_template) = book_variant else {
            panic!("assembly should emit full type variants before SQI refinement");
        };
        let TemplateComponent::Contributor(book_author) = book_template
            .first()
            .expect("book variant component should be present")
        else {
            panic!("book variant should remain a contributor template");
        };
        assert_eq!(book_author.and, Some(AndOptions::Text));
        assert_eq!(book_author.name_form, Some(NameForm::Initials));
        assert_eq!(book_author.name_order, Some(NameOrder::FamilyFirstOnly));
    }

    fn explicit_sqi_boundary_output() -> CompiledOutput {
        let mut type_templates = TypeTemplateMap::new();
        type_templates.insert(
            TypeSelector::Single("book".to_string()),
            vec![explicit_author_component()],
        );

        CompiledOutput {
            options: citum_schema::options::Config {
                contributors: Some(ContributorConfig {
                    display_as_sort: Some(DisplayAsSort::First),
                    and: Some(AndOptions::Text),
                    name_form: Some(NameForm::Initials),
                    ..ContributorConfig::default()
                }),
                titles: Some(TitlesConfig {
                    default: Some(TitleRendering {
                        text_case: Some(TextCase::SentenceApa),
                        ..TitleRendering::default()
                    }),
                    ..TitlesConfig::default()
                }),
                ..citum_schema::options::Config::default()
            },
            bibliography_options: None,
            citation_contributor_overrides: None,
            bibliography_contributor_overrides: None,
            new_cit: vec![
                explicit_author_component(),
                TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    rendering: Rendering {
                        text_case: Some(TextCase::SentenceApa),
                        ..Rendering::default()
                    },
                    ..TemplateTitle::default()
                }),
            ],
            citation_locales: None,
            new_bib: vec![],
            bibliography_locales: None,
            type_templates: Some(type_templates),
            citation_wrap: None,
            citation_item_wrap: None,
            citation_prefix: None,
            citation_suffix: None,
            citation_delimiter: None,
            citation_subsequent_override: None,
            citation_ibid_override: None,
        }
    }

    fn explicit_author_component() -> TemplateComponent {
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author.into(),
            form: ContributorForm::Long,
            name_order: Some(NameOrder::FamilyFirstOnly),
            name_form: Some(NameForm::Initials),
            and: Some(AndOptions::Text),
            ..TemplateContributor::default()
        })
    }

    fn legacy_sort(keys: &[&str]) -> LegacySort {
        LegacySort {
            keys: keys
                .iter()
                .map(|key| LegacySortKey {
                    variable: Some((*key).to_string()),
                    macro_name: None,
                    sort: None,
                })
                .collect(),
        }
    }

    fn minimal_legacy_style() -> LegacyStyle {
        LegacyStyle {
            version: "1.0".to_string(),
            xmlns: "http://purl.org/net/xbiblio/csl".to_string(),
            class: "in-text".to_string(),
            default_locale: None,
            initialize_with: None,
            initialize_with_hyphen: None,
            names_delimiter: None,
            name_as_sort_order: None,
            sort_separator: None,
            delimiter_precedes_last: None,
            delimiter_precedes_et_al: None,
            demote_non_dropping_particle: None,
            and: None,
            page_range_format: None,
            info: Info::default(),
            locale: vec![],
            macros: vec![],
            citation: Citation {
                layout: Layout {
                    prefix: None,
                    suffix: None,
                    delimiter: None,
                    children: vec![],
                },
                localized_layouts: Vec::new(),
                sort: None,
                collapse: None,
                et_al_min: None,
                et_al_use_first: None,
                disambiguate_add_year_suffix: None,
                disambiguate_add_names: None,
                disambiguate_add_givenname: None,
                disambiguate_givenname_rule: None,
            },
            bibliography: None,
        }
    }
}
