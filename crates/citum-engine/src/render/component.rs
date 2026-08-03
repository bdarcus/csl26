/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::format::QuoteMarks;
use citum_schema::options::{Config, bibliography::BibliographyConfig, titles::TitleRendering};
use citum_schema::template::{Rendering, TemplateComponent, TitleType};
use std::sync::Arc;

/// A processed template component with its rendered value.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcTemplateComponent {
    /// The original template component (for rendering instructions).
    pub template_component: TemplateComponent,
    /// The 0-based source index in the active layout template, when requested.
    pub template_index: Option<usize>,
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
    pub config: Option<Arc<Config>>,
    /// Optional bibliography-only configuration.
    pub bibliography_config: Option<Arc<BibliographyConfig>>,
    /// Effective language for this rendered component.
    pub item_language: Option<String>,
    /// Locale-resolved quote mark characters, threaded from the active [`Locale`]'s
    /// [`GrammarOptions`](citum_schema::locale::GrammarOptions) so `quote`/`wrap: quotes`
    /// render the style's actual quotation convention instead of a hardcoded default.
    ///
    /// [`Locale`]: citum_schema::locale::Locale
    pub quote_marks: QuoteMarks,
    /// Whether this component begins a sentence according to processor-owned render context.
    pub sentence_initial: bool,
    /// Whether the value is already pre-formatted (e.g. from a List or substitution).
    pub pre_formatted: bool,
    /// True when this component's rendered text is *only* a bibliography
    /// numeric label (the renderer's synthetic `[label, following]`
    /// group with an empty `following`, e.g. no author). Bibliography
    /// assembly must still write the label text, but must not treat it as
    /// real preceding content for the *next* component's separator decision
    /// — the label attaches directly to whatever content actually opens the
    /// entry, exactly as if the label were not there.
    pub label_only: bool,
}

/// A processed template (list of rendered components).
pub type ProcTemplate = Vec<ProcTemplateComponent>;

/// A processed bibliography entry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcEntry {
    /// The reference ID.
    pub id: String,
    /// The processed template components.
    pub template: ProcTemplate,
    /// Metadata for interactivity (tooltips, etc.)
    pub metadata: super::format::ProcEntryMetadata,
}

use super::format::{OutputFormat, SemanticAttribute};
use super::plain::PlainText;
use std::borrow::Cow;

/// Resolve the semantic CSS class for a rendered component based on its template type.
fn resolve_semantic_class(component: &ProcTemplateComponent) -> Option<String> {
    use citum_schema::template::{DateVariable, SimpleVariable};
    match &component.template_component {
        TemplateComponent::Title(t) => match t.title {
            TitleType::Primary => Some("citum-title".to_string()),
            TitleType::ContainerTitle
            | TitleType::ParentMonograph
            | TitleType::ParentSerial
            | TitleType::CollectionTitle => Some("citum-container-title".to_string()),
            _ => Some("citum-title".to_string()),
        },
        TemplateComponent::Contributor(c) => Some(format!(
            "citum-{}",
            c.contributor
                .as_slice()
                .iter()
                .map(citum_schema::template::ContributorRole::as_str)
                .collect::<Vec<_>>()
                .join("-")
        )),
        TemplateComponent::Date(d) => Some(format!(
            "citum-{}",
            match d.date {
                DateVariable::Issued => "issued",
                DateVariable::Accessed => "accessed",
                DateVariable::OriginalPublished => "original-published",
                DateVariable::Submitted => "submitted",
                DateVariable::EventDate => "event-date",
                DateVariable::Copyright => "copyright",
                DateVariable::Printing => "printing",
            }
        )),
        TemplateComponent::Number(n) => Some(format!("citum-{}", n.number.as_key())),
        TemplateComponent::Identifier(identifier) => Some(format!(
            "citum-identifier-{}",
            identifier.identifier.as_str()
        )),
        TemplateComponent::Variable(v) => Some(format!(
            "citum-{}",
            match v.variable {
                SimpleVariable::Doi => "doi",
                SimpleVariable::Url => "url",
                SimpleVariable::Isbn => "isbn",
                SimpleVariable::Issn => "issn",
                SimpleVariable::Pmid => "pmid",
                SimpleVariable::Note => "note",
                SimpleVariable::Publisher => "publisher",
                SimpleVariable::PublisherPlace => "publisher-place",
                SimpleVariable::ContainerTitleShort => "container-title-short",
                SimpleVariable::Archive => "archive",
                _ => "variable",
            }
        )),
        TemplateComponent::Message(m) => Some(format!(
            "citum-message-{}",
            m.message
                .chars()
                .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
                .collect::<String>()
                .trim_matches('-')
        )),
        _ => None,
    }
}

