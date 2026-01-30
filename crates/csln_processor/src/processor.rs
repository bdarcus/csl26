/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! The CSLN processor for rendering citations and bibliographies.
//!
//! ## Architecture
//!
//! The processor is intentionally "dumb" - it applies the style as written
//! without implicit logic. Style-specific behavior (e.g., suppress publisher
//! for journals) should be expressed in the style YAML via `overrides`, not
//! hardcoded here.
//!
//! ## CSL 1.0 Compatibility
//!
//! The processor implements the CSL 1.0 "variable-once" rule:
//! > "Substituted variables are suppressed in the rest of the output to
//! > prevent duplication."
//!
//! This is tracked via `rendered_vars` in `process_template()`.

use crate::error::ProcessorError;
use crate::reference::{Bibliography, Citation, Name, Reference};
use crate::render::{citation_to_string, refs_to_string, ProcTemplate, ProcTemplateComponent};
use crate::values::{ComponentValues, ProcHints, RenderContext, RenderOptions};
use csln_core::locale::Locale;
use csln_core::options::{Config, SortKey};
use csln_core::template::{TemplateComponent, WrapPunctuation};
use csln_core::Style;
use std::collections::HashMap;

/// The CSLN processor.
///
/// Takes a style, bibliography, and citations, and produces formatted output.
#[derive(Debug)]
pub struct Processor {
    /// The style definition.
    style: Style,
    /// The bibliography (references keyed by ID).
    bibliography: Bibliography,
    /// The locale for terms and formatting.
    locale: Locale,
    /// Default configuration.
    default_config: Config,
    /// Pre-calculated processing hints.
    hints: HashMap<String, ProcHints>,
    /// Citation numbers assigned to references (for numeric styles).
    citation_numbers: std::cell::RefCell<HashMap<String, usize>>,
}

impl Default for Processor {
    fn default() -> Self {
        Self {
            style: Style::default(),
            bibliography: Bibliography::default(),
            locale: Locale::en_us(),
            default_config: Config::default(),
            hints: HashMap::new(),
            citation_numbers: std::cell::RefCell::new(HashMap::new()),
        }
    }
}

/// Processed output containing citations and bibliography.
#[derive(Debug, Default)]
pub struct ProcessedReferences {
    /// Rendered bibliography entries.
    pub bibliography: Vec<ProcTemplate>,
    /// Rendered citations (if any).
    pub citations: Option<Vec<String>>,
}

impl Processor {
    /// Create a new processor with default English locale.
    pub fn new(style: Style, bibliography: Bibliography) -> Self {
        Self::with_locale(style, bibliography, Locale::en_us())
    }

    /// Create a new processor with a custom locale.
    pub fn with_locale(style: Style, bibliography: Bibliography, locale: Locale) -> Self {
        let mut processor = Processor {
            style,
            bibliography,
            locale,
            default_config: Config::default(),
            hints: HashMap::new(),
            citation_numbers: std::cell::RefCell::new(HashMap::new()),
        };

        // Pre-calculate hints for disambiguation
        processor.hints = processor.calculate_hints();
        processor
    }

    /// Create a new processor, loading the locale based on the style's default-locale.
    ///
    /// The `locales_dir` should point to a directory containing YAML locale files
    /// (e.g., "en-US.yaml", "de-DE.yaml").
    pub fn with_style_locale(
        style: Style,
        bibliography: Bibliography,
        locales_dir: &std::path::Path,
    ) -> Self {
        let locale = if let Some(ref locale_id) = style.info.default_locale {
            Locale::load(locale_id, locales_dir)
        } else {
            Locale::en_us()
        };
        Self::with_locale(style, bibliography, locale)
    }

    /// Get the style configuration.
    fn get_config(&self) -> &Config {
        self.style.options.as_ref().unwrap_or(&self.default_config)
    }

