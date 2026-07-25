/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Conversion of citeproc-js's fixed HTML rich-text tag set to Djot.
//!
//! citeproc-js authors title case-protection and inline formatting as literal
//! HTML (`<span class="nocase">`, `<i>`, `<b>`, `<sc>`, `<sup>`, `<sub>`)
//! rather than Djot, Citum's canonical inline markup for free-text fields.
//! [`html_markup_to_djot`] converts that fixed tag set at ingestion, so it is
//! interpreted by the renderer instead of leaking verbatim into rendered
//! output. Not gated behind `legacy-convert`: both the CSL-JSON conversion
//! path (`reference::conversion`) and the plain biblatex path
//! (`citum-refs::biblatex`) need it, and neither implies the other.

/// Djot equivalent of one of the fixed HTML tags citeproc-js authors as
/// title rich-text markup.
#[derive(Clone, Copy)]
enum DjotTag {
    Emph,
    Strong,
    SmallCaps,
    Superscript,
    Subscript,
    NoCase,
    /// A `<span>` with no recognized class/style -- kept for stack bookkeeping
    /// so its matching `</span>` doesn't pop an unrelated tag's closer.
    Passthrough,
}

impl DjotTag {
    fn open(self) -> &'static str {
        match self {
            Self::Emph => "_",
            Self::Strong => "*",
            Self::SmallCaps | Self::Superscript | Self::Subscript | Self::NoCase => "[",
            Self::Passthrough => "",
        }
    }

    fn close(self) -> &'static str {
        match self {
            Self::Emph => "_",
            Self::Strong => "*",
            Self::SmallCaps => "]{.smallcaps}",
            Self::Superscript => "]{.superscript}",
            Self::Subscript => "]{.subscript}",
            Self::NoCase => "]{.nocase}",
            Self::Passthrough => "",
        }
    }
}

/// Tag names citeproc-js is known to emit for title rich-text markup.
const RECOGNIZED_HTML_TAG_NAMES: [&str; 6] = ["i", "b", "sc", "sup", "sub", "span"];

/// Classify a parsed opening-tag body (without `<`/`>`), e.g. `span
/// class="nocase"`, returning `None` for tag names outside the fixed
/// citeproc set.
fn classify_open_tag(tag_inner: &str) -> Option<DjotTag> {
    let name = tag_inner.split_whitespace().next()?.to_ascii_lowercase();
    if !RECOGNIZED_HTML_TAG_NAMES.contains(&name.as_str()) {
        return None;
    }
    Some(match name.as_str() {
        "i" => DjotTag::Emph,
        "b" => DjotTag::Strong,
        "sc" => DjotTag::SmallCaps,
        "sup" => DjotTag::Superscript,
        "sub" => DjotTag::Subscript,
        _ => {
            let attrs = tag_inner.to_ascii_lowercase();
            if attrs.contains("nocase") {
                DjotTag::NoCase
            } else if attrs.contains("small-caps") || attrs.contains("smallcaps") {
                DjotTag::SmallCaps
            } else {
                DjotTag::Passthrough
            }
        }
    })
}

/// Convert citeproc-js's fixed HTML rich-text tag set to Djot inline markup.
///
/// citeproc-js authors title case-protection and inline formatting as literal
/// HTML (`<span class="nocase">`, `<i>`, `<b>`, `<sc>`, `<sup>`, `<sub>`)
/// rather than Djot, Citum's canonical inline markup for free-text fields. Left
/// unconverted, these tags leak verbatim into rendered output instead of being
/// interpreted (`csl26-zaqk`, `csl26-6eoi`). Only this fixed tag set is
/// converted -- never a generic `<...>` strip, since titles may legitimately
/// contain literal `<`/`>` characters outside it.
#[allow(
    clippy::string_slice,
    reason = "every slice boundary here is a byte offset of an ASCII '<' or '>' delimiter \
              (from char_indices()/find('>')), which is always a valid char boundary"
)]
#[must_use]
pub fn html_markup_to_djot(text: &str) -> String {
    if !text.contains('<') {
        return text.to_string();
    }

    let mut output = String::with_capacity(text.len());
    let mut closers: Vec<&'static str> = Vec::new();
    let mut chars = text.char_indices();

    while let Some((start, ch)) = chars.next() {
        if ch != '<' {
            output.push(ch);
            continue;
        }
        let Some(rel_end) = text[start..].find('>') else {
            // Unterminated tag: keep the remainder literal.
            output.push_str(&text[start..]);
            break;
        };
        let end = start + rel_end;
        let tag_inner = &text[start + 1..end];
        let tag_char_len = text[start..=end].chars().count();
        for _ in 1..tag_char_len {
            chars.next();
        }

        if let Some(name) = tag_inner.strip_prefix('/') {
            let name = name.trim().to_ascii_lowercase();
            if RECOGNIZED_HTML_TAG_NAMES.contains(&name.as_str()) {
                match closers.pop() {
                    Some(closer) => output.push_str(closer),
                    // Stray/mismatched close tag: keep it literal rather than
                    // silently dropping content.
                    None => output.push_str(&text[start..=end]),
                }
            } else {
                output.push_str(&text[start..=end]);
            }
            continue;
        }

        match classify_open_tag(tag_inner) {
            Some(tag) => {
                output.push_str(tag.open());
                closers.push(tag.close());
            }
            None => output.push_str(&text[start..=end]),
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_markup_to_djot_leaves_plain_text_unchanged() {
        assert_eq!(
            html_markup_to_djot("plain text with no markup"),
            "plain text with no markup"
        );
    }

    #[test]
    fn html_markup_to_djot_converts_nocase_span() {
        // The exact convention citeproc-js emits and the CSL test suite's
        // own fixtures use, e.g. `textcase_TitleCaseWithFinalNocase.txt`.
        assert_eq!(
            html_markup_to_djot(r#"a <span class="nocase">Smith</span> pencil"#),
            "a [Smith]{.nocase} pencil"
        );
    }

    #[test]
    fn html_markup_to_djot_converts_emphasis_and_strong() {
        assert_eq!(html_markup_to_djot("<i>Homo Sapiens</i>"), "_Homo Sapiens_");
        assert_eq!(html_markup_to_djot("<b>Loud</b>"), "*Loud*");
    }

    #[test]
    fn html_markup_to_djot_converts_nested_emphasis_inside_nocase_span() {
        assert_eq!(
            html_markup_to_djot(r#"<span class="nocase"><i>DNA</i></span> Replication"#),
            "[_DNA_]{.nocase} Replication"
        );
    }

    #[test]
    fn html_markup_to_djot_leaves_unrecognized_tags_and_bare_angle_brackets_literal() {
        // Titles may legitimately contain `<`/`>` outside the fixed citeproc
        // tag set (e.g. a math/comparison expression) -- must not be stripped.
        assert_eq!(
            html_markup_to_djot("<em>ignored</em> a < b"),
            "<em>ignored</em> a < b"
        );
    }

    #[test]
    fn html_markup_to_djot_converts_superscript_ordinal() {
        // <sup>/<sub> in a title/abstract/genre-style free-text field, e.g. a
        // French ordinal in an edition statement. NOT applicable to CSL's
        // `number` variable -- verified against real citeproc-js that field
        // is exempt from flip-flop (see `normalize_rich_text_markup`'s doc
        // comment in `conversion/mod.rs`); this function itself is field-
        // agnostic and only converts what its caller feeds it.
        assert_eq!(html_markup_to_djot("1<sup>er</sup>"), "1[er]{.superscript}");
    }
}
