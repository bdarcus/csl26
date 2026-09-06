/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::render::component::{ProcTemplate, RenderedComponent, render_component_detailed};
use crate::render::format::{OutputFormat, RealizedPunctuation};
use crate::render::plain::PlainText;
use crate::render::punctuation::{
    is_terminal_punctuation, leading_movable_mark, move_punctuation_into_quote,
    resolve_punctuation_collision, strong_terminal_comma_policy,
};
use citum_schema::template::WrapPunctuation;

/// Append `delim` then `next` to `content`, applying house-style punctuation rules at the
/// join point.
///
/// Cases are handled in priority order:
/// 1. **Punctuation-in-quote, delimiter-led** – when `punctuation_in_quote` is set and `delim`
///    starts with a period or comma, that mark is pulled *inside* a preceding closing
///    quotation mark before appending the rest of the delimiter and `next` verbatim.
/// 2. **Punctuation-in-quote, `next`-led** – when `delim` is empty (or does not itself start
///    with `.`/`,`) but `next` supplies its own leading period or comma (e.g. a component's own
///    `prefix: ". Aired "`), that mark is pulled inside the quote the same way, and the
///    remainder of `next` is appended after it.
/// 3. **Punctuation collision** – when format `F`'s *visible* last char of `content` and the
///    first char of `delim` are both terminal punctuation, the pair is resolved via
///    [`resolve_punctuation_collision`] (e.g. `".` + `". "` → `". "` rather than `".. "`). If
///    the raw content genuinely ends with that char, it's popped and merged as before; if the
///    visible terminal punctuation is hidden behind trailing markup (e.g. a LaTeX `\emph{...}`
///    close brace), the raw markup is left alone and the delimiter's redundant leading
///    punctuation is dropped instead.
/// 4. **Default** – append `delim` and `next` verbatim.
///
/// `delim` must already be decomposed from the same string that will be
/// spliced into `content` — see [`RealizedPunctuation`].
#[inline]
fn push_delimiter<F: OutputFormat<Output = String>>(
    content: &mut String,
    delim: &RealizedPunctuation<'_>,
    next: &RenderedComponent,
    punctuation_in_quote: bool,
    strong_terminal_comma_policy: citum_schema::options::StrongTerminalCommaPolicy,
    close_quote: &str,
) {
    let delim_first = delim.core();

    if punctuation_in_quote {
        if matches!(delim_first, Some('.' | ','))
            && move_punctuation_into_quote::<F>(content, delim_first.unwrap_or('.'), close_quote)
        {
            // Case 1: pull the leading period/comma of the delimiter inside the quote.
            content.push_str(delim.tail());
            content.push_str(&next.text);
            return;
        }
        if delim_first.is_none()
            && let Some((mark, rest)) = leading_movable_mark::<F>(&next.text)
            && move_punctuation_into_quote::<F>(content, mark, close_quote)
        {
            // Case 2: `next` supplies its own leading punctuation.
            content.push_str(&rest);
            return;
        }
    }

    let Some(first) = delim_first else {
        content.push_str(delim.text());
        content.push_str(&next.text);
        return;
    };
    let Some(visible_last) = F::default().visible_text(content).chars().last() else {
        content.push_str(delim.text());
        content.push_str(&next.text);
        return;
    };

    if !is_terminal_punctuation(visible_last) || !is_terminal_punctuation(first) {
        // Case 4: no special rule — append the delimiter verbatim.
        content.push_str(delim.text());
    } else if content.ends_with(visible_last) {
        // Case 3a: raw content genuinely ends with the visible terminal char — merge as before.
        content.pop();
        content.push_str(&resolve_punctuation_collision(
            visible_last,
            first,
            strong_terminal_comma_policy,
        ));
        content.push_str(delim.tail());
    } else {
        // Case 3b: the visible terminal punctuation is behind trailing markup (e.g. LaTeX
        // `}`). A retained comma can safely follow the markup wrapper; all collapsing cases
        // leave the raw markup alone and drop the incoming punctuation as before.
        if first == ','
            && crate::render::punctuation::is_strong_terminal(visible_last)
            && strong_terminal_comma_policy
                == citum_schema::options::StrongTerminalCommaPolicy::KeepBoth
        {
            content.push_str(delim.text());
        } else {
            content.push_str(delim.tail());
        }
    }
    content.push_str(&next.text);
}

