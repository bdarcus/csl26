/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use citum_schema::template::{
    Rendering, SimpleVariable, TemplateComponent, TemplateVariable, WrapPunctuation,
};
use csl_legacy::model::{CslNode, Layout, Macro};
use std::collections::HashSet;

#[allow(clippy::indexing_slicing, reason = "idx is found via position()")]
pub(super) fn ensure_numeric_locator_citation_component(
    layout: &Layout,
    template: &mut Vec<TemplateComponent>,
) {
    if !layout_uses_citation_locator(layout) || citation_template_has_locator(template) {
        return;
    }

    let locator_component = TemplateComponent::Variable(TemplateVariable {
        variable: SimpleVariable::Locator,
        rendering: Rendering {
            prefix: Some(", ".into()),
            ..Default::default()
        },
        ..Default::default()
    });

    // The reference marker is not a component, so the locator is simply the
    // citation body; the marker slot renders ahead of it.
    // See docs/specs/REFERENCE_MARKERS.md.
    template.push(locator_component);
}

pub(super) fn normalize_wrapped_numeric_locator_citation_component(
    layout: &Layout,
    template: &mut [TemplateComponent],
    citation_delimiter: &mut Option<String>,
) {
    let Some((locator_wrap, no_inner_delimiter)) =
        find_wrapped_locator_group_format(&layout.children)
    else {
        return;
    };

    if !nodes_contain_citation_number(&layout.children) || !citation_template_has_locator(template)
    {
        return;
    }

    if apply_wrapped_locator_formatting(template, &locator_wrap) && no_inner_delimiter {
        *citation_delimiter = Some(String::new());
    }
}

