/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Template presets, references, and locale-scoped template overrides.

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::embedded;
use crate::template::{LocalizedTemplateVariants, Template};

/// Available embedded template presets.
///
/// These reference battle-tested templates for common citation styles.
/// See `citum_schema::embedded` for the actual template implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TemplatePreset {
    /// APA 7th edition (author-date)
    Apa,
    /// Chicago Manual of Style (author-date)
    ChicagoAuthorDate,
    /// Vancouver (numeric)
    Vancouver,
    /// IEEE (numeric)
    Ieee,
    /// Harvard/Elsevier (author-date)
    Harvard,
    /// Numeric citation number only (citation-focused preset)
    NumericCitation,
}

/// A reference to a template, which can be either a named builtin preset
/// or a URI (e.g., `file://...`, `@hub/...`, `https://...`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum TemplateReference {
    /// A named builtin template preset.
    Preset(TemplatePreset),
    /// A URI reference to a remote or local template.
    Uri(String),
}

impl From<TemplatePreset> for TemplateReference {
    fn from(preset: TemplatePreset) -> Self {
        TemplateReference::Preset(preset)
    }
}

impl TemplateReference {
    /// Resolve this reference to a citation template when it names a built-in preset.
    #[must_use]
    pub fn citation_template(&self) -> Option<Template> {
        match self {
            Self::Preset(preset) => Some(preset.citation_template()),
            Self::Uri(_) => None,
        }
    }

    /// Resolve this reference to a bibliography template when it names a built-in preset.
    #[must_use]
    pub fn bibliography_template(&self) -> Option<Template> {
        match self {
            Self::Preset(preset) => Some(preset.bibliography_template()),
            Self::Uri(_) => None,
        }
    }
}

impl TemplatePreset {
    /// Resolve this preset to a citation template.
    #[must_use]
    pub fn citation_template(&self) -> Template {
        match self {
            TemplatePreset::Apa => embedded::apa_citation(),
            TemplatePreset::ChicagoAuthorDate => embedded::chicago_author_date_citation(),
            TemplatePreset::Vancouver => embedded::vancouver_citation(),
            TemplatePreset::Ieee => embedded::ieee_citation(),
            TemplatePreset::Harvard => embedded::harvard_citation(),
            TemplatePreset::NumericCitation => embedded::numeric_citation(),
        }
    }

    /// Resolve this preset to a bibliography template.
    #[must_use]
    pub fn bibliography_template(&self) -> Template {
        match self {
            TemplatePreset::Apa => embedded::apa_bibliography(),
            TemplatePreset::ChicagoAuthorDate => embedded::chicago_author_date_bibliography(),
            TemplatePreset::Vancouver => embedded::vancouver_bibliography(),
            TemplatePreset::Ieee => embedded::ieee_bibliography(),
            TemplatePreset::Harvard => embedded::harvard_bibliography(),
            // Citation-focused preset; Vancouver bibliography is the closest numeric fallback.
            TemplatePreset::NumericCitation => embedded::vancouver_bibliography(),
        }
    }
}

/// Locale-scoped template override with optional fallback behavior.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct LocalizedTemplateSpec {
    /// Language tags that should select this template override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<Vec<String>>,
    /// Whether this override is the fallback when no locale matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
    /// Template used when this localized override is selected.
    pub template: Template,
    /// Locale-owned type-specific template replacements.
    ///
    /// These take precedence over the section-level type variants when this locale matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schema",
        schemars(
            schema_with = "crate::template::type_keyed_map_schema::<crate::template::Template>"
        )
    )]
    pub type_variants: Option<LocalizedTemplateVariants>,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

/// A template resolved together with the locale selected by its localized branch.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedLocalizedTemplate {
    /// Concrete template selected for the reference.
    pub template: Template,
    /// Locale declared by the matching branch, or `None` for a default/base template.
    pub locale: Option<String>,
    /// Locale-owned type-specific replacements selected with this template.
    pub type_variants: Option<LocalizedTemplateVariants>,
}

pub(crate) fn matched_localized_template(
    locales: &[LocalizedTemplateSpec],
    language: &str,
) -> Option<ResolvedLocalizedTemplate> {
    let exact = locales.iter().find_map(|spec| {
        spec.locale.as_ref()?.iter().find_map(|candidate| {
            candidate
                .eq_ignore_ascii_case(language)
                .then(|| ResolvedLocalizedTemplate {
                    template: spec.template.clone(),
                    locale: Some(candidate.clone()),
                    type_variants: spec.type_variants.clone(),
                })
        })
    });
    if exact.is_some() {
        return exact;
    }

    let primary = language.split(['-', '_']).next().unwrap_or(language);
    locales.iter().find_map(|spec| {
        spec.locale.as_ref()?.iter().find_map(|candidate| {
            candidate
                .split(['-', '_'])
                .next()
                .is_some_and(|candidate_primary| candidate_primary.eq_ignore_ascii_case(primary))
                .then(|| ResolvedLocalizedTemplate {
                    template: spec.template.clone(),
                    locale: Some(candidate.clone()),
                    type_variants: spec.type_variants.clone(),
                })
        })
    })
}
