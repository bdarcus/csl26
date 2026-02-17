/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::localization::MonthFormat;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Date config: either a preset name or explicit configuration.
///
/// Allows styles to write `dates: long` as shorthand, or provide
/// full explicit configuration with field-level overrides.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum DateConfigEntry {
    /// A named preset (e.g., "long", "short", "numeric", "iso").
    Preset(crate::presets::DatePreset),
    /// Explicit date configuration.
    Explicit(DateConfig),
}

impl Default for DateConfigEntry {
    fn default() -> Self {
        DateConfigEntry::Explicit(DateConfig::default())
    }
}

impl DateConfigEntry {
    /// Resolve this entry to a concrete `DateConfig`.
    pub fn resolve(&self) -> DateConfig {
        match self {
            DateConfigEntry::Preset(preset) => preset.config(),
            DateConfigEntry::Explicit(config) => config.clone(),
        }
    }
}

/// Date formatting configuration.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DateConfig {
    pub month: MonthFormat,
    /// Marker for uncertain dates (e.g., "?" or "uncertain"). None suppresses display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uncertainty_marker: Option<String>,
    /// Marker for approximate dates (e.g., "ca. " or "~"). None suppresses display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximation_marker: Option<String>,
    /// Delimiter for date ranges (default: en-dash "–").
    #[serde(default = "default_range_delimiter")]
    pub range_delimiter: String,
    /// Marker for open-ended ranges (e.g., "–present"). None uses locale default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_range_marker: Option<String>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

fn default_range_delimiter() -> String {
    "–".to_string() // U+2013 en-dash
}

impl Default for DateConfig {
    fn default() -> Self {
        Self {
            month: MonthFormat::Long,
            uncertainty_marker: Some("?".to_string()),
            approximation_marker: Some("ca. ".to_string()),
            range_delimiter: default_range_delimiter(),
            open_range_marker: None,
            custom: None,
        }
    }
}
