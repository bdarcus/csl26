/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Rendering utilities for CSLN templates.

use csln_core::template::{TemplateComponent, WrapPunctuation};
use std::fmt::Write;

/// A processed template component with its rendered value.
#[derive(Debug, Clone)]
pub struct ProcTemplateComponent {
    /// The original template component (for rendering instructions).
    pub template_component: TemplateComponent,
    /// The processed values.
    pub value: String,
    /// Optional prefix from value extraction.
    pub prefix: Option<String>,
    /// Optional suffix from value extraction.
    pub suffix: Option<String>,
}

/// A processed template (list of rendered components).
pub type ProcTemplate = Vec<ProcTemplateComponent>;

/// Render processed templates into a final bibliography string.
pub fn refs_to_string(proc_templates: Vec<ProcTemplate>) -> String {
    let mut output = String::new();
    for (i, proc_template) in proc_templates.iter().enumerate() {
        if i > 0 {
            output.push_str("\n\n");
        }
        for (j, component) in proc_template.iter().enumerate() {
            let rendered = render_component(component);
            // Skip empty components
            if rendered.is_empty() {
                continue;
            }
            // Add separator if needed (not after punctuation)
            if j > 0 && !output.is_empty() {
                let last_char = output.chars().last().unwrap_or(' ');
                if !matches!(last_char, '.' | ',' | ':' | ';' | ' ') {
                    output.push_str(". ");
                } else if last_char == '.' {
                    output.push(' ');
                } else if !last_char.is_whitespace() {
                    // After comma/colon/semicolon, just add space
                    output.push(' ');
                }
            }
            let _ = write!(&mut output, "{}", rendered);
        }
        // Don't add period if last component is DOI/URL (they end with the link)
        let last_is_link = proc_template.last().map_or(false, |c| {
            matches!(&c.template_component, 
                TemplateComponent::Variable(v) if matches!(v.variable, 
                    csln_core::template::SimpleVariable::Doi | csln_core::template::SimpleVariable::Url
                )
            )
        });
        if !output.ends_with('.') && !last_is_link {
            output.push('.');
        }
    }
    output
}

/// Render a single citation.
pub fn citation_to_string(proc_template: &ProcTemplate, wrap_parens: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    
    for component in proc_template {
        parts.push(render_component(component));
    }
    
    let content = parts.join(", ");
    
    if wrap_parens {
        format!("({})", content)
    } else {
        content
    }
}

/// Render a single component to string.
fn render_component(component: &ProcTemplateComponent) -> String {
    let rendering = component.template_component.rendering();

    let prefix = rendering.prefix.as_deref().unwrap_or_default();
    let suffix = rendering.suffix.as_deref().unwrap_or_default();
    let wrap = rendering.wrap.as_ref().unwrap_or(&WrapPunctuation::None);

    let (wrap_open, wrap_close) = match wrap {
        WrapPunctuation::None => ("", ""),
        WrapPunctuation::Parentheses => ("(", ")"),
        WrapPunctuation::Brackets => ("[", "]"),
    };

    // Apply emphasis/strong/quote
    let mut text = component.value.clone();
    if rendering.emph == Some(true) {
        text = format!("_{}_", text);
    }
    if rendering.strong == Some(true) {
        text = format!("**{}**", text);
    }
    if rendering.quote == Some(true) {
        text = format!("\"{}\"", text);
    }

    format!(
        "{}{}{}{}{}{}{}",
        wrap_open,
        prefix,
        component.prefix.as_deref().unwrap_or_default(),
        text,
        component.suffix.as_deref().unwrap_or_default(),
        suffix,
        wrap_close
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, Rendering,
        TemplateContributor, TemplateDate,
    };

    #[test]
    fn test_citation_to_string() {
        let template = vec![
            ProcTemplateComponent {
                template_component: TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Short,
                    delimiter: None,
                    rendering: Rendering::default(),
                }),
                value: "Kuhn".to_string(),
                prefix: None,
                suffix: None,
            },
            ProcTemplateComponent {
                template_component: TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                }),
                value: "1962".to_string(),
                prefix: None,
                suffix: None,
            },
        ];

        let result = citation_to_string(&template, true);
        assert_eq!(result, "(Kuhn, 1962)");
    }

    #[test]
    fn test_render_with_wrap() {
        let component = ProcTemplateComponent {
            template_component: TemplateComponent::Date(TemplateDate {
                date: DateVariable::Issued,
                form: DateForm::Year,
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses),
                    ..Default::default()
                },
            }),
            value: "1962".to_string(),
            prefix: None,
            suffix: None,
        };

        let result = render_component(&component);
        assert_eq!(result, "(1962)");
    }

    #[test]
    fn test_render_with_emphasis() {
        let component = ProcTemplateComponent {
            template_component: TemplateComponent::Title(csln_core::template::TemplateTitle {
                title: csln_core::template::TitleType::Primary,
                form: None,
                rendering: Rendering {
                    emph: Some(true),
                    ..Default::default()
                },
            }),
            value: "The Structure of Scientific Revolutions".to_string(),
            prefix: None,
            suffix: None,
        };

        let result = render_component(&component);
        assert_eq!(result, "_The Structure of Scientific Revolutions_");
    }
}
