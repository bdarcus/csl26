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

/// Resolve a multilingual string based on style configuration.
///
/// Applies BCP 47 fallback logic:
/// 1. Exact tag match (e.g., "ja-Latn-hepburn")
/// 2. Script prefix match (e.g., "ja-Latn")
/// 3. Fallback to original field
///
/// # Arguments
/// * `string` - The multilingual string to resolve
/// * `mode` - The rendering mode from style config
/// * `preferred_script` - Optional preferred script (e.g., "Latn")
/// * `style_locale` - The style's locale for translation matching
pub fn resolve_multilingual_string(
    string: &csln_core::reference::types::MultilingualString,
    mode: Option<&csln_core::options::MultilingualMode>,
    preferred_script: Option<&String>,
    style_locale: &str,
) -> String {
    use csln_core::options::MultilingualMode;
    use csln_core::reference::types::MultilingualString;

    match string {
        MultilingualString::Simple(s) => s.clone(),
        MultilingualString::Complex(complex) => {
            let mode = mode.unwrap_or(&MultilingualMode::Primary);

            match mode {
                MultilingualMode::Primary => complex.original.clone(),

                MultilingualMode::Transliterated => {
                    // Try exact match first, then prefix match, then fallback
                    if let Some(script) = preferred_script {
                        // Try exact script match
                        if let Some(trans) = complex.transliterations.get(script) {
                            return trans.clone();
                        }

                        // Try substring match (e.g., "Latn" matches "ja-Latn-hepburn")
                        for (tag, trans) in &complex.transliterations {
                            if tag.contains(script) {
                                return trans.clone();
                            }
                        }
                    }

                    // Fallback: use any available transliteration, or original
                    complex
                        .transliterations
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| complex.original.clone())
                }

                MultilingualMode::Translated => {
                    // Try to match style locale
                    complex
                        .translations
                        .get(style_locale)
                        .cloned()
                        .unwrap_or_else(|| complex.original.clone())
                }

                MultilingualMode::Combined => {
                    // Format: "transliterated [translated]" or fallback variants
                    let trans = if let Some(script) = preferred_script {
                        complex.transliterations.get(script).or_else(|| {
                            complex
                                .transliterations
                                .iter()
                                .find(|(tag, _)| tag.contains(script))
                                .map(|(_, v)| v)
                        })
                    } else {
                        complex.transliterations.values().next()
                    };

                    let translation = complex.translations.get(style_locale);

                    match (trans, translation) {
                        (Some(t), Some(tr)) => format!("{} [{}]", t, tr),
                        (Some(t), None) => t.clone(),
                        (None, Some(tr)) => format!("{} [{}]", complex.original, tr),
                        (None, None) => complex.original.clone(),
                    }
                }
            }
        }
    }
}

