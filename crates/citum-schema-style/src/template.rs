/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Template components for Citum styles.
//!
//! This module defines the declarative template language for Citum.
//! Unlike CSL 1.0's procedural rendering elements, these components
//! are simple, typed instructions that the processor interprets.
//!
//! ## Design Philosophy
//!
//! **Explicit over magic**: All rendering behavior should be expressible in the
//! style YAML. The processor should not have hidden conditional logic based on
//! reference types. Instead, use `overrides` to declare type-specific behavior.
//!
//! ## Type-Specific Overrides
//!
//! Components support `overrides` to customize rendering per reference type:
//!
//! ```yaml
//! - variable: publisher
//!   overrides:
//!     article-journal:
//!       suppress: true  # Don't show publisher for journals
//! - number: pages
//!   overrides:
//!     chapter:
//!       wrap: parentheses
//!       label-form: short  # Show as "(pp. 1-10)" for English chapters
//! ```
//!
//! This keeps all conditional logic in the style, making it testable and portable.

use crate::locale::{GeneralTerm, GrammaticalGender, TermForm};
use indexmap::IndexMap;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

mod reference;
pub(crate) mod resolution;

pub(crate) use reference::matched_localized_template;
pub use reference::{
    LocalizedTemplateSpec, ResolvedLocalizedTemplate, TemplatePreset, TemplateReference,
};
pub(crate) use resolution::{inherited_variant_context, resolve_style_template_variants};

/// Resolve a style's local template variants in place without inherited
/// context, materializing every diff variant as a full template.
///
/// Diff variants are resolved against the style's own section templates and
/// intra-section `extends` chains. Emitters that re-parent a style (for
/// example the migration wrapper path) use this before attaching `extends`:
/// a diff derived against the local template would otherwise resolve against
/// the parent's same-selector variant at render time.
///
/// # Errors
///
/// Returns a [`crate::ResolutionError`] when a variant cycle, missing
/// parent, or non-matching diff operation is found.
pub fn resolve_local_template_variants(
    style: &mut crate::Style,
) -> Result<(), crate::ResolutionError> {
    resolution::resolve_style_template_variants(style, None)
}

/// A named template (reusable sequence of components).
pub type Template = Vec<TemplateComponent>;

/// Type-specific template variants keyed by reference-type selector.
pub type TemplateVariants = IndexMap<TypeSelector, TemplateVariant>;

/// Locale-owned type-specific template replacements keyed by reference-type selector.
///
/// Localized variants are complete templates because they select after the section's
/// main type-variant resolution has completed.
pub type LocalizedTemplateVariants = IndexMap<TypeSelector, Template>;

/// Vertical text alignment relative to the baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum VerticalAlign {
    /// Render at the baseline (default).
    Baseline,
    /// Render as superscript.
    Superscript,
    /// Render as subscript.
    Subscript,
}

/// Rendering instructions applied to template components.
///
/// These fields are flattened into parent structs, so in YAML you write:
/// ```yaml
/// - title: primary
///   emph: true
///   prefix: "In "
/// ```
/// Rather than nesting under a `rendering:` key.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", default)]
pub struct Rendering {
    /// Text-case transform to apply to the rendered value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_case: Option<crate::options::titles::TextCase>,
    /// Render in italics/emphasis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    /// Render in quotes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<bool>,
    /// Render in bold/strong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    /// Render in small caps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    /// Vertical alignment to apply to rendered output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    /// Text or a semantic punctuation mark to prepend to the rendered value
    /// (outside any wrap).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<DelimiterPunctuation>,
    /// Text or a semantic punctuation mark to append to the rendered value
    /// (outside any wrap).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<DelimiterPunctuation>,
    /// Wrapping punctuation and optional inner affixes (text inside the wrap).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<WrapConfig>,
    /// If true, suppress this component entirely (render as empty string).
    /// Useful for type-specific overrides like suppressing publisher for journals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
    /// Override name initialization (e.g., ". " or "").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    /// Override name form (e.g., initials, full, family-only).
    #[serde(skip_serializing_if = "Option::is_none", rename = "name-form")]
    pub name_form: Option<crate::options::contributors::NameForm>,
    /// Strip trailing periods from rendered value.
    #[serde(skip_serializing_if = "Option::is_none", rename = "strip-periods")]
    pub strip_periods: Option<bool>,
}

impl Rendering {
    /// Merge another rendering configuration into this one.
    ///
    /// The other rendering takes precedence, overwriting any fields that are present.
    pub fn merge(&mut self, other: &Rendering) {
        crate::merge_options!(
            self,
            other,
            text_case,
            emph,
            quote,
            strong,
            small_caps,
            vertical_align,
            prefix,
            suffix,
            wrap,
            suppress,
            initialize_with,
            name_form,
            strip_periods,
        );
    }
}

/// Punctuation to wrap a component in.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum WrapPunctuation {
    #[default]
    Parentheses,
    Brackets,
    Quotes,
}

/// Wrapping punctuation and optional inner affixes applied around a rendered value.
///
/// Combines the wrap punctuation with optional text that appears inside the wrap
/// (between the wrap character and the rendered content).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct WrapConfig {
    /// The wrapping punctuation style.
    pub punctuation: WrapPunctuation,
    /// Text inserted after the opening wrap character but before the content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_prefix: Option<String>,
    /// Text inserted after the content but before the closing wrap character.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_suffix: Option<String>,
}

impl<'de> serde::Deserialize<'de> for WrapConfig {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct WrapConfigVisitor;

        impl<'de> serde::de::Visitor<'de> for WrapConfigVisitor {
            type Value = WrapConfig;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    f,
                    "a wrap punctuation string or a mapping with a 'punctuation' key"
                )
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<WrapConfig, E> {
                let punctuation = match v {
                    "parentheses" => WrapPunctuation::Parentheses,
                    "brackets" => WrapPunctuation::Brackets,
                    "quotes" => WrapPunctuation::Quotes,
                    other => {
                        return Err(E::unknown_variant(
                            other,
                            &["parentheses", "brackets", "quotes"],
                        ));
                    }
                };
                Ok(WrapConfig {
                    punctuation,
                    inner_prefix: None,
                    inner_suffix: None,
                })
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<WrapConfig, A::Error> {
                let mut punctuation: Option<WrapPunctuation> = None;
                let mut inner_prefix: Option<String> = None;
                let mut inner_suffix: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "punctuation" => {
                            punctuation = Some(map.next_value()?);
                        }
                        "inner-prefix" => {
                            inner_prefix = Some(map.next_value()?);
                        }
                        "inner-suffix" => {
                            inner_suffix = Some(map.next_value()?);
                        }
                        other => {
                            return Err(serde::de::Error::unknown_field(
                                other,
                                &["punctuation", "inner-prefix", "inner-suffix"],
                            ));
                        }
                    }
                }

                let punctuation =
                    punctuation.ok_or_else(|| serde::de::Error::missing_field("punctuation"))?;
                Ok(WrapConfig {
                    punctuation,
                    inner_prefix,
                    inner_suffix,
                })
            }
        }

        deserializer.deserialize_any(WrapConfigVisitor)
    }
}

