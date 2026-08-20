/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for title fields with smartening, form selection,
//! and text-case transforms.

use crate::reference::Reference;
use crate::render::format::unicode_quote_marks;
use crate::render::rich_text::{
    InlineRenderContext, render_djot_inline_with_transform_and_context,
};
use crate::values::text_case::{self, apply_text_case_with_language};
use crate::values::{
    ComponentValues, ProcHints, ProcValues, RenderOptions, effective_component_language,
};
use citum_schema::locale::Locale;
use citum_schema::options::titles::{TextCase, TitleRendering};
use citum_schema::reference::ClassExtension;
use citum_schema::reference::types::{StructuredTitle, Subtitle, Title};
use citum_schema::template::{TemplateComponent, TemplateTitle, TitleForm, TitleType};

/// Converts straight apostrophes and double quotes to curly quotes when the
/// surrounding context is unambiguous.
///
/// Ambiguous characters are preserved as straight quotes so titles containing
/// measurements or other non-quotation uses do not get rewritten arbitrarily.
fn smarten_title_quotes_at_depth(input: &str, quote_depth: usize) -> String {
    let mut out = String::with_capacity(input.len());
    let mut it = input.char_indices().peekable();
    let mut prev: Option<char> = None;
    let mut open_single_quotes = 0usize;
    let mut open_double_quotes = 0usize;

    while let Some((_, ch)) = it.next() {
        let next = it.peek().map(|(_, c)| *c);
        let prev_is_alpha = prev.is_some_and(char::is_alphabetic);
        let prev_is_digit = prev.is_some_and(|c| c.is_ascii_digit());
        let prev_can_close_double_quote = prev.is_some_and(|c| {
            c.is_alphanumeric() || matches!(c, '\'' | '"' | '\u{2019}' | '\u{201D}')
        });
        let next_is_alpha = next.is_some_and(char::is_alphabetic);
        let next_is_digit = next.is_some_and(|c| c.is_ascii_digit());
        let next_is_alnum = next.is_some_and(char::is_alphanumeric);
        let prev_opens_quote =
            prev.is_none_or(|c| c.is_whitespace() || "([{\u{2018}\u{201C}'\"".contains(c));
        let next_closes_quote =
            next.is_none_or(|c| c.is_whitespace() || ".,;:!?)]}\u{2019}\u{201D}'\"".contains(c));

        match ch {
            '\'' => {
                let (open_quote, close_quote) = unicode_quote_marks(quote_depth + 1);
                if (prev_is_alpha && next_is_alpha) || (prev_opens_quote && next_is_digit) {
                    out.push('\u{2019}');
                } else if prev_opens_quote && next_is_alnum {
                    out.push_str(open_quote);
                    open_single_quotes += 1;
                } else if (open_single_quotes > 0 || prev_is_alpha || prev_is_digit)
                    && next_closes_quote
                {
                    out.push_str(close_quote);
                    open_single_quotes = open_single_quotes.saturating_sub(1);
                } else {
                    out.push('\'');
                }
            }
            '"' => {
                let (open_quote, close_quote) =
                    unicode_quote_marks(quote_depth + open_double_quotes);
                if prev_opens_quote && next_is_alnum {
                    out.push_str(open_quote);
                    open_double_quotes += 1;
                } else if open_double_quotes > 0 && prev_can_close_double_quote && next_closes_quote
                {
                    let close_depth = quote_depth + open_double_quotes.saturating_sub(1);
                    let (_, close_quote) = unicode_quote_marks(close_depth);
                    out.push_str(close_quote);
                    open_double_quotes -= 1;
                } else if prev_is_alpha && next_closes_quote {
                    out.push_str(close_quote);
                } else {
                    out.push('"');
                }
            }
            _ => out.push(ch),
        }

        prev = Some(ch);
    }
    out
}

fn title_text(title: &Title, form: Option<&TitleForm>) -> String {
    match title {
        Title::Shorthand(short, long) => {
            if matches!(form, Some(TitleForm::Short)) {
                short.clone()
            } else {
                long.clone()
            }
        }
        Title::Single(s) => s.clone(),
        _ => title.to_string(),
    }
}

