use csl_legacy::model::{CslNode, Style};
use csln_core::options::{DateConfig, MonthFormat};

pub fn extract_date_config(style: &Style) -> Option<DateConfig> {
    let mut config = DateConfig::default();
    let mut found_date = false;

    // Scan bibliography for month format
    if let Some(bib) = &style.bibliography {
        if let Some(format) = scan_for_month_format(&bib.layout.children, style) {
            config.month = format;
            found_date = true;
        } else if scan_for_any_date(&bib.layout.children, style) {
            found_date = true;
        }
    }

    // Fallback to citation if bibliography didn't have it
    if !found_date {
        if let Some(format) = scan_for_month_format(&style.citation.layout.children, style) {
            config.month = format;
            found_date = true;
        } else if scan_for_any_date(&style.citation.layout.children, style) {
            found_date = true;
        }
    }

    if found_date { Some(config) } else { None }
}

fn scan_for_any_date(nodes: &[CslNode], style: &Style) -> bool {
    for node in nodes {
        match node {
            CslNode::Date(_) => return true,
            CslNode::Text(t) => {
                if let Some(macro_name) = &t.macro_name
                    && let Some(m) = style.macros.iter().find(|m| &m.name == macro_name)
                    && scan_for_any_date(&m.children, style)
                {
                    return true;
                }
            }
            CslNode::Group(g) => {
                if scan_for_any_date(&g.children, style) {
                    return true;
                }
            }
            CslNode::Choose(c) => {
                if scan_for_any_date(&c.if_branch.children, style) {
                    return true;
                }
                for branch in &c.else_if_branches {
                    if scan_for_any_date(&branch.children, style) {
                        return true;
                    }
                }
                if let Some(else_branch) = &c.else_branch
                    && scan_for_any_date(else_branch, style)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn scan_for_month_format(nodes: &[CslNode], style: &Style) -> Option<MonthFormat> {
    for node in nodes {
        match node {
            CslNode::Date(d) => {
                if let Some(form) = &d.form {
                    return Some(match form.as_str() {
                        "short" => MonthFormat::Short,
                        "numeric" | "numeric-leading-zeros" => MonthFormat::Numeric,
                        _ => MonthFormat::Long,
                    });
                }
                // Check parts for month form
                for part in &d.parts {
                    if part.name == "month"
                        && let Some(form) = &part.form
                    {
                        return Some(match form.as_str() {
                            "short" => MonthFormat::Short,
                            "numeric" | "numeric-leading-zeros" => MonthFormat::Numeric,
                            _ => MonthFormat::Long,
                        });
                    }
                }
            }
            CslNode::Text(t) => {
                if let Some(macro_name) = &t.macro_name
                    && let Some(m) = style.macros.iter().find(|m| &m.name == macro_name)
                    && let Some(format) = scan_for_month_format(&m.children, style)
                {
                    return Some(format);
                }
            }
            CslNode::Group(g) => {
                if let Some(format) = scan_for_month_format(&g.children, style) {
                    return Some(format);
                }
            }
            CslNode::Choose(c) => {
                if let Some(format) = scan_for_month_format(&c.if_branch.children, style) {
                    return Some(format);
                }
                for branch in &c.else_if_branches {
                    if let Some(format) = scan_for_month_format(&branch.children, style) {
                        return Some(format);
                    }
                }
                if let Some(else_branch) = &c.else_branch
                    && let Some(format) = scan_for_month_format(else_branch, style)
                {
                    return Some(format);
                }
            }
            _ => {}
        }
    }
    None
}
