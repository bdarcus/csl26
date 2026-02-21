use csl_legacy::model::{CslNode, Layout, Style};
use csln_core::options::AndOptions;
use csln_core::template::{ContributorRole, TemplateComponent};

/// Extract the suffix on the author macro call from the bibliography layout.
pub fn extract_author_suffix(layout: &Layout) -> Option<String> {
    for node in &layout.children {
        // Check for group containing author macro call
        if let CslNode::Group(g) = node {
            for child in &g.children {
                if let CslNode::Text(t) = child
                    && t.macro_name.as_deref() == Some("author")
                {
                    // Found the author macro call - return its suffix
                    return t.suffix.clone();
                }
            }
        }
        // Check for direct author macro call at top level
        if let CslNode::Text(t) = node
            && t.macro_name.as_deref() == Some("author")
        {
            return t.suffix.clone();
        }
    }
    None
}

/// Apply the extracted author suffix to the author component in the template.
pub fn apply_author_suffix(components: &mut [TemplateComponent], suffix: Option<String>) {
    if let Some(suffix) = suffix {
        for component in components {
            if let TemplateComponent::Contributor(c) = component
                && c.contributor == ContributorRole::Author
            {
                // Set or update the suffix
                c.rendering.suffix = Some(suffix.clone());
            }
        }
    }
}

/// Check if the bibliography name element has an 'and' attribute.
pub fn extract_bibliography_and(style: &Style) -> Option<AndOptions> {
    for macro_def in &style.macros {
        if macro_def.name == "author"
            && let Some(result) = find_name_and(&macro_def.children)
        {
            return Some(result);
        }
    }

    if let Some(bib) = &style.bibliography
        && let Some(result) = find_name_and(&bib.layout.children)
    {
        return Some(result);
    }

    None
}

fn find_name_and(nodes: &[CslNode]) -> Option<AndOptions> {
    for node in nodes {
        match node {
            CslNode::Name(name) => {
                if let Some(and) = &name.and {
                    return Some(match and.as_str() {
                        "text" => AndOptions::Text,
                        "symbol" => AndOptions::Symbol,
                        _ => AndOptions::None,
                    });
                }
                return Some(AndOptions::None);
            }
            CslNode::Names(names) => {
                if let Some(result) = find_name_and(&names.children) {
                    return Some(result);
                }
            }
            CslNode::Group(g) => {
                if let Some(result) = find_name_and(&g.children) {
                    return Some(result);
                }
            }
            CslNode::Choose(c) => {
                if let Some(result) = find_name_and(&c.if_branch.children) {
                    return Some(result);
                }
                for branch in &c.else_if_branches {
                    if let Some(result) = find_name_and(&branch.children) {
                        return Some(result);
                    }
                }
                if let Some(else_branch) = &c.else_branch
                    && let Some(result) = find_name_and(else_branch)
                {
                    return Some(result);
                }
            }
            CslNode::Substitute(s) => {
                if let Some(result) = find_name_and(&s.children) {
                    return Some(result);
                }
            }
            _ => {}
        }
    }
    None
}

/// Apply the bibliography 'and' setting to author components in the template.
pub fn apply_bibliography_and(components: &mut [TemplateComponent], bib_and: Option<AndOptions>) {
    if let Some(bib_and) = bib_and {
        for component in components {
            if let TemplateComponent::Contributor(c) = component
                && c.contributor == ContributorRole::Author
            {
                c.and = Some(bib_and.clone());
            }
        }
    }
}
