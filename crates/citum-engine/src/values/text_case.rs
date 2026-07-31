/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Title text-case transforms.
//!
//! Implements structured-title-aware casing for bibliography output.
//! All transforms operate on Djot-markup-bearing strings and respect
//! `.nocase` span protection via the rich-text renderer.

use citum_schema::NoteStartTextCase;
use citum_schema::options::titles::TextCase;
use icu_casemap::CaseMapper;
use icu_locale::LanguageIdentifier;

/// Apply a text-case transform to a single plain-text segment.
///
/// This function handles the core casing logic for a single string.
/// For structured titles with subtitles, use [`apply_to_structured_parts`].
///
/// `.nocase`-protected spans are handled at the Djot rendering layer,
/// not here — this function operates on already-resolved text segments.
#[must_use]
pub fn apply_text_case(text: &str, case: TextCase) -> String {
    apply_text_case_with_language(text, case, None)
}

/// Apply a text-case transform using locale-tailored Unicode case mappings.
///
/// `language` accepts a BCP 47 language tag. Missing or malformed tags use
/// the Unicode root locale, preserving deterministic default behavior.
#[must_use]
pub fn apply_text_case_with_language(text: &str, case: TextCase, language: Option<&str>) -> String {
    let language = language_identifier_for_tag(language);
    apply_text_case_with_language_id(text, case, &language)
}

pub(crate) fn apply_text_case_with_language_id(
    text: &str,
    case: TextCase,
    language: &LanguageIdentifier,
) -> String {
    match case {
        TextCase::AsIs => text.to_string(),
        TextCase::Lowercase => lowercase(text, language),
        TextCase::Uppercase => uppercase(text, language),
        TextCase::CapitalizeFirst => capitalize_first_word_with_language_id(text, language),
        TextCase::Sentence | TextCase::SentenceApa | TextCase::SentenceNlm => {
            to_sentence_case_with_language_id(text, language)
        }
        TextCase::Title => to_title_case_with_language_id(text, language),
    }
}

pub(crate) fn language_identifier_for_tag(language: Option<&str>) -> LanguageIdentifier {
    super::parse_language_identifier(language).unwrap_or_else(root_language_identifier)
}

fn root_language_identifier() -> LanguageIdentifier {
    icu_locale::langid!("und")
}

fn lowercase(text: &str, language: &LanguageIdentifier) -> String {
    CaseMapper::new()
        .lowercase_to_string(text, language)
        .into_owned()
}

fn uppercase(text: &str, language: &LanguageIdentifier) -> String {
    CaseMapper::new()
        .uppercase_to_string(text, language)
        .into_owned()
}

/// Apply text-case to a structured title (main + subtitles).
///
/// The key difference between sentence-case variants:
/// - `SentenceApa`: capitalize first word of main title AND each subtitle
/// - `SentenceNlm`: capitalize first word of main title only
/// - Other variants: applied uniformly to each part
#[must_use]
pub fn apply_to_structured_parts(
    main: &str,
    subtitles: &[&str],
    case: TextCase,
) -> (String, Vec<String>) {
    apply_to_structured_parts_with_language(main, subtitles, case, None)
}

/// Apply locale-tailored text casing to a structured title (main + subtitles).
#[must_use]
pub fn apply_to_structured_parts_with_language(
    main: &str,
    subtitles: &[&str],
    case: TextCase,
    language: Option<&str>,
) -> (String, Vec<String>) {
    let language = language_identifier_for_tag(language);
    match case {
        TextCase::SentenceApa => {
            let main_cased = to_sentence_case_with_language_id(main, &language);
            let subs_cased = subtitles
                .iter()
                .map(|s| to_sentence_case_with_language_id(s, &language))
                .collect();
            (main_cased, subs_cased)
        }
        TextCase::SentenceNlm => {
            let main_cased = to_sentence_case_with_language_id(main, &language);
            // NLM: subtitles keep only explicit/protected capitals (lowercase the rest)
            let subs_cased = subtitles.iter().map(|s| lowercase(s, &language)).collect();
            (main_cased, subs_cased)
        }
        _ => {
            let main_cased = apply_text_case_with_language_id(main, case, &language);
            let subs_cased = subtitles
                .iter()
                .map(|s| apply_text_case_with_language_id(s, case, &language))
                .collect();
            (main_cased, subs_cased)
        }
    }
}