impl From<WrapPunctuation> for WrapConfig {
    fn from(punctuation: WrapPunctuation) -> Self {
        WrapConfig {
            punctuation,
            inner_prefix: None,
            inner_suffix: None,
        }
    }
}

/// Canonical reference type names recognized by the Citum engine.
///
/// Used by [`validate_type_name`] to detect likely typos.
pub const VALID_TYPE_NAMES: &[&str] = &[
    "book",
    "manual",
    "report",
    "thesis",
    "webpage",
    "map",
    "post",
    "interview",
    "manuscript",
    "personal-communication",
    "document",
    "chapter",
    "entry-dictionary",
    "paper-conference",
    "article-journal",
    "article-magazine",
    "article-newspaper",
    "broadcast",
    "motion-picture",
    "collection",
    "legal-case",
    "statute",
    "treaty",
    "hearing",
    "regulation",
    "brief",
    "classic",
    "patent",
    "dataset",
    "standard",
    "software",
    // Special keywords
    "all",
    "default",
];

/// Returns `true` if `s` is a recognized reference type name.
///
/// Normalizes underscores to hyphens before comparing, so both
/// `"article_journal"` and `"article-journal"` are accepted.
/// Returns `false` for unrecognized names (likely typos).
pub fn validate_type_name(s: &str) -> bool {
    let normalized = s.replace('_', "-");
    VALID_TYPE_NAMES.iter().any(|&known| known == normalized)
}

/// Selector for reference types in overrides.
/// Can be a single type string or a list of types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum TypeSelector {
    Single(String),
    Multiple(Vec<String>),
}

impl Serialize for TypeSelector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TypeSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = TypeSelector;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or a sequence of strings")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(E::custom)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut types = Vec::new();
                while let Some(t) = seq.next_element::<String>()? {
                    types.push(t);
                }
                if types.len() == 1 {
                    Ok(TypeSelector::Single(types.remove(0)))
                } else {
                    Ok(TypeSelector::Multiple(types))
                }
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl std::fmt::Display for TypeSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeSelector::Single(s) => write!(f, "{s}"),
            TypeSelector::Multiple(types) => write!(f, "{}", types.join(",")),
        }
    }
}

impl std::str::FromStr for TypeSelector {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(',') {
            Ok(TypeSelector::Multiple(
                s.split(',').map(|t| t.trim().to_string()).collect(),
            ))
        } else {
            Ok(TypeSelector::Single(s.to_string()))
        }
    }
}

impl TypeSelector {
    /// Check whether this selector matches a reference type.
    ///
    /// Type names are compared after normalizing underscores to hyphens, so
    /// "legal_case" and "legal-case" are treated as equivalent (matching both
    /// CSL 1.0 underscore convention and Citum hyphen convention).
    ///
    /// The special keyword "all" always matches any reference type.
    pub fn matches(&self, ref_type: &str) -> bool {
        let normalized_ref = ref_type.replace('_', "-");
        let base_ref = normalized_ref
            .split_once('+')
            .map(|(base, _)| base)
            .unwrap_or(&normalized_ref);
        let eq = |s: &str| -> bool {
            s == ref_type
                || s.replace('_', "-") == normalized_ref
                || s.replace('_', "-") == base_ref
                || s == "all"
                || (s == "default" && ref_type == "default")
        };
        match self {
            TypeSelector::Single(s) => eq(s),
            TypeSelector::Multiple(types) => types.iter().any(|t| eq(t)),
        }
    }

    /// Returns any type names in this selector that are not in [`VALID_TYPE_NAMES`].
    ///
    /// An empty vec means all names are valid. Callers should emit a
    /// [`crate::SchemaWarning`] for each returned name.
    pub fn unknown_type_names(&self) -> Vec<&str> {
        match self {
            TypeSelector::Single(s) => {
                if validate_type_name(s) {
                    vec![]
                } else {
                    vec![s.as_str()]
                }
            }
            TypeSelector::Multiple(types) => types
                .iter()
                .filter(|s| !validate_type_name(s))
                .map(|s| s.as_str())
                .collect(),
        }
    }
}

/// A template component - the building blocks of citation/bibliography templates.
///
/// Each variant handles a specific data type with appropriate formatting options.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
#[non_exhaustive]
pub enum TemplateComponent {
    Contributor(TemplateContributor),
    Date(TemplateDate),
    Title(TemplateTitle),
    Number(TemplateNumber),
    Identifier(TemplateIdentifier),
    Variable(TemplateVariable),
    Message(TemplateMessage),
    Group(TemplateGroup),
    Term(TemplateTerm),
    TypeLabel(TemplateTypeLabel),
}

impl Default for TemplateComponent {
    fn default() -> Self {
        TemplateComponent::Variable(TemplateVariable::default())
    }
}

impl TemplateComponent {
    /// Return the rendering options for this component.
    ///
    /// Every template component has rendering options like emphasis, wrapping, and prefixes.
    pub fn rendering(&self) -> &Rendering {
        crate::dispatch_component!(self, |inner| &inner.rendering)
    }

    /// Return the mutable rendering options for this component.
    ///
    /// Provides mutable access to rendering fields (prefix, suffix, etc.)
    /// that are present on all template component variants.
    pub fn rendering_mut(&mut self) -> &mut Rendering {
        crate::dispatch_component!(self, |inner| &mut inner.rendering)
    }
}

/// Type-specific template override, either as a complete legacy template or a V3 diff.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum TemplateVariant {
    /// Complete replacement template used by Template V1/V2 styles.
    Full(Vec<TemplateComponent>),
    /// Structural diff applied to a parent template during style resolution.
    Diff(TemplateVariantDiff),
}

impl TemplateVariant {
    /// Return this variant as a concrete template if it has already been resolved.
    #[must_use]
    pub fn as_template(&self) -> Option<&[TemplateComponent]> {
        match self {
            Self::Full(template) => Some(template.as_slice()),
            Self::Diff(_) => None,
        }
    }

    /// Return this variant as a mutable concrete template if it has already been resolved.
    pub fn as_template_mut(&mut self) -> Option<&mut Vec<TemplateComponent>> {
        match self {
            Self::Full(template) => Some(template),
            Self::Diff(_) => None,
        }
    }

    /// Convert this variant into its concrete template if it has already been resolved.
    #[must_use]
    pub fn into_template(self) -> Option<Vec<TemplateComponent>> {
        match self {
            Self::Full(template) => Some(template),
            Self::Diff(_) => None,
        }
    }
}

impl From<Vec<TemplateComponent>> for TemplateVariant {
    fn from(template: Vec<TemplateComponent>) -> Self {
        Self::Full(template)
    }
}

/// Structural diff that derives a type-specific template from a parent template.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateVariantDiff {
    /// Optional parent type variant selector within the same section.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<TypeSelector>,
    /// Rendering-only modifications applied in authored order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modify: Vec<TemplateModifyOperation>,
    /// Component removals applied in authored order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub remove: Vec<TemplateRemoveOperation>,
    /// Component additions applied in authored order.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub add: Vec<TemplateAddOperation>,
}

