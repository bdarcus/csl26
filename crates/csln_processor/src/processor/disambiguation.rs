use crate::reference::{Bibliography, Reference};
use crate::values::ProcHints;
use csln_core::options::Config;
use std::collections::{HashMap, HashSet};

use crate::grouping::GroupSorter;
use csln_core::grouping::GroupSort;
use csln_core::locale::Locale;

/// Handles disambiguation logic for author-date citations.
///
/// Disambiguation resolves ambiguities when multiple references produce
/// identical rendered strings. The processor applies strategies in cascade:
///
/// 1. **Name expansion** (`disambiguate-add-names`): If et-al is triggered
///    in the base citation, try expanding the author list to differentiate
///    references with same first author and year.
///
/// 2. **Given name expansion** (`disambiguate-add-givenname`): Add initials
///    or full given names to author list to resolve remaining collisions
///    (e.g., "Smith, John" vs "Smith, Jane").
///
/// 3. **Combined expansion**: Try showing both more names AND given names
///    to maximize differentiation before falling back to year suffix.
///
/// 4. **Year suffix fallback** (`disambiguate-add-year-suffix`): If above
///    strategies fail, append letters (a, b, c, ..., z, aa, ab, ...) to
///    the year. Sorting is deterministic by reference title (lowercase).
///
/// ## Algorithm Overview
///
/// - References are grouped by author-year key (e.g., "smith:2020")
/// - For each group with 2+ collisions, strategies are applied in order
/// - Once a strategy resolves ambiguity, higher-priority strategies skip
/// - Year suffix assignment is deterministic by title sort order
///
/// ## Output
///
/// Returns `ProcHints` for each reference containing:
/// - `group_index`: Position within collision group (1-indexed)
/// - `group_length`: Total references in collision group
/// - `group_key`: Author-year key used for grouping
/// - `disamb_condition`: Whether year suffix should be applied
/// - `expand_given_names`: Whether to show given names/initials
/// - `min_names_to_show`: Minimum author count for name expansion
pub struct Disambiguator<'a> {
    bibliography: &'a Bibliography,
    config: &'a Config,
    locale: &'a Locale,
    group_sort: Option<&'a GroupSort>,
}

impl<'a> Disambiguator<'a> {
    pub fn new(bibliography: &'a Bibliography, config: &'a Config, locale: &'a Locale) -> Self {
        Self {
            bibliography,
            config,
            locale,
            group_sort: None,
        }
    }

    pub fn with_group_sort(
        bibliography: &'a Bibliography,
        config: &'a Config,
        locale: &'a Locale,
        group_sort: &'a GroupSort,
    ) -> Self {
        Self {
            bibliography,
            config,
            locale,
            group_sort: Some(group_sort),
        }
    }

    /// Calculate processing hints for disambiguation across all references.
    ///
    /// This is a single-pass algorithm that:
    /// 1. Groups references by author-year collision key
    /// 2. For each group with multiple references, applies disambiguation
    ///    strategies in cascade order
    /// 3. Returns pre-calculated hints for the renderer
    ///
    /// ## Cascade Order
    ///
    /// For each collision group:
    /// - Try expanding author list (et-al → full names)
    /// - Try adding given names/initials
    /// - Try combined approach (more names + given names)
    /// - Fall back to year suffix (a, b, c, ...)
    ///
    /// ## Performance
    ///
    /// - O(n) for grouping, where n = number of references
    /// - O(g²) for collision detection within each group g
    /// - Total: O(n + Σ(g²)) where typical g << n
    ///
    /// ## Example
    ///
    /// Input bibliography:
    /// - Smith, John (2020) - "Article A"
    /// - Smith, Jane (2020) - "Article B"
    /// - Brown, Tom (2020) - "Article C"
    ///
    /// Output hints:
    /// - "smith-john-2020": { expand_given_names: true, group_length: 2 }
    /// - "smith-jane-2020": { expand_given_names: true, group_length: 2 }
    /// - "brown-tom-2020": { } (no collision)
    pub fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let mut hints = HashMap::new();

        let refs: Vec<&Reference> = self.bibliography.values().collect();
        // Group by base citation key (e.g. "smith:2020")
        let grouped = self.group_references(refs);

