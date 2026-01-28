/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Extracts global style options from CSL 1.0 structures into CSLN Config.
//!
//! This module implements "semantic upsampling" - inferring the bibliographic
//! intent from CSL 1.0's procedural template structure and encoding it as
//! declarative options in CSLN.

use csl_legacy::model::{CslNode, Style, Names, Substitute};
use csln_core::options::{
    AndOptions, Config, ContributorConfig, DateConfig,
    DisplayAsSort, Processing, ShortenListOptions, Substitute as CslnSubstitute,
    SubstituteKey, TitlesConfig,
};

/// Extracts global configuration options from a CSL 1.0 style.
pub struct OptionsExtractor;

impl OptionsExtractor {
    /// Extract a Config from the given CSL 1.0 style.
    pub fn extract(style: &Style) -> Config {
        let mut config = Config::default();

        // 1. Detect processing mode from citation attributes
        config.processing = Self::detect_processing_mode(style);

        // 2. Extract contributor options from citation/bibliography attributes
        config.contributors = Self::extract_contributor_config(style);

        // 3. Extract substitute patterns from the first <names> block with <substitute>
        config.substitute = Self::extract_substitute_pattern(style);

        // 4. Extract date configuration
        config.dates = Self::extract_date_config(style);

        // 5. Extract title configuration from formatting patterns
        config.titles = Self::extract_title_config(style);

        config
    }

    /// Detect the processing mode (author-date, numeric, note) from citation attributes.
    fn detect_processing_mode(style: &Style) -> Option<Processing> {
        // disambiguate-add-year-suffix is a strong signal for author-date
        if style.citation.disambiguate_add_year_suffix == Some(true) {
            return Some(Processing::AuthorDate);
        }

        // Check style class attribute if available
        // CSL 1.0 uses class="in-text" or class="note"
        // For now, default to AuthorDate as it's most common
        None
    }

    /// Extract contributor formatting options from citation/bibliography.
    fn extract_contributor_config(style: &Style) -> Option<ContributorConfig> {
        let mut config = ContributorConfig::default();
        let mut has_config = false;

        // Check citation-level et-al settings
        let citation = &style.citation;
        if citation.et_al_min.is_some() || citation.et_al_use_first.is_some() {
            config.shorten = Some(ShortenListOptions {
                min: citation.et_al_min.unwrap_or(4) as u8,
                use_first: citation.et_al_use_first.unwrap_or(1) as u8,
                ..Default::default()
            });
            has_config = true;
        }

        // Check bibliography-level et-al settings (may override)
        if let Some(bib) = &style.bibliography {
            if bib.et_al_min.is_some() || bib.et_al_use_first.is_some() {
                // Only set if not already set by citation
                if config.shorten.is_none() {
                    config.shorten = Some(ShortenListOptions {
                        min: bib.et_al_min.unwrap_or(4) as u8,
                        use_first: bib.et_al_use_first.unwrap_or(1) as u8,
                        ..Default::default()
                    });
                    has_config = true;
                }
            }
        }

        // Walk macros to find <name> elements with global settings
        for macro_def in &style.macros {
            Self::extract_name_options_from_nodes(&macro_def.children, &mut config, &mut has_config);
        }

        if has_config {
            Some(config)
        } else {
            None
        }
    }

