/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citation section specification.

use std::collections::HashMap;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grouping;
use crate::options::CitationOptions;
use crate::template::{
    DelimiterPunctuation, LocalizedTemplateSpec, ResolvedLocalizedTemplate, Template,
    TemplateReference, TemplateVariant, TemplateVariants, matched_localized_template,
};

/// String forms accepted by [`CitationCollapse`]'s hand-written `Deserialize`.
const CITATION_COLLAPSE_STRING_VARIANTS: &[&str] = &["citation-number", "same-author"];

/// Citation collapse behavior for multi-item citations.
///
/// `Deserialize`/`Serialize` are hand-written (see the `impl` blocks below),
/// not derived — a plain `#[derive]` on a tuple variant like `SameAuthor(..)`
/// only accepts the tagged-map form (`{ "same-author": {...} }`), not the bare
/// `same-author` string shorthand. This mirrors `Processing`
/// (`crate::options::processing`), which hand-writes its own impls for the
/// same reason on `Label(..)`.
// `rename_all` is retained for `JsonSchema` derive (the hand-written
// `Serialize`/`Deserialize` impls below already use kebab-case names
// directly).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum CitationCollapse {
    /// Collapse adjacent citation numbers into a numeric range such as `1–3`.
    CitationNumber,
    /// Collapse a same-author multi-item group onto one author name with a
    /// joined year/date list. See `docs/specs/SAME_AUTHOR_COLLAPSE.md`.
    SameAuthor(SameAuthorCollapse),
}

impl Serialize for CitationCollapse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            CitationCollapse::CitationNumber => serializer.serialize_str("citation-number"),
            CitationCollapse::SameAuthor(config) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("same-author", config)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for CitationCollapse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct CitationCollapseVisitor;

        impl<'de> Visitor<'de> for CitationCollapseVisitor {
            type Value = CitationCollapse;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a citation collapse string or map")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<CitationCollapse, E> {
                match v {
                    "citation-number" => Ok(CitationCollapse::CitationNumber),
                    "same-author" => {
                        Ok(CitationCollapse::SameAuthor(SameAuthorCollapse::default()))
                    }
                    other => Err(E::unknown_variant(other, CITATION_COLLAPSE_STRING_VARIANTS)),
                }
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<CitationCollapse, A::Error> {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::invalid_length(0, &"1"))?;
                match key.as_str() {
                    "same-author" => {
                        let config: SameAuthorCollapse = map.next_value()?;
                        Ok(CitationCollapse::SameAuthor(config))
                    }
                    other => Err(de::Error::unknown_field(other, &["same-author"])),
                }
            }
        }

        deserializer.deserialize_any(CitationCollapseVisitor)
    }
}

/// Configuration for [`CitationCollapse::SameAuthor`].
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct SameAuthorCollapse {
    /// How same-year disambiguation suffixes render inside a collapsed group.
    #[serde(default)]
    pub year_suffix: YearSuffixCollapse,
    /// Delimiter joining items within a collapsed same-author group. Mirrors
    /// CSL's `cite-group-delimiter`, which feeds two distinct mechanisms with
    /// top precedence in both: when set, it overrides the engine's existing
    /// year-to-year join fallback, and it overrides `year_suffix_delimiter`
    /// for the suffix-to-suffix join (`year_suffix: Merged`/`Ranged`). `None`
    /// leaves the year join on its existing fallback and the suffix join on
    /// `year_suffix_delimiter` / `citation.multi-cite-delimiter`. See
    /// `docs/specs/SAME_AUTHOR_COLLAPSE.md` §13.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    /// Delimiter preceding a merged/ranged suffix token specifically (e.g.
    /// `2020a-b`). Mirrors CSL's `year-suffix-delimiter`; affects only the
    /// suffix-to-suffix join, beneath `delimiter` (which wins when both are
    /// set) and above the `citation.multi-cite-delimiter` fallback. Has no
    /// effect on the ordinary year-to-year join. See
    /// `docs/specs/SAME_AUTHOR_COLLAPSE.md` §13.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_suffix_delimiter: Option<DelimiterPunctuation>,
}

/// How same-year disambiguation suffixes render inside a same-author collapsed
/// group. Mirrors CSL's `year-suffix` / `year-suffix-ranged` collapse values.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum YearSuffixCollapse {
    /// `Smith (2020a, 2020b)` — Citum's existing behavior; each suffixed year
    /// token stays atomic. See `CITATION_CLUSTER_RENDERING.md` §Same-Year
    /// Disambiguation.
    #[default]
    Separate,
    /// `Smith (2020a, b)` — CSL `collapse="year-suffix"`. Adjacent
    /// same-author, same-year suffixed tokens merge onto one shared year,
    /// joined by the resolved delimiter. See
    /// `docs/specs/SAME_AUTHOR_COLLAPSE.md` §13.
    Merged,
    /// `Smith (2020a–c)` — CSL `collapse="year-suffix-ranged"`. Like
    /// `Merged`, additionally collapsing runs of 3+ consecutive suffixes into
    /// an en-dash range. See `docs/specs/SAME_AUTHOR_COLLAPSE.md` §13.
    Ranged,
}

