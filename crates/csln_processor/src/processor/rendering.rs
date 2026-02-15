use crate::error::ProcessorError;
use crate::reference::{Bibliography, Reference};
use crate::render::{ProcTemplate, ProcTemplateComponent};
use crate::values::{ComponentValues, ProcHints, RenderContext, RenderOptions};
use csln_core::locale::Locale;
use csln_core::options::Config;
use csln_core::template::TemplateComponent;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

pub struct Renderer<'a> {
    pub style: &'a csln_core::Style,
    pub bibliography: &'a Bibliography,
    pub locale: &'a Locale,
    pub config: &'a Config,
    pub hints: &'a HashMap<String, ProcHints>,
    pub citation_numbers: &'a RefCell<HashMap<String, usize>>,
}

impl<'a> Renderer<'a> {
    pub fn new(
        style: &'a csln_core::Style,
        bibliography: &'a Bibliography,
        locale: &'a Locale,
        config: &'a Config,
        hints: &'a HashMap<String, ProcHints>,
        citation_numbers: &'a RefCell<HashMap<String, usize>>,
    ) -> Self {
        Self {
            style,
            bibliography,
            locale,
            config,
            hints,
            citation_numbers,
        }
    }

    /// Render citation items without grouping.
    pub fn render_ungrouped_citation(
        &self,
        items: &[crate::reference::CitationItem],
        template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
        intra_delimiter: &str,
    ) -> Result<Vec<String>, ProcessorError> {
        self.render_ungrouped_citation_with_format::<crate::render::plain::PlainText>(
            items,
            template,
            mode,
            intra_delimiter,
        )
    }

    pub fn render_ungrouped_citation_with_format<F>(
        &self,
        items: &[crate::reference::CitationItem],
        template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
        intra_delimiter: &str,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let mut rendered_items = Vec::new();
        let fmt = F::default();

        for item in items {
            let reference = self
                .bibliography
                .get(&item.id)
                .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;

            let citation_number = self.get_or_assign_citation_number(&item.id);

            if let Some(proc) = self.process_template_with_number(
                reference,
                template,
                RenderContext::Citation,
                mode.clone(),
                citation_number,
                item.locator.as_deref(),
                item.label.clone(),
            ) {
                let item_str = crate::render::citation::citation_to_string_with_format::<F>(
                    &proc,
                    None,
                    None,
                    None,
                    Some(intra_delimiter),
                );
                if !item_str.is_empty() {
                    let prefix = item.prefix.as_deref().unwrap_or("");
                    let suffix = item.suffix.as_deref().unwrap_or("");
                    if !prefix.is_empty() || !suffix.is_empty() {
                        rendered_items.push(fmt.affix(prefix, item_str, suffix));
                    } else {
                        rendered_items.push(item_str);
                    }
                }
            }
        }

        Ok(rendered_items)
    }

    /// Render citation items with author grouping for author-date styles.
    pub fn render_grouped_citation(
        &self,
        items: &[crate::reference::CitationItem],
        template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
        intra_delimiter: &str,
    ) -> Result<Vec<String>, ProcessorError> {
        self.render_grouped_citation_with_format::<crate::render::plain::PlainText>(
            items,
            template,
            mode,
            intra_delimiter,
        )
    }

