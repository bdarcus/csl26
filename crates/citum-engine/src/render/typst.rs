/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Typst output format.

use std::ops::Range;

use super::format::{OutputFormat, QuoteMarks, realize_wrap};
use super::visible_scan::{RunBuilder, skip_balanced};
use crate::values::ScriptClass;
use citum_schema::template::WrapPunctuation;

/// Typst renderer.
#[derive(Debug, Clone, Default)]
pub struct Typst;

impl Typst {
    fn escape_text(input: &str) -> String {
        let mut escaped = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '#' | '[' | ']' | '<' | '>' | '*' | '_' | '@' | '$' => {
                    escaped.push('\\');
                    escaped.push(ch);
                }
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    fn escape_string(input: &str) -> String {
        let mut escaped = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Return the length of the longest consecutive backtick run in `s`.
    fn longest_backtick_run(s: &str) -> usize {
        let mut max = 0usize;
        let mut cur = 0usize;
        for ch in s.chars() {
            if ch == '`' {
                cur += 1;
                if cur > max {
                    max = cur;
                }
            } else {
                cur = 0;
            }
        }
        max
    }
}

impl OutputFormat for Typst {
    type Output = String;

    fn text(&self, s: &str) -> Self::Output {
        Self::escape_text(s)
    }

    fn join(&self, items: Vec<Self::Output>, delimiter: &str) -> Self::Output {
        items.join(delimiter)
    }

    fn finish(&self, output: Self::Output) -> String {
        output
    }

    fn emph(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("#emph[{content}]")
    }

    fn strong(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("#strong[{content}]")
    }

    fn small_caps(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("#smallcaps[{content}]")
    }

    fn superscript(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("#super[{content}]")
    }

    fn quote(&self, content: Self::Output, marks: &QuoteMarks) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        let (open, close) = marks.for_depth(0);
        format!("{open}{content}{close}")
    }

    fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{}{}{}", self.text(prefix), content, self.text(suffix))
    }

    fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{}{}{}", self.text(prefix), content, self.text(suffix))
    }

    fn wrap_punctuation(
        &self,
        wrap: &WrapPunctuation,
        content: Self::Output,
        marks: &QuoteMarks,
        script: ScriptClass,
        realization: Option<&citum_schema::options::PunctuationRealization>,
    ) -> Self::Output {
        match realize_wrap(wrap, script, realization) {
            Some((open, close)) => {
                format!("{}{}{}", self.text(&open), content, self.text(&close))
            }
            None => self.quote(content, marks),
        }
    }

    fn semantic(&self, _class: &str, content: Self::Output) -> Self::Output {
        content
    }

    fn annotation(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("\n#block(class: \"citum-annotation\")[{}]", content)
    }

    fn citation(&self, ids: Vec<String>, content: Self::Output) -> Self::Output {
        if content.is_empty() || ids.len() != 1 {
            return content;
        }

        #[allow(clippy::unwrap_used, reason = "length checked")]
        let id = ids.first().unwrap();
        format!("#link(<{}>)[{}]", self.format_id(id), content)
    }

    fn link(&self, url: &str, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }

        if let Some(label) = url.strip_prefix('#') {
            format!("#link(<{}>)[{}]", self.format_id(label), content)
        } else {
            format!(r#"#link("{}")[{}]"#, Self::escape_string(url), content)
        }
    }

    fn format_id(&self, id: &str) -> String {
        let mut normalized = String::with_capacity(id.len() + 4);
        normalized.push_str("ref-");
        for ch in id.chars() {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                normalized.push(ch);
            } else {
                normalized.push('-');
            }
        }
        normalized
    }

    // ── Block-level body markup methods ────────────────────────────────────

    fn paragraph(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("{content}\n\n")
    }

    fn block_quote(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        let trimmed = content.trim_end();
        format!("#quote(block: true)[\n{trimmed}\n]\n\n")
    }

    fn bullet_list(&self, items: Vec<Self::Output>) -> Self::Output {
        if items.is_empty() {
            return String::new();
        }
        let body = items
            .iter()
            .map(|item| format!("- {}", item.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{body}\n\n")
    }

    fn ordered_list(&self, items: Vec<Self::Output>) -> Self::Output {
        if items.is_empty() {
            return String::new();
        }
        let body = items
            .iter()
            .map(|item| format!("+ {}", item.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{body}\n\n")
    }

    fn heading(&self, level: u8, content: Self::Output) -> Self::Output {
        let marks = "=".repeat(level.max(1) as usize);
        format!("{marks} {content}\n\n")
    }

    fn code_block(&self, lang: Option<&str>, content: Self::Output) -> Self::Output {
        let fence = "`".repeat(Self::longest_backtick_run(&content).max(2) + 1);
        let lang_tag = lang.unwrap_or("");
        format!("{fence}{lang_tag}\n{content}{fence}\n\n")
    }

    fn inline_code(&self, content: Self::Output) -> Self::Output {
        let ticks = "`".repeat(Self::longest_backtick_run(&content) + 1);
        format!("{ticks}{content}{ticks}")
    }

    fn strikeout(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("#strike[{content}]")
    }

    fn hard_break(&self) -> Self::Output {
        "\\\n".to_string()
    }

    fn bibliography(
        &self,
        entries: Vec<Self::Output>,
        _layout: &super::format::BibliographyLayout,
    ) -> Self::Output {
        self.join(entries, "\n\n")
    }

    fn entry(
        &self,
        id: &str,
        content: Self::Output,
        url: Option<&str>,
        _metadata: &super::format::ProcEntryMetadata,
    ) -> Self::Output {
        let content = if let Some(u) = url {
            self.link(u, content)
        } else {
            content
        };

        format!("{} <{}>", content, self.format_id(id))
    }

    /// Strip `#func(...)[...]` wrappers (function name, parenthesized
    /// arguments — e.g. a `#link("url")` target — and the content group's
    /// `[`/`]` delimiters), keeping the bracketed content visible. A literal
    /// `[content]` not preceded by `#func` (i.e. from
    /// [`Self::wrap_punctuation`]'s `WrapPunctuation::Brackets`) keeps its
    /// brackets visible, since those are house-style punctuation, not
    /// markup. Backslash-escaped characters are visible as themselves.
    fn visible_runs(&self, fragment: &str) -> Vec<Range<usize>> {
        let mut runs = RunBuilder::default();
        let chars: Vec<(usize, char)> = fragment.char_indices().collect();
        let mut i = 0;
        let mut bracket_stack: Vec<bool> = Vec::new();
        while let Some(&(pos, ch)) = chars.get(i) {
            match ch {
                '\\' => {
                    if let Some(&(epos, echar)) = chars.get(i + 1) {
                        runs.push_visible(epos, epos + echar.len_utf8());
                    }
                    i += 2;
                }
                '#' => i = consume_function_head(&chars, i, &mut bracket_stack),
                '[' => {
                    bracket_stack.push(false);
                    runs.push_visible(pos, pos + 1);
                    i += 1;
                }
                ']' => {
                    if !bracket_stack.pop().unwrap_or(false) {
                        runs.push_visible(pos, pos + 1);
                    }
                    i += 1;
                }
                _ => {
                    runs.push_visible(pos, pos + ch.len_utf8());
                    i += 1;
                }
            }
        }
        runs.finish()
    }
}

/// Consume a `#ident(...)?[`-style function head starting at the `#` at
/// `chars[i]`: the identifier and any parenthesized arguments are markup
/// (invisible); if a content group `[` follows, push `true` onto
/// `bracket_stack` so its matching `]` is later recognized as markup too.
fn consume_function_head(
    chars: &[(usize, char)],
    i: usize,
    bracket_stack: &mut Vec<bool>,
) -> usize {
    let mut j = i + 1;
    while chars
        .get(j)
        .is_some_and(|&(_, c)| c.is_ascii_alphanumeric() || c == '_')
    {
        j += 1;
    }
    if chars.get(j).map(|&(_, c)| c) == Some('(') {
        j = skip_balanced(chars, j, '(', ')', true);
    }
    if chars.get(j).map(|&(_, c)| c) == Some('[') {
        bracket_stack.push(true);
        j += 1;
    }
    j
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;

    #[test]
    fn visible_text_strips_emph_function_and_brackets() {
        let fmt = Typst;
        assert_eq!(fmt.visible_text("#emph[Title.]"), "Title.");
    }

    #[test]
    fn visible_text_hides_link_target_keeps_content() {
        let fmt = Typst;
        assert_eq!(
            fmt.visible_text(r#"#link("https://example.com/a.b")[Example]"#),
            "Example"
        );
    }

    #[test]
    fn visible_text_handles_nested_functions() {
        let fmt = Typst;
        assert_eq!(fmt.visible_text("#strong[#emph[Title.]]"), "Title.");
    }

    #[test]
    fn visible_text_keeps_literal_wrap_brackets_visible() {
        let fmt = Typst;
        // WrapPunctuation::Brackets: bare `[content]`, not a function call.
        assert_eq!(fmt.visible_text("[Dataset]"), "[Dataset]");
    }

    #[test]
    fn visible_text_keeps_escaped_punctuation() {
        let fmt = Typst;
        assert_eq!(fmt.visible_text(r"A \# B"), "A # B");
    }
}
