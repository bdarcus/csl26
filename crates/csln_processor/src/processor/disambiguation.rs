use crate::reference::{Bibliography, Reference};
use crate::values::ProcHints;
use csln_core::options::Config;
use std::collections::{HashMap, HashSet};

/// Handles disambiguation logic for author-date citations.
pub struct Disambiguator<'a> {
    bibliography: &'a Bibliography,
    config: &'a Config,
}

impl<'a> Disambiguator<'a> {
    pub fn new(bibliography: &'a Bibliography, config: &'a Config) -> Self {
        Self {
            bibliography,
            config,
        }
    }

    /// Calculate processing hints for disambiguation.
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
        // Sort group by title for consistent suffix assignment (a, b, c...)
        // This matches citeproc-js behavior where suffixes are alphabetical by title
        let mut sorted_group: Vec<&Reference> = group.to_vec();
        sorted_group.sort_by(|a, b| {
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
