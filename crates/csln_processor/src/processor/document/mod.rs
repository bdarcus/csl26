/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Document-level citation processing.

pub mod djot;

use crate::processor::Processor;
use crate::Citation;

/// A trait for document parsers that can identify citations.
pub trait CitationParser {
    /// Find and extract citations from a document string.
    /// Returns a list of (start_index, end_index, citation_model) tuples.
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)>;
}

/// Document output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    /// Plain text (raw markup).
    Plain,
    /// Djot markup.
    Djot,
    /// HTML output.
    Html,
}

impl Processor {
    /// Process citations in a document and append a bibliography.
    pub fn process_document<P, F>(
        &self,
        content: &str,
        parser: &P,
        format: DocumentFormat,
    ) -> String
    where
        P: CitationParser,
        F: crate::render::format::OutputFormat<Output = String>,
    {
        use crate::render::plain::PlainText;

        let mut result = String::new();
        let mut last_idx = 0;
        let citations = parser.parse_citations(content);

        // Always render citations as plain text for Djot documents
        // HTML conversion happens at the end via jotdown
        for (start, end, citation) in citations {
            result.push_str(&content[last_idx..start]);
            match self.process_citation_with_format::<PlainText>(&citation) {
                Ok(rendered) => result.push_str(&rendered),
                Err(_) => result.push_str(&content[start..end]),
            }
            last_idx = end;
        }

        result.push_str(&content[last_idx..]);
        result.push_str(
            "

# Bibliography

",
        );
        let bib_content = self.render_bibliography_with_format::<PlainText>();
        result.push_str(&bib_content);

        // Convert to HTML if requested
        match format {
            DocumentFormat::Html => self::djot::djot_to_html(&result),
            DocumentFormat::Djot | DocumentFormat::Plain => result,
        }
    }
}
