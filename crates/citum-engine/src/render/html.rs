/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! HTML output format.

use super::format::{OutputFormat, QuoteMarks, SemanticAttribute, realize_wrap};
use crate::values::ScriptClass;
use citum_schema::template::WrapPunctuation;
use std::fmt::Write;

#[derive(Default, Clone)]
/// Renders processed citations and bibliography entries as HTML fragments.
pub struct Html;

impl Html {
    fn sanitize_href(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for ch in value.chars() {
            if ch.is_ascii_control()
                || ch.is_whitespace()
                || matches!(ch, '"' | '\'' | '<' | '>' | '&')
            {
                let mut buf = [0u8; 4];
                for byte in ch.encode_utf8(&mut buf).as_bytes() {
                    escaped.push('%');
                    let _ = write!(escaped, "{byte:02X}");
                }
            } else {
                escaped.push(ch);
            }
        }
        escaped
    }

    fn escape_attribute_value(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    /// Escapes the three HTML-active characters (`&`, `<`, `>`) in text content.
    ///
    /// Order matters: `&` must be escaped first, otherwise the entities emitted
    /// for `<` and `>` would themselves be escaped.
    fn escape_text(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

impl OutputFormat for Html {
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

    fn heading(&self, level: u8, content: Self::Output) -> Self::Output {
        let level = level.clamp(1, 6);
        format!("<h{level}>{content}</h{level}>\n\n")
    }

    fn emph(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("<em>{content}</em>")
    }

    fn strong(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("<b>{content}</b>")
    }

    fn small_caps(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(r#"<span style="font-variant:small-caps">{content}</span>"#)
    }

    fn superscript(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("<sup>{content}</sup>")
    }

    fn quote(&self, content: Self::Output, marks: &QuoteMarks) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        let (open, close) = marks.for_depth(0);
        format!("{open}{content}{close}")
    }

    fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{prefix}{content}{suffix}")
    }

    fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{prefix}{content}{suffix}")
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

    fn semantic(&self, class: &str, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(r#"<span class="{class}">{content}</span>"#)
    }

    fn annotation(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("<div class=\"citum-annotation\">{content}</div>")
    }

    fn semantic_with_attributes(
        &self,
        class: &str,
        content: Self::Output,
        attributes: &[SemanticAttribute],
    ) -> Self::Output {
        if content.is_empty() {
            return content;
        }

        let mut extra_attrs = String::new();
        for attribute in attributes {
            let _ = write!(
                &mut extra_attrs,
                r#" {}="{}""#,
                attribute.name,
                Self::escape_attribute_value(&attribute.value)
            );
        }

        format!(r#"<span class="{class}"{extra_attrs}>{content}</span>"#)
    }

    fn citation(&self, ids: Vec<String>, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        let ids_str = Self::escape_attribute_value(&ids.join(" "));
        format!(r#"<span class="citum-citation" data-ref="{ids_str}">{content}</span>"#)
    }

    fn link(&self, url: &str, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(r#"<a href="{}">{}</a>"#, Self::sanitize_href(url), content)
    }

    fn format_id(&self, id: &str) -> String {
        format!("ref-{id}")
    }

    fn bibliography(
        &self,
        entries: Vec<Self::Output>,
        layout: &super::format::BibliographyLayout,
    ) -> Self::Output {
        let class = if layout.hanging_indent {
            "citum-bibliography citum-bibliography--hanging-indent"
        } else {
            "citum-bibliography"
        };
        format!(
            r#"<div class="{class}">
{}
</div>"#,
            self.join(entries, "\n")
        )
    }

    fn entry(
        &self,
        id: &str,
        content: Self::Output,
        url: Option<&str>,
        metadata: &super::format::ProcEntryMetadata,
    ) -> Self::Output {
        let content = if let Some(u) = url {
            self.link(u, content)
        } else {
            content
        };

        let mut attrs = format!(
            r#"id="{}""#,
            Self::escape_attribute_value(&self.format_id(id))
        );
        if let Some(author) = &metadata.author {
            attrs.push_str(r#" data-author=""#);
            attrs.push_str(&Self::escape_attribute_value(author));
            attrs.push('"');
        }
        if let Some(year) = &metadata.year {
            attrs.push_str(r#" data-year=""#);
            attrs.push_str(&Self::escape_attribute_value(year));
            attrs.push('"');
        }
        if let Some(title) = &metadata.title {
            attrs.push_str(r#" data-title=""#);
            attrs.push_str(&Self::escape_attribute_value(title));
            attrs.push('"');
        }

        format!(r#"<div class="citum-entry" {attrs}>{content}</div>"#)
    }

    fn entry_slots(
        &self,
        marker: Option<Self::Output>,
        body: Self::Output,
        layout: &super::format::BibliographyLayout,
    ) -> Self::Output {
        let Some(marker) = marker else {
            return body;
        };
        if layout.second_field_align.is_none() {
            // No alignment declared: fall back to the flush fuse every other
            // format uses, unchanged from before this method existed.
            return self.join(vec![marker, body], "");
        }
        // `flush` and `margin` render identically here — both are two
        // sibling boxes; the distinction is a CSS-layer concern for the
        // consuming stylesheet, not a difference in Citum's own markup. See
        // docs/specs/SECOND_FIELD_ALIGN.md.
        format!(
            r#"<div class="citum-entry-marker">{marker}</div><div class="citum-entry-body">{body}</div>"#
        )
    }

    fn visible_runs(&self, fragment: &str) -> Vec<std::ops::Range<usize>> {
        let mut runs = super::visible_scan::RunBuilder::default();
        let mut in_tag = false;
        for (i, ch) in fragment.char_indices() {
            match ch {
                '<' => in_tag = true,
                '>' if in_tag => in_tag = false,
                _ if !in_tag => runs.push_visible(i, i + ch.len_utf8()),
                _ => {}
            }
        }
        runs.finish()
    }
}
