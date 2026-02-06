use csl_legacy::model::Style;
use csln_core::options::{Disambiguation, Processing, ProcessingCustom};

pub fn detect_processing_mode(style: &Style) -> Option<Processing> {
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
        // Extract disambiguation settings from citation
        let mut disamb = Disambiguation {
            year_suffix: true, // Standard for author-date
            ..Default::default()
        };

        if let Some(opt) = style.citation.disambiguate_add_names {
            disamb.names = opt;
        }
        if let Some(opt) = style.citation.disambiguate_add_givenname {
            disamb.add_givenname = opt;
        }

        return Some(Processing::Custom(ProcessingCustom {
            disambiguate: Some(disamb),
            ..Default::default()
        }));
    }

    None
}
