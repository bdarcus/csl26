/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Title config: either a preset name or explicit configuration.
///
/// Allows styles to write `titles: apa` as shorthand, or provide
/// full explicit configuration with field-level overrides.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TitlesConfigEntry {
    /// A named preset (e.g., "apa", "chicago", "humanities", "scientific").
    Preset(crate::presets::TitlePreset),
    /// Explicit title configuration.
    Explicit(Box<TitlesConfig>),
}

impl Default for TitlesConfigEntry {
    fn default() -> Self {
        TitlesConfigEntry::Explicit(Box::default())
    }
}

impl TitlesConfigEntry {
    /// Resolve this entry to a concrete `TitlesConfig`.
    pub fn resolve(&self) -> TitlesConfig {
        match self {
            TitlesConfigEntry::Preset(preset) => preset.config(),
            TitlesConfigEntry::Explicit(config) => *config.clone(),
        }
    }
}

/// Title formatting configuration by title type.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TitlesConfig {
    /// Mapping of reference types to title categories.
    /// Category keys: monograph, periodical, component.
    /// Example: { "thesis": "monograph", "article-journal": "periodical" }
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub type_mapping: HashMap<String, String>,
    /// Formatting for component titles (articles, chapters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<TitleRendering>,
    /// Formatting for monograph titles (books).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monograph: Option<TitleRendering>,
    /// Formatting for monograph containers (book containing chapters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_monograph: Option<TitleRendering>,
    /// Formatting for periodical titles (journals, magazines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periodical: Option<TitleRendering>,
    /// Formatting for serial titles (series).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<TitleRendering>,
    /// Default formatting for all titles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<TitleRendering>,
    /// Unknown fields captured for forward compatibility.
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_json::Value>,
}

/// Rendering options for titles.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TitleRendering {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_caps: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

impl TitleRendering {
    pub fn to_rendering(&self) -> crate::template::Rendering {
        crate::template::Rendering {
            emph: self.emph,
            quote: self.quote,
            strong: self.strong,
            small_caps: self.small_caps,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
            ..Default::default()
        }
    }
}
