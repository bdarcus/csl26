/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Document-level citation processing.

use crate::processor::Processor;
use crate::render::format::OutputFormat;
use crate::Citation;
use regex::Regex;
use csln_core::citation::CitationMode;

/// A trait for document parsers that can identify citations.
pub trait CitationParser {
    /// Find and extract citations from a document string.
    /// Returns a list of (start_index, end_index, citation_model) tuples.
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)>;
}

/// A simple regex-based parser for Pandoc-style citations: `[@key]` and `@key`.
pub struct RegexCitationParser {
    parenthetical_regex: Regex,
    narrative_regex: Regex,
}

impl Default for RegexCitationParser {
    fn default() -> Self {
        Self {
            parenthetical_regex: Regex::new(r"\[@(?P<key>[^\]\s]+)\]").unwrap(),
            // Narrative: @key (must not be preceded by alphanumeric to avoid emails)
            narrative_regex: Regex::new(r"(?P<prefix>^|\s)@(?P<key>[a-zA-Z0-9_-]+)").unwrap(),
        }
    }
}

impl CitationParser for RegexCitationParser {
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)> {
        let mut results = Vec::new();

        // Find parenthetical [@key]
        for cap in self.parenthetical_regex.captures_iter(content) {
            let m = cap.get(0).unwrap();
            let key = cap.name("key").unwrap().as_str();
            results.push((m.start(), m.end(), Citation::simple(key)));
        }

        // Find narrative @key
        for cap in self.narrative_regex.captures_iter(content) {
            let full_match = cap.get(0).unwrap();
            let prefix = cap.name("prefix").unwrap();
            let key = cap.name("key").unwrap().as_str();

            let mut citation = Citation::simple(key);
            citation.mode = CitationMode::Integral;

            // The match includes the prefix (space/start), but we only want to replace the @key part
            let start = full_match.start() + prefix.as_str().len();
            results.push((start, full_match.end(), citation));
        }

        // Sort by start index
        results.sort_by_key(|r| r.0);
        results
    }
}

impl Processor {
    /// Process a full document by identifying and rendering citations.
    /// Returns the document content with citations replaced by their rendered forms,
    /// followed by the bibliography.
    pub fn process_document<P, F>(&self, content: &str, parser: &P) -> String
    where
        P: CitationParser,
        F: OutputFormat<Output = String>,
    {
        let mut result = String::new();
        let mut last_idx = 0;
        let citations = parser.parse_citations(content);
        let mut cited_ids = std::collections::HashSet::new();

        for (start, end, citation) in citations {
            // Add content before citation
            result.push_str(&content[last_idx..start]);

            // Track IDs for bibliography
            for item in &citation.items {
                cited_ids.insert(item.id.clone());
            }

            // Render citation
            match self.process_citation_with_format::<F>(&citation) {
                Ok(rendered) => result.push_str(&rendered),
                Err(_) => result.push_str(&content[start..end]), // Fallback to raw on error
            }

            last_idx = end;
        }

        // Add remaining content
        result.push_str(&content[last_idx..]);

        // Append bibliography
        let bib_header = "

# Bibliography

";
        result.push_str(bib_header);

        // Filter bibliography to only cited items
        // TODO: In a real document processor, we'd want to preserve the full bibliography
        // or filter based on citations. For now, we process all references.
        let bib_content = self.render_bibliography_with_format::<F>();
        result.push_str(&bib_content);

        result
    }
}
