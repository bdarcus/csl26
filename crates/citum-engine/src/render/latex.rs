/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! LaTeX output format.

use std::borrow::Cow;
use std::ops::Range;

use super::format::{OutputFormat, QuoteMarks, realize_wrap};
use super::visible_scan::{RunBuilder, skip_balanced};
use crate::values::ScriptClass;
use citum_schema::template::WrapPunctuation;

/// LaTeX renderer.
#[derive(Debug, Clone, Default)]
pub struct Latex;

impl Latex {
    /// Escapes characters that break a LaTeX `\href{...}` URL argument.
    ///
    /// Minimal set for the audit finding: a bare `%` starts a LaTeX comment
    /// and truncates the rest of the line, `#` breaks `{}`-grouping (macro
    /// parameter syntax), and `\` would otherwise be read as a control
    /// sequence. `\` is escaped first so the backslash-introducing escapes
    /// for `%`/`#` are not themselves re-escaped.
    fn escape_href_target(url: &str) -> String {
        url.replace('\\', r"\textbackslash{}")
            .replace('%', r"\%")
            .replace('#', r"\#")
    }
}

impl OutputFormat for Latex {
    type Output = String;

    fn text(&self, s: &str) -> Self::Output {
        let mut res = String::with_capacity(s.len() + 10);
        for c in s.chars() {
            match c {
                '\\' => res.push_str(r"\textbackslash{}"),
                '{' => res.push_str(r"\{"),
                '}' => res.push_str(r"\}"),
                '$' => res.push_str(r"\$"),
                '&' => res.push_str(r"\&"),
                '#' => res.push_str(r"\#"),
                '_' => res.push_str(r"\_"),
                '%' => res.push_str(r"\%"),
                '~' => res.push_str(r"\textasciitilde{}"),
                '^' => res.push_str(r"\textasciicircum{}"),
                _ => res.push(c),
            }
        }
        res
    }

    fn join(&self, items: Vec<Self::Output>, delimiter: &str) -> Self::Output {
        items.join(&self.text(delimiter))
    }

    fn finish(&self, output: Self::Output) -> String {
        // Escape any bare & not already preceded by backslash.
        // Locale terms (e.g. the & from AndOptions::Symbol) bypass text() and
        // arrive here unescaped; this final pass makes the output valid LaTeX.
        let mut result = String::with_capacity(output.len() + 4);
        let mut prev = '\0';
        for c in output.chars() {
            if c == '&' && prev != '\\' {
                result.push_str(r"\&");
            } else {
                result.push(c);
            }
            prev = c;
        }
        result
    }

    fn emph(&self, content: Self::Output) -> Self::Output {
        format!(r"\emph{{{content}}}")
    }

    fn strong(&self, content: Self::Output) -> Self::Output {
        format!(r"\textbf{{{content}}}")
    }

    fn small_caps(&self, content: Self::Output) -> Self::Output {
        format!(r"\textsc{{{content}}}")
    }

    fn superscript(&self, content: Self::Output) -> Self::Output {
        format!(r"\textsuperscript{{{content}}}")
    }

    fn quote(&self, content: Self::Output, marks: &QuoteMarks) -> Self::Output {
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
        // In LaTeX, we could use custom commands if we wanted semantic tagging
        // For now, just return content
        content
    }

    fn annotation(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(
            "\n\\begin{{citumannotation}}\n{}\n\\end{{citumannotation}}",
            content
        )
    }

    fn link(&self, url: &str, content: Self::Output) -> Self::Output {
        let target = Self::escape_href_target(url);
        format!(r"\href{{{target}}}{{{content}}}")
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
        format!("\\begin{{quote}}\n{trimmed}\n\\end{{quote}}\n\n")
    }

