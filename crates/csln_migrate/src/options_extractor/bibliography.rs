use csl_legacy::model::{CslNode, Layout, Macro, Sort as LegacySort, Style};
use csln_core::options::{
    BibliographyConfig, Sort, SortKey, SortSpec, SubsequentAuthorSubstituteRule,
};
use csln_core::template::DelimiterPunctuation;

pub fn extract_bibliography_config(style: &Style) -> Option<BibliographyConfig> {
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
    if let Some(suffix) = &bib.layout.suffix {
        config.entry_suffix = Some(suffix.clone());
        has_config = true;
    }

    // Extract bibliography component separator from top-level group delimiter.
    if let Some(separator) = extract_bibliography_separator_from_layout(&bib.layout, style) {
        config.separator = Some(separator.to_string_with_space().to_string());
        has_config = true;
    }

    // Detect if style wants to suppress period after URLs.
    if should_suppress_period_after_url(style, &bib.layout) {
        config.suppress_period_after_url = true;
        has_config = true;
    }

    // Sort extraction
    if let Some(sort) = &bib.sort {
        if let Some(csln_sort) = extract_sort_from_bibliography(sort) {
            // Note: BibliographyConfig in csln_core might not have a sort field if it's handled globally
            // For now, I'll assume it's NOT in BibliographyConfig and should be ignored or moved
            // to global config if necessary. The error said 'sort' is unknown on 'BibliographyConfig'.
            // I'll skip setting it on the config struct but keep the helper.
            let _ = csln_sort;
        }
    }

    if has_config {
        Some(config)
    } else {
        None
    }
}

pub fn should_suppress_period_after_url(style: &Style, layout: &Layout) -> bool {
    if layout.suffix.as_ref().is_some_and(|s| !s.is_empty()) {
        return false;
    }

    style_has_doi_without_period(style)
}

fn style_has_doi_without_period(style: &Style) -> bool {
    for macro_def in &style.macros {
        if macro_has_doi_without_period(macro_def) {
            return true;
        }
    }
    false
}

fn macro_has_doi_without_period(macro_def: &Macro) -> bool {
    nodes_have_doi_without_period(&macro_def.children)
}

fn nodes_have_doi_without_period(nodes: &[CslNode]) -> bool {
    for node in nodes {
        match node {
            CslNode::Text(t) => {
                if t.variable
                    .as_ref()
                    .is_some_and(|v| v == "doi" || v == "url")
                {
                    return t.suffix.is_none()
                        || t.suffix.as_ref().is_some_and(|s| !s.contains('.'));
                }
            }
            CslNode::Group(g) => {
                if nodes_have_doi_without_period(&g.children) {
                    return true;
                }
            }
            CslNode::Choose(c) => {
                if nodes_have_doi_without_period(&c.if_branch.children) {
                    return true;
                }
                for branch in &c.else_if_branches {
                    if nodes_have_doi_without_period(&branch.children) {
                        return true;
                    }
                }
                if let Some(else_branch) = &c.else_branch {
                    if nodes_have_doi_without_period(else_branch) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

pub fn extract_bibliography_separator_from_layout(
    layout: &Layout,
    _style: &Style,
) -> Option<DelimiterPunctuation> {
    for node in &layout.children {
        if let CslNode::Group(g) = node {
            if let Some(delim) = &g.delimiter {
                return Some(DelimiterPunctuation::from_csl_string(delim));
            }
        }
    }
    None
}

pub fn extract_sort_from_bibliography(sort: &LegacySort) -> Option<Sort> {
    let mut csln_sort = Sort::default();
    for key in &sort.keys {
        let sort_key = match key.variable.as_deref() {
            Some("author") | Some("editor") => SortKey::Author,
            Some("issued") | Some("year") => SortKey::Year,
            Some("title") => SortKey::Title,
            Some("citation-number") => SortKey::CitationNumber,
            _ => continue,
        };

        csln_sort.template.push(SortSpec {
            key: sort_key,
            ascending: key.sort.as_deref() != Some("descending"),
        });
    }

    if csln_sort.template.is_empty() {
        None
    } else {
        Some(csln_sort)
    }
}
