/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Group-specific sorting for bibliography grouping.
//!
//! This module implements per-group sorting with support for:
//! - Type-order sorting (explicit sequence like [legal-case, statute, treaty])
//! - Name-order sorting (family-given vs given-family for multilingual bibliographies)
//! - Integration with standard sort keys (author, title, issued)

use csln_core::grouping::{GroupSort, GroupSortKey, NameSortOrder, SortKey as GroupSortKeyType};
use csln_core::locale::Locale;

use crate::reference::Reference;

pub struct GroupSorter<'a> {
    locale: &'a Locale,
}

impl<'a> GroupSorter<'a> {
    pub fn new(locale: &'a Locale) -> Self {
        Self { locale }
    }

    /// Sort references according to a group sort specification.
    ///
    /// Applies sort keys in order, with later keys acting as tiebreakers.
    ///
    /// # Arguments
    ///
    /// * `references` - References to sort
    /// * `sort_spec` - Group sort specification
    pub fn sort_references<'b>(
        &self,
        mut references: Vec<&'b Reference>,
        sort_spec: &GroupSort,
    ) -> Vec<&'b Reference> {
        references.sort_by(|a, b| {
            for sort_key in &sort_spec.template {
                let cmp = self.compare_by_key(a, b, sort_key);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            std::cmp::Ordering::Equal
        });
        references
    }

    /// Compare two references by a single sort key.
    fn compare_by_key(
        &self,
        a: &Reference,
        b: &Reference,
        sort_key: &GroupSortKey,
    ) -> std::cmp::Ordering {
        let cmp = match &sort_key.key {
            GroupSortKeyType::RefType => {
                if let Some(order) = &sort_key.order {
                    // Type-order sorting: explicit sequence
                    self.compare_by_type_order(a, b, order)
                } else {
                    // Alphabetical type comparison
                    a.ref_type().cmp(&b.ref_type())
                }
            }
            GroupSortKeyType::Author => {
                if let Some(name_order) = &sort_key.sort_order {
                    // Name-order sorting: culturally appropriate collation
                    self.compare_by_author_with_order(a, b, *name_order)
                } else {
                    // Default: family-given (Western convention)
                    self.compare_by_author_with_order(a, b, NameSortOrder::FamilyGiven)
                }
            }
            GroupSortKeyType::Title => self.compare_by_title(a, b),
            GroupSortKeyType::Issued => self.compare_by_issued(a, b),
            GroupSortKeyType::Field(field_name) => self.compare_by_field(a, b, field_name),
        };

        if sort_key.ascending {
            cmp
        } else {
            cmp.reverse()
        }
    }

    /// Compare by type using explicit order sequence.
    ///
    /// Types appear in the order specified, regardless of alphabetical content.
    /// Types not in the order list sort after those in the list, alphabetically.
    fn compare_by_type_order(
        &self,
        a: &Reference,
        b: &Reference,
        order: &[String],
    ) -> std::cmp::Ordering {
        let a_type = a.ref_type();
        let b_type = b.ref_type();

        let a_pos = order.iter().position(|t| t == &a_type);
        let b_pos = order.iter().position(|t| t == &b_type);

        match (a_pos, b_pos) {
            (Some(a_idx), Some(b_idx)) => a_idx.cmp(&b_idx),
            (Some(_), None) => std::cmp::Ordering::Less, // a in order, b not
            (None, Some(_)) => std::cmp::Ordering::Greater, // b in order, a not
            (None, None) => a_type.cmp(&b_type),         // both not in order, alphabetical
        }
    }

    /// Compare by author with culturally appropriate name ordering.
    fn compare_by_author_with_order(
        &self,
        a: &Reference,
        b: &Reference,
        name_order: NameSortOrder,
    ) -> std::cmp::Ordering {
        let a_key = self.extract_author_sort_key(a, name_order);
        let b_key = self.extract_author_sort_key(b, name_order);
        a_key.cmp(&b_key)
    }

    /// Extract author sort key with specified name ordering.
    fn extract_author_sort_key(&self, reference: &Reference, name_order: NameSortOrder) -> String {
        reference
            .author()
            .and_then(|c| c.to_names_vec().first().cloned())
            .map(|name| match name_order {
                NameSortOrder::FamilyGiven => {
                    // Western: "Smith, John" → sort by "smith"
                    name.family_or_literal().to_lowercase()
                }
                NameSortOrder::GivenFamily => {
                    // Vietnamese: "Nguyễn Văn A" → sort by "nguyễn"
                    // For Vietnamese names, family comes first, but we need to use
                    // the full name since the display order matches sort order
                    name.family_or_literal().to_lowercase()
                }
            })
            .or_else(|| {
                // Fallback to editor
                reference
                    .editor()
                    .and_then(|c| c.to_names_vec().first().cloned())
                    .map(|name| name.family_or_literal().to_lowercase())
            })
            .or_else(|| {
                // Fallback to title
                reference.title().map(|t| {
                    self.locale
                        .strip_sort_articles(&t.to_string())
                        .to_lowercase()
                })
            })
            .unwrap_or_default()
    }

    /// Compare by title (with article stripping).
    fn compare_by_title(&self, a: &Reference, b: &Reference) -> std::cmp::Ordering {
        let a_title = self
            .locale
            .strip_sort_articles(&a.title().map(|t| t.to_string()).unwrap_or_default())
            .to_lowercase();
        let b_title = self
            .locale
            .strip_sort_articles(&b.title().map(|t| t.to_string()).unwrap_or_default())
            .to_lowercase();
        a_title.cmp(&b_title)
    }

    /// Compare by issued date.
    fn compare_by_issued(&self, a: &Reference, b: &Reference) -> std::cmp::Ordering {
        let a_year = a
            .issued()
            .and_then(|d| d.year().parse::<i32>().ok())
            .unwrap_or(0);
        let b_year = b
            .issued()
            .and_then(|d| d.year().parse::<i32>().ok())
            .unwrap_or(0);
        a_year.cmp(&b_year)
    }

    /// Compare by custom field.
    fn compare_by_field(
        &self,
        a: &Reference,
        b: &Reference,
        field_name: &str,
    ) -> std::cmp::Ordering {
        match field_name {
            "language" => {
                let a_lang = a.language().unwrap_or_default();
                let b_lang = b.language().unwrap_or_default();
                a_lang.cmp(&b_lang)
            }
            // Future: support for keywords, custom metadata
            _ => std::cmp::Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::grouping::GroupSortKey;

    fn make_locale() -> Locale {
        Locale::en_us()
    }

    fn make_reference(
        id: &str,
        ref_type: &str,
        author_family: &str,
        title: &str,
        year: i32,
    ) -> Reference {
        let json = serde_json::json!({
            "id": id,
            "type": ref_type,
            "author": [{"family": author_family, "given": "Test"}],
            "issued": {"date-parts": [[year]]},
            "title": title,
            "container-title": "Test Container",
        });
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(json).unwrap();
        legacy.into()
    }

    #[test]
    fn test_type_order_sorting() {
        let locale = make_locale();
        let sorter = GroupSorter::new(&locale);

        // Use standard CSL JSON types for testing
        let journal = make_reference("r1", "article-journal", "Smith", "Title J", 1990);
        let magazine = make_reference("r2", "article-magazine", "Jones", "Title M", 2000);
        let newspaper = make_reference("r3", "article-newspaper", "Brown", "Title N", 1985);
        let book = make_reference("r4", "book", "Davis", "Title B", 1995);

        let mut refs = vec![&book, &newspaper, &journal, &magazine];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::RefType,
                ascending: true,
                order: Some(vec![
                    "article-journal".to_string(),
                    "article-magazine".to_string(),
                    "article-newspaper".to_string(),
                ]),
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be: article-journal, article-magazine, article-newspaper, then book (alphabetically after)
        assert_eq!(refs[0].id().unwrap(), "r1"); // article-journal
        assert_eq!(refs[1].id().unwrap(), "r2"); // article-magazine
        assert_eq!(refs[2].id().unwrap(), "r3"); // article-newspaper
        assert_eq!(refs[3].id().unwrap(), "r4"); // book
    }

    #[test]
    fn test_author_family_given_order() {
        let locale = make_locale();
        let sorter = GroupSorter::new(&locale);

        let smith = make_reference("r1", "book", "Smith", "Title", 2000);
        let jones = make_reference("r2", "book", "Jones", "Title", 2000);
        let brown = make_reference("r3", "book", "Brown", "Title", 2000);

        let mut refs = vec![&smith, &jones, &brown];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Author,
                ascending: true,
                order: None,
                sort_order: Some(NameSortOrder::FamilyGiven),
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be alphabetical by family name
        assert_eq!(refs[0].id().unwrap(), "r3"); // Brown
        assert_eq!(refs[1].id().unwrap(), "r2"); // Jones
        assert_eq!(refs[2].id().unwrap(), "r1"); // Smith
    }

    #[test]
    fn test_issued_descending() {
        let locale = make_locale();
        let sorter = GroupSorter::new(&locale);

        let old = make_reference("r1", "book", "Smith", "Title", 1990);
        let new = make_reference("r2", "book", "Jones", "Title", 2020);
        let mid = make_reference("r3", "book", "Brown", "Title", 2005);

        let mut refs = vec![&old, &new, &mid];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Issued,
                ascending: false, // Descending
                order: None,
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be newest first
        assert_eq!(refs[0].id().unwrap(), "r2"); // 2020
        assert_eq!(refs[1].id().unwrap(), "r3"); // 2005
        assert_eq!(refs[2].id().unwrap(), "r1"); // 1990
    }

    #[test]
    fn test_composite_sort() {
        let locale = make_locale();
        let sorter = GroupSorter::new(&locale);

        let smith2020 = make_reference("r1", "book", "Smith", "Title", 2020);
        let smith2010 = make_reference("r2", "book", "Smith", "Title", 2010);
        let jones2020 = make_reference("r3", "book", "Jones", "Title", 2020);

        let mut refs = vec![&smith2020, &jones2020, &smith2010];

        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: GroupSortKeyType::Author,
                    ascending: true,
                    order: None,
                    sort_order: Some(NameSortOrder::FamilyGiven),
                },
                GroupSortKey {
                    key: GroupSortKeyType::Issued,
                    ascending: false, // Descending within author
                    order: None,
                    sort_order: None,
                },
            ],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be: Jones 2020, then Smith 2020, then Smith 2010
        assert_eq!(refs[0].id().unwrap(), "r3"); // Jones 2020
        assert_eq!(refs[1].id().unwrap(), "r1"); // Smith 2020
        assert_eq!(refs[2].id().unwrap(), "r2"); // Smith 2010
    }

    #[test]
    fn test_legal_citation_sort() {
        let locale = make_locale();
        let sorter = GroupSorter::new(&locale);

        let case_a = make_reference("r1", "legal-case", "", "Doe v. Smith", 1990);
        let case_b = make_reference("r2", "legal-case", "", "Brown v. Board", 1954);

        let mut refs = vec![&case_a, &case_b];

        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: GroupSortKeyType::Title, // Case name
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: GroupSortKeyType::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
        };

        refs = sorter.sort_references(refs, &sort_spec);
        assert_eq!(refs[0].id().unwrap(), "r2"); // Brown v. Board
    }

    #[test]
    fn test_legal_hierarchy_sort() {
        let locale = make_locale();
        let sorter = GroupSorter::new(&locale);

        let statute = make_reference("r1", "statute", "", "Clean Air Act", 1970);
        let case = make_reference("r2", "legal-case", "", "Roe v. Wade", 1973);
        let treaty = make_reference("r3", "treaty", "", "Paris Agreement", 2015);

        let mut refs = vec![&treaty, &case, &statute];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::RefType,
                ascending: true,
                order: Some(vec![
                    "legal-case".to_string(),
                    "statute".to_string(),
                    "treaty".to_string(),
                ]),
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Hierarchy: case, statute, treaty
        assert_eq!(refs[0].id().unwrap(), "r2");
        assert_eq!(refs[1].id().unwrap(), "r1");
        assert_eq!(refs[2].id().unwrap(), "r3");
    }
}