    pub fn render_grouped_citation_with_format<F>(
        &self,
        items: &[crate::reference::CitationItem],
        template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
        intra_delimiter: &str,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        use crate::reference::CitationItem;

        // Group adjacent items by author key
        let mut groups: Vec<Vec<&CitationItem>> = Vec::new();

        for item in items {
            let reference = self.bibliography.get(&item.id);
            let author_key = reference
                .and_then(|r| r.author())
                .map(|authors| {
                    authors
                        .to_names_vec()
                        .iter()
                        .map(|a| a.family_or_literal().to_lowercase())
                        .collect::<Vec<_>>()
                        .join("|")
                })
                .unwrap_or_default();

            // Check if this item has the same author as the previous group
            let should_group = if let Some(last_group) = groups.last() {
                if let Some(last_item) = last_group.last() {
                    let last_ref = self.bibliography.get(&last_item.id);
                    let last_key = last_ref
                        .and_then(|r| r.author())
                        .map(|authors| {
                            authors
                                .to_names_vec()
                                .iter()
                                .map(|a| a.family_or_literal().to_lowercase())
                                .collect::<Vec<_>>()
                                .join("|")
                        })
                        .unwrap_or_default();
                    author_key == last_key && item.prefix.is_none() && !author_key.is_empty()
                } else {
                    false
                }
            } else {
                false
            };

            if should_group {
                groups.last_mut().unwrap().push(item);
            } else {
                groups.push(vec![item]);
            }
        }

        let mut rendered_groups = Vec::new();
        let fmt = F::default();

        for group in groups {
            if group.len() == 1 {
                let item = group[0];
                let reference = self
                    .bibliography
                    .get(&item.id)
                    .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;

                let citation_number = self.get_or_assign_citation_number(&item.id);

                if let Some(proc) = self.process_template_with_number(
                    reference,
                    template,
                    RenderContext::Citation,
                    mode.clone(),
                    citation_number,
                    item.locator.as_deref(),
                    item.label.clone(),
                ) {
                    let item_str = crate::render::citation::citation_to_string_with_format::<F>(
                        &proc,
                        None,
                        None,
                        None,
                        None,
                    );
                    if !item_str.is_empty() {
                        let prefix = item.prefix.as_deref().unwrap_or("");
                        let suffix = item.suffix.as_deref().unwrap_or("");
                        if !prefix.is_empty() || !suffix.is_empty() {
                            rendered_groups.push(fmt.affix(prefix, item_str, suffix));
                        } else {
                            rendered_groups.push(item_str);
                        }
                    }
                }
            } else {
                let first_item = group[0];
                let first_ref = self
                    .bibliography
                    .get(&first_item.id)
                    .ok_or_else(|| ProcessorError::ReferenceNotFound(first_item.id.clone()))?;

                let author_part = self.render_author_for_grouping(first_ref, template, mode);

                let mut year_parts = Vec::new();
                for item in &group {
                    let reference = self
                        .bibliography
                        .get(&item.id)
                        .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;

                    let year_part = self.render_year_for_grouping(reference);
                    if !year_part.is_empty() {
                        let suffix = item.suffix.as_deref().unwrap_or("");
                        if !suffix.is_empty() {
                            year_parts.push(fmt.affix("", year_part, suffix));
                        } else {
                            year_parts.push(year_part);
                        }
                    }
                }

                let prefix = first_item.prefix.as_deref().unwrap_or("");
                if !author_part.is_empty() && !year_parts.is_empty() {
                    let joined_years = year_parts.join(", ");
                    // The intra_delimiter is already applied inside citation_to_string for individual items,
                    // but here we are manually assembling Author + Years.
                    let content = format!("{} ({})", author_part, joined_years);
                    rendered_groups.push(fmt.affix(prefix, content, ""));
                } else if !author_part.is_empty() {
                    rendered_groups.push(fmt.affix(prefix, author_part, ""));
                }
            }
        }

        Ok(rendered_groups)
    }

    /// Render just the author part for citation grouping.
    fn render_author_for_grouping(
        &self,
        reference: &Reference,
        _template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
    ) -> String {
        // For grouping, we need the short author form
        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context: RenderContext::Citation,
            mode: mode.clone(),
            locator: None,
            locator_label: None,
        };

