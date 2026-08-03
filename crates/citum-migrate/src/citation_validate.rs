/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Inferred citation template validation and contributor normalization.

use citum_migrate::{
    fixups::{
        citation_template_has_citation_number, citation_template_is_author_year_only,
        normalize_author_date_inferred_contributors, normalize_contributor_form_to_short,
        note_citation_template_is_underfit,
    },
    template_resolver,
};
use citum_schema::template::TemplateComponent;

pub(crate) fn is_inferred_source(source: &template_resolver::TemplateSource) -> bool {
    matches!(
        source,
        template_resolver::TemplateSource::InferredCached(_)
            | template_resolver::TemplateSource::InferredLive
    )
}

/// Validates the inferred citation template and normalizes contributor forms.
///
/// Rejects invalid inferred templates (falling back to XML), then applies
/// contributor form normalization for note, author-date, and in-text styles.
pub(crate) fn validate_and_normalize_inferred_citations(
    resolved: &mut template_resolver::ResolvedTemplates,
    options: &citum_schema::options::Config,
    legacy_style: &csl_legacy::model::Style,
    style_name: &str,
    citation_has_scope_shorten: bool,
) {
    reject_invalid_inferred_citation(resolved, options, legacy_style, style_name);
    normalize_inferred_citation_contributors(
        resolved,
        options,
        legacy_style,
        style_name,
        citation_has_scope_shorten,
    );
}

fn reject_invalid_inferred_citation(
    resolved: &mut template_resolver::ResolvedTemplates,
    options: &citum_schema::options::Config,
    legacy_style: &csl_legacy::model::Style,
    style_name: &str,
) {
    let Some(resolved_cit) = resolved.citation.as_ref() else {
        return;
    };
    if !is_inferred_source(&resolved_cit.source) {
        return;
    }
    if let Some(reason) =
        inferred_citation_reject_reason(&resolved_cit.template, options, legacy_style)
    {
        tracing::debug!(
            "Rejecting inferred citation template for {style_name}: {reason}. Falling back to XML citation template."
        );
        resolved.citation = None;
    }
}

fn inferred_citation_reject_reason(
    template: &[TemplateComponent],
    _options: &citum_schema::options::Config,
    legacy_style: &csl_legacy::model::Style,
) -> Option<&'static str> {
    if template.is_empty() {
        Some("empty citation template")
    } else if legacy_style.class == "note" && note_citation_template_is_underfit(template) {
        Some("note style citation template is contributor-only underfit")
    } else {
        None
    }
}

fn normalize_inferred_citation_contributors(
    resolved: &mut template_resolver::ResolvedTemplates,
    options: &citum_schema::options::Config,
    legacy_style: &csl_legacy::model::Style,
    style_name: &str,
    citation_has_scope_shorten: bool,
) {
    normalize_note_or_author_date_citation(resolved, options, legacy_style, style_name);
    normalize_in_text_citation(
        resolved,
        legacy_style,
        style_name,
        citation_has_scope_shorten,
    );
}

fn normalize_note_or_author_date_citation(
    resolved: &mut template_resolver::ResolvedTemplates,
    options: &citum_schema::options::Config,
    legacy_style: &csl_legacy::model::Style,
    style_name: &str,
) {
    let should_normalize = legacy_style.class == "note"
        || options
            .processing
            .as_ref()
            .is_some_and(citum_schema::options::Processing::is_author_date_family);
    if !should_normalize {
        return;
    }
    let Some(resolved_cit) = resolved.citation.as_mut() else {
        return;
    };
    if !is_inferred_source(&resolved_cit.source) {
        return;
    }
    if citation_template_is_author_year_only(&resolved_cit.template)
        && normalize_contributor_form_to_short(&mut resolved_cit.template)
    {
        tracing::debug!(
            "Normalized citation contributor form to short for {style_name} (author-year inferred citation template)."
        );
    }
}

fn normalize_in_text_citation(
    resolved: &mut template_resolver::ResolvedTemplates,
    legacy_style: &csl_legacy::model::Style,
    style_name: &str,
    citation_has_scope_shorten: bool,
) {
    if legacy_style.class != "in-text" {
        return;
    }
    let Some(resolved_cit) = resolved.citation.as_mut() else {
        return;
    };
    if !is_inferred_source(&resolved_cit.source) {
        return;
    }
    let is_author_year_shape = citation_template_is_author_year_only(&resolved_cit.template)
        && !citation_template_has_citation_number(&resolved_cit.template);
    if is_author_year_shape
        && normalize_author_date_inferred_contributors(
            &mut resolved_cit.template,
            citation_has_scope_shorten,
        )
    {
        tracing::debug!(
            "Normalized inferred author-date citation contributors for {style_name} (family-short + scoped shorten)."
        );
    }
}
