/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Substitution rules for missing author data.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum SubstituteConfig {
    /// A named preset (e.g., "standard", "editor-first", "title-first").
    Preset(crate::presets::SubstitutePreset),
    /// Explicit substitution configuration.
    Explicit(Substitute),
}

impl Default for SubstituteConfig {
    fn default() -> Self {
        SubstituteConfig::Explicit(Substitute::default())
    }
}

impl SubstituteConfig {
    /// Resolve this config to a concrete `Substitute`.
    pub fn resolve(&self) -> Substitute {
        match self {
            SubstituteConfig::Preset(preset) => preset.config(),
            SubstituteConfig::Explicit(config) => config.clone(),
        }
    }
}

/// Explicit substitution configuration.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Substitute {
    /// Form to use for contributor roles when substituting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributor_role_form: Option<String>,
    /// Ordered list of fields to try as substitutes.
    #[serde(default)]
    pub template: Vec<SubstituteKey>,
    /// Type-specific substitution overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub overrides: HashMap<String, Vec<SubstituteKey>>,
}

impl Default for Substitute {
    fn default() -> Self {
        Self {
            contributor_role_form: None,
            template: vec![
                SubstituteKey::Editor,
                SubstituteKey::Title,
                SubstituteKey::Translator,
            ],
            overrides: HashMap::new(),
        }
    }
}

/// Fields that can be used as author substitutes.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum SubstituteKey {
    Editor,
    Title,
    Translator,
}