/// Render a single component to string using the default `PlainText` format.
#[must_use]
pub fn render_component(component: &ProcTemplateComponent) -> String {
    PlainText.finish(render_component_with_format::<PlainText>(component))
}

/// Render a single component using a specific output format.
#[must_use]
pub fn render_component_with_format<F: OutputFormat<Output = String>>(
    component: &ProcTemplateComponent,
) -> F::Output {
    render_component_with_format_and_renderer::<F>(component, &F::default(), true)
}

/// A rendered component's text.
///
/// Exists so punctuation-in-quote join sites (`render/bibliography.rs`,
/// `render/citation.rs`, `render/punctuation.rs`) have a named type to
/// extend if a future increment needs to carry more than the text — today
/// they detect a movable leading mark from `text` itself, via
/// `render::punctuation::leading_movable_mark`, which compares the rendered
/// string's *visible* first character (not its raw one) against the
/// *visible* closing quote it might relocate into. An earlier version of
/// this fix instead typed the mark from the component's realized outer
/// `prefix`, gating the move on that alone; a CJK regression test caught
/// that this misses leading marks supplied by other means (value-extraction
/// prefixes, nested-group joins), so the visible-character check — a strict
/// superset of what the old raw sniff caught, just no longer defeated by a
/// semantic wrapper's markup — is the actual fix.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RenderedComponent {
    /// The fully rendered component text (affixes, wraps, semantics applied).
    pub(crate) text: String,
}

/// As [`render_component_with_format`], but returns a [`RenderedComponent`]
/// rather than a bare string.
pub(crate) fn render_component_detailed<F: OutputFormat<Output = String>>(
    component: &ProcTemplateComponent,
) -> RenderedComponent {
    render_component_detailed_with_format_and_renderer::<F>(component, &F::default(), true)
}

fn realized_component_affixes<'a>(
    rendering: &'a Rendering,
    script: crate::values::ScriptClass,
    realization: Option<&'a citum_schema::options::PunctuationRealization>,
) -> (Cow<'a, str>, Cow<'a, str>) {
    let realize = |punctuation: &'a citum_schema::template::DelimiterPunctuation, position| {
        super::format::realize_punctuation(punctuation, script, realization, position)
    };
    let prefix = rendering
        .prefix
        .as_ref()
        .map(|punctuation| realize(punctuation, super::format::PunctuationPosition::Prefix))
        .unwrap_or(Cow::Borrowed(""));
    let suffix = rendering
        .suffix
        .as_ref()
        .map(|punctuation| realize(punctuation, super::format::PunctuationPosition::Suffix))
        .unwrap_or(Cow::Borrowed(""));
    (prefix, suffix)
}

fn apply_component_semantics<F>(
    component: &ProcTemplateComponent,
    fmt: &F,
    show_semantics: bool,
    output: F::Output,
) -> F::Output
where
    F: OutputFormat<Output = String>,
{
    if !show_semantics {
        return output;
    }
    let Some(class) = resolve_semantic_class(component) else {
        return output;
    };
    let semantic_attributes = component
        .template_index
        .map(|index| {
            vec![SemanticAttribute {
                name: "data-index",
                value: index.to_string(),
            }]
        })
        .unwrap_or_default();
    fmt.semantic_with_attributes(&class, output, &semantic_attributes)
}

