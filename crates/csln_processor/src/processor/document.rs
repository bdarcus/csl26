/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Document-level citation processing.

use crate::processor::Processor;
use crate::render::format::OutputFormat;
use crate::{Citation, CitationItem};
use csln_core::citation::{CitationMode, LocatorType};
use winnow::ascii::space0;
use winnow::combinator::{alt, opt, repeat};
use winnow::error::ContextError;
use winnow::prelude::*;
use winnow::token::{take_until, take_while};

/// A trait for document parsers that can identify citations.
pub trait CitationParser {
    /// Find and extract citations from a document string.
    /// Returns a list of (start_index, end_index, citation_model) tuples.
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)>;
}

/// A parser for Djot citations using winnow.
/// Syntax: `[prefix @key1; @key2, locator suffix]` or `@key[locator]`
pub struct WinnowCitationParser;

impl Default for WinnowCitationParser {
    fn default() -> Self {
        Self
    }
}

impl CitationParser for WinnowCitationParser {
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)> {
        let mut results = Vec::new();
        let mut input = content;
        let mut offset = 0;

        while !input.is_empty() {
            let next_bracket = input.find('[');
            let next_at = input.find('@');

            let start_pos = match (next_bracket, next_at) {
                (Some(b), Some(a)) => std::cmp::min(b, a),
                (Some(b), None) => b,
                (None, Some(a)) => a,
                (None, None) => break,
            };

            let potential = &input[start_pos..];
            let mut p_input = potential;

            // Try to parse the citation structure
            if let Ok(citation) = parse_any_citation(&mut p_input) {
                let consumed = potential.len() - p_input.len();
                let end_pos = start_pos + consumed;
                results.push((offset + start_pos, offset + end_pos, citation));

                let shift = end_pos;
                input = &input[shift..];
                offset += shift;
            } else {
                // Not a citation, skip and continue
                let shift = start_pos + 1;
                input = &input[shift..];
                offset += shift;
            }
        }

        results
    }
}

/// Parse either parenthetical `[...]` or narrative `@key [...]`
fn parse_any_citation(input: &mut &str) -> winnow::Result<Citation, ContextError> {
    alt((parse_parenthetical_citation, parse_narrative_citation)).parse_next(input)
}

/// Parse `[content]`
fn parse_parenthetical_citation(input: &mut &str) -> winnow::Result<Citation, ContextError> {
    let _ = '['.parse_next(input)?;
    let citation = parse_citation_content.parse_next(input)?;
    let _ = ']'.parse_next(input)?;
    Ok(citation)
}

/// Parse `@key(infix)[locator]`, `@key(infix)`, `@key[locator]`, or just `@key`
fn parse_narrative_citation(input: &mut &str) -> winnow::Result<Citation, ContextError> {
    let _: char = '@'.parse_next(input)?;
    let key: &str =
        take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-').parse_next(input)?;

    let mut item = CitationItem {
        id: key.to_string(),
        ..Default::default()
    };

    // Try to parse optional infix in parentheses: (infix)
    let mut input_checkpoint = *input;
    let infix_res: winnow::Result<&str, ContextError> =
        parse_citation_infix_parens(&mut input_checkpoint);

    if let Ok(infix_part) = infix_res {
        *input = input_checkpoint;
        if !infix_part.is_empty() {
            item.infix = Some(infix_part.to_string());
        }
    }

    // Try to parse optional locator in brackets: [locator]
    let mut input_checkpoint = *input;
    let locator_res: winnow::Result<&str, ContextError> =
        parse_citation_locator_brackets(&mut input_checkpoint);

    let mut citation = Citation {
        mode: CitationMode::Integral,
        ..Default::default()
    };

    if let Ok(locator_part) = locator_res {
        *input = input_checkpoint;
        parse_hybrid_locators(&mut item, locator_part);
    }

    citation.items.push(item);
    Ok(citation)
}