/// Partial component selector used to locate anchors in a template.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(transparent)]
pub struct TemplateComponentSelector {
    /// Component fields that must be present with equal values on the target component.
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl TemplateComponentSelector {
    /// Returns `true` when this selector has no fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns `true` when every selector field is present with the same value.
    #[must_use]
    pub fn matches(&self, component: &TemplateComponent) -> bool {
        let Ok(serde_json::Value::Object(component_fields)) = serde_json::to_value(component)
        else {
            return false;
        };

        self.fields.iter().all(|(key, expected)| {
            component_fields
                .get(key)
                .is_some_and(|actual| selector_value_matches(expected, actual))
        })
    }
}

fn selector_value_matches(expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
    match (expected, actual) {
        (serde_json::Value::Object(expected_fields), serde_json::Value::Object(actual_fields)) => {
            expected_fields.iter().all(|(key, expected_value)| {
                actual_fields.get(key).is_some_and(|actual_value| {
                    selector_value_matches(expected_value, actual_value)
                })
            })
        }
        (serde_json::Value::Array(expected_items), serde_json::Value::Array(actual_items)) => {
            expected_items.len() == actual_items.len()
                && expected_items.iter().zip(actual_items.iter()).all(
                    |(expected_item, actual_item)| {
                        selector_value_matches(expected_item, actual_item)
                    },
                )
        }
        _ => expected == actual,
    }
}

/// Rendering-only modification for the component matched by `match`.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateModifyOperation {
    /// Selector identifying exactly one component to modify.
    #[serde(rename = "match")]
    pub match_selector: TemplateComponentSelector,
    /// Override the localized number label form when modifying number components.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_form: Option<LabelForm>,
    /// Rendering fields to merge onto the matched component.
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// Removal operation for the component matched by `match`.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateRemoveOperation {
    /// Selector identifying exactly one component to remove.
    #[serde(rename = "match")]
    pub match_selector: TemplateComponentSelector,
}

/// Addition operation that inserts a component before or after an anchor.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateAddOperation {
    /// Anchor selector before which the component should be inserted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<TemplateComponentSelector>,
    /// Anchor selector after which the component should be inserted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<TemplateComponentSelector>,
    /// Component to insert.
    pub component: TemplateComponent,
}

/// Configuration for role labels (e.g., "eds.", "trans.").
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct RoleLabel {
    /// Locale term key for the role (e.g., "editor", "translator").
    pub term: String,
    /// Term form: short ("eds.") or long ("editors").
    #[serde(default)]
    pub form: RoleLabelForm,
    /// Where to place the label relative to names.
    #[serde(default)]
    pub placement: LabelPlacement,
    /// Optional case transform applied to the resolved label term, e.g.
    /// `capitalize-first` renders "Eds." from the locale's "eds." (as IEEE
    /// requires). When unset the term is rendered as the locale stores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_case: Option<crate::options::titles::TextCase>,
    /// Optional punctuation wrapped around the resolved label term.
    ///
    /// The wrap is applied before the label's outer `prefix` and `suffix`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<Box<WrapConfig>>,
    /// Optional affix rendered before the label term, overriding the
    /// placement-derived default (a space for a wrapped suffix label, `", "`
    /// for an unwrapped suffix label, and empty for prefix placement). Mirrors
    /// CSL 1.0 `cs:label` `prefix` (e.g. `" ("` for elsevier's `" (Eds.)"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<DelimiterPunctuation>,
    /// Optional affix rendered after the label term, overriding the
    /// placement-derived default (empty for suffix placement, `" "` for
    /// prefix placement). Mirrors CSL 1.0 `cs:label` `suffix` (e.g. `")"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<DelimiterPunctuation>,
}

/// Term form for role labels.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RoleLabelForm {
    #[default]
    Short,
    Long,
}

/// Label placement relative to contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelPlacement {
    Prefix,
    #[default]
    Suffix,
}

/// One contributor role or an ordered list of roles rendered as one name list.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum ContributorRoles {
    /// A conventional single-role contributor component.
    Single(ContributorRole),
    /// Two or more contributor roles rendered as a merged list.
    Multiple(#[cfg_attr(feature = "schema", schemars(length(min = 2)))] Vec<ContributorRole>),
}

impl Default for ContributorRoles {
    fn default() -> Self {
        Self::Single(ContributorRole::Author)
    }
}

impl ContributorRoles {
    /// Return all declared roles in authoring order.
    #[must_use]
    pub fn as_slice(&self) -> &[ContributorRole] {
        match self {
            Self::Single(role) => std::slice::from_ref(role),
            Self::Multiple(roles) => roles,
        }
    }

    /// Return the role when this is the scalar form.
    #[must_use]
    pub fn as_single(&self) -> Option<&ContributorRole> {
        match self {
            Self::Single(role) => Some(role),
            Self::Multiple(_) => None,
        }
    }

    /// Return whether this is the list form.
    #[must_use]
    pub fn is_multiple(&self) -> bool {
        matches!(self, Self::Multiple(_))
    }

    /// Return whether the declaration contains `role`.
    #[must_use]
    pub fn contains(&self, role: &ContributorRole) -> bool {
        self.as_slice().contains(role)
    }
}

impl From<ContributorRole> for ContributorRoles {
    fn from(role: ContributorRole) -> Self {
        Self::Single(role)
    }
}

impl From<Vec<ContributorRole>> for ContributorRoles {
    fn from(roles: Vec<ContributorRole>) -> Self {
        Self::Multiple(roles)
    }
}

impl PartialEq<ContributorRole> for ContributorRoles {
    fn eq(&self, other: &ContributorRole) -> bool {
        self.as_single() == Some(other)
    }
}

/// Ordering policy for a merged contributor list.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ContributorMergeOrder {
    /// Preserve the unified reference contributor order.
    #[default]
    Document,
    /// Group entries by the component's declared role order.
    Role,
}

/// Role-label placement mode for merged contributor entries.
#[derive(Debug, Default, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ContributorLabelMode {
    /// Attach a singular role label to every rendered person.
    #[default]
    Individual,
    /// Attach one singular or plural label to each contiguous role run.
    Collective,
    /// Render names without role labels.
    None,
}

/// Per-role overrides within a merged contributor list.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ContributorMergeRole {
    /// Override the merged list's default label mode for this role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<ContributorLabelMode>,
    /// Override label term, form, placement, case, and affixes for this role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<RoleLabel>,
}

/// Configuration for rendering multiple contributor roles as one name list.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ContributorMerge {
    /// Effective ordering of entries in the merged list.
    #[serde(default)]
    pub order: ContributorMergeOrder,
    /// Default role-label mode for entries in the merged list.
    #[serde(default)]
    pub labels: ContributorLabelMode,
    /// Optional per-role label overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub roles: HashMap<ContributorRole, ContributorMergeRole>,
    /// Whether identical people in different roles render as one entry.
    #[serde(default = "default_combine_same_person")]
    pub combine_same_person: bool,
    /// Verbatim connector used when composing a missing combined-role term.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_conjunction: Option<String>,
}

fn default_combine_same_person() -> bool {
    true
}

impl Default for ContributorMerge {
    fn default() -> Self {
        Self {
            order: ContributorMergeOrder::Document,
            labels: ContributorLabelMode::Individual,
            roles: HashMap::new(),
            combine_same_person: true,
            role_conjunction: None,
        }
    }
}

