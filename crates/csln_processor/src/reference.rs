/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Reference types for the CSLN processor.
//!
//! This module provides a CSL-JSON compatible reference model that can be
//! used with the CSLN processor. It's designed to be compatible with existing
//! CSL-JSON data while supporting CSLN's enhanced features.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A bibliographic reference item.
/// This is compatible with CSL-JSON format.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Reference {
    /// Unique identifier for the reference.
    pub id: String,
    /// The type of reference (book, article-journal, etc.)
    #[serde(rename = "type")]
    pub ref_type: String,
    /// Authors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Vec<Name>>,
    /// Editors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor: Option<Vec<Name>>,
    /// Translators
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translator: Option<Vec<Name>>,
    /// Primary title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Container title (journal, book, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_title: Option<String>,
    /// Collection title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_title: Option<String>,
    /// Issued date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued: Option<DateVariable>,
    /// Accessed date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed: Option<DateVariable>,
    /// Volume
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<StringOrNumber>,
    /// Issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<StringOrNumber>,
    /// Page or page range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// Edition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<StringOrNumber>,
    /// DOI
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "DOI")]
    pub doi: Option<String>,
    /// URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "URL")]
    pub url: Option<String>,
    /// ISBN
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ISBN")]
    pub isbn: Option<String>,
    /// ISSN
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ISSN")]
    pub issn: Option<String>,
    /// Publisher
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    /// Publisher place
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_place: Option<String>,
    /// Genre
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Abstract
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,
    /// Note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Number of pages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_pages: Option<StringOrNumber>,
    /// Number of volumes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_of_volumes: Option<StringOrNumber>,
    /// Additional fields not explicitly modeled
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A name (person or organization).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Name {
    pub family: Option<String>,
    pub given: Option<String>,
    /// Literal name (for organizations or single-field names)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub literal: Option<String>,
    /// Name suffix (Jr., III, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    /// Dropping particle (de, van, etc. that sorts with given name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dropping_particle: Option<String>,
    /// Non-dropping particle (de, van, etc. that sorts with family name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub non_dropping_particle: Option<String>,
}

impl Name {
    /// Create a new structured name.
    pub fn new(family: &str, given: &str) -> Self {
        Self {
            family: Some(family.to_string()),
            given: Some(given.to_string()),
            ..Default::default()
        }
    }

    /// Create a literal name (organization or single-field).
    pub fn literal(name: &str) -> Self {
        Self {
            literal: Some(name.to_string()),
            ..Default::default()
        }
    }

    /// Get the family name or literal.
    pub fn family_or_literal(&self) -> &str {
        self.family
            .as_deref()
            .or(self.literal.as_deref())
            .unwrap_or("")
    }
}

/// A date variable (CSL-JSON format).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DateVariable {
    /// Date parts: [[year, month, day], [end_year, end_month, end_day]]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_parts: Option<Vec<Vec<i32>>>,
    /// Literal date string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub literal: Option<String>,
    /// Raw date string (for parsing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// Season (1-4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<i32>,
    /// Circa (approximate date)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circa: Option<bool>,
}

impl DateVariable {
    /// Create a date with year only.
    pub fn year(year: i32) -> Self {
        Self {
            date_parts: Some(vec![vec![year]]),
            ..Default::default()
        }
    }

    /// Create a date with year and month.
    pub fn year_month(year: i32, month: i32) -> Self {
        Self {
            date_parts: Some(vec![vec![year, month]]),
            ..Default::default()
        }
    }

    /// Create a full date.
    pub fn full(year: i32, month: i32, day: i32) -> Self {
        Self {
            date_parts: Some(vec![vec![year, month, day]]),
            ..Default::default()
        }
    }

    /// Get the year from the first date part.
    pub fn year_value(&self) -> Option<i32> {
        self.date_parts
            .as_ref()
            .and_then(|parts| parts.first())
            .and_then(|date| date.first())
            .copied()
    }

    /// Get the month from the first date part.
    pub fn month_value(&self) -> Option<i32> {
        self.date_parts
            .as_ref()
            .and_then(|parts| parts.first())
            .and_then(|date| date.get(1))
            .copied()
    }

    /// Get the day from the first date part.
    pub fn day_value(&self) -> Option<i32> {
        self.date_parts
            .as_ref()
            .and_then(|parts| parts.first())
            .and_then(|date| date.get(2))
            .copied()
    }
}

/// A value that can be either a string or number.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrNumber {
    String(String),
    Number(i64),
}

impl std::fmt::Display for StringOrNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Number(n) => write!(f, "{}", n),
        }
    }
}

/// A bibliography is a collection of references keyed by ID.
pub type Bibliography = HashMap<String, Reference>;

/// Input citations for processing.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Citation {
    /// The citation ID (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The citation items.
    pub items: Vec<CitationItem>,
}

/// A single citation item referencing a bibliography entry.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CitationItem {
    /// The reference ID (citekey).
    pub id: String,
    /// Locator type (page, chapter, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Locator value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    /// Prefix text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    /// Suffix text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csl_json() {
        let json = r#"{
            "id": "kuhn1962",
            "type": "book",
            "author": [{"family": "Kuhn", "given": "Thomas S."}],
            "title": "The Structure of Scientific Revolutions",
            "issued": {"date-parts": [[1962]]},
            "publisher": "University of Chicago Press",
            "publisher-place": "Chicago"
        }"#;

        let reference: Reference = serde_json::from_str(json).unwrap();
        assert_eq!(reference.id, "kuhn1962");
        assert_eq!(reference.ref_type, "book");
        assert_eq!(
            reference.author.as_ref().unwrap()[0].family,
            Some("Kuhn".to_string())
        );
        assert_eq!(reference.issued.as_ref().unwrap().year_value(), Some(1962));
    }

    #[test]
    fn test_date_variable() {
        let date = DateVariable::year(2023);
        assert_eq!(date.year_value(), Some(2023));
        assert_eq!(date.month_value(), None);

        let date = DateVariable::year_month(2023, 6);
        assert_eq!(date.year_value(), Some(2023));
        assert_eq!(date.month_value(), Some(6));
    }
}