/// Render a processed template into a final citation string using `PlainText` format.
#[must_use]
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
#[must_use]
pub fn citation_to_string_with_format<F: OutputFormat<Output = String>>(
    proc_template: &ProcTemplate,
    wrap: Option<&WrapPunctuation>,
    prefix: Option<&str>,
    suffix: Option<&str>,
    delimiter: Option<&str>,
) -> String {
    let mut parts: Vec<RenderedComponent> = Vec::new();

    for component in proc_template {
        let rendered = render_component_detailed::<F>(component);
        if !rendered.text.is_empty() {
            parts.push(rendered);
        }
    }

    let delim = RealizedPunctuation::new(std::borrow::Cow::Borrowed(delimiter.unwrap_or("")));
    let no_delim = RealizedPunctuation::new(std::borrow::Cow::Borrowed(""));
    let punctuation_in_quote = proc_template
        .first()
        .and_then(|c| c.config.as_ref())
        .is_some_and(|cfg| cfg.punctuation_in_quote);
    let strong_terminal_comma_policy = strong_terminal_comma_policy(
        proc_template
            .first()
            .and_then(|component| component.config.as_deref()),
    );
    let close_quote = proc_template
        .first()
        .map(|c| c.quote_marks.close.as_str())
        .unwrap_or("\u{201D}");

    let mut content = String::new();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            content.push_str(&part.text);
        } else {
            // A part whose own realized prefix already supplies a leading
            // separator (e.g. a component's own `", "`) must not also take
            // the shared item delimiter, or the join doubles up. Route
            // through `push_delimiter` with an empty delimiter rather than
            // skipping the call outright, so Case 2 (moving the part's own
            // leading `.`/`,` inside a preceding closing quote) still fires.
            //
            // A locator's resolved `attach` (`join_delimiter_override`)
            // takes precedence over both: it replaces the shared delimiter
            // outright rather than merely suppressing it, and — because it
            // is applied only here, never baked into the part's own
            // `text` — it cannot appear when this part is `i == 0` (see
            // `ProcTemplateComponent::locator_attach`'s doc comment).
            let override_delim = part
                .join_delimiter_override
                .as_deref()
                .map(|text| RealizedPunctuation::new(std::borrow::Cow::Borrowed(text)));
            let effective_delim = if let Some(override_delim) = override_delim.as_ref() {
                override_delim
            } else if part.supplies_own_leading_separator {
                &no_delim
            } else {
                &delim
            };
            push_delimiter::<F>(
                &mut content,
                effective_delim,
                part,
                punctuation_in_quote,
                strong_terminal_comma_policy,
                close_quote,
            );
        }
    }

    let (open, close) = match wrap {
        Some(WrapPunctuation::Parentheses) => ("(", ")"),
        Some(WrapPunctuation::Brackets) => ("[", "]"),
        Some(WrapPunctuation::Quotes) => ("\u{201C}", "\u{201D}"),
        _ => (prefix.unwrap_or(""), suffix.unwrap_or("")),
    };

    let assembled = format!("{open}{content}{close}");

    // The citation-level `delimiter`/`prefix`/`suffix`/`wrap` above are applied
    // outside each component's own rendering (which already remaps its own
    // full-width delimiters — see `render::component`), so a literal full-width
    // wrap like GB/T author-date's `prefix: （ suffix: ）` needs the same
    // script-aware remap applied here. All components in one citation render
    // share the same reference, so the first component's language stands in
    // for the whole citation.
    if proc_template
        .first()
        .is_some_and(crate::render::component::wants_latin_punctuation)
    {
        crate::render::component::remap_to_latin_punctuation(assembled)
    } else {
        assembled
    }
}

