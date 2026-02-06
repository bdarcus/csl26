use csl_legacy::model::{CslNode, Layout};
use csln_core::template::WrapPunctuation;

/// Infer citation wrapping from CSL layout.
pub fn infer_citation_wrapping(
    layout: &Layout,
) -> (Option<WrapPunctuation>, Option<String>, Option<String>) {
    // First check layout-level prefix/suffix
    let layout_wrap = match (layout.prefix.as_deref(), layout.suffix.as_deref()) {
        (Some("("), Some(")")) => Some((Some(WrapPunctuation::Parentheses), None, None)),
        (Some("["), Some("]")) => Some((Some(WrapPunctuation::Brackets), None, None)),
        _ => None,
    };

    if let Some(wrap) = layout_wrap {
        return wrap;
    }

    // Check for group-level wrapping (common in numeric styles like IEEE)
    if let Some(wrap) = find_group_wrapping(&layout.children) {
        return wrap;
    }

    // Fall back to layout prefix/suffix if set (edge cases)
    match (layout.prefix.as_deref(), layout.suffix.as_deref()) {
        (None, None) | (Some(""), Some("")) | (Some(""), None) | (None, Some("")) => {
            (None, None, None)
        }
        _ => (None, layout.prefix.clone(), layout.suffix.clone()),
    }
}

fn find_group_wrapping(
    nodes: &[CslNode],
) -> Option<(Option<WrapPunctuation>, Option<String>, Option<String>)> {
    for node in nodes {
        if let CslNode::Group(g) = node {
            match (g.prefix.as_deref(), g.suffix.as_deref()) {
                (Some("("), Some(")")) => {
                    return Some((Some(WrapPunctuation::Parentheses), None, None))
                }
                (Some("["), Some("]")) => {
                    return Some((Some(WrapPunctuation::Brackets), None, None))
                }
                _ => {
                    // Recurse into nested groups
                    if let Some(wrap) = find_group_wrapping(&g.children) {
                        return Some(wrap);
                    }
                }
            }
        }
    }
    None
}

/// Extract the intra-citation delimiter from the layout.
///
/// Finds the delimiter between author and date in a citation layout.
/// This should be the delimiter of the INNERMOST group that directly contains
/// both author and date (not counting intermediate groups that might also contain
/// other elements like locators).
pub fn extract_citation_delimiter(layout: &Layout) -> Option<String> {
    fn is_author_macro(node: &CslNode) -> bool {
        match node {
            CslNode::Text(t) => t
                .macro_name
                .as_deref()
                .is_some_and(|m| m.contains("author")),
            CslNode::Names(_) => true,
            CslNode::Group(g) => g.children.iter().any(is_author_macro),
            CslNode::Choose(c) => {
                c.if_branch.children.iter().any(is_author_macro)
                    || c.else_if_branches
                        .iter()
                        .any(|b| b.children.iter().any(is_author_macro))
                    || c.else_branch
                        .as_ref()
                        .is_some_and(|e| e.iter().any(is_author_macro))
            }
            _ => false,
        }
    }

    fn is_date_macro(node: &CslNode) -> bool {
        match node {
            CslNode::Text(t) => t
                .macro_name
                .as_deref()
                .is_some_and(|m| m.contains("year") || m.contains("date")),
            CslNode::Date(_) => true,
            CslNode::Group(g) => g.children.iter().any(is_date_macro),
            CslNode::Choose(c) => {
                c.if_branch.children.iter().any(is_date_macro)
                    || c.else_if_branches
                        .iter()
                        .any(|b| b.children.iter().any(is_date_macro))
                    || c.else_branch
                        .as_ref()
                        .is_some_and(|e| e.iter().any(is_date_macro))
            }
            _ => false,
        }
    }

    // Look for groups that directly contain both author and date at the SAME level.
    // This handles cases like:
    //   <group delimiter=" ">
    //     <text macro="author-short"/>
    //     <text macro="year"/>
    //   </group>
    // The key is "at the SAME level" - if they're in different nested groups,
    // we want the innermost group that has them both.
    fn find_innermost_delimiter(nodes: &[CslNode]) -> Option<String> {
        // First, check if any child directly contains both author and date
        for node in nodes {
            if let CslNode::Group(g) = node {
                let has_author = g.children.iter().any(is_author_macro);
                let has_date = g.children.iter().any(is_date_macro);

                if has_author && has_date {
                    // Count how many direct children are "meaningful" (not just groups)
                    let direct_author_or_date = g
                        .children
                        .iter()
                        .filter(|child| {
                            matches!(
                                child,
                                CslNode::Text(_) | CslNode::Names(_) | CslNode::Date(_)
                            ) && (is_author_macro(child) || is_date_macro(child))
                        })
                        .count();

                    // If this group has author and date as direct children (not deeply nested),
                    // use its delimiter
                    if direct_author_or_date >= 2 {
                        if let Some(delimiter) = &g.delimiter {
                            return Some(delimiter.clone());
                        }
                    } else {
                        // Recurse into the group to find the innermost one
                        if let Some(delim) = find_innermost_delimiter(&g.children) {
                            return Some(delim);
                        }
                    }
                }
            }
        }
        None
    }

    if let Some(delim) = find_innermost_delimiter(&layout.children) {
        return Some(delim);
    }

    // Fallback: check if date macro call has a prefix that acts as a delimiter
    for node in &layout.children {
        if let CslNode::Group(g) = node {
            for child in &g.children {
                if is_date_macro(child) {
                    if let CslNode::Text(t) = child {
                        if let Some(prefix) = &t.prefix {
                            return Some(prefix.clone());
                        }
                    }
                }
            }
        }
    }

    None
}