/// Returns true if the given language tag indicates English.
#[must_use]
pub fn is_english_language(lang: Option<&str>) -> bool {
    match lang {
        Some(tag) => {
            let primary = tag.split('-').next().unwrap_or(tag);
            primary.eq_ignore_ascii_case("en")
        }
        // Default: assume English for backward compatibility
        None => true,
    }
}

/// Resolve the effective text-case, applying language fallback.
///
/// For non-English languages without defined transforms, returns `AsIs`.
#[must_use]
pub fn resolve_text_case(case: TextCase, language: Option<&str>) -> TextCase {
    if is_english_language(language) {
        case
    } else {
        // Non-English: only explicit as-is, lowercase, uppercase pass through.
        // All English-specific transforms fall back to as-is.
        match case {
            TextCase::AsIs | TextCase::Lowercase | TextCase::Uppercase => case,
            _ => TextCase::AsIs,
        }
    }
}

/// Apply a note-start text-case transform using the same language fallback rules
/// as other locale-backed casing behavior.
#[must_use]
pub(crate) fn apply_note_start_text_case(
    value: &str,
    text_case: NoteStartTextCase,
    language: Option<&str>,
) -> String {
    let case = match text_case {
        NoteStartTextCase::CapitalizeFirst => TextCase::CapitalizeFirst,
        NoteStartTextCase::Lowercase => TextCase::Lowercase,
    };
    apply_text_case_with_language(value, resolve_text_case(case, language), language)
}

/// Returns true if any character in `word` other than the first is uppercase.
///
/// Per CSL 1.0 / citeproc-js, a word carrying internal capitalization (an
/// all-caps acronym like "DNA", or a mixed-case brand/name like "McDonald" or
/// "iPhone") is presumed deliberately cased and is left untouched by the
/// sentence- and title-case transforms; only words whose casing is limited to
/// (at most) a leading capital are transformed. Uses `char::is_uppercase` so
/// non-ASCII scripts with case are handled correctly.
fn has_internal_uppercase(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next();
    chars.any(char::is_uppercase)
}

/// Rebuild `text` with each whitespace-delimited word replaced by the
/// corresponding entry in `parts`, preserving the original whitespace runs
/// (leading/trailing/internal spacing) exactly as written.
fn rebuild_with_original_whitespace(text: &str, parts: &[String]) -> String {
    let mut result = String::with_capacity(text.len());
    let mut word_iter = parts.iter();
    let mut in_word = false;
    let mut current_word = word_iter.next();

    for ch in text.chars() {
        if ch.is_whitespace() {
            if in_word {
                in_word = false;
                current_word = word_iter.next();
            }
            result.push(ch);
        } else if !in_word && let Some(word) = current_word {
            result.push_str(word);
            in_word = true;
        }
    }

    result
}

/// Convert text to sentence case: lowercase every word and capitalize the
/// first, except words carrying internal capitalization (acronyms, mixed-case
/// names), which are preserved exactly as written — including the first word,
/// which is left unmodified rather than force-capitalized.
#[cfg(test)]
fn to_sentence_case(text: &str) -> String {
    let language = root_language_identifier();
    to_sentence_case_with_language_id(text, &language)
}

fn to_sentence_case_with_language_id(text: &str, language: &LanguageIdentifier) -> String {
    if text.is_empty() {
        return String::new();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut parts: Vec<String> = Vec::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        if has_internal_uppercase(word) {
            parts.push((*word).to_string());
        } else if i == 0 {
            parts.push(capitalize_first_word_with_language_id(
                &lowercase(word, language),
                language,
            ));
        } else {
            parts.push(lowercase(word, language));
        }
    }

    rebuild_with_original_whitespace(text, &parts)
}

