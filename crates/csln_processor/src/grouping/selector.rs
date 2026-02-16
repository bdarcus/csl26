/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Selector evaluation for bibliography grouping.
//!
//! This module implements predicate logic for matching references against
//! group selectors. Supports type-based, field-based, and citation status
//! filtering with negation for fallback groups.

use csln_core::grouping::{CitedStatus, FieldMatcher, GroupSelector, TypeSelector};
use std::collections::HashSet;

use crate::reference::Reference;

/// Evaluates group selectors to match references.
///
/// The evaluator holds the set of cited and silent reference IDs to support
/// citation status filtering.
pub struct SelectorEvaluator<'a> {
    /// IDs of references cited visibly in the document.
    cited_ids: &'a HashSet<String>,
    /// IDs of references cited silently (nocite).
    silent_ids: &'a HashSet<String>,
}

impl<'a> SelectorEvaluator<'a> {
    /// Create a new selector evaluator.
    ///
    /// # Arguments
    ///
    /// * `cited_ids` - Set of reference IDs cited visibly
    /// * `silent_ids` - Set of reference IDs cited silently (nocite)
    pub fn new(cited_ids: &'a HashSet<String>, silent_ids: &'a HashSet<String>) -> Self {
        Self {
            cited_ids,
            silent_ids,
        }
    }

    /// Evaluate a selector against a reference.
    ///
    /// Returns `true` if the reference matches the selector predicate.
    /// All specified conditions must match (AND logic).
    ///
    /// # Arguments
    ///
    /// * `reference` - The reference to test
    /// * `selector` - The selector predicate
    pub fn matches(&self, reference: &Reference, selector: &GroupSelector) -> bool {
        let mut result = true;

        // Type filtering
        if let Some(type_sel) = &selector.ref_type {
            result &= self.matches_type(reference, type_sel);
        }

        // Citation status filtering
        if let Some(cited) = &selector.cited {
            result &= self.matches_cited_status(reference, cited);
        }

        // Field filtering
        if let Some(fields) = &selector.field {
            for (field_name, matcher) in fields {
                result &= self.matches_field(reference, field_name, matcher);
            }
        }

        // Negation
        if let Some(not_sel) = &selector.not {
            result &= !self.matches(reference, not_sel);
        }

        result
    }

    /// Match reference type.
    fn matches_type(&self, reference: &Reference, type_sel: &TypeSelector) -> bool {
        let ref_type = reference.ref_type();
        match type_sel {
            TypeSelector::Single(t) => ref_type == t.as_str(),
            TypeSelector::Multiple(types) => types.iter().any(|t| ref_type == t.as_str()),
        }
    }

    /// Match citation status.
    fn matches_cited_status(&self, reference: &Reference, cited: &CitedStatus) -> bool {
        let id = reference.id().unwrap_or_default();
        match cited {
            CitedStatus::Visible => self.cited_ids.contains(&id),
            CitedStatus::Silent => self.silent_ids.contains(&id),
            CitedStatus::Any => true,
        }
    }

    /// Match field value.
    ///
    /// Currently supports matching against the `language` field.
    /// Future: extend to support arbitrary custom metadata fields.
    fn matches_field(
        &self,
        reference: &Reference,
        field_name: &str,
        matcher: &FieldMatcher,
    ) -> bool {
        match field_name {
            "language" => {
                let lang = reference.language().unwrap_or_default();
                self.matches_field_value(&lang, matcher)
            }
            // Future: support for keywords, custom metadata
            _ => false,
        }
    }

