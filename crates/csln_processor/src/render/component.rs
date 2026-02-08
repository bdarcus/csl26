/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use csln_core::options::Config;
use csln_core::template::{Rendering, TemplateComponent, TitleType, WrapPunctuation};

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
    let inner_prefix = rendering.inner_prefix.as_deref().unwrap_or_default();
    let inner_suffix = rendering.inner_suffix.as_deref().unwrap_or_default();
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

    // Build output: outer_prefix + wrap_open + inner_prefix + extracted_prefix + text + extracted_suffix + inner_suffix + wrap_close + outer_suffix
    format!(
        "{}{}{}{}{}{}{}{}{}",
        prefix,
        wrap_open,
        inner_prefix,
        component.prefix.as_deref().unwrap_or_default(),
        text,
        component.suffix.as_deref().unwrap_or_default(),
        inner_suffix,
        wrap_close,
        suffix
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

    // Use type_mapping if available to resolve category
    let mapped_category = ref_type.and_then(|rt| titles_config.type_mapping.get(rt));

    let rendering = match title_type {
        TitleType::ParentSerial => {
            if let Some(cat) = mapped_category {
                match cat.as_str() {
                    "periodical" => titles_config.periodical.as_ref(),
                    "serial" => titles_config.serial.as_ref(),
                    _ => titles_config.periodical.as_ref(),
                }
            } else if let Some(rt) = ref_type {
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
            if let Some(cat) = mapped_category {
                match cat.as_str() {
                    "component" => titles_config.component.as_ref(),
                    "monograph" => titles_config.monograph.as_ref(),
                    _ => titles_config.default.as_ref(),
                }
            } else if let Some(rt) = ref_type {
                // Legacy hardcoded logic
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