/// Render a single component using a specific output format and an existing renderer instance.
pub fn render_component_with_format_and_renderer<F: OutputFormat<Output = String>>(
    component: &ProcTemplateComponent,
    fmt: &F,
    show_semantics: bool,
) -> F::Output {
    render_component_detailed_with_format_and_renderer::<F>(component, fmt, show_semantics).text
}

/// As [`render_component_with_format_and_renderer`], but returns a
/// [`RenderedComponent`] rather than a bare string.
pub(crate) fn render_component_detailed_with_format_and_renderer<
    F: OutputFormat<Output = String>,
>(
    component: &ProcTemplateComponent,
    fmt: &F,
    show_semantics: bool,
) -> RenderedComponent {
    // Get merged rendering (global config + local settings + overrides)
    let rendering = get_effective_rendering(component);

    // Check if suppressed
    if rendering.suppress == Some(true) {
        return RenderedComponent::default();
    }

    let multilingual = component
        .config
        .as_ref()
        .and_then(|config| config.multilingual.as_ref());
    let (script, realization) = crate::values::punctuation_realization_context(
        component.item_language.as_deref(),
        multilingual,
        component.quote_marks.punctuation_realization.as_ref(),
    );
    let (prefix, suffix) = realized_component_affixes(&rendering, script, realization.as_deref());
    let inner_prefix = rendering
        .wrap
        .as_ref()
        .and_then(|w| w.inner_prefix.as_deref())
        .unwrap_or_default();
    let inner_suffix = rendering
        .wrap
        .as_ref()
        .and_then(|w| w.inner_suffix.as_deref())
        .unwrap_or_default();

    let mut output = if component.pre_formatted {
        // If already pre-formatted (e.g. from a List), don't escape again.
        // We just need to convert the String back to Output (which is String here).
        fmt.join(vec![component.value.clone()], "")
    } else {
        fmt.text(&component.value)
    };

    // Apply styles, links, inner affixes, wrap, outer affixes, then semantics.
    if rendering.emph == Some(true) {
        output = fmt.emph(output);
    }
    if rendering.strong == Some(true) {
        output = fmt.strong(output);
    }
    if rendering.small_caps == Some(true) {
        output = fmt.small_caps(output);
    }
    if rendering.vertical_align == Some(citum_schema::VerticalAlign::Superscript) {
        output = fmt.superscript(output);
    }
    // A `wrap: quotes` (applied below) already surrounds the value in quotation
    // marks; honoring the `quote` flag as well would double them (`““Title””`).
    // Only apply the flag when the wrap is not itself a quote wrap.
    let wrapped_in_quotes = rendering
        .wrap
        .as_ref()
        .is_some_and(|w| w.punctuation == citum_schema::template::WrapPunctuation::Quotes);
    if rendering.quote == Some(true) && !wrapped_in_quotes {
        output = fmt.quote(output, &component.quote_marks);
    }

    if let Some(url) = &component.url {
        output = fmt.link(url, output);
    }

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

    if let Some(wrap_config) = rendering.wrap.as_ref() {
        output = fmt.wrap_punctuation(
            &wrap_config.punctuation,
            output,
            &component.quote_marks,
            script,
            realization.as_deref(),
        );
    }

    if !prefix.is_empty() || !suffix.is_empty() {
        output = super::format::apply_punctuation_affixes(
            fmt,
            rendering
                .prefix
                .as_ref()
                .map(|punctuation| (punctuation, prefix.as_ref())),
            output,
            rendering
                .suffix
                .as_ref()
                .map(|punctuation| (punctuation, suffix.as_ref())),
        );
    }

    output = apply_component_semantics(component, fmt, show_semantics, output);

    // 7. Legacy literal-punctuation compatibility shim. Semantic punctuation
    // realizes before this point; this late remap remains only for external
    // bilingual styles authored with the original `punctuation: latin` option.
    if wants_latin_punctuation(component) {
        output = remap_to_latin_punctuation(output);
    }

    RenderedComponent { text: output }
}

