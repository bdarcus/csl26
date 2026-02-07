/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contributor formatting configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ContributorConfig {
    /// When to display a contributor's name in sort order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_as_sort: Option<DisplayAsSort>,
    /// String to append after initialized given names (e.g., ". " for "J. Smith").
    /// If None, full given names are used (e.g., "John Smith").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with: Option<String>,
    /// Whether to include a hyphen when initializing names (e.g., "J.-P. Sartre").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initialize_with_hyphen: Option<bool>,
    /// Shorten the list of contributors (et al. handling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shorten: Option<ShortenListOptions>,
    /// The delimiter between contributors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    /// Conjunction between last two contributors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub and: Option<AndOptions>,
    /// When to include delimiter before the last contributor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_last: Option<DelimiterPrecedesLast>,
    /// When to include delimiter before "et al.".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter_precedes_et_al: Option<DelimiterPrecedesLast>,
    /// When and how to display contributor roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<RoleOptions>,
    /// Global format for editor labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_label_format: Option<EditorLabelFormat>,
    /// Handling of non-dropping particles (e.g., "van" in "van Gogh").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub demote_non_dropping_particle: Option<DemoteNonDroppingParticle>,
    /// Delimiter between family and given name when inverted (default: ", ").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_separator: Option<String>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Format for editor labels.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EditorLabelFormat {
    /// "edited by John Doe"
    VerbPrefix,
    /// "John Doe (Ed.)"
    ShortSuffix,
    /// "John Doe, editor"
    LongSuffix,
}

/// Options for demoting non-dropping particles.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DemoteNonDroppingParticle {
    Never,
    SortOnly,
    #[default]
    DisplayAndSort,
}

/// When to display names in sort order (family-first).
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DisplayAsSort {
    All,
    First,
    #[default]
    None,
}

/// Conjunction options between contributors.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum AndOptions {
    Text,
    Symbol,
    #[default]
    None,
}

/// Role display options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RoleOptions {
    /// Contributor roles for which to omit the role description.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub omit: Vec<String>,
    /// Global role label form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// Global prefix for role labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Global suffix for role labels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Formatting for specific roles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<HashMap<String, RoleRendering>>,
}

/// Rendering options for contributor roles.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RoleRendering {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_order: Option<crate::template::NameOrder>,
}

impl RoleRendering {
    pub fn to_rendering(&self) -> crate::template::Rendering {
        crate::template::Rendering {
            emph: self.emph,
            strong: self.strong,
            small_caps: self.small_caps,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            ..Default::default()
        }
    }
}

/// When to use delimiter before last contributor.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DelimiterPrecedesLast {
    AfterInvertedName,
    Always,
    Never,
    #[default]
    Contextual,
}

/// Et al. / list shortening options.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ShortenListOptions {
    /// Minimum number of names to trigger shortening.
    pub min: u8,
    /// Number of names to show when shortened.
    pub use_first: u8,
    /// Number of names to show after the ellipsis (et-al-use-last).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_last: Option<u8>,
    /// How to render "and others".
    #[serde(default)]
    pub and_others: AndOtherOptions,
    /// When to use delimiter before last name.
    #[serde(default)]
    pub delimiter_precedes_last: DelimiterPrecedesLast,
}

impl Default for ShortenListOptions {
    fn default() -> Self {
        Self {
            min: 4,
            use_first: 1,
            use_last: None,
            and_others: AndOtherOptions::default(),
            delimiter_precedes_last: DelimiterPrecedesLast::default(),
        }
    }
}

/// How to render "and others" / et al.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AndOtherOptions {
    #[default]
    EtAl,
    Text,
}
