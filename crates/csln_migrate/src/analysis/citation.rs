use csl_legacy::model::{CslNode, Layout, Macro};
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
/// Uses depth-first search to find the DEEPEST group that contains both
/// author and date, handling nested groups, Choose blocks, and macro expansion.
pub fn extract_citation_delimiter(layout: &Layout, macros: &[Macro]) -> Option<String> {
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

    // Find the deepest group that contains both author and date.
    // Returns (delimiter, depth) tuple for comparison.
    //
    // This handles:
    // 1. Flat structures: <group><author/><date/></group> (APA)
    // 2. Nested groups: <group><group><author/><date/></group></group> (Springer)
    // 3. Choose blocks: <group><choose><author+date macros/></choose></group> (Chicago)
    // 4. Macro expansion: When author+date are in a macro, expand it to find delimiter
    fn find_deepest_delimiter(nodes: &[CslNode], macros: &[Macro]) -> Option<(String, usize)> {
        let mut best: Option<(String, usize)> = None;

        for node in nodes {
            match node {
                CslNode::Group(g) => {
                    let has_author = g.children.iter().any(is_author_macro);
                    let has_date = g.children.iter().any(is_date_macro);

                    if has_author && has_date {
                        // This group contains both author and date (possibly in subtree).
                        // Check if there's a deeper group inside.
                        if let Some((delim, depth)) = find_deepest_delimiter(&g.children, macros) {
                            // Found a deeper group
                            let new_depth = depth + 1;
                            if best.is_none() || new_depth > best.as_ref().unwrap().1 {
                                best = Some((delim, new_depth));
                            }
                        } else if let Some(delimiter) = &g.delimiter {
                            // No deeper group found, use this group's delimiter
                            if best.is_none() || 1 > best.as_ref().unwrap().1 {
                                best = Some((delimiter.clone(), 1));
                            }
                        }
                    }
                }
                CslNode::Choose(c) => {
                    // Recurse into Choose branches to find groups inside
                    if let Some(result) = find_deepest_delimiter(&c.if_branch.children, macros) {
                        if best.is_none() || result.1 > best.as_ref().unwrap().1 {
                            best = Some(result);
                        }
                    }
                    for branch in &c.else_if_branches {
                        if let Some(result) = find_deepest_delimiter(&branch.children, macros) {
                            if best.is_none() || result.1 > best.as_ref().unwrap().1 {
                                best = Some(result);
                            }
                        }
                    }
                    if let Some(else_branch) = &c.else_branch {
                        if let Some(result) = find_deepest_delimiter(else_branch, macros) {
                            if best.is_none() || result.1 > best.as_ref().unwrap().1 {
                                best = Some(result);
                            }
                        }
                    }
                }
                CslNode::Text(t) => {
                    // Check if this is a macro call that contains both author and date
                    if let Some(macro_name) = &t.macro_name {
                        if macro_name.contains("author") && macro_name.contains("date") {
                            // This macro likely contains both author and date
                            // Expand it and search for delimiter inside
                            if let Some(macro_def) = macros.iter().find(|m| &m.name == macro_name) {
                                if let Some(result) =
                                    find_deepest_delimiter(&macro_def.children, macros)
                                {
                                    // Found a delimiter in the macro
                                    // Add depth for the macro boundary
                                    let new_depth = result.1 + 1;
                                    if best.is_none() || new_depth > best.as_ref().unwrap().1 {
                                        best = Some((result.0, new_depth));
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        best
    }

    if let Some((delim, _)) = find_deepest_delimiter(&layout.children, macros) {
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