fn parent_short_title(reference: &Reference, title_type: &TitleType) -> Option<String> {
    match title_type {
        TitleType::ContainerTitle => reference.container_title().and_then(|t| match t {
            Title::Shorthand(short, _) => Some(short),
            Title::Single(s) => Some(s),
            _ => None,
        }),
        TitleType::ParentMonograph => {
            if reference.ref_type() == "chapter" || reference.ref_type() == "paper-conference" {
                reference.container_title().and_then(|t| match t {
                    Title::Shorthand(short, _) => Some(short),
                    Title::Single(s) => Some(s),
                    _ => None,
                })
            } else {
                None
            }
        }
        TitleType::ParentSerial => {
            if crate::values::type_class::is_serial_parent_type(&reference.ref_type()) {
                reference.container_title().and_then(|t| match t {
                    Title::Shorthand(short, _) => Some(short),
                    Title::Single(s) => Some(s),
                    _ => None,
                })
            } else {
                None
            }
        }
        TitleType::CollectionTitle => reference.collection_title().and_then(|t| match t {
            Title::Shorthand(short, _) => Some(short),
            Title::Single(s) => Some(s),
            _ => None,
        }),
        _ => None,
    }
}

/// Cheap heuristic for whether `value` contains Djot inline markup
/// (`_emph_`, `*strong*`, `[text](url)`, `[text]{.class}`, `` `code` ``),
/// used both to route title rendering through the Djot pipeline and to
/// decide whether a sort key needs markup stripped (see `sort_support`'s
/// `title_sort_text`) — kept as one predicate so the two paths agree on
/// what counts as markup.
pub(crate) fn looks_like_djot_markup(value: &str) -> bool {
    value.contains('_')
        || value.contains('*')
        || value.contains("](")
        || value.contains("{.")
        || value.contains('`')
}

/// Build a text-transform closure that applies case transform then smart quotes.
///
/// The closure is used as the Djot text-leaf transform, so `.nocase` spans
/// bypass it automatically via the rich-text renderer.
fn make_case_transform(
    case: TextCase,
    quote_depth: usize,
    language: Option<&str>,
) -> impl FnMut(&str) -> String {
    let mut capitalize_first_pending = true;
    let language = text_case::language_identifier_for_tag(language);
    move |text: &str| {
        let cased = match case {
            TextCase::Sentence | TextCase::SentenceApa | TextCase::SentenceNlm => {
                // Word-level sentence case, shared with the plain-text path so a
                // title's casing does not depend on whether it happens to contain
                // Djot markup: words carrying internal capitalization (acronyms,
                // mixed-case names) are preserved verbatim rather than flattened,
                // and the "capitalize the title's first word" state threads across
                // text leaves split around markup (e.g. emphasis).
                let (result, pending) =
                    text_case::sentence_case_words(text, &language, capitalize_first_pending);
                capitalize_first_pending = pending;
                result
            }
            _ => text_case::apply_text_case_with_language_id(text, case, &language),
        };
        smarten_title_quotes_at_depth(&cased, quote_depth)
    }
}

/// Render a single title part through Djot with case transform + smart quotes.
/// Returns (`rendered_value`, `has_explicit_link`).
fn render_part_with_case<F: crate::render::format::OutputFormat<Output = String>>(
    value: &str,
    fmt: &F,
    case: Option<TextCase>,
    quote_depth: usize,
    language: Option<&str>,
) -> (String, bool) {
    let context = InlineRenderContext { quote_depth };
    if looks_like_djot_markup(value) {
        match case {
            Some(tc) => render_djot_inline_with_transform_and_context(
                value,
                fmt,
                context,
                make_case_transform(tc, quote_depth, language),
            ),
            None => {
                render_djot_inline_with_transform_and_context(value, fmt, context, move |text| {
                    smarten_title_quotes_at_depth(text, quote_depth)
                })
            }
        }
    } else {
        let result = match case {
            Some(tc) => smarten_title_quotes_at_depth(
                &apply_text_case_with_language(value, tc, language),
                quote_depth,
            ),
            None => smarten_title_quotes_at_depth(value, quote_depth),
        };
        (result, false)
    }
}

