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
//! The separator between bibliography components can be configured via
//! `bibliography.separator` in the style's options (e.g., ". " for Chicago/APA,
//! ", " for Elsevier). The renderer also checks component prefixes to skip
//! automatic separators when components provide their own punctuation.

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
    /// Optional URL for hyperlinking.
    pub url: Option<String>,
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
/// The separator between components is configured via `bibliography.separator`
/// in the style's options (defaults to ". " if not specified).
/// This is modified based on:
/// - Component's rendered prefix (comma/semicolon skip separator)
/// - Component type (dates always get period separator)
/// - Parenthetical content (gets space only, not period)
///
/// Components can override this via their `prefix` rendering field.
/// For example, `prefix: ", "` will suppress the automatic separator.
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

        // Get the bibliography separator from the config, defaulting to ". "
        let default_separator = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .and_then(|cfg| cfg.bibliography.as_ref())
            .and_then(|bib| bib.separator.as_deref())
            .unwrap_or(". ");

        for (j, component) in proc_template.iter().enumerate() {
            let rendered = render_component(component);
            // Skip empty components (e.g., suppressed by type override)
            if rendered.is_empty() {
                continue;
            }

            // Add separator between components
            // The separator is configured via bibliography.separator (defaults to ". ")
            if j > 0 && !output.is_empty() {
                let last_char = output.chars().last().unwrap_or(' ');
                let first_char = rendered.chars().next().unwrap_or(' ');

                // Date components always need period separator (author-date style)
                let is_date = matches!(&component.template_component, TemplateComponent::Date(_));

                if matches!(first_char, ',' | ';' | ':' | ' ') {
                    // Component provides its own punctuation/spacing via prefix
                    // No separator needed
                } else if first_char == '(' && !is_date {
                    // Parenthetical content (e.g., chapter pages "(pp. 1-10)")
                    // gets space only, not period
                    if !last_char.is_whitespace() {
                        output.push(' ');
                    }
                } else if !matches!(last_char, '.' | ',' | ':' | ';' | ' ' | ']') {
                    // Default: add separator between components
                    // Note: ']' is excluded for numeric citations where [1] directly precedes author
                    // Locale option: place periods inside quotation marks (American style)
                    if punctuation_in_quote
                        && (last_char == '"' || last_char == '\u{201D}')
                        && default_separator.starts_with('.')
                    {
                        output.pop(); // Remove closing quote
                                      // Use matching quote style
                        let quote_str = if last_char == '\u{201D}' {
                            ".\u{201D} "
                        } else {
                            ".\" "
                        };
                        output.push_str(quote_str);
                    } else {
                        output.push_str(default_separator);
                    }
                } else if last_char == '.' {
                    // Already have period, just add space
                    output.push(' ');
                } else if last_char == ']' {
                    // After closing bracket (numeric citations), no separator
                    // IEEE: "[1]Author" not "[1] Author"
                } else if !last_char.is_whitespace() {
                    // After comma/colon/semicolon, just add space
                    output.push(' ');
                }
            }
            let _ = write!(&mut output, "{}", rendered);
        }

        // Apply entry suffix from bibliography config (extracted from CSL layout suffix).
        // If explicitly set, use that value; otherwise use heuristic (period unless DOI/URL).
        let entry_suffix = proc_template
            .first()
            .and_then(|c| c.config.as_ref())
            .and_then(|cfg| cfg.bibliography.as_ref())
            .and_then(|bib| bib.entry_suffix.as_deref());

        match entry_suffix {
            Some(suffix) if !suffix.is_empty() => {
                // Explicit suffix from CSL layout (e.g., ".")
                // Don't double-add if entry already ends with this punctuation
                if !output.ends_with(suffix.chars().next().unwrap_or('.')) {
                    // Handle punctuation-in-quote for period suffix
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
            Some(_) => {
                // Empty suffix explicitly set - no entry-terminating punctuation
            }
            None => {
                // No explicit suffix - use legacy heuristic: add period unless DOI/URL
                let last_is_link = proc_template
                    .iter()
                    .rev()
                    .find(|c| !render_component(c).is_empty())
                    .is_some_and(|c| {
                        matches!(
                            &c.template_component,
                            TemplateComponent::Variable(v)
                                if matches!(
                                    v.variable,
                                    csln_core::template::SimpleVariable::Doi
                                        | csln_core::template::SimpleVariable::Url
                                )
                        )
                    });
                if !output.ends_with('.') && !last_is_link {
                    if punctuation_in_quote
                        && (output.ends_with('"') || output.ends_with('\u{201D}'))
                    {
                        let is_curly = output.ends_with('\u{201D}');
                        output.pop();
                        output.push_str(if is_curly { ".\u{201D}" } else { ".\"" });
                    } else {
                        output.push('.');
                    }
                }
            }
        }
    }

    // Clean up dangling punctuation from empty components.
    // When a component is empty/suppressed, its predecessor's suffix may be left behind.
    // E.g., "Publisher, " + empty location = "Publisher, ." → should be "Publisher."
    cleanup_dangling_punctuation(&mut output);

    output
}

/// Remove dangling punctuation patterns caused by empty components.
///
/// Patterns cleaned:
/// - ", ." → "."  (comma before final period)
/// - ", ," → ","  (double comma)
/// - ": ." → "."  (colon before final period)
/// - "; ." → "."  (semicolon before final period)
/// - ",  " → ", " (extra space after comma)
fn cleanup_dangling_punctuation(output: &mut String) {
    // Replace common patterns iteratively until no more changes
    let patterns = [
        (", .", "."),
        (", ,", ","),
        (": .", "."),
        ("; .", "."),
        (",  ", ", "),
        (". .", "."),
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

    // Join parts with delimiter, but move delimiter inside quotes for substituted titles.
    // APA convention: ("Title", 2020) not ("Title," 2020)
    let content = if parts.len() > 1 {
        let mut result = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                // Check if previous part ended with a closing quote
                let prev = &parts[i - 1];
                if prev.ends_with('\u{201D}') || prev.ends_with('"') {
                    // Move delimiter inside the quote (APA/CSL convention)
                    // For "Title" with delimiter ", " → "Title," not "Title, "
                    result.pop(); // Remove the closing quote we just added
                    let delim_trimmed = delim.trim(); // Remove spaces from delimiter
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

    // wrap takes precedence over prefix/suffix
    let (open, close) = match wrap {
        Some(WrapPunctuation::Parentheses) => ("(", ")"),
        Some(WrapPunctuation::Brackets) => ("[", "]"),
        Some(WrapPunctuation::Quotes) => ("\u{201C}", "\u{201D}"), // U+201C (") and U+201D (")
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
        WrapPunctuation::Quotes => ("\u{201C}", "\u{201D}"), // U+201C (") and U+201D (")
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
        text = format!("\u{201C}{}\u{201D}", text); // U+201C (") and U+201D (")
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
            url: None,
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
            url: None,
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
            url: None,
        };

        let result = render_component(&component);
        assert_eq!(result, "_The Structure of Scientific Revolutions_");
    }
}