/// Capitalize the first alphabetic character of the string,
/// preserving leading whitespace and punctuation.
///
/// A leading digit blocks capitalization entirely rather than being skipped
/// over: a string like `"35 mm film"` has no leading word to capitalize (the
/// first token is a numeral, not a word), so it is left as-is instead of
/// capitalizing the first letter found later in the string (which would
/// wrongly produce `"35 Mm film"`).
#[cfg(test)]
pub(crate) fn capitalize_first_word(text: &str) -> String {
    let language = root_language_identifier();
    capitalize_first_word_with_language_id(text, &language)
}

fn capitalize_first_word_with_language_id(text: &str, language: &LanguageIdentifier) -> String {
    let mut result = String::with_capacity(text.len());
    let mut found_first = false;
    let mut blocked_by_digit = false;
    for ch in text.chars() {
        if !found_first && !blocked_by_digit && ch.is_alphabetic() {
            result.push_str(&uppercase(&ch.to_string(), language));
            found_first = true;
        } else {
            if !found_first && ch.is_ascii_digit() {
                blocked_by_digit = true;
            }
            result.push(ch);
        }
    }
    result
}

/// Capitalize the first alphabetic character of the string, skipping over
/// HTML tags, LaTeX command prefixes, and Typst command prefixes.
///
/// Use this variant when the input may already contain rendered markup from a
/// pre-formatted component. For plain-text input, behaviour is identical to
/// [`capitalize_first_word`].
#[cfg(test)]
pub(crate) fn capitalize_first_word_markup_aware(text: &str) -> String {
    capitalize_first_word_markup_aware_with_language(text, None)
}

pub(crate) fn capitalize_first_word_markup_aware_with_language(
    text: &str,
    language: Option<&str>,
) -> String {
    let language = language_identifier_for_tag(language);
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let Some(&b) = bytes.get(i) else { break };

        // Skip HTML tag: <...>
        // `i` always points to an ASCII byte here, so the slice is on a char boundary.
        if b == b'<'
            && let Some(end) = text.get(i..).and_then(|s| s.find('>'))
        {
            i += end + 1;
            continue;
        }

        // Skip LaTeX command prefix: \letters{ or \letters[...]{
        if b == b'\\' {
            let cmd_start = i + 1;
            let cmd_len = bytes
                .get(cmd_start..)
                .unwrap_or_default()
                .iter()
                .take_while(|&&c| c.is_ascii_alphabetic())
                .count();
            if cmd_len > 0 {
                let after_cmd = cmd_start + cmd_len;
                // Skip optional [...]
                let after_opt = if bytes.get(after_cmd) == Some(&b'[') {
                    text.get(after_cmd..)
                        .and_then(|s| s.find(']'))
                        .map(|e| after_cmd + e + 1)
                        .unwrap_or(after_cmd)
                } else {
                    after_cmd
                };
                if bytes.get(after_opt) == Some(&b'{') {
                    i = after_opt + 1;
                    continue;
                }
            }
        }

        // Skip Typst command prefix: #letters[
        if b == b'#' {
            let cmd_start = i + 1;
            let cmd_len = bytes
                .get(cmd_start..)
                .unwrap_or_default()
                .iter()
                .take_while(|&&c| c.is_ascii_alphabetic())
                .count();
            if cmd_len > 0 {
                let after_cmd = cmd_start + cmd_len;
                if bytes.get(after_cmd) == Some(&b'[') {
                    i = after_cmd + 1;
                    continue;
                }
            }
        }

        // Decode the next Unicode character. `i` is always on a char boundary:
        // the markup-skip branches only advance past ASCII bytes.
        let ch = text.get(i..).and_then(|s| s.chars().next()).unwrap_or('\0');
        if ch.is_alphabetic() {
            let ch_len = ch.len_utf8();
            let mut result = String::with_capacity(text.len());
            result.push_str(text.get(..i).unwrap_or_default());
            result.push_str(&uppercase(&ch.to_string(), &language));
            result.push_str(text.get(i + ch_len..).unwrap_or_default());
            return result;
        }

        i += ch.len_utf8().max(1);
    }

    text.to_string()
}

