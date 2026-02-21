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

pub mod disambiguation;
pub mod document;
pub mod labels;
pub mod matching;
pub mod rendering;
pub mod sorting;

#[cfg(test)]
mod tests;

use crate::error::ProcessorError;
use crate::reference::{Bibliography, Citation, CitationItem, Reference};
use crate::render::{ProcEntry, ProcTemplate};
use crate::values::ProcHints;
use csln_core::Style;
use csln_core::locale::Locale;
use csln_core::options::Config;
use csln_core::template::WrapPunctuation;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use self::disambiguation::Disambiguator;
use self::matching::Matcher;
use self::rendering::Renderer;
use self::sorting::Sorter;

/// The CSLN processor.
///
/// Takes a style, bibliography, and citations, and produces formatted output.
#[derive(Debug)]
pub struct Processor {
    /// The style definition.
    pub style: Style,
    /// The bibliography (references keyed by ID).
    pub bibliography: Bibliography,
    /// The locale for terms and formatting.
    pub locale: Locale,
    /// Default configuration.
    pub default_config: Config,
    /// Pre-calculated processing hints.
    pub hints: HashMap<String, ProcHints>,
    /// Citation numbers assigned to references (for numeric styles).
    pub citation_numbers: RefCell<HashMap<String, usize>>,
    /// IDs of items that were cited in a visible way.
    pub cited_ids: RefCell<HashSet<String>>,
}

impl Default for Processor {
    fn default() -> Self {
        Self {
            style: Style::default(),
            bibliography: Bibliography::default(),
            locale: Locale::en_us(),
            default_config: Config::default(),
            hints: HashMap::new(),
            citation_numbers: RefCell::new(HashMap::new()),
            cited_ids: RefCell::new(HashSet::new()),
        }
    }
}
/// Processed output containing citations and bibliography.
#[derive(Debug, Default)]
pub struct ProcessedReferences {
    /// Rendered bibliography entries.
    pub bibliography: Vec<ProcEntry>,
    /// Rendered citations (if any).
    pub citations: Option<Vec<String>>,
}

impl Processor {
    /// Returns true when style processing mode is note-based.
    fn is_note_style(&self) -> bool {
        self.get_config()
            .processing
            .as_ref()
            .is_some_and(|p| matches!(p, csln_core::options::Processing::Note))
    }

    /// Normalize citation note context for note styles.
    ///
    /// Document/plugin layers should provide explicit `note_number` values.
    /// When missing, this method assigns sequential note numbers in citation order.
    pub fn normalize_note_context(&self, citations: &[Citation]) -> Vec<Citation> {
        if !self.is_note_style() {
            return citations.to_vec();
        }

        let mut next_note = 1_u32;
        citations
            .iter()
            .cloned()
            .map(|mut c| {
                if let Some(n) = c.note_number {
                    if n >= next_note {
                        next_note = n.saturating_add(1);
                    }
                } else {
                    c.note_number = Some(next_note);
                    next_note = next_note.saturating_add(1);
                }
                c
            })
            .collect()
    }

