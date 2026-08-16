/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Extracts global style options from CSL 1.0 structures into Citum Config.

pub mod bibliography;
pub mod contributors;
pub mod dates;
pub mod locators;
pub mod numbers;
pub mod processing;
pub mod titles;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests;

use citum_schema::options::{Config, SubstituteConfig};
use csl_legacy::model::Style;

/// The full set of configuration and flags extracted from a CSL 1.0 style.
pub struct MigrationOptions {
    /// The global configuration.
    pub options: Config,
    /// Bibliography-specific options.
    pub bibliography_options: Option<citum_schema::BibliographyOptions>,
    /// Citation-scoped contributor overrides.
    pub citation_contributor_overrides: Option<citum_schema::options::ContributorConfig>,
    /// Bibliography-scoped contributor overrides.
    pub bibliography_contributor_overrides: Option<citum_schema::options::ContributorConfig>,
    /// Whether the citation has a scoped shorten option.
    pub citation_has_scope_shorten: bool,
    /// Whether an authored date fallback conditional could not be represented.
    pub unsupported_date_fallback: bool,
}

/// Extracts global style options from a CSL 1.0 structure into Citum Config.
pub struct OptionsExtractor;

impl OptionsExtractor {
    /// Extracts a full set of migration options from the given CSL 1.0 style.
    pub fn extract_migration_options(style: &Style) -> MigrationOptions {
        let mut options = Self::extract(style);
        Self::apply_preset_extractions(&mut options);
        let date_fallback = self::dates::extract_date_fallback(style);
        options.date_fallback = date_fallback.policy;

        let bibliography_options =
            self::bibliography::extract_bibliography_config(style).map(|config| {
                citum_schema::BibliographyOptions {
                    article_journal: config.article_journal,
                    subsequent_author_substitute: config.subsequent_author_substitute,
                    subsequent_author_substitute_rule: config.subsequent_author_substitute_rule,
                    hanging_indent: config.hanging_indent,
                    entry_suffix: config.entry_suffix,
                    separator: config.separator,
                    suppress_period_after_url: config.suppress_period_after_url,
                    compound_numeric: config.compound_numeric,
                    ..Default::default()
                }
            });

        let citation_contributor_overrides =
            self::contributors::extract_citation_contributor_overrides(style);
        let bibliography_contributor_overrides =
            self::contributors::extract_bibliography_contributor_overrides(style);

        let citation_has_scope_shorten = citation_contributor_overrides
            .as_ref()
            .and_then(|contributors| contributors.shorten.as_ref())
            .is_some();

        MigrationOptions {
            options,
            bibliography_options,
            citation_contributor_overrides,
            bibliography_contributor_overrides,
            citation_has_scope_shorten,
            unsupported_date_fallback: date_fallback.unsupported,
        }
    }

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

            // Explicit no-date and alternative-date branches.
            date_fallback: self::dates::extract_date_fallback(style).policy,

            // 5. Extract title configuration
            titles: self::titles::extract_title_config(style),

            // 6. Extract page range format
            page_range_format: self::numbers::extract_page_range_format(style),

            // 7. Extract locator configuration
            locators: self::locators::extract_locator_config(style),

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

    fn apply_preset_extractions(options: &mut Config) {
        use crate::base_detector;
        if let Some(contributors) = options.contributors.clone()
            && let Some(preset) = base_detector::detect_contributor_preset(&contributors)
        {
            options.contributors = Some(preset.config());
        }

        if let Some(titles) = options.titles.clone()
            && let Some(preset) = base_detector::detect_title_preset(&titles)
        {
            options.titles = Some(preset.config());
        }

        if let Some(dates) = options.dates.clone()
            && let Some(preset) = base_detector::detect_date_preset(&dates)
        {
            options.dates = Some(preset.config());
        }

        if let Some(SubstituteConfig::Explicit(substitute)) = options.substitute.clone()
            && let Some(preset) = base_detector::detect_substitute_preset(&substitute)
        {
            options.substitute = Some(SubstituteConfig::Preset(preset));
        }
    }
}