/// A contributor component for rendering names.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateContributor {
    /// Which contributor role or ordered role list to render.
    pub contributor: ContributorRoles,
    /// How to display the contributor (long names, short, with label, etc.).
    pub form: ContributorForm,
    /// Components rendered when the author slot has no contributor, no
    /// title/editor/translator substitute matched, and `substitute.template`
    /// is exhausted — e.g. `message: term.anonymous` for GB/T 7714's `佚名`
    /// placeholder. Only consulted for `contributor: author`; other roles
    /// (editor, translator, ...) are unaffected. Mirrors
    /// `TemplateDate.fallback` in shape and in the "tried in order, first
    /// non-empty wins" semantics. See `csl26-6eak`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Vec<TemplateComponent>>,
    /// Optional role label configuration (e.g., "eds." for editors).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<RoleLabel>,
    /// Configuration used when `contributor` is an ordered role list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge: Option<ContributorMerge>,
    /// Override the global name order for this specific component.
    /// Use to show editors as "Given Family" even when global setting is "Family, Given".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_order: Option<NameOrder>,
    /// Override the name form (e.g., initials, full, family-only) for this specific component.
    #[serde(skip_serializing_if = "Option::is_none", rename = "name-form")]
    pub name_form: Option<crate::options::contributors::NameForm>,
    /// Custom delimiter between names (overrides global setting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    /// Delimiter between family and given name when inverted (overrides global setting).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    /// Shorten the list of names (et al. configuration).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shorten: Option<crate::options::ShortenListOptions>,
    /// Override the conjunction between the last two names.
    /// Use `none` for bibliography when citation uses `text` or `symbol`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<crate::options::AndOptions>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Explicit grammatical gender override for role-label agreement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<GrammaticalGender>,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Name display order.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NameOrder {
    /// Display as "Given Family" (e.g., "John Smith").
    GivenFirst,
    /// Display as "Family, Given" (e.g., "Smith, John").
    #[default]
    FamilyFirst,
    /// First contributor inverted ("Family, Given"); subsequent contributors given-first.
    FamilyFirstOnly,
    /// Every contributor except the last inverted ("Family, Given"); the last
    /// contributor rendered given-first. "Last" is the last name of the full
    /// contributor list; under et-al truncation that name may be elided, in
    /// which case all rendered names invert.
    FamilyFirstExceptLast,
}

/// How to render contributor names.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ContributorForm {
    #[default]
    Long,
    Short,
    FamilyOnly,
    Verb,
    VerbShort,
}

crate::str_enum! {
    /// Contributor roles.
    #[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
    pub enum ContributorRole {
        #[default] Author = "author",
        Chair = "chair",
        Editor = "editor",
        Translator = "translator",
        /// Author of annotations accompanying the work.
        Annotator = "annotator",
        /// Author of a commentary on the work.
        Commentator = "commentator",
        /// Author of a foreword accompanying the work.
        ForewordAuthor = "foreword-author",
        /// Author of an introduction accompanying the work.
        IntroductionAuthor = "introduction-author",
        /// Author of an afterword accompanying the work.
        AfterwordAuthor = "afterword-author",
        Director = "director",
        Publisher = "publisher",
        Recipient = "recipient",
        Interviewer = "interviewer",
        Interviewee = "interviewee",
        Guest = "guest",
        Performer = "performer",
        Inventor = "inventor",
        Counsel = "counsel",
        Composer = "composer",
        Writer = "writer",
        Producer = "producer",
        CollectionEditor = "collection-editor",
        ContainerAuthor = "container-author",
        EditorialDirector = "editorial-director",
        TextualEditor = "textual-editor",
        Illustrator = "illustrator",
        OriginalAuthor = "original-author",
        ReviewedAuthor = "reviewed-author"
    }
}

/// A date component for rendering dates.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateDate {
    pub date: DateVariable,
    pub form: DateForm,
    /// Authoritative fallback components used when the primary date is missing.
    ///
    /// When every component is empty, including for an empty list, the date is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<Vec<TemplateComponent>>,
    /// When true, never wrap this component's opaque calendar-date `note`
    /// (e.g. a Minguo/era annotation), regardless of the section's
    /// `note-wrap` setting. Use on the redundant occurrence when a style
    /// legitimately renders the same date variable more than once per item
    /// (e.g. a short front-matter year plus a full-precision date later in
    /// the body) so the annotation appears exactly once rather than on every
    /// occurrence. See `docs/specs/CALENDAR_DATE_ANNOTATIONS.md` and
    /// `csl26-gl0n`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_note: Option<bool>,
    /// When true, never inline a year-suffix disambiguator (e.g. "1947a")
    /// into this component's rendering, regardless of `hints.disamb_condition`.
    /// Use on the redundant occurrence when a style legitimately renders
    /// `issued` more than once per item — the mirror of `suppress_note`: a
    /// dual-date shape typically wants the suffix on the short front year
    /// and the calendar-note annotation on the full body date, so each flag
    /// suppresses the opposite occurrence's copy. See `csl26-6eak`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_disamb_suffix: Option<bool>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Date variables.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DateVariable {
    #[default]
    Issued,
    Accessed,
    OriginalPublished,
    Submitted,
    EventDate,
    /// Copyright year, used as a publication-year substitute when the true
    /// issue date is unknown (e.g. GB/T 7714 §7.5.4.3's `c1988`).
    Copyright,
    /// Printing/impression year, another publication-year substitute (e.g.
    /// GB/T 7714 §7.5.4.3's `1995印刷`).
    Printing,
}

crate::str_enum! {
    /// Date rendering forms.
    #[derive(Debug, Default, Clone, PartialEq)]
    pub enum DateForm {
        #[default]
        Year = "year",
        YearMonth = "year-month",
        /// Month name only, no year or day: "June" (e.g. magazines whose year
        /// is already supplied by the author-date position).
        Month = "month",
        Full = "full",
        MonthDay = "month-day",
        YearMonthDay = "year-month-day",
        DayMonthAbbrYear = "day-month-abbr-year",
        /// Abbreviated month + day + year in US order: "Jan 15, 2024".
        MonthAbbrDayYear = "month-abbr-day-year"
    }
}

/// A title component.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateTitle {
    pub title: TitleType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TitleForm>,
    /// When true, suppress this title component unless the reference needs
    /// disambiguation (i.e. multiple works by the same author appear in the
    /// document). Used by author-class styles (e.g. MLA) where the title
    /// appears in citations only to resolve same-author ambiguity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguate_only: Option<bool>,
    /// When true, remove every period from the rendered title text (e.g. an
    /// abbreviated journal name "Br. Med. J." → "Br Med J").
    ///
    /// Deliberately separate from the shared `Rendering::strip_periods`
    /// (which only trims a single *trailing* period elsewhere in the
    /// engine, e.g. term/number rendering): a title can legitimately
    /// contain a period as ordinary text (a proper noun, a domain name like
    /// "Merriam-Webster.com"), so full-period removal is opt-in per
    /// component rather than folded into the general-purpose flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_periods_all: Option<bool>,
    #[serde(flatten, default)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Types of titles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TitleType {
    /// The primary title of the cited work.
    #[default]
    Primary,
    /// Title of the parent work containing the cited work.
    ContainerTitle,
    /// Title of a book/monograph containing the cited work.
    ParentMonograph,
    /// Title of a periodical/serial containing the cited work.
    ParentSerial,
    /// Title of a series or collection containing the cited work.
    CollectionTitle,
    /// Title of the work's original publication (e.g. a translation's source-language title).
    Original,
}

