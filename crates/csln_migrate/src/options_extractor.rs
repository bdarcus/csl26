/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Extracts global style options from CSL 1.0 structures into CSLN Config.
//!
//! This module implements "semantic upsampling" - inferring the bibliographic
//! intent from CSL 1.0's procedural template structure and encoding it as
//! declarative options in CSLN.

use csl_legacy::model::{CslNode, Layout, Macro, Names, Sort as LegacySort, Style, Substitute};
use csln_core::options::{
    AndOptions, BibliographyConfig, Config, ContributorConfig, DateConfig, DelimiterPrecedesLast,
    DemoteNonDroppingParticle, Disambiguation, DisplayAsSort, EditorLabelFormat, Group,
    PageRangeFormat, Processing, ProcessingCustom, ShortenListOptions, Sort, SortKey, SortSpec,
    SubsequentAuthorSubstituteRule, Substitute as CslnSubstitute, SubstituteConfig, SubstituteKey,
    TitlesConfig,
};
use csln_core::template::DelimiterPunctuation;

/// Extracts global configuration options from a CSL 1.0 style.
pub struct OptionsExtractor;

impl OptionsExtractor {
    /// Extract a Config from the given CSL 1.0 style.
    pub fn extract(style: &Style) -> Config {
        Config {
            // 1. Detect processing mode from citation attributes
            processing: Self::detect_processing_mode(style),

            // 2. Extract contributor options from style-level attributes and citation/bibliography
            contributors: Self::extract_contributor_config(style),

            // 3. Extract substitute patterns from the first <names> block with <substitute>
            substitute: Self::extract_substitute_pattern(style).map(SubstituteConfig::Explicit),

            // 4. Extract date configuration
            dates: Self::extract_date_config(style),

            // 5. Extract title configuration from formatting patterns
            titles: Self::extract_title_config(style),

            // 6. Extract page range format from style-level attribute
            page_range_format: Self::extract_page_range_format(style),

            // 7. Extract bibliography-specific settings
            bibliography: Self::extract_bibliography_config(style),

            // 8. Punctuation-in-quote from locale (en-US has true by default)
            punctuation_in_quote: Self::extract_punctuation_in_quote(style),

            // 9. Volume-pages delimiter from serial source groups
            volume_pages_delimiter: Self::extract_volume_pages_delimiter(style),

            ..Config::default()
        }
    }

    /// Extract the delimiter between volume/issue and pages from serial source macros.
    /// This looks for groups that contain both volume and page variables.
    fn extract_volume_pages_delimiter(style: &Style) -> Option<DelimiterPunctuation> {
        let bib_macros = Self::collect_bibliography_macros(style);

        for macro_def in &style.macros {
            if bib_macros.contains(&macro_def.name) {
                if let Some(delimiter) =
                    Self::find_volume_pages_delimiter_in_nodes(&macro_def.children)
                {
                    return Some(DelimiterPunctuation::from_csl_string(&delimiter));
                }
            }
        }
        None
    }

