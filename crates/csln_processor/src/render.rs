/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Rendering utilities for CSLN templates.
//!
//! This module converts processed template components into final output strings.
//!
//! ## Design Notes
//!
//! The separator logic here is currently somewhat implicit (checking punctuation
//! characters). Ideally, separators should be explicitly declared in the style.
//!
//! TODO: Consider adding explicit `separator` field to template components,
//! allowing styles to declare `separator: ". "` or `separator: ", "` directly.
//! This would move the logic from processor to style, making behavior more
//! predictable and testable.

use csln_core::options::Config;
use csln_core::template::{Rendering, TemplateComponent, TitleType, WrapPunctuation};
use std::fmt::Write;

/// A processed template component with its rendered value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcTemplateComponent {
    /// The original template component (for rendering instructions).
    pub template_component: TemplateComponent,
    /// The processed values.
    pub value: String,
    /// Optional prefix from value extraction.
    pub prefix: Option<String>,
    /// Optional suffix from value extraction.
    pub suffix: Option<String>,
    /// Reference type for type-specific overrides.
    pub ref_type: Option<String>,
    /// Optional global configuration.
    pub config: Option<Config>,
}

/// A processed template (list of rendered components).
pub type ProcTemplate = Vec<ProcTemplateComponent>;

/// Render processed templates into a final bibliography string.
///
/// ## Separator Logic
///
/// The default separator between components is `. ` (period-space).
/// This is modified based on:
/// - Component's rendered prefix (comma/semicolon skip separator)
/// - Component type (dates always get period separator)
/// - Parenthetical content (gets space only, not period)
///
/// Components can override this via their `prefix` rendering field.
/// For example, `prefix: ", "` will suppress the default `. ` separator.
pub fn refs_to_string(proc_templates: Vec<ProcTemplate>) -> String {
    let mut output = String::new();
    for (i, proc_template) in proc_templates.iter().enumerate() {
        if i > 0 {
            output.push_str("\n\n");
        }

        // Check locale option for punctuation placement in quotes.
        // Extract from first component's config (all components share the same config).
        let punctuation_in_quote = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .is_some_and(|cfg| cfg.punctuation_in_quote);

        for (j, component) in proc_template.iter().enumerate() {
            let rendered = render_component(component);
            // Skip empty components (e.g., suppressed by type override)
            if rendered.is_empty() {
                continue;
            }

            // Add separator between components
            // NOTE: This logic is implicit based on punctuation. A future improvement
            // would be to add explicit `separator` field to template components.
            if j > 0 && !output.is_empty() {
                let last_char = output.chars().last().unwrap_or(' ');
                let first_char = rendered.chars().next().unwrap_or(' ');

                // Date components always need period separator (author-date style)
                let is_date = matches!(&component.template_component, TemplateComponent::Date(_));

                if matches!(first_char, ',' | ';' | ':') {
                    // Component provides its own punctuation via prefix (e.g., ", 436–444")
                    // No separator needed
                } else if first_char == '(' && !is_date {
                    // Parenthetical content (e.g., chapter pages "(pp. 1-10)")
                    // gets space only, not period
                    if !last_char.is_whitespace() {
                        output.push(' ');
                    }
                } else if !matches!(last_char, '.' | ',' | ':' | ';' | ' ') {
                    // Default: add period-space separator
                    // Locale option: place periods inside quotation marks (American style)
                    if punctuation_in_quote && last_char == '"' {
                        output.pop(); // Remove closing quote
                        output.push_str(".\" "); // Add period inside, then quote + space
                    } else {
                        output.push_str(". ");
                    }
                } else if last_char == '.' {
                    // Already have period, just add space
                    output.push(' ');
                } else if !last_char.is_whitespace() {
                    // After comma/colon/semicolon, just add space
                    output.push(' ');
                }
            }
            let _ = write!(&mut output, "{}", rendered);
        }

        // Add trailing period unless entry ends with DOI/URL
        // (links are self-terminating and shouldn't have period after)
        let last_is_link = proc_template.iter().rev()
            .find(|c| !render_component(c).is_empty())
            .is_some_and(|c| {
                matches!(&c.template_component,
                    TemplateComponent::Variable(v) if matches!(v.variable,
                        csln_core::template::SimpleVariable::Doi | csln_core::template::SimpleVariable::Url
                    )
                )
            });
        if !output.ends_with('.') && !last_is_link {
            // Locale option: place periods inside quotation marks (American style)
            if punctuation_in_quote && output.ends_with('"') {
                output.pop();
                output.push_str(".\"");
            } else {
                output.push('.');
            }
        }
    }
    output
}

