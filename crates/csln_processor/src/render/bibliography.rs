/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::render::component::{render_component_with_format, ProcTemplate};
use crate::render::format::OutputFormat;
use crate::render::plain::PlainText;
use std::fmt::Write;

/// Render processed templates into a final bibliography string using PlainText format.
pub fn refs_to_string(proc_templates: Vec<ProcTemplate>) -> String {
    refs_to_string_with_format::<PlainText>(proc_templates)
}

/// Render processed templates into a final bibliography string using a specific format.
pub fn refs_to_string_with_format<F: OutputFormat<Output = String>>(
    proc_templates: Vec<ProcTemplate>,
) -> String {
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
            let rendered = render_component_with_format::<F>(component);
            if rendered.is_empty() {
                continue;
            }

            // Add separator between components.
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
                let ends_with_separator = matches!(last_char, '.' | ',' | ':' | ';' | ' ');

                if starts_with_separator {
                    // Component prefix already provides separation (or opens with paren)
                    // If it starts with '(' and output doesn't end with space, add one
                    if first_char == '(' && !last_char.is_whitespace() && last_char != '[' {
                        output.push(' ');
                    }
                } else if ends_with_separator {
                    // Output already has punctuation; just add space if needed
                    if !last_char.is_whitespace() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::component::ProcTemplateComponent;
    use csln_core::template::{Rendering, TemplateComponent};

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

    #[test]
    fn test_no_suppression_after_parenthesis() {
        use csln_core::options::{BibliographyConfig, Config};

        let config = Config {
            bibliography: Some(BibliographyConfig {
                separator: Some(", ".to_string()),
                entry_suffix: Some("".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let c1 = ProcTemplateComponent {
            template_component: TemplateComponent::Contributor(
                csln_core::template::TemplateContributor {
                    contributor: csln_core::template::ContributorRole::Editor,
                    rendering: Rendering {
                        wrap: Some(csln_core::template::WrapPunctuation::Parentheses),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            value: "Eds.".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config.clone()),
            url: None,
        };

        let c2 = ProcTemplateComponent {
            template_component: TemplateComponent::Title(csln_core::template::TemplateTitle {
                title: csln_core::template::TitleType::Primary,
                rendering: Rendering::default(),
                ..Default::default()
            }),
            value: "Title".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: Some(config),
            url: None,
        };

        let template = vec![vec![c1, c2]];
        let result = refs_to_string(template);
        assert_eq!(result, "(Eds.), Title");
    }
}