fn parse_citation_locator_brackets<'a>(
    input: &mut &'a str,
) -> winnow::Result<&'a str, ContextError> {
    let _ = '['.parse_next(input)?;
    let l = take_until(0.., ']').parse_next(input)?;
    let _ = ']'.parse_next(input)?;
    Ok(l)
}

fn parse_citation_infix_parens<'a>(input: &mut &'a str) -> winnow::Result<&'a str, ContextError> {
    let _ = '('.parse_next(input)?;
    let i = take_until(0.., ')').parse_next(input)?;
    let _ = ')'.parse_next(input)?;
    Ok(i)
}

fn parse_citation_content(input: &mut &str) -> winnow::Result<Citation, ContextError> {
    let mut citation = Citation::default();

    // Global Prefix: everything before first @
    let prefix_part: &str = take_until(0.., "@").parse_next(input)?;
    if !prefix_part.is_empty() {
        let trimmed = prefix_part.trim_end_matches(';').trim_start_matches(' ');
        if !trimmed.is_empty() {
            citation.prefix = Some(trimmed.to_string());
        }
    }

    let items: Vec<CitationItem> = repeat(1.., parse_citation_item).parse_next(input)?;
    citation.items = items;

    // Global Suffix: anything remaining before ]
    let suffix_part: &str = take_while(0.., |c: char| c != ']').parse_next(input)?;
    if !suffix_part.is_empty() {
        citation.suffix = Some(suffix_part.to_string());
    }

    Ok(citation)
}

fn parse_citation_item(input: &mut &str) -> winnow::Result<CitationItem, ContextError> {
    let _ = space0.parse_next(input)?;
    let _: char = '@'.parse_next(input)?;
    let key: &str =
        take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-').parse_next(input)?;

    let mut item = CitationItem {
        id: key.to_string(),
        ..Default::default()
    };

    let after_key: &str = take_while(0.., |c: char| c != ';' && c != ']').parse_next(input)?;

    if let Some(comma_pos) = after_key.find(',') {
        let locator_part = after_key[comma_pos + 1..].trim();
        parse_hybrid_locators(&mut item, locator_part);
    }

    let _ = opt(';').parse_next(input)?;
    let _ = space0.parse_next(input)?;

    Ok(item)
}

/// Parse locators in either `p. 23` or `page: 23, section: V` format.
fn parse_hybrid_locators(item: &mut CitationItem, locator_str: &str) {
    let lp = locator_str.trim();
    if lp.is_empty() {
        return;
    }

    // Check for explicit key-value: `page: 23`
    if let Some(colon_pos) = lp.find(':') {
        let key = lp[..colon_pos].trim().to_lowercase();
        let val_with_rest = lp[colon_pos + 1..].trim();

        // Handle potential comma for multiple structured locators: `page: 23, section: V`
        // For now, we only support one locator per item in the model, so we take the first.
        let val = if let Some(comma_pos) = val_with_rest.find(',') {
            &val_with_rest[..comma_pos]
        } else {
            val_with_rest
        };

        item.label = map_label_str(&key);
        item.locator = Some(val.trim().to_string());
    } else {
        // Fallback to shorthand: `p. 23`
        if let Some(space_pos) = lp.find(' ') {
            let label_str = lp[..space_pos].trim_end_matches('.');
            let value = &lp[space_pos + 1..];

            item.label = map_label_str(label_str);
            item.locator = Some(value.to_string());
        } else {
            // No label, assume page
            item.label = Some(LocatorType::Page);
            item.locator = Some(lp.to_string());
        }
    }
}

fn map_label_str(s: &str) -> Option<LocatorType> {
    match s.trim().trim_end_matches('.').to_lowercase().as_str() {
        "p" | "page" | "pp" => Some(LocatorType::Page),
        "vol" | "volume" => Some(LocatorType::Volume),
        "ch" | "chap" | "chapter" => Some(LocatorType::Chapter),
        "sec" | "section" => Some(LocatorType::Section),
        "fig" | "figure" => Some(LocatorType::Figure),
        "line" | "l" => Some(LocatorType::Line),
        "note" | "n" => Some(LocatorType::Note),
        "part" => Some(LocatorType::Part),
        "col" | "column" => Some(LocatorType::Column),
        _ => Some(LocatorType::Page),
    }
}

