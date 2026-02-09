/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Output format trait for pluggable renderers.

use csln_core::template::WrapPunctuation;

/// Trait for defining how to render template components into a specific format.
///
/// Implementations of this trait define how various formatting instructions
/// (emphasis, quotes, links, etc.) are translated into specific markup or text.
pub trait OutputFormat: Default + Clone {
    /// The type used for intermediate rendered content.
    ///
    /// For simple text formats, this is usually `String`. More complex formats
    /// might use an AST or a specialized builder type.
    type Output;

    /// Convert a raw string into the format's output type.
    ///
    /// The implementation should handle any necessary character escaping
    /// required by the target format.
    fn text(&self, s: &str) -> Self::Output;

    /// Join multiple outputs into a single output using a delimiter.
    fn join(&self, items: Vec<Self::Output>, delimiter: &str) -> Self::Output;

    /// Convert the intermediate output into the final result string.
    ///
    /// This is called exactly once at the end of the rendering process
    /// for a top-level component (citation or bibliography entry).
    fn finish(&self, output: Self::Output) -> String;

    /// Render content with emphasis (typically italics).
    fn emph(&self, content: Self::Output) -> Self::Output;

    /// Render content with strong emphasis (typically bold).
    fn strong(&self, content: Self::Output) -> Self::Output;

    /// Render content in small capitals.
    fn small_caps(&self, content: Self::Output) -> Self::Output;

    /// Render content enclosed in quotation marks.
    fn quote(&self, content: Self::Output) -> Self::Output;

    /// Apply outer prefix and suffix strings to the content.
    ///
    /// These are typically the "prefix" and "suffix" fields from the CSLN style.
    fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output;

    /// Apply inner prefix and suffix strings to the content.
    ///
    /// These are applied inside any wrapping punctuation.
    fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output;

    /// Wrap the content in specific punctuation (parentheses, brackets, or quotes).
    fn wrap_punctuation(&self, wrap: &WrapPunctuation, content: Self::Output) -> Self::Output;

    /// Apply a semantic identifier (class) to the content.
    ///
    /// This is used for machine readability or fine-grained CSS styling.
    /// Examples include "csln-title", "csln-author", "csln-doi".
    fn semantic(&self, class: &str, content: Self::Output) -> Self::Output;

    /// Hyperlink the content to a URL.
    fn link(&self, url: &str, content: Self::Output) -> Self::Output;
}