    /// Recursively search nodes for <name> elements and extract their options.
    fn extract_name_options_from_nodes(
        nodes: &[CslNode],
        config: &mut ContributorConfig,
        has_config: &mut bool,
    ) {
        for node in nodes {
            match node {
                CslNode::Names(names) => {
                    Self::extract_from_names(names, config, has_config);
                }
                CslNode::Group(g) => {
                    Self::extract_name_options_from_nodes(&g.children, config, has_config);
                }
                CslNode::Choose(c) => {
                    Self::extract_name_options_from_nodes(&c.if_branch.children, config, has_config);
                    for branch in &c.else_if_branches {
                        Self::extract_name_options_from_nodes(&branch.children, config, has_config);
                    }
                    if let Some(else_children) = &c.else_branch {
                        Self::extract_name_options_from_nodes(else_children, config, has_config);
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract options from a <names> element.
    fn extract_from_names(names: &Names, config: &mut ContributorConfig, has_config: &mut bool) {
        // Check children for <name> element
        for child in &names.children {
            if let CslNode::Name(name) = child {
                // name-as-sort-order
                if let Some(order) = &name.name_as_sort_order {
                    config.display_as_sort = Some(match order.as_str() {
                        "first" => DisplayAsSort::First,
                        "all" => DisplayAsSort::All,
                        _ => DisplayAsSort::None,
                    });
                    *has_config = true;
                }

                // and
                if let Some(and) = &name.and {
                    config.and = Some(match and.as_str() {
                        "symbol" => AndOptions::Symbol,
                        "text" => AndOptions::Text,
                        _ => AndOptions::None,
                    });
                    *has_config = true;
                }

                // delimiter
                if name.delimiter.is_some() && config.delimiter.is_none() {
                    config.delimiter = name.delimiter.clone();
                    *has_config = true;
                }
            }
        }

        // et-al from names element itself
        if names.et_al_min.is_some() && config.shorten.is_none() {
            config.shorten = Some(ShortenListOptions {
                min: names.et_al_min.unwrap_or(4) as u8,
                use_first: names.et_al_use_first.unwrap_or(1) as u8,
                ..Default::default()
            });
            *has_config = true;
        }
    }

    /// Extract substitute pattern from the first <names> with <substitute>.
    fn extract_substitute_pattern(style: &Style) -> Option<CslnSubstitute> {
        // Search macros for <substitute> elements
        for macro_def in &style.macros {
            if let Some(sub) = Self::find_substitute_in_nodes(&macro_def.children) {
                return Some(sub);
            }
        }
        None
    }

    /// Recursively find a <substitute> element and convert it.
    fn find_substitute_in_nodes(nodes: &[CslNode]) -> Option<CslnSubstitute> {
        for node in nodes {
            match node {
                CslNode::Names(names) => {
                    for child in &names.children {
                        if let CslNode::Substitute(sub) = child {
                            return Some(Self::convert_substitute(sub));
                        }
                    }
                }
                CslNode::Group(g) => {
                    if let Some(sub) = Self::find_substitute_in_nodes(&g.children) {
                        return Some(sub);
                    }
                }
                CslNode::Choose(c) => {
                    if let Some(sub) = Self::find_substitute_in_nodes(&c.if_branch.children) {
                        return Some(sub);
                    }
                    for branch in &c.else_if_branches {
                        if let Some(sub) = Self::find_substitute_in_nodes(&branch.children) {
                            return Some(sub);
                        }
                    }
                    if let Some(else_children) = &c.else_branch {
                        if let Some(sub) = Self::find_substitute_in_nodes(else_children) {
                            return Some(sub);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Convert a CSL 1.0 <substitute> to CSLN Substitute.
    fn convert_substitute(sub: &Substitute) -> CslnSubstitute {
        let mut template = Vec::new();

        for child in &sub.children {
            Self::extract_substitute_keys(child, &mut template);
        }

        CslnSubstitute {
            contributor_role_form: None, // Could be inferred from label forms
            template,
        }
    }

    /// Recursively extract substitute keys from a node.
    fn extract_substitute_keys(node: &CslNode, template: &mut Vec<SubstituteKey>) {
        match node {
            CslNode::Names(names) => {
                // Map the variable to a SubstituteKey
                for var in names.variable.split_whitespace() {
                    match var {
                        "editor" | "editorial-director" => {
                            if !template.contains(&SubstituteKey::Editor) {
                                template.push(SubstituteKey::Editor);
                            }
                        }
                        "translator" => {
                            if !template.contains(&SubstituteKey::Translator) {
                                template.push(SubstituteKey::Translator);
                            }
                        }
                        _ => {} // Other name variables not yet supported
                    }
                }
            }
            CslNode::Text(t) => {
                // Check if it's a title variable or a macro (likely containing title)
                if let Some(var) = &t.variable {
                    if var == "title" || var == "container-title" {
                        if !template.contains(&SubstituteKey::Title) {
                            template.push(SubstituteKey::Title);
                        }
                    }
                }
                // Macro calls that likely contain title
                if let Some(macro_name) = &t.macro_name {
                    if macro_name.contains("title") {
                        if !template.contains(&SubstituteKey::Title) {
                            template.push(SubstituteKey::Title);
                        }
                    }
                }
            }
            CslNode::Choose(c) => {
                // Recurse into choose branches
                for child in &c.if_branch.children {
                    Self::extract_substitute_keys(child, template);
                }
                for branch in &c.else_if_branches {
                    for child in &branch.children {
                        Self::extract_substitute_keys(child, template);
                    }
                }
                if let Some(else_children) = &c.else_branch {
                    for child in else_children {
                        Self::extract_substitute_keys(child, template);
                    }
                }
            }
            CslNode::Group(g) => {
                for child in &g.children {
                    Self::extract_substitute_keys(child, template);
                }
            }
            _ => {}
        }
    }

    /// Extract date configuration from style.
    fn extract_date_config(_style: &Style) -> Option<DateConfig> {
        // TODO: Walk dates and infer month format from date-parts
        None
    }

    /// Extract title configuration from formatting patterns.
    fn extract_title_config(_style: &Style) -> Option<TitlesConfig> {
        // TODO: Analyze title formatting in templates
        // Look for patterns like italics on container-title vs quotes on article-title
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csl_legacy::parser::parse_style;

    fn parse_csl(xml: &str) -> Result<Style, String> {
        let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
        parse_style(doc.root_element())
    }

    #[test]
    fn test_extract_author_date_processing() {
        let csl = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>Test</title></info>
  <citation disambiguate-add-year-suffix="true">
    <layout><text variable="title"/></layout>
  </citation>
  <bibliography><layout><text variable="title"/></layout></bibliography>
</style>"#;

        let style = parse_csl(csl).unwrap();
        let config = OptionsExtractor::extract(&style);

        assert_eq!(config.processing, Some(Processing::AuthorDate));
    }

    #[test]
    fn test_extract_et_al_from_citation() {
        let csl = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>Test</title></info>
  <citation et-al-min="3" et-al-use-first="1">
    <layout><text variable="title"/></layout>
  </citation>
  <bibliography><layout><text variable="title"/></layout></bibliography>
</style>"#;

        let style = parse_csl(csl).unwrap();
        let config = OptionsExtractor::extract(&style);

        assert!(config.contributors.is_some());
        let contrib = config.contributors.unwrap();
        assert!(contrib.shorten.is_some());
        let shorten = contrib.shorten.unwrap();
        assert_eq!(shorten.min, 3);
        assert_eq!(shorten.use_first, 1);
    }

    #[test]
    fn test_extract_substitute_pattern() {
        let csl = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>Test</title></info>
  <macro name="author">
    <names variable="author">
      <name/>
      <substitute>
        <names variable="editor"/>
        <text variable="title"/>
      </substitute>
    </names>
  </macro>
  <citation><layout><text macro="author"/></layout></citation>
  <bibliography><layout><text macro="author"/></layout></bibliography>
</style>"#;

        let style = parse_csl(csl).unwrap();
        let config = OptionsExtractor::extract(&style);

        assert!(config.substitute.is_some());
        let sub = config.substitute.unwrap();
        assert_eq!(sub.template.len(), 2);
        assert_eq!(sub.template[0], SubstituteKey::Editor);
        assert_eq!(sub.template[1], SubstituteKey::Title);
    }

    #[test]
    fn test_extract_from_real_apa() {
        let apa_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap()
            .parent().unwrap()
            .join("styles/apa.csl");
        
        let xml = std::fs::read_to_string(&apa_path)
            .expect("Failed to read apa.csl");
        let style = parse_csl(&xml).expect("Failed to parse apa.csl");
        let config = OptionsExtractor::extract(&style);

        // APA should have author-date processing (has disambiguate-add-year-suffix)
        assert_eq!(config.processing, Some(Processing::AuthorDate), 
            "APA should be detected as author-date style");

        // APA has et-al settings
        assert!(config.contributors.is_some(), "APA should have contributor config");
        
        // APA has substitute pattern (editor, then title)
        assert!(config.substitute.is_some(), "APA should have substitute config");

        // Print for debugging
        println!("APA Config extracted:");
        println!("  Processing: {:?}", config.processing);
        if let Some(ref contrib) = config.contributors {
            println!("  Contributors: {:?}", contrib);
        }
        if let Some(ref sub) = config.substitute {
            println!("  Substitute: {:?}", sub);
        }
    }
}