    /// Process all references to get rendered output.
    pub fn process_references(&self) -> ProcessedReferences {
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        let mut bibliography: Vec<ProcTemplate> = Vec::new();
        let mut prev_reference: Option<&Reference> = None;

        let bib_config = self.get_config().bibliography.as_ref();
        let substitute = bib_config.and_then(|c| c.subsequent_author_substitute.as_ref());

        for (index, reference) in sorted_refs.iter().enumerate() {
            // Bibliography entry number is 1-based position in sorted list
            let entry_number = index + 1;
            if let Some(mut proc) = self.process_bibliography_entry(reference, entry_number) {
                // Apply subsequent author substitution if enabled
                if let Some(sub_string) = substitute {
                    if let Some(prev) = prev_reference {
                        // Check if primary contributor matches
                        if self.contributors_match(prev, reference) {
                            self.apply_author_substitution(&mut proc, sub_string);
                        }
                    }
                }

                bibliography.push(proc);
                prev_reference = Some(reference);
            }
        }

        ProcessedReferences {
            bibliography,
            citations: None,
        }
    }

    /// Process a single citation.
    pub fn process_citation(&self, citation: &Citation) -> Result<String, ProcessorError> {
        let citation_spec = self.style.citation.as_ref();
        let template = citation_spec
            .map(|cs| cs.template.as_slice())
            .unwrap_or_default();

        // Get intra-citation delimiter (between components like author and year)
        let intra_delimiter = citation_spec
            .and_then(|cs| cs.delimiter.as_deref())
            .unwrap_or(", ");

        // Render each citation item, then join with inter-citation delimiter
        let mut rendered_items = Vec::new();

        for item in &citation.items {
            let reference = self
                .bibliography
                .get(&item.id)
                .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;

            // Get or assign citation number for numeric styles
            let citation_number = self.get_or_assign_citation_number(&item.id);

            if let Some(proc) = self.process_template_with_number(
                reference,
                template,
                RenderContext::Citation,
                citation_number,
            ) {
                // Each item's parts are joined with intra-citation delimiter
                let item_str = citation_to_string(&proc, None, None, None, Some(intra_delimiter));
                if !item_str.is_empty() {
                    rendered_items.push(item_str);
                }
            }
        }

        // Get wrap/prefix/suffix/multi-cite-delimiter from citation spec
        let wrap = citation_spec.and_then(|cs| cs.wrap.as_ref());
        let prefix = citation_spec.and_then(|cs| cs.prefix.as_deref());
        let suffix = citation_spec.and_then(|cs| cs.suffix.as_deref());
        // Inter-citation delimiter (between multiple refs)
        let inter_delimiter = citation_spec
            .and_then(|cs| cs.multi_cite_delimiter.as_deref())
            .unwrap_or("; ");

        let content = rendered_items.join(inter_delimiter);

        // Apply wrap or prefix/suffix
        let (open, close) = match wrap {
            Some(WrapPunctuation::Parentheses) => ("(", ")"),
            Some(WrapPunctuation::Brackets) => ("[", "]"),
            _ => (prefix.unwrap_or(""), suffix.unwrap_or("")),
        };

        Ok(format!("{}{}{}", open, content, close))
    }

    /// Get the citation number for a reference, assigning one if not yet cited.
    fn get_or_assign_citation_number(&self, ref_id: &str) -> usize {
        let mut numbers = self.citation_numbers.borrow_mut();
        let next_num = numbers.len() + 1;
        *numbers.entry(ref_id.to_string()).or_insert(next_num)
    }