/// Title rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TitleForm {
    Short,
    #[default]
    Long,
}

/// A number component (volume, issue, pages, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateNumber {
    pub number: NumberVariable,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<NumberForm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_form: Option<LabelForm>,
    /// When `true`, show this pages component even when a locator is present in a note-style citation.
    /// By default, pages are suppressed in note-style citations when a locator is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_with_locator: Option<bool>,
    #[serde(flatten)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,
    /// Explicit grammatical gender override for number/ordinal agreement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<GrammaticalGender>,
    /// When set, resolve this number's locale term (e.g. GB/T 7714's `edition`
    /// or `volume` general terms) at the given form and wrap the value with
    /// it — but only when the resolved value is numeric (citeproc-style
    /// `is-numeric`). Non-numeric values — including free-text editions
    /// (`修订版`) and pre-labeled volumes (`美国卷`) — render bare, since the
    /// source standard treats those as already-complete strings.
    ///
    /// The term text is locale-owned, not style-owned: a term containing a
    /// literal `%s` (e.g. zh-CN's `第%s卷`, matching the CSL-M source term)
    /// wraps the value at that position; a term without `%s` (e.g. `版`)
    /// follows the value as a space-separated suffix. See
    /// `docs/specs/TEMPLATE_V3.md` §2.4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_numeric: Option<LabelForm>,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Number variables.
///
/// Use `number:` when the value is treated as a number by the style:
/// numeric labels, numeric-specific formatting, ordinals, roman numerals, or
/// locator-aware punctuation. Use `variable:` instead when the field should be
/// passed through as plain text without number formatting semantics.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub enum NumberVariable {
    #[default]
    Volume,
    Issue,
    Pages,
    Edition,
    ChapterNumber,
    CollectionNumber,
    NumberOfPages,
    NumberOfVolumes,
    /// First-occurrence note number for the cited reference (note styles only).
    /// Populated from the document processor; omitted (not rendered) when the
    /// citation is not in a subsequent position or no first-note number is available.
    FirstReferenceNoteNumber,
    Number,
    DocketNumber,
    PatentNumber,
    StandardNumber,
    ReportNumber,
    PartNumber,
    SupplementNumber,
    PrintingNumber,
    /// A custom numbering variable rendered from an arbitrary numbering kind.
    Custom(String),
}

impl NumberVariable {
    /// Return the canonical kebab-case key for this numeric variable.
    #[must_use]
    pub fn as_key(&self) -> Cow<'_, str> {
        match self {
            Self::Volume => Cow::Borrowed("volume"),
            Self::Issue => Cow::Borrowed("issue"),
            Self::Pages => Cow::Borrowed("pages"),
            Self::Edition => Cow::Borrowed("edition"),
            Self::ChapterNumber => Cow::Borrowed("chapter-number"),
            Self::CollectionNumber => Cow::Borrowed("collection-number"),
            Self::NumberOfPages => Cow::Borrowed("number-of-pages"),
            Self::NumberOfVolumes => Cow::Borrowed("number-of-volumes"),
            Self::FirstReferenceNoteNumber => Cow::Borrowed("first-reference-note-number"),
            Self::Number => Cow::Borrowed("number"),
            Self::DocketNumber => Cow::Borrowed("docket-number"),
            Self::PatentNumber => Cow::Borrowed("patent-number"),
            Self::StandardNumber => Cow::Borrowed("standard-number"),
            Self::ReportNumber => Cow::Borrowed("report-number"),
            Self::PartNumber => Cow::Borrowed("part-number"),
            Self::SupplementNumber => Cow::Borrowed("supplement-number"),
            Self::PrintingNumber => Cow::Borrowed("printing-number"),
            Self::Custom(value) => normalize_kind_key(value)
                .map(Cow::Owned)
                .unwrap_or_else(|| Cow::Borrowed(value.as_str())),
        }
    }

    fn from_key(value: &str) -> Result<Self, String> {
        let canonical = normalize_kind_key(value)
            .ok_or_else(|| "number variable must not be empty".to_string())?;
        Ok(match canonical.as_str() {
            "volume" => Self::Volume,
            "issue" => Self::Issue,
            "pages" => Self::Pages,
            "edition" => Self::Edition,
            "chapter-number" => Self::ChapterNumber,
            "collection-number" => Self::CollectionNumber,
            "number-of-pages" => Self::NumberOfPages,
            "number-of-volumes" => Self::NumberOfVolumes,
            // `NumberVariable` is an open vocabulary — unknown kebab-case keys
            // become `Custom` numbering kinds. These two names are reserved:
            // they denote processor-owned reference markers, so without an
            // explicit reservation they would parse as custom kinds and render
            // nothing. See `docs/specs/REFERENCE_MARKERS.md`.
            "citation-number" => {
                return Err(
                    "`citation-number` is a processor-owned reference marker, not a \
                            number variable: declare `label-mode: numeric` on citation.options \
                            or bibliography.options"
                        .to_string(),
                );
            }
            "citation-label" => {
                return Err(
                    "`citation-label` is a processor-owned reference marker, not a \
                            number variable: declare `label-mode: alphabetic` on \
                            citation.options or bibliography.options"
                        .to_string(),
                );
            }
            "first-reference-note-number" => Self::FirstReferenceNoteNumber,
            "number" => Self::Number,
            "docket-number" => Self::DocketNumber,
            "patent-number" => Self::PatentNumber,
            "standard-number" => Self::StandardNumber,
            "report-number" => Self::ReportNumber,
            "part-number" => Self::PartNumber,
            "supplement-number" => Self::SupplementNumber,
            "printing-number" => Self::PrintingNumber,
            _ => Self::Custom(canonical),
        })
    }
}

impl PartialEq for NumberVariable {
    fn eq(&self, other: &Self) -> bool {
        self.as_key().as_ref() == other.as_key().as_ref()
    }
}

impl Eq for NumberVariable {}

impl Hash for NumberVariable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_key().as_ref().hash(state);
    }
}

impl Serialize for NumberVariable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_key().as_ref())
    }
}

impl<'de> Deserialize<'de> for NumberVariable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_key(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schema")]
impl JsonSchema for NumberVariable {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "NumberVariable".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "description": "Known number variable keyword or custom kebab-case identifier."
        })
    }
}

/// Number rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum NumberForm {
    #[default]
    Numeric,
    Ordinal,
    Roman,
}

fn normalize_kind_key(value: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut pending_dash = false;

    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !normalized.is_empty() {
                normalized.push('-');
            }
            normalized.push(ch.to_ascii_lowercase());
            pending_dash = false;
        } else if !normalized.is_empty() {
            pending_dash = true;
        }
    }

    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Label rendering forms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LabelForm {
    Long,
    #[default]
    Short,
    Symbol,
}

