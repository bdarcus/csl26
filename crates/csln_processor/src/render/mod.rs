/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Rendering utilities for CSLN templates.

pub mod component;

pub use component::{render_component, ProcTemplate, ProcTemplateComponent};
use csln_core::template::{TemplateComponent, WrapPunctuation};
use std::fmt::Write;

/// Render processed templates into a final bibliography string.
pub fn refs_to_string(proc_templates: Vec<ProcTemplate>) -> String {
    let mut output = String::new();
    for (i, proc_template) in proc_templates.iter().enumerate() {
        if i > 0 {
            output.push_str("\n\n");
        }

        // Check locale option for punctuation placement in quotes.
        let punctuation_in_quote = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .is_some_and(|cfg| cfg.punctuation_in_quote);

        // Get the bibliography separator from the config, defaulting to ". "
        let default_separator = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .and_then(|cfg| cfg.bibliography.as_ref())
            .and_then(|bib| bib.separator.as_deref())
            .unwrap_or(". ");

        for (j, component) in proc_template.iter().enumerate() {
            let rendered = render_component(component);
            if rendered.is_empty() {
                continue;
            }

            // Add separator between components.
            //
            // The template's separator (from BibliographyConfig) joins components.
            // Components express any non-default separation via their prefix field.
            // This logic is intentionally simple: we skip adding the separator only
            // when punctuation is already present (to avoid doubling).
            if j > 0 && !output.is_empty() {
                let last_char = output.chars().last().unwrap_or(' ');
                let first_char = rendered.chars().next().unwrap_or(' ');

                // Derive the first punctuation/char of the separator for comparison
                let sep_first_char = default_separator.chars().next().unwrap_or('.');

                // Skip adding separator if:
                // 1. The rendered component already starts with separator-like punctuation
                // 2. The output already ends with separator-like punctuation
                // 3. Special handling for quotes with punctuation-in-quote locales
                let starts_with_separator = matches!(first_char, ',' | ';' | ':' | ' ' | '.' | '(');
                let ends_with_separator =
                    matches!(last_char, '.' | ',' | ':' | ';' | ' ' | ']' | ')');

                if starts_with_separator {
                    // Component prefix already provides separation (or opens with paren)
                    // If it starts with '(' and output doesn't end with space, add one
                    if first_char == '(' && !last_char.is_whitespace() {
                        output.push(' ');
                    }
                } else if ends_with_separator {
                    // Output already has punctuation; just add space if needed
                    if !last_char.is_whitespace() && last_char != ']' {
                        output.push(' ');
                    }
                } else if punctuation_in_quote
                    && (last_char == '"' || last_char == '\u{201D}')
                    && sep_first_char == '.'
                {
                    // Special case: move period inside closing quote for locales that want it
                    output.pop();
                    let quote_str = if last_char == '\u{201D}' {
                        ".\u{201D} "
                    } else {
                        ".\" "
                    };
                    output.push_str(quote_str);
                } else {
                    // Normal case: add the configured separator
                    output.push_str(default_separator);
                }
            }
            let _ = write!(&mut output, "{}", rendered);
        }

        // Apply entry suffix
        let bib_cfg = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .and_then(|cfg| cfg.bibliography.as_ref());
        let entry_suffix = bib_cfg.and_then(|bib| bib.entry_suffix.as_deref());
        match entry_suffix {
            Some(suffix) if !suffix.is_empty() => {
                // Always suppress trailing period after URLs/DOIs — virtually
                // no style wants "https://doi.org/10.1234/example." with a
                // trailing period. This mirrors citeproc-js suffix collapsing.
                let ends_with_url = ends_with_url_or_doi(&output);
                if ends_with_url {
                    // Skip entry suffix for entries ending with URL/DOI
                } else if !output.ends_with(suffix.chars().next().unwrap_or('.')) {
                    if suffix == "."
                        && punctuation_in_quote
                        && (output.ends_with('"') || output.ends_with('\u{201D}'))
                    {
                        let is_curly = output.ends_with('\u{201D}');
                        output.pop();
                        output.push_str(if is_curly { ".\u{201D}" } else { ".\"" });
                    } else {
                        output.push_str(suffix);
                    }
                }
            }
            _ => {}
        }
    }

    cleanup_dangling_punctuation(&mut output);
    output
}