    fn bullet_list(&self, items: Vec<Self::Output>) -> Self::Output {
        if items.is_empty() {
            return String::new();
        }
        let body = items
            .iter()
            .map(|item| format!("  \\item {}", item.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\\begin{{itemize}}\n{body}\n\\end{{itemize}}\n\n")
    }

    fn ordered_list(&self, items: Vec<Self::Output>) -> Self::Output {
        if items.is_empty() {
            return String::new();
        }
        let body = items
            .iter()
            .map(|item| format!("  \\item {}", item.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\\begin{{enumerate}}\n{body}\n\\end{{enumerate}}\n\n")
    }

    fn heading(&self, level: u8, content: Self::Output) -> Self::Output {
        let cmd = match level {
            1 => "\\section",
            2 => "\\subsection",
            3 => "\\subsubsection",
            _ => "\\paragraph",
        };
        format!("{cmd}{{{content}}}\n\n")
    }

    fn unnumbered_heading(&self, level: u8, content: Self::Output) -> Self::Output {
        let cmd = match level {
            1 => "\\section*",
            2 => "\\subsection*",
            3 => "\\subsubsection*",
            _ => "\\paragraph*",
        };
        format!("{cmd}{{{content}}}\n\n")
    }

    fn code_block(&self, _lang: Option<&str>, content: Self::Output) -> Self::Output {
        format!("\\begin{{verbatim}}\n{content}\\end{{verbatim}}\n\n")
    }

    fn inline_code(&self, content: Self::Output) -> Self::Output {
        // \texttt is not verbatim; escape LaTeX specials in the raw code content.
        format!("\\texttt{{{}}}", self.text(&content))
    }

    fn strikeout(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("\\sout{{{content}}}")
    }

    fn hard_break(&self) -> Self::Output {
        "\\\\\n".to_string()
    }

    fn bibliography(
        &self,
        entries: Vec<Self::Output>,
        _layout: &super::format::BibliographyLayout,
    ) -> Self::Output {
        entries.join("\\par\\vspace{0.5em}")
    }

    fn entry(
        &self,
        _id: &str,
        content: Self::Output,
        _url: Option<&str>,
        _metadata: &super::format::ProcEntryMetadata,
    ) -> Self::Output {
        format!("\\noindent\\hangindent=2em\\hangafter=1 {content}")
    }

    /// Strip LaTeX commands (`\emph`, `\textbf`, ...) and their brace
    /// delimiters, keeping brace *contents* visible. A backslash-escaped
    /// punctuation mark (`\&`, `\_`, `\%`, `\#`, `\$`, `\{`, `\}`) is visible
    /// as the escaped character itself. `\href{target}{content}` is
    /// special-cased so the URL target stays invisible — it must not
    /// participate in separator/dedup decisions — while `content` does.
    ///
    /// Named commands that stand in for a single visible character
    /// (`\textbackslash{}`, `\textasciitilde{}`, `\textasciicircum{}`) have
    /// no backing byte range here — see [`Self::visible_text`], which
    /// synthesizes them for boundary decisions.
    fn visible_runs(&self, fragment: &str) -> Vec<Range<usize>> {
        let mut runs = RunBuilder::default();
        let chars: Vec<(usize, char)> = fragment.char_indices().collect();
        let mut i = 0;
        while let Some(&(pos, ch)) = chars.get(i) {
            if ch == '\\' {
                let (escape, next_i) = scan_backslash_escape(&chars, i);
                if let LatexEscape::Punct(_) = escape
                    && let Some(&(epos, echar)) = chars.get(i + 1)
                {
                    runs.push_visible(epos, epos + echar.len_utf8());
                }
                i = next_i;
                continue;
            }
            if ch == '{' || ch == '}' {
                i += 1;
                continue;
            }
            runs.push_visible(pos, pos + ch.len_utf8());
            i += 1;
        }
        runs.finish()
    }

    /// As [`Self::visible_runs`], but additionally synthesizes the single
    /// visible character that `\textbackslash{}`/`\textasciitilde{}`/
    /// `\textasciicircum{}` stand in for (`\`, `~`, `^`), since those have no
    /// backing byte range in the raw fragment. Used for read-only boundary
    /// decisions (first/last visible char); `visible_runs` stays raw-byte
    /// accurate for `cleanup_dangling_punctuation`'s in-place raw edits.
    fn visible_text<'a>(&self, fragment: &'a str) -> Cow<'a, str> {
        let chars: Vec<(usize, char)> = fragment.char_indices().collect();
        let mut i = 0;
        let mut owned = String::with_capacity(fragment.len());
        let mut any_markup = false;
        while let Some(&(_, ch)) = chars.get(i) {
            if ch == '\\' {
                any_markup = true;
                let (escape, next_i) = scan_backslash_escape(&chars, i);
                match escape {
                    LatexEscape::Punct(c) => owned.push(c),
                    LatexEscape::Command { synth: Some(c) } => owned.push(c),
                    LatexEscape::Command { synth: None } | LatexEscape::Bare => {}
                }
                i = next_i;
                continue;
            }
            if ch == '{' || ch == '}' {
                any_markup = true;
                i += 1;
                continue;
            }
            owned.push(ch);
            i += 1;
        }
        if any_markup {
            Cow::Owned(owned)
        } else {
            Cow::Borrowed(fragment)
        }
    }
}

/// Outcome of classifying the backslash escape at `chars[i]`.
enum LatexEscape {
    /// An escaped punctuation mark (`\&`, `\_`, ...); visible as the escaped char.
    Punct(char),
    /// A named ascii-alpha command. `synth` holds the single character it
    /// stands in for (`\textbackslash{}` → `\`, etc.), when applicable.
    Command { synth: Option<char> },
    /// A lone backslash with no recognized continuation.
    Bare,
}

/// Classify the `\` at `chars[i]`, returning the outcome and the index just
/// past it — past the command name (and past `\href`'s target brace group),
/// or past the escaped char for punctuation escapes.
fn scan_backslash_escape(chars: &[(usize, char)], i: usize) -> (LatexEscape, usize) {
    match chars.get(i + 1).map(|&(_, c)| c) {
        Some(c @ ('{' | '}' | '$' | '&' | '#' | '_' | '%')) => (LatexEscape::Punct(c), i + 2),
        Some(c) if c.is_ascii_alphabetic() => {
            let mut j = i + 1;
            let mut command = String::new();
            while let Some(&(_, cc)) = chars.get(j) {
                if !cc.is_ascii_alphabetic() {
                    break;
                }
                command.push(cc);
                j += 1;
            }
            let synth = match command.as_str() {
                "textbackslash" => Some('\\'),
                "textasciitilde" => Some('~'),
                "textasciicircum" => Some('^'),
                _ => None,
            };
            let end = if command == "href" {
                skip_balanced(chars, j, '{', '}', false)
            } else {
                j
            };
            (LatexEscape::Command { synth }, end)
        }
        _ => (LatexEscape::Bare, i + 1),
    }
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
    fn visible_text_strips_emph_command_and_braces() {
        let fmt = Latex;
        assert_eq!(fmt.visible_text(r"\emph{Title.}"), "Title.");
    }

    #[test]
    fn visible_text_keeps_escaped_punctuation() {
        let fmt = Latex;
        assert_eq!(fmt.visible_text(r"Smith \& Jones"), "Smith & Jones");
    }

    #[test]
    fn visible_text_hides_href_target_keeps_content() {
        let fmt = Latex;
        assert_eq!(
            fmt.visible_text(r"\href{https://example.com/a.b}{Example}"),
            "Example"
        );
    }

    #[test]
    fn visible_text_handles_nested_commands() {
        let fmt = Latex;
        assert_eq!(fmt.visible_text(r"\textbf{\emph{Title.}}"), "Title.");
    }

    #[test]
    fn visible_text_is_borrowed_when_no_markup() {
        let fmt = Latex;
        assert_eq!(fmt.visible_text("Plain text."), "Plain text.");
    }

    #[test]
    fn visible_text_synthesizes_escaped_backslash() {
        let fmt = Latex;
        assert_eq!(fmt.visible_text(r"C:\textbackslash{}Users"), r"C:\Users");
    }

    #[test]
    fn visible_text_synthesizes_escaped_tilde() {
        let fmt = Latex;
        assert_eq!(
            fmt.visible_text(r"Title\textasciitilde{}Subtitle"),
            "Title~Subtitle"
        );
    }

    #[test]
    fn visible_text_synthesizes_escaped_caret() {
        let fmt = Latex;
        assert_eq!(fmt.visible_text(r"x\textasciicircum{}2"), "x^2");
    }

    #[test]
    fn visible_text_synthesized_char_is_seen_as_the_trailing_char() {
        // A field ending in an escaped tilde must expose that tilde as the
        // last visible char, not disappear (the gap this fixes: the raw
        // fragment's last char is `}`, not `~`).
        let fmt = Latex;
        let rendered = fmt.emph(r"Title\textasciitilde{}".to_string());
        assert_eq!(fmt.visible_text(&rendered).chars().last(), Some('~'));
    }

    #[test]
    fn visible_runs_does_not_claim_a_byte_range_for_synthesized_chars() {
        // visible_runs stays raw-byte accurate (used for in-place raw edits
        // in cleanup_dangling_punctuation) — the synthesized char has no
        // backing byte, so it must not appear as a run.
        let fmt = Latex;
        let runs = fmt.visible_runs(r"\textasciitilde{}");
        assert!(runs.is_empty(), "expected no visible runs, got {runs:?}");
    }
}
