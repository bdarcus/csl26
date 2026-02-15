/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bibliography-specific configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct BibliographyConfig {
    /// String to substitute for repeating authors (e.g., "———").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_author_substitute: Option<String>,
    /// Rule for when to apply the substitute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_author_substitute_rule: Option<SubsequentAuthorSubstituteRule>,
    /// Whether to use a hanging indent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hanging_indent: Option<bool>,
    /// Suffix appended to each bibliography entry (e.g., ".").
    /// Extracted from CSL 1.0 `<layout suffix=".">` attribute.
    /// If None, a trailing period is added by default unless entry ends with DOI/URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_suffix: Option<String>,
    /// Separator between bibliography components (e.g., ". " for Chicago/APA, ", " for Elsevier).
    /// Extracted from CSL 1.0 group delimiter attribute.
    /// Defaults to ". " if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
    /// Whether to suppress the trailing period after URLs/DOIs.
    /// Default behavior is to add a period (Chicago, MLA style).
    /// Set to true to suppress the period (APA 7th, Bluebook style).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub suppress_period_after_url: bool,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Rules for subsequent author substitution.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SubsequentAuthorSubstituteRule {
    /// Substitute only if ALL authors match.
    #[default]
    CompleteAll,
    /// Substitute each matching name individually.
    CompleteEach,
    /// Substitute each matching name until the first mismatch.
    PartialEach,
    /// Substitute only the first name if it matches.
    PartialFirst,
}