/// Resolve a multilingual contributor name based on style configuration.
///
/// Uses holistic name matching - selects the entire name variant (original/transliterated/translated)
/// as a unit rather than mixing fields from different variants.
///
/// # Arguments
/// * `contributor` - The contributor to resolve
/// * `mode` - The rendering mode from style config
/// * `preferred_script` - Optional preferred script (e.g., "Latn")
/// * `style_locale` - The style's locale for translation matching
pub fn resolve_multilingual_name(
    contributor: &csln_core::reference::contributor::Contributor,
    mode: Option<&csln_core::options::MultilingualMode>,
    preferred_script: Option<&String>,
    style_locale: &str,
) -> Vec<crate::reference::FlatName> {
    use csln_core::options::MultilingualMode;
    use csln_core::reference::contributor::Contributor;

    match contributor {
        // Simple and structured names have no multilingual data
        Contributor::SimpleName(_) | Contributor::StructuredName(_) => contributor.to_names_vec(),

        // Multilingual names: select variant holistically
        Contributor::Multilingual(m) => {
            let mode = mode.unwrap_or(&MultilingualMode::Primary);

            let selected_name = match mode {
                MultilingualMode::Primary => &m.original,

                MultilingualMode::Transliterated => {
                    if let Some(script) = preferred_script {
                        // Try exact script match
                        if let Some(name) = m.transliterations.get(script) {
                            name
                        } else {
                            // Try substring match (e.g., "Latn" matches "ru-Latn-alalc97")
                            m.transliterations
                                .iter()
                                .find(|(tag, _)| tag.contains(script))
                                .map(|(_, n)| n)
                                .unwrap_or(&m.original)
                        }
                    } else {
                        // Use any available transliteration
                        m.transliterations.values().next().unwrap_or(&m.original)
                    }
                }

                MultilingualMode::Translated => {
                    m.translations.get(style_locale).unwrap_or(&m.original)
                }

                // Combined mode for names defaults to transliterated (parenthetical combo not common for names)
                MultilingualMode::Combined => {
                    if let Some(script) = preferred_script {
                        m.transliterations
                            .get(script)
                            .or_else(|| {
                                m.transliterations
                                    .iter()
                                    .find(|(tag, _)| tag.contains(script))
                                    .map(|(_, n)| n)
                            })
                            .unwrap_or(&m.original)
                    } else {
                        m.transliterations.values().next().unwrap_or(&m.original)
                    }
                }
            };

            // Convert selected name to FlatName
            vec![crate::reference::FlatName {
                given: Some(selected_name.given.to_string()),
                family: Some(selected_name.family.to_string()),
                suffix: selected_name.suffix.clone(),
                dropping_particle: selected_name.dropping_particle.clone(),
                non_dropping_particle: selected_name.non_dropping_particle.clone(),
                literal: None,
            }]
        }

        Contributor::ContributorList(l) => {
            l.0.iter()
                .flat_map(|c| resolve_multilingual_name(c, mode, preferred_script, style_locale))
                .collect()
        }
    }
}

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
    if let Some(links) = global_links
        && let Some(anchor) = &links.anchor
        && *anchor == component_anchor
    {
        return resolve_url(links, reference);
    }

    None
}

/// Processed values ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct ProcValues<T = String> {
    /// The primary formatted value.
    pub value: T,
    /// Optional prefix to prepend.
    pub prefix: Option<String>,
    /// Optional suffix to append.
    pub suffix: Option<String>,
    /// Optional URL for hyperlinking.
    pub url: Option<String>,
    /// Variable key that was substituted (e.g., "title:Primary" when title replaces author).
    /// Used to prevent duplicate rendering per CSL variable-once rule.
    pub substituted_key: Option<String>,
    /// Whether the value is already pre-formatted.
    pub pre_formatted: bool,
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
    /// Visibility modifier for this item.
    pub visibility: csln_core::citation::ItemVisibility,
    /// Optional locator value (e.g. "42")
    pub locator: Option<&'a str>,
    /// Optional locator label (e.g. page, section)
    pub locator_label: Option<csln_core::citation::LocatorType>,
}

/// Trait for extracting values from template components.
pub trait ComponentValues {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>>;
}

impl ComponentValues for TemplateComponent {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        match self {
            TemplateComponent::Contributor(c) => c.values::<F>(reference, hints, options),
            TemplateComponent::Date(d) => d.values::<F>(reference, hints, options),
            TemplateComponent::Title(t) => t.values::<F>(reference, hints, options),
            TemplateComponent::Number(n) => n.values::<F>(reference, hints, options),
            TemplateComponent::Variable(v) => v.values::<F>(reference, hints, options),
            TemplateComponent::List(l) => l.values::<F>(reference, hints, options),
            TemplateComponent::Term(t) => t.values::<F>(reference, hints, options),
            _ => None,
        }
    }
}

/// Check if periods should be stripped based on three-tier precedence.
///
/// Resolution order:
/// 1. Component-level `strip_periods`
/// 2. Global config `strip_periods`
/// 3. Defaults to false
pub fn should_strip_periods(
    rendering: &csln_core::template::Rendering,
    options: &RenderOptions<'_>,
) -> bool {
    rendering
        .strip_periods
        .or(options.config.strip_periods)
        .unwrap_or(false)
}

/// Strip trailing periods from a string.
///
/// Only removes periods at the end of the string, preserves internal periods
/// (e.g., "Ph.D." remains unchanged if there's no trailing period).
pub fn strip_trailing_periods(s: &str) -> String {
    s.trim_end_matches('.').to_string()
}