/// Whether this component opts into the legacy literal-punctuation remap.
///
/// New styles express punctuation with semantic marks. This fixed compatibility
/// shim remains crate-wide because external literal-authored styles may place
/// punctuation at component, citation-section, or citation-spec boundaries.
pub(crate) fn wants_latin_punctuation(component: &ProcTemplateComponent) -> bool {
    let configured = component
        .config
        .as_ref()
        .and_then(|cfg| cfg.multilingual.as_ref())
        .and_then(|ml| ml.scripts.get("latin"))
        .is_some_and(|script| {
            script.punctuation == Some(citum_schema::options::PunctuationStyle::Latin)
        });

    configured && crate::values::is_latin_script_language(component.item_language.as_deref())
}

/// Apply the legacy fixed CJK-to-Latin literal-punctuation remap.
///
/// `：`(U+FF1A) → `: `, `，`(U+FF0C) → `, `, `（`(U+FF08) → `(`, `）`(U+FF09) → `)`,
/// then any resulting doubled space is collapsed. Do not extend this table;
/// new punctuation behavior belongs in semantic realization.
pub(crate) fn remap_to_latin_punctuation(text: String) -> String {
    if !text.contains(['：', '，', '（', '）']) {
        return text;
    }

    let mut mapped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '：' => mapped.push_str(": "),
            '，' => mapped.push_str(", "),
            '（' => mapped.push('('),
            '）' => mapped.push(')'),
            _ => mapped.push(ch),
        }
    }

    while mapped.contains("  ") {
        mapped = mapped.replace("  ", " ");
    }
    mapped
}

/// Get effective rendering, applying global config, then local template settings, then type-specific overrides.
#[must_use]
pub fn get_effective_rendering(component: &ProcTemplateComponent) -> Rendering {
    let mut effective = Rendering::default();

    // 1. Layer global config
    if let Some(config) = &component.config {
        match &component.template_component {
            TemplateComponent::Title(t) => {
                if let Some(global_title) = get_title_category_rendering(
                    &t.title,
                    component.ref_type.as_deref(),
                    component.item_language.as_deref(),
                    config,
                ) {
                    effective.merge(&global_title);
                }
            }
            TemplateComponent::Contributor(c) => {
                if let Some(contributors_config) = &config.contributors
                    && let Some(role_config) = &contributors_config.role
                    && let Some(primary_role) = c.contributor.as_slice().first()
                    && let Some(role_rendering) = role_config.role_rendering(primary_role)
                {
                    effective.merge(&role_rendering.to_rendering());
                }
            }
            // Add other component types here as we expand Config
            _ => {}
        }
    }

    // 2. Layer local template rendering
    effective.merge(component.template_component.rendering());

    effective
}

/// Resolve title-category-specific rendering overrides for a title component.
///
/// The returned rendering reflects title type, mapped reference category, and
/// optional language-specific overrides from the style configuration.
#[must_use]
pub fn get_title_category_rendering(
    title_type: &TitleType,
    ref_type: Option<&str>,
    language: Option<&str>,
    config: &Config,
) -> Option<Rendering> {
    get_title_category_title_rendering(title_type, ref_type, language, config)
        .map(|rendering| rendering.to_rendering())
}

