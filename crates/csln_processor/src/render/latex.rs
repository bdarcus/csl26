/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! LaTeX output format.

use super::format::OutputFormat;
use csln_core::template::WrapPunctuation;

/// LaTeX renderer.
#[derive(Debug, Clone, Default)]
pub struct Latex;

impl OutputFormat for Latex {
    type Output = String;

    fn text(&self, s: &str) -> Self::Output {
        // Basic LaTeX escaping
        s.replace('\\', r"\textbackslash{}")
            .replace('{', r"\{")
            .replace('}', r"\}")
            .replace('$', r"\$")
            .replace('&', r"\&")
            .replace('#', r"\#")
            .replace('_', r"\_")
            .replace('%', r"\%")
            .replace('~', r"\textasciitilde{}")
            .replace('^', r"\textasciicircum{}")
    }

    fn join(&self, items: Vec<Self::Output>, delimiter: &str) -> Self::Output {
        items.join(delimiter)
    }

    fn finish(&self, output: Self::Output) -> String {
        output
    }

    fn emph(&self, content: Self::Output) -> Self::Output {
        format!(r"\textit{{{}}}", content)
    }

    fn strong(&self, content: Self::Output) -> Self::Output {
        format!(r"\textbf{{{}}}", content)
    }

    fn small_caps(&self, content: Self::Output) -> Self::Output {
        format!(r"\textsc{{{}}}", content)
    }

    fn quote(&self, content: Self::Output) -> Self::Output {
        format!("``{}''", content)
    }

    fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{}{}{}", self.text(prefix), content, self.text(suffix))
    }

    fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
        format!("{}{}{}", self.text(prefix), content, self.text(suffix))
    }

    fn wrap_punctuation(&self, wrap: &WrapPunctuation, content: Self::Output) -> Self::Output {
        match wrap {
            WrapPunctuation::Parentheses => format!("({})", content),
            WrapPunctuation::Brackets => format!("[{}]", content),
            WrapPunctuation::Quotes => self.quote(content),
            WrapPunctuation::None => content,
        }
    }

    fn semantic(&self, _class: &str, content: Self::Output) -> Self::Output {
        // In LaTeX, we could use custom commands if we wanted semantic tagging
        // For now, just return content
        content
    }

    fn link(&self, url: &str, content: Self::Output) -> Self::Output {
        format!(r"\href{{{}}}{{{}}}", url, content)
    }
}
