/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Shared punctuation classification and collision resolution.

use crate::render::component::RenderedComponent;
use crate::render::format::{OutputFormat, RealizedPunctuation};
use citum_schema::options::{Config, StrongTerminalCommaPolicy};

/// Return the resolved strong-terminal/comma policy from a processed config.
pub(crate) fn strong_terminal_comma_policy(config: Option<&Config>) -> StrongTerminalCommaPolicy {
    config
        .and_then(|config| config.punctuation.as_ref())
        .and_then(|punctuation| punctuation.strong_terminal_comma_policy)
        .unwrap_or_default()
}

/// The three punctuation classes named in `PUNCTUATION_NORMALIZATION.md`,
/// informed by Unicode UAX #14/#29 categories: sentence-terminal marks split
/// into strong (never collapsed away by a following comma, absent a
/// collapsing locale policy) and weak, and comma-like marks form their own
/// class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PunctuationClass {
    /// `!`, `?`, `…`.
    StrongTerminal,
    /// `.`, `:`.
    WeakTerminal,
    /// `,`, `;`.
    CommaLike,
}

impl PunctuationClass {
    /// Classify `ch`, or return `None` when it does not participate in
    /// punctuation-collision resolution.
    pub(crate) fn of(ch: char) -> Option<Self> {
        match ch {
            '!' | '?' | '…' => Some(Self::StrongTerminal),
            '.' | ':' => Some(Self::WeakTerminal),
            ',' | ';' => Some(Self::CommaLike),
            _ => None,
        }
    }
}

/// Return whether `ch` participates in punctuation-collision resolution.
pub(crate) fn is_terminal_punctuation(ch: char) -> bool {
    PunctuationClass::of(ch).is_some()
}

/// Return whether `ch` is a strong terminal punctuation mark.
pub(crate) fn is_strong_terminal(ch: char) -> bool {
    PunctuationClass::of(ch) == Some(PunctuationClass::StrongTerminal)
}

/// Visible text of `fragment` under `F`, paired with each visible byte's raw
/// index in `fragment` — built from `visible_runs` (the raw-byte-accurate
/// primitive; see its own doc for why this must not use `visible_text`
/// directly), so callers can locate an edit point in the raw fragment from a
/// match found in the visible projection. Shared by
/// [`move_punctuation_into_quote`]/[`first_visible_char_and_raw_range`]'s slow
/// path and `render/bibliography.rs`'s `cleanup_dangling_punctuation`.
pub(crate) fn visible_projection<F: OutputFormat<Output = String>>(
    fragment: &str,
) -> (String, Vec<usize>) {
    let fmt = F::default();
    let runs = fmt.visible_runs(fragment);
    let mut visible = String::with_capacity(fragment.len());
    let mut raw_pos = Vec::with_capacity(fragment.len());
    for run in runs {
        if let Some(slice) = fragment.get(run.clone()) {
            visible.push_str(slice);
            raw_pos.extend(run);
        }
    }
    (visible, raw_pos)
}

/// Return whether `fragment` is entirely visible under `F` (no markup at
/// all) — the common case, always true for
/// [`PlainText`](crate::render::plain::PlainText). Callers use this to skip
/// [`visible_projection`]'s allocation on the hot, markup-free path.
fn is_fully_visible<F: OutputFormat<Output = String>>(fragment: &str) -> bool {
    let runs = F::default().visible_runs(fragment);
    runs.len() == 1 && runs.first() == Some(&(0..fragment.len()))
}

/// Return the raw byte index in `fragment` at which `target` begins, when
/// `target` is the visible (markup-stripped) suffix of `fragment` under `F`.
/// `None` when it is not.
fn visible_suffix_raw_index<F: OutputFormat<Output = String>>(
    fragment: &str,
    target: &str,
) -> Option<usize> {
    if target.is_empty() {
        return None;
    }
    if is_fully_visible::<F>(fragment) {
        return fragment
            .ends_with(target)
            .then(|| fragment.len() - target.len());
    }
    let (visible, raw_pos) = visible_projection::<F>(fragment);
    if !visible.ends_with(target) {
        return None;
    }
    raw_pos.get(visible.len() - target.len()).copied()
}

/// Return the first visible character of `text` under `F`, together with the
/// raw byte range it occupies — in a single pass over `visible_runs`, so
/// callers needing both the character's identity and its raw removal range
/// (e.g. [`leading_movable_mark`]) don't project the same fragment twice.
fn first_visible_char_and_raw_range<F: OutputFormat<Output = String>>(
    text: &str,
) -> Option<(char, std::ops::Range<usize>)> {
    if is_fully_visible::<F>(text) {
        let ch = text.chars().next()?;
        return Some((ch, 0..ch.len_utf8()));
    }
    let (visible, raw_pos) = visible_projection::<F>(text);
    let ch = visible.chars().next()?;
    let start = *raw_pos.first()?;
    Some((ch, start..start + ch.len_utf8()))
}

