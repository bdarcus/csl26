/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::render::component::{render_component_with_format, ProcTemplate};
use crate::render::format::OutputFormat;
use crate::render::plain::PlainText;
use csln_core::template::WrapPunctuation;

/// Render a processed template into a final citation string using PlainText format.
pub fn citation_to_string(
    proc_template: &ProcTemplate,
    wrap: Option<&WrapPunctuation>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    delimiter: Option<&str>,
) -> String {
    citation_to_string_with_format::<PlainText>(proc_template, wrap, prefix, suffix, delimiter)
}

/// Render a processed template into a final citation string using a specific format.
pub fn citation_to_string_with_format<F: OutputFormat<Output = String>>(
    proc_template: &ProcTemplate,
    wrap: Option<&WrapPunctuation>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    delimiter: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    for component in proc_template {
        let rendered = render_component_with_format::<F>(component);
        if !rendered.is_empty() {
            parts.push(rendered);
        }
    }

    let delim = delimiter.unwrap_or("");
    let content = parts.join(delim);

    let (open, close) = match wrap {
        Some(WrapPunctuation::Parentheses) => ("(", ")"),
        Some(WrapPunctuation::Brackets) => ("[", "]"),
        Some(WrapPunctuation::Quotes) => ("\u{201C}", "\u{201D}"),
        _ => (prefix.unwrap_or(""), suffix.unwrap_or("")),
    };

    format!("{}{}{}", open, content, close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::component::ProcTemplateComponent;
    use csln_core::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateComponent,
        TemplateContributor, TemplateDate,
    };

    #[test]
    fn test_citation_to_string() {
        let template = vec![
            ProcTemplateComponent {
                template_component: TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Short,
                    name_order: None,
                    delimiter: None,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                value: "Kuhn".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: None,
                url: None,
                pre_formatted: false,
            },
            ProcTemplateComponent {
                template_component: TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                value: "1962".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: None,
                url: None,
                pre_formatted: false,
            },
        ];

        let result = citation_to_string(
            &template,
            Some(&WrapPunctuation::Parentheses),
            None,
            None,
            Some(", "),
        );
        assert_eq!(result, "(Kuhn, 1962)");
    }
}