/// Render a structured title with per-part case transforms.
///
/// For `SentenceApa`, each subtitle gets sentence-case (first word capitalized).
/// For `SentenceNlm`, subtitles are lowercased (no first-word capitalization).
fn render_structured_title<F: crate::render::format::OutputFormat<Output = String>>(
    st: &StructuredTitle,
    fmt: &F,
    case: Option<TextCase>,
    short: bool,
    quote_depth: usize,
    language: Option<&str>,
    delimiters: (&str, &str),
) -> (String, bool) {
    let (main_rendered, has_link) =
        render_part_with_case(&st.main, fmt, case, quote_depth, language);
    if short {
        return (main_rendered, has_link);
    }

    let subtitle_case = case.map(|c| match c {
        TextCase::SentenceNlm => TextCase::Lowercase,
        other => other,
    });

    let mut has_link = has_link;

    let subs: Vec<&str> = match &st.sub {
        Subtitle::String(s) => vec![s.as_str()],
        Subtitle::Vector(v) => v.iter().map(std::string::String::as_str).collect(),
    };
    if subs.is_empty() {
        return (main_rendered, has_link);
    }

    let mut rendered_subtitles = Vec::with_capacity(subs.len());
    for sub in subs {
        let (sub_rendered, sub_link) =
            render_part_with_case(sub, fmt, subtitle_case, quote_depth, language);
        has_link |= sub_link;
        rendered_subtitles.push(sub_rendered);
    }
    let (primary_delimiter, subtitle_delimiter) = delimiters;
    let subtitle_group = rendered_subtitles.join(subtitle_delimiter);

    let mut rendered = main_rendered;
    rendered.push_str(primary_delimiter);
    rendered.push_str(&subtitle_group);

    (rendered, has_link)
}

/// Resolve the effective text-case for this title component.
fn resolve_effective_text_case(
    template: &TemplateTitle,
    reference: &Reference,
    options: &RenderOptions<'_>,
    language: Option<&str>,
) -> Option<TextCase> {
    // 1. Template-level override takes precedence
    if let Some(tc) = template.rendering.text_case {
        return Some(apply_language_fallback(tc, language));
    }

    // 2. Global title-category config
    let ref_type = reference.ref_type();

    if let Some(rendering) = crate::render::component::get_title_category_rendering(
        &template.title,
        Some(&ref_type),
        language,
        &options.config,
    ) && let Some(tc) = rendering.text_case
    {
        return Some(apply_language_fallback(tc, language));
    }

    None
}

/// Resolve the effective text-case for a title rendered outside its normal
/// `title:` template component, e.g. when the CSL `substitute` chain falls
/// through to `title` because the reference has no author/editor/translator.
///
/// There is no `TemplateTitle` to consult for a per-component override in
/// that path, so this only resolves the style's category-level `titles:`
/// configuration (mirroring the second half of [`resolve_effective_text_case`]).
pub(crate) fn resolve_substitute_text_case(
    title_type: &TitleType,
    reference: &Reference,
    options: &RenderOptions<'_>,
) -> Option<TextCase> {
    let ref_type = reference.ref_type();
    let language = effective_title_language(title_type, reference);
    let rendering = crate::render::component::get_title_category_rendering(
        title_type,
        Some(&ref_type),
        language.as_deref(),
        &options.config,
    )?;
    let tc = rendering.text_case?;
    Some(apply_language_fallback(tc, language.as_deref()))
}

/// Render a substituted title string through the same Djot-aware pipeline as
/// [`TemplateTitle::values`] (case transform with `.nocase` protection, smart
/// quotes), for use by the author-substitute chain which has no
/// `TemplateTitle` component to route the string through directly.
///
/// Returns `(rendered_value, has_explicit_link, is_djot)` — `is_djot` tells
/// the caller whether `rendered_value` is already format-rendered (Djot path)
/// or still needs `fmt.text(...)` escaping (plain path), matching how
/// [`render_title_variant`] reports `pre_formatted`.
pub(crate) fn render_substitute_title_text<
    F: crate::render::format::OutputFormat<Output = String>,
>(
    value: &str,
    fmt: &F,
    case: Option<TextCase>,
    quote_depth: usize,
    language: Option<&str>,
) -> (String, bool, bool) {
    let is_djot = looks_like_djot_markup(value);
    let (rendered, has_explicit_link) =
        render_part_with_case(value, fmt, case, quote_depth, language);
    (rendered, has_explicit_link, is_djot)
}