    /// Initialize numeric citation numbers from bibliography insertion order.
    ///
    /// citeproc-js registers all bibliography items before citation rendering in
    /// the oracle workflow, so numeric labels are stable by reference registry
    /// order rather than first-citation order.
    ///
    /// When the style declares an explicit bibliography sort, citation numbers
    /// must follow that sorted bibliography order.
    fn initialize_numeric_citation_numbers(&self) {
        let is_numeric = self
            .get_config()
            .processing
            .as_ref()
            .is_some_and(|p| matches!(p, csln_core::options::Processing::Numeric));
        if !is_numeric {
            return;
        }

        let mut numbers = self.citation_numbers.borrow_mut();
        if !numbers.is_empty() {
            return;
        }

        let ordered_ids: Vec<String> = if let Some(sort_spec) = self
            .style
            .bibliography
            .as_ref()
            .and_then(|b| b.sort.as_ref())
        {
            let sorter = crate::grouping::GroupSorter::new(&self.locale);
            sorter
                .sort_references(self.bibliography.values().collect(), sort_spec)
                .into_iter()
                .filter_map(|reference| reference.id())
                .collect()
        } else {
            self.bibliography.keys().cloned().collect()
        };

        for (index, ref_id) in ordered_ids.into_iter().enumerate() {
            numbers.insert(ref_id.clone(), index + 1);
        }
    }

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
            citation_numbers: RefCell::new(HashMap::new()),
            cited_ids: RefCell::new(HashSet::new()),
        };

        // Pre-calculate hints for disambiguation
        processor.hints = processor.calculate_hints();
        processor
    }

    /// Create a new processor with an existing style, bibliography, and locale.
    /// Used for testing when you already have loaded components.
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
    pub fn get_config(&self) -> &Config {
        self.style.options.as_ref().unwrap_or(&self.default_config)
    }

    /// Get merged config for citation context.
    ///
    /// Combines global options with citation-specific overrides.
    pub fn get_citation_config(&self) -> std::borrow::Cow<'_, Config> {
        let base = self.get_config();
        match self
            .style
            .citation
            .as_ref()
            .and_then(|c| c.options.as_ref())
        {
            Some(cite_opts) => std::borrow::Cow::Owned(Config::merged(base, cite_opts)),
            None => std::borrow::Cow::Borrowed(base),
        }
    }

    /// Get merged config for bibliography context.
    ///
    /// Combines global options with bibliography-specific overrides.
    pub fn get_bibliography_config(&self) -> std::borrow::Cow<'_, Config> {
        let base = self.get_config();
        match self
            .style
            .bibliography
            .as_ref()
            .and_then(|b| b.options.as_ref())
        {
            Some(bib_opts) => std::borrow::Cow::Owned(Config::merged(base, bib_opts)),
            None => std::borrow::Cow::Borrowed(base),
        }
    }

    /// Process all references to get rendered output.
    pub fn process_references(&self) -> ProcessedReferences {
        self.initialize_numeric_citation_numbers();
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        let mut bibliography: Vec<ProcEntry> = Vec::new();
        let mut prev_reference: Option<&Reference> = None;

        let bib_config = self.get_config().bibliography.as_ref();
        let substitute = bib_config.and_then(|c| c.subsequent_author_substitute.as_ref());

        for (index, reference) in sorted_refs.iter().enumerate() {
            // For numeric styles, use the citation number assigned when first cited.
            // For other styles, use position in sorted bibliography.
            let ref_id = reference.id().unwrap_or_default();
            let entry_number = self
                .citation_numbers
                .borrow()
                .get(&ref_id)
                .copied()
                .unwrap_or(index + 1);
            if let Some(mut proc) = self.process_bibliography_entry(reference, entry_number) {
                // Apply subsequent author substitution if enabled
                if let Some(sub_string) = substitute
                    && let Some(prev) = prev_reference
                {
                    // Check if primary contributor matches
                    if self.contributors_match(prev, reference) {
                        let bib_config = self.get_bibliography_config();
                        let renderer = Renderer::new(
                            &self.style,
                            &self.bibliography,
                            &self.locale,
                            &bib_config,
                            &self.hints,
                            &self.citation_numbers,
                        );
                        renderer.apply_author_substitution(&mut proc, sub_string);
                    }
                }

                bibliography.push(ProcEntry {
                    id: ref_id.clone(),
                    template: proc,
                    metadata: self.extract_metadata(reference),
                });
                prev_reference = Some(reference);
            }
        }

        ProcessedReferences {
            bibliography,
            citations: None,
        }
    }

    /// Extract basic metadata for interactivity.
    fn extract_metadata(&self, reference: &Reference) -> crate::render::format::ProcEntryMetadata {
        use crate::render::format::ProcEntryMetadata;
        use crate::values::RenderOptions;

        let options = RenderOptions {
            config: self.get_config(),
            locale: &self.locale,
            context: crate::values::RenderContext::Bibliography,
            mode: csln_core::citation::CitationMode::NonIntegral,
            suppress_author: false,
            locator: None,
            locator_label: None,
        };

        ProcEntryMetadata {
            author: reference
                .author()
                .map(|a| crate::values::format_contributors_short(&a.to_names_vec(), &options)),
            year: reference.issued().map(|i| i.year().to_string()),
            title: reference.title().map(|t| t.to_string()),
        }
    }

    /// Process a single citation.
    pub fn process_citation(&self, citation: &Citation) -> Result<String, ProcessorError> {
        self.process_citation_with_format::<crate::render::plain::PlainText>(citation)
    }

    /// Process a bibliography entry.
    pub fn process_bibliography_entry(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate> {
        // Use bibliography-specific merged config
        let bib_config = self.get_bibliography_config();

        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            &bib_config,
            &self.hints,
            &self.citation_numbers,
        );
        renderer.process_bibliography_entry(reference, entry_number)
    }

    /// Sort references according to style instructions.
    pub fn sort_references<'a>(&self, references: Vec<&'a Reference>) -> Vec<&'a Reference> {
        // Use global bibliography sort spec if present
        if let Some(sort_spec) = self
            .style
            .bibliography
            .as_ref()
            .and_then(|b| b.sort.as_ref())
        {
            let sorter = crate::grouping::GroupSorter::new(&self.locale);
            return sorter.sort_references(references, sort_spec);
        }

        let sorter = Sorter::new(self.get_config(), &self.locale);
        sorter.sort_references(references)
    }

    /// Sort citation items according to style instructions.
    pub fn sort_citation_items(
        &self,
        items: Vec<CitationItem>,
        spec: &csln_core::CitationSpec,
    ) -> Vec<CitationItem> {
        if let Some(sort_spec) = &spec.sort {
            let mut items_with_refs: Vec<(CitationItem, &Reference)> = items
                .into_iter()
                .filter_map(|item| self.bibliography.get(&item.id).map(|r| (item, r)))
                .collect();

            let sorter = crate::grouping::GroupSorter::new(&self.locale);
            items_with_refs.sort_by(|a, b| {
                for sort_key in &sort_spec.template {
                    let cmp = sorter.compare_by_key(a.1, b.1, sort_key);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
                std::cmp::Ordering::Equal
            });

            return items_with_refs.into_iter().map(|(item, _)| item).collect();
        }
        items
    }

    /// Calculate processing hints for disambiguation.
    pub fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let cite_config = self.get_citation_config();
        let config = cite_config.as_ref();

        // Use global bibliography sort spec if present for year-suffix sorting
        let bib_sort = self
            .style
            .bibliography
            .as_ref()
            .and_then(|b| b.sort.as_ref());

        let disambiguator = if let Some(sort_spec) = bib_sort {
            Disambiguator::with_group_sort(&self.bibliography, config, &self.locale, sort_spec)
        } else {
            Disambiguator::new(&self.bibliography, config, &self.locale)
        };

        disambiguator.calculate_hints()
    }

    /// Check if primary contributors (authors/editors) match between two references.
    pub fn contributors_match(&self, prev: &Reference, current: &Reference) -> bool {
        let matcher = Matcher::new(&self.style, &self.default_config);
        matcher.contributors_match(prev, current)
    }

    /// Apply the substitution string to the primary contributor component.
    pub fn apply_author_substitution(&self, proc: &mut ProcTemplate, substitute: &str) {
        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            self.get_config(),
            &self.hints,
            &self.citation_numbers,
        );
        renderer.apply_author_substitution(proc, substitute);
    }

    /// Render the bibliography to a string using a specific format.
    pub fn render_bibliography_with_format<F>(&self) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        self.initialize_numeric_citation_numbers();
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        let mut bibliography: Vec<ProcEntry> = Vec::new();
        let mut prev_reference: Option<&Reference> = None;

        let bib_config = self.get_config().bibliography.as_ref();
        let substitute = bib_config.and_then(|c| c.subsequent_author_substitute.as_ref());

        for (index, reference) in sorted_refs.iter().enumerate() {
            let ref_id = reference.id().unwrap_or_default();
            let entry_number = self
                .citation_numbers
                .borrow()
                .get(&ref_id)
                .copied()
                .unwrap_or(index + 1);

            if let Some(mut proc) =
                self.process_bibliography_entry_with_format::<F>(reference, entry_number)
            {
                if let Some(sub_string) = substitute
                    && let Some(prev) = prev_reference
                    && self.contributors_match(prev, reference)
                {
                    let bib_config = self.get_bibliography_config();
                    let renderer = Renderer::new(
                        &self.style,
                        &self.bibliography,
                        &self.locale,
                        &bib_config,
                        &self.hints,
                        &self.citation_numbers,
                    );
                    renderer.apply_author_substitution_with_format::<F>(&mut proc, sub_string);
                }

                bibliography.push(ProcEntry {
                    id: ref_id.clone(),
                    template: proc,
                    metadata: self.extract_metadata(reference),
                });
                prev_reference = Some(reference);
            }
        }

        crate::render::refs_to_string_with_format::<F>(bibliography)
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
        // Use bibliography-specific merged config
        let bib_config = self.get_bibliography_config();

        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            &bib_config,
            &self.hints,
            &self.citation_numbers,
        );
        renderer.process_bibliography_entry_with_format::<F>(reference, entry_number)
    }

    /// Render a citation to a string using a specific format.
    pub fn process_citation_with_format<F>(
        &self,
        citation: &Citation,
    ) -> Result<String, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        self.initialize_numeric_citation_numbers();
        // Track cited IDs
        for item in &citation.items {
            self.cited_ids.borrow_mut().insert(item.id.clone());
        }

        // Resolve the effective citation spec
        let default_spec = csln_core::CitationSpec::default();
        let effective_spec = self
            .style
            .citation
            .as_ref()
            .map(|cs| cs.resolve_for_mode(&citation.mode))
            .unwrap_or(std::borrow::Cow::Borrowed(&default_spec));

        let template_vec = effective_spec.resolve_template().unwrap_or_default();
        let template = template_vec.as_slice();

        // Sort items if sort spec is present
        let sorted_items = self.sort_citation_items(citation.items.clone(), &effective_spec);

        let intra_delimiter = effective_spec.delimiter.as_deref().unwrap_or(", ");
        let renderer_delimiter = if intra_delimiter == "none" || intra_delimiter.is_empty() {
            ""
        } else {
            intra_delimiter
        };

        let inter_delimiter = effective_spec
            .multi_cite_delimiter
            .as_deref()
            .unwrap_or("; ");

        let cite_config = self.get_citation_config();
        let processing = cite_config.processing.clone().unwrap_or_default();
        let is_author_date = !matches!(
            processing,
            csln_core::options::Processing::Numeric | csln_core::options::Processing::Label(_)
        );
        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            &cite_config,
            &self.hints,
            &self.citation_numbers,
        );

        // Process group components
        let rendered_groups = if is_author_date {
            renderer.render_grouped_citation_with_format::<F>(
                &sorted_items,
                template,
                &citation.mode,
                renderer_delimiter,
                citation.suppress_author,
            )?
        } else {
            renderer.render_ungrouped_citation_with_format::<F>(
                &sorted_items,
                template,
                &citation.mode,
                renderer_delimiter,
                citation.suppress_author,
            )?
        };

        let fmt = F::default();
        let content = fmt.join(rendered_groups, inter_delimiter);

        // Apply citation-level prefix/suffix from input
        let citation_prefix = citation.prefix.as_deref().unwrap_or("");
        let citation_suffix = citation.suffix.as_deref().unwrap_or("");

        // Ensure proper spacing for prefix/suffix
        let formatted_prefix =
            if !citation_prefix.is_empty() && !citation_prefix.ends_with(char::is_whitespace) {
                format!("{} ", citation_prefix)
            } else {
                citation_prefix.to_string()
            };

        let formatted_suffix =
            if !citation_suffix.is_empty() && !citation_suffix.starts_with(char::is_whitespace) {
                format!(" {}", citation_suffix)
            } else {
                citation_suffix.to_string()
            };

        let output = if !citation_prefix.is_empty() || !citation_suffix.is_empty() {
            fmt.affix(&formatted_prefix, content, &formatted_suffix)
        } else {
            content
        };

        // Get wrap/prefix/suffix from citation spec
        let wrap = effective_spec
            .wrap
            .as_ref()
            .unwrap_or(&WrapPunctuation::None);
        let spec_prefix = effective_spec.prefix.as_deref().unwrap_or("");
        let spec_suffix = effective_spec.suffix.as_deref().unwrap_or("");

        // For integral (narrative) citations, don't apply wrapping
        // (they're part of the narrative text, not parenthetical)
        let wrapped = if matches!(citation.mode, csln_core::citation::CitationMode::Integral) {
            // Integral mode: skip wrapping, apply only prefix/suffix
            if !spec_prefix.is_empty() || !spec_suffix.is_empty() {
                fmt.affix(spec_prefix, output, spec_suffix)
            } else {
                output
            }
        } else if *wrap != WrapPunctuation::None {
            // Non-integral mode: apply wrap
            fmt.wrap_punctuation(wrap, output)
        } else if !spec_prefix.is_empty() || !spec_suffix.is_empty() {
            fmt.affix(spec_prefix, output, spec_suffix)
        } else {
            output
        };

        Ok(fmt.finish(wrapped))
    }

    /// Render multiple citations in order with note-context normalization.
    pub fn process_citations(&self, citations: &[Citation]) -> Result<Vec<String>, ProcessorError> {
        self.process_citations_with_format::<crate::render::plain::PlainText>(citations)
    }

    /// Render multiple citations in order with note-context normalization.
    pub fn process_citations_with_format<F>(
        &self,
        citations: &[Citation],
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let normalized = self.normalize_note_context(citations);
        normalized
            .iter()
            .map(|c| self.process_citation_with_format::<F>(c))
            .collect()
    }

    /// Render the bibliography to a string.
    pub fn render_bibliography(&self) -> String {
        self.render_bibliography_with_format::<crate::render::plain::PlainText>()
    }

    /// Render the bibliography with grouping for uncited (nocite) items.
    ///
    /// If `style.bibliography.groups` is defined, uses configurable grouping
    /// with per-group sorting. Otherwise, falls back to hardcoded cited/uncited
    /// grouping for backward compatibility.
    pub fn render_grouped_bibliography_with_format<F>(&self) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let processed = self.process_references();

        // Check if style defines custom groups
        if let Some(bib_spec) = &self.style.bibliography
            && let Some(groups) = &bib_spec.groups
        {
            return self.render_with_custom_groups::<F>(&processed.bibliography, groups);
        }

        // Fallback to hardcoded cited/uncited grouping
        self.render_with_legacy_grouping::<F>(&processed.bibliography)
    }

    fn resolve_group_heading(&self, heading: &csln_core::GroupHeading) -> Option<String> {
        match heading {
            csln_core::GroupHeading::Literal { literal } => Some(literal.clone()),
            csln_core::GroupHeading::Term { term, form } => self
                .locale
                .general_term(term, form.unwrap_or(csln_core::locale::TermForm::Long))
                .map(ToOwned::to_owned),
            csln_core::GroupHeading::Localized { localized } => {
                self.resolve_localized_heading(localized)
            }
        }
    }

    fn resolve_localized_heading(&self, localized: &HashMap<String, String>) -> Option<String> {
        fn language_tag(locale: &str) -> &str {
            locale.split('-').next().unwrap_or(locale)
        }

        let mut candidates: Vec<String> = Vec::new();
        let mut push_candidate = |locale: &str| {
            let candidate = locale.to_string();
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        push_candidate(&self.locale.locale);
        push_candidate(language_tag(&self.locale.locale));

        if let Some(default_locale) = self.style.info.default_locale.as_deref() {
            push_candidate(default_locale);
            push_candidate(language_tag(default_locale));
        }

        push_candidate("en-US");
        push_candidate("en");

        for locale in candidates {
            if let Some(value) = localized.get(&locale) {
                return Some(value.clone());
            }
        }

        localized
            .iter()
            .min_by(|a, b| a.0.cmp(b.0))
            .map(|(_, value)| value.clone())
    }

    /// Render bibliography with configurable groups.
    fn render_with_custom_groups<F>(
        &self,
        bibliography: &[ProcEntry],
        groups: &[csln_core::BibliographyGroup],
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        use crate::grouping::{GroupSorter, SelectorEvaluator};
        use csln_core::grouping::DisambiguationScope;
        use std::collections::HashSet;

        let fmt = F::default();
        let cited_ids = self.cited_ids.borrow();

        let evaluator = SelectorEvaluator::new(&cited_ids);
        let sorter = GroupSorter::new(&self.locale);

        let mut assigned: HashSet<String> = HashSet::new();
        let mut result = String::new();

        for group in groups {
            // Find items matching this group's selector
            let matching_refs: Vec<&Reference> = bibliography
                .iter()
                .filter(|entry| !assigned.contains(&entry.id))
                .filter_map(|entry| {
                    self.bibliography
                        .get(&entry.id)
                        .filter(|reference| evaluator.matches(reference, &group.selector))
                })
                .collect();

            if matching_refs.is_empty() {
                continue;
            }

            // Mark as assigned (first-match semantics)
            for r in &matching_refs {
                if let Some(id) = r.id() {
                    assigned.insert(id);
                }
            }

            // Sort using per-group or global sort
            let sorted_refs = if let Some(sort_spec) = &group.sort {
                sorter.sort_references(matching_refs, sort_spec)
            } else {
                // references in `matching_refs` are in original global-sort order
                matching_refs
            };

            // Handle local disambiguation if requested
            let local_hints = if matches!(group.disambiguate, Some(DisambiguationScope::Locally)) {
                let mut group_bib = Bibliography::new();
                for r in &sorted_refs {
                    group_bib.insert(r.id().unwrap_or_default(), (*r).clone());
                }
                let disambiguator = if let Some(sort_spec) = &group.sort {
                    Disambiguator::with_group_sort(
                        &group_bib,
                        self.get_config(),
                        &self.locale,
                        sort_spec,
                    )
                } else {
                    Disambiguator::new(&group_bib, self.get_config(), &self.locale)
                };
                Some(disambiguator.calculate_hints())
            } else {
                None
            };

            // Re-render entries if local hints or local template is present
            let entries_vec: Vec<ProcEntry> = if local_hints.is_some() || group.template.is_some() {
                let hints = local_hints.as_ref().unwrap_or(&self.hints);
                let bib_config = self.get_bibliography_config();

                // Create a local style if we have a group-specific template
                let effective_style = if let Some(group_template) = &group.template {
                    let mut local_style = self.style.clone();
                    if let Some(bib_spec) = local_style.bibliography.as_mut() {
                        bib_spec.template = Some(group_template.clone());
                    }
                    std::borrow::Cow::Owned(local_style)
                } else {
                    std::borrow::Cow::Borrowed(&self.style)
                };

                let renderer = Renderer::new(
                    &effective_style,
                    &self.bibliography,
                    &self.locale,
                    &bib_config,
                    hints,
                    &self.citation_numbers,
                );

                sorted_refs
                    .into_iter()
                    .enumerate()
                    .map(|(i, r)| ProcEntry {
                        id: r.id().unwrap_or_default(),
                        template: renderer
                            .process_bibliography_entry(r, i + 1)
                            .unwrap_or_default(),
                        metadata: self.extract_metadata(r),
                    })
                    .collect()
            } else {
                // Use pre-rendered entries in sorted order
                sorted_refs
                    .into_iter()
                    .filter_map(|r| {
                        let id = r.id()?;
                        bibliography.iter().find(|e| e.id == id).cloned()
                    })
                    .collect()
            };

            // Add group heading
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            if let Some(heading) = &group.heading
                && let Some(resolved_heading) = self.resolve_group_heading(heading)
            {
                result.push_str(&format!("# {}\n\n", resolved_heading));
            }

            // Render entries
            result.push_str(&crate::render::refs_to_string_with_format::<F>(entries_vec));
        }

        // Fallback for ungrouped items
        let unassigned: Vec<ProcEntry> = bibliography
            .iter()
            .filter(|e| !assigned.contains(&e.id))
            .cloned()
            .collect();

        if !unassigned.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(&crate::render::refs_to_string_with_format::<F>(unassigned));
        }

        fmt.finish(result)
    }

    /// Legacy hardcoded cited/uncited grouping.
    fn render_with_legacy_grouping<F>(&self, bibliography: &[ProcEntry]) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let cited_ids = self.cited_ids.borrow();

        // Items cited visibly
        let cited_entries: Vec<ProcEntry> = bibliography
            .iter()
            .filter(|e| cited_ids.contains(&e.id))
            .cloned()
            .collect();

        let mut result = String::new();

        if !cited_entries.is_empty() {
            result.push_str(&crate::render::refs_to_string_with_format::<F>(
                cited_entries,
            ));
        }

        fmt.finish(result)
    }
}
