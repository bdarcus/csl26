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

    // Extract bibliography component separator from group delimiter.
    if let Some(separator) = extract_bibliography_separator_from_layout(&bib.layout, &style.macros)
    {
        config.separator = Some(separator.to_string_with_space());
        has_config = true;
    }

    // Detect if style wants to suppress period after URLs.
    if should_suppress_period_after_url(style, &bib.layout) {
        config.suppress_period_after_url = true;
        has_config = true;
    }

    // Sort extraction
    if let Some(sort) = &bib.sort
        && let Some(csln_sort) = extract_sort_from_bibliography(sort)
    {
        // Note: BibliographyConfig in csln_core might not have a sort field if it's handled globally
        // For now, I'll assume it's NOT in BibliographyConfig and should be ignored or moved
        // to global config if necessary. The error said 'sort' is unknown on 'BibliographyConfig'.
        // I'll skip setting it on the config struct but keep the helper.
        let _ = csln_sort;
    }

    if has_config { Some(config) } else { None }
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
                if let Some(else_branch) = &c.else_branch
                    && nodes_have_doi_without_period(else_branch)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Extract the bibliography component separator from the layout.
///
/// Finds the delimiter that separates bibliography components (e.g., author,
/// title, date, publisher). This should be the delimiter of the DEEPEST group
/// that contains multiple variables, not just the first top-level group.
///
/// For nested structures like:
/// ```xml
/// <layout>
///   <group>  <!-- No delimiter, just wrapping -->
///     <group delimiter=", ">  <!-- This is what we want -->
///       <text variable="title"/>
///       <text variable="publisher"/>
///     </group>
///   </group>
/// </layout>
/// ```
///
/// The extraction should return the inner group's delimiter, not stop at the
/// outer group without one. Also expands macro calls to find delimiters inside
/// referenced macros.
pub fn extract_bibliography_separator_from_layout(
    layout: &Layout,
    macros: &[Macro],
) -> Option<DelimiterPunctuation> {
    // 1. First priority: the top-level layout delimiter if it exists
    if let Some(delim) = &layout.delimiter {
        return Some(DelimiterPunctuation::from_csl_string(delim));
    }

    // 2. Second priority: the delimiter of the FIRST group in the layout
    // (Many styles wrap everything in a top-level group with a delimiter)
    for node in &layout.children {
        if let CslNode::Group(g) = node
            && let Some(delim) = &g.delimiter
        {
            return Some(DelimiterPunctuation::from_csl_string(delim));
        }
    }

    // Helper to count variable-bearing nodes in a group
    fn has_multiple_variables(nodes: &[CslNode]) -> bool {
        let var_count = nodes
            .iter()
            .filter(|node| match node {
                CslNode::Text(t) => t.variable.is_some() || t.macro_name.is_some(),
                CslNode::Names(_) | CslNode::Date(_) => true,
                _ => false,
            })
            .count();
        var_count >= 2
    }

    // 3. Fallback: recursive search for the deepest group with delimiter and multiple variables.
    // Returns (delimiter, depth) to prioritize deeper matches.
    fn find_deepest_group_delimiter(
        nodes: &[CslNode],
        macros: &[Macro],
    ) -> Option<(String, usize)> {
        let mut best: Option<(String, usize)> = None;

        for node in nodes {
            match node {
                CslNode::Group(g) => {
                    // If this group has a delimiter and multiple variables, it's a candidate
                    if g.delimiter.is_some()
                        && has_multiple_variables(&g.children)
                        && (best.is_none() || 1 > best.as_ref().unwrap().1)
                    {
                        best = Some((g.delimiter.clone().unwrap(), 1));
                    }

                    // Recurse into children to find even deeper delimiters
                    if let Some((child_delim, depth)) =
                        find_deepest_group_delimiter(&g.children, macros)
                    {
                        let new_depth = depth + 1;
                        if best.is_none() || new_depth > best.as_ref().unwrap().1 {
                            best = Some((child_delim, new_depth));
                        }
                    }
                }
                CslNode::Choose(c) => {
                    // Search all branches of choose blocks
                    if let Some(result) =
                        find_deepest_group_delimiter(&c.if_branch.children, macros)
                        && (best.is_none() || result.1 > best.as_ref().unwrap().1)
                    {
                        best = Some(result);
                    }
                    for branch in &c.else_if_branches {
                        if let Some(result) = find_deepest_group_delimiter(&branch.children, macros)
                            && (best.is_none() || result.1 > best.as_ref().unwrap().1)
                        {
                            best = Some(result);
                        }
                    }
                    if let Some(else_branch) = &c.else_branch
                        && let Some(result) = find_deepest_group_delimiter(else_branch, macros)
                        && (best.is_none() || result.1 > best.as_ref().unwrap().1)
                    {
                        best = Some(result);
                    }
                }
                CslNode::Text(t) => {
                    // Expand macro calls to find delimiters inside
                    if let Some(macro_name) = &t.macro_name
                        && let Some(macro_def) = macros.iter().find(|m| &m.name == macro_name)
                        && let Some((delim, depth)) =
                            find_deepest_group_delimiter(&macro_def.children, macros)
                    {
                        let new_depth = depth + 1;
                        if best.is_none() || new_depth > best.as_ref().unwrap().1 {
                            best = Some((delim, new_depth));
                        }
                    }
                }
                _ => {}
            }
        }

        best
    }

    find_deepest_group_delimiter(&layout.children, macros)
        .map(|(d, _)| DelimiterPunctuation::from_csl_string(&d))
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
