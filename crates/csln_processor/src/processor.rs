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
use crate::reference::{Bibliography, Citation, Reference};
use crate::render::{citation_to_string, refs_to_string, ProcTemplate, ProcTemplateComponent};
use crate::values::{ComponentValues, ProcHints, RenderContext, RenderOptions};
use csln_core::locale::Locale;
use csln_core::options::{Config, Processing, SortKey};
use csln_core::template::TemplateComponent;
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
}

impl Default for Processor {
    fn default() -> Self {
        Self {
            style: Style::default(),
            bibliography: Bibliography::default(),
            locale: Locale::en_us(),
            default_config: Config::default(),
            hints: HashMap::new(),
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
        };

        // Pre-calculate hints for disambiguation
        processor.hints = processor.calculate_hints();
        processor
    }

    /// Get the style configuration.
    fn get_config(&self) -> &Config {
        self.style.options.as_ref().unwrap_or(&self.default_config)
    }

    /// Process all references to get rendered output.
    pub fn process_references(&self) -> ProcessedReferences {
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        
        let bibliography: Vec<ProcTemplate> = sorted_refs
            .iter()
            .filter_map(|reference| self.process_bibliography_entry(reference))
            .collect();

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

        let mut all_items = Vec::new();
        
        for item in &citation.items {
            let reference = self.bibliography
                .get(&item.id)
                .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;
            
            if let Some(proc) = self.process_template(reference, template, RenderContext::Citation) {
                all_items.extend(proc);
            }
        }

        // Determine if we should wrap in parentheses
        let wrap_parens = matches!(
            self.get_config().processing,
            Some(Processing::AuthorDate)
        );

        Ok(citation_to_string(&all_items.into_iter().collect(), wrap_parens))
    }

    /// Process a bibliography entry.
    fn process_bibliography_entry(&self, reference: &Reference) -> Option<ProcTemplate> {
        let bib_spec = self.style.bibliography.as_ref()?;
        self.process_template(reference, &bib_spec.template, RenderContext::Bibliography)
    }

    /// Process a template for a reference.
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
    fn process_template(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        context: RenderContext,
    ) -> Option<ProcTemplate> {
        let config = self.get_config();
        let options = RenderOptions { config, locale: &self.locale, context };
        let default_hint = ProcHints::default();
        let hint = self.hints.get(&reference.id).unwrap_or(&default_hint);

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
                let values = component.values(reference, hint, &options)?;
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
                            let a_author = a.author
                                .as_ref()
                                .and_then(|names| names.first())
                                .map(|n| n.family_or_literal().to_lowercase())
                                .unwrap_or_default();
                            let b_author = b.author
                                .as_ref()
                                .and_then(|names| names.first())
                                .map(|n| n.family_or_literal().to_lowercase())
                                .unwrap_or_default();
                            a_author.cmp(&b_author)
                        });
                    }
                    SortKey::Year => {
                        refs.sort_by(|a, b| {
                            let a_year = a.issued.as_ref().and_then(|d| d.year_value()).unwrap_or(0);
                            let b_year = b.issued.as_ref().and_then(|d| d.year_value()).unwrap_or(0);
                            b_year.cmp(&a_year) // Descending
                        });
                    }
                    SortKey::Title => {
                        refs.sort_by(|a, b| {
                            let a_title = a.title.as_deref().unwrap_or("").to_lowercase();
                            let b_title = b.title.as_deref().unwrap_or("").to_lowercase();
                            a_title.cmp(&b_title)
                        });
                    }
                    _ => {}
                }
            }
        }

        refs
    }

    /// Calculate processing hints for disambiguation.
    fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let refs: Vec<&Reference> = self.bibliography.values().collect();
        let grouped = self.group_references(refs);

        grouped
            .iter()
            .flat_map(|(key, group)| {
                let group_len = group.len();
                group.iter().enumerate().map(move |(i, reference)| {
                    let hint = ProcHints {
                        disamb_condition: group_len > 1,
                        group_index: i + 1,
                        group_length: group_len,
                        group_key: key.clone(),
                    };
                    (reference.id.clone(), hint)
                })
            })
            .collect()
    }

    /// Group references by author-year for disambiguation.
    fn group_references<'a>(&self, references: Vec<&'a Reference>) -> HashMap<String, Vec<&'a Reference>> {
        let mut groups: HashMap<String, Vec<&'a Reference>> = HashMap::new();

        for reference in references {
            let key = self.make_group_key(reference);
            groups.entry(key).or_default().push(reference);
        }

        groups
    }

    /// Create a grouping key for a reference.
    fn make_group_key(&self, reference: &Reference) -> String {
        let author = reference.author
            .as_ref()
            .and_then(|names| names.first())
            .map(|n| n.family_or_literal())
            .unwrap_or("");
        
        let year = reference.issued
            .as_ref()
            .and_then(|d| d.year_value())
            .map(|y| y.to_string())
            .unwrap_or_default();

        format!("{}:{}", author.to_lowercase(), year)
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
        _ => None, // Future component types
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{DateVariable, Name};
    use csln_core::options::{AndOptions, ContributorConfig, DisplayAsSort, ShortenListOptions};
    use csln_core::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar,
        Rendering, TemplateComponent, TemplateContributor, TemplateDate, TemplateTitle,
        TitleType, WrapPunctuation,
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
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: TDateVar::Issued,
                        form: DateForm::Year,
                        rendering: Rendering::default(),
                    }),
                ],
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
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: TDateVar::Issued,
                        form: DateForm::Year,
                        rendering: Rendering {
                            wrap: Some(WrapPunctuation::Parentheses),
                            ..Default::default()
                        },
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        form: None,
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        overrides: None,
                    }),
                ],
            }),
            templates: None,
        }
    }

    fn make_bibliography() -> Bibliography {
        let mut bib = HashMap::new();
        
        bib.insert("kuhn1962".to_string(), Reference {
            id: "kuhn1962".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            issued: Some(DateVariable::year(1962)),
            publisher: Some("University of Chicago Press".to_string()),
            ..Default::default()
        });

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
        bib.insert("kuhn1962b".to_string(), Reference {
            id: "kuhn1962b".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Function of Measurement in Modern Physical Science".to_string()),
            issued: Some(DateVariable::year(1962)),
            ..Default::default()
        });

        let processor = Processor::new(style, bib);
        let hints = &processor.hints;

        // Both should have disambiguation condition true
        assert!(hints.get("kuhn1962").unwrap().disamb_condition);
        assert!(hints.get("kuhn1962b").unwrap().disamb_condition);
    }
}
