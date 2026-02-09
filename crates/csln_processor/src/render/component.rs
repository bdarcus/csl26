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

use super::format::OutputFormat;
use super::plain::PlainText;

/// Render a single component to string using the default PlainText format.
pub fn render_component(component: &ProcTemplateComponent) -> String {
    PlainText.finish(render_component_with_format::<PlainText>(component))
}

/// Render a single component using a specific output format.
pub fn render_component_with_format<F: OutputFormat>(
    component: &ProcTemplateComponent,
) -> F::Output {
    let fmt = F::default();

    // Get merged rendering (global config + local settings + overrides)
    let rendering = get_effective_rendering(component);

    // Check if suppressed
    if rendering.suppress == Some(true) {
        return fmt.text("");
    }

    let prefix = rendering.prefix.as_deref().unwrap_or_default();
    let suffix = rendering.suffix.as_deref().unwrap_or_default();
    let inner_prefix = rendering.inner_prefix.as_deref().unwrap_or_default();
    let inner_suffix = rendering.inner_suffix.as_deref().unwrap_or_default();
    let wrap = rendering.wrap.as_ref().unwrap_or(&WrapPunctuation::None);

    let mut output = fmt.text(&component.value);

    // Order of application:
    // 1. Text styles (emph, strong, etc.)
    // 2. Links
    // 3. Inner affixes
    // 4. Wrap
    // 5. Outer affixes
    // 6. Semantic classes (last, to wrap everything)

    // 1. Apply text styles
    if rendering.emph == Some(true) {
        output = fmt.emph(output);
    }
    if rendering.strong == Some(true) {
        output = fmt.strong(output);
    }
    if rendering.small_caps == Some(true) {
        output = fmt.small_caps(output);
    }
    if rendering.quote == Some(true) {
        output = fmt.quote(output);
    }

    // 2. Apply links if URL is present
    if let Some(url) = &component.url {
        output = fmt.link(url, output);
    }

    // 3. Inner affixes + extracted val prefix/suffix
    let total_inner_prefix = format!(
        "{}{}",
        inner_prefix,
        component.prefix.as_deref().unwrap_or_default()
    );
    let total_inner_suffix = format!(
        "{}{}",
        component.suffix.as_deref().unwrap_or_default(),
        inner_suffix
    );

    if !total_inner_prefix.is_empty() || !total_inner_suffix.is_empty() {
        output = fmt.inner_affix(&total_inner_prefix, output, &total_inner_suffix);
    }

    // 4. Wrap
    if *wrap != WrapPunctuation::None {
        output = fmt.wrap_punctuation(wrap, output);
    }

    // 5. Outer affixes
    if !prefix.is_empty() || !suffix.is_empty() {
        output = fmt.affix(prefix, output, suffix);
    }

    // 6. Apply semantic class based on component type
    let show_semantics = component
        .config
        .as_ref()
        .and_then(|c| c.semantic_classes)
        .unwrap_or(true);

    if show_semantics {
        use csln_core::template::{DateVariable, NumberVariable, SimpleVariable};
        let semantic_class = match &component.template_component {
            TemplateComponent::Title(t) => match t.title {
                TitleType::Primary => Some("csln-title".to_string()),
                TitleType::ParentMonograph | TitleType::ParentSerial => {
                    Some("csln-container-title".to_string())
                }
                _ => Some("csln-title".to_string()),
            },
            TemplateComponent::Contributor(c) => Some(format!("csln-{}", c.contributor.as_str())),
            TemplateComponent::Date(d) => Some(format!(
                "csln-{}",
                match d.date {
                    DateVariable::Issued => "issued",
                    DateVariable::Accessed => "accessed",
                    DateVariable::OriginalPublished => "original-published",
                    DateVariable::Submitted => "submitted",
                    DateVariable::EventDate => "event-date",
                }
            )),
            TemplateComponent::Number(n) => Some(format!(
                "csln-{}",
                match n.number {
                    NumberVariable::Volume => "volume",
                    NumberVariable::Issue => "issue",
                    NumberVariable::Pages => "pages",
                    NumberVariable::Edition => "edition",
                    NumberVariable::ChapterNumber => "chapter-number",
                    NumberVariable::CollectionNumber => "collection-number",
                    NumberVariable::NumberOfPages => "number-of-pages",
                    NumberVariable::NumberOfVolumes => "number-of-volumes",
                    NumberVariable::CitationNumber => "citation-number",
                    _ => "number",
                }
            )),
            TemplateComponent::Variable(v) => Some(format!(
                "csln-{}",
                match v.variable {
                    SimpleVariable::Doi => "doi",
                    SimpleVariable::Url => "url",
                    SimpleVariable::Isbn => "isbn",
                    SimpleVariable::Issn => "issn",
                    SimpleVariable::Pmid => "pmid",
                    SimpleVariable::Note => "note",
                    SimpleVariable::Publisher => "publisher",
                    SimpleVariable::PublisherPlace => "publisher-place",
                    SimpleVariable::Archive => "archive",
                    _ => "variable",
                }
            )),
            _ => None,
        };

        if let Some(class) = semantic_class {
            output = fmt.semantic(&class, output);
        }
    }

    output
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

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::template::{Rendering, TemplateComponent, TemplateTitle, TitleType};

    #[test]
    fn test_render_with_emphasis() {
        let component = ProcTemplateComponent {
            template_component: TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                rendering: Rendering {
                    emph: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            }),
            value: "The Structure of Scientific Revolutions".to_string(),
            ..Default::default()
        };

        let result = render_component(&component);
        assert_eq!(result, "_The Structure of Scientific Revolutions_");
    }
}
