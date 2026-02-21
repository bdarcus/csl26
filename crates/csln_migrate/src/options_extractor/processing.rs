use csl_legacy::model::{CslNode, Style};
use csln_core::options::{
    Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
};
use std::collections::HashSet;

pub fn detect_processing_mode(style: &Style) -> Option<Processing> {
    // 0. Note styles are explicit in CSL and should map directly.
    if style.class == "note" {
        return Some(Processing::Note);
    }

    // 1. Explicitly numeric style
    // Check if bibliography uses second-field-align (heuristic for numeric labels)
    // Actually, check if it's APA (not numeric) or check common markers
    // Since 'second_field_align' is missing in my model read, I'll use a safer heuristic.

    // Helper to recursively search for citation-number in layout nodes
    fn has_citation_number(nodes: &[csl_legacy::model::CslNode]) -> bool {
        use csl_legacy::model::CslNode;
        nodes.iter().any(|node| match node {
            CslNode::Number(n) => n.variable == "citation-number",
            CslNode::Group(g) => has_citation_number(&g.children),
            CslNode::Text(t) if t.variable.as_deref() == Some("citation-number") => true,
            _ => false,
        })
    }

    let is_numeric =
        style.class == "in-text" && has_citation_number(&style.citation.layout.children);

    if is_numeric {
        return Some(Processing::Numeric);
    }

    // 2. Author-date style
    // Some styles hide date/year logic in nested macro trees. Follow macro calls
    // recursively so we don't miss author-date processing config extraction.
    let mut visited_macros = HashSet::new();
    let is_author_date =
        nodes_have_author_date_signal(&style.citation.layout.children, style, &mut visited_macros);

    if is_author_date {
        // Extract disambiguation settings from citation-level attributes.
        // Legacy CSL defaults are effectively "no extra names / no extra given
        // names" unless explicitly requested. Defaulting to names=true here
        // causes over-disambiguation and suppresses expected et-al behavior.
        let disamb = Disambiguation {
            names: style.citation.disambiguate_add_names.unwrap_or(false),
            add_givenname: style.citation.disambiguate_add_givenname.unwrap_or(false),
            // Author-date styles commonly rely on year suffixes; keep this true
            // unless legacy style explicitly disables it.
            year_suffix: style.citation.disambiguate_add_year_suffix.unwrap_or(true),
        };

        let sort = style.citation.sort.as_ref().and_then(extract_sort);
        let group = sort.as_ref().and_then(extract_group_from_sort);

        return Some(Processing::Custom(ProcessingCustom {
            sort,
            group,
            disambiguate: Some(disamb),
        }));
    }

    None
}

fn nodes_have_author_date_signal(
    nodes: &[CslNode],
    style: &Style,
    visited_macros: &mut HashSet<String>,
) -> bool {
    nodes
        .iter()
        .any(|node| node_has_author_date_signal(node, style, visited_macros))
}

fn node_has_author_date_signal(
    node: &CslNode,
    style: &Style,
    visited_macros: &mut HashSet<String>,
) -> bool {
    match node {
        CslNode::Date(_) => true,
        CslNode::Text(t) => {
            if t.variable.as_deref().is_some_and(|v| {
                matches!(
                    v,
                    "issued" | "original-date" | "event-date" | "accessed" | "year-suffix"
                )
            }) {
                return true;
            }

            if let Some(macro_name) = &t.macro_name {
                let lowered = macro_name.to_ascii_lowercase();
                if lowered.contains("year") || lowered.contains("date") {
                    return true;
                }

                if visited_macros.insert(macro_name.clone())
                    && let Some(macro_def) = style.macros.iter().find(|m| m.name == *macro_name)
                    && nodes_have_author_date_signal(&macro_def.children, style, visited_macros)
                {
                    return true;
                }
            }

            false
        }
        CslNode::Group(g) => nodes_have_author_date_signal(&g.children, style, visited_macros),
        CslNode::Choose(c) => {
            nodes_have_author_date_signal(&c.if_branch.children, style, visited_macros)
                || c.else_if_branches
                    .iter()
                    .any(|b| nodes_have_author_date_signal(&b.children, style, visited_macros))
                || c.else_branch.as_ref().is_some_and(|nodes| {
                    nodes_have_author_date_signal(nodes, style, visited_macros)
                })
        }
        CslNode::Names(n) => nodes_have_author_date_signal(&n.children, style, visited_macros),
        _ => false,
    }
}

fn extract_sort(legacy_sort: &csl_legacy::model::Sort) -> Option<Sort> {
    let template: Vec<SortSpec> = legacy_sort
        .keys
        .iter()
        .filter_map(|key| {
            let key_kind = key
                .variable
                .as_ref()
                .and_then(|name| parse_sort_key(name))
                .or_else(|| {
                    key.macro_name
                        .as_ref()
                        .and_then(|name| parse_sort_key(name))
                })?;

            let ascending = key.sort.as_deref() != Some("descending");
            Some(SortSpec {
                key: key_kind,
                ascending,
            })
        })
        .collect();

    if template.is_empty() {
        None
    } else {
        Some(Sort {
            shorten_names: false,
            render_substitutions: false,
            template,
        })
    }
}

fn extract_group_from_sort(sort: &Sort) -> Option<Group> {
    let mut keys: Vec<SortKey> = Vec::new();

    for spec in &sort.template {
        match spec.key {
            SortKey::Author | SortKey::Year | SortKey::Title => {
                if !keys.contains(&spec.key) {
                    keys.push(spec.key.clone());
                }
            }
            SortKey::CitationNumber => {}
            _ => {}
        }
    }

    if keys.is_empty() {
        None
    } else {
        Some(Group { template: keys })
    }
}

fn parse_sort_key(name: &str) -> Option<SortKey> {
    let lowered = name.to_ascii_lowercase();

    if lowered == "citation-number" || lowered.contains("citation-number") {
        Some(SortKey::CitationNumber)
    } else if lowered == "author" || lowered.contains("author") {
        Some(SortKey::Author)
    } else if lowered == "issued"
        || lowered == "year"
        || lowered.contains("year")
        || lowered.contains("date")
    {
        Some(SortKey::Year)
    } else if lowered == "title" || lowered.contains("title") {
        Some(SortKey::Title)
    } else {
        None
    }
}