/// A simple variable component (DOI, ISBN, URL, etc.).
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateVariable {
    pub variable: SimpleVariable,
    #[serde(flatten)]
    pub rendering: Rendering,
    /// Structured link options (DOI, URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<crate::options::LinksConfig>,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// A supplementary standardized identifier component.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateIdentifier {
    /// Validated identifier name to render from `reference.identifiers`.
    pub identifier: crate::reference::IdentifierName,
    #[serde(flatten, default)]
    pub rendering: Rendering,
}

/// An MF2 message call inside a citation or bibliography template.
///
/// The style chooses the message ID and supplies structured argument sources;
/// the message body comes from the style or active locale.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateMessage {
    /// Locale message ID to evaluate, such as `pattern.accessed-date`.
    pub message: String,
    /// Optional term form used when `message` addresses a `term.*` locale item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TermForm>,
    /// Explicit grammatical gender override for term-backed message selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<GrammaticalGender>,
    /// Named argument sources pre-rendered before message evaluation.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub args: HashMap<String, MessageArgSource>,
    #[serde(flatten, default)]
    pub rendering: Rendering,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// A structured source for one named locale-message argument.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum MessageArgSource {
    /// A literal string argument.
    Literal { literal: String },
    /// The canonical reference-type key used for MF2 selection.
    ReferenceType {
        #[serde(rename = "reference-type")]
        reference_type: MessageReferenceTypeSource,
    },
    /// A carrier label derived from raw medium or online-resource metadata.
    Carrier { carrier: MessageCarrierSource },
    /// A rendered contributor argument.
    Contributor(Box<TemplateContributor>),
    /// A rendered date argument.
    Date(TemplateDate),
    /// A rendered group argument.
    Group(TemplateGroup),
    /// A rendered title argument.
    Title(TemplateTitle),
    /// A rendered number argument.
    Number(TemplateNumber),
    /// A rendered variable argument.
    Variable(TemplateVariable),
    /// A rendered locale term argument.
    Term(TemplateTerm),
}

impl MessageArgSource {
    /// Convert this argument source into a normal template component when it
    /// should be rendered through the standard component pipeline.
    #[must_use]
    pub fn as_template_component(&self) -> Option<TemplateComponent> {
        match self {
            Self::Literal { .. } | Self::ReferenceType { .. } | Self::Carrier { .. } => None,
            Self::Contributor(component) => {
                Some(TemplateComponent::Contributor(component.as_ref().clone()))
            }
            Self::Date(component) => Some(TemplateComponent::Date(component.clone())),
            Self::Group(component) => Some(TemplateComponent::Group(component.clone())),
            Self::Title(component) => Some(TemplateComponent::Title(component.clone())),
            Self::Number(component) => Some(TemplateComponent::Number(component.clone())),
            Self::Variable(component) => Some(TemplateComponent::Variable(component.clone())),
            Self::Term(component) => Some(TemplateComponent::Term(component.clone())),
        }
    }
}

/// Reference-type value exposed to a style-owned MF2 message.
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MessageReferenceTypeSource {
    /// Use the canonical Citum reference-type key.
    Key,
}

/// Carrier classification exposed to a style-owned MF2 message.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MessageCarrierSource {
    /// Value used when URL, DOI, or CSTR identifies an online resource.
    pub online: String,
    /// Value used when neither a raw medium nor online metadata is available.
    pub absent: String,
}

/// Simple string variables.
///
/// Use `variable:` for string passthrough fields, even when the field name is
/// also present in [`NumberVariable`]. For example, `variable: volume` keeps the
/// source value as plain text, while `number: volume` opts into numeric
/// formatting behavior.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SimpleVariable {
    #[default]
    Doi,
    Isbn,
    Issn,
    Url,
    Pmid,
    Pmcid,
    Abstract,
    Note,
    Annote,
    Keyword,
    Genre,
    RawGenre,
    Medium,
    RawMedium,
    Source,
    Status,
    Archive,
    ArchiveLocation,
    ArchiveName,
    ArchivePlace,
    ArchiveCollection,
    ArchiveCollectionId,
    ArchiveSeries,
    ArchiveBox,
    ArchiveFolder,
    ArchiveItem,
    ArchiveUrl,
    EprintId,
    EprintServer,
    EprintClass,
    Publisher,
    PublisherPlace,
    OriginalPublisher,
    OriginalPublisherPlace,
    EventTitle,
    EventPlace,
    Dimensions,
    References,
    Scale,
    Version,
    VolumeTitle,
    Locator,
    ContainerTitleShort,
    Authority,
    Code,
    Reporter,
    Page,
    Section,
    Volume,
    Number,
    DocketNumber,
    PatentNumber,
    StandardNumber,
    ReportNumber,
    AdsBibcode,
}

/// A term component for rendering locale-specific text.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateTerm {
    /// Which term to render.
    pub term: GeneralTerm,
    /// Form: long (default), short, or symbol.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<TermForm>,
    /// Explicit grammatical gender override for term selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<GrammaticalGender>,
    #[serde(flatten, default)]
    pub rendering: Rendering,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Where a [`TemplateTypeLabel`] resolves its text from.
///
/// `#[non_exhaustive]` with a single variant today: the label always
/// describes the reference's own type. Kept as an enum (rather than a bare
/// marker field) so a future label source can be added without a schema
/// break.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TypeLabelSource {
    /// Resolve the label from the reference's own type: prefer its
    /// `genre`/`medium`, falling back to a locale term keyed by `ref_type`.
    #[default]
    ReferenceType,
}

/// A localized label describing the reference's own type (e.g. "Dataset",
/// "Classical work"), resolved from `genre`/`medium` with a locale-term
/// fallback keyed by `ref_type`.
///
/// Emits only the resolved term text — wrap it in `wrap: brackets` (or any
/// other `Rendering` option) at the style level to match a particular
/// style's presentation, the same as any other component.
///
/// See `docs/specs/TYPE_CLASSIFICATION_CENTRALIZATION.md`.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateTypeLabel {
    /// The label's text source. Currently always `reference-type`.
    #[serde(rename = "type-label")]
    pub type_label: TypeLabelSource,
    #[serde(flatten, default)]
    pub rendering: Rendering,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// A group component for grouping multiple components with a delimiter,
/// matching CSL 1.0 `<group>` semantics.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateGroup {
    pub group: Vec<TemplateComponent>,
    /// Optional field-presence condition that controls whether the group renders.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_when: Option<TemplateGroupCondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<DelimiterPunctuation>,
    #[serde(flatten, default)]
    pub rendering: Rendering,

    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Field-presence condition for rendering a template group.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TemplateGroupCondition {
    /// Required field that must be present for the group to render.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_present: Option<TemplateConditionField>,
    /// Required field that must be absent for the group to render.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_absent: Option<TemplateConditionField>,
}