/// Detect a movable leading `.`/`,` at the *visible* start of `text` under
/// `F`, and return it together with `text` minus that mark's raw bytes.
///
/// The mark may come from anywhere in `text`'s construction — a template
/// `prefix:`, a value-extraction prefix, a nested group's own join — the
/// source doesn't matter; what matters is whether the rendered component's
/// *visible* content genuinely opens with one of these marks. This is
/// deliberately not narrowed to a single typed source: an earlier version of
/// this fix gated the move on a mark typed from the component's realized
/// outer `prefix` alone, which a CJK regression test caught missing marks
/// supplied by other means. `None` when the visible text does not lead with
/// `.`/`,` at all.
pub(crate) fn leading_movable_mark<F: OutputFormat<Output = String>>(
    text: &str,
) -> Option<(char, String)> {
    let (mark, range) = first_visible_char_and_raw_range::<F>(text)?;
    if !matches!(mark, '.' | ',') {
        return None;
    }
    #[allow(
        clippy::string_slice,
        reason = "range is derived from visible_runs/char boundaries"
    )]
    let rest = format!("{}{}", &text[..range.start], &text[range.end..]);
    Some((mark, rest))
}

/// Move a trailing `punct` (`.` or `,`) inside a preceding closing quotation
/// mark, in place. `close_quote` is the locale-resolved closing glyph (e.g.
/// `\u{201D}` for en-US, `\u{00BB}` for fr-FR); a bare `"` is also accepted as
/// a legacy fallback for literal-authored ASCII quotes. Both are located via
/// [`visible_suffix_raw_index`], so the closing mark is found even when
/// markup (an HTML `</span>`, a LaTeX `}`, ...) trails it in the raw string.
/// Returns `false` (leaving `accumulated` untouched) when neither glyph is
/// found at the visible end.
pub(crate) fn move_punctuation_into_quote<F: OutputFormat<Output = String>>(
    accumulated: &mut String,
    punct: char,
    close_quote: &str,
) -> bool {
    if !close_quote.is_empty()
        && let Some(idx) = visible_suffix_raw_index::<F>(accumulated, close_quote)
    {
        accumulated.insert(idx, punct);
        return true;
    }
    if close_quote != "\""
        && let Some(idx) = visible_suffix_raw_index::<F>(accumulated, "\"")
    {
        accumulated.insert(idx, punct);
        return true;
    }
    false
}

/// Join `parts` with `delimiter`, applying [`move_punctuation_into_quote`] at each boundary
/// when `punctuation_in_quote` is active. The leading period or comma to move may come from
/// `delimiter` itself, or — when `delimiter` is empty — from the next part's own leading mark,
/// detected via [`leading_movable_mark`] (e.g. a component's self-supplied `prefix`, the shape
/// `group:` delimiters rely on for self-delimiting items). Used for `group:` template joins,
/// which otherwise have no punctuation dynamics at all.
///
/// `delimiter` must already be decomposed from the same (post-escape) string
/// that will be spliced into the output — see [`RealizedPunctuation`].
pub(crate) fn join_with_quote_movement<F: OutputFormat<Output = String>>(
    parts: Vec<RenderedComponent>,
    delimiter: &RealizedPunctuation<'_>,
    punctuation_in_quote: bool,
    close_quote: &str,
) -> String {
    let mut iter = parts.into_iter();
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut result = first.text;

    for part in iter {
        let delim_first = delimiter.core();

        let moved_via_delimiter = punctuation_in_quote
            && matches!(delim_first, Some('.' | ','))
            && move_punctuation_into_quote::<F>(
                &mut result,
                delim_first.unwrap_or('.'),
                close_quote,
            );

        if moved_via_delimiter {
            result.push_str(delimiter.tail());
            result.push_str(&part.text);
            continue;
        }

        if punctuation_in_quote
            && delim_first.is_none()
            && let Some((mark, rest)) = leading_movable_mark::<F>(&part.text)
            && move_punctuation_into_quote::<F>(&mut result, mark, close_quote)
        {
            result.push_str(&rest);
            continue;
        }

        result.push_str(delimiter.text());
        result.push_str(&part.text);
    }

    result
}