fn resolve_effective_title_rendering(
    template: &TemplateTitle,
    reference: &Reference,
    options: &RenderOptions<'_>,
) -> Option<TitleRendering> {
    let component = TemplateComponent::Title(template.clone());
    let item_language = effective_component_language(reference, &component);
    let ref_type = reference.ref_type();
    crate::render::component::get_title_category_title_rendering(
        &template.title,
        Some(&ref_type),
        item_language.as_deref(),
        &options.config,
    )
}

fn structured_title_delimiters<'a>(
    rendering: Option<&'a TitleRendering>,
    locale: &'a Locale,
) -> (&'a str, &'a str) {
    let primary_delimiter = rendering
        .and_then(|rendering| rendering.primary_delimiter.as_deref())
        .unwrap_or(locale.grammar_options.title_subtitle_delimiter.as_str());
    let subtitle_delimiter = rendering
        .and_then(|rendering| rendering.subtitle_delimiter.as_deref())
        .unwrap_or(locale.grammar_options.subtitle_delimiter.as_str());
    (primary_delimiter, subtitle_delimiter)
}

fn effective_title_quote_depth(
    template: &TemplateTitle,
    reference: &Reference,
    options: &RenderOptions<'_>,
) -> usize {
    let component = TemplateComponent::Title(template.clone());
    let item_language = effective_component_language(reference, &component);
    let mut rendering = crate::render::component::get_title_category_rendering(
        &template.title,
        options.ref_type.as_deref(),
        item_language.as_deref(),
        &options.config,
    )
    .unwrap_or_default();
    rendering.merge(&template.rendering);
    usize::from(rendering.quote == Some(true))
}

/// Apply language-aware fallback: non-English → as-is for English-specific transforms.
fn apply_language_fallback(case: TextCase, language: Option<&str>) -> TextCase {
    text_case::resolve_text_case(case, language)
}

pub(crate) fn effective_title_language(
    title_type: &TitleType,
    reference: &Reference,
) -> Option<String> {
    let component = TemplateComponent::Title(TemplateTitle {
        title: title_type.clone(),
        ..Default::default()
    });
    effective_component_language(reference, &component)
}

impl ComponentValues for TemplateTitle {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        if self.disambiguate_only == Some(true)
            && (hints.group_length <= 1 || hints.suppress_disambiguation_title)
        {
            return None;
        }

        let quote_depth = effective_title_quote_depth(self, reference, options);

        if matches!(self.form, Some(TitleForm::Short))
            && let Some(short_title) = parent_short_title(reference, &self.title)
            && !short_title.is_empty()
        {
            let (value, pre_formatted) = if looks_like_djot_markup(&short_title) {
                let (v, _) = render_djot_inline_with_transform_and_context(
                    &short_title,
                    &F::default(),
                    InlineRenderContext { quote_depth },
                    move |text| smarten_title_quotes_at_depth(text, quote_depth),
                );
                (v, true)
            } else {
                (
                    smarten_title_quotes_at_depth(&short_title, quote_depth),
                    false,
                )
            };
            let value = crate::values::apply_abbreviation(value, options.abbreviation_map);
            let value = if self.strip_periods_all == Some(true) {
                crate::values::strip_all_periods(&value)
            } else {
                value
            };
            return Some(ProcValues {
                value,
                prefix: None,
                suffix: None,
                url: None,
                substituted_key: None,
                pre_formatted,
            });
        }

        let title = resolve_primary_title(reference, &self.title)?;
        let effective_language = effective_title_language(&self.title, reference);
        let effective_case =
            resolve_effective_text_case(self, reference, options, effective_language.as_deref());
        let effective_title_rendering = resolve_effective_title_rendering(self, reference, options);
        let (value, has_explicit_link, pre_formatted) = render_title_variant::<F>(
            &title,
            self.form.as_ref(),
            effective_case,
            effective_title_rendering.as_ref(),
            options,
            quote_depth,
            effective_language.as_deref(),
        );

        if value.is_empty() {
            return None;
        }