    /// Process a bibliography entry.
    ///
    /// Uses type-specific template if available for the reference's item type,
    /// otherwise falls back to the default template.
    fn process_bibliography_entry(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate> {
        let bib_spec = self.style.bibliography.as_ref()?;

        // Check for type-specific template
        let template = if let Some(type_templates) = &bib_spec.type_templates {
            type_templates
                .get(&reference.ref_type)
                .unwrap_or(&bib_spec.template)
        } else {
            &bib_spec.template
        };

        self.process_template_with_number(
            reference,
            template,
            RenderContext::Bibliography,
            entry_number,
        )
    }

    /// Process a template for a reference with citation number.
    ///
    /// Iterates through template components, extracting values from the reference.
    /// Empty values are skipped. Implements the CSL 1.0 "variable-once" rule:
    /// each variable can only be rendered once per reference to prevent duplication.
    ///
    /// ## Variable Deduplication
    ///
    /// Per CSL 1.0 spec: "Substituted variables are suppressed in the rest of the
    /// output to prevent duplication." We implement this by tracking rendered
    /// variables in a HashSet and skipping any that have already been rendered.
    ///
    /// This prevents issues like author appearing twice if used as substitute
    /// for editor.
    fn process_template_with_number(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        context: RenderContext,
        citation_number: usize,
    ) -> Option<ProcTemplate> {
        let config = self.get_config();
        let options = RenderOptions {
            config,
            locale: &self.locale,
            context,
        };
        let default_hint = ProcHints::default();
        let base_hint = self.hints.get(&reference.id).unwrap_or(&default_hint);

        // Create a hint with citation number
        let hint = ProcHints {
            citation_number: if citation_number > 0 {
                Some(citation_number)
            } else {
                None
            },
            ..base_hint.clone()
        };

        // Track rendered variables to prevent duplicates (CSL 1.0 spec:
        // "Substituted variables are suppressed in the rest of the output")
        let mut rendered_vars: std::collections::HashSet<String> = std::collections::HashSet::new();

        let components: Vec<ProcTemplateComponent> = template
            .iter()
            .filter_map(|component| {
                // Get unique key for this variable (e.g., "contributor:Author")
                let var_key = get_variable_key(component);

                // Skip if this variable was already rendered
                if let Some(ref key) = var_key {
                    if rendered_vars.contains(key) {
                        return None;
                    }
                }

                // Extract value from reference
                let values = component.values(reference, &hint, &options)?;
                if values.value.is_empty() {
                    return None;
                }

                // Mark variable as rendered for deduplication
                if let Some(key) = var_key {
                    rendered_vars.insert(key);
                }

                Some(ProcTemplateComponent {
                    template_component: component.clone(),
                    value: values.value,
                    prefix: values.prefix,
                    suffix: values.suffix,
                    ref_type: Some(reference.ref_type.clone()),
                    config: Some(options.config.clone()),
                })
            })
            .collect();

        if components.is_empty() {
            None
        } else {
            Some(components)
        }
    }

    /// Sort references according to style instructions.
    fn sort_references<'a>(&self, references: Vec<&'a Reference>) -> Vec<&'a Reference> {
        let mut refs = references;
        let config = self.get_config();
        let processing = config.processing.as_ref().cloned().unwrap_or_default();
        let proc_config = processing.config();

        if let Some(sort_config) = &proc_config.sort {
            // Apply sorts in reverse order (last sort is most significant)
            for sort in sort_config.template.iter().rev() {
                match sort.key {
                    SortKey::Author => {
                        refs.sort_by(|a, b| {
                            let a_author = a
                                .author
                                .as_ref()
                                .and_then(|names| names.first())
                                .map(|n| n.family_or_literal().to_lowercase())
                                .unwrap_or_default();
                            let b_author = b
                                .author
                                .as_ref()
                                .and_then(|names| names.first())
                                .map(|n| n.family_or_literal().to_lowercase())
                                .unwrap_or_default();

                            if sort.ascending {
                                a_author.cmp(&b_author)
                            } else {
                                b_author.cmp(&a_author)
                            }
                        });
                    }
                    SortKey::Year => {
                        refs.sort_by(|a, b| {
                            let a_year =
                                a.issued.as_ref().and_then(|d| d.year_value()).unwrap_or(0);
                            let b_year =
                                b.issued.as_ref().and_then(|d| d.year_value()).unwrap_or(0);

                            if sort.ascending {
                                a_year.cmp(&b_year)
                            } else {
                                b_year.cmp(&a_year)
                            }
                        });
                    }
                    SortKey::Title => {
                        refs.sort_by(|a, b| {
                            let a_title = a.title.as_deref().unwrap_or("").to_lowercase();
                            let b_title = b.title.as_deref().unwrap_or("").to_lowercase();

                            if sort.ascending {
                                a_title.cmp(&b_title)
                            } else {
                                b_title.cmp(&a_title)
                            }
                        });
                    }
                    SortKey::CitationNumber => {
                        // For citation-number sorting, we need to use the citation order
                        // This is typically set during citation processing
                        // For now, keep original order (first cited = first in bib)
                    }
                    // Handle future SortKey variants (non_exhaustive)
                    _ => {}
                }
            }
        }