/// Render a single citation.
///
/// Uses wrap if specified, otherwise falls back to prefix/suffix.
/// Delimiter defaults to ", " if not specified.
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
    let content = parts.join(delim);

    // wrap takes precedence over prefix/suffix
    let (open, close) = match wrap {
        Some(WrapPunctuation::Parentheses) => ("(", ")"),
        Some(WrapPunctuation::Brackets) => ("[", "]"),
        _ => (prefix.unwrap_or(""), suffix.unwrap_or("")),
    };

    format!("{}{}{}", open, content, close)
}

/// Render a single component to string.
pub fn render_component(component: &ProcTemplateComponent) -> String {
    // Get merged rendering (global config + local settings + overrides)
    let rendering = get_effective_rendering(component);

    // Check if suppressed
    if rendering.suppress == Some(true) {
        return String::new();
    }

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
    if rendering.small_caps == Some(true) {
        text = format!("<span style=\"font-variant:small-caps\">{}</span>", text);
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

/// Get effective rendering, applying global config, then local template settings, then type-specific overrides.
pub fn get_effective_rendering(component: &ProcTemplateComponent) -> Rendering {
    let mut effective = Rendering::default();

    // 1. Layer global config
    if let Some(config) = &component.config {
        match &component.template_component {
            TemplateComponent::Title(t) => {
                if let Some(global_title) =
                    get_title_category_rendering(&t.title, component.ref_type.as_deref(), config)
                {
                    effective.merge(&global_title);
                }
            }
            TemplateComponent::Contributor(c) => {
                if let Some(contributors_config) = &config.contributors {
                    if let Some(role_config) = &contributors_config.role {
                        if let Some(role_rendering) = role_config
                            .roles
                            .as_ref()
                            .and_then(|r| r.get(c.contributor.as_str()))
                        {
                            effective.merge(&role_rendering.to_rendering());
                        }
                    }
                }
            }
            // Add other component types here as we expand Config
            _ => {}
        }
    }

    // 2. Layer local template rendering
    effective.merge(component.template_component.rendering());

    // 3. Layer type-specific overrides
    if let Some(ref_type) = &component.ref_type {
        if let Some(overrides) = component.template_component.overrides() {
            if let Some(type_override) = overrides.get(ref_type) {
                effective.merge(type_override);
            }
        }
    }

    effective
}

pub fn get_title_category_rendering(
    title_type: &TitleType,
    ref_type: Option<&str>,
    config: &Config,
) -> Option<Rendering> {
    let titles_config = config.titles.as_ref()?;

    let rendering = match title_type {
        TitleType::ParentSerial => {
            if let Some(rt) = ref_type {
                if matches!(
                    rt,
                    "article-journal" | "article-magazine" | "article-newspaper"
                ) {
                    titles_config.periodical.as_ref()
                } else {
                    titles_config.serial.as_ref()
                }
            } else {
                titles_config.periodical.as_ref()
            }
        }
        TitleType::ParentMonograph => titles_config
            .container_monograph
            .as_ref()
            .or(titles_config.monograph.as_ref()),
        TitleType::Primary => {
            if let Some(rt) = ref_type {
                // "Component" titles: articles, chapters, entries - typically quoted
                if matches!(
                    rt,
                    "article-journal"
                        | "article-magazine"
                        | "article-newspaper"
                        | "chapter"
                        | "entry"
                        | "entry-dictionary"
                        | "entry-encyclopedia"
                        | "paper-conference"
                        | "post"
                        | "post-weblog"
                ) {
                    titles_config.component.as_ref()
                } else if matches!(rt, "book" | "thesis" | "report") {
                    titles_config.monograph.as_ref()
                } else {
                    titles_config.default.as_ref()
                }
            } else {
                titles_config.default.as_ref()
            }
        }
        _ => None,
    };

    rendering
        .map(|r| r.to_rendering())
        .or_else(|| titles_config.default.as_ref().map(|d| d.to_rendering()))
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
    fn test_citation_to_string_no_parens() {
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
            },
        ];

        // No wrap, space delimiter
        let result = citation_to_string(&template, None, None, None, Some(" "));
        assert_eq!(result, "Kuhn 1962");
    }

    #[test]
    fn test_citation_to_string_prefix_suffix_fallback() {
        let template = vec![ProcTemplateComponent {
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
        }];

        // Edge case: space before paren
        let result = citation_to_string(&template, None, Some(" ("), Some(")"), None);
        assert_eq!(result, " (Kuhn)");
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
                ..Default::default()
            }),
            value: "1962".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: None,
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
                overrides: None,
                ..Default::default()
            }),
            value: "The Structure of Scientific Revolutions".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
            config: None,
        };

        let result = render_component(&component);
        assert_eq!(result, "_The Structure of Scientific Revolutions_");
    }
}