#[allow(dead_code)]
fn is_link_component(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Variable(v) => {
            matches!(
                v.variable,
                csln_core::template::SimpleVariable::Doi | csln_core::template::SimpleVariable::Url
            )
        }
        TemplateComponent::List(list) => list.items.last().map(is_link_component).unwrap_or(false),
        _ => false,
    }
}

/// Check if the output ends with a URL or DOI (to suppress trailing period).
fn ends_with_url_or_doi(output: &str) -> bool {
    let trimmed = output.trim_end_matches('.');
    let trimmed = trimmed.trim_end();
    // Check if the last "word" looks like a URL or DOI
    if let Some(last_segment) = trimmed.rsplit_once(' ') {
        let last = last_segment.1;
        last.starts_with("https://") || last.starts_with("http://") || last.starts_with("doi.org/")
    } else {
        trimmed.starts_with("https://")
            || trimmed.starts_with("http://")
            || trimmed.starts_with("doi.org/")
    }
}

fn cleanup_dangling_punctuation(output: &mut String) {
    let patterns = [
        (", .", "."),
        (", ,", ","),
        (": .", "."),
        ("; .", "."),
        (",  ", ", "),
        (". .", "."),
        (".. ", ". "),
        ("..", "."),
    ];

    let mut changed = true;
    while changed {
        changed = false;
        for (pattern, replacement) in &patterns {
            if output.contains(pattern) {
                *output = output.replace(pattern, replacement);
                changed = true;
            }
        }
    }
}

pub fn citation_to_string(
    proc_template: &ProcTemplate,
    wrap: Option<&WrapPunctuation>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    delimiter: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    for component in proc_template {
        let rendered = render_component(component);
        if !rendered.is_empty() {
            parts.push(rendered);
        }
    }

    let delim = delimiter.unwrap_or(", ");

    let content = if parts.len() > 1 {
        let mut result = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                let prev = &parts[i - 1];
                if prev.ends_with('\u{201D}') || prev.ends_with('"') {
                    result.pop();
                    let delim_trimmed = delim.trim();
                    if prev.ends_with('\u{201D}') {
                        result.push_str(delim_trimmed);
                        result.push('\u{201D}');
                        result.push(' ');
                    } else {
                        result.push_str(delim_trimmed);
                        result.push('"');
                        result.push(' ');
                    }
                } else {
                    result.push_str(delim);
                }
            }
            result.push_str(part);
        }
        result
    } else {
        parts.join(delim)
    };

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
    use csln_core::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateContributor,
        TemplateDate,
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
                overrides: None,
                ..Default::default()
            }),
            value: "The Structure of Scientific Revolutions".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: None,
            url: None,
        };

        let result = render_component(&component);
        assert_eq!(result, "_The Structure of Scientific Revolutions_");
    }

    #[test]
    fn test_bibliography_separator_suppression() {
        use csln_core::options::{BibliographyConfig, Config};

        let config = Config {
            bibliography: Some(BibliographyConfig {
                separator: Some(". ".to_string()),
                entry_suffix: Some("".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let c1 = ProcTemplateComponent {
            template_component: TemplateComponent::Variable(
                csln_core::template::TemplateVariable {
                    variable: csln_core::template::SimpleVariable::Publisher,
                    rendering: Rendering::default(),
                    ..Default::default()
                },
            ),
            value: "Publisher1".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone()),
            url: None,
        };

        let c2 = ProcTemplateComponent {
            template_component: TemplateComponent::Variable(
                csln_core::template::TemplateVariable {
                    variable: csln_core::template::SimpleVariable::PublisherPlace,
                    rendering: Rendering {
                        prefix: Some(". ".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            value: "Place".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config),
            url: None,
        };

        let template = vec![vec![c1, c2]];
        let result = refs_to_string(template);
        assert_eq!(result, "Publisher1. Place");
    }
}