    /// Recursively search for a group containing both volume and page,
    /// and extract its delimiter. Prefers innermost matching groups.
    fn find_volume_pages_delimiter_in_nodes(nodes: &[CslNode]) -> Option<String> {
        for node in nodes {
            match node {
                CslNode::Group(g) => {
                    // First, recurse into children to find innermost match
                    if let Some(delimiter) = Self::find_volume_pages_delimiter_in_nodes(&g.children)
                    {
                        return Some(delimiter);
                    }

                    // Check if this group directly contains both volume and page
                    // (not just transitively through deeply nested children)
                    let has_volume = Self::group_directly_contains_variable(&g.children, "volume");
                    let has_page = Self::group_directly_contains_variable(&g.children, "page")
                        || Self::group_contains_macro_with_page(&g.children);

                    if has_volume && has_page {
                        // Found the group - return its delimiter
                        if let Some(delim) = &g.delimiter {
                            return Some(delim.clone());
                        }
                    }
                }
                CslNode::Choose(c) => {
                    // Check all branches
                    if let Some(d) =
                        Self::find_volume_pages_delimiter_in_nodes(&c.if_branch.children)
                    {
                        return Some(d);
                    }
                    for branch in &c.else_if_branches {
                        if let Some(d) =
                            Self::find_volume_pages_delimiter_in_nodes(&branch.children)
                        {
                            return Some(d);
                        }
                    }
                    if let Some(else_children) = &c.else_branch {
                        if let Some(d) = Self::find_volume_pages_delimiter_in_nodes(else_children) {
                            return Some(d);
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Check if a group's direct children (one level deep only) contain a variable.
    /// This avoids matching outer groups that only transitively contain the variable.
    fn group_directly_contains_variable(nodes: &[CslNode], var_name: &str) -> bool {
        for node in nodes {
            match node {
                CslNode::Text(t) => {
                    if t.variable.as_ref().is_some_and(|v| v == var_name) {
                        return true;
                    }
                }
                CslNode::Number(n) => {
                    if n.variable == var_name {
                        return true;
                    }
                }
                // Check one level of nesting (group containing the variable directly)
                CslNode::Group(g) => {
                    for child in &g.children {
                        match child {
                            CslNode::Text(t) => {
                                if t.variable.as_ref().is_some_and(|v| v == var_name) {
                                    return true;
                                }
                            }
                            CslNode::Number(n) => {
                                if n.variable == var_name {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                CslNode::Choose(c) => {
                    // Check choose branches at this level
                    if Self::group_directly_contains_variable(&c.if_branch.children, var_name) {
                        return true;
                    }
                    for branch in &c.else_if_branches {
                        if Self::group_directly_contains_variable(&branch.children, var_name) {
                            return true;
                        }
                    }
                    if let Some(else_children) = &c.else_branch {
                        if Self::group_directly_contains_variable(else_children, var_name) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if nodes contain a macro call that likely contains page variable.
    /// This handles cases where page is in a separate macro like "source-serial-locator".
    fn group_contains_macro_with_page(nodes: &[CslNode]) -> bool {
        for node in nodes {
            if let CslNode::Text(t) = node {
                if let Some(macro_name) = &t.macro_name {
                    // Common macro names that contain page variable
                    if macro_name.contains("locator")
                        || macro_name.contains("page")
                        || macro_name.contains("pages")
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Extract punctuation-in-quote setting from locale.
    /// en-US locale uses American style (punctuation inside quotes).
    /// Most other locales default to false.
    fn extract_punctuation_in_quote(style: &Style) -> bool {
        // Check default-locale attribute; en-US/en-GB have different conventions
        // en-US: punctuation inside quotes (true)
        // en-GB and most others: punctuation outside quotes (false)
        match style.default_locale.as_deref() {
            Some(locale) if locale.starts_with("en-US") => true,
            Some(locale) if locale.starts_with("en-GB") => false,
            // Default to true for unspecified (CSL defaults to en-US)
            // or generic "en" which is typically American
            Some(locale) if locale.starts_with("en") => true,
            None => true, // CSL default locale is en-US
            _ => false,   // Other languages typically don't use American style
        }
    }

    /// Extract page range format from style-level page-range-format attribute.
    fn extract_page_range_format(style: &Style) -> Option<PageRangeFormat> {
        style
            .page_range_format
            .as_ref()
            .and_then(|f| match f.as_str() {
                "expanded" => Some(PageRangeFormat::Expanded),
                "minimal" => Some(PageRangeFormat::Minimal),
                "minimal-two" => Some(PageRangeFormat::MinimalTwo),
                "chicago" => Some(PageRangeFormat::Chicago),
                "chicago-15" => Some(PageRangeFormat::Chicago),
                "chicago-16" => Some(PageRangeFormat::Chicago16),
                _ => None,
            })
    }

    /// Extract bibliography-specific configuration.
    fn extract_bibliography_config(style: &Style) -> Option<BibliographyConfig> {
        let bib = style.bibliography.as_ref()?;

        let mut config = BibliographyConfig::default();
        let mut has_config = false;

        if let Some(sub) = &bib.subsequent_author_substitute {
            config.subsequent_author_substitute = Some(sub.clone());
            has_config = true;
        }

        if let Some(rule) = &bib.subsequent_author_substitute_rule {
            config.subsequent_author_substitute_rule = match rule.as_str() {
                "complete-all" => Some(SubsequentAuthorSubstituteRule::CompleteAll),
                "complete-each" => Some(SubsequentAuthorSubstituteRule::CompleteEach),
                "partial-each" => Some(SubsequentAuthorSubstituteRule::PartialEach),
                "partial-first" => Some(SubsequentAuthorSubstituteRule::PartialFirst),
                _ => Some(SubsequentAuthorSubstituteRule::CompleteAll),
            };
            has_config = true;
        }

        if let Some(hanging) = bib.hanging_indent {
            config.hanging_indent = Some(hanging);
            has_config = true;
        }

        // Extract layout suffix (e.g., "." from `<layout suffix=".">`).
        // This controls entry-terminating punctuation.
        if let Some(suffix) = &bib.layout.suffix {
            config.entry_suffix = Some(suffix.clone());
            has_config = true;
        }

        // Extract bibliography component separator from top-level group delimiter.
        // Chicago/APA typically use ". " while Elsevier uses ", ".
        if let Some(separator) = Self::extract_bibliography_separator_from_layout(&bib.layout) {
            config.separator = Some(separator);
            has_config = true;
        }

        // Detect if style wants to suppress period after URLs.
        // APA 7th and Bluebook omit the period after DOI/URL to avoid breaking links.
        // Heuristic: check if DOI/URL appears at the end of bibliography macros
        // without a suffix that adds a period.
        if Self::should_suppress_period_after_url(style, &bib.layout) {
            config.suppress_period_after_url = true;
            has_config = true;
        }

        if has_config {
            Some(config)
        } else {
            None
        }
    }

    /// Detect if a bibliography layout should suppress the period after URLs.
    ///
    /// Returns true if:
    /// 1. The layout has no suffix or an empty suffix, AND
    /// 2. DOI or URL appears in the style without a period suffix
    ///
    /// This matches APA 7th behavior where DOI/URL ends the entry without a period.
    fn should_suppress_period_after_url(style: &Style, layout: &Layout) -> bool {
        // If layout has an explicit period suffix, don't suppress
        if let Some(suffix) = &layout.suffix {
            if suffix.contains('.') {
                return false;
            }
        }

        // Check if DOI/URL appears in macros without period suffix.
        // This catches APA-style where DOI is rendered via macro.
        Self::style_has_doi_without_period(style)
    }

    /// Check if style has DOI/URL text nodes without a period suffix.
    /// This indicates APA-style behavior where DOI ends without trailing period.
    fn style_has_doi_without_period(style: &Style) -> bool {
        for macro_def in &style.macros {
            if Self::macro_has_doi_without_period(macro_def) {
                return true;
            }
        }
        false
    }

    /// Check if a macro contains DOI/URL without a period suffix.
    fn macro_has_doi_without_period(macro_def: &Macro) -> bool {
        Self::nodes_have_doi_without_period(&macro_def.children)
    }

    /// Recursively check nodes for DOI/URL without period suffix.
    fn nodes_have_doi_without_period(nodes: &[CslNode]) -> bool {
        for node in nodes {
            match node {
                CslNode::Text(text) => {
                    if let Some(var) = &text.variable {
                        let var_lower = var.to_lowercase();
                        if var_lower == "doi" || var_lower == "url" {
                            // Check if suffix contains period
                            let has_period_suffix =
                                text.suffix.as_ref().is_some_and(|s| s.contains('.'));
                            if !has_period_suffix {
                                return true;
                            }
                        }
                    }
                }
                CslNode::Group(group) => {
                    if Self::nodes_have_doi_without_period(&group.children) {
                        return true;
                    }
                }
                CslNode::Choose(choose) => {
                    if Self::nodes_have_doi_without_period(&choose.if_branch.children) {
                        return true;
                    }
                    for else_if in &choose.else_if_branches {
                        if Self::nodes_have_doi_without_period(&else_if.children) {
                            return true;
                        }
                    }
                    if let Some(else_children) = &choose.else_branch {
                        if Self::nodes_have_doi_without_period(else_children) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Extract bibliography component separator from CSL layout.
    ///
    /// The bibliography separator is the delimiter between bibliography components.
    /// In CSL 1.0, this is typically the delimiter on the top-level group in the
    /// bibliography layout.
    fn extract_bibliography_separator_from_layout(layout: &Layout) -> Option<String> {
        // Check if the layout children start with a top-level group
        for node in &layout.children {
            if let CslNode::Group(group) = node {
                // If this group has a delimiter, use it as the bibliography separator
                if let Some(ref delimiter) = group.delimiter {
                    return Some(delimiter.to_string());
                }
            }
        }

        // No delimiter found - return None (processor will use default ". ")
        None
    }

    /// Extract sort configuration from CSL 1.0 bibliography.
    ///
    /// Maps CSL sort keys to CSLN SortKey enum:
    /// - `citation-number` → CitationNumber (numeric styles)
    /// - `author` or macro containing "author" → Author
    /// - `issued` or macro containing "date"/"year" → Year
    /// - `title` → Title
    fn extract_sort_from_bibliography(sort: &LegacySort) -> Option<Sort> {
        let mut specs = Vec::new();

        for key in &sort.keys {
            let ascending = key.sort.as_deref() != Some("descending");

            // Determine sort key from variable or macro name
            let sort_key = if let Some(var) = &key.variable {
                match var.as_str() {
                    "citation-number" => Some(SortKey::CitationNumber),
                    "author" => Some(SortKey::Author),
                    "issued" => Some(SortKey::Year),
                    "title" => Some(SortKey::Title),
                    _ => None,
                }
            } else if let Some(macro_name) = &key.macro_name {
                let m = macro_name.to_lowercase();
                if m.contains("author") || m.contains("contributor") {
                    Some(SortKey::Author)
                } else if m.contains("date") || m.contains("year") || m.contains("issued") {
                    Some(SortKey::Year)
                } else if m.contains("title") {
                    Some(SortKey::Title)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(sk) = sort_key {
                specs.push(SortSpec { key: sk, ascending });
            }
        }

        if specs.is_empty() {
            None
        } else {
            Some(Sort {
                shorten_names: false,
                render_substitutions: false,
                template: specs,
            })
        }
    }

    /// Detect the processing mode (author-date, numeric, note) from citation attributes.
    fn detect_processing_mode(style: &Style) -> Option<Processing> {
        // Check if this is a numeric style by looking at citation sort
        // Numeric styles typically sort citations by citation-number
        let is_numeric_citation = style.citation.sort.as_ref().is_some_and(|s| {
            s.keys.iter().any(|k| {
                k.variable.as_deref() == Some("citation-number")
                    || k.macro_name.as_deref() == Some("citation-number")
            })
        });

        if is_numeric_citation {
            // Numeric styles: preserve citation order (no sort needed - use insertion order)
            return Some(Processing::Numeric);
        }

        // First, extract sort configuration from bibliography if available
        let bib_sort = style
            .bibliography
            .as_ref()
            .and_then(|b| b.sort.as_ref())
            .and_then(Self::extract_sort_from_bibliography);

        // Check if this is a numeric style (sorts by citation-number)
        let is_numeric = bib_sort.as_ref().is_some_and(|s| {
            s.template
                .first()
                .is_some_and(|spec| spec.key == SortKey::CitationNumber)
        });

        if is_numeric {
            return Some(Processing::Custom(ProcessingCustom {
                sort: bib_sort,
                group: None,
                disambiguate: None,
            }));
        }

        // disambiguate-add-year-suffix is a strong signal for author-date
        if style.citation.disambiguate_add_year_suffix == Some(true) {
            let names = style.citation.disambiguate_add_names.unwrap_or(false);
            let add_givenname = style.citation.disambiguate_add_givenname.unwrap_or(false);

            // Standard AuthorDate profile in CSLN is names=true, givenname=true
            // Use the enum which has built-in Author+Year sort
            if names && add_givenname {
                return Some(Processing::AuthorDate);
            }

            // Custom author-date config with extracted sort or default
            let sort = bib_sort.unwrap_or_else(|| Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![
                    SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Year,
                        ascending: true,
                    },
                ],
            });

            return Some(Processing::Custom(ProcessingCustom {
                sort: Some(sort),
                group: Some(Group {
                    template: vec![SortKey::Author, SortKey::Year],
                }),
                disambiguate: Some(Disambiguation {
                    names,
                    add_givenname,
                    year_suffix: true,
                }),
            }));
        }

        // If we have explicit sort but no disambiguation, still use it
        if let Some(sort) = bib_sort {
            return Some(Processing::Custom(ProcessingCustom {
                sort: Some(sort),
                group: None,
                disambiguate: None,
            }));
        }

        // Check style class attribute if available
        // CSL 1.0 uses class="in-text" or class="note"
        // For now, default to AuthorDate as it's most common
        None
    }

    /// Extract contributor formatting options from style-level attributes and bibliography context.
    ///
    /// This only extracts name options from macros that are transitively called from
    /// the bibliography layout. This avoids picking up citation-specific or legal-specific
    /// name formats that shouldn't apply to general bibliography rendering.
    fn extract_contributor_config(style: &Style) -> Option<ContributorConfig> {
        let mut config = ContributorConfig::default();
        let mut has_config = false;

        // Style-level options (on <style> element)
        // These are inherited by all names unless overridden
        if let Some(init_with) = &style.initialize_with {
            config.initialize_with = Some(init_with.clone());
            has_config = true;
        }
        if let Some(init_hyphen) = style.initialize_with_hyphen {
            config.initialize_with_hyphen = Some(init_hyphen);
            has_config = true;
        }
        if let Some(and) = &style.and {
            config.and = match and.as_str() {
                "text" => Some(AndOptions::Text),
                "symbol" => Some(AndOptions::Symbol),
                _ => None,
            };
            if config.and.is_some() {
                has_config = true;
            }
        }
        if let Some(sort_order) = &style.name_as_sort_order {
            config.display_as_sort = match sort_order.as_str() {
                "first" => Some(DisplayAsSort::First),
                "all" => Some(DisplayAsSort::All),
                _ => None,
            };
            if config.display_as_sort.is_some() {
                has_config = true;
            }
        }
        // Extract names-delimiter from style level (this is the canonical delimiter)
        if let Some(delim) = &style.names_delimiter {
            config.delimiter = Some(delim.clone());
            has_config = true;
        }
        // Extract 'and' from style level if present
        if let Some(and) = &style.and {
            config.and = Some(match and.as_str() {
                "symbol" => AndOptions::Symbol,
                "text" => AndOptions::Text,
                _ => AndOptions::None,
            });
            has_config = true;
        }
        if let Some(dpl) = &style.delimiter_precedes_last {
            config.delimiter_precedes_last = match dpl.as_str() {
                "contextual" => Some(DelimiterPrecedesLast::Contextual),
                "after-inverted-name" => Some(DelimiterPrecedesLast::AfterInvertedName),
                "always" => Some(DelimiterPrecedesLast::Always),
                "never" => Some(DelimiterPrecedesLast::Never),
                _ => None,
            };
            if config.delimiter_precedes_last.is_some() {
                has_config = true;
            }
        }
        if let Some(dpea) = &style.delimiter_precedes_et_al {
            config.delimiter_precedes_et_al = match dpea.as_str() {
                "contextual" => Some(DelimiterPrecedesLast::Contextual),
                "after-inverted-name" => Some(DelimiterPrecedesLast::AfterInvertedName),
                "always" => Some(DelimiterPrecedesLast::Always),
                "never" => Some(DelimiterPrecedesLast::Never),
                _ => None,
            };
            if config.delimiter_precedes_et_al.is_some() {
                has_config = true;
            }
        }
        if let Some(dndp) = &style.demote_non_dropping_particle {
            config.demote_non_dropping_particle = match dndp.as_str() {
                "never" => Some(DemoteNonDroppingParticle::Never),
                "sort-only" => Some(DemoteNonDroppingParticle::SortOnly),
                "display-and-sort" => Some(DemoteNonDroppingParticle::DisplayAndSort),
                _ => None,
            };
            if config.demote_non_dropping_particle.is_some() {
                has_config = true;
            }
        }

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

        // Collect macros from both citation and bibliography layouts
        let bib_macros = Self::collect_bibliography_macros(style);
        let cite_macros = Self::collect_citation_macros(style);

        // Extract name options from macros used in either context.
        // For initialize-with specifically, only extract from bibliography-only macros
        // to avoid picking up citation-specific initials (e.g., Chicago uses initials
        // in citations but full names in bibliography).
        // Other options like 'and' should be extracted from both contexts.
        for macro_def in &style.macros {
            let in_bib = bib_macros.contains(&macro_def.name);
            let in_cite = cite_macros.contains(&macro_def.name);
            if in_bib || in_cite {
                // Only extract initialize-with from bibliography-only macros
                let extract_initialize = in_bib && !in_cite;
                Self::extract_name_options_from_nodes(
                    &macro_def.children,
                    &mut config,
                    &mut has_config,
                    extract_initialize,
                );
            }
        }

        if has_config {
            Some(config)
        } else {
            None
        }
    }

    /// Collect all macro names that are transitively called from bibliography layout.
    fn collect_bibliography_macros(style: &Style) -> std::collections::HashSet<String> {
        let mut macros = std::collections::HashSet::new();

        // Start with macros called directly from bibliography layout
        if let Some(bib) = &style.bibliography {
            Self::collect_macro_refs_from_nodes(&bib.layout.children, &mut macros);
        }

        // Transitively expand: for each macro in the set, add macros it calls
        let macro_map: std::collections::HashMap<&str, &Macro> =
            style.macros.iter().map(|m| (m.name.as_str(), m)).collect();

        let mut changed = true;
        while changed {
            changed = false;
            let current: Vec<String> = macros.iter().cloned().collect();
            for name in current {
                if let Some(macro_def) = macro_map.get(name.as_str()) {
                    let before = macros.len();
                    Self::collect_macro_refs_from_nodes(&macro_def.children, &mut macros);
                    if macros.len() > before {
                        changed = true;
                    }
                }
            }
        }

        macros
    }

    /// Collect all macro names that are transitively called from citation layout.
    fn collect_citation_macros(style: &Style) -> std::collections::HashSet<String> {
        let mut macros = std::collections::HashSet::new();

        // Start with macros called directly from citation layout
        Self::collect_macro_refs_from_nodes(&style.citation.layout.children, &mut macros);

        // Transitively expand: for each macro in the set, add macros it calls
        let macro_map: std::collections::HashMap<&str, &Macro> =
            style.macros.iter().map(|m| (m.name.as_str(), m)).collect();

        let mut changed = true;
        while changed {
            changed = false;
            let current: Vec<String> = macros.iter().cloned().collect();
            for name in current {
                if let Some(macro_def) = macro_map.get(name.as_str()) {
                    let before = macros.len();
                    Self::collect_macro_refs_from_nodes(&macro_def.children, &mut macros);
                    if macros.len() > before {
                        changed = true;
                    }
                }
            }
        }

        macros
    }

    /// Collect macro names referenced in nodes via <text macro="..."/>.
    fn collect_macro_refs_from_nodes(
        nodes: &[CslNode],
        macros: &mut std::collections::HashSet<String>,
    ) {
        for node in nodes {
            match node {
                CslNode::Text(t) => {
                    if let Some(macro_name) = &t.macro_name {
                        macros.insert(macro_name.clone());
                    }
                }
                CslNode::Group(g) => {
                    Self::collect_macro_refs_from_nodes(&g.children, macros);
                }
                CslNode::Choose(c) => {
                    Self::collect_macro_refs_from_nodes(&c.if_branch.children, macros);
                    for branch in &c.else_if_branches {
                        Self::collect_macro_refs_from_nodes(&branch.children, macros);
                    }
                    if let Some(else_children) = &c.else_branch {
                        Self::collect_macro_refs_from_nodes(else_children, macros);
                    }
                }
                CslNode::Names(n) => {
                    Self::collect_macro_refs_from_nodes(&n.children, macros);
                }
                _ => {}
            }
        }
    }

    /// Recursively search nodes for <name> elements and extract their options.
    ///
    /// `extract_initialize` controls whether to extract `initialize-with` from this context.
    /// Should be true only for bibliography-only macros to avoid picking up citation-specific
    /// initials that shouldn't apply to bibliography output (e.g., Chicago uses initials
    /// in citations but full names in bibliography).
    fn extract_name_options_from_nodes(
        nodes: &[CslNode],
        config: &mut ContributorConfig,
        has_config: &mut bool,
        extract_initialize: bool,
    ) {
        for node in nodes {
            match node {
                CslNode::Names(names) => {
                    Self::extract_from_names(names, config, has_config, extract_initialize);
                }
                CslNode::Group(g) => {
                    Self::extract_name_options_from_nodes(
                        &g.children,
                        config,
                        has_config,
                        extract_initialize,
                    );
                }
                CslNode::Choose(c) => {
                    Self::extract_name_options_from_nodes(
                        &c.if_branch.children,
                        config,
                        has_config,
                        extract_initialize,
                    );
                    for branch in &c.else_if_branches {
                        Self::extract_name_options_from_nodes(
                            &branch.children,
                            config,
                            has_config,
                            extract_initialize,
                        );
                    }
                    if let Some(else_children) = &c.else_branch {
                        Self::extract_name_options_from_nodes(
                            else_children,
                            config,
                            has_config,
                            extract_initialize,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract options from a <names> element.
    ///
    /// `extract_initialize` controls whether to extract `initialize-with` from this context.
    /// Should be true only for bibliography-only macros to avoid applying citation-specific
    /// initials to bibliography output.
    fn extract_from_names(
        names: &Names,
        config: &mut ContributorConfig,
        has_config: &mut bool,
        extract_initialize: bool,
    ) {
        // Check children for <name> element
        for child in &names.children {
            if let CslNode::Name(name) = child {
                // initialize-with (controls whether names are initialized to initials)
                // Only extract from bibliography-only macros to avoid picking up
                // citation-specific initials (e.g., Chicago uses initials in citations
                // but full given names in bibliography).
                if extract_initialize && config.initialize_with.is_none() {
                    if let Some(init_with) = &name.initialize_with {
                        config.initialize_with = Some(init_with.clone());
                        *has_config = true;
                    }
                }

                // initialize-with-hyphen (also only from bibliography context)
                if extract_initialize {
                    if let Some(hyphen) = name.initialize_with_hyphen {
                        config.initialize_with_hyphen = Some(hyphen);
                        *has_config = true;
                    }
                }

                // name-as-sort-order
                if let Some(order) = &name.name_as_sort_order {
                    config.display_as_sort = Some(match order.as_str() {
                        "first" => DisplayAsSort::First,
                        "all" => DisplayAsSort::All,
                        _ => DisplayAsSort::None,
                    });
                    *has_config = true;
                }

                // Extract 'and' from name elements
                // Prefer bibliography context (name-as-sort-order present) but also extract
                // from citation context if not yet found
                if let Some(and) = &name.and {
                    let is_bib_context = name.name_as_sort_order.is_some();
                    // Extract if: not yet set, OR this is bib context (override citation setting)
                    if config.and.is_none() || is_bib_context {
                        config.and = Some(match and.as_str() {
                            "symbol" => AndOptions::Symbol,
                            "text" => AndOptions::Text,
                            _ => AndOptions::None,
                        });
                        *has_config = true;
                    }
                }

                // delimiter - only extract from name elements with name-as-sort-order
                // to avoid picking up specialized delimiters (e.g., "-" for treaty parties)
                if name.delimiter.is_some() && config.delimiter.is_none() {
                    let is_primary_bib_format = name.name_as_sort_order.is_some();
                    if is_primary_bib_format {
                        config.delimiter = name.delimiter.clone();
                        *has_config = true;
                    }
                }

                // delimiter-precedes-last (from <name> element)
                if let Some(dpl) = &name.delimiter_precedes_last {
                    config.delimiter_precedes_last = match dpl.as_str() {
                        "contextual" => Some(DelimiterPrecedesLast::Contextual),
                        "after-inverted-name" => Some(DelimiterPrecedesLast::AfterInvertedName),
                        "always" => Some(DelimiterPrecedesLast::Always),
                        "never" => Some(DelimiterPrecedesLast::Never),
                        _ => None,
                    };
                    if config.delimiter_precedes_last.is_some() {
                        *has_config = true;
                    }
                }
            }
        }

        // Check for editor label format
        if (names.variable.contains("editor") || names.variable.contains("translator"))
            && config.editor_label_format.is_none()
        {
            let mut label_pos = None;
            let mut name_pos = None;
            let mut label_form = None;

            for (i, child) in names.children.iter().enumerate() {
                match child {
                    CslNode::Name(_) => name_pos = Some(i),
                    CslNode::Label(label) => {
                        label_pos = Some(i);
                        label_form = label.form.clone();
                    }
                    _ => {}
                }
            }

            if let (Some(l_pos), Some(n_pos)) = (label_pos, name_pos) {
                if l_pos < n_pos {
                    // Label before name (VerbPrefix)
                    config.editor_label_format = Some(EditorLabelFormat::VerbPrefix);
                    *has_config = true;
                } else {
                    // Label after name
                    config.editor_label_format = match label_form.as_deref() {
                        Some("short") => Some(EditorLabelFormat::ShortSuffix),
                        Some("long") | Some("verb") => Some(EditorLabelFormat::LongSuffix),
                        _ => Some(EditorLabelFormat::ShortSuffix), // Default to short for suffix
                    };
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

        // Also search layout
        if let Some(sub) = Self::find_substitute_in_nodes(&style.citation.layout.children) {
            return Some(sub);
        }
        if let Some(bib) = &style.bibliography {
            if let Some(sub) = Self::find_substitute_in_nodes(&bib.layout.children) {
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
        let mut overrides = std::collections::HashMap::new();

        for child in &sub.children {
            Self::extract_substitute_keys(child, &mut template, &mut overrides);
        }

        CslnSubstitute {
            contributor_role_form: None, // Could be inferred from label forms
            template,
            overrides,
        }
    }

    /// Recursively extract substitute keys from a node.
    fn extract_substitute_keys(
        node: &CslNode,
        template: &mut Vec<SubstituteKey>,
        overrides: &mut std::collections::HashMap<String, Vec<SubstituteKey>>,
    ) {
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
                    if (var == "title" || var == "container-title")
                        && !template.contains(&SubstituteKey::Title)
                    {
                        template.push(SubstituteKey::Title);
                    }
                }
                // Macro calls that likely contain title
                if let Some(macro_name) = &t.macro_name {
                    if macro_name.contains("title") && !template.contains(&SubstituteKey::Title) {
                        template.push(SubstituteKey::Title);
                    }
                }
            }
            CslNode::Choose(c) => {
                // Extract type-conditional substitutions.
                // See: https://github.com/bdarcus/csl26/issues/66

                // Handle if branch
                if let Some(types) = &c.if_branch.type_ {
                    let mut type_template = Vec::new();
                    for child in &c.if_branch.children {
                        Self::extract_substitute_keys(child, &mut type_template, overrides);
                    }
                    if !type_template.is_empty() {
                        for t in types.split_whitespace() {
                            overrides.insert(t.to_string(), type_template.clone());
                        }
                    }
                }

                // Handle else-if branches
                for elseif in &c.else_if_branches {
                    if let Some(types) = &elseif.type_ {
                        let mut type_template = Vec::new();
                        for child in &elseif.children {
                            Self::extract_substitute_keys(child, &mut type_template, overrides);
                        }
                        if !type_template.is_empty() {
                            for t in types.split_whitespace() {
                                overrides.insert(t.to_string(), type_template.clone());
                            }
                        }
                    }
                }

                // Handle else branch (adds to the default template)
                if let Some(else_nodes) = &c.else_branch {
                    for child in else_nodes {
                        Self::extract_substitute_keys(child, template, overrides);
                    }
                }
            }
            CslNode::Group(g) => {
                for child in &g.children {
                    Self::extract_substitute_keys(child, template, overrides);
                }
            }
            _ => {}
        }
    }

    /// Extract date configuration from style.
    fn extract_date_config(style: &Style) -> Option<DateConfig> {
        let mut formats = std::collections::HashMap::new();

        // Scan all macros for date formatting
        for macro_def in &style.macros {
            Self::scan_for_month_format(&macro_def.children, &mut formats);
        }

        // Also scan citation and bibliography layouts if they have direct date calls
        Self::scan_for_month_format(&style.citation.layout.children, &mut formats);

        if let Some(bib) = &style.bibliography {
            Self::scan_for_month_format(&bib.layout.children, &mut formats);
        }

        // Find most frequent format
        let best_format = formats
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(fmt, _)| fmt);

        best_format.map(|month| DateConfig {
            month,
            _extra: std::collections::HashMap::new(),
        })
    }

    /// Recursively scan nodes for month format in date parts.
    fn scan_for_month_format(
        nodes: &[CslNode],
        formats: &mut std::collections::HashMap<csln_core::options::MonthFormat, usize>,
    ) {
        use csln_core::options::MonthFormat;

        for node in nodes {
            match node {
                CslNode::Date(d) => {
                    // Check date-parts attribute first
                    if let Some(parts) = &d.date_parts {
                        if parts == "year-month-day" || parts == "year-month" {
                            // Default is usually numeric or long depending on context,
                            // but explicit date-part children override this.
                        }
                    }

                    // Check child date-part elements
                    // Note: Date parts are in `parts` field, not `children` CslNode variant
                    for part in &d.parts {
                        if part.name == "month" {
                            let fmt = match part.form.as_deref() {
                                Some("short") => MonthFormat::Short,
                                Some("numeric") | Some("numeric-leading-zeros") => {
                                    MonthFormat::Numeric
                                }
                                Some("long") | None => MonthFormat::Long,
                                _ => MonthFormat::Long,
                            };
                            *formats.entry(fmt).or_insert(0) += 1;
                        }
                    }
                }
                CslNode::Group(g) => Self::scan_for_month_format(&g.children, formats),
                CslNode::Choose(c) => {
                    // if_branch is direct
                    Self::scan_for_month_format(&c.if_branch.children, formats);

                    for elseif in &c.else_if_branches {
                        Self::scan_for_month_format(&elseif.children, formats);
                    }
                    // else_branch is Option<Vec<CslNode>>
                    if let Some(else_nodes) = &c.else_branch {
                        Self::scan_for_month_format(else_nodes, formats);
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract title formatting configuration by scanning for quote/emph usage on titles.
    fn extract_title_config(style: &Style) -> Option<TitlesConfig> {
        let mut component_quotes = false;
        let mut periodical_emph = false;

        // Scan bibliography macros for title formatting patterns
        let bib_macros = Self::collect_bibliography_macros(style);

        for macro_def in &style.macros {
            if bib_macros.contains(&macro_def.name) {
                Self::scan_for_title_formatting(
                    &macro_def.children,
                    &mut component_quotes,
                    &mut periodical_emph,
                );
            }
        }

        if component_quotes || periodical_emph {
            Some(TitlesConfig {
                component: if component_quotes {
                    Some(csln_core::options::TitleRendering {
                        quote: Some(true),
                        ..Default::default()
                    })
                } else {
                    None
                },
                periodical: if periodical_emph {
                    Some(csln_core::options::TitleRendering {
                        emph: Some(true),
                        ..Default::default()
                    })
                } else {
                    None
                },
                ..Default::default()
            })
        } else {
            None
        }
    }

    /// Recursively scan nodes for title formatting (quotes on title variable).
    fn scan_for_title_formatting(
        nodes: &[CslNode],
        component_quotes: &mut bool,
        periodical_emph: &mut bool,
    ) {
        for node in nodes {
            match node {
                CslNode::Text(t) => {
                    // Check for quotes on title variable (article/chapter titles)
                    if t.variable.as_ref().is_some_and(|v| v == "title") && t.quotes == Some(true) {
                        *component_quotes = true;
                    }
                    // Check for italics on container-title (journal/periodical names)
                    if t.variable.as_ref().is_some_and(|v| v == "container-title")
                        && t.formatting
                            .font_style
                            .as_ref()
                            .is_some_and(|s| s == "italic")
                    {
                        *periodical_emph = true;
                    }
                }
                CslNode::Group(g) => {
                    Self::scan_for_title_formatting(&g.children, component_quotes, periodical_emph);
                }
                CslNode::Choose(c) => {
                    Self::scan_for_title_formatting(
                        &c.if_branch.children,
                        component_quotes,
                        periodical_emph,
                    );
                    for branch in &c.else_if_branches {
                        Self::scan_for_title_formatting(
                            &branch.children,
                            component_quotes,
                            periodical_emph,
                        );
                    }
                    if let Some(else_children) = &c.else_branch {
                        Self::scan_for_title_formatting(
                            else_children,
                            component_quotes,
                            periodical_emph,
                        );
                    }
                }
                _ => {}
            }
        }
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
  <citation disambiguate-add-year-suffix="true" disambiguate-add-names="true" disambiguate-add-givenname="true">
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
        let sub = config.substitute.unwrap().resolve();
        assert_eq!(sub.template.len(), 2);
        assert_eq!(sub.template[0], SubstituteKey::Editor);
        assert_eq!(sub.template[1], SubstituteKey::Title);
    }

    #[test]
    fn test_extract_from_real_apa() {
        let apa_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("styles/apa.csl");

        let xml = std::fs::read_to_string(&apa_path).expect("Failed to read apa.csl");
        let style = parse_csl(&xml).expect("Failed to parse apa.csl");
        let config = OptionsExtractor::extract(&style);

        // APA should have author-date processing (has disambiguate-add-year-suffix)
        assert_eq!(
            config.processing,
            Some(Processing::AuthorDate),
            "APA should be detected as author-date style"
        );

        // APA has et-al settings
        assert!(
            config.contributors.is_some(),
            "APA should have contributor config"
        );

        // APA has substitute pattern (editor, then title)
        assert!(
            config.substitute.is_some(),
            "APA should have substitute config"
        );

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

    #[test]
    fn test_extract_editor_label_format() {
        // Test VerbPrefix (Chicago style)
        let chicago_csl = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>Chicago</title></info>
  <macro name="editor">
    <names variable="editor">
      <label form="verb" suffix=" "/>
      <name/>
    </names>
  </macro>
  <citation><layout><text macro="editor"/></layout></citation>
  <bibliography><layout><text macro="editor"/></layout></bibliography>
</style>"#;

        let style = parse_csl(chicago_csl).unwrap();
        let config = OptionsExtractor::extract(&style);
        assert_eq!(
            config.contributors.unwrap().editor_label_format,
            Some(EditorLabelFormat::VerbPrefix)
        );

        // Test ShortSuffix (APA style)
        let apa_csl = r#"<?xml version="1.0" encoding="utf-8"?>
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><title>APA</title></info>
  <macro name="editor">
    <names variable="editor">
      <name/>
      <label form="short" prefix=" (" suffix=")"/>
    </names>
  </macro>
  <citation><layout><text macro="editor"/></layout></citation>
  <bibliography><layout><text macro="editor"/></layout></bibliography>
</style>"#;

        let style = parse_csl(apa_csl).unwrap();
        let config = OptionsExtractor::extract(&style);
        assert_eq!(
            config.contributors.unwrap().editor_label_format,
            Some(EditorLabelFormat::ShortSuffix)
        );
    }
}
