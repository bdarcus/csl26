#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Template;

/// A bibliography group with selector, optional heading, and per-group sorting.
///
/// Groups allow styles to divide bibliographies into labeled sections with
/// distinct sorting rules. Items match the first group whose selector evaluates
/// to true (first-match semantics).
///
/// # Examples
///
/// ```yaml
/// groups:
///   - id: vietnamese
///     heading: "Tài liệu tiếng Việt"
///     selector:
///       field:
///         language: vi
///     sort:
///       template:
///         - key: author
///           sort-order: given-family
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct BibliographyGroup {
    /// Unique identifier for this group.
    pub id: String,

    /// Optional heading to display above this group.
    /// Omit for no heading (e.g., fallback group).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,

    /// Selector predicate to match references.
    pub selector: GroupSelector,

    /// Optional per-group sorting specification.
    /// Falls back to global bibliography sort if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<GroupSort>,

    /// Optional per-group template override.
    /// Falls back to global bibliography template if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
}

/// Selector predicate for matching references to groups.
///
/// All specified conditions must match (AND logic).
/// Use the `not` field for negation-based fallback groups.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct GroupSelector {
    /// Match references by type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub ref_type: Option<TypeSelector>,

    /// Match references by citation status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cited: Option<CitedStatus>,

    /// Match references by field values (e.g., language, keywords).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<HashMap<String, FieldMatcher>>,

    /// Negation for fallback groups.
    /// Matches references that do NOT match the nested selector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<GroupSelector>>,
}

/// Type-based selector.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum TypeSelector {
    /// Match a single type.
    Single(String),
    /// Match any of multiple types.
    Multiple(Vec<String>),
}

/// Citation status filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CitedStatus {
    /// Match only references cited in the document.
    Visible,
    /// Match only nocite references (silent citations).
    Silent,
    /// Match all references regardless of citation status.
    Any,
}

/// Field value matcher.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum FieldMatcher {
    /// Match exact field value.
    Exact(String),
    /// Match any of multiple values.
    Multiple(Vec<String>),
    // Future: Pattern(FieldPattern) for regex/glob matching
}

/// Per-group sorting specification.
///
/// Sorting follows a template of sort keys, applied in order.
/// The first key is the primary sort, second is the tiebreaker, etc.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct GroupSort {
    /// Ordered list of sort keys to apply.
    pub template: Vec<GroupSortKey>,
}

/// A single sort key in a group sorting template.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct GroupSortKey {
    /// The field or variable to sort by.
    pub key: SortKey,

    /// Sort order direction.
    #[serde(default = "default_true")]
    pub ascending: bool,

    /// For type-based ordering: explicit type sequence.
    ///
    /// Example: `["legal-case", "statute", "treaty"]` for Bluebook hierarchy.
    /// Items appear in this order regardless of alphabetical content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,

    /// For name-based sorting: culturally appropriate name order.
    ///
    /// Example: `given-family` for Vietnamese, `family-given` for Western.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<NameSortOrder>,
}

fn default_true() -> bool {
    true
}

/// Sort key selector.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SortKey {
    /// Sort by reference type.
    #[serde(rename = "type")]
    RefType,
    /// Sort by author/contributor.
    Author,
    /// Sort by title.
    Title,
    /// Sort by issued date.
    Issued,
    /// Sort by custom field.
    Field(String),
}

/// Name sorting order for culturally appropriate collation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum NameSortOrder {
    /// Family name first (Western convention).
    /// Example: "Smith, John" → "S" sorts before "T"
    FamilyGiven,
    /// Given name first (Vietnamese convention).
    /// Example: "Nguyễn Văn A" → "Nguyễn" sorts before "Trần"
    GivenFamily,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_selector_type_single() {
        let yaml = r#"
type: legal-case
"#;
        let selector: GroupSelector = serde_yaml::from_str(yaml).unwrap();
        assert!(selector.ref_type.is_some());
        match selector.ref_type.unwrap() {
            TypeSelector::Single(t) => assert_eq!(t, "legal-case"),
            _ => panic!("Expected Single"),
        }
    }

    #[test]
    fn test_group_selector_type_multiple() {
        let yaml = r#"
type: [legal-case, statute, treaty]
"#;
        let selector: GroupSelector = serde_yaml::from_str(yaml).unwrap();
        match selector.ref_type.unwrap() {
            TypeSelector::Multiple(types) => {
                assert_eq!(types, vec!["legal-case", "statute", "treaty"]);
            }
            _ => panic!("Expected Multiple"),
        }
    }

    #[test]
    fn test_group_selector_field_exact() {
        let yaml = r#"
field:
  language: vi
"#;
        let selector: GroupSelector = serde_yaml::from_str(yaml).unwrap();
        let fields = selector.field.unwrap();
        match fields.get("language").unwrap() {
            FieldMatcher::Exact(lang) => assert_eq!(lang, "vi"),
            _ => panic!("Expected Exact"),
        }
    }

    #[test]
    fn test_group_selector_negation() {
        let yaml = r#"
not:
  type: legal-case
"#;
        let selector: GroupSelector = serde_yaml::from_str(yaml).unwrap();
        let negated = selector.not.unwrap();
        assert!(negated.ref_type.is_some());
    }

    #[test]
    fn test_bibliography_group_minimal() {
        let yaml = r#"
id: cases
selector:
  type: legal-case
"#;
        let group: BibliographyGroup = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(group.id, "cases");
        assert!(group.heading.is_none());
        assert!(group.sort.is_none());
    }

    #[test]
    fn test_bibliography_group_full() {
        let yaml = r#"
id: vietnamese
heading: "Tài liệu tiếng Việt"
selector:
  field:
    language: vi
sort:
  template:
    - key: author
      sort-order: given-family
    - key: issued
      ascending: false
"#;
        let group: BibliographyGroup = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(group.id, "vietnamese");
        assert_eq!(group.heading.unwrap(), "Tài liệu tiếng Việt");

        let sort = group.sort.unwrap();
        assert_eq!(sort.template.len(), 2);

        match &sort.template[0].key {
            SortKey::Author => {}
            _ => panic!("Expected Author"),
        }
        assert_eq!(
            sort.template[0].sort_order,
            Some(NameSortOrder::GivenFamily)
        );

        match &sort.template[1].key {
            SortKey::Issued => {}
            _ => panic!("Expected Issued"),
        }
        assert!(!sort.template[1].ascending);
    }

    #[test]
    fn test_type_order_sorting() {
        let yaml = r#"
template:
  - key: type
    order: [legal-case, statute, treaty]
"#;
        let sort: GroupSort = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(sort.template.len(), 1);

        let order = sort.template[0].order.as_ref().unwrap();
        assert_eq!(order, &vec!["legal-case", "statute", "treaty"]);
    }
}
