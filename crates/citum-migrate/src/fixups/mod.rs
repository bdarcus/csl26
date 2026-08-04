/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "lib/crate")]

use citum_schema::template::{TemplateComponent, TypeSelector, WrapPunctuation};
use csl_legacy::model::Layout;

mod gating;
mod locator;
mod media;
mod template;

/// Re-groups or drops a leaked root-level `in` term left bare by template
/// specialization, restoring the engine's term-only group suppression.
pub fn gate_leaked_in_term(template: &mut Vec<TemplateComponent>) {
    gating::gate_leaked_in_term(template);
}

/// Gates url and accessed components to web-only type templates.
///
/// Must be called before `build_type_variants` so that the cleaned base and
/// type-template components are reflected consistently in derived diffs.
pub fn gate_web_only_url_accessed(
    base_template: &mut Vec<TemplateComponent>,
    type_templates: &mut indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>,
) {
    gating::gate_web_only_url_accessed(base_template, type_templates);
}

/// Returns whether a type selector matches any candidate type name.
#[must_use]
pub fn selector_matches_any(selector: &TypeSelector, candidates: &[&str]) -> bool {
    media::selector_matches_any(selector, candidates)
}

/// Normalizes legal-case type templates after migration from legacy CSL.
pub fn normalize_legal_case_type_template(
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
) {
    media::normalize_legal_case_type_template(legacy_style, type_templates);
}

/// Ensures inferred media-oriented type templates exist when legacy signals require them.
pub fn ensure_inferred_media_type_templates(
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
    bibliography_template: &[TemplateComponent],
) {
    media::ensure_inferred_media_type_templates(
        legacy_style,
        type_templates,
        bibliography_template,
    );
}

/// Ensures personal communication types are omitted when legacy behavior suppresses them.
pub fn ensure_personal_communication_omitted(
    legacy_style: &csl_legacy::model::Style,
    citation_template: &[TemplateComponent],
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
) {
    media::ensure_personal_communication_omitted(legacy_style, citation_template, type_templates);
}

/// Ensures inferred patent templates exist when legacy output expects them.
pub fn ensure_inferred_patent_type_template(
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
    bibliography_template: &[TemplateComponent],
) {
    media::ensure_inferred_patent_type_template(
        legacy_style,
        type_templates,
        bibliography_template,
    );
}

/// Adds a numeric locator component when the legacy citation layout uses one.
pub fn ensure_numeric_locator_citation_component(
    layout: &Layout,
    template: &mut Vec<TemplateComponent>,
) {
    locator::ensure_numeric_locator_citation_component(layout, template);
}

/// Normalizes wrapped numeric locator formatting extracted from legacy layout groups.
pub fn normalize_wrapped_numeric_locator_citation_component(
    layout: &Layout,
    template: &mut [TemplateComponent],
    citation_delimiter: &mut Option<String>,
) {
    locator::normalize_wrapped_numeric_locator_citation_component(
        layout,
        template,
        citation_delimiter,
    );
}

/// Normalizes author-date locator formatting based on legacy citation layout hints.
pub fn normalize_author_date_locator_citation_component(
    layout: &Layout,
    macros: &[csl_legacy::model::Macro],
    template: &mut Vec<TemplateComponent>,
) {
    locator::normalize_author_date_locator_citation_component(layout, macros, template);
}

/// Converts a legacy layout group wrapping the citation number into the
/// declarative `item-wrap` that encloses marker plus item body, clearing the
/// cluster wrap. See `docs/specs/REFERENCE_MARKERS.md`.
#[must_use]
pub fn move_group_wrap_to_citation_items(
    layout: &Layout,
    citation_wrap: &mut Option<WrapPunctuation>,
) -> Option<citum_schema::options::LabelWrap> {
    locator::move_group_wrap_to_citation_items(layout, citation_wrap)
}

/// Returns whether a note citation template is too small to preserve note semantics.
#[must_use]
pub fn note_citation_template_is_underfit(template: &[TemplateComponent]) -> bool {
    template::note_citation_template_is_underfit(template)
}

/// Returns whether a citation template contains only author and year structure.
#[must_use]
pub fn citation_template_is_author_year_only(template: &[TemplateComponent]) -> bool {
    template::citation_template_is_author_year_only(template)
}

/// Converts long contributor forms to short forms throughout a migrated template.
pub fn normalize_contributor_form_to_short(template: &mut [TemplateComponent]) -> bool {
    template::normalize_contributor_form_to_short(template)
}

/// Normalizes inferred contributor rendering for author-date citation templates.
pub fn normalize_author_date_inferred_contributors(
    template: &mut [TemplateComponent],
    drop_component_shorten: bool,
) -> bool {
    template::normalize_author_date_inferred_contributors(template, drop_component_shorten)
}

/// Returns whether a migrated type template should be merged into inferred output.
#[must_use]
pub fn should_merge_inferred_type_template(
    type_name: &str,
    inferred_template: &[TemplateComponent],
    candidate_template: &[TemplateComponent],
) -> bool {
    template::should_merge_inferred_type_template(type_name, inferred_template, candidate_template)
}

/// Scrubs literal artifacts introduced by inferred template fragments.
pub fn scrub_inferred_literal_artifacts(component: &mut TemplateComponent) {
    template::scrub_inferred_literal_artifacts(component);
}
