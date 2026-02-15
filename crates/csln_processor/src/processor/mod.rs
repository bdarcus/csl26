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
pub mod matching;
pub mod rendering;
pub mod sorting;

#[cfg(test)]
mod tests;

use crate::error::ProcessorError;
use crate::reference::{Bibliography, Citation, Reference};
use crate::render::{ProcEntry, ProcTemplate};
use crate::values::ProcHints;
use csln_core::locale::Locale;
use csln_core::options::Config;
use csln_core::template::WrapPunctuation;
use csln_core::Style;
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
    /// IDs of items that were cited only silently (nocite).
    pub silent_ids: RefCell<HashSet<String>>,
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
            silent_ids: RefCell::new(HashSet::new()),
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
            silent_ids: RefCell::new(HashSet::new()),
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
                if let Some(sub_string) = substitute {
                    if let Some(prev) = prev_reference {
                        // Check if primary contributor matches
                        if self.contributors_match(prev, reference) {
                            self.apply_author_substitution(&mut proc, sub_string);
                        }
                    }
                }

                bibliography.push(ProcEntry {
                    id: ref_id,
                    template: proc,
                });
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
        // Resolve the effective citation spec based on the mode (Integral vs NonIntegral)
        // If the style has no citation spec, we use a default one.
        let default_spec = csln_core::CitationSpec::default();
        let effective_spec = self
            .style
            .citation
            .as_ref()
            .map(|cs| cs.resolve_for_mode(&citation.mode))
            .unwrap_or(std::borrow::Cow::Borrowed(&default_spec));

        let template_vec = effective_spec.resolve_template().unwrap_or_default();
        let template = template_vec.as_slice();

        // Get intra-citation delimiter (between components like author and year)
        let intra_delimiter = effective_spec.delimiter.as_deref().unwrap_or(", ");

        // Inter-citation delimiter (between different author groups)
        let inter_delimiter = effective_spec
            .multi_cite_delimiter
            .as_deref()
            .unwrap_or("; ");

        // Check if this is an author-date style that supports grouping
        let is_author_date = self
            .style
            .options
            .as_ref()
            .and_then(|o| o.processing.as_ref())
            .map(|p| matches!(p, csln_core::options::Processing::AuthorDate))
            .unwrap_or(false);

        // Use citation-specific merged config
        let cite_config = self.get_citation_config();

        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            &cite_config,
            &self.hints,
            &self.citation_numbers,
        );

        // Group adjacent items by author for author-date styles
        let rendered_groups = if is_author_date && citation.items.len() > 1 {
            renderer.render_grouped_citation(
                &citation.items,
                template,
                &citation.mode,
                intra_delimiter,
            )?
        } else {
            // No grouping - render each item separately
            renderer.render_ungrouped_citation(
                &citation.items,
                template,
                &citation.mode,
                intra_delimiter,
            )?
        };

        let content = rendered_groups.join(inter_delimiter);

        if content.is_empty() {
            return Ok(String::new());
        }

        // Get wrap/prefix/suffix from citation spec
        let wrap = effective_spec.wrap.as_ref();
        let prefix = effective_spec.prefix.as_deref();
        let suffix = effective_spec.suffix.as_deref();

        // Apply citation-level prefix from input
        let citation_prefix = citation.prefix.as_deref().unwrap_or("");
        let citation_suffix = citation.suffix.as_deref().unwrap_or("");

        // Apply wrap or prefix/suffix
        let (open, close) = if matches!(citation.mode, csln_core::citation::CitationMode::Integral)
        {
            ("", "")
        } else {
            match wrap {
                Some(WrapPunctuation::Parentheses) => ("(", ")"),
                Some(WrapPunctuation::Brackets) => ("[", "]"),
                Some(WrapPunctuation::Quotes) => ("\u{201C}", "\u{201D}"),
                _ => (prefix.unwrap_or(""), suffix.unwrap_or("")),
            }
        };

        // If it was integral mode, we might still want the spec prefix/suffix
        let (spec_open, spec_close) =
            if matches!(citation.mode, csln_core::citation::CitationMode::Integral) {
                (prefix.unwrap_or(""), suffix.unwrap_or(""))
            } else {
                ("", "")
            };

        // Ensure citation-level suffix has proper spacing
        let formatted_suffix =
            if citation_suffix.is_empty() || citation_suffix.starts_with(char::is_whitespace) {
                citation_suffix.to_string()
            } else {
                format!(" {}", citation_suffix)
            };

        Ok(format!(
            "{}{}{}{}{}{}{}",
            spec_open, open, citation_prefix, content, formatted_suffix, close, spec_close
        ))
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
        let sorter = Sorter::new(self.get_config(), &self.locale);
        sorter.sort_references(references)
    }

    /// Calculate processing hints for disambiguation.
    pub fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let config = self.get_config();
        let disambiguator = Disambiguator::new(&self.bibliography, config);
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
        let processed = self.process_references();
        crate::render::refs_to_string_with_format::<F>(processed.bibliography)
    }

    /// Render a citation to a string using a specific format.
    pub fn process_citation_with_format<F>(
        &self,
        citation: &Citation,
    ) -> Result<String, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        // Track cited IDs
        for item in &citation.items {
            if matches!(item.visibility, csln_core::citation::ItemVisibility::Hidden) {
                self.silent_ids.borrow_mut().insert(item.id.clone());
            } else {
                self.cited_ids.borrow_mut().insert(item.id.clone());
            }
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

        let intra_delimiter = effective_spec.delimiter.as_deref().unwrap_or(", ");
        let inter_delimiter = effective_spec
            .multi_cite_delimiter
            .as_deref()
            .unwrap_or("; ");

        let is_author_date = self
            .style
            .options
            .as_ref()
            .and_then(|o| o.processing.as_ref())
            .map(|p| matches!(p, csln_core::options::Processing::AuthorDate))
            .unwrap_or(false);

        let cite_config = self.get_citation_config();
        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            &cite_config,
            &self.hints,
            &self.citation_numbers,
        );

        // Process group components
        let rendered_groups = if is_author_date && citation.items.len() > 1 {
            renderer.render_grouped_citation_with_format::<F>(
                &citation.items,
                template,
                &citation.mode,
                intra_delimiter,
            )?
        } else {
            renderer.render_ungrouped_citation_with_format::<F>(
                &citation.items,
                template,
                &citation.mode,
                intra_delimiter,
            )?
        };

        let fmt = F::default();
        let content = fmt.join(rendered_groups, inter_delimiter);

        // Apply citation-level prefix/suffix from input
        let citation_prefix = citation.prefix.as_deref().unwrap_or("");
        let citation_suffix = citation.suffix.as_deref().unwrap_or("");

        // Ensure proper spacing before suffix
        let spaced_suffix =
            if !citation_suffix.is_empty() && !citation_suffix.starts_with(char::is_whitespace) {
                format!(" {}", citation_suffix)
            } else {
                citation_suffix.to_string()
            };

        let output = if !citation_prefix.is_empty() || !citation_suffix.is_empty() {
            fmt.affix(citation_prefix, content, &spaced_suffix)
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

    /// Render the bibliography to a string.
    pub fn render_bibliography(&self) -> String {
        self.render_bibliography_with_format::<crate::render::plain::PlainText>()
    }

    /// Render the bibliography with grouping for uncited (nocite) items.
    pub fn render_grouped_bibliography_with_format<F>(&self) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let processed = self.process_references();
        let fmt = F::default();

        let cited_ids = self.cited_ids.borrow();
        let silent_ids = self.silent_ids.borrow();

        // Items cited visibly
        let cited_entries: Vec<ProcEntry> = processed
            .bibliography
            .iter()
            .filter(|e| cited_ids.contains(&e.id))
            .cloned()
            .collect();

        // Items only cited silently (nocite) AND not cited visibly anywhere else
        let uncited_entries: Vec<ProcEntry> = processed
            .bibliography
            .iter()
            .filter(|e| !cited_ids.contains(&e.id) && silent_ids.contains(&e.id))
            .cloned()
            .collect();

        let mut result = String::new();

        if !cited_entries.is_empty() {
            result.push_str(&crate::render::refs_to_string_with_format::<F>(
                cited_entries,
            ));
        }

        if !uncited_entries.is_empty() {
            // Add spacing between groups
            if !result.is_empty() {
                result.push_str("\n\n");
            }

            // Simple hardcoded heading for now as requested
            result.push_str("# Additional Reading\n\n");
            result.push_str(&crate::render::refs_to_string_with_format::<F>(
                uncited_entries,
            ));
        }

        fmt.finish(result)
    }
}