/// Text-case transform applied when a citation renders at note start.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NoteStartTextCase {
    /// Uppercase the first character of the rendered citation.
    CapitalizeFirst,
    /// Lowercase the rendered citation text.
    Lowercase,
}

/// Citation specification.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct CitationSpec {
    /// Citation-specific option overrides merged over the style config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<CitationOptions>,
    /// Reference to an embedded template preset or external template.
    ///
    /// A complete `template` takes precedence. Pairing `template-ref` with a
    /// template diff in the same section is invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_ref: Option<TemplateReference>,
    /// Default template or inherited template diff when no localized override is selected.
    /// Diffs are materialized as complete templates during style resolution.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub template: Option<TemplateVariant>,
    /// Locale-specific template overrides checked before the default template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locales: Option<Vec<LocalizedTemplateSpec>>,
    /// Type-specific template overrides for citations. When present, replaces
    /// the default citation template for references of the specified types.
    /// Type-variant lookup happens after mode (integral/non-integral) resolution.
    /// If both the main spec and the active mode sub-spec have a `type-variants`
    /// entry for the same type, the mode-specific one wins.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "type-variants"
    )]
    #[cfg_attr(
        feature = "schema",
        schemars(
            schema_with = "crate::template::type_keyed_map_schema::<crate::template::TemplateVariant>"
        )
    )]
    pub type_variants: Option<TemplateVariants>,
    /// Wrap the entire citation in punctuation. Preferred over prefix/suffix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<crate::template::WrapConfig>,
    /// Prefix for the citation (use only when `wrap` doesn't suffice, e.g., " (" or "[Ref ").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<DelimiterPunctuation>,
    /// Suffix for the citation (use only when `wrap` doesn't suffice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<DelimiterPunctuation>,
    /// Delimiter between components within a single citation item (e.g., ", " or " ").
    /// Defaults to ", ".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    /// Delimiter between multiple citation items (e.g., "; ").
    /// Defaults to "; ".
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "multi-cite-delimiter")]
    pub multi_cite_delimiter: Option<DelimiterPunctuation>,
    /// Optional collapse behavior for adjacent multi-item citations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapse: Option<CitationCollapse>,
    /// Optional citation sorting specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<grouping::GroupSortEntry>,
    /// Configuration for integral (narrative) citations (e.g., "Smith (2020)").
    /// Overrides fields from the main citation spec when mode is Integral.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integral: Option<Box<CitationSpec>>,
    /// Configuration for non-integral (parenthetical) citations (e.g., "(Smith, 2020)").
    /// Overrides fields from the main citation spec when mode is NonIntegral.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_integral: Option<Box<CitationSpec>>,
    /// Configuration for subsequent citations.
    /// Overrides fields from the main citation spec when position is Subsequent.
    /// Useful for short-form citations in note-based styles or author-date styles
    /// that show abbreviated citations after the first mention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent: Option<Box<CitationSpec>>,
    /// Configuration for ibid citations (ibid or ibid with locator).
    /// Overrides fields from the main citation spec when position is Ibid or IbidWithLocator.
    /// If present, takes precedence over `subsequent` for these positions.
    /// Allows compact rendering like "ibid." or "ibid., p. 45".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ibid: Option<Box<CitationSpec>>,
    /// Optional text-case transform for standalone note-start citation output.
    ///
    /// This is a style-owned rendering dimension layered on top of the
    /// existing repeated-note state, not a new citation `Position`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_start_text_case: Option<NoteStartTextCase>,
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