impl Processor {
    pub fn process_document<P, F>(
        &self,
        content: &str,
        parser: &P,
        format: DocumentFormat,
    ) -> String
    where
        P: CitationParser,
        F: OutputFormat<Output = String>,
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
        result.push_str("\n\n# Bibliography\n\n");
        let bib_content = self.render_bibliography_with_format::<PlainText>();
        result.push_str(&bib_content);

        // Convert to HTML if requested
        match format {
            DocumentFormat::Html => djot_to_html(&result),
            DocumentFormat::Djot | DocumentFormat::Plain => result,
        }
    }
}

/// Convert Djot markup to HTML using jotdown.
fn djot_to_html(djot: &str) -> String {
    let events = jotdown::Parser::new(djot);
    jotdown::html::render_to_string(events)
}

/// Document output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    /// Plain text (raw Djot markup).
    Plain,
    /// Djot markup (same as Plain).
    Djot,
    /// HTML output.
    Html,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_complex_djot_citation() {
        let parser = WinnowCitationParser;
        let content = "[see ;@kuhn1962; @watson1953, ch. 2]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.prefix, Some("see ".to_string()));
        assert_eq!(citation.items.len(), 2);
        assert_eq!(citation.items[0].id, "kuhn1962");
        assert_eq!(citation.items[1].id, "watson1953");
        assert_eq!(citation.items[1].locator, Some("2".to_string()));
        assert_eq!(citation.items[1].label, Some(LocatorType::Chapter));
    }

    #[test]
    fn test_parse_narrative_with_locator() {
        let parser = WinnowCitationParser;
        let content = "@kuhn1962[p. 10]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.mode, CitationMode::Integral);
        assert_eq!(citation.items.len(), 1);
        assert_eq!(citation.items[0].id, "kuhn1962");
        assert_eq!(citation.items[0].locator, Some("10".to_string()));
        assert_eq!(citation.items[0].label, Some(LocatorType::Page));
    }

    #[test]
    fn test_parse_structured_locator() {
        let parser = WinnowCitationParser;
        let content = "[@kuhn1962, section: 5]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.items[0].locator, Some("5".to_string()));
        assert_eq!(citation.items[0].label, Some(LocatorType::Section));
    }

    #[test]
    fn test_parse_infix_citation() {
        let parser = WinnowCitationParser;
        let content = "@smith(argues that x)";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.mode, CitationMode::Integral);
        assert_eq!(citation.items.len(), 1);
        assert_eq!(citation.items[0].id, "smith");
        assert_eq!(citation.items[0].infix, Some("argues that x".to_string()));
    }

    #[test]
    fn test_parse_infix_with_locator() {
        let parser = WinnowCitationParser;
        let content = "@smith(argues that x)[23]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.mode, CitationMode::Integral);
        assert_eq!(citation.items.len(), 1);
        assert_eq!(citation.items[0].id, "smith");
        assert_eq!(citation.items[0].infix, Some("argues that x".to_string()));
        assert_eq!(citation.items[0].locator, Some("23".to_string()));
        assert_eq!(citation.items[0].label, Some(LocatorType::Page));
    }

    #[test]
    fn test_parse_infix_with_structured_locator() {
        let parser = WinnowCitationParser;
        let content = "@jones(notes that y)[chapter: 5]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.items[0].id, "jones");
        assert_eq!(citation.items[0].infix, Some("notes that y".to_string()));
        assert_eq!(citation.items[0].locator, Some("5".to_string()));
        assert_eq!(citation.items[0].label, Some(LocatorType::Chapter));
    }
}