/// Return the visible character immediately preceding the point where
/// [`move_punctuation_into_quote`] would insert a trailing mark: just inside
/// a closing quotation glyph when one sits at the visible end of `text`, or
/// the last non-whitespace visible character otherwise. Used to check
/// whether a mark about to be appended (e.g. a bibliography entry suffix)
/// collides with punctuation already there — via [`resolve_punctuation_collision`]
/// — rather than being genuinely additive (a quoted title already ending in
/// `?` doesn't also need a trailing `.`: `"?".` is wrong, `?"` is right).
pub(crate) fn char_before_insertion_point<F: OutputFormat<Output = String>>(
    text: &str,
    close_quote: &str,
) -> Option<char> {
    let visible = F::default().visible_text(text);
    let mut chars: Vec<char> = visible.chars().collect();
    while matches!(chars.last(), Some(ch) if ch.is_whitespace()) {
        chars.pop();
    }
    if !close_quote.is_empty() && chars.ends_with(&close_quote.chars().collect::<Vec<_>>()) {
        chars.truncate(chars.len().saturating_sub(close_quote.chars().count()));
    } else if close_quote != "\"" && chars.last() == Some(&'"') {
        chars.pop();
    }
    while matches!(chars.last(), Some(ch) if ch.is_whitespace()) {
        chars.pop();
    }
    chars.last().copied()
}

