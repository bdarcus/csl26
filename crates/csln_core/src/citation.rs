/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Citation input model for the CSLN processor.
//!
//! This module defines the structures for representing citations as input
//! to the processor. Citations reference entries in the bibliography and
//! can include locators, prefixes, suffixes, and mode information.

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A list of citations to process.
pub type Citations = Vec<Citation>;

/// Citation mode for author-date styles.
///
/// Determines how the author name is rendered relative to the citation.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CitationMode {
    /// Author inline in text: "Smith (2020) argues..."
    /// Also known as "narrative" or "in-text" citations.
    Integral,
    /// Author in parentheses: "(Smith, 2020)"
    /// The default mode for most citations.
    #[default]
    NonIntegral,
}

/// A citation containing one or more references.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Citation {
    /// The citation ID (optional, for tracking).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Note number for footnote/endnote styles.
    /// Assigned by the document processor, not the citation processor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_number: Option<u32>,
    /// Citation mode: integral (narrative) vs non-integral (parenthetical).
    /// Only relevant for author-date styles.
    #[serde(default, skip_serializing_if = "is_default_mode")]
    pub mode: CitationMode,
    /// Suppress the author name across all items in this citation.
    /// Used when the author is already named in the prose: "Smith argues (2020)".
    /// Applies uniformly to all items — per-item suppression is not supported
    /// because mixed-visibility citations are typographically incoherent.
    #[serde(default, skip_serializing_if = "is_false")]
    pub suppress_author: bool,
    /// Prefix text before all citation items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Suffix text after all citation items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// The citation items (references being cited).
    pub items: Vec<CitationItem>,
}

impl Citation {
    /// Create a simple citation for a single ID.
    pub fn simple(id: &str) -> Self {
        Self {
            items: vec![CitationItem {
                id: id.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }
}

/// Helper for skip_serializing_if on mode field.
fn is_default_mode(mode: &CitationMode) -> bool {
    *mode == CitationMode::NonIntegral
}

/// Helper for skip_serializing_if on bool fields that default to false.
fn is_false(b: &bool) -> bool {
    !b
}

/// Locator types for pinpoint citations.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LocatorType {
    Book,
    Chapter,
    Column,
    Figure,
    Folio,
    Line,
    Note,
    Number,
    Opus,
    #[default]
    Page,
    Paragraph,
    Part,
    Section,
    SubVerbo,
    Verse,
    Volume,
    Issue,
}

/// A single citation item referencing a bibliography entry.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct CitationItem {
    /// The reference ID (citekey).
    pub id: String,
    /// Locator type (page, chapter, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<LocatorType>,
    /// Locator value (e.g., "42-45" for pages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    /// Prefix text before this item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Suffix text after this item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_citation_deserialization() {
        let json = r#"
        {
            "items": [
                {
                    "id": "kuhn1962"
                }
            ],
            "mode": "integral"
        }
        "#;
        let citation: Citation = serde_json::from_str(json).unwrap();
        assert_eq!(citation.items.len(), 1);
        assert_eq!(citation.items[0].id, "kuhn1962");
        assert_eq!(citation.mode, CitationMode::Integral);
    }

    #[test]
    fn test_citation_item_with_locator() {
        let json = r#"
        {
            "id": "kuhn1962",
            "label": "page",
            "locator": "42-45"
        }
        "#;
        let item: CitationItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "kuhn1962");
        assert_eq!(item.label, Some(LocatorType::Page));
        assert_eq!(item.locator, Some("42-45".to_string()));
    }
}