/// Apply a text-case transform to a pre-formatted string that may contain
/// rendered markup.
///
/// Delegates to locale-aware markup capitalization for `CapitalizeFirst` and
/// applies the requested locale-aware transform for all other cases.
pub(crate) fn apply_text_case_markup_aware_with_language(
    text: &str,
    case: TextCase,
    language: Option<&str>,
) -> String {
    match case {
        TextCase::CapitalizeFirst => {
            capitalize_first_word_markup_aware_with_language(text, language)
        }
        _ => apply_text_case_with_language(text, case, language),
    }
}

// English title-case stop words (articles, short conjunctions, short prepositions).
const TITLE_CASE_STOP_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "for", "from", "in", "nor", "of", "on", "or", "so",
    "the", "to", "up", "yet", "along", "between", "during", "with", "v", "vs",
];

/// Hyphen-like characters that join compound words for title-case purposes.
///
/// Includes the ASCII hyphen-minus and the en dash: bibliographic titles
/// commonly use an en dash as a hyphen substitute in compounds like
/// "Aging–Disability Nexus" (as-typed source data), and CMOS title-cases
/// each component the same way it would for an ASCII-hyphenated compound.
const HYPHEN_LIKE_CHARS: [char; 2] = ['-', '\u{2013}'];

fn contains_hyphen_like(text: &str) -> bool {
    text.contains(HYPHEN_LIKE_CHARS)
}

/// Capitalize non-stop-word components of a hyphen-joined compound for title case.
///
/// Splits on both the ASCII hyphen and the en dash (see [`HYPHEN_LIKE_CHARS`]),
/// preserving whichever separator character was actually used.
fn capitalize_hyphenated(word: &str, language: &LanguageIdentifier) -> String {
    let mut result = String::with_capacity(word.len());
    let mut last_end = 0;
    let part_count = word.matches(HYPHEN_LIKE_CHARS).count() + 1;
    for (part_index, (idx, sep)) in word.match_indices(HYPHEN_LIKE_CHARS).enumerate() {
        let part = word.get(last_end..idx).unwrap_or_default();
        result.push_str(&capitalize_hyphen_part(
            part,
            part_index == 0 || part_index + 1 == part_count,
            language,
        ));
        result.push_str(sep);
        last_end = idx + sep.len();
    }
    let tail = word.get(last_end..).unwrap_or_default();
    result.push_str(&capitalize_hyphen_part(tail, false, language));
    result
}

fn capitalize_hyphen_part(part: &str, force_all: bool, language: &LanguageIdentifier) -> String {
    if force_all {
        capitalize_first_word_with_language_id(part, language)
    } else {
        let alpha_core = part.trim_matches(|c: char| !c.is_alphanumeric());
        if TITLE_CASE_STOP_WORDS.contains(&alpha_core) {
            part.to_string()
        } else {
            capitalize_first_word_with_language_id(part, language)
        }
    }
}

fn trim_trailing_closing_punctuation(word: &str) -> &str {
    word.trim_end_matches(['"', '\'', ')', ']', '}', '»', '”', '’'])
}

/// Convert text to English headline-style title case.
///
/// Capitalizes the first and last word unconditionally.
/// Interior stop words (articles, short prepositions, conjunctions) stay lowercase.
/// The first word after `:`, `?`, or `!` is always capitalized.
/// Hyphenated compounds capitalize each non-stop-word component.
#[cfg(test)]
fn to_title_case(text: &str) -> String {
    let language = root_language_identifier();
    to_title_case_with_language_id(text, &language)
}

