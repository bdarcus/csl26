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

    /// Check if this is a numeric style with integral mode.
    fn should_render_author_year_for_numeric_integral(
        &self,
        mode: &csln_core::citation::CitationMode,
    ) -> bool {
        if !matches!(mode, csln_core::citation::CitationMode::Integral) {
            return false;
        }

        let is_numeric = self
            .config
            .processing
            .as_ref()
            .map(|p| matches!(p, csln_core::options::Processing::Numeric))
            .unwrap_or(false);

        if !is_numeric {
            return false;
        }

        // If the style provides an explicit integral template, use it instead of the hardcoded default.
        let has_explicit_integral = self
            .style
            .citation
            .as_ref()
            .map(|cs| cs.integral.is_some())
            .unwrap_or(false);

        !has_explicit_integral
    }

    /// Ensure suffix has proper spacing (add space if suffix doesn't start with
    /// punctuation and isn't empty).
    fn ensure_suffix_spacing(&self, suffix: &str) -> String {
        if suffix.is_empty() {
            String::new()
        } else if suffix.starts_with(char::is_whitespace)
            || suffix.starts_with(',')
            || suffix.starts_with(';')
            || suffix.starts_with('.')
        {
            // Already has leading space or punctuation
            suffix.to_string()
        } else {
            // Add space before suffix to separate from content
            format!(" {}", suffix)
        }
    }

    /// Render author + citation number for numeric integral citations.
    fn render_author_year_for_numeric_integral_with_format<F>(
        &self,
        reference: &Reference,
        item: &crate::reference::CitationItem,
        citation_number: usize,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context: RenderContext::Citation,
            mode: csln_core::citation::CitationMode::Integral,
            visibility: item.visibility,
            locator: item.locator.as_deref(),
            locator_label: item.label.clone(),
        };

        // Render author in short form
        let author_part = if let Some(authors) = reference.author() {
            let mode = self
                .config
                .multilingual
                .as_ref()
                .and_then(|m| m.name_mode.as_ref());
            let preferred_script = self
                .config
                .multilingual
                .as_ref()
                .and_then(|m| m.preferred_script.as_ref());
            let locale_str = &self.locale.locale;

            let names_vec = crate::values::resolve_multilingual_name(
                &authors,
                mode,
                preferred_script,
                locale_str,
            );
            fmt.text(&crate::values::format_contributors_short(
                &names_vec, &options,
            ))
        } else {
            String::new()
        };

        // Format: "Author [N]"
        if !author_part.is_empty() {
            format!("{} [{}]", author_part, citation_number)
        } else {
            // Fallback: just citation number if no author
            format!("[{}]", citation_number)
        }
    }

    #[allow(dead_code)]
    fn render_author_year_for_numeric_integral(
        &self,
        reference: &Reference,
        item: &crate::reference::CitationItem,
        citation_number: usize,
    ) -> String {
        self.render_author_year_for_numeric_integral_with_format::<crate::render::plain::PlainText>(
            reference,
            item,
            citation_number,
        )
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

        // For numeric styles with integral mode, render author-year instead
        let use_author_year = self.should_render_author_year_for_numeric_integral(mode);

        for item in items {
            let reference = self
                .bibliography
                .get(&item.id)
                .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;

            if use_author_year {
                // Numeric integral: render author + citation number
                let citation_number = self.get_or_assign_citation_number(&item.id);
                let item_str = self.render_author_year_for_numeric_integral_with_format::<F>(
                    reference,
                    item,
                    citation_number,
                );
                if !item_str.is_empty() {
                    let prefix = item.prefix.as_deref().unwrap_or("");
                    let suffix = item.suffix.as_deref().unwrap_or("");

                    let formatted_prefix =
                        if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
                            format!("{} ", prefix)
                        } else {
                            prefix.to_string()
                        };

                    let content = if !prefix.is_empty() || !suffix.is_empty() {
                        let spaced_suffix = self.ensure_suffix_spacing(suffix);
                        fmt.affix(&formatted_prefix, item_str, &spaced_suffix)
                    } else {
                        item_str
                    };
                    rendered_items.push(fmt.citation(vec![item.id.clone()], content));
                }
            } else {
                // Standard rendering: use template with citation number
                let citation_number = self.get_or_assign_citation_number(&item.id);

                // Check for integral mode specific template
                let integral_spec = if matches!(mode, csln_core::citation::CitationMode::Integral) {
                    self.style
                        .citation
                        .as_ref()
                        .and_then(|cs| cs.integral.as_ref())
                } else {
                    None
                };

                let (effective_template, effective_delim) = if let Some(spec) = integral_spec {
                    if let Some(t) = spec.template.as_ref() {
                        (t.as_slice(), spec.delimiter.as_deref().unwrap_or(" "))
                    } else {
                        (template, intra_delimiter)
                    }
                } else {
                    (template, intra_delimiter)
                };

                if let Some(proc) = self.process_template_with_number_with_format::<F>(
                    reference,
                    effective_template,
                    RenderContext::Citation,
                    mode.clone(),
                    item.visibility,
                    citation_number,
                    item.locator.as_deref(),
                    item.label.clone(),
                ) {
                    let item_str = crate::render::citation::citation_to_string_with_format::<F>(
                        &proc,
                        None,
                        None,
                        None,
                        Some(effective_delim),
                    );
                    if !item_str.is_empty() {
                        let prefix = item.prefix.as_deref().unwrap_or("");
                        let suffix = item.suffix.as_deref().unwrap_or("");

                        let formatted_prefix =
                            if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
                                format!("{} ", prefix)
                            } else {
                                prefix.to_string()
                            };

                        let content = if !prefix.is_empty() || !suffix.is_empty() {
                            let spaced_suffix = self.ensure_suffix_spacing(suffix);
                            fmt.affix(&formatted_prefix, item_str, &spaced_suffix)
                        } else {
                            item_str
                        };
                        rendered_items.push(fmt.citation(vec![item.id.clone()], content));
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
                    let last_author_key = self
                        .bibliography
                        .get(&last_item.id)
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
                    author_key == last_author_key && item.prefix.is_none() && !author_key.is_empty()
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
            let first_item = group[0];
            let first_ref = self
                .bibliography
                .get(&first_item.id)
                .ok_or_else(|| ProcessorError::ReferenceNotFound(first_item.id.clone()))?;

            // If we have an explicit integral template and we're in integral mode,
            // we should try to use it.
            let integral_spec = if matches!(mode, csln_core::citation::CitationMode::Integral) {
                self.style
                    .citation
                    .as_ref()
                    .and_then(|cs| cs.integral.as_ref())
            } else {
                None
            };

            if let Some(spec) = integral_spec {
                if let Some(template) = spec.template.as_ref() {
                    // Narrative mode with explicit template (e.g., APA 7th)
                    let citation_number = self.get_or_assign_citation_number(&first_item.id);
                    if let Some(proc) = self.process_template_with_number_with_format::<F>(
                        first_ref,
                        template,
                        RenderContext::Citation,
                        mode.clone(),
                        first_item.visibility,
                        citation_number,
                        first_item.locator.as_deref(),
                        first_item.label.clone(),
                    ) {
                        // Use integral-specific delimiter, defaulting to space for narrative
                        let integral_delimiter = spec.delimiter.as_deref().unwrap_or(" ");
                        let item_str = crate::render::citation::citation_to_string_with_format::<F>(
                            &proc,
                            None,
                            None,
                            None,
                            Some(integral_delimiter),
                        );

                        let ids: Vec<String> = group.iter().map(|item| item.id.clone()).collect();
                        let prefix = first_item.prefix.as_deref().unwrap_or("");
                        let suffix = first_item.suffix.as_deref().unwrap_or("");

                        let formatted_prefix =
                            if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
                                format!("{} ", prefix)
                            } else {
                                prefix.to_string()
                            };

                        let content = if !prefix.is_empty() || !suffix.is_empty() {
                            let spaced_suffix = self.ensure_suffix_spacing(suffix);
                            fmt.affix(&formatted_prefix, item_str, &spaced_suffix)
                        } else {
                            item_str
                        };

                        rendered_groups.push(fmt.citation(ids, content));
                        continue;
                    }
                }
            }

            // Fallback to default hardcoded grouping (or if no integral template)
            let author_part =
                self.render_author_for_grouping_with_format::<F>(first_ref, template, mode);

            let mut year_parts = Vec::new();
            for item in &group {
                let reference = self
                    .bibliography
                    .get(&item.id)
                    .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;

                let year_part = self.render_year_for_grouping_with_format::<F>(reference);
                if !year_part.is_empty() {
                    let suffix = item.suffix.as_deref().unwrap_or("");
                    if !suffix.is_empty() {
                        let spaced_suffix = self.ensure_suffix_spacing(suffix);
                        year_parts.push(fmt.affix("", year_part, &spaced_suffix));
                    } else {
                        year_parts.push(year_part);
                    }
                }
            }

            let prefix = first_item.prefix.as_deref().unwrap_or("");
            if !author_part.is_empty() && !year_parts.is_empty() {
                let joined_years = year_parts.join(intra_delimiter);
                // Format based on citation mode:
                // Integral: "Kuhn (1962a, 1962b)" - years in parentheses
                // NonIntegral: "Kuhn, 1962a, 1962b" - no inner parens (outer wrap adds them)
                let content = match mode {
                    csln_core::citation::CitationMode::Integral => {
                        // Check for visibility overrides
                        if matches!(
                            first_item.visibility,
                            csln_core::citation::ItemVisibility::SuppressAuthor
                        ) {
                            // Should theoretically not happen in narrative mode, but handle gracefully
                            format!("({})", joined_years)
                        } else {
                            // Default narrative: Kuhn (1962)
                            format!("{} ({})", author_part, joined_years)
                        }
                    }
                    csln_core::citation::CitationMode::NonIntegral => {
                        if matches!(
                            first_item.visibility,
                            csln_core::citation::ItemVisibility::SuppressAuthor
                        ) {
                            // Parenthetical SuppressAuthor: 1962
                            joined_years
                        } else {
                            // Default parenthetical: Kuhn, 1962
                            if self.config.punctuation_in_quote
                                && intra_delimiter.starts_with(',')
                                && (author_part.ends_with('"') || author_part.ends_with('\u{201D}'))
                            {
                                let is_curly = author_part.ends_with('\u{201D}');
                                let mut fixed_author = author_part.clone();
                                fixed_author.pop();
                                format!(
                                    "{},{}{}{}",
                                    fixed_author,
                                    if is_curly { '\u{201D}' } else { '"' },
                                    &intra_delimiter[1..],
                                    joined_years
                                )
                            } else {
                                format!("{}{}{}", author_part, intra_delimiter, joined_years)
                            }
                        }
                    }
                };
                let ids: Vec<String> = group.iter().map(|item| item.id.clone()).collect();

                let formatted_prefix =
                    if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
                        format!("{} ", prefix)
                    } else {
                        prefix.to_string()
                    };

                rendered_groups.push(fmt.citation(ids, fmt.affix(&formatted_prefix, content, "")));
            } else if !author_part.is_empty() {
                let ids: Vec<String> = group.iter().map(|item| item.id.clone()).collect();

                let formatted_prefix =
                    if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
                        format!("{} ", prefix)
                    } else {
                        prefix.to_string()
                    };

                rendered_groups
                    .push(fmt.citation(ids, fmt.affix(&formatted_prefix, author_part, "")));
            } else if !year_parts.is_empty() {
                // Year-only case (SuppressAuthor)
                let content = year_parts.join(intra_delimiter);
                let ids: Vec<String> = group.iter().map(|item| item.id.clone()).collect();

                let formatted_prefix =
                    if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
                        format!("{} ", prefix)
                    } else {
                        prefix.to_string()
                    };

                rendered_groups.push(fmt.citation(ids, fmt.affix(&formatted_prefix, content, "")));
            }
        }

        Ok(rendered_groups)
    }

    /// Render just the author part for citation grouping.
    fn render_author_for_grouping_with_format<F>(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context: RenderContext::Citation,
            mode: mode.clone(),
            visibility: csln_core::citation::ItemVisibility::Default,
            locator: None,
            locator_label: None,
        };

        // Try to use the first component from the template if it's a contributor or title.
        // This ensures substitution, shortening, and mode-dependent conjunctions are respected.
        if let Some(comp) = template.first() {
            if matches!(
                comp,
                TemplateComponent::Contributor(_) | TemplateComponent::Title(_)
            ) {
                let hints = self
                    .hints
                    .get(&reference.id().unwrap_or_default())
                    .cloned()
                    .unwrap_or_default();
                if let Some(vals) = comp.values::<F>(reference, &hints, &options) {
                    if !vals.value.is_empty() {
                        return vals.value;
                    }
                }
            }
        }

        // Fallback for cases where first component isn't suitable or returned empty
        if let Some(authors) = reference.author() {
            let mode = self
                .config
                .multilingual
                .as_ref()
                .and_then(|m| m.name_mode.as_ref());
            let preferred_script = self
                .config
                .multilingual
                .as_ref()
                .and_then(|m| m.preferred_script.as_ref());
            let locale_str = &self.locale.locale;

            let names_vec = crate::values::resolve_multilingual_name(
                &authors,
                mode,
                preferred_script,
                locale_str,
            );
            F::default().text(&crate::values::format_contributors_short(
                &names_vec, &options,
            ))
        } else {
            String::new()
        }
    }
    #[allow(dead_code)]
    fn render_author_for_grouping(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        mode: &csln_core::citation::CitationMode,
    ) -> String {
        self.render_author_for_grouping_with_format::<crate::render::plain::PlainText>(
            reference, template, mode,
        )
    }

    /// Render just the year part (with suffix) for citation grouping.
    fn render_year_for_grouping_with_format<F>(&self, reference: &Reference) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
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
                    crate::values::int_to_letter(hints.group_index as u32).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            return fmt.text(&format!("{}{}", year, suffix));
        }
        String::new()
    }

    #[allow(dead_code)]
    fn render_year_for_grouping(&self, reference: &Reference) -> String {
        self.render_year_for_grouping_with_format::<crate::render::plain::PlainText>(reference)
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
        self.process_bibliography_entry_with_format::<crate::render::plain::PlainText>(
            reference,
            entry_number,
        )
    }

    /// Process a bibliography entry with specific format.
    pub fn process_bibliography_entry_with_format<F>(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let bib_spec = self.style.bibliography.as_ref()?;

        // Resolve default template (handles preset vs explicit)
        let default_template = bib_spec.resolve_template()?;

        // Determine effective template (override or default)
        let ref_type = reference.ref_type();
        let template = if let Some(type_templates) = &bib_spec.type_templates {
            let mut matched_template = None;
            for (selector, t) in type_templates {
                if selector.matches(&ref_type) {
                    matched_template = Some(t.clone());
                    break;
                }
            }
            matched_template.unwrap_or(default_template)
        } else {
            default_template
        };

        let template_ref = &template;

        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context: RenderContext::Bibliography,
            mode: csln_core::citation::CitationMode::NonIntegral,
            visibility: csln_core::citation::ItemVisibility::Default,
            locator: None,
            locator_label: None,
        };

        self.process_template_with_number_internal_with_format::<F>(
            reference,
            template_ref,
            options,
            entry_number,
        )
    }

    /// Process a template for a reference with citation number.
    #[allow(clippy::too_many_arguments)]
    pub fn process_template_with_number(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        context: RenderContext,
        mode: csln_core::citation::CitationMode,
        visibility: csln_core::citation::ItemVisibility,
        citation_number: usize,
        locator: Option<&str>,
        locator_label: Option<csln_core::citation::LocatorType>,
    ) -> Option<ProcTemplate> {
        self.process_template_with_number_with_format::<crate::render::plain::PlainText>(
            reference,
            template,
            context,
            mode,
            visibility,
            citation_number,
            locator,
            locator_label,
        )
    }

    /// Process a template for a reference with citation number and specific format.
    #[allow(clippy::too_many_arguments)]
    pub fn process_template_with_number_with_format<F>(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        context: RenderContext,
        mode: csln_core::citation::CitationMode,
        visibility: csln_core::citation::ItemVisibility,
        citation_number: usize,
        locator: Option<&str>,
        locator_label: Option<csln_core::citation::LocatorType>,
    ) -> Option<ProcTemplate>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let options = RenderOptions {
            config: self.config,
            locale: self.locale,
            context,
            mode,
            visibility,
            locator,
            locator_label,
        };
        self.process_template_with_number_internal_with_format::<F>(
            reference,
            template,
            options,
            citation_number,
        )
    }

    #[allow(dead_code)]
    fn process_template_with_number_internal(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        options: RenderOptions<'_>,
        citation_number: usize,
    ) -> Option<ProcTemplate> {
        self.process_template_with_number_internal_with_format::<crate::render::plain::PlainText>(
            reference,
            template,
            options,
            citation_number,
        )
    }

    fn process_template_with_number_internal_with_format<F>(
        &self,
        reference: &Reference,
        template: &[TemplateComponent],
        options: RenderOptions<'_>,
        citation_number: usize,
    ) -> Option<ProcTemplate>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
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

                // Extract value from reference using the requested format
                let mut values = component.values::<F>(reference, &hint, &options)?;
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
                    pre_formatted: values.pre_formatted,
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
        self.apply_author_substitution_with_format::<crate::render::plain::PlainText>(
            proc, substitute,
        );
    }

    /// Apply the substitution string to the primary contributor component with specific format.
    pub fn apply_author_substitution_with_format<F>(
        &self,
        proc: &mut ProcTemplate,
        substitute: &str,
    ) where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        if let Some(component) = proc
            .iter_mut()
            .find(|c| matches!(c.template_component, TemplateComponent::Contributor(_)))
        {
            let fmt = F::default();
            component.value = fmt.text(substitute);
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
            custom: None,
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
            custom: None,
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
            custom: None,
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