/// Resolve title-category-specific title rendering options for a title component.
///
/// The returned rendering reflects title type, mapped reference category, and
/// optional language-specific overrides from the style configuration.
#[must_use]
pub fn get_title_category_title_rendering(
    title_type: &TitleType,
    ref_type: Option<&str>,
    language: Option<&str>,
    config: &Config,
) -> Option<TitleRendering> {
    let titles_config = config.titles.as_ref()?;

    // Use type_mapping if available to resolve category
    let mapped_category = ref_type.and_then(|rt| {
        titles_config
            .type_mapping
            .as_ref()
            .and_then(|mapping| mapping.get(rt))
    });

    use crate::values::type_class::TitleCategory;

    let rendering = match title_type {
        TitleType::ContainerTitle => {
            if let Some(cat) = mapped_category {
                match cat.as_str() {
                    "periodical" => titles_config.periodical.as_ref(),
                    "serial" => titles_config.serial.as_ref(),
                    "monograph" | "collection" => titles_config
                        .container_monograph
                        .as_ref()
                        .or(titles_config.monograph.as_ref()),
                    _ => titles_config.default.as_ref(),
                }
            } else if let Some(rt) = ref_type {
                match crate::values::type_class::container_title_category(rt) {
                    TitleCategory::Periodical => titles_config.periodical.as_ref(),
                    TitleCategory::ContainerMonograph => titles_config
                        .container_monograph
                        .as_ref()
                        .or(titles_config.monograph.as_ref()),
                    _ => titles_config.default.as_ref(),
                }
            } else {
                titles_config.default.as_ref()
            }
        }
        TitleType::ParentSerial => {
            if let Some(cat) = mapped_category {
                match cat.as_str() {
                    "periodical" => titles_config.periodical.as_ref(),
                    "serial" => titles_config.serial.as_ref(),
                    _ => titles_config.periodical.as_ref(),
                }
            } else if let Some(rt) = ref_type {
                match crate::values::type_class::parent_serial_title_category(rt) {
                    TitleCategory::Periodical => titles_config.periodical.as_ref(),
                    _ => titles_config.serial.as_ref(),
                }
            } else {
                titles_config.periodical.as_ref()
            }
        }
        TitleType::ParentMonograph => titles_config
            .container_monograph
            .as_ref()
            .or(titles_config.monograph.as_ref()),
        TitleType::CollectionTitle => titles_config
            .container_monograph
            .as_ref()
            .or(titles_config.monograph.as_ref())
            .or(titles_config.default.as_ref()),
        TitleType::Primary => {
            if let Some(cat) = mapped_category {
                match cat.as_str() {
                    "component" => titles_config.component.as_ref(),
                    "monograph" => titles_config.monograph.as_ref(),
                    _ => titles_config.default.as_ref(),
                }
            } else if let Some(rt) = ref_type {
                match crate::values::type_class::title_category(rt) {
                    TitleCategory::Component => titles_config.component.as_ref(),
                    TitleCategory::Monograph => titles_config.monograph.as_ref(),
                    _ => titles_config.default.as_ref(),
                }
            } else {
                titles_config.default.as_ref()
            }
        }
        _ => None,
    };

    let selected = rendering.or(titles_config.default.as_ref())?;
    let mut effective = selected.clone();
    if let Some(override_rendering) = selected.locale_override(language) {
        effective.merge(override_rendering);
    }
    Some(effective)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use citum_schema::template::{Rendering, TemplateComponent, TemplateTitle, TitleType};

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

    #[test]
    fn given_quote_flag_and_quote_wrap_when_render_then_single_pair_of_quotes() {
        use citum_schema::template::{WrapConfig, WrapPunctuation};

        // Migrated styles can carry both a global `titles.*.quote` flag and a
        // template `wrap: quotes`; applying both would double the quotes.
        let component = ProcTemplateComponent {
            template_component: TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                rendering: Rendering {
                    quote: Some(true),
                    wrap: Some(WrapConfig {
                        punctuation: WrapPunctuation::Quotes,
                        inner_prefix: None,
                        inner_suffix: None,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            }),
            value: "The Structure of Scientific Revolutions".to_string(),
            ..Default::default()
        };

        let result = render_component(&component);
        assert_eq!(
            result,
            "\u{201C}The Structure of Scientific Revolutions\u{201D}"
        );
    }

    #[test]
    fn given_quote_flag_and_non_quote_wrap_when_render_then_both_applied() {
        use citum_schema::template::{WrapConfig, WrapPunctuation};

        // A non-quote wrap (parentheses) does not subsume the quote flag, so
        // both must still apply.
        let component = ProcTemplateComponent {
            template_component: TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                rendering: Rendering {
                    quote: Some(true),
                    wrap: Some(WrapConfig {
                        punctuation: WrapPunctuation::Parentheses,
                        inner_prefix: None,
                        inner_suffix: None,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            }),
            value: "Title".to_string(),
            ..Default::default()
        };

        let result = render_component(&component);
        assert_eq!(result, "(\u{201C}Title\u{201D})");
    }
}