impl CitationSpec {
    /// Resolve the effective template for this citation.
    ///
    /// Returns the explicit resolved template if present, otherwise resolves
    /// `template-ref`. A pending diff is not exposed as a concrete template.
    pub fn resolve_template(&self) -> Option<Template> {
        self.template
            .as_ref()
            .and_then(TemplateVariant::as_template)
            .map(<[_]>::to_vec)
            .or_else(|| {
                self.template_ref
                    .as_ref()
                    .and_then(TemplateReference::citation_template)
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

    /// Resolve the template for a given reference type and language.
    ///
    /// First checks `type_variants` for an entry matching `ref_type`.
    /// Falls back to `resolve_template_for_language` if no type-specific
    /// template is found.
    pub fn resolve_template_for_type(
        &self,
        ref_type: &str,
        language: Option<&str>,
    ) -> Option<Template> {
        self.resolve_localized_template_for_type(ref_type, language)
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

    /// Resolve the effective spec for a given citation mode.
    ///
    /// If a mode-specific spec exists (e.g., `integral`), it merges with and overrides
    /// the base spec.
    pub fn resolve_for_mode(
        &self,
        mode: &crate::citation::CitationMode,
    ) -> std::borrow::Cow<'_, CitationSpec> {
        use crate::citation::CitationMode;
        let mode_spec = match mode {
            CitationMode::Integral => self.integral.as_ref(),
            CitationMode::NonIntegral => self.non_integral.as_ref(),
        };

        match mode_spec {
            Some(spec) => {
                // Merge logic: mode specific > base
                let mut merged = self.clone();
                // We don't want to recurse infinitely or keep the mode specs in the merged result
                merged.integral = None;
                merged.non_integral = None;

                match (&mut merged.options, &spec.options) {
                    (Some(base), Some(mode)) => base.merge(mode),
                    (None, Some(mode)) => merged.options = Some(mode.clone()),
                    _ => {}
                }
                if spec.template_ref.is_some() {
                    merged.template_ref = spec.template_ref.clone();
                }
                if spec.template.is_some() {
                    merged.template = spec.template.clone();
                }
                if spec.locales.is_some() {
                    merged.locales = spec.locales.clone();
                }
                if spec.type_variants.is_some() {
                    merged.type_variants = spec.type_variants.clone();
                }
                if spec.wrap.is_some() {
                    merged.wrap = spec.wrap.clone();
                }
                if spec.prefix.is_some() {
                    merged.prefix = spec.prefix.clone();
                }
                if spec.suffix.is_some() {
                    merged.suffix = spec.suffix.clone();
                }
                if spec.delimiter.is_some() {
                    merged.delimiter = spec.delimiter.clone();
                }
                if spec.multi_cite_delimiter.is_some() {
                    merged.multi_cite_delimiter = spec.multi_cite_delimiter.clone();
                }
                if spec.collapse.is_some() {
                    merged.collapse = spec.collapse.clone();
                }
                if spec.sort.is_some() {
                    merged.sort = spec.sort.clone();
                }
                if spec.note_start_text_case.is_some() {
                    merged.note_start_text_case = spec.note_start_text_case;
                }

                std::borrow::Cow::Owned(merged)
            }
            None => std::borrow::Cow::Borrowed(self),
        }
    }

    /// Resolve the effective spec for a given citation position.
    ///
    /// If a position-specific spec exists (e.g., `ibid` for Ibid position),
    /// it merges with and overrides the base spec. Position resolution should
    /// be applied before mode resolution to allow position-specific modes.
    ///
    /// Priority: ibid > subsequent > base
    pub fn resolve_for_position(
        &self,
        position: Option<&crate::citation::Position>,
    ) -> std::borrow::Cow<'_, CitationSpec> {
        use crate::citation::Position;

        let position_spec = match position {
            Some(Position::Ibid | Position::IbidWithLocator) => {
                self.ibid.as_ref().or(self.subsequent.as_ref())
            }
            Some(Position::Subsequent) => self.subsequent.as_ref(),
            Some(Position::First) | None => None,
        };

        match position_spec {
            Some(spec) => {
                // Merge logic: position specific > base
                let mut merged = self.clone();
                // Don't recurse infinitely or keep position specs in merged result
                merged.subsequent = None;
                merged.ibid = None;

                match (&mut merged.options, &spec.options) {
                    (Some(base), Some(mode)) => base.merge(mode),
                    (None, Some(mode)) => merged.options = Some(mode.clone()),
                    _ => {}
                }
                if spec.template_ref.is_some() {
                    merged.template_ref = spec.template_ref.clone();
                }
                if spec.template.is_some() {
                    merged.template = spec.template.clone();
                    // A position spec with its own template is a complete override —
                    // clear inherited type_variants so the engine uses this template
                    // directly rather than branching by ref type. If the position spec
                    // wants type-specific rendering it must declare type_variants itself.
                    if spec.type_variants.is_none() {
                        merged.type_variants = None;
                    }
                }
                if spec.locales.is_some() {
                    merged.locales = spec.locales.clone();
                }
                if spec.type_variants.is_some() {
                    merged.type_variants = spec.type_variants.clone();
                }
                if spec.wrap.is_some() {
                    merged.wrap = spec.wrap.clone();
                }
                if spec.prefix.is_some() {
                    merged.prefix = spec.prefix.clone();
                }
                if spec.suffix.is_some() {
                    merged.suffix = spec.suffix.clone();
                }
                if spec.delimiter.is_some() {
                    merged.delimiter = spec.delimiter.clone();
                }
                if spec.multi_cite_delimiter.is_some() {
                    merged.multi_cite_delimiter = spec.multi_cite_delimiter.clone();
                }
                if spec.collapse.is_some() {
                    merged.collapse = spec.collapse.clone();
                }
                if spec.sort.is_some() {
                    merged.sort = spec.sort.clone();
                }
                if spec.note_start_text_case.is_some() {
                    merged.note_start_text_case = spec.note_start_text_case;
                }

                std::borrow::Cow::Owned(merged)
            }
            None => std::borrow::Cow::Borrowed(self),
        }
    }
}
