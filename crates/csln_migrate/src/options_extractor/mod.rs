/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Extracts global style options from CSL 1.0 structures into CSLN Config.

pub mod bibliography;
pub mod contributors;
pub mod dates;
pub mod numbers;
pub mod processing;
pub mod titles;

#[cfg(test)]
mod tests;

use csl_legacy::model::Style;
use csln_core::options::{Config, SubstituteConfig};

/// Extracts global configuration options from a CSL 1.0 style.
pub struct OptionsExtractor;

impl OptionsExtractor {
    /// Extract a Config from the given CSL 1.0 style.
    pub fn extract(style: &Style) -> Config {
        Config {
            // 1. Detect processing mode from citation attributes
            processing: self::processing::detect_processing_mode(style),

            // 2. Extract contributor options
            contributors: self::contributors::extract_contributor_config(style),

            // 3. Extract substitute patterns
            substitute: self::contributors::extract_substitute_pattern(style)
                .map(SubstituteConfig::Explicit),

            // 4. Extract date configuration
            dates: self::dates::extract_date_config(style),

            // 5. Extract title configuration
            titles: self::titles::extract_title_config(style),

            // 6. Extract page range format
            page_range_format: self::numbers::extract_page_range_format(style),

            // 7. Extract bibliography-specific settings
            bibliography: self::bibliography::extract_bibliography_config(style),

            // 8. Punctuation-in-quote heuristic
            punctuation_in_quote: Self::extract_punctuation_in_quote(style),

            // 9. Volume-pages delimiter
            volume_pages_delimiter: {
                // Collect macros needed for delimiter extraction
                let mut macros = std::collections::HashSet::new();
                if let Some(bib) = &style.bibliography {
                    Self::collect_macro_refs_from_nodes(&bib.layout.children, &mut macros);
                }
                self::numbers::extract_volume_pages_delimiter(style, &macros)
            },

            ..Config::default()
        }
    }

    fn extract_punctuation_in_quote(style: &Style) -> bool {
        match style.default_locale.as_deref() {
            Some(locale) if locale.starts_with("en-US") => true,
            Some(locale) if locale.starts_with("en-GB") => false,
            Some(locale) if locale.starts_with("en") => true,
            None => true,
            _ => false,
        }
    }

    fn collect_macro_refs_from_nodes(
        nodes: &[csl_legacy::model::CslNode],
        macros: &mut std::collections::HashSet<String>,
    ) {
        use csl_legacy::model::CslNode;
        for node in nodes {
            match node {
                CslNode::Text(t) => {
                    if let Some(name) = &t.macro_name {
                        macros.insert(name.clone());
                    }
                }
                CslNode::Group(g) => Self::collect_macro_refs_from_nodes(&g.children, macros),
                CslNode::Choose(c) => {
                    Self::collect_macro_refs_from_nodes(&c.if_branch.children, macros);
                    for branch in &c.else_if_branches {
                        Self::collect_macro_refs_from_nodes(&branch.children, macros);
                    }
                    if let Some(else_branch) = &c.else_branch {
                        Self::collect_macro_refs_from_nodes(else_branch, macros);
                    }
                }
                CslNode::Names(n) => Self::collect_macro_refs_from_nodes(&n.children, macros),
                _ => {}
            }
        }
    }
}
