/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Djot document parsing and HTML conversion.

use super::CitationParser;
use crate::{Citation, CitationItem};
use csln_core::citation::{CitationMode, LocatorType};
use winnow::ascii::space0;
use winnow::combinator::{opt, repeat};
use winnow::error::ContextError;
use winnow::prelude::*;
use winnow::token::{take_until, take_while};

/// A parser for Djot citations using winnow.
/// Syntax: `[@key]`, `[+@key]`, or `[-@key]`. Multi-cites: `[@key1; @key2]`.
pub struct DjotParser;

impl Default for DjotParser {
    fn default() -> Self {
        Self
    }
}

fn parse_suppress_author_modifier(input: &mut &str) -> winnow::Result<bool, ContextError> {
    let modifier: Option<char> = opt('-').parse_next(input)?;
    Ok(modifier.is_some())
}

fn parse_integral_modifier(input: &mut &str) -> winnow::Result<bool, ContextError> {
    let modifier: Option<char> = opt('+').parse_next(input)?;
    Ok(modifier.is_some())
}

impl CitationParser for DjotParser {
    fn parse_citations(&self, content: &str) -> Vec<(usize, usize, Citation)> {
        let mut results = Vec::new();
        let mut input = content;
        let mut offset = 0;

        while !input.is_empty() {
            let next_bracket = input.find('[');
            let start_pos = match next_bracket {
                Some(b) => b,
                None => break,
            };

            let potential = &input[start_pos..];
            let mut p_input = potential;

            // Try to parse the citation structure: [content]
            if let Ok(citation) = parse_parenthetical_citation(&mut p_input) {
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

/// Parse `[content]`
fn parse_parenthetical_citation(input: &mut &str) -> winnow::Result<Citation, ContextError> {
    let _ = '['.parse_next(input)?;
    let citation = parse_citation_content.parse_next(input)?;
    let _ = ']'.parse_next(input)?;
    Ok(citation)
}

fn parse_citation_content(input: &mut &str) -> winnow::Result<Citation, ContextError> {
    let mut citation = Citation::default();
    let mut detected_integral = false;
    let mut suppress_author = false;

    // Split by semicolon for multiple items
    let inner: &str = take_until(0.., ']').parse_next(input)?;

    // Basic item parsing: items are separated by semicolons
    let items: Vec<CitationItem> = repeat(1.., |input: &mut &str| {
        let _ = space0.parse_next(input)?;
        let is_integral = parse_integral_modifier.parse_next(input).unwrap_or(false);
        if is_integral {
            detected_integral = true;
        }
        let suppress = parse_suppress_author_modifier(input)?;
        if suppress {
            suppress_author = true;
        }
        let item = parse_citation_item_no_integral(input)?;
        let _ = opt(';').parse_next(input)?;
        let _ = space0.parse_next(input)?;
        Ok(item)
    })
    .parse_next(&mut inner.trim())?;

    citation.items = items;
    citation.suppress_author = suppress_author;
    if detected_integral {
        citation.mode = CitationMode::Integral;
    }

    Ok(citation)
}

fn parse_citation_item_no_integral(input: &mut &str) -> winnow::Result<CitationItem, ContextError> {
    let _ = space0.parse_next(input)?;
    let _: char = '@'.parse_next(input)?;
    let key: &str =
        take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '-').parse_next(input)?;

    let mut item = CitationItem {
        id: key.to_string(),
        ..Default::default()
    };

    let _ = space0.parse_next(input)?;

    // Only consume text after key if there's a comma. Otherwise,
    // leave remaining text for the multi-cite separator.
    let checkpoint = *input;
    let after_key: &str = take_while(0.., |c: char| c != ';' && c != ']').parse_next(input)?;

    if let Some(comma_pos) = after_key.find(',') {
        let locator_part = after_key[comma_pos + 1..].trim();
        parse_hybrid_locators(&mut item, locator_part);
    } else {
        // No comma found: don't consume the text, restore position
        *input = checkpoint;
    }

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

            if let Some(lt) = map_label_str(label_str) {
                item.label = Some(lt);
                item.locator = Some(value.to_string());
            } else {
                // Not a known label, treat whole string as locator (default to page)
                item.label = Some(LocatorType::Page);
                item.locator = Some(lp.to_string());
            }
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
        _ => None,
    }
}

/// Convert Djot markup to HTML using jotdown.
pub fn djot_to_html(djot: &str) -> String {
    let events = jotdown::Parser::new(djot);
    jotdown::html::render_to_string(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_multi_cite_with_locators() {
        let parser = DjotParser;
        let content = "[@kuhn1962; @watson1953, ch. 2]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.items.len(), 2);
        assert_eq!(citation.items[0].id, "kuhn1962");
        assert_eq!(citation.items[1].id, "watson1953");
        assert_eq!(citation.items[1].locator, Some("2".to_string()));
        assert_eq!(citation.items[1].label, Some(LocatorType::Chapter));
    }

    #[test]
    fn test_parse_structured_locator() {
        let parser = DjotParser;
        let content = "[@kuhn1962, section: 5]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.items[0].locator, Some("5".to_string()));
        assert_eq!(citation.items[0].label, Some(LocatorType::Section));
    }

    #[test]
    fn test_parse_suppress_author() {
        let parser = DjotParser;
        let content = "[-@kuhn1962]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.items[0].id, "kuhn1962");
        assert!(citation.suppress_author);
    }

    #[test]
    fn test_parse_bracketed_integral_citation() {
        let parser = DjotParser;
        let content = "[+@kuhn1962]";
        let citations = parser.parse_citations(content);

        assert_eq!(citations.len(), 1);
        let (_, _, citation) = &citations[0];
        assert_eq!(citation.mode, CitationMode::Integral);
        assert_eq!(citation.items[0].id, "kuhn1962");
        assert!(!citation.suppress_author);
    }

    #[test]
    fn test_parse_semicolon_without_citation() {
        let parser = DjotParser;
        let content = "[foo; bar]";
        let citations = parser.parse_citations(content);

        // Should not parse as a citation if no '@' keys are present
        assert_eq!(citations.len(), 0);
    }
}
