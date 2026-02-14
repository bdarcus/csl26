/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw locale format for YAML parsing.
/// This is a simpler format that uses string keys for terms.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RawLocale {
    /// The locale identifier (e.g., "en-US", "de-DE").
    pub locale: String,
    /// Date-related terms.
    #[serde(default)]
    pub dates: RawDateTerms,
    /// Role terms keyed by role name.
    #[serde(default)]
    pub roles: HashMap<String, RawRoleTerm>,
    /// General terms keyed by term name.
    #[serde(default)]
    pub terms: HashMap<String, RawTermValue>,
}

/// Raw date terms for YAML parsing.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RawDateTerms {
    #[serde(default)]
    pub months: RawMonthNames,
    #[serde(default)]
    pub seasons: Vec<String>,
    #[serde(default)]
    pub uncertainty_term: Option<String>,
    #[serde(default)]
    pub open_ended_term: Option<String>,
}

/// Raw month names for YAML parsing.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct RawMonthNames {
    #[serde(default)]
    pub long: Vec<String>,
    #[serde(default)]
    pub short: Vec<String>,
}

/// Raw role term with form-keyed values.
#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema)]
pub struct RawRoleTerm {
    #[serde(default)]
    pub long: Option<RawTermValue>,
    #[serde(default)]
    pub short: Option<RawTermValue>,
    #[serde(default)]
    pub verb: Option<RawTermValue>,
    #[serde(default, rename = "verb-short")]
    pub verb_short: Option<RawTermValue>,
}

/// A term value that can be a simple string or have singular/plural forms.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum RawTermValue {
    /// Simple string value.
    Simple(String),
    /// Form-keyed value (for terms with long/short forms).
    Forms(HashMap<String, RawTermValue>),
    /// Singular/plural forms.
    SingularPlural { singular: String, plural: String },
}

impl Default for RawTermValue {
    fn default() -> Self {
        RawTermValue::Simple(String::new())
    }
}

impl RawTermValue {
    /// Get the simple string value.
    pub fn as_string(&self) -> Option<&str> {
        match self {
            RawTermValue::Simple(s) => Some(s),
            _ => None,
        }
    }
}
