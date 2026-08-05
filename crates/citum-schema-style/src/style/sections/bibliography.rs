/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Bibliography section specification.

use std::collections::HashMap;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grouping;
use crate::options::BibliographyOptions;
use crate::template::{
    LocalizedTemplateSpec, ResolvedLocalizedTemplate, Template, TemplateReference,
    TemplateVariants, matched_localized_template,
};

fn default_true() -> bool {
    true
}

/// Bibliography specification.
#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct BibliographySpec {
    /// Bibliography-specific option overrides merged over the style config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<BibliographyOptions>,
    /// Reference to an embedded template preset or external template.
    ///
    /// If both `template-ref` and `template` are present, `template` takes precedence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_ref: Option<TemplateReference>,
    /// Default template for entries when no localized override is selected.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub template: Option<Template>,
    /// Locale-specific template overrides checked before the default template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locales: Option<Vec<LocalizedTemplateSpec>>,
    /// Type-specific template overrides. When present, replaces the default
    /// template for entries of the specified types. Keys are reference type
    /// names (e.g., "chapter", "article-journal").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "schema",
        schemars(
            schema_with = "crate::template::type_keyed_map_schema::<crate::template::TemplateVariant>"
        )
    )]
    pub type_variants: Option<TemplateVariants>,
    /// Optional global bibliography sorting specification.
    ///
    /// When present, used for sorting the flat bibliography or as default
    /// for groups that don't specify their own sort.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<grouping::GroupSortEntry>,
    /// Whether to apply manual `groups:` bibliography grouping.
    ///
    /// Defaults to `true`. Set to `false` to disable the `groups:` configuration
    /// and render a flat bibliography instead. Automatic sort-partition sections
    /// are unaffected by this toggle.
    // TODO: consider defaulting to false once grouping matures for publishing workflows
    #[serde(default = "default_true")]
    pub groups_enabled: bool,
    /// Optional bibliography grouping specification.
    ///
    /// When present, divides the bibliography into labeled sections with
    /// optional per-group sorting. Items match the first group whose selector
    /// evaluates to true (first-match semantics). Omit for flat bibliography.
    ///
    /// See `BibliographyGroup` for examples.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<grouping::BibliographyGroup>>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
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

impl Default for BibliographySpec {
    fn default() -> Self {
        Self {
            options: None,
            template_ref: None,
            template: None,
            locales: None,
            type_variants: None,
            sort: None,
            groups_enabled: true,
            groups: None,
            custom: None,
            unknown_fields: std::collections::BTreeMap::new(),
        }
    }
}

impl BibliographySpec {
    /// Resolve the effective template for this bibliography.
    ///
    /// Returns the explicit `template` if present, otherwise resolves `template-ref`.
    /// Returns `None` if neither is specified.
    pub fn resolve_template(&self) -> Option<Template> {
        self.template.clone().or_else(|| {
            self.template_ref
                .as_ref()
                .and_then(TemplateReference::bibliography_template)
        })
    }

    /// Resolve a template and the locale selected by its localized branch.
    pub fn resolve_localized_template(
        &self,
        language: Option<&str>,
    ) -> Option<ResolvedLocalizedTemplate> {
        if let Some(matched) = language
            .zip(self.locales.as_deref())
            .and_then(|(language, locales)| matched_localized_template(locales, language))
        {
            return Some(matched);
        }

        self.locales
            .as_ref()
            .and_then(|locales| {
                locales
                    .iter()
                    .find(|spec| spec.default.unwrap_or(false))
                    .map(|spec| ResolvedLocalizedTemplate {
                        template: spec.template.clone(),
                        locale: None,
                        type_variants: spec.type_variants.clone(),
                    })
            })
            .or_else(|| {
                self.resolve_template()
                    .map(|template| ResolvedLocalizedTemplate {
                        template,
                        locale: None,
                        type_variants: None,
                    })
            })
    }

    /// Resolve the template for a language while discarding locale metadata.
    pub fn resolve_template_for_language(&self, language: Option<&str>) -> Option<Template> {
        self.resolve_localized_template(language)
            .map(|resolved| resolved.template)
    }

    /// Resolve a type variant while retaining any locale selected for the reference.
    pub fn resolve_localized_template_for_type(
        &self,
        ref_type: &str,
        language: Option<&str>,
    ) -> Option<ResolvedLocalizedTemplate> {
        let mut resolved = self.resolve_localized_template(language)?;
        if let Some(template) = resolved
            .type_variants
            .as_ref()
            .and_then(|variants| {
                variants.iter().find_map(|(selector, template)| {
                    selector.matches(ref_type).then(|| template.clone())
                })
            })
            .or_else(|| {
                self.type_variants.as_ref().and_then(|variants| {
                    variants.iter().find_map(|(selector, variant)| {
                        selector
                            .matches(ref_type)
                            .then(|| variant.clone().into_template())
                            .flatten()
                    })
                })
            })
        {
            resolved.template = template;
        }
        Some(resolved)
    }

    /// Resolve the bibliography template for a reference type and language.
    pub fn resolve_template_for_type(
        &self,
        ref_type: &str,
        language: Option<&str>,
    ) -> Option<Template> {
        self.resolve_localized_template_for_type(ref_type, language)
            .map(|resolved| resolved.template)
    }
}
