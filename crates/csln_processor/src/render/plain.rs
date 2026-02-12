/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Plain text output format.

use super::format::OutputFormat;
use csln_core::template::WrapPunctuation;

#[derive(Default, Clone)]
pub struct PlainText;

impl OutputFormat for PlainText {
    type Output = String;

    fn text(&self, s: &str) -> Self::Output {
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
        format!("_{}_", content)
    }

    fn strong(&self, content: Self::Output) -> Self::Output {
        if content.is_empty() {
            return content;
        }
        format!("**{}**", content)
    }

    fn small_caps(&self, content: Self::Output) -> Self::Output {
        // Plain text fallback for small caps could be capitals, but usually we just keep it as is
        // or use span-like markers if we wanted, but let's stick to plain text.
        content
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

    fn semantic(&self, _class: &str, content: Self::Output) -> Self::Output {
        // Plain text ignores semantic classes
        content
    }

    fn link(&self, _url: &str, content: Self::Output) -> Self::Output {
        // Plain text just renders the text content of the link
        content
    }

    fn entry(&self, _id: &str, content: Self::Output, _url: Option<&str>) -> Self::Output {
        content
    }
}