/// The `join_delimiter_override` of the first component in `proc_template`
/// that renders non-empty text, or `None` if none do or that first
/// component carries no override.
///
/// A grouped multi-item citation renders each item's author-stripped
/// template through [`citation_to_string_with_format`], but joins the
/// *externally*-rendered author heading to that result separately, using a
/// delimiter guessed from the template's structurally-first remaining
/// component (see `filter_author_from_template` in
/// `processor/rendering/grouped/core.rs`). That guess is wrong whenever the
/// structurally-first component (e.g. a `disambiguate-only` title) renders
/// empty at runtime and a later component with its own `attach` (e.g. a
/// locator, `docs/specs/LOCATOR_RENDERING.md`) becomes the item's true first
/// visible content. This function lets that external caller ask
/// `citation_to_string_with_format`'s own answer instead of re-guessing.
#[must_use]
pub fn leading_join_delimiter_override<F: OutputFormat<Output = String>>(
    proc_template: &ProcTemplate,
) -> Option<String> {
    for component in proc_template {
        let rendered = render_component_detailed::<F>(component);
        if !rendered.text.is_empty() {
            return rendered.join_delimiter_override;
        }
    }
    None
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
    use crate::render::component::ProcTemplateComponent;
    use crate::render::typst::Typst;
    use citum_schema::options::Config;
    use citum_schema::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, Rendering, TemplateComponent,
        TemplateContributor, TemplateDate, TemplateTitle, TitleType,
    };
    use rstest::rstest;

    /// A rendered part, standing in for the detailed render
    /// `push_delimiter`'s caller produces in production. A leading `.`/`,`
    /// in `text` is detected via `leading_movable_mark` from the text's own
    /// visible content, so no separate "with mark" variant is needed.
    fn part(text: &str) -> RenderedComponent {
        RenderedComponent {
            text: text.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_citation_to_string() {
        let template = vec![
            ProcTemplateComponent {
                template_component: TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author.into(),
                    form: ContributorForm::Short,
                    name_order: None,
                    delimiter: None,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                template_index: None,
                value: "Kuhn".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: None,
                bibliography_config: None,
                url: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                locator_attach: None,
            },
            ProcTemplateComponent {
                template_component: TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                template_index: None,
                value: "1962".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: None,
                bibliography_config: None,
                url: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                locator_attach: None,
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
    fn test_punctuation_in_quote_moves_comma_inside_closing_quote() {
        let config = Config {
            punctuation_in_quote: true,
            ..Default::default()
        };
        let template = vec![
            ProcTemplateComponent {
                template_component: TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    rendering: Rendering {
                        quote: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                template_index: None,
                value: "colon".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: Some(config.clone().into()),
                bibliography_config: None,
                url: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                locator_attach: None,
            },
            ProcTemplateComponent {
                template_component: TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                template_index: None,
                value: "period".to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: Some(config.into()),
                bibliography_config: None,
                url: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                locator_attach: None,
            },
        ];

        let plain = citation_to_string(&template, None, None, None, Some(", "));
        let typst =
            citation_to_string_with_format::<Typst>(&template, None, None, None, Some(", "));

        assert_eq!(plain, "“colon,” period");
        assert_eq!(typst, "“colon,” period");
    }

    #[rstest]
    #[case('.', "period")]
    #[case(',', "comma")]
    fn push_delimiter_moves_delimiter_led_mark_inside_closing_quote(
        #[case] mark: char,
        #[case] label: &str,
    ) {
        // Period was previously unhandled here — only comma-led delimiters moved.
        let mut content = "“Title”".to_string();
        let delim = RealizedPunctuation::new(format!("{mark} ").into());

        push_delimiter::<PlainText>(
            &mut content,
            &delim,
            &part("Next"),
            true,
            citum_schema::options::StrongTerminalCommaPolicy::default(),
            "”",
        );

        assert_eq!(
            content,
            format!("“Title{mark}” Next"),
            "{label}-led delimiter should move inside the quote"
        );
    }

    #[rstest]
    #[case('.', "period")]
    #[case(',', "comma")]
    fn push_delimiter_moves_next_part_own_leading_mark_inside_closing_quote(
        #[case] mark: char,
        #[case] label: &str,
    ) {
        // An empty delimiter (a `group:` with `delimiter: ''`) plus a next part
        // that supplies its own leading punctuation via a component-level
        // `prefix` (e.g. `prefix: ". Aired "` on the following component) — the
        // shape the delimiter-only check never saw.
        let mut content = "“Title”".to_string();
        let next = part(&format!("{mark} Aired 1980"));
        let delim = RealizedPunctuation::new("".into());

        push_delimiter::<PlainText>(
            &mut content,
            &delim,
            &next,
            true,
            citum_schema::options::StrongTerminalCommaPolicy::default(),
            "”",
        );

        assert_eq!(
            content,
            format!("“Title{mark}” Aired 1980"),
            "{label}-led next part should move inside the quote"
        );
    }

    #[test]
    fn push_delimiter_moves_mark_inside_a_locale_specific_close_quote() {
        // A French-style guillemet close quote rather than the en-US curly
        // quote — the hardcoded '"'/'\u{201D}' match this replaces would never
        // fire for this glyph.
        let mut content = "«Titre»".to_string();
        let delim = RealizedPunctuation::new(", ".into());

        push_delimiter::<PlainText>(
            &mut content,
            &delim,
            &part("Suite"),
            true,
            citum_schema::options::StrongTerminalCommaPolicy::default(),
            "»",
        );

        assert_eq!(content, "«Titre,» Suite");
    }

    #[test]
    fn push_delimiter_moves_next_part_own_leading_mark_when_it_is_not_a_typed_prefix() {
        // Regression: `test_chinese_article_three_part_title` caught an
        // earlier version of this fix that typed the leading mark from the
        // component's realized outer `prefix` and gated the move on that
        // alone. Here the leading `, ` comes from somewhere else entirely
        // (a value-extraction prefix or a nested group's own join, not a
        // template `prefix:`) — `leading_movable_mark` must not care about
        // the source, only whether the visible text opens with the mark.
        let mut content = "“Qingdai yilai Sanxia diqu shuihan zaihai de chubu yanjiu \
            [A preliminary study of floods and droughts since the Qing dynasty]”"
            .to_string();
        let next = part(", Zhongguo shehui kexue 1 (1999): 168-79.");
        let delim = RealizedPunctuation::new("".into());

        push_delimiter::<PlainText>(
            &mut content,
            &delim,
            &next,
            true,
            citum_schema::options::StrongTerminalCommaPolicy::default(),
            "”",
        );

        assert_eq!(
            content,
            "“Qingdai yilai Sanxia diqu shuihan zaihai de chubu yanjiu \
            [A preliminary study of floods and droughts since the Qing dynasty],” \
            Zhongguo shehui kexue 1 (1999): 168-79."
        );
    }

    #[test]
    fn test_punctuation_outside_quotes_preserves_full_monty_matrix() {
        let config = Config {
            punctuation_in_quote: false,
            ..Default::default()
        };
        let suffixes = [
            ("colon", ":"),
            ("period", "."),
            ("semicolon", ";"),
            ("exclamation", "!"),
            ("question", "?"),
            ("comma", ","),
        ];
        let delimiters = [
            ("ENDING IN COLON", ": "),
            ("ENDING IN PERIOD", ". "),
            ("ENDING IN SEMICOLON", "; "),
            ("ENDING IN EXCLAMATION", "! "),
            ("ENDING IN QUESTION", "? "),
            ("ENDING IN COMMA", ", "),
        ];

        let mut lines = Vec::new();
        for (heading, delimiter) in delimiters {
            lines.push(heading.to_string());
            for (value, suffix) in suffixes {
                let template = full_monty_template(&config, heading, value, suffix);
                lines.push(citation_to_string(
                    &template,
                    None,
                    None,
                    None,
                    Some(delimiter),
                ));
            }
        }

        let plain = lines.join("\n");
        let expected = r"ENDING IN COLON
“colon”: colon
“period”.: colon
“semicolon”; colon
“exclamation”! colon
“question”? colon
“comma”,: colon
ENDING IN PERIOD
“colon”: period
“period”. period
“semicolon”; period
“exclamation”! period
“question”? period
“comma”,. period
ENDING IN SEMICOLON
“colon”:; semicolon
“period”.; semicolon
“semicolon”; semicolon
“exclamation”!; semicolon
“question”?; semicolon
“comma”,; semicolon
ENDING IN EXCLAMATION
“colon”! exclamation
“period”.! exclamation
“semicolon”! exclamation
“exclamation”! exclamation
“question”?! exclamation
“comma”,! exclamation
ENDING IN QUESTION
“colon”? question
“period”.? question
“semicolon”? question
“exclamation”!? question
“question”? question
“comma”,? question
ENDING IN COMMA
“colon”:, comma
“period”., comma
“semicolon”;, comma
“exclamation”!, comma
“question”?, comma
“comma”, comma";

        let mut typst_lines = Vec::new();
        for (heading, delimiter) in delimiters {
            typst_lines.push(heading.to_string());
            for (value, suffix) in suffixes {
                let template = full_monty_template(&config, heading, value, suffix);
                typst_lines.push(citation_to_string_with_format::<Typst>(
                    &template,
                    None,
                    None,
                    None,
                    Some(delimiter),
                ));
            }
        }
        let typst = typst_lines.join("\n");

        assert_eq!(plain, expected);
        assert_eq!(typst, expected);
    }

    #[test]
    fn test_keep_terminal_policy_suppresses_comma_after_strong_terminal_marks() {
        let config = Config {
            punctuation: Some(citum_schema::options::PunctuationConfig {
                strong_terminal_comma_policy: Some(
                    citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal,
                ),
                delimiter_suppressing_terminal_marks: None,
            }),
            ..Default::default()
        };
        let cases = [
            ("question", "?", "“question”? comma"),
            ("exclamation", "!", "“exclamation”! comma"),
            ("ellipsis", "…", "“ellipsis”… comma"),
        ];

        for (value, suffix, expected) in cases {
            let template = full_monty_template(&config, "ENDING IN COMMA", value, suffix);
            assert_eq!(
                citation_to_string(&template, None, None, None, Some(", ")),
                expected
            );
        }
    }

    #[test]
    fn test_keep_both_policy_preserves_comma_after_ellipsis() {
        let config = Config::default();
        let template = full_monty_template(&config, "ENDING IN COMMA", "ellipsis", "…");

        assert_eq!(
            citation_to_string(&template, None, None, None, Some(", ")),
            "“ellipsis”…, comma"
        );
    }

    #[test]
    fn test_strong_terminal_comma_policy_applies_across_typst_markup() {
        let template = |policy| {
            let config = Config {
                punctuation: Some(citum_schema::options::PunctuationConfig {
                    strong_terminal_comma_policy: Some(policy),
                    delimiter_suppressing_terminal_marks: None,
                }),
                ..Default::default()
            };
            vec![
                ProcTemplateComponent {
                    template_component: TemplateComponent::Title(TemplateTitle {
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    value: "Title!".to_string(),
                    config: Some(config.clone().into()),
                    ..Default::default()
                },
                ProcTemplateComponent {
                    template_component: TemplateComponent::Date(TemplateDate {
                        date: DateVariable::Issued,
                        form: DateForm::Year,
                        ..Default::default()
                    }),
                    value: "next".to_string(),
                    config: Some(config.into()),
                    ..Default::default()
                },
            ]
        };

        assert_eq!(
            citation_to_string_with_format::<Typst>(
                &template(citum_schema::options::StrongTerminalCommaPolicy::KeepBoth),
                None,
                None,
                None,
                Some(", "),
            ),
            "#emph[Title!], next"
        );
        assert_eq!(
            citation_to_string_with_format::<Typst>(
                &template(citum_schema::options::StrongTerminalCommaPolicy::KeepTerminal),
                None,
                None,
                None,
                Some(", "),
            ),
            "#emph[Title!] next"
        );
    }

    fn full_monty_template(
        config: &Config,
        heading: &str,
        value: &str,
        suffix: &str,
    ) -> Vec<ProcTemplateComponent> {
        vec![
            ProcTemplateComponent {
                template_component: TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    rendering: Rendering {
                        quote: Some(true),
                        suffix: Some(suffix.into()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                template_index: None,
                value: value.to_string(),
                prefix: None,
                suffix: None,
                ref_type: None,
                config: Some(config.clone().into()),
                bibliography_config: None,
                url: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                locator_attach: None,
            },
            ProcTemplateComponent {
                template_component: TemplateComponent::Date(TemplateDate {
                    date: DateVariable::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                template_index: None,
                value: {
                    #[allow(
                        clippy::string_slice,
                        reason = "heading is guaranteed to start with prefix"
                    )]
                    let val = heading["ENDING IN ".len()..].to_ascii_lowercase();
                    val
                },
                prefix: None,
                suffix: None,
                ref_type: None,
                config: Some(config.clone().into()),
                bibliography_config: None,
                url: None,
                item_language: None,
                quote_marks: Default::default(),
                sentence_initial: false,
                pre_formatted: false,
                locator_attach: None,
            },
        ]
    }
}
