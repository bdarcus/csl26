use csl_legacy::model::Style;
use csln_core::options::{
    Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
};

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
    // Check if citation uses year-suffix or disambiguation
    let is_author_date = style.citation.layout.children.iter().any(|node| {
        use csl_legacy::model::CslNode;
        match node {
            CslNode::Text(t) => t
                .macro_name
                .as_ref()
                .is_some_and(|m| m.contains("year") || m.contains("date")),
            CslNode::Group(g) => g.children.iter().any(|c| matches!(c, CslNode::Date(_))),
            _ => false,
        }
    });

    if is_author_date {
        // Extract disambiguation settings from citation-level attributes.
        let mut disamb = Disambiguation {
            // Author-date styles commonly rely on year suffixes; allow explicit
            // CSL settings to override this default.
            year_suffix: style.citation.disambiguate_add_year_suffix.unwrap_or(true),
            ..Default::default()
        };

        if let Some(opt) = style.citation.disambiguate_add_names {
            disamb.names = opt;
        }
        if let Some(opt) = style.citation.disambiguate_add_givenname {
            disamb.add_givenname = opt;
        }

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
