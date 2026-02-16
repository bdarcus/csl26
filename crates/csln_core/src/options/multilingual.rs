/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Multilingual rendering options.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct MultilingualConfig {
    /// Preferred rendering mode for titles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_mode: Option<MultilingualMode>,
    /// Preferred rendering mode for names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_mode: Option<MultilingualMode>,
    /// Preferred script for transliterations (e.g., "Latn").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_script: Option<String>,
    /// Script-specific behavior configuration.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub scripts: HashMap<String, ScriptConfig>,
}

/// Rendering modes for multilingual content.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MultilingualMode {
    /// Use original script.
    Primary,
    /// Use transliteration.
    Transliterated,
    /// Use translation matching style locale.
    Translated,
    /// Combine multiple views (e.g., "transliterated [translated]").
    /// For now, this is a placeholder for more complex formatting strings.
    Combined,
}

/// Configuration for specific scripts.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct ScriptConfig {
    /// Whether to use native ordering for this script (e.g., FamilyGiven for CJK).
    #[serde(default)]
    pub use_native_ordering: bool,
    /// Custom delimiter for this script.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
}
