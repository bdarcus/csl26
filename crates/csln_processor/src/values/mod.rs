/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Value extraction for template components.
//!
//! This module provides the logic to extract formatted values from references
//! based on template component specifications.

pub mod contributor;
pub mod date;
pub mod list;
pub mod number;
pub mod term;
pub mod title;
pub mod variable;

#[cfg(test)]
mod tests;

use crate::reference::Reference;
use csln_core::locale::Locale;
use csln_core::options::Config;
use csln_core::template::TemplateComponent;

pub use contributor::format_contributors_short;
pub use date::int_to_letter;

/// Resolve the URL for a component based on its links configuration and the reference data.
pub fn resolve_url(
    links: &csln_core::options::LinksConfig,
    reference: &Reference,
) -> Option<String> {
    use csln_core::options::LinkTarget;

    let target = links.target.as_ref().unwrap_or(&LinkTarget::UrlOrDoi);

    match target {
        LinkTarget::Url => reference.url().map(|u| u.to_string()),
        LinkTarget::Doi => reference.doi().map(|d| format!("https://doi.org/{}", d)),
        LinkTarget::UrlOrDoi => reference
            .url()
            .map(|u| u.to_string())
            .or_else(|| reference.doi().map(|d| format!("https://doi.org/{}", d))),
        LinkTarget::Pubmed => reference
            .id()
            .filter(|id| id.starts_with("pmid:"))
            .map(|id| format!("https://pubmed.ncbi.nlm.nih.gov/{}/", &id[5..])),
        LinkTarget::Pmcid => reference
            .id()
            .filter(|id| id.starts_with("pmc:"))
            .map(|id| format!("https://www.ncbi.nlm.nih.gov/pmc/articles/{}/", &id[4..])),
    }
}

/// Resolve the effective URL for a component, checking local links then falling back to global config.
pub fn resolve_effective_url(
    local_links: Option<&csln_core::options::LinksConfig>,
    global_links: Option<&csln_core::options::LinksConfig>,
    reference: &Reference,
    component_anchor: csln_core::options::LinkAnchor,
) -> Option<String> {
    use csln_core::options::LinkAnchor;

    // 1. Check local links first
    if let Some(links) = local_links {
        let anchor = links.anchor.as_ref().unwrap_or(&LinkAnchor::Component);
        if matches!(anchor, LinkAnchor::Component) || *anchor == component_anchor {
            return resolve_url(links, reference);
        }
    }

    // 2. Fall back to global links if anchor matches this component type
    if let Some(links) = global_links {
        if let Some(anchor) = &links.anchor {
            if *anchor == component_anchor {
                return resolve_url(links, reference);
            }
        }
    }

    None
}

/// Processed values ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct ProcValues {
    /// The primary formatted value.
    pub value: String,
    /// Optional prefix to prepend.
    pub prefix: Option<String>,
    /// Optional suffix to append.
    pub suffix: Option<String>,
    /// Optional URL for hyperlinking.
    pub url: Option<String>,
    /// Variable key that was substituted (e.g., "title:Primary" when title replaces author).
    /// Used to prevent duplicate rendering per CSL variable-once rule.
    pub substituted_key: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcHints {
    /// Whether disambiguation is active (triggers year-suffix).
    pub disamb_condition: bool,
    /// Index in the disambiguation group (1-based).
    pub group_index: usize,
    /// Total size of the disambiguation group.
    pub group_length: usize,
    /// The grouping key used.
    pub group_key: String,
    /// Whether to expand given names for disambiguation.
    pub expand_given_names: bool,
    /// Minimum number of names to show to resolve ambiguity (overrides et-al-use-first).
    pub min_names_to_show: Option<usize>,
    /// Citation number for numeric citation styles (1-based).
    pub citation_number: Option<usize>,
}

/// Context for rendering (citation vs bibliography).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderContext {
    #[default]
    Citation,
    Bibliography,
}

/// Options for rendering.
pub struct RenderOptions<'a> {
    pub config: &'a Config,
    pub locale: &'a Locale,
    pub context: RenderContext,
    pub mode: csln_core::citation::CitationMode,
}

/// Trait for extracting values from template components.
pub trait ComponentValues {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues>;
}

impl ComponentValues for TemplateComponent {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        match self {
            TemplateComponent::Contributor(c) => c.values(reference, hints, options),
            TemplateComponent::Date(d) => d.values(reference, hints, options),
            TemplateComponent::Title(t) => t.values(reference, hints, options),
            TemplateComponent::Number(n) => n.values(reference, hints, options),
            TemplateComponent::Variable(v) => v.values(reference, hints, options),
            TemplateComponent::List(l) => l.values(reference, hints, options),
            TemplateComponent::Term(t) => t.values(reference, hints, options),
            _ => None,
        }
    }
}