fn find_wrapped_locator_group_format(nodes: &[CslNode]) -> Option<(WrapPunctuation, bool)> {
    for node in nodes {
        match node {
            CslNode::Group(group) => {
                let wrap = match (group.prefix.as_deref(), group.suffix.as_deref()) {
                    (Some("("), Some(")")) => Some(WrapPunctuation::Parentheses),
                    (Some("["), Some("]")) => Some(WrapPunctuation::Brackets),
                    _ => None,
                };
                if let Some(wrap) = wrap
                    && nodes_use_citation_locator(&group.children)
                {
                    return Some((wrap, group.delimiter.is_none()));
                }

                if let Some(found) = find_wrapped_locator_group_format(&group.children) {
                    return Some(found);
                }
            }
            CslNode::Choose(choose) => {
                if let Some(found) = find_wrapped_locator_group_format(&choose.if_branch.children) {
                    return Some(found);
                }
                for branch in &choose.else_if_branches {
                    if let Some(found) = find_wrapped_locator_group_format(&branch.children) {
                        return Some(found);
                    }
                }
                if let Some(else_branch) = choose.else_branch.as_ref()
                    && let Some(found) = find_wrapped_locator_group_format(else_branch)
                {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

fn apply_wrapped_locator_formatting(
    template: &mut [TemplateComponent],
    wrap: &WrapPunctuation,
) -> bool {
    use citum_schema::template::WrapConfig;

    let mut changed = false;
    for component in template {
        #[allow(
            clippy::collapsible_match,
            reason = "cannot use match guard due to mutable borrow of captured variable"
        )]
        match component {
            TemplateComponent::Variable(variable)
                if variable.variable == SimpleVariable::Locator =>
            {
                let wrap_config = WrapConfig {
                    punctuation: wrap.clone(),
                    inner_prefix: None,
                    inner_suffix: None,
                };
                if variable.rendering.wrap.as_ref() != Some(&wrap_config) {
                    variable.rendering.wrap = Some(wrap_config);
                    changed = true;
                }
                if variable.rendering.prefix.is_some() {
                    variable.rendering.prefix = None;
                    changed = true;
                }
                if variable.rendering.suffix.is_some() {
                    variable.rendering.suffix = None;
                    changed = true;
                }
            }
            TemplateComponent::Group(list) => {
                if apply_wrapped_locator_formatting(&mut list.group, wrap) {
                    changed = true;
                }
            }
            _ => {}
        }
    }
    changed
}

pub(super) fn normalize_author_date_locator_citation_component(
    layout: &Layout,
    macros: &[Macro],
    template: &mut Vec<TemplateComponent>,
) {
    if !layout_uses_citation_locator(layout) {
        return;
    }

    let locator_prefix = infer_locator_group_delimiter(layout)
        .or_else(|| {
            let mut visited = HashSet::new();
            infer_locator_prefix_from_nodes(&layout.children, macros, &mut visited)
        })
        .unwrap_or_else(|| " ".into());
    if apply_author_date_locator_formatting(template, &locator_prefix) {
        return;
    }

    template.push(TemplateComponent::Variable(TemplateVariable {
        variable: SimpleVariable::Locator,
        rendering: Rendering {
            prefix: Some(locator_prefix.into()),
            ..Default::default()
        },
        ..Default::default()
    }));
}

fn infer_locator_group_delimiter(layout: &Layout) -> Option<String> {
    if let Some(delimiter) = layout.delimiter.as_ref()
        && layout
            .children
            .iter()
            .position(node_uses_citation_locator)
            .is_some_and(|index| index > 0)
        && !delimiter.is_empty()
    {
        return Some(delimiter.clone());
    }

    infer_locator_group_delimiter_from_nodes(&layout.children)
}

fn infer_locator_group_delimiter_from_nodes(nodes: &[CslNode]) -> Option<String> {
    for node in nodes {
        match node {
            CslNode::Group(group) => {
                if let Some(delimiter) = group.delimiter.as_ref()
                    && group
                        .children
                        .iter()
                        .position(node_uses_citation_locator)
                        .is_some_and(|index| index > 0)
                    && !delimiter.is_empty()
                {
                    return Some(delimiter.clone());
                }

                if let Some(delimiter) = infer_locator_group_delimiter_from_nodes(&group.children) {
                    return Some(delimiter);
                }
            }
            CslNode::Choose(choose) => {
                if let Some(delimiter) =
                    infer_locator_group_delimiter_from_nodes(&choose.if_branch.children)
                {
                    return Some(delimiter);
                }
                for branch in &choose.else_if_branches {
                    if let Some(delimiter) =
                        infer_locator_group_delimiter_from_nodes(&branch.children)
                    {
                        return Some(delimiter);
                    }
                }
                if let Some(else_branch) = choose.else_branch.as_ref()
                    && let Some(delimiter) = infer_locator_group_delimiter_from_nodes(else_branch)
                {
                    return Some(delimiter);
                }
            }
            _ => {}
        }
    }
    None
}

fn apply_author_date_locator_formatting(
    template: &mut [TemplateComponent],
    locator_prefix: &str,
) -> bool {
    let mut found_locator = false;
    for component in template {
        #[allow(
            clippy::collapsible_match,
            reason = "cannot use match guard due to mutable borrow of captured variable"
        )]
        match component {
            TemplateComponent::Variable(variable)
                if variable.variable == SimpleVariable::Locator =>
            {
                found_locator = true;
                if should_replace_author_date_locator_prefix(
                    variable.rendering.prefix.as_deref(),
                    locator_prefix,
                ) {
                    variable.rendering.prefix = Some(locator_prefix.into());
                }
            }
            TemplateComponent::Group(list) => {
                if apply_author_date_locator_formatting(&mut list.group, locator_prefix) {
                    found_locator = true;
                }
            }
            _ => {}
        }
    }
    found_locator
}

fn should_replace_author_date_locator_prefix(
    existing_prefix: Option<&str>,
    preferred_prefix: &str,
) -> bool {
    match existing_prefix {
        None => true,
        Some("") => true,
        Some(prefix) if prefix == preferred_prefix => false,
        Some(prefix) => prefix.trim().is_empty() && preferred_prefix != prefix,
    }
}

fn infer_locator_prefix_from_nodes(
    nodes: &[CslNode],
    macros: &[Macro],
    visited_macros: &mut HashSet<String>,
) -> Option<String> {
    for node in nodes {
        match node {
            CslNode::Text(t) => {
                let is_locator = t.variable.as_deref() == Some("locator")
                    || t.macro_name
                        .as_deref()
                        .is_some_and(macro_name_indicates_locator);
                if !is_locator {
                    continue;
                }

                if let Some(prefix) = t.prefix.as_ref()
                    && !prefix.is_empty()
                {
                    return Some(prefix.clone());
                }

                if let Some(macro_name) = t.macro_name.as_ref()
                    && visited_macros.insert(macro_name.clone())
                    && let Some(macro_def) = macros.iter().find(|m| m.name == *macro_name)
                    && let Some(prefix) =
                        infer_locator_prefix_from_nodes(&macro_def.children, macros, visited_macros)
                {
                    return Some(prefix);
                }
            }
            CslNode::Group(g) => {
                if let Some(prefix) =
                    infer_locator_prefix_from_nodes(&g.children, macros, visited_macros)
                {
                    return Some(prefix);
                }
            }
            CslNode::Choose(c) => {
                if let Some(prefix) =
                    infer_locator_prefix_from_nodes(&c.if_branch.children, macros, visited_macros)
                {
                    return Some(prefix);
                }
                for branch in &c.else_if_branches {
                    if let Some(prefix) =
                        infer_locator_prefix_from_nodes(&branch.children, macros, visited_macros)
                    {
                        return Some(prefix);
                    }
                }
                if let Some(else_branch) = c.else_branch.as_ref()
                    && let Some(prefix) =
                        infer_locator_prefix_from_nodes(else_branch, macros, visited_macros)
                {
                    return Some(prefix);
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn move_group_wrap_to_citation_items(
    layout: &Layout,
    citation_wrap: &mut Option<WrapPunctuation>,
) -> Option<citum_schema::options::LabelWrap> {
    let wrap = citation_wrap.clone()?;
    if !layout_has_group_wrap_for_citation_number(layout, &wrap) {
        return None;
    }
    // The bracket encloses the marker together with the locator (`[1, p. 5]`),
    // which is an item wrap rather than a cluster wrap.
    // See docs/specs/REFERENCE_MARKERS.md.
    let item_wrap = match wrap {
        WrapPunctuation::Brackets => citum_schema::options::LabelWrap::Brackets,
        WrapPunctuation::Parentheses => citum_schema::options::LabelWrap::Parentheses,
        WrapPunctuation::Quotes => return None,
    };
    *citation_wrap = None;
    Some(item_wrap)
}

fn citation_template_has_locator(template: &[TemplateComponent]) -> bool {
    template.iter().any(component_has_locator)
}

fn component_has_locator(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Variable(v) => v.variable == SimpleVariable::Locator,
        TemplateComponent::Group(list) => list.group.iter().any(component_has_locator),
        _ => false,
    }
}

fn layout_uses_citation_locator(layout: &Layout) -> bool {
    nodes_use_citation_locator(&layout.children)
}

fn nodes_use_citation_locator(nodes: &[CslNode]) -> bool {
    nodes.iter().any(node_uses_citation_locator)
}

fn node_uses_citation_locator(node: &CslNode) -> bool {
    match node {
        CslNode::Text(t) => {
            t.variable.as_deref() == Some("locator")
                || t.macro_name
                    .as_deref()
                    .is_some_and(macro_name_indicates_locator)
        }
        CslNode::Group(g) => nodes_use_citation_locator(&g.children),
        CslNode::Choose(c) => {
            nodes_use_citation_locator(&c.if_branch.children)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_use_citation_locator(&b.children))
                || c.else_branch
                    .as_ref()
                    .is_some_and(|children| nodes_use_citation_locator(children))
        }
        _ => false,
    }
}

fn macro_name_indicates_locator(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered.contains("citation-locator") || lowered.contains("locator")
}

fn layout_has_group_wrap_for_citation_number(layout: &Layout, wrap: &WrapPunctuation) -> bool {
    let (prefix, suffix) = match wrap {
        WrapPunctuation::Brackets => ("[", "]"),
        WrapPunctuation::Parentheses => ("(", ")"),
        _ => return false,
    };
    nodes_have_wrapped_citation_number_group(&layout.children, prefix, suffix)
}

fn nodes_have_wrapped_citation_number_group(nodes: &[CslNode], prefix: &str, suffix: &str) -> bool {
    nodes
        .iter()
        .any(|node| node_has_wrapped_citation_number_group(node, prefix, suffix))
}

fn node_has_wrapped_citation_number_group(node: &CslNode, prefix: &str, suffix: &str) -> bool {
    match node {
        CslNode::Group(g) => {
            if g.prefix.as_deref() == Some(prefix)
                && g.suffix.as_deref() == Some(suffix)
                && nodes_contain_citation_number(&g.children)
            {
                return true;
            }
            nodes_have_wrapped_citation_number_group(&g.children, prefix, suffix)
        }
        CslNode::Choose(c) => {
            nodes_have_wrapped_citation_number_group(&c.if_branch.children, prefix, suffix)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_have_wrapped_citation_number_group(&b.children, prefix, suffix))
                || c.else_branch.as_ref().is_some_and(|children| {
                    nodes_have_wrapped_citation_number_group(children, prefix, suffix)
                })
        }
        _ => false,
    }
}

fn nodes_contain_citation_number(nodes: &[CslNode]) -> bool {
    nodes.iter().any(node_contains_citation_number)
}

fn node_contains_citation_number(node: &CslNode) -> bool {
    match node {
        CslNode::Text(t) => t.variable.as_deref() == Some("citation-number"),
        CslNode::Number(n) => n.variable == "citation-number",
        CslNode::Group(g) => nodes_contain_citation_number(&g.children),
        CslNode::Choose(c) => {
            nodes_contain_citation_number(&c.if_branch.children)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_contain_citation_number(&b.children))
                || c.else_branch
                    .as_ref()
                    .is_some_and(|children| nodes_contain_citation_number(children))
        }
        _ => false,
    }
}