        for (key, group) in grouped {
            let group_len = group.len();

            if group_len > 1 {
                // Different references colliding in their base citation form
                let disamb_config = self
                    .config
                    .processing
                    .as_ref()
                    .and_then(|p| p.config().disambiguate);

                let add_names = disamb_config.as_ref().map(|d| d.names).unwrap_or(false);
                let add_givenname = disamb_config
                    .as_ref()
                    .map(|d| d.add_givenname)
                    .unwrap_or(false);

                let mut resolved = false;

                // 1. Try expanding names (et-al expansion)
                if add_names {
                    if let Some(n) = self.check_names_resolution(&group) {
                        for (i, reference) in group.iter().enumerate() {
                            hints.insert(
                                reference.id().unwrap_or_default(),
                                ProcHints {
                                    disamb_condition: false,
                                    group_index: i + 1,
                                    group_length: group_len,
                                    group_key: key.clone(),
                                    expand_given_names: false,
                                    min_names_to_show: Some(n),
                                    ..Default::default()
                                },
                            );
                        }
                        resolved = true;
                    }
                }

                // 2. Try expanding given names for the base name list
                if !resolved && add_givenname && self.check_givenname_resolution(&group, None) {
                    for (i, reference) in group.iter().enumerate() {
                        hints.insert(
                            reference.id().unwrap_or_default(),
                            ProcHints {
                                disamb_condition: false,
                                group_index: i + 1,
                                group_length: group_len,
                                group_key: key.clone(),
                                expand_given_names: true,
                                min_names_to_show: None,
                                ..Default::default()
                            },
                        );
                    }
                    resolved = true;
                }

                // 3. Try combined expansion: multiple names + given names
                if !resolved && add_names && add_givenname {
                    // Find if there's an N such that expanding both names and given names works
                    let max_authors = group
                        .iter()
                        .map(|r| r.author().map(|a| a.to_names_vec().len()).unwrap_or(0))
                        .max()
                        .unwrap_or(0);

                    for n in 2..=max_authors {
                        if self.check_givenname_resolution(&group, Some(n)) {
                            for (idx, reference) in group.iter().enumerate() {
                                hints.insert(
                                    reference.id().unwrap_or_default(),
                                    ProcHints {
                                        disamb_condition: false,
                                        group_index: idx + 1,
                                        group_length: group_len,
                                        group_key: key.clone(),
                                        expand_given_names: true,
                                        min_names_to_show: Some(n),
                                        ..Default::default()
                                    },
                                );
                            }
                            resolved = true;
                            break;
                        }
                    }
                }

                // 4. Fallback to year-suffix
                if !resolved {
                    self.apply_year_suffix(&mut hints, &group, key, group_len, false);
                }
            } else {
                // No collision
                hints.insert(group[0].id().unwrap_or_default(), ProcHints::default());
            }
        }