/// Reference fields that can be tested by a template group condition.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TemplateConditionField {
    /// The primary author contributor.
    Author,
    /// The editor contributor.
    Editor,
    /// The recipient contributor.
    Recipient,
    /// The translator contributor.
    Translator,
    /// The primary title.
    Title,
    /// The series or collection title.
    CollectionTitle,
    /// The issued date.
    Issued,
    /// The original publication date.
    OriginalPublished,
    /// The publisher name.
    Publisher,
    /// The original publisher name (e.g. a reprint's first publisher).
    OriginalPublisher,
    /// The original publisher place (e.g. a reprint's first place of publication).
    OriginalPublisherPlace,
    /// The original title (e.g. a translation's title in its source language).
    OriginalTitle,
    /// The DOI identifier.
    Doi,
    /// The reference genre or item type label.
    Genre,
    /// The archive or repository name.
    Archive,
    /// The archive shelfmark or repository location.
    ArchiveLocation,
    /// The volume number, or the issue number when volume is absent (i.e.
    /// "does this serial component have any volume/issue identifier at
    /// all?"). Used to detect online-first articles that have not yet been
    /// assigned to an issue, which need a full publication date instead of
    /// a bare year.
    VolumeOrIssue,
}

/// Literal text or an explicit semantic punctuation mark.
///
/// YAML strings are always literal. Semantic marks use the explicit mapping
/// form `{ mark: comma }`, so a string such as `comma` is never interpreted as
/// punctuation intent.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum DelimiterPunctuation {
    /// A semantic comma mark.
    #[default]
    Comma,
    /// A semantic semicolon mark.
    Semicolon,
    /// A semantic period mark.
    Period,
    /// A semantic colon mark.
    Colon,
    /// A semantic parentheses pair.
    Parentheses,
    /// A semantic brackets pair.
    Brackets,
    /// A literal ampersand delimiter retained for programmatic compatibility.
    Ampersand,
    /// A literal vertical-line delimiter retained for programmatic compatibility.
    VerticalLine,
    /// A literal slash delimiter retained for programmatic compatibility.
    Slash,
    /// A literal hyphen delimiter retained for programmatic compatibility.
    Hyphen,
    /// A literal space delimiter retained for programmatic compatibility.
    Space,
    /// An empty literal delimiter retained for programmatic compatibility.
    None,
    /// Literal punctuation or text (e.g., `": "` or `"comma"`).
    Custom(String),
}

#[cfg(feature = "schema")]
impl JsonSchema for DelimiterPunctuation {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DelimiterPunctuation".into()
    }

    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "oneOf": [
                {
                    "type": "string",
                    "description": "Literal punctuation or text."
                },
                {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["mark"],
                    "properties": {
                        "mark": {
                            "type": "string",
                            "enum": [
                                "comma",
                                "colon",
                                "semicolon",
                                "period",
                                "parentheses",
                                "brackets"
                            ]
                        }
                    }
                }
            ],
            "description": "Literal text or an explicit semantic punctuation mark."
        })
    }
}

impl Serialize for DelimiterPunctuation {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;

        let mark = match self {
            Self::Comma => Some("comma"),
            Self::Semicolon => Some("semicolon"),
            Self::Period => Some("period"),
            Self::Colon => Some("colon"),
            Self::Parentheses => Some("parentheses"),
            Self::Brackets => Some("brackets"),
            Self::Ampersand
            | Self::VerticalLine
            | Self::Slash
            | Self::Hyphen
            | Self::Space
            | Self::None
            | Self::Custom(_) => None,
        };

        if let Some(mark) = mark {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry("mark", mark)?;
            map.end()
        } else {
            serializer.serialize_str(self.as_default_str())
        }
    }
}

impl<'de> Deserialize<'de> for DelimiterPunctuation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct MarkReference {
            mark: String,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum LiteralOrMark {
            Literal(String),
            Mark(MarkReference),
        }

        match LiteralOrMark::deserialize(deserializer)? {
            LiteralOrMark::Literal(value) => Ok(Self::Custom(value)),
            LiteralOrMark::Mark(reference) => match reference.mark.as_str() {
                "comma" => Ok(Self::Comma),
                "colon" => Ok(Self::Colon),
                "semicolon" => Ok(Self::Semicolon),
                "period" => Ok(Self::Period),
                "parentheses" => Ok(Self::Parentheses),
                "brackets" => Ok(Self::Brackets),
                other => Err(serde::de::Error::unknown_variant(
                    other,
                    &[
                        "comma",
                        "colon",
                        "semicolon",
                        "period",
                        "parentheses",
                        "brackets",
                    ],
                )),
            },
        }
    }
}

impl DelimiterPunctuation {
    /// Return whether this value carries semantic punctuation intent rather
    /// than literal text.
    #[must_use]
    pub fn is_semantic(&self) -> bool {
        matches!(
            self,
            Self::Comma
                | Self::Semicolon
                | Self::Period
                | Self::Colon
                | Self::Parentheses
                | Self::Brackets
        )
    }

    /// Return the historical Latin/default literal form.
    #[must_use]
    pub fn as_default_str(&self) -> &str {
        match self {
            Self::Comma => ", ",
            Self::Semicolon => "; ",
            Self::Period => ". ",
            Self::Colon => ": ",
            Self::Parentheses => "()",
            Self::Brackets => "[]",
            Self::Ampersand => " & ",
            Self::VerticalLine => " | ",
            Self::Slash => "/",
            Self::Hyphen => "-",
            Self::Space => " ",
            Self::None => "",
            Self::Custom(value) => value,
        }
    }

    /// Convert this delimiter to a string with trailing space.
    ///
    /// Returns the punctuation followed by a space, except for Space (single space) and None (empty string).
    pub fn to_string_with_space(&self) -> String {
        self.as_default_str().to_string()
    }

    /// Parse a delimiter from a CSL 1.0 delimiter string.
    ///
    /// Handles common patterns like ", ", ": ", etc.
    /// Returns the Custom variant for unrecognized delimiters.
    pub fn from_csl_string(s: &str) -> Self {
        if s == " " {
            return Self::Space;
        }

        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            return Self::None;
        }

        match trimmed {
            "," => Self::Comma,
            ";" => Self::Semicolon,
            "." => Self::Period,
            ":" => Self::Colon,
            "&" => Self::Ampersand,
            "|" => Self::VerticalLine,
            "/" => Self::Slash,
            "-" => Self::Hyphen,
            _ => Self::Custom(s.to_string()),
        }
    }
}

impl std::ops::Deref for DelimiterPunctuation {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_default_str()
    }
}

impl std::fmt::Display for DelimiterPunctuation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_default_str())
    }
}

impl From<String> for DelimiterPunctuation {
    fn from(value: String) -> Self {
        Self::Custom(value)
    }
}

impl From<&str> for DelimiterPunctuation {
    fn from(value: &str) -> Self {
        Self::Custom(value.to_string())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;

    #[test]
    fn test_contributor_deserialization() {
        let yaml = r#"
contributor: author
form: long
"#;
        let comp: TemplateContributor = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comp.contributor, ContributorRole::Author);
        assert_eq!(comp.form, ContributorForm::Long);
    }

