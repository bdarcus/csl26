/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! HTML output format.

use super::format::OutputFormat;
use csln_core::template::WrapPunctuation;

#[derive(Default, Clone)]
pub struct Html;

impl OutputFormat for Html {
    type Output = String;

    fn text(&self, s: &str) -> Self::Output {
        // As requested, we avoid escaping and use raw Unicode.
        s.to_string()
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
        format!("<i>{}</i>", content)
    }

    fn strong(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("<b>{}</b>", content)
    }

    fn small_caps(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(
            r#"<span style="font-variant:small-caps">{}</span>"#,
            content
        )
    }

    fn quote(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("\u{201C}{}\u{201D}", content)
    }

    fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{}{}{}", prefix, content, suffix)
    }

    fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{}{}{}", prefix, content, suffix)
    }

    fn wrap_punctuation(&self, wrap: &WrapPunctuation, content: Self::Output) -> Self::Output {
        match wrap {
            WrapPunctuation::Parentheses => format!("({})", content),
            WrapPunctuation::Brackets => format!("[{}]", content),
            WrapPunctuation::Quotes => format!("\u{201C}{}\u{201D}", content),
            WrapPunctuation::None => content,
        }
    }

    fn semantic(&self, class: &str, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(r#"<span class="{}">{}</span>"#, class, content)
    }

    fn citation(&self, ids: Vec<String>, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        let ids_str = ids.join(" ");
        format!(
            r#"<span class="csln-citation" data-ref="{}">{}</span>"#,
            ids_str, content
        )
    }

    fn link(&self, url: &str, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!(r#"<a href="{}">{}</a>"#, url, content)
    }

    fn format_id(&self, id: &str) -> String {
        format!("ref-{}", id)
    }

    fn bibliography(&self, entries: Vec<Self::Output>) -> Self::Output {
        format!(
            r#"<div class="csln-bibliography">
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

        let mut attrs = format!(r#"id="{}""#, self.format_id(id));
        if let Some(author) = &metadata.author {
            attrs.push_str(&format!(r#" data-author="{}""#, author));
        }
        if let Some(year) = &metadata.year {
            attrs.push_str(&format!(r#" data-year="{}""#, year));
        }
        if let Some(title) = &metadata.title {
            attrs.push_str(&format!(r#" data-title="{}""#, title));
        }

        format!(r#"<div class="csln-entry" {}>{}</div>"#, attrs, content)
    }
}