fn to_title_case_with_language_id(text: &str, language: &LanguageIdentifier) -> String {
    if text.is_empty() {
        return String::new();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let last_idx = words.len() - 1;
    let mut parts: Vec<String> = Vec::with_capacity(words.len());
    let mut capitalize_next = false;

    for (i, word) in words.iter().enumerate() {
        if has_internal_uppercase(word) {
            // Acronym or mixed-case name (e.g. "DNA", "McDonald", "iPhone"):
            // preserve exactly as written, regardless of position.
            parts.push((*word).to_string());
        } else {
            let lower = lowercase(word, language);
            if i == 0 || i == last_idx || capitalize_next {
                if contains_hyphen_like(&lower) {
                    parts.push(capitalize_hyphenated(&lower, language));
                } else {
                    parts.push(capitalize_first_word_with_language_id(&lower, language));
                }
            } else {
                // Strip leading/trailing punctuation when checking stop words so that
                // words like "(and" or "and)" are still treated as the stop word "and".
                let alpha_core = lower.trim_matches(|c: char| !c.is_alphanumeric());
                if TITLE_CASE_STOP_WORDS.contains(&alpha_core) {
                    parts.push(lower);
                } else if contains_hyphen_like(&lower) {
                    parts.push(capitalize_hyphenated(&lower, language));
                } else {
                    parts.push(capitalize_first_word_with_language_id(&lower, language));
                }
            }
        }
        // Capitalize the next word after sentence-ending punctuation or a colon,
        // even when that punctuation is followed by a closing quote or bracket.
        let punctuation_core = trim_trailing_closing_punctuation(word);
        capitalize_next = punctuation_core.ends_with(':')
            || punctuation_core.ends_with('?')
            || punctuation_core.ends_with('!');
    }

    rebuild_with_original_whitespace(text, &parts)
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

    // --- capitalize_first_word ---

    #[test]
    fn test_capitalize_first_word_basic() {
        assert_eq!(capitalize_first_word("hello world"), "Hello world");
    }

    #[test]
    fn test_capitalize_first_word_leading_space() {
        assert_eq!(capitalize_first_word("  hello"), "  Hello");
    }

    #[test]
    fn test_capitalize_first_word_empty() {
        assert_eq!(capitalize_first_word(""), "");
    }

    #[test]
    fn test_capitalize_first_word_already_upper() {
        assert_eq!(capitalize_first_word("Hello"), "Hello");
    }

    #[test]
    fn given_leading_numeral_when_capitalize_first_word_then_left_as_is() {
        // "35 mm film" has no leading word to capitalize; the first letter
        // ("m" in "mm") must not be hunted down and capitalized instead.
        assert_eq!(capitalize_first_word("35 mm film"), "35 mm film");
    }

    #[test]
    fn given_leading_letter_when_capitalize_first_word_then_capitalized() {
        assert_eq!(capitalize_first_word("in Korean"), "In Korean");
    }

    #[test]
    fn turkish_case_mapping_handles_dotted_and_dotless_i() {
        assert_eq!(
            apply_text_case_with_language("istanbul ızmir", TextCase::Uppercase, Some("tr-TR"),),
            "İSTANBUL IZMİR"
        );
        assert_eq!(
            apply_text_case_with_language("İSTANBUL IĞDIR", TextCase::Lowercase, Some("tr-TR"),),
            "istanbul ığdır"
        );
        assert_eq!(
            apply_text_case_with_language("istanbul", TextCase::CapitalizeFirst, Some("tr")),
            "İstanbul"
        );
        assert_eq!(
            apply_text_case_with_language("istanbul ızmir", TextCase::Uppercase, Some("tr_TR")),
            "İSTANBUL IZMİR"
        );
    }

    #[test]
    fn azerbaijani_case_mapping_uses_turkic_rules() {
        assert_eq!(
            apply_text_case_with_language("izmir ı", TextCase::Uppercase, Some("az-Latn-AZ")),
            "İZMİR I"
        );
    }

    #[test]
    fn sentence_case_uses_the_supplied_language() {
        assert_eq!(
            apply_text_case_with_language("istanbul Izmir", TextCase::Sentence, Some("tr-TR"),),
            "İstanbul ızmir"
        );
    }

    #[test]
    fn structured_subtitles_use_the_supplied_language() {
        let (main, subtitles) = apply_to_structured_parts_with_language(
            "istanbul tarihi",
            &["izmir incelemesi"],
            TextCase::SentenceApa,
            Some("tr-TR"),
        );

        assert_eq!(main, "İstanbul tarihi");
        assert_eq!(subtitles, ["İzmir incelemesi"]);
    }

    #[test]
    fn malformed_language_tag_uses_root_case_mapping() {
        assert_eq!(
            apply_text_case_with_language("istanbul", TextCase::Uppercase, Some("not_a_language"),),
            "ISTANBUL"
        );
    }

    #[test]
    fn locale_override_suffix_preserves_base_language_mapping() {
        assert_eq!(
            apply_text_case_with_language(
                "istanbul",
                TextCase::Uppercase,
                Some("tr-TR-citum_override"),
            ),
            "İSTANBUL"
        );
    }

    // --- capitalize_first_word_markup_aware ---

    #[test]
    fn test_capitalize_markup_aware_plain_text() {
        assert_eq!(
            capitalize_first_word_markup_aware("the collected essays"),
            "The collected essays"
        );
    }

    #[test]
    fn test_capitalize_markup_aware_html_tag() {
        assert_eq!(
            capitalize_first_word_markup_aware("<em>the collected essays</em>"),
            "<em>The collected essays</em>"
        );
    }

    #[test]
    fn capitalize_markup_aware_uses_the_supplied_language() {
        assert_eq!(
            apply_text_case_markup_aware_with_language(
                "<em>istanbul</em>",
                TextCase::CapitalizeFirst,
                Some("tr-TR"),
            ),
            "<em>İstanbul</em>"
        );
    }

    #[test]
    fn test_capitalize_markup_aware_html_nested_tags() {
        assert_eq!(
            capitalize_first_word_markup_aware(r#"<span class="x"><em>the title</em></span>"#),
            r#"<span class="x"><em>The title</em></span>"#
        );
    }

    #[test]
    fn test_capitalize_markup_aware_latex_command() {
        assert_eq!(
            capitalize_first_word_markup_aware(r"\emph{the collected essays}"),
            r"\emph{The collected essays}"
        );
    }

    #[test]
    fn test_capitalize_markup_aware_latex_number_not_corrupted() {
        // Regression: \emph{521} must not become \Emph{521}
        assert_eq!(
            capitalize_first_word_markup_aware(r"\emph{521}"),
            r"\emph{521}"
        );
    }

    #[test]
    fn test_capitalize_markup_aware_typst_command() {
        assert_eq!(
            capitalize_first_word_markup_aware("#emph[the collected essays]"),
            "#emph[The collected essays]"
        );
    }

    #[test]
    fn test_capitalize_markup_aware_plain_underscore_delimiters() {
        // PlainText emph uses _..._; _ is non-alphabetic so this was already safe
        assert_eq!(
            capitalize_first_word_markup_aware("_the collected essays_"),
            "_The collected essays_"
        );
    }

    #[test]
    fn test_capitalize_markup_aware_empty_string() {
        assert_eq!(capitalize_first_word_markup_aware(""), "");
    }

    #[test]
    fn test_capitalize_markup_aware_all_markup_no_text() {
        assert_eq!(capitalize_first_word_markup_aware("<em></em>"), "<em></em>");
    }

    // --- to_sentence_case ---

    #[test]
    fn test_sentence_case_basic() {
        assert_eq!(
            to_sentence_case("The Quick Brown Fox"),
            "The quick brown fox"
        );
    }

    #[test]
    fn test_sentence_case_all_caps() {
        // All-caps words are presumed deliberately cased (acronyms) and are
        // preserved verbatim, including as the first word.
        assert_eq!(to_sentence_case("DNA REPLICATION"), "DNA REPLICATION");
    }

    #[test]
    fn test_sentence_case_empty() {
        assert_eq!(to_sentence_case(""), "");
    }

    #[test]
    fn given_mixed_case_word_when_sentence_case_then_preserved() {
        assert_eq!(
            to_sentence_case("An Introduction to DNA"),
            "An introduction to DNA"
        );
    }

    #[test]
    fn given_lowercase_iphone_when_sentence_case_then_lowercased() {
        assert_eq!(to_sentence_case("the iphone problem"), "The iphone problem");
    }

    #[test]
    fn given_mixed_case_iphone_when_sentence_case_then_preserved() {
        assert_eq!(to_sentence_case("the iPhone problem"), "The iPhone problem");
    }

    #[test]
    fn given_mixed_case_surname_when_sentence_case_then_preserved() {
        assert_eq!(
            to_sentence_case("a study of McDonald"),
            "A study of McDonald"
        );
    }

    // --- to_title_case ---

    #[test]
    fn test_title_case_basic() {
        assert_eq!(to_title_case("the quick brown fox"), "The Quick Brown Fox");
    }

    #[test]
    fn test_title_case_stop_words() {
        assert_eq!(
            to_title_case("a tale of two cities"),
            "A Tale of Two Cities"
        );
    }

    #[test]
    fn test_title_case_last_word_capitalized() {
        assert_eq!(
            to_title_case("the world we live in"),
            "The World We Live In"
        );
    }

    #[test]
    fn test_title_case_after_colon() {
        assert_eq!(
            to_title_case("the title: a subtitle"),
            "The Title: A Subtitle"
        );
    }

    #[test]
    fn test_title_case_after_colon_stop_word() {
        // First word after colon is a stop word but must still be capitalized
        assert_eq!(
            to_title_case("history of the world: a new perspective"),
            "History of the World: A New Perspective"
        );
    }

    #[test]
    fn test_title_case_after_question_mark() {
        assert_eq!(
            to_title_case("who's black and why? a hidden chapter"),
            "Who's Black and Why? A Hidden Chapter"
        );
    }

    #[test]
    fn test_title_case_after_question_mark_with_closing_quote() {
        assert_eq!(
            to_title_case("who's black and why?\" a hidden chapter"),
            "Who's Black and Why?\" A Hidden Chapter"
        );
    }

    #[test]
    fn test_title_case_from_is_stop_word() {
        assert_eq!(
            to_title_case("a hidden chapter from the eighteenth-century invention of race"),
            "A Hidden Chapter from the Eighteenth-Century Invention of Race"
        );
    }

    #[test]
    fn test_title_case_hyphenated_compound() {
        assert_eq!(
            to_title_case("eighteenth-century studies"),
            "Eighteenth-Century Studies"
        );
    }

    #[test]
    fn test_title_case_hyphenated_stop_word_part() {
        // "well-to-do": "to" is a stop word → stays lowercase in interior position
        assert_eq!(to_title_case("a well-to-do family"), "A Well-to-Do Family");
    }

    #[test]
    fn given_hyphenated_title_with_stop_word_when_title_case_then_interior_stop_word_stays_lowercase()
     {
        assert_eq!(
            to_title_case("text-to-speech systems"),
            "Text-to-Speech Systems"
        );
    }

    #[test]
    fn given_long_title_prepositions_when_title_case_then_interior_stop_words_stay_lowercase() {
        assert_eq!(
            to_title_case(
                "fighting for forests: protection and exploitation during the war with China"
            ),
            "Fighting for Forests: Protection and Exploitation during the War with China"
        );
    }

    #[test]
    fn given_en_dash_compound_when_title_case_then_both_sides_capitalized() {
        // Source data sometimes uses an en dash ("–") as a hyphen substitute
        // in a compound like "Aging–Disability"; CMOS title-cases each side
        // the same way it would for an ASCII-hyphenated compound.
        assert_eq!(
            to_title_case("the aging\u{2013}disability nexus"),
            "The Aging\u{2013}Disability Nexus"
        );
    }

    // --- apply_to_structured_parts ---

    #[test]
    fn test_sentence_apa_structured() {
        let (main, subs) = apply_to_structured_parts(
            "Understanding Citation Systems",
            &["History and Practice", "A Comparative View"],
            TextCase::SentenceApa,
        );
        assert_eq!(main, "Understanding citation systems");
        assert_eq!(subs, vec!["History and practice", "A comparative view"]);
    }

    #[test]
    fn test_sentence_nlm_structured() {
        let (main, subs) = apply_to_structured_parts(
            "Understanding Citation Systems",
            &["History and Practice"],
            TextCase::SentenceNlm,
        );
        assert_eq!(main, "Understanding citation systems");
        // NLM: subtitles lowercased (no first-word capitalization)
        assert_eq!(subs, vec!["history and practice"]);
    }

    #[test]
    fn test_title_case_structured() {
        // "DNA" is already mixed-case in the source data and must be
        // preserved verbatim by the title-case transform.
        let (main, subs) =
            apply_to_structured_parts("the DNA of empire", &["a new perspective"], TextCase::Title);
        assert_eq!(main, "The DNA of Empire");
        assert_eq!(subs, vec!["A New Perspective"]);
    }

    #[test]
    fn given_mixed_case_surname_when_title_case_then_preserved() {
        assert_eq!(to_title_case("a study of McDonald"), "A Study of McDonald");
    }

    // --- resolve_text_case ---

    #[test]
    fn test_english_language_detection() {
        assert!(is_english_language(Some("en")));
        assert!(is_english_language(Some("en-US")));
        assert!(is_english_language(Some("en-GB")));
        assert!(is_english_language(None));
        assert!(!is_english_language(Some("de")));
        assert!(!is_english_language(Some("fr-FR")));
    }

    #[test]
    fn test_resolve_non_english_falls_back() {
        assert_eq!(
            resolve_text_case(TextCase::SentenceApa, Some("de")),
            TextCase::AsIs
        );
        assert_eq!(
            resolve_text_case(TextCase::Title, Some("fr")),
            TextCase::AsIs
        );
        // Explicit lowercase/uppercase pass through for any language
        assert_eq!(
            resolve_text_case(TextCase::Lowercase, Some("de")),
            TextCase::Lowercase
        );
    }

    #[test]
    fn test_resolve_english_passes_through() {
        assert_eq!(
            resolve_text_case(TextCase::SentenceApa, Some("en")),
            TextCase::SentenceApa
        );
        assert_eq!(
            resolve_text_case(TextCase::Title, Some("en-US")),
            TextCase::Title
        );
    }

    #[test]
    fn test_note_start_capitalize_first_uses_english_language_rules() {
        assert_eq!(
            apply_note_start_text_case(
                "edited by",
                NoteStartTextCase::CapitalizeFirst,
                Some("en-US"),
            ),
            "Edited by"
        );
    }

    #[test]
    fn test_note_start_capitalize_first_falls_back_to_as_is_for_non_english() {
        assert_eq!(
            apply_note_start_text_case(
                "hg. von",
                NoteStartTextCase::CapitalizeFirst,
                Some("de-DE"),
            ),
            "hg. von"
        );
    }

    #[test]
    fn test_note_start_capitalize_first_is_no_op_for_uncased_scripts() {
        assert_eq!(
            apply_note_start_text_case("ابن سينا", NoteStartTextCase::CapitalizeFirst, Some("ar"),),
            "ابن سينا"
        );
    }
}