        refs
    }

    /// Calculate processing hints for disambiguation.
    fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let mut hints = HashMap::new();
        let config = self.get_config();

        let refs: Vec<&Reference> = self.bibliography.values().collect();
        // Group by base citation key (e.g. "smith:2020")
        let grouped = self.group_references(refs);

        for (key, group) in grouped {
            let group_len = group.len();

            if group_len > 1 {
                // Different references colliding in their base citation form
                let disamb_config = config
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
                                reference.id.clone(),
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
                            reference.id.clone(),
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
                        .map(|r| r.author.as_ref().map(|a| a.len()).unwrap_or(0))
                        .max()
                        .unwrap_or(0);

                    for n in 2..=max_authors {
                        if self.check_givenname_resolution(&group, Some(n)) {
                            for (idx, reference) in group.iter().enumerate() {
                                hints.insert(
                                    reference.id.clone(),
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
                hints.insert(group[0].id.clone(), ProcHints::default());
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
        for (i, reference) in group.iter().enumerate() {
            hints.insert(
                reference.id.clone(),
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
            .map(|r| r.author.as_ref().map(|a| a.len()).unwrap_or(0))
            .max()
            .unwrap_or(0);

        for n in 2..=max_authors {
            let mut seen = std::collections::HashSet::new();
            let mut collision = false;
            for reference in group {
                let authors = reference.author.as_ref();
                let key = if let Some(a) = authors {
                    a.iter()
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
        let mut seen = std::collections::HashSet::new();
        for reference in group {
            if let Some(authors) = &reference.author {
                let n = min_names.unwrap_or(1);
                // Create a key for the first n authors with full names
                let key = authors
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
                    return false;
                }
            } else if !seen.insert("".to_string()) {
                return false;
            }
        }
        true
    }

    /// Group references by author-year for disambiguation.
    fn group_references<'a>(
        &self,
        references: Vec<&'a Reference>,
    ) -> HashMap<String, Vec<&'a Reference>> {
        let mut groups: HashMap<String, Vec<&'a Reference>> = HashMap::new();
        let config = self.get_config();

        for reference in references {
            let key = self.make_group_key(reference, config);
            groups.entry(key).or_default().push(reference);
        }

        groups
    }

    /// Create a grouping key for a reference based on its base citation form.
    fn make_group_key(&self, reference: &Reference, config: &Config) -> String {
        let shorten = config
            .contributors
            .as_ref()
            .and_then(|c| c.shorten.as_ref());

        let author_key = if let Some(authors) = &reference.author {
            if let Some(opts) = shorten {
                if authors.len() >= opts.min as usize {
                    // Show 'use_first' names in the base citation
                    authors
                        .iter()
                        .take(opts.use_first as usize)
                        .map(|n| n.family_or_literal().to_lowercase())
                        .collect::<Vec<_>>()
                        .join(",")
                        + ",et-al"
                } else {
                    authors
                        .iter()
                        .map(|n| n.family_or_literal().to_lowercase())
                        .collect::<Vec<_>>()
                        .join(",")
                }
            } else {
                authors
                    .iter()
                    .map(|n| n.family_or_literal().to_lowercase())
                    .collect::<Vec<_>>()
                    .join(",")
            }
        } else {
            "".to_string()
        };

        let year = reference
            .issued
            .as_ref()
            .and_then(|d| d.year_value())
            .map(|y| y.to_string())
            .unwrap_or_default();

        format!("{}:{}", author_key, year)
    }

    /// Check if primary contributors (authors/editors) match between two references.
    fn contributors_match(&self, prev: &Reference, current: &Reference) -> bool {
        // TODO: This should ideally check the *primary* contributor variables as defined
        // by the style's substitution logic (e.g., Author -> Editor -> Title).
        // For now, we'll just check names for simplification.

        let prev_contributors = self.get_primary_contributors(prev);
        let curr_contributors = self.get_primary_contributors(current);

        match (prev_contributors, curr_contributors) {
            (Some(p), Some(c)) => p == c,
            _ => false,
        }
    }

    /// Get the primary contributors for a reference (currently just Author).
    fn get_primary_contributors<'a>(&self, reference: &'a Reference) -> Option<&'a Vec<Name>> {
        // Simple fallback logic: Author -> Editor -> Translator
        reference
            .author
            .as_ref()
            .or(reference.editor.as_ref())
            .or(reference.translator.as_ref())
    }

    /// Apply the substitution string to the primary contributor component.
    fn apply_author_substitution(&self, proc: &mut ProcTemplate, substitute: &str) {
        if let Some(component) = proc
            .iter_mut()
            .find(|c| matches!(c.template_component, TemplateComponent::Contributor(_)))
        {
            component.value = substitute.to_string();
            // Important: Verify if we need to clear prefix/suffix or not depending on specs
            // Usually suffixes like "." remain, but prefixes might not.
        }
    }

    /// Render the bibliography to a string.
    pub fn render_bibliography(&self) -> String {
        let processed = self.process_references();
        refs_to_string(processed.bibliography)
    }
}

/// Get a unique key for a template component's variable.
///
/// Used to implement the CSL 1.0 "variable-once" rule. Each component type
/// generates a key based on its specific variable (e.g., "contributor:Author",
/// "date:Issued", "title:Primary").
///
/// List components return None because they can contain multiple variables
/// and should not be deduplicated as a whole.
///
/// ## Examples
/// - `Contributor(Author)` → `"contributor:Author"`
/// - `Date(Issued)` → `"date:Issued"`
/// - `Title(ParentSerial)` → `"title:ParentSerial"`
fn get_variable_key(component: &TemplateComponent) -> Option<String> {
    use csln_core::template::*;

    match component {
        TemplateComponent::Contributor(c) => Some(format!("contributor:{:?}", c.contributor)),
        TemplateComponent::Date(d) => Some(format!("date:{:?}", d.date)),
        TemplateComponent::Title(t) => Some(format!("title:{:?}", t.title)),
        TemplateComponent::Number(n) => Some(format!("number:{:?}", n.number)),
        TemplateComponent::Variable(v) => Some(format!("variable:{:?}", v.variable)),
        TemplateComponent::List(_) => None, // Lists contain multiple variables, not deduplicated
        _ => None,                          // Future component types
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{DateVariable, Name};
    use csln_core::options::{
        AndOptions, ContributorConfig, DisplayAsSort, Processing, ShortenListOptions,
    };
    use csln_core::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate, TemplateTitle, TitleType,
        WrapPunctuation,
    };
    use csln_core::{BibliographySpec, CitationSpec, StyleInfo};

    fn make_style() -> Style {
        Style {
            info: StyleInfo {
                title: Some("APA".to_string()),
                id: Some("apa".to_string()),
                ..Default::default()
            },
            options: Some(Config {
                processing: Some(Processing::AuthorDate),
                contributors: Some(ContributorConfig {
                    shorten: Some(ShortenListOptions {
                        min: 3,
                        use_first: 1,
                        ..Default::default()
                    }),
                    and: Some(AndOptions::Symbol),
                    display_as_sort: Some(DisplayAsSort::First),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            citation: Some(CitationSpec {
                options: None,
                template: vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author,
                        form: ContributorForm::Short,
                        name_order: None,
                        delimiter: None,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: TDateVar::Issued,
                        form: DateForm::Year,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                ],
                wrap: Some(WrapPunctuation::Parentheses),
                ..Default::default()
            }),
            bibliography: Some(BibliographySpec {
                options: None,
                template: vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author,
                        form: ContributorForm::Long,
                        name_order: None,
                        delimiter: None,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: TDateVar::Issued,
                        form: DateForm::Year,
                        rendering: Rendering {
                            wrap: Some(WrapPunctuation::Parentheses),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        form: None,
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        overrides: None,
                        ..Default::default()
                    }),
                ],
                ..Default::default()
            }),
            templates: None,
            ..Default::default()
        }
    }

    fn make_bibliography() -> Bibliography {
        let mut bib = HashMap::new();

        bib.insert(
            "kuhn1962".to_string(),
            Reference {
                id: "kuhn1962".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
                title: Some("The Structure of Scientific Revolutions".to_string()),
                issued: Some(DateVariable::year(1962)),
                publisher: Some("University of Chicago Press".to_string()),
                ..Default::default()
            },
        );

        bib
    }

    #[test]
    fn test_process_citation() {
        let style = make_style();
        let bib = make_bibliography();
        let processor = Processor::new(style, bib);

        let citation = Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            }],
        };

        let result = processor.process_citation(&citation).unwrap();
        assert_eq!(result, "(Kuhn, 1962)");
    }

    #[test]
    fn test_render_bibliography() {
        let style = make_style();
        let bib = make_bibliography();
        let processor = Processor::new(style, bib);

        let result = processor.render_bibliography();

        // Check it contains the key parts
        assert!(result.contains("Kuhn"));
        assert!(result.contains("(1962)"));
        assert!(result.contains("_The Structure of Scientific Revolutions_"));
    }

    #[test]
    fn test_disambiguation_hints() {
        let style = make_style();
        let mut bib = make_bibliography();

        // Add another Kuhn 1962 reference to trigger disambiguation
        bib.insert(
            "kuhn1962b".to_string(),
            Reference {
                id: "kuhn1962b".to_string(),
                ref_type: "article-journal".to_string(),
                author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
                title: Some("The Function of Measurement in Modern Physical Science".to_string()),
                issued: Some(DateVariable::year(1962)),
                ..Default::default()
            },
        );

        let processor = Processor::new(style, bib);
        let hints = &processor.hints;

        // Both should have disambiguation condition true
        assert!(hints.get("kuhn1962").unwrap().disamb_condition);
        assert!(hints.get("kuhn1962b").unwrap().disamb_condition);
    }

    #[test]
    fn test_disambiguation_givenname() {
        use csln_core::options::{
            Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
        };

        // Style with add-givenname enabled
        let mut style = make_style();
        style.options = Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
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
            })),
            contributors: Some(ContributorConfig {
                initialize_with: Some(". ".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        });

        let mut bib = HashMap::new();
        bib.insert(
            "smith2020a".to_string(),
            Reference {
                id: "smith2020a".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![Name::new("Smith", "John")]),
                issued: Some(DateVariable::year(2020)),
                ..Default::default()
            },
        );
        bib.insert(
            "smith2020b".to_string(),
            Reference {
                id: "smith2020b".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![Name::new("Smith", "Alice")]),
                issued: Some(DateVariable::year(2020)),
                ..Default::default()
            },
        );

        let processor = Processor::new(style, bib);

        let hints = &processor.hints;

        // Verify hints
        assert!(hints.get("smith2020a").unwrap().expand_given_names);
        assert!(hints.get("smith2020b").unwrap().expand_given_names);
        assert!(!hints.get("smith2020a").unwrap().disamb_condition); // No year suffix

        // Verify output
        let cit_a = processor
            .process_citation(&Citation {
                id: Some("c1".to_string()),
                items: vec![crate::reference::CitationItem {
                    id: "smith2020a".to_string(),
                    ..Default::default()
                }],
            })
            .unwrap();

        let cit_b = processor
            .process_citation(&Citation {
                id: Some("c2".to_string()),
                items: vec![crate::reference::CitationItem {
                    id: "smith2020b".to_string(),
                    ..Default::default()
                }],
            })
            .unwrap();

        // Should expand to "J. Smith" and "A. Smith" (because initialized)
        assert!(cit_a.contains("J. Smith"));
        assert!(cit_b.contains("A. Smith"));
    }

    #[test]
    fn test_disambiguation_add_names() {
        use csln_core::options::{
            Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
        };

        let mut style = make_style();
        style.options = Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
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
                    names: true, // disambiguate-add-names
                    add_givenname: false,
                    year_suffix: true,
                }),
            })),
            contributors: Some(ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 2,
                    use_first: 1,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        let mut bib = HashMap::new();
        // Two works by Smith & Jones and Smith & Brown
        // Both would be "Smith et al. (2020)"
        bib.insert(
            "ref1".to_string(),
            Reference {
                id: "ref1".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![
                    Name::new("Smith", "John"),
                    Name::new("Jones", "Peter"),
                ]),
                issued: Some(DateVariable::year(2020)),
                ..Default::default()
            },
        );
        bib.insert(
            "ref2".to_string(),
            Reference {
                id: "ref2".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![
                    Name::new("Smith", "John"),
                    Name::new("Brown", "Alice"),
                ]),
                issued: Some(DateVariable::year(2020)),
                ..Default::default()
            },
        );

        let processor = Processor::new(style, bib);

        // Verify hints
        assert_eq!(
            processor.hints.get("ref1").unwrap().min_names_to_show,
            Some(2)
        );
        assert_eq!(
            processor.hints.get("ref2").unwrap().min_names_to_show,
            Some(2)
        );

        // Verify output
        let cit_1 = processor
            .process_citation(&Citation {
                id: Some("c1".to_string()),
                items: vec![crate::reference::CitationItem {
                    id: "ref1".to_string(),
                    ..Default::default()
                }],
            })
            .unwrap();

        let cit_2 = processor
            .process_citation(&Citation {
                id: Some("c2".to_string()),
                items: vec![crate::reference::CitationItem {
                    id: "ref2".to_string(),
                    ..Default::default()
                }],
            })
            .unwrap();

        // Should expand to "Smith, Jones" and "Smith, Brown" (no et al. because only 2 names)
        assert!(cit_1.contains("Smith") && cit_1.contains("Jones"));
        assert!(cit_2.contains("Smith") && cit_2.contains("Brown"));
    }

    #[test]
    fn test_disambiguation_combined_expansion() {
        use csln_core::options::{
            Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
        };

        // This test simulates the "Sam Smith & Julie Smith" scenario but with
        // two items that remain ambiguous after name expansion alone.
        // Item 1: [Sam Smith, Julie Smith] 2020 -> "Smith & Smith" (base)
        // Item 2: [Sam Smith, Bob Smith] 2020   -> "Smith & Smith" (base)
        // Both would be "Smith et al." if min=3, but here they collide even as "Smith & Smith".
        // They need both expanded names AND expanded given names.

        let mut style = make_style();
        style.options = Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
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
            })),
            contributors: Some(ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 2,
                    use_first: 1,
                    ..Default::default()
                }),
                initialize_with: Some(". ".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        });

        let mut bib = HashMap::new();
        bib.insert(
            "ref1".to_string(),
            Reference {
                id: "ref1".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![Name::new("Smith", "Sam"), Name::new("Smith", "Julie")]),
                issued: Some(DateVariable::year(2020)),
                ..Default::default()
            },
        );
        bib.insert(
            "ref2".to_string(),
            Reference {
                id: "ref2".to_string(),
                ref_type: "book".to_string(),
                author: Some(vec![Name::new("Smith", "Sam"), Name::new("Smith", "Bob")]),
                issued: Some(DateVariable::year(2020)),
                ..Default::default()
            },
        );

        let processor = Processor::new(style, bib);

        // Verify output
        let cit_1 = processor
            .process_citation(&Citation {
                id: Some("c1".to_string()),
                items: vec![crate::reference::CitationItem {
                    id: "ref1".to_string(),
                    ..Default::default()
                }],
            })
            .unwrap();

        let cit_2 = processor
            .process_citation(&Citation {
                id: Some("c2".to_string()),
                items: vec![crate::reference::CitationItem {
                    id: "ref2".to_string(),
                    ..Default::default()
                }],
            })
            .unwrap();

        // Should expand to "S. Smith & J. Smith" and "S. Smith & B. Smith"
        assert!(
            cit_1.contains("S. Smith") && cit_1.contains("J. Smith"),
            "Output was: {}",
            cit_1
        );
        assert!(
            cit_2.contains("S. Smith") && cit_2.contains("B. Smith"),
            "Output was: {}",
            cit_2
        );
    }

    #[test]
    fn test_apa_titles_config() {
        use csln_core::options::{Config, TitleRendering, TitlesConfig};
        use csln_core::template::{Rendering, TemplateTitle, TitleType};
        use crate::reference::{DateVariable, Name, Reference};

        let config = Config {
            titles: Some(TitlesConfig {
                periodical: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                monograph: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                container_monograph: Some(TitleRendering {
                    emph: Some(true),
                    prefix: Some("In ".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let bib_template = vec![
            TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            TemplateComponent::Title(TemplateTitle {
                title: TitleType::ParentSerial,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            TemplateComponent::Title(TemplateTitle {
                title: TitleType::ParentMonograph,
                rendering: Rendering::default(),
                ..Default::default()
            }),
        ];

        let style = Style {
            options: Some(config),
            bibliography: Some(csln_core::BibliographySpec {
                template: bib_template,
                ..Default::default()
            }),
            ..Default::default()
        };

        let references = vec![
            Reference {
                id: "art1".to_string(),
                ref_type: "article-journal".to_string(),
                title: Some("A Title".to_string()),
                container_title: Some("Nature".to_string()),
                ..Default::default()
            },
            Reference {
                id: "ch1".to_string(),
                ref_type: "chapter".to_string(),
                title: Some("A Chapter".to_string()),
                container_title: Some("A Book".to_string()),
                ..Default::default()
            },
            Reference {
                id: "bk1".to_string(),
                ref_type: "book".to_string(),
                title: Some("A Global Book".to_string()),
                ..Default::default()
            },
        ];

        let processor = Processor::new(
            style,
            references
                .into_iter()
                .map(|r| (r.id.clone(), r))
                .collect(),
        );

        let res = processor.render_bibliography();
        
        // Book Case: Primary title -> monograph category -> Italic, No "In "
        assert!(res.contains("_A Global Book_"), "Book title should be italicized: {}", res);
        assert!(!res.contains("In _A Global Book_"), "Book title should NOT have 'In ' prefix: {}", res);

        // Journal Article Case: ParentSerial -> periodical category -> Italic, No "In "
        assert!(res.contains("_Nature_"), "Journal title should be italicized: {}", res);
        assert!(!res.contains("In _Nature_"), "Journal title should NOT have 'In ' prefix: {}", res);

        // Chapter Case: ParentMonograph -> container_monograph category -> Italic, WITH "In "
        assert!(res.contains("In _A Book_"), "Chapter container title should have 'In ' prefix: {}", res);
    }
}