        use citum_schema::options::LinkAnchor;
        let value = crate::values::apply_abbreviation(value, options.abbreviation_map);
        let value = if self.strip_periods_all == Some(true) {
            crate::values::strip_all_periods(&value)
        } else {
            value
        };
        let url = crate::values::resolve_effective_url(
            self.links.as_ref(),
            options.config.links.as_ref(),
            reference,
            LinkAnchor::Title,
        );
        Some(ProcValues {
            value,
            prefix: None,
            suffix: None,
            url: if has_explicit_link { None } else { url },
            substituted_key: None,
            pre_formatted,
        })
    }
}

/// Resolve which title field to render for the given `TitleType` and reference.
fn resolve_primary_title(reference: &Reference, title_type: &TitleType) -> Option<Title> {
    match title_type {
        TitleType::Primary => reference.title(),
        TitleType::ContainerTitle => reference.container_title(),
        TitleType::ParentMonograph => match reference.extension() {
            ClassExtension::Monograph(_)
            | ClassExtension::CollectionComponent(_)
            | ClassExtension::Event(_)
            | ClassExtension::AudioVisual(_) => reference.container_title(),
            _ => None,
        },
        TitleType::ParentSerial => match reference.extension() {
            ClassExtension::SerialComponent(_)
            | ClassExtension::LegalCase(_)
            | ClassExtension::Treaty(_) => reference.container_title(),
            _ => None,
        },
        TitleType::CollectionTitle => reference.collection_title(),
        TitleType::Original => reference.original_title(),
        _ => None,
    }
}

/// Render a `Title` value into `(rendered_string, has_explicit_link, pre_formatted)`.
///
/// Handles structured, multilingual, and plain title variants with case transforms.
fn render_title_variant<F: crate::render::format::OutputFormat<Output = String>>(
    title: &Title,
    form: Option<&TitleForm>,
    effective_case: Option<TextCase>,
    effective_title_rendering: Option<&TitleRendering>,
    options: &RenderOptions<'_>,
    quote_depth: usize,
    language: Option<&str>,
) -> (String, bool, bool) {
    let fmt = F::default();
    match title {
        Title::Structured(st) => {
            let short = matches!(form, Some(TitleForm::Short));
            let delimiters = structured_title_delimiters(effective_title_rendering, options.locale);
            let (value, has_link) = render_structured_title(
                st,
                &fmt,
                effective_case,
                short,
                quote_depth,
                language,
                delimiters,
            );
            let pre_formatted = if short {
                looks_like_djot_markup(&st.main)
            } else {
                looks_like_djot_markup(&title_text(title, form))
            };
            (value, has_link, pre_formatted)
        }
        Title::Multilingual(m) => {
            let (mode, preferred_transliteration, preferred_script) =
                resolve_multilingual_title_config(options);
            let complex = citum_schema::reference::types::MultilingualString::Complex(m.clone());
            let value = crate::values::resolve_multilingual_string(
                &complex,
                mode,
                preferred_transliteration,
                preferred_script,
                options.locale.locale.as_str(),
            );
            let (rendered, has_link) =
                render_part_with_case(&value, &fmt, effective_case, quote_depth, language);
            let pre_formatted = looks_like_djot_markup(&value);
            (rendered, has_link, pre_formatted)
        }
        _ => {
            let value = title_text(title, form);
            let (rendered, has_link) =
                render_part_with_case(&value, &fmt, effective_case, quote_depth, language);
            let pre_formatted = looks_like_djot_markup(&value);
            (rendered, has_link, pre_formatted)
        }
    }
}

/// Resolve multilingual title config (mode, transliteration, script) from render options.
fn resolve_multilingual_title_config<'a>(
    options: &'a RenderOptions<'a>,
) -> (
    Option<&'a citum_schema::options::MultilingualMode>,
    Option<&'a [String]>,
    Option<&'a String>,
) {
    let mode = options
        .config
        .multilingual
        .as_ref()
        .and_then(|ml| ml.title_mode.as_ref());
    let preferred_transliteration = options
        .config
        .multilingual
        .as_ref()
        .and_then(|ml| ml.preferred_transliteration.as_deref());
    let preferred_script = options
        .config
        .multilingual
        .as_ref()
        .and_then(|ml| ml.preferred_script.as_ref());
    (mode, preferred_transliteration, preferred_script)
}