        hints
    }

    fn apply_year_suffix(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        group: &[&Reference],
        key: String,
        len: usize,
        expand_names: bool,
    ) {
        let sorted_group = if let Some(sort_spec) = self.group_sort {
            // Use GroupSorter for per-group ordering
            let sorter = GroupSorter::new(self.locale);
            sorter.sort_references(group.to_vec(), sort_spec)
        } else {
            // Fallback to title sorting (default behavior)
            let mut sorted: Vec<&Reference> = group.to_vec();
            sorted.sort_by(|a, b| {
                let a_title = a
                    .title()
                    .map(|t| t.to_string())
                    .unwrap_or_default()
                    .to_lowercase();
                let b_title = b
                    .title()
                    .map(|t| t.to_string())
                    .unwrap_or_default()
                    .to_lowercase();
                a_title.cmp(&b_title)
            });
            sorted
        };

        for (i, reference) in sorted_group.iter().enumerate() {
            hints.insert(
                reference.id().unwrap_or_default(),
                ProcHints {
                    disamb_condition: true,
                    group_index: i + 1,
                    group_length: len,
                    group_key: key.clone(),
                    expand_given_names: expand_names,
                    min_names_to_show: None,
                    ..Default::default()
                },
            );
        }
    }

    /// Check if showing more names resolves ambiguity in the group.
    fn check_names_resolution(&self, group: &[&Reference]) -> Option<usize> {
        let max_authors = group
            .iter()
            .map(|r| r.author().map(|a| a.to_names_vec().len()).unwrap_or(0))
            .max()
            .unwrap_or(0);

        for n in 2..=max_authors {
            let mut seen = HashSet::new();
            let mut collision = false;
            for reference in group {
                let key = if let Some(a) = reference.author() {
                    a.to_names_vec()
                        .iter()
                        .take(n)
                        .map(|name| name.family_or_literal().to_lowercase())
                        .collect::<Vec<_>>()
                        .join("|")
                } else {
                    "".to_string()
                };
                if !seen.insert(key) {
                    collision = true;
                    break;
                }
            }
            if !collision {
                return Some(n);
            }
        }
        None
    }

    /// Check if expanding to full names resolves ambiguity in the group.
    /// If `min_names` is Some(n), it checks resolution when showing n names.
    fn check_givenname_resolution(&self, group: &[&Reference], min_names: Option<usize>) -> bool {
        let mut seen = HashSet::new();
        let mut collision = false;
        for reference in group {
            if let Some(authors) = reference.author() {
                let n = min_names.unwrap_or(1);
                // Create a key for the first n authors with full names
                let key = authors
                    .to_names_vec()
                    .iter()
                    .take(n)
                    .map(|n| {
                        format!(
                            "{:?}|{:?}|{:?}|{:?}",
                            n.family, n.given, n.non_dropping_particle, n.dropping_particle
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("||");

                if !seen.insert(key) {
                    collision = true;
                    break;
                }
            } else if !seen.insert("".to_string()) {
                collision = true;
                break;
            }
        }
        !collision
    }

    /// Group references by author-year for disambiguation.
    fn group_references<'b>(
        &self,
        references: Vec<&'b Reference>,
    ) -> HashMap<String, Vec<&'b Reference>> {
        let mut groups: HashMap<String, Vec<&'b Reference>> = HashMap::new();

        for reference in references {
            let key = self.make_group_key(reference);
            groups.entry(key).or_default().push(reference);
        }

        groups
    }

    /// Create a grouping key for a reference based on its base citation form.
    fn make_group_key(&self, reference: &Reference) -> String {
        let shorten = self
            .config
            .contributors
            .as_ref()
            .and_then(|c| c.shorten.as_ref());

        let author_key = if let Some(authors) = reference.author() {
            let names_vec = authors.to_names_vec();
            if let Some(opts) = shorten {
                if names_vec.len() >= opts.min as usize {
                    // Show 'use_first' names in the base citation
                    names_vec
                        .iter()
                        .take(opts.use_first as usize)
                        .map(|n| n.family_or_literal().to_lowercase())
                        .collect::<Vec<_>>()
                        .join(",")
                        + ",et-al"
                } else {
                    names_vec
                        .iter()
                        .map(|n| n.family_or_literal().to_lowercase())
                        .collect::<Vec<_>>()
                        .join(",")
                }
            } else {
                names_vec
                    .iter()
                    .map(|n| n.family_or_literal().to_lowercase())
                    .collect::<Vec<_>>()
                    .join(",")
            }
        } else {
            "".to_string()
        };

        let year = reference
            .issued()
            .and_then(|d| d.year().parse::<i32>().ok())
            .map(|y| y.to_string())
            .unwrap_or_default();

        format!("{}:{}", author_key, year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::grouping::{GroupSort, GroupSortKey, SortKey};
    use csln_core::reference::{
        Contributor, EdtfString, InputReference as Reference, Monograph, MonographType,
        MultilingualString, StructuredName, Title,
    };

    fn make_ref(id: &str, family: &str, title: &str, year: i32) -> Reference {
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.to_string()),
            r#type: MonographType::Book,
            title: Title::Single(title.to_string()),
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple("Test".to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            editor: None,
            translator: None,
            issued: EdtfString(year.to_string()),
            publisher: None,
            url: None,
            accessed: None,
            language: None,
            note: None,
            isbn: None,
            doi: None,
            edition: None,
            genre: None,
            keywords: None,
            original_date: None,
            original_title: None,
        }))
    }

    #[test]
    fn test_group_aware_year_suffix_sort() {
        let r1 = make_ref("r1", "Smith", "Beta", 2020);
        let r2 = make_ref("r2", "Smith", "Alpha", 2020);

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), r1);
        bib.insert("r2".to_string(), r2);

        let config = Config::default();
        let locale = Locale::en_us();

        // 1. Default sorting (by title): r2 (Alpha) should be 'a', r1 (Beta) should be 'b'
        let disamb_default = Disambiguator::new(&bib, &config, &locale);
        let hints_default = disamb_default.calculate_hints();

        assert_eq!(hints_default.get("r2").unwrap().group_index, 1);
        assert_eq!(hints_default.get("r1").unwrap().group_index, 2);

        // 2. Custom group sort: Sort by title descending -> r1 (Beta) should be 'a', r2 (Alpha) should be 'b'
        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: SortKey::Title,
                ascending: false,
                order: None,
                sort_order: None,
            }],
        };

        let disamb_custom = Disambiguator::with_group_sort(&bib, &config, &locale, &sort_spec);
        let hints_custom = disamb_custom.calculate_hints();

        assert_eq!(hints_custom.get("r1").unwrap().group_index, 1);
        assert_eq!(hints_custom.get("r2").unwrap().group_index, 2);
    }
}
