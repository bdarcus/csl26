/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Processing mode for citation/bibliography generation.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Processing {
    #[default]
    AuthorDate,
    Numeric,
    Note,
    Custom(ProcessingCustom),
}

/// Custom processing configuration.
#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct ProcessingCustom {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Sort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Group>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disambiguate: Option<Disambiguation>,
}

impl Processing {
    /// Get the effective configuration for this processing mode.
    pub fn config(&self) -> ProcessingCustom {
        match self {
            Processing::AuthorDate => ProcessingCustom {
                sort: Some(Sort {
                    shorten_names: false,
                    render_substitutions: false,
                    template: vec![
                        SortSpec {
                            key: SortKey::Author,
                            ascending: true,
                        },
                        SortSpec {
                            key: SortKey::Year,
                            ascending: true,
                        },
                    ],
                }),
                group: Some(Group {
                    template: vec![SortKey::Author, SortKey::Year],
                }),
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: true,
                    year_suffix: true,
                }),
            },
            Processing::Numeric => ProcessingCustom {
                sort: None,
                group: None,
                disambiguate: None,
            },
            Processing::Note => ProcessingCustom {
                sort: None,
                group: None,
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: false,
                    year_suffix: false,
                }),
            },
            Processing::Custom(custom) => custom.clone(),
        }
    }
}

/// Disambiguation settings.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Disambiguation {
    pub names: bool,
    #[serde(default)]
    pub add_givenname: bool,
    pub year_suffix: bool,
}

impl Default for Disambiguation {
    fn default() -> Self {
        Self {
            names: true,
            add_givenname: false,
            year_suffix: false,
        }
    }
}

/// Sorting configuration.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Sort {
    /// Shorten name lists for sorting the same as for display.
    #[serde(default)]
    pub shorten_names: bool,
    /// Use same substitutions for sorting as for rendering.
    #[serde(default)]
    pub render_substitutions: bool,
    /// Sort keys in order.
    pub template: Vec<SortSpec>,
}

/// A single sort specification.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SortSpec {
    pub key: SortKey,
    #[serde(default = "default_ascending")]
    pub ascending: bool,
}

fn default_ascending() -> bool {
    true
}

/// Available sort keys.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SortKey {
    #[default]
    Author,
    Year,
    Title,
    /// Sort by citation order (for numeric styles).
    CitationNumber,
}

/// Grouping configuration for bibliography.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Group {
    pub template: Vec<SortKey>,
}