/// Resolve one punctuation pair while preserving the established compatibility matrix.
pub(crate) fn resolve_punctuation_collision(
    first: char,
    second: char,
    strong_terminal_comma_policy: StrongTerminalCommaPolicy,
) -> String {
    if second == ','
        && is_strong_terminal(first)
        && strong_terminal_comma_policy == StrongTerminalCommaPolicy::KeepTerminal
    {
        return first.to_string();
    }

    match (first, second) {
        (':', ':') => ":".to_string(),
        ('.', ':') => ".:".to_string(),
        (';', ':') => ";".to_string(),
        ('!', ':') => "!".to_string(),
        ('?', ':') => "?".to_string(),
        (',', ':') => ",:".to_string(),
        (':', '.') => ":".to_string(),
        ('.', '.') => ".".to_string(),
        (';', '.') => ";".to_string(),
        ('!', '.') => "!".to_string(),
        ('?', '.') => "?".to_string(),
        (',', '.') => ",.".to_string(),
        (':', ';') => ":;".to_string(),
        ('.', ';') => ".;".to_string(),
        (';', ';') => ";".to_string(),
        ('!', ';') => "!;".to_string(),
        ('?', ';') => "?;".to_string(),
        (',', ';') => ",;".to_string(),
        (':', '!') => "!".to_string(),
        ('.', '!') => ".!".to_string(),
        (';', '!') => "!".to_string(),
        ('!', '!') => "!".to_string(),
        ('?', '!') => "?!".to_string(),
        (',', '!') => ",!".to_string(),
        (':', '?') => "?".to_string(),
        ('.', '?') => ".?".to_string(),
        (';', '?') => "?".to_string(),
        ('!', '?') => "!?".to_string(),
        ('?', '?') => "?".to_string(),
        (',', '?') => ",?".to_string(),
        (':', ',') => ":,".to_string(),
        ('.', ',') => ".,".to_string(),
        (';', ',') => ";,".to_string(),
        ('!', ',') => "!,".to_string(),
        ('?', ',') => "?,".to_string(),
        (',', ',') => ",".to_string(),
        _ => format!("{first}{second}"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;
    use crate::render::html::Html;
    use crate::render::latex::Latex;
    use crate::render::plain::PlainText;
    use rstest::rstest;

    /// A rendered part, standing in for the detailed render
    /// `join_with_quote_movement`'s caller produces in production. A leading
    /// `.`/`,` in `text` is detected via [`leading_movable_mark`] from the
    /// text's own visible content, so no separate "with mark" variant is
    /// needed.
    fn part(text: &str) -> RenderedComponent {
        RenderedComponent {
            text: text.to_string(),
            ..Default::default()
        }
    }

    #[rstest]
    #[case::strong_terminal_before_close_quote("“Are Flaxseeds All That?”", "\u{201D}", Some('?'))]
    #[case::weak_terminal_before_close_quote("“Staff, 23 June 2025:”", "\u{201D}", Some(':'))]
    #[case::locale_specific_guillemet_close("«Titre?»", "»", Some('?'))]
    #[case::no_quote_present_falls_back_to_bare_last_char("Vous descendez?", "\u{201D}", Some('?'))]
    #[case::trailing_whitespace_is_skipped("“Deep Learning?” ", "\u{201D}", Some('?'))]
    #[case::empty_text("", "\u{201D}", None)]
    fn char_before_insertion_point_finds_the_mark_behind_a_close_quote_or_the_bare_end(
        #[case] text: &str,
        #[case] close_quote: &str,
        #[case] expected: Option<char>,
    ) {
        assert_eq!(
            char_before_insertion_point::<PlainText>(text, close_quote),
            expected
        );
    }

    #[rstest]
    #[case('.', "period")]
    #[case(',', "comma")]
    fn join_with_quote_movement_moves_group_delimiter_led_mark_inside_closing_quote(
        #[case] mark: char,
        #[case] label: &str,
    ) {
        // A `group:` join whose own `delimiter:` field carries the mark (e.g.
        // the chicago `interview` variant's `delimiter: ". "`) — the shape
        // `TemplateGroup::values` previously joined with a bare `fmt.join`,
        // with no punctuation dynamics at all.
        let parts = vec![part("“Title”"), part("2023")];
        let delimiter = RealizedPunctuation::new(format!("{mark} ").into());

        let joined = join_with_quote_movement::<PlainText>(parts, &delimiter, true, "”");

        assert_eq!(
            joined,
            format!("“Title{mark}” 2023"),
            "{label}-led group delimiter should move inside the quote"
        );
    }

    #[rstest]
    #[case('.', "period")]
    #[case(',', "comma")]
    fn join_with_quote_movement_moves_next_item_own_leading_mark_inside_closing_quote_when_delimiter_is_empty(
        #[case] mark: char,
        #[case] label: &str,
    ) {
        // A `group:` with `delimiter: ''` where each item is self-delimiting
        // via its own `prefix` (e.g. chicago's
        // `- title: primary ... - variable: locator prefix: ", "`).
        let parts = vec![part("“Title”"), part(&format!("{mark} 1"))];
        let delimiter = RealizedPunctuation::new("".into());

        let joined = join_with_quote_movement::<PlainText>(parts, &delimiter, true, "”");

        assert_eq!(
            joined,
            format!("“Title{mark}” 1"),
            "{label}-led self-delimiting item should move inside the quote"
        );
    }

    #[test]
    fn join_with_quote_movement_leaves_group_delimiter_led_mark_outside_quote_when_disabled() {
        let parts = vec![part("“Title”"), part("2023")];
        let delimiter = RealizedPunctuation::new(". ".into());

        let joined = join_with_quote_movement::<PlainText>(parts, &delimiter, false, "”");

        assert_eq!(joined, "“Title”. 2023");
    }

    #[test]
    fn join_with_quote_movement_moves_mark_inside_a_locale_specific_close_quote() {
        let parts = vec![part("«Titre»"), part("2023")];
        let delimiter = RealizedPunctuation::new(", ".into());

        let joined = join_with_quote_movement::<PlainText>(parts, &delimiter, true, "»");

        assert_eq!(joined, "«Titre,» 2023");
    }

    #[test]
    fn move_punctuation_into_quote_finds_the_close_quote_behind_trailing_html_markup() {
        // The close quote is not the raw string's last character once a
        // semantic wrapper (`</span>`) trails it — `move_punctuation_into_quote`
        // must locate it via the visible projection rather than a raw `ends_with`.
        let mut accumulated = r#"<span class="citum-title">“Title”</span>"#.to_string();
        let moved = move_punctuation_into_quote::<Html>(&mut accumulated, '.', "”");

        assert!(moved, "expected the mark to be moved: {accumulated}");
        assert_eq!(accumulated, r#"<span class="citum-title">“Title.”</span>"#);
    }

    #[test]
    fn move_punctuation_into_quote_finds_the_close_quote_behind_trailing_latex_markup() {
        // Same as the HTML case, for a LaTeX command's closing brace.
        let mut accumulated = r"\emph{“Title”}".to_string();
        let moved = move_punctuation_into_quote::<Latex>(&mut accumulated, '.', "”");

        assert!(moved, "expected the mark to be moved: {accumulated}");
        assert_eq!(accumulated, "\\emph{“Title.”}");
    }

    #[test]
    fn move_punctuation_into_quote_returns_false_when_no_close_quote_is_present() {
        let mut accumulated = r#"<span class="citum-title">Title</span>"#.to_string();
        let moved = move_punctuation_into_quote::<Html>(&mut accumulated, '.', "”");

        assert!(!moved);
        assert_eq!(accumulated, r#"<span class="citum-title">Title</span>"#);
    }

    #[rstest]
    #[case::plain(". Aired September 28", " Aired September 28")]
    #[case::html(
        r#"<span class="citum-issued">. Aired September 28</span>"#,
        r#"<span class="citum-issued"> Aired September 28</span>"#
    )]
    fn leading_movable_mark_strips_the_mark_behind_leading_markup(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        // The style-supplied leading mark sits behind a semantic wrapper's
        // opening tag once `apply_component_semantics` wraps the affixed
        // output — the raw string's first byte is markup, not the mark.
        let found = if input.starts_with('<') {
            leading_movable_mark::<Html>(input)
        } else {
            leading_movable_mark::<PlainText>(input)
        };

        assert_eq!(found, Some(('.', expected.to_string())));
    }

    #[test]
    fn leading_movable_mark_returns_none_when_first_visible_char_is_not_a_mark() {
        let found = leading_movable_mark::<PlainText>("Aired September 28");

        assert_eq!(found, None);
    }
}
