/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Shared punctuation classification and collision resolution.

use crate::render::format::RealizedPunctuation;
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

/// Move a trailing `punct` (`.` or `,`) inside a preceding closing quotation
/// mark, in place. `close_quote` is the locale-resolved closing glyph (e.g.
/// `\u{201D}` for en-US, `\u{00BB}` for fr-FR); a bare `"` is also accepted as
/// a legacy fallback for literal-authored ASCII quotes. Returns `false`
/// (leaving `accumulated` untouched) when neither glyph is found at the end.
pub(crate) fn move_punctuation_into_quote(
    accumulated: &mut String,
    punct: char,
    close_quote: &str,
) -> bool {
    if !close_quote.is_empty() && accumulated.ends_with(close_quote) {
        let split = accumulated.len() - close_quote.len();
        accumulated.insert(split, punct);
        return true;
    }
    if close_quote != "\"" && accumulated.ends_with('"') {
        accumulated.pop();
        accumulated.push(punct);
        accumulated.push('"');
        return true;
    }
    false
}

/// Join `parts` with `delimiter`, applying [`move_punctuation_into_quote`] at each boundary
/// when `punctuation_in_quote` is active. The leading period or comma to move may come from
/// `delimiter` itself, or — when `delimiter` is empty — from the next part's own leading
/// character (e.g. a component's self-supplied `prefix`, the shape `group:` delimiters rely on
/// for self-delimiting items). Used for `group:` template joins, which otherwise have no
/// punctuation dynamics at all.
///
/// `delimiter` must already be decomposed from the same (post-escape) string
/// that will be spliced into the output — see [`RealizedPunctuation`].
#[allow(
    clippy::string_slice,
    reason = "UTF-8 safe slicing based on char boundary checks"
)]
pub(crate) fn join_with_quote_movement(
    parts: Vec<String>,
    delimiter: &RealizedPunctuation<'_>,
    punctuation_in_quote: bool,
    close_quote: &str,
) -> String {
    let mut iter = parts.into_iter();
    let Some(mut result) = iter.next() else {
        return String::new();
    };

    for part in iter {
        let delim_first = delimiter.core();

        let moved_via_delimiter = punctuation_in_quote
            && matches!(delim_first, Some('.' | ','))
            && move_punctuation_into_quote(&mut result, delim_first.unwrap_or('.'), close_quote);

        if moved_via_delimiter {
            result.push_str(delimiter.tail());
            result.push_str(&part);
            continue;
        }

        if punctuation_in_quote
            && delim_first.is_none()
            && let Some(part_first) = part.chars().next()
            && matches!(part_first, '.' | ',')
            && move_punctuation_into_quote(&mut result, part_first, close_quote)
        {
            result.push_str(&part[part_first.len_utf8()..]);
            continue;
        }

        result.push_str(delimiter.text());
        result.push_str(&part);
    }

    result
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
    use rstest::rstest;

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
        let parts = vec!["“Title”".to_string(), "2023".to_string()];
        let delimiter = RealizedPunctuation::new(format!("{mark} ").into());

        let joined = join_with_quote_movement(parts, &delimiter, true, "”");

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
        let parts = vec!["“Title”".to_string(), format!("{mark} 1")];
        let delimiter = RealizedPunctuation::new("".into());

        let joined = join_with_quote_movement(parts, &delimiter, true, "”");

        assert_eq!(
            joined,
            format!("“Title{mark}” 1"),
            "{label}-led self-delimiting item should move inside the quote"
        );
    }

    #[test]
    fn join_with_quote_movement_leaves_group_delimiter_led_mark_outside_quote_when_disabled() {
        let parts = vec!["“Title”".to_string(), "2023".to_string()];
        let delimiter = RealizedPunctuation::new(". ".into());

        let joined = join_with_quote_movement(parts, &delimiter, false, "”");

        assert_eq!(joined, "“Title”. 2023");
    }

    #[test]
    fn join_with_quote_movement_moves_mark_inside_a_locale_specific_close_quote() {
        let parts = vec!["«Titre»".to_string(), "2023".to_string()];
        let delimiter = RealizedPunctuation::new(", ".into());

        let joined = join_with_quote_movement(parts, &delimiter, true, "»");

        assert_eq!(joined, "«Titre,» 2023");
    }
}
