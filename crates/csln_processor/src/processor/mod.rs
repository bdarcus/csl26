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
pub mod matching;
pub mod rendering;
pub mod sorting;

#[cfg(test)]
mod tests;

use crate::error::ProcessorError;
use crate::reference::{Bibliography, Citation, Reference};
use crate::render::{refs_to_string, ProcTemplate};
use crate::values::ProcHints;
use csln_core::locale::Locale;
use csln_core::options::Config;
use csln_core::template::WrapPunctuation;
use csln_core::Style;
use std::collections::HashMap;

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
    pub citation_numbers: std::cell::RefCell<HashMap<String, usize>>,
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

    /// Process all references to get rendered output.
    pub fn process_references(&self) -> ProcessedReferences {
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        let mut bibliography: Vec<ProcTemplate> = Vec::new();
        let mut prev_reference: Option<&Reference> = None;

        let bib_config = self.get_config().bibliography.as_ref();
        let substitute = bib_config.and_then(|c| c.subsequent_author_substitute.as_ref());

        for (index, reference) in sorted_refs.iter().enumerate() {
            // For numeric styles, use the citation number assigned when first cited.
            // For other styles, use position in sorted bibliography.
            let entry_number = self
                .citation_numbers
                .borrow()
                .get(&reference.id().unwrap_or_default())
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
        let template_vec = citation_spec
            .and_then(|cs| cs.resolve_template())
            .unwrap_or_default();
        let template = template_vec.as_slice();

        // Get intra-citation delimiter (between components like author and year)
        let intra_delimiter = citation_spec
            .and_then(|cs| cs.delimiter.as_deref())
            .unwrap_or(", ");

        // Inter-citation delimiter (between different author groups)
        let inter_delimiter = citation_spec
            .and_then(|cs| cs.multi_cite_delimiter.as_deref())
            .unwrap_or("; ");

        // Check if this is an author-date style that supports grouping
        let is_author_date = self
            .style
            .options
            .as_ref()
            .and_then(|o| o.processing.as_ref())
            .map(|p| matches!(p, csln_core::options::Processing::AuthorDate))
            .unwrap_or(false);

        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            self.get_config(),
            &self.hints,
            &self.citation_numbers,
        );

        // Group adjacent items by author for author-date styles
        let rendered_groups = if is_author_date && citation.items.len() > 1 {
            renderer.render_grouped_citation(&citation.items, template, intra_delimiter)?
        } else {
            // No grouping - render each item separately
            renderer.render_ungrouped_citation(&citation.items, template, intra_delimiter)?
        };

        let content = rendered_groups.join(inter_delimiter);

        // Get wrap/prefix/suffix from citation spec
        let wrap = citation_spec.and_then(|cs| cs.wrap.as_ref());
        let prefix = citation_spec.and_then(|cs| cs.prefix.as_deref());
        let suffix = citation_spec.and_then(|cs| cs.suffix.as_deref());

        // Apply citation-level prefix from input
        let citation_prefix = citation.prefix.as_deref().unwrap_or("");
        let citation_suffix = citation.suffix.as_deref().unwrap_or("");

        // Apply wrap or prefix/suffix
        let (open, close) = match wrap {
            Some(WrapPunctuation::Parentheses) => ("(", ")"),
            Some(WrapPunctuation::Brackets) => ("[", "]"),
            Some(WrapPunctuation::Quotes) => ("\u{201C}", "\u{201D}"),
            _ => (prefix.unwrap_or(""), suffix.unwrap_or("")),
        };

        Ok(format!(
            "{}{}{}{}{}",
            open, citation_prefix, content, citation_suffix, close
        ))
    }

    /// Process a bibliography entry.
    pub fn process_bibliography_entry(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate> {
        let renderer = Renderer::new(
            &self.style,
            &self.bibliography,
            &self.locale,
            self.get_config(),
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

    /// Render the bibliography to a string.
    pub fn render_bibliography(&self) -> String {
        let processed = self.process_references();
        refs_to_string(processed.bibliography)
    }
}