    /// Match a field value against a matcher.
    fn matches_field_value(&self, value: &str, matcher: &FieldMatcher) -> bool {
        match matcher {
            FieldMatcher::Exact(expected) => value == expected,
            FieldMatcher::Multiple(values) => values.iter().any(|v| value == v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_reference(id: &str, ref_type: &str, language: Option<&str>) -> Reference {
        // Create a minimal CSL JSON reference
        // CSL JSON spec uses hyphens in type names (article-journal, article-magazine, etc.)
        let json = serde_json::json!({
            "id": id,
            "type": ref_type,
            "language": language.unwrap_or(""),
            "title": "Test Title",
            "container-title": "Test Container",  // Needed for article/chapter types
        });
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(json).unwrap();
        legacy.into()
    }

    #[test]
    fn test_type_selector_single() {
        let cited_ids = HashSet::new();
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let selector = GroupSelector {
            ref_type: Some(TypeSelector::Single("article-journal".to_string())),
            cited: None,
            field: None,
            not: None,
        };

        let article = make_reference("r1", "article-journal", None);
        let book = make_reference("r2", "book", None);

        assert!(evaluator.matches(&article, &selector));
        assert!(!evaluator.matches(&book, &selector));
    }

    #[test]
    fn test_type_selector_multiple() {
        let cited_ids = HashSet::new();
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let selector = GroupSelector {
            ref_type: Some(TypeSelector::Multiple(vec![
                "article-journal".to_string(),
                "article-magazine".to_string(),
                "article-newspaper".to_string(),
            ])),
            cited: None,
            field: None,
            not: None,
        };

        let journal = make_reference("r1", "article-journal", None);
        let magazine = make_reference("r2", "article-magazine", None);
        let book = make_reference("r3", "book", None);

        assert!(evaluator.matches(&journal, &selector));
        assert!(evaluator.matches(&magazine, &selector));
        assert!(!evaluator.matches(&book, &selector));
    }

    #[test]
    fn test_cited_status_visible() {
        let mut cited_ids = HashSet::new();
        cited_ids.insert("r1".to_string());
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let selector = GroupSelector {
            ref_type: None,
            cited: Some(CitedStatus::Visible),
            field: None,
            not: None,
        };

        let cited = make_reference("r1", "book", None);
        let uncited = make_reference("r2", "book", None);

        assert!(evaluator.matches(&cited, &selector));
        assert!(!evaluator.matches(&uncited, &selector));
    }

    #[test]
    fn test_cited_status_silent() {
        let cited_ids = HashSet::new();
        let mut silent_ids = HashSet::new();
        silent_ids.insert("r2".to_string());
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let selector = GroupSelector {
            ref_type: None,
            cited: Some(CitedStatus::Silent),
            field: None,
            not: None,
        };

        let cited = make_reference("r1", "book", None);
        let silent = make_reference("r2", "book", None);

        assert!(!evaluator.matches(&cited, &selector));
        assert!(evaluator.matches(&silent, &selector));
    }

    #[test]
    fn test_field_language_exact() {
        let cited_ids = HashSet::new();
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "language".to_string(),
            FieldMatcher::Exact("vi".to_string()),
        );

        let selector = GroupSelector {
            ref_type: None,
            cited: None,
            field: Some(fields),
            not: None,
        };

        let vietnamese = make_reference("r1", "book", Some("vi"));
        let english = make_reference("r2", "book", Some("en"));

        assert!(evaluator.matches(&vietnamese, &selector));
        assert!(!evaluator.matches(&english, &selector));
    }

    #[test]
    fn test_field_language_multiple() {
        let cited_ids = HashSet::new();
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "language".to_string(),
            FieldMatcher::Multiple(vec!["vi".to_string(), "th".to_string()]),
        );

        let selector = GroupSelector {
            ref_type: None,
            cited: None,
            field: Some(fields),
            not: None,
        };

        let vietnamese = make_reference("r1", "book", Some("vi"));
        let thai = make_reference("r2", "book", Some("th"));
        let english = make_reference("r3", "book", Some("en"));

        assert!(evaluator.matches(&vietnamese, &selector));
        assert!(evaluator.matches(&thai, &selector));
        assert!(!evaluator.matches(&english, &selector));
    }

    #[test]
    fn test_negation() {
        let cited_ids = HashSet::new();
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "language".to_string(),
            FieldMatcher::Exact("vi".to_string()),
        );

        let selector = GroupSelector {
            ref_type: None,
            cited: None,
            field: None,
            not: Some(Box::new(GroupSelector {
                ref_type: None,
                cited: None,
                field: Some(fields),
                not: None,
            })),
        };

        let vietnamese = make_reference("r1", "book", Some("vi"));
        let english = make_reference("r2", "book", Some("en"));

        // NOT vietnamese = false (should not match)
        assert!(!evaluator.matches(&vietnamese, &selector));
        // NOT vietnamese = true (should match)
        assert!(evaluator.matches(&english, &selector));
    }

    #[test]
    fn test_combined_and_logic() {
        let mut cited_ids = HashSet::new();
        cited_ids.insert("r1".to_string());
        let silent_ids = HashSet::new();
        let evaluator = SelectorEvaluator::new(&cited_ids, &silent_ids);

        let mut fields = std::collections::HashMap::new();
        fields.insert(
            "language".to_string(),
            FieldMatcher::Exact("vi".to_string()),
        );

        let selector = GroupSelector {
            ref_type: Some(TypeSelector::Single("book".to_string())),
            cited: Some(CitedStatus::Visible),
            field: Some(fields),
            not: None,
        };

        let match_all = make_reference("r1", "book", Some("vi"));
        let wrong_type = make_reference("r1", "article", Some("vi"));
        let wrong_lang = make_reference("r1", "book", Some("en"));
        let uncited = make_reference("r2", "book", Some("vi"));

        // All conditions match
        assert!(evaluator.matches(&match_all, &selector));
        // Wrong type
        assert!(!evaluator.matches(&wrong_type, &selector));
        // Wrong language
        assert!(!evaluator.matches(&wrong_lang, &selector));
        // Not cited
        assert!(!evaluator.matches(&uncited, &selector));
    }
}