    #[test]
    fn test_contributor_name_order_family_first_except_last_deserialization() {
        let yaml = r#"
contributor: author
form: long
name-order: family-first-except-last
"#;
        let comp: TemplateContributor = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comp.name_order, Some(NameOrder::FamilyFirstExceptLast));
    }

    #[test]
    fn test_template_component_untagged() {
        let yaml = r#"
- contributor: author
  form: short
- date: issued
  form: year
- title: primary
"#;
        let components: Vec<TemplateComponent> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(components.len(), 3);

        match &components[0] {
            TemplateComponent::Contributor(c) => {
                assert_eq!(c.contributor, ContributorRole::Author);
            }
            _ => panic!("Expected Contributor"),
        }

        match &components[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(d.date, DateVariable::Issued);
            }
            _ => panic!("Expected Date"),
        }
    }

    #[test]
    fn test_flattened_rendering() {
        // Test that rendering options can be specified directly on the component
        let yaml = r#"
- title: parent-monograph
  prefix: "In "
  emph: true
- date: issued
  form: year
  wrap: parentheses
"#;
        let components: Vec<TemplateComponent> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(components.len(), 2);

        match &components[0] {
            TemplateComponent::Title(t) => {
                assert_eq!(t.rendering.prefix.as_deref(), Some("In "));
                assert_eq!(t.rendering.emph, Some(true));
            }
            _ => panic!("Expected Title"),
        }

        match &components[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(
                    d.rendering.wrap,
                    Some(WrapConfig {
                        punctuation: WrapPunctuation::Parentheses,
                        inner_prefix: None,
                        inner_suffix: None,
                    })
                );
            }
            _ => panic!("Expected Date"),
        }
    }

    #[test]
    fn test_number_variable_custom_normalizes_manual_construction() {
        let number = NumberVariable::Custom("Reel Label".to_string());

        assert_eq!(number.as_key(), "reel-label");
        assert_eq!(
            number,
            serde_yaml::from_str::<NumberVariable>("reel-label")
                .expect("custom number variable should parse")
        );
        assert_eq!(
            serde_json::to_string(&number).expect("custom number variable should serialize"),
            "\"reel-label\""
        );
    }

    #[test]
    fn test_contributor_with_wrap() {
        let yaml = r#"
contributor: publisher
form: short
wrap: parentheses
"#;
        let comp: TemplateContributor = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comp.contributor, ContributorRole::Publisher);
        assert_eq!(
            comp.rendering.wrap,
            Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            })
        );
    }

    #[test]
    fn test_variable_deserialization() {
        // Test that `variable: publisher` parses as Variable, not Number
        let yaml = "variable: publisher\n";
        let comp: TemplateComponent = serde_yaml::from_str(yaml).unwrap();
        match comp {
            TemplateComponent::Variable(v) => {
                assert_eq!(v.variable, SimpleVariable::Publisher);
            }
            _ => panic!("Expected Variable(Publisher), got {:?}", comp),
        }
    }

    #[test]
    fn test_message_component_deserialization() {
        let yaml = r#"
message: pattern.in-container
args:
  container:
    group:
    - title: parent-monograph
      emph: true
text-case: capitalize-first
"#;
        let comp: TemplateComponent = serde_yaml::from_str(yaml).unwrap();

        match comp {
            TemplateComponent::Message(message) => {
                assert_eq!(message.message, "pattern.in-container");
                assert!(matches!(
                    message.args.get("container"),
                    Some(MessageArgSource::Group(group)) if group.group.len() == 1
                        && matches!(
                            group.group.first(),
                            Some(TemplateComponent::Title(title))
                                if title.title == TitleType::ParentMonograph
                                    && title.rendering.emph == Some(true)
                        )
                ));
                assert_eq!(
                    message.rendering.text_case,
                    Some(crate::options::titles::TextCase::CapitalizeFirst)
                );
            }
            _ => panic!("Expected Message component, got {comp:?}"),
        }
    }

    #[test]
    fn test_term_backed_message_component_deserializes_form() {
        let yaml = r#"
message: term.in
form: long
suffix: ":"
"#;
        let comp: TemplateComponent = serde_yaml::from_str(yaml).unwrap();

        match comp {
            TemplateComponent::Message(message) => {
                assert_eq!(message.message, "term.in");
                assert_eq!(message.form, Some(TermForm::Long));
                assert_eq!(message.rendering.suffix.as_deref(), Some(":"));
            }
            _ => panic!("Expected Message component, got {comp:?}"),
        }
    }

    #[test]
    fn test_group_deserializes_term_backed_message_component_with_form() {
        let yaml = r#"
group:
- message: term.in
  form: long
  suffix: ":"
- title: parent-monograph
"#;
        let comp: TemplateComponent = serde_yaml::from_str(yaml).unwrap();

        match comp {
            TemplateComponent::Group(group) => {
                assert!(matches!(
                    group.group.first(),
                    Some(TemplateComponent::Message(message))
                        if message.message == "term.in"
                            && message.form == Some(TermForm::Long)
                            && message.rendering.suffix.as_deref() == Some(":")
                ));
            }
            _ => panic!("Expected Group component, got {comp:?}"),
        }
    }

    #[test]
    fn test_variable_array_parsing() {
        let yaml = r#"
- variable: doi
  prefix: "https://doi.org/"
- variable: publisher
"#;
        let comps: Vec<TemplateComponent> = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(comps.len(), 2);
        match &comps[0] {
            TemplateComponent::Variable(v) => assert_eq!(v.variable, SimpleVariable::Doi),
            _ => panic!("Expected Variable for doi, got {:?}", comps[0]),
        }
        match &comps[1] {
            TemplateComponent::Variable(v) => assert_eq!(v.variable, SimpleVariable::Publisher),
            _ => panic!("Expected Variable for publisher, got {:?}", comps[1]),
        }
    }

    #[test]
    fn test_type_selector_default_only_matches_default_context() {
        let selector = TypeSelector::Single("default".to_string());
        assert!(selector.matches("default"));
        assert!(!selector.matches("article-journal"));

        let mixed = TypeSelector::Multiple(vec!["default".to_string(), "chapter".to_string()]);
        assert!(mixed.matches("default"));
        assert!(mixed.matches("chapter"));
        assert!(!mixed.matches("book"));
    }

    #[test]
    fn test_template_component_selector_matches_nested_partial_group() {
        let component: TemplateComponent = serde_yaml::from_str(
            r#"
delimiter: ""
group:
- number: volume
  wrap:
    punctuation: brackets
- contributor: author
  form: long
"#,
        )
        .unwrap();
        let selector = TemplateComponentSelector {
            fields: BTreeMap::from([(
                "group".to_string(),
                serde_json::json!([
                    { "number": "volume" },
                    { "contributor": "author" }
                ]),
            )]),
        };

        assert!(selector.matches(&component));
    }

    #[test]
    fn test_delimiter_from_csl_string_normalizes_none_and_trimmed_values() {
        assert_eq!(
            DelimiterPunctuation::from_csl_string("none"),
            DelimiterPunctuation::None
        );
        assert_eq!(
            DelimiterPunctuation::from_csl_string(" none "),
            DelimiterPunctuation::None
        );
        assert_eq!(
            DelimiterPunctuation::from_csl_string(" "),
            DelimiterPunctuation::Space
        );
        assert_eq!(
            DelimiterPunctuation::from_csl_string(" : "),
            DelimiterPunctuation::Colon
        );
    }
}
