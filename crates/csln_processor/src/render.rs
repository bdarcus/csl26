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

use csln_core::template::{TemplateComponent, WrapPunctuation, Rendering};
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
    /// Reference type for type-specific overrides.
    pub ref_type: Option<String>,
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
                    output.push_str(". ");
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
            .map_or(false, |c| {
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
    // Get base rendering and apply type-specific overrides if present
    let base_rendering = component.template_component.rendering();
    let rendering = get_effective_rendering(component, base_rendering);
    
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

/// Get effective rendering, applying type-specific overrides if present.
fn get_effective_rendering(component: &ProcTemplateComponent, base: &Rendering) -> Rendering {
    let ref_type = match &component.ref_type {
        Some(t) => t,
        None => return base.clone(),
    };
    
    // Check for overrides based on component type
    let overrides = match &component.template_component {
        TemplateComponent::Number(n) => n.overrides.as_ref(),
        TemplateComponent::Variable(v) => v.overrides.as_ref(),
        TemplateComponent::Title(t) => t.overrides.as_ref(),
        TemplateComponent::List(l) => l.overrides.as_ref(),
        _ => None,
    };
    
    if let Some(override_map) = overrides {
        if let Some(type_override) = override_map.get(ref_type) {
            // Merge: override takes precedence, but use base for None values
            return Rendering {
                emph: type_override.emph.or(base.emph),
                quote: type_override.quote.or(base.quote),
                strong: type_override.strong.or(base.strong),
                prefix: type_override.prefix.clone().or(base.prefix.clone()),
                suffix: type_override.suffix.clone().or(base.suffix.clone()),
                wrap: type_override.wrap.clone().or(base.wrap.clone()),
                suppress: type_override.suppress.or(base.suppress),
            };
        }
    }
    
    base.clone()
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
                    name_order: None,
                    delimiter: None,
                    rendering: Rendering::default(),
                }),
                value: "Kuhn".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
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
                ref_type: None,
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
            ref_type: None,
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
            }),
            value: "The Structure of Scientific Revolutions".to_string(),
            prefix: None,
            suffix: None,
            ref_type: None,
        };

        let result = render_component(&component);
        assert_eq!(result, "_The Structure of Scientific Revolutions_");
    }
}
