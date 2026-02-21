use csl_legacy::model::{CslNode, Style};
use csln_core::options::PageRangeFormat;
use csln_core::template::DelimiterPunctuation;
use std::collections::HashSet;

pub fn extract_volume_pages_delimiter(
    style: &Style,
    bib_macros: &HashSet<String>,
) -> Option<DelimiterPunctuation> {
    for macro_def in &style.macros {
        if bib_macros.contains(&macro_def.name)
            && let Some(delimiter) = find_volume_pages_delimiter_in_nodes(&macro_def.children)
        {
            return Some(DelimiterPunctuation::from_csl_string(&delimiter));
        }
    }
    None
}

fn find_volume_pages_delimiter_in_nodes(nodes: &[CslNode]) -> Option<String> {
    for node in nodes {
        match node {
            CslNode::Group(g) => {
                if let Some(delimiter) = find_volume_pages_delimiter_in_nodes(&g.children) {
                    return Some(delimiter);
                }

                let has_volume = group_directly_contains_variable(&g.children, "volume");
                let has_page = group_directly_contains_variable(&g.children, "page")
                    || group_contains_macro_with_page(&g.children);

                if has_volume
                    && has_page
                    && let Some(delim) = &g.delimiter
                {
                    return Some(delim.clone());
                }
            }
            CslNode::Choose(c) => {
                if let Some(d) = find_volume_pages_delimiter_in_nodes(&c.if_branch.children) {
                    return Some(d);
                }
                for branch in &c.else_if_branches {
                    if let Some(d) = find_volume_pages_delimiter_in_nodes(&branch.children) {
                        return Some(d);
                    }
                }
                if let Some(else_children) = &c.else_branch
                    && let Some(d) = find_volume_pages_delimiter_in_nodes(else_children)
                {
                    return Some(d);
                }
            }
            _ => {}
        }
    }
    None
}

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
                if group_directly_contains_variable(&c.if_branch.children, var_name) {
                    return true;
                }
                for branch in &c.else_if_branches {
                    if group_directly_contains_variable(&branch.children, var_name) {
                        return true;
                    }
                }
                if let Some(else_children) = &c.else_branch
                    && group_directly_contains_variable(else_children, var_name)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn group_contains_macro_with_page(nodes: &[CslNode]) -> bool {
    for node in nodes {
        if let CslNode::Text(t) = node
            && let Some(macro_name) = &t.macro_name
            && (macro_name.contains("locator")
                || macro_name.contains("page")
                || macro_name.contains("pages"))
        {
            return true;
        }
    }
    false
}

pub fn extract_page_range_format(style: &Style) -> Option<PageRangeFormat> {
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