        // Use short form for citations
        if let Some(authors) = reference.author() {
            crate::values::format_contributors_short(&authors.to_names_vec(), &options)
        } else {
            String::new()
        }
    }

    /// Render just the year part (with suffix) for citation grouping.
    fn render_year_for_grouping(&self, reference: &Reference) -> String {
        let hints = self
            .hints
            .get(&reference.id().unwrap_or_default())
            .cloned()
            .unwrap_or_default();

        // Format year with disambiguation suffix
        if let Some(issued) = reference.issued() {
            let year = issued.year();
            let suffix = if hints.disamb_condition && hints.group_index > 0 {
                // Check if year suffix is enabled
                let use_suffix = self
                    .config
                    .processing
                    .as_ref()
                    .map(|p| {
                        p.config()
                            .disambiguate
                            .as_ref()
                            .map(|d| d.year_suffix)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                if use_suffix {
                    crate::values::int_to_letter((hints.group_index % 26) as u32)
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            return format!("{}{}", year, suffix);
        }
        String::new()
    }

    /// Get the citation number for a reference, assigning one if not yet cited.
    fn get_or_assign_citation_number(&self, ref_id: &str) -> usize {
        let mut numbers = self.citation_numbers.borrow_mut();
        let next_num = numbers.len() + 1;
        *numbers.entry(ref_id.to_string()).or_insert(next_num)
    }

    /// Process a bibliography entry.
    pub fn process_bibliography_entry(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate> {
        let bib_spec = self.style.bibliography.as_ref()?;

        // Resolve default template (handles preset vs explicit)
        let default_template = bib_spec.resolve_template()?;

        // Determine effective template (override or default)
        let template = if let Some(type_templates) = &bib_spec.type_templates {
            type_templates
                .get(&reference.ref_type())
                .cloned()
                .unwrap_or(default_template)
        } else {
            default_template
        };

        let template_ref = &template;

        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context: RenderContext::Bibliography,
            mode: csln_core::citation::CitationMode::NonIntegral,
            locator: None,
            locator_label: None,
        };

        self.process_template_with_number_internal(reference, template_ref, options, entry_number)
    }

    /// Process a template for a reference with citation number.
    pub fn process_template_with_number(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        context: RenderContext,
        mode: csln_core::citation::CitationMode,
        citation_number: usize,
        locator: Option<&str>,
        locator_label: Option<csln_core::citation::LocatorType>,
    ) -> Option<ProcTemplate> {
        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context,
            mode,
            locator,
            locator_label,
        };
        self.process_template_with_number_internal(reference, template, options, citation_number)
    }

    fn process_template_with_number_internal(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        options: RenderOptions<'_>,
        citation_number: usize,
    ) -> Option<ProcTemplate> {
        let default_hint = ProcHints::default();
        let base_hint = self
            .hints
            .get(&reference.id().unwrap_or_default())
            .unwrap_or(&default_hint);

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
        let mut rendered_vars: HashSet<String> = HashSet::new();

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
                let mut values = component.values(reference, &hint, &options)?;
                if values.value.is_empty() {
                    return None;
                }

                // If whole-entry linking is enabled and this component doesn't have a URL,
                // try to resolve it from global config.
                if values.url.is_none() {
                    if let Some(links) = &options.config.links {
                        use csln_core::options::LinkAnchor;
                        if matches!(links.anchor, Some(LinkAnchor::Entry)) {
                            values.url = crate::values::resolve_url(links, reference);
                        }
                    }
                }

                // Mark variable as rendered for deduplication
                if let Some(key) = var_key {
                    rendered_vars.insert(key);
                }
                // Also mark substituted variable (e.g., title when it replaces author)
                if let Some(sub_key) = &values.substituted_key {
                    rendered_vars.insert(sub_key.clone());
                }

                Some(ProcTemplateComponent {
                    template_component: component.clone(),
                    value: values.value,
                    prefix: values.prefix,
                    suffix: values.suffix,
                    url: values.url,
                    ref_type: Some(reference.ref_type().to_string()),
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

    /// Apply the substitution string to the primary contributor component.
    pub fn apply_author_substitution(&self, proc: &mut ProcTemplate, substitute: &str) {
        if let Some(component) = proc
            .iter_mut()
            .find(|c| matches!(c.template_component, TemplateComponent::Contributor(_)))
        {
            component.value = substitute.to_string();
        }
    }
}

/// Get a unique key for a template component's variable.
///
/// The key includes rendering context (prefix/suffix) to allow the same variable
/// to render multiple times if it appears in semantically different contexts.
/// This enables styles like Chicago that require year after author AND after publisher.
pub fn get_variable_key(component: &TemplateComponent) -> Option<String> {
    use csln_core::template::*;

    // Helper to create context suffix from rendering options
    let context_suffix = |rendering: &Rendering| -> String {
        match (&rendering.prefix, &rendering.suffix) {
            (Some(p), Some(s)) => format!(":{}_{}", p, s),
            (Some(p), None) => format!(":{}", p),
            (None, Some(s)) => format!(":{}", s),
            (None, None) => String::new(),
        }
    };

    match component {
        TemplateComponent::Contributor(c) => {
            let ctx = context_suffix(&c.rendering);
            Some(format!("contributor:{:?}{}", c.contributor, ctx))
        }
        TemplateComponent::Date(d) => {
            let ctx = context_suffix(&d.rendering);
            Some(format!("date:{:?}{}", d.date, ctx))
        }
        TemplateComponent::Variable(v) => {
            let ctx = context_suffix(&v.rendering);
            Some(format!("variable:{:?}{}", v.variable, ctx))
        }
        TemplateComponent::Title(t) => {
            let ctx = context_suffix(&t.rendering);
            Some(format!("title:{:?}{}", t.title, ctx))
        }
        TemplateComponent::Number(n) => {
            let ctx = context_suffix(&n.rendering);
            Some(format!("number:{:?}{}", n.number, ctx))
        }
        TemplateComponent::List(_) => None, // Lists contain multiple variables, not deduplicated
        _ => None,                          // Future component types
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::template::*;

    #[test]
    fn test_variable_key_includes_context() {
        // Date with no prefix/suffix
        let date1 = TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering::default(),
            fallback: None,
            links: None,
            overrides: None,
            _extra: std::collections::HashMap::new(),
        });

        // Same date with prefix
        let date2 = TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                prefix: Some(", ".to_string()),
                ..Default::default()
            },
            fallback: None,
            links: None,
            overrides: None,
            _extra: std::collections::HashMap::new(),
        });

        // Same date with suffix
        let date3 = TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            fallback: None,
            links: None,
            overrides: None,
            _extra: std::collections::HashMap::new(),
        });

        let key1 = get_variable_key(&date1);
        let key2 = get_variable_key(&date2);
        let key3 = get_variable_key(&date3);

        // All three should have different keys due to different contexts
        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key2, key3);

        // Verify the keys include context markers
        assert_eq!(key1, Some("date:Issued".to_string()));
        assert_eq!(key2, Some("date:Issued:, ".to_string()));
        assert_eq!(key3, Some("date:Issued:.".to_string()));
    }
}
