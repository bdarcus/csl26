use csl_legacy::model::{CslNode, Style};
use csln_core::options::{TitleRendering, TitlesConfig};

pub fn extract_title_config(style: &Style) -> Option<TitlesConfig> {
    let mut config = TitlesConfig::default();
    let mut has_config = false;

    // Scan bibliography for periodical formatting (italics)
    if let Some(bib) = &style.bibliography {
        if let Some(rendering) =
            scan_for_title_formatting(&bib.layout.children, style, "container-title")
        {
            config.periodical = Some(rendering.clone());
            config.serial = Some(rendering);
            has_config = true;
        }
        if let Some(rendering) = scan_for_title_formatting(&bib.layout.children, style, "title") {
            config.monograph = Some(rendering);
            has_config = true;
        }
    }

    if has_config {
        Some(config)
    } else {
        None
    }
}

fn scan_for_title_formatting(
    nodes: &[CslNode],
    style: &Style,
    var_name: &str,
) -> Option<TitleRendering> {
    for node in nodes {
        match node {
            CslNode::Text(t) => {
                if t.variable.as_ref().is_some_and(|v| v == var_name)
                    && t.formatting
                        .font_style
                        .as_ref()
                        .is_some_and(|s| s == "italic")
                {
                    return Some(TitleRendering {
                        emph: Some(true),
                        ..Default::default()
                    });
                }
                if let Some(macro_name) = &t.macro_name {
                    if let Some(m) = style.macros.iter().find(|m| &m.name == macro_name) {
                        if let Some(rendering) =
                            scan_for_title_formatting(&m.children, style, var_name)
                        {
                            return Some(rendering);
                        }
                    }
                }
            }
            CslNode::Group(g) => {
                if let Some(rendering) = scan_for_title_formatting(&g.children, style, var_name) {
                    return Some(rendering);
                }
            }
            CslNode::Choose(c) => {
                if let Some(rendering) =
                    scan_for_title_formatting(&c.if_branch.children, style, var_name)
                {
                    return Some(rendering);
                }
                for branch in &c.else_if_branches {
                    if let Some(rendering) =
                        scan_for_title_formatting(&branch.children, style, var_name)
                    {
                        return Some(rendering);
                    }
                }
                if let Some(else_branch) = &c.else_branch {
                    if let Some(rendering) = scan_for_title_formatting(else_branch, style, var_name)
                    {
                        return Some(rendering);
                    }
                }
            }
            _ => {}
        }
    }
    None
}
