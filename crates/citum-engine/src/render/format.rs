/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Output format trait for pluggable renderers.

use std::borrow::Cow;
use std::ops::Range;

use crate::values::ScriptClass;
use citum_schema::locale::GrammarOptions;
use citum_schema::options::PunctuationRealization;
use citum_schema::template::{DelimiterPunctuation, WrapPunctuation};

/// Position in which a semantic punctuation mark is realized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PunctuationPosition {
    /// A separator between rendered values.
    Separator,
    /// An affix before rendered content.
    Prefix,
    /// An affix after rendered content.
    Suffix,
}

/// Realize literal text or a semantic punctuation mark for a script class.
///
/// Literal strings are returned unchanged. Style overrides take precedence
/// over the engine default table.
#[must_use]
pub(crate) fn realize_punctuation<'a>(
    punctuation: &'a DelimiterPunctuation,
    script: ScriptClass,
    overrides: Option<&'a PunctuationRealization>,
    position: PunctuationPosition,
) -> Cow<'a, str> {
    use DelimiterPunctuation as Punctuation;

    let override_value = overrides.and_then(|table| match punctuation {
        Punctuation::Comma => table.comma.as_deref().map(Cow::Borrowed),
        Punctuation::Colon => table.colon.as_deref().map(Cow::Borrowed),
        Punctuation::Semicolon => table.semicolon.as_deref().map(Cow::Borrowed),
        Punctuation::Period => table.period.as_deref().map(Cow::Borrowed),
        Punctuation::Parentheses => table
            .parentheses
            .as_ref()
            .map(|pair| pair_mark(pair, position)),
        Punctuation::Brackets => table
            .brackets
            .as_ref()
            .map(|pair| pair_mark(pair, position)),
        Punctuation::Ampersand
        | Punctuation::VerticalLine
        | Punctuation::Slash
        | Punctuation::Hyphen
        | Punctuation::Space
        | Punctuation::None
        | Punctuation::Custom(_) => None,
    });
    if let Some(value) = override_value {
        return value;
    }

    let default = match (punctuation, script, position) {
        (Punctuation::Comma, ScriptClass::Latin, _) => ", ",
        (Punctuation::Comma, ScriptClass::Cjk, _) => "，",
        (Punctuation::Comma, ScriptClass::Mixed, _) => "，",
        (Punctuation::Colon, ScriptClass::Latin, _) => ": ",
        (Punctuation::Colon, ScriptClass::Cjk, _) => "：",
        (Punctuation::Colon, ScriptClass::Mixed, _) => "：",
        (Punctuation::Semicolon, ScriptClass::Latin, _) => "; ",
        (Punctuation::Semicolon, ScriptClass::Cjk, _) => "；",
        (Punctuation::Semicolon, ScriptClass::Mixed, _) => "；",
        (Punctuation::Period, ScriptClass::Latin, _) => ". ",
        (Punctuation::Period, ScriptClass::Cjk, _) => "。",
        (Punctuation::Period, ScriptClass::Mixed, _) => ". ",
        (Punctuation::Parentheses, ScriptClass::Latin, PunctuationPosition::Prefix) => "(",
        (Punctuation::Parentheses, ScriptClass::Latin, PunctuationPosition::Suffix) => ")",
        (Punctuation::Parentheses, ScriptClass::Cjk, PunctuationPosition::Prefix) => "（",
        (Punctuation::Parentheses, ScriptClass::Mixed, PunctuationPosition::Prefix) => "（",
        (Punctuation::Parentheses, ScriptClass::Cjk, PunctuationPosition::Suffix) => "）",
        (Punctuation::Parentheses, ScriptClass::Mixed, PunctuationPosition::Suffix) => "）",
        (Punctuation::Brackets, ScriptClass::Latin, PunctuationPosition::Prefix) => "[",
        (Punctuation::Brackets, ScriptClass::Latin, PunctuationPosition::Suffix) => "]",
        (Punctuation::Brackets, ScriptClass::Cjk, PunctuationPosition::Prefix) => "【",
        (Punctuation::Brackets, ScriptClass::Mixed, PunctuationPosition::Prefix) => "[",
        (Punctuation::Brackets, ScriptClass::Cjk, PunctuationPosition::Suffix) => "】",
        (Punctuation::Brackets, ScriptClass::Mixed, PunctuationPosition::Suffix) => "]",
        (Punctuation::Parentheses, ScriptClass::Latin, PunctuationPosition::Separator) => "()",
        (Punctuation::Parentheses, ScriptClass::Cjk, PunctuationPosition::Separator) => "（）",
        (Punctuation::Parentheses, ScriptClass::Mixed, PunctuationPosition::Separator) => "（）",
        (Punctuation::Brackets, ScriptClass::Latin, PunctuationPosition::Separator) => "[]",
        (Punctuation::Brackets, ScriptClass::Cjk, PunctuationPosition::Separator) => "【】",
        (Punctuation::Brackets, ScriptClass::Mixed, PunctuationPosition::Separator) => "[]",
        (
            Punctuation::Ampersand
            | Punctuation::VerticalLine
            | Punctuation::Slash
            | Punctuation::Hyphen
            | Punctuation::Space
            | Punctuation::None
            | Punctuation::Custom(_),
            _,
            _,
        ) => return Cow::Borrowed(punctuation.as_default_str()),
    };
    Cow::Borrowed(default)
}

fn pair_mark(pair: &[String; 2], position: PunctuationPosition) -> Cow<'_, str> {
    match position {
        PunctuationPosition::Prefix => Cow::Borrowed(pair[0].as_str()),
        PunctuationPosition::Suffix => Cow::Borrowed(pair[1].as_str()),
        PunctuationPosition::Separator => Cow::Owned(format!("{}{}", pair[0], pair[1])),
    }
}

/// A realized separator decomposed into its leading character and the tail
/// that follows it, so punctuation-in-quote join sites (`render/bibliography.rs`,
/// `render/citation.rs`, `render/punctuation.rs`, `processor/rendering/grouped/core.rs`)
/// no longer each call `.chars().next()` on a plain `&str` to rediscover what a
/// mark's identity already determined at realization.
///
/// [`Self::core`] mirrors `text.chars().next()` exactly, so every existing
/// `matches!(core(), Some('.' | ','))` or `core() == Some(',')` comparison at
/// a join site is byte-identical to what it replaces. This intentionally does
/// *not* expose a [`PunctuationClass`](crate::render::punctuation::PunctuationClass)
/// of the realized glyph: classifying the
/// rendered character (rather than the source mark) is wrong for non-ASCII
/// realizations (a CJK `，` is comma-like by origin but not by any ASCII
/// classification of its glyph), and nothing in this codebase needs it —
/// quote-movement/collision resolution in `PUNCTUATION_NORMALIZATION.md` is
/// deliberately scoped to the Latin `.`/`,` convention. Extending it to
/// full-width marks is a real design question for a future spec increment,
/// not a byproduct of typing separators.
///
/// Must be built from the same string a join site will actually splice into
/// its output. For the `group:` join sites this is the *post-escape* string
/// (after `fmt.text`/`fmt.join` round-tripping), matching what
/// `.chars().next()` inspected before this type existed — see
/// `docs/specs/PUNCTUATION_REALIZATION.md` §6 on realization strictly
/// preceding output-format escaping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RealizedPunctuation<'a> {
    text: Cow<'a, str>,
    core_len: usize,
}

impl<'a> RealizedPunctuation<'a> {
    /// Decompose an already-realized separator string.
    pub(crate) fn new(text: Cow<'a, str>) -> Self {
        let core_len = text.chars().next().map(char::len_utf8).unwrap_or(0);
        Self { text, core_len }
    }

    /// The full realized separator text.
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The separator's leading character, or `None` when the separator is empty.
    pub(crate) fn core(&self) -> Option<char> {
        self.text.chars().next()
    }

    /// The separator with its leading character removed.
    pub(crate) fn tail(&self) -> &str {
        #[allow(
            clippy::string_slice,
            reason = "core_len is a char boundary derived from chars().next()"
        )]
        &self.text[self.core_len..]
    }

    /// Return whether the realized separator is the empty string.
    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Detach from the borrowed input, cloning if necessary.
    pub(crate) fn into_owned(self) -> RealizedPunctuation<'static> {
        RealizedPunctuation {
            text: Cow::Owned(self.text.into_owned()),
            core_len: self.core_len,
        }
    }
}

/// Realize `punctuation` and decompose the result — see [`RealizedPunctuation`].
#[must_use]
pub(crate) fn realize_punctuation_decomposed<'a>(
    punctuation: &'a DelimiterPunctuation,
    script: ScriptClass,
    overrides: Option<&'a PunctuationRealization>,
    position: PunctuationPosition,
) -> RealizedPunctuation<'a> {
    RealizedPunctuation::new(realize_punctuation(
        punctuation,
        script,
        overrides,
        position,
    ))
}

/// Apply realized punctuation affixes while routing semantic glyphs through
/// the active output format's text escaping.
pub(crate) fn apply_punctuation_affixes<F>(
    fmt: &F,
    prefix: Option<(&DelimiterPunctuation, &str)>,
    mut content: String,
    suffix: Option<(&DelimiterPunctuation, &str)>,
) -> String
where
    F: OutputFormat<Output = String>,
{
    if let Some((punctuation, text)) = prefix {
        content = if punctuation.is_semantic() {
            fmt.join(vec![fmt.text(text), content], "")
        } else {
            fmt.affix(text, content, "")
        };
    }
    if let Some((punctuation, text)) = suffix {
        content = if punctuation.is_semantic() {
            fmt.join(vec![content, fmt.text(text)], "")
        } else {
            fmt.affix("", content, text)
        };
    }
    content
}

/// Return Unicode quote marks for a nesting depth.
///
/// Even depths use outer double quotes; odd depths use inner single quotes.
#[must_use]
pub fn unicode_quote_marks(depth: usize) -> (&'static str, &'static str) {
    if depth.is_multiple_of(2) {
        ("\u{201C}", "\u{201D}")
    } else {
        ("\u{2018}", "\u{2019}")
    }
}

/// Locale-resolved quote mark characters, threaded from
/// [`GrammarOptions`] through to rendering so that
/// styles using non-English quotation conventions (e.g. fr-FR guillemets) render correctly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteMarks {
    /// Opening outer quotation mark.
    pub open: String,
    /// Closing outer quotation mark.
    pub close: String,
    /// Opening inner (nested) quotation mark.
    pub open_inner: String,
    /// Closing inner (nested) quotation mark.
    pub close_inner: String,
    /// Semantic punctuation realization table from the active locale.
    pub punctuation_realization: Option<citum_schema::options::PunctuationRealization>,
}

impl QuoteMarks {
    /// Return the opening and closing quote delimiters for a nesting depth.
    ///
    /// Depth 0 (and other even depths) use the outer pair; odd depths use the inner pair.
    #[must_use]
    pub fn for_depth(&self, depth: usize) -> (&str, &str) {
        if depth.is_multiple_of(2) {
            (&self.open, &self.close)
        } else {
            (&self.open_inner, &self.close_inner)
        }
    }
}

impl Default for QuoteMarks {
    /// The historical hardcoded English fallback, used when no resolved locale is available.
    fn default() -> Self {
        let (open, close) = unicode_quote_marks(0);
        let (open_inner, close_inner) = unicode_quote_marks(1);
        Self {
            open: open.to_string(),
            close: close.to_string(),
            open_inner: open_inner.to_string(),
            close_inner: close_inner.to_string(),
            punctuation_realization: None,
        }
    }
}

impl From<&GrammarOptions> for QuoteMarks {
    fn from(options: &GrammarOptions) -> Self {
        Self {
            open: options.open_quote.clone(),
            close: options.close_quote.clone(),
            open_inner: options.open_inner_quote.clone(),
            close_inner: options.close_inner_quote.clone(),
            punctuation_realization: None,
        }
    }
}

impl From<&citum_schema::locale::Locale> for QuoteMarks {
    fn from(locale: &citum_schema::locale::Locale) -> Self {
        Self {
            open: locale.grammar_options.open_quote.clone(),
            close: locale.grammar_options.close_quote.clone(),
            open_inner: locale.grammar_options.open_inner_quote.clone(),
            close_inner: locale.grammar_options.close_inner_quote.clone(),
            punctuation_realization: locale.punctuation_realization.clone(),
        }
    }
}

/// Extra attributes applied to semantic wrappers when a renderer supports them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticAttribute {
    /// The attribute name.
    pub name: &'static str,
    /// The attribute value.
    pub value: String,
}

/// Realize a semantic [`WrapPunctuation`] into the `(open, close)` glyph pair
/// for a script class.
///
/// Returns `None` for [`WrapPunctuation::Quotes`], which realizes through
/// locale-resolved quote marks (`QuoteMarks`) rather than a fixed pair — see
/// `docs/specs/PUNCTUATION_REALIZATION.md` §2. The table is closed for v1;
/// new marks or script classes require a spec revision.
#[must_use]
pub(crate) fn realize_wrap<'a>(
    wrap: &WrapPunctuation,
    script: ScriptClass,
    overrides: Option<&'a PunctuationRealization>,
) -> Option<(Cow<'a, str>, Cow<'a, str>)> {
    if let Some(pair) = overrides.and_then(|table| match wrap {
        WrapPunctuation::Parentheses => table.parentheses.as_ref(),
        WrapPunctuation::Brackets => table.brackets.as_ref(),
        WrapPunctuation::Quotes => None,
    }) {
        return Some((
            Cow::Borrowed(pair[0].as_str()),
            Cow::Borrowed(pair[1].as_str()),
        ));
    }

    match (wrap, script) {
        (WrapPunctuation::Parentheses, ScriptClass::Latin) => {
            Some((Cow::Borrowed("("), Cow::Borrowed(")")))
        }
        (WrapPunctuation::Parentheses, ScriptClass::Cjk) => {
            Some((Cow::Borrowed("（"), Cow::Borrowed("）")))
        }
        (WrapPunctuation::Parentheses, ScriptClass::Mixed) => {
            Some((Cow::Borrowed("（"), Cow::Borrowed("）")))
        }
        (WrapPunctuation::Brackets, ScriptClass::Latin) => {
            Some((Cow::Borrowed("["), Cow::Borrowed("]")))
        }
        (WrapPunctuation::Brackets, ScriptClass::Cjk) => {
            Some((Cow::Borrowed("【"), Cow::Borrowed("】")))
        }
        (WrapPunctuation::Brackets, ScriptClass::Mixed) => {
            Some((Cow::Borrowed("["), Cow::Borrowed("]")))
        }
        (WrapPunctuation::Quotes, _) => None,
    }
}

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

    /// Render content as superscript text.
    fn superscript(&self, content: Self::Output) -> Self::Output;

    /// Return the opening and closing quote delimiters for a nesting depth.
    ///
    /// Depth 0 is an outer quote pair, depth 1 is the first inner quote pair,
    /// and deeper levels alternate between those two pairs. `marks` carries the
    /// locale-resolved quote characters; callers with no resolved locale can pass
    /// `&QuoteMarks::default()` to keep the historical English fallback.
    fn quote_marks<'a>(&self, depth: usize, marks: &'a QuoteMarks) -> (&'a str, &'a str) {
        marks.for_depth(depth)
    }

    /// Render content enclosed in quotation marks at a specific nesting depth.
    fn quote_with_depth(
        &self,
        content: Self::Output,
        depth: usize,
        marks: &QuoteMarks,
    ) -> Self::Output {
        let (open, close) = self.quote_marks(depth, marks);
        self.affix(open, content, close)
    }

    /// Render content enclosed in outer quotation marks.
    fn quote(&self, content: Self::Output, marks: &QuoteMarks) -> Self::Output {
        self.quote_with_depth(content, 0, marks)
    }

    /// Apply outer prefix and suffix strings to the content.
    ///
    /// These are typically the "prefix" and "suffix" fields from the Citum style.
    fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output;

    /// Apply inner prefix and suffix strings to the content.
    ///
    /// These are applied inside any wrapping punctuation.
    fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output;

    /// Wrap the content in specific punctuation (parentheses, brackets, or quotes).
    ///
    /// `marks` supplies the locale-resolved quote characters for the `Quotes`
    /// variant. `script` selects the half-width or full-width glyph form for
    /// the `Parentheses`/`Brackets` variants — see `realize_wrap` and
    /// `docs/specs/PUNCTUATION_REALIZATION.md`.
    fn wrap_punctuation(
        &self,
        wrap: &WrapPunctuation,
        content: Self::Output,
        marks: &QuoteMarks,
        script: ScriptClass,
        realization: Option<&PunctuationRealization>,
    ) -> Self::Output;

    /// Apply a semantic identifier (class) to the content.
    ///
    /// This is used for machine readability or fine-grained CSS styling.
    /// Examples include "citum-title", "citum-author", "citum-doi".
    fn semantic(&self, class: &str, content: Self::Output) -> Self::Output;

    /// Render an annotation block.
    ///
    /// This is typically called at the end of a bibliography entry to render
    /// reader-supplied notes.
    fn annotation(&self, content: Self::Output) -> Self::Output;

    // ── Block-level methods (used by the body markup renderer) ─────────────
    // Defaults produce plain passthrough so existing format impls need not change.

    /// Render a paragraph block.
    fn paragraph(&self, content: Self::Output) -> Self::Output {
        content
    }

    /// Render a block quotation.
    fn block_quote(&self, content: Self::Output) -> Self::Output {
        content
    }

    /// Render an unordered (bullet) list from pre-rendered item strings.
    fn bullet_list(&self, items: Vec<Self::Output>) -> Self::Output {
        self.join(items, "\n")
    }

    /// Render an ordered (numbered) list from pre-rendered item strings.
    fn ordered_list(&self, items: Vec<Self::Output>) -> Self::Output {
        self.join(items, "\n")
    }

    /// Render a list item.
    fn list_item(&self, content: Self::Output) -> Self::Output {
        content
    }

    /// Render a heading at the given level (1 = top-level).
    fn heading(&self, _level: u8, content: Self::Output) -> Self::Output {
        content
    }

    /// Render an unnumbered heading at the given level.
    ///
    /// Used for generated section headings (e.g. bibliography group
    /// headings) that must not participate in document section numbering.
    /// Defaults to [`Self::heading`]; formats with numbered headings
    /// (LaTeX) override this with their unnumbered variants.
    fn unnumbered_heading(&self, level: u8, content: Self::Output) -> Self::Output {
        self.heading(level, content)
    }

    /// Render a fenced or indented code block with an optional language hint.
    ///
    /// `content` is the raw (unescaped) code text.
    fn code_block(&self, _lang: Option<&str>, content: Self::Output) -> Self::Output {
        content
    }

    /// Render inline code.
    fn inline_code(&self, content: Self::Output) -> Self::Output {
        content
    }

    /// Render strikethrough text.
    fn strikeout(&self, content: Self::Output) -> Self::Output {
        content
    }

    /// Render a hard line break.
    fn hard_break(&self) -> Self::Output {
        self.text(" ")
    }

    /// Apply a semantic identifier plus optional attributes to the content.
    ///
    /// Formats that do not support extra attributes can ignore them and reuse
    /// [`Self::semantic`].
    fn semantic_with_attributes(
        &self,
        class: &str,
        content: Self::Output,
        _attributes: &[SemanticAttribute],
    ) -> Self::Output {
        self.semantic(class, content)
    }

    /// Render a full citation container with one or more reference IDs.
    fn citation(&self, _ids: Vec<String>, content: Self::Output) -> Self::Output {
        content
    }

    // ── Visible-text methods ────────────────────────────────────────────────
    // Used by bibliography/citation punctuation-boundary logic so separator
    // and dedup decisions look at logical text, not backend markup (the
    // "backends differ only in markup" rule — see DESIGN_PRINCIPLES §7).

    /// Byte ranges of `fragment` that are visible (non-markup) text, in order.
    ///
    /// The default treats the whole fragment as visible, which is correct
    /// for [`PlainText`](crate::render::plain::PlainText) and safe for any
    /// third-party format that hasn't implemented a lexer: boundary logic
    /// simply falls back to looking at raw characters, as it always has.
    /// Backends whose inline methods (`emph`, `link`, `wrap_punctuation`,
    /// ...) emit markup should override this to exclude it.
    fn visible_runs(&self, fragment: &str) -> Vec<Range<usize>> {
        let mut runs = Vec::new();
        if !fragment.is_empty() {
            runs.push(0..fragment.len());
        }
        runs
    }

    /// The visible (markup-stripped) text of a rendered fragment.
    ///
    /// Borrows `fragment` unchanged when it is entirely visible (the common
    /// case); otherwise stitches the visible runs into an owned `String`.
    fn visible_text<'a>(&self, fragment: &'a str) -> Cow<'a, str> {
        let runs = self.visible_runs(fragment);
        if runs.len() == 1 && runs.first() == Some(&(0..fragment.len())) {
            return Cow::Borrowed(fragment);
        }
        let mut owned = String::with_capacity(fragment.len());
        for run in runs {
            if let Some(slice) = fragment.get(run) {
                owned.push_str(slice);
            }
        }
        Cow::Owned(owned)
    }

    /// Hyperlink the content to a URL.
    fn link(&self, url: &str, content: Self::Output) -> Self::Output;

    /// Format a reference ID for use as a target or link (e.g. adding a prefix).
    fn format_id(&self, id: &str) -> String {
        id.to_string()
    }

    /// Render a full bibliography container.
    ///
    /// The default implementation joins the entries with double newlines and
    /// ignores `layout`.
    fn bibliography(
        &self,
        entries: Vec<Self::Output>,
        _layout: &BibliographyLayout,
    ) -> Self::Output {
        self.join(entries, "\n\n")
    }

    /// Render a single bibliography entry with its unique identifier and optional link.
    ///
    /// The default implementation just returns the content.
    fn entry(
        &self,
        _id: &str,
        content: Self::Output,
        _url: Option<&str>,
        _metadata: &ProcEntryMetadata,
    ) -> Self::Output {
        content
    }

    /// Join a bibliography entry's reference marker and body into the entry's
    /// rendered content, honoring `layout`.
    ///
    /// The default implementation fuses the already-rendered marker and body with no separator —
    /// `[1]J. Smith`, matching citeproc-js `second-field-align` output
    /// flattened to text — and ignores `layout` entirely, so every format
    /// that doesn't override this method is unaffected by a style declaring
    /// `second-field-align`. Only [`Html`](super::html::Html) overrides this,
    /// to emit the marker and body as sibling slots when
    /// `layout.second_field_align` is set. See
    /// `docs/specs/SECOND_FIELD_ALIGN.md`.
    fn entry_slots(
        &self,
        marker: Option<Self::Output>,
        body: Self::Output,
        _layout: &BibliographyLayout,
    ) -> Self::Output {
        match marker {
            Some(marker) => self.join(vec![marker, body], ""),
            None => body,
        }
    }
}

/// Runtime bibliography layout, resolved from
/// [`BibliographyConfig`](citum_schema::options::BibliographyConfig) and
/// handed to the output format at the point a marker meets its body and when
/// the bibliography container is emitted. See
/// `docs/specs/SECOND_FIELD_ALIGN.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BibliographyLayout {
    /// CSL `second-field-align`. `None` when the style declares neither
    /// `flush` nor `margin`.
    pub second_field_align: Option<citum_schema::options::SecondFieldAlign>,
    /// CSL `hanging-indent`.
    pub hanging_indent: bool,
}

impl BibliographyLayout {
    /// Resolve a [`BibliographyLayout`] from a style's bibliography config.
    #[must_use]
    pub fn from_config(config: Option<&citum_schema::options::BibliographyConfig>) -> Self {
        let Some(config) = config else {
            return Self::default();
        };
        Self {
            second_field_align: config.second_field_align,
            hanging_indent: config.hanging_indent.unwrap_or(false),
        }
    }
}

/// Metadata for a processed bibliography entry, used for interactivity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcEntryMetadata {
    /// Rendered primary author(s) string.
    pub author: Option<String>,
    /// Rendered year string.
    pub year: Option<String>,
    /// Rendered title string.
    pub title: Option<String>,
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
    use rstest::rstest;

    #[derive(Default, Clone)]
    struct DummyFormat;

    impl OutputFormat for DummyFormat {
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
            format!("emph({content})")
        }
        fn strong(&self, content: Self::Output) -> Self::Output {
            format!("strong({content})")
        }
        fn small_caps(&self, content: Self::Output) -> Self::Output {
            format!("sc({content})")
        }
        fn superscript(&self, content: Self::Output) -> Self::Output {
            format!("sup({content})")
        }
        fn affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
            format!("{prefix}{content}{suffix}")
        }
        fn inner_affix(&self, prefix: &str, content: Self::Output, suffix: &str) -> Self::Output {
            format!("{prefix}{content}{suffix}")
        }
        fn wrap_punctuation(
            &self,
            _wrap: &WrapPunctuation,
            content: Self::Output,
            _marks: &QuoteMarks,
            _script: ScriptClass,
            _realization: Option<&PunctuationRealization>,
        ) -> Self::Output {
            content
        }
        fn semantic(&self, class: &str, content: Self::Output) -> Self::Output {
            format!("sem[{class}]({content})")
        }
        fn annotation(&self, content: Self::Output) -> Self::Output {
            format!("annot({content})")
        }
        fn link(&self, url: &str, content: Self::Output) -> Self::Output {
            format!("link[{url}]({content})")
        }
    }

    #[test]
    fn test_realize_wrap() {
        for (wrap, script, expected) in [
            (
                WrapPunctuation::Parentheses,
                ScriptClass::Latin,
                Some(("(", ")")),
            ),
            (
                WrapPunctuation::Parentheses,
                ScriptClass::Cjk,
                Some(("（", "）")),
            ),
            (
                WrapPunctuation::Brackets,
                ScriptClass::Latin,
                Some(("[", "]")),
            ),
            (
                WrapPunctuation::Brackets,
                ScriptClass::Cjk,
                Some(("【", "】")),
            ),
            (WrapPunctuation::Quotes, ScriptClass::Latin, None),
            (WrapPunctuation::Quotes, ScriptClass::Cjk, None),
        ] {
            assert_eq!(
                realize_wrap(&wrap, script, None)
                    .map(|(open, close)| (open.into_owned(), close.into_owned())),
                expected.map(|(open, close)| (open.to_string(), close.to_string())),
                "{wrap:?}/{script:?}"
            );
        }
    }

    #[test]
    fn paired_punctuation_override_includes_both_marks_as_separator() {
        let overrides = PunctuationRealization {
            parentheses: Some(["〔".to_string(), "〕".to_string()]),
            ..PunctuationRealization::default()
        };

        assert_eq!(
            realize_punctuation(
                &DelimiterPunctuation::Parentheses,
                ScriptClass::Cjk,
                Some(&overrides),
                PunctuationPosition::Separator,
            ),
            "〔〕"
        );
    }

    #[test]
    fn test_default_methods() {
        let fmt = DummyFormat;
        assert_eq!(
            fmt.semantic_with_attributes("test", "content".to_string(), &[]),
            "sem[test](content)"
        );
        assert_eq!(
            fmt.citation(vec!["id1".to_string()], "content".to_string()),
            "content"
        );
        assert_eq!(fmt.format_id("id1"), "id1");
        assert_eq!(
            fmt.bibliography(
                vec!["entry1".to_string(), "entry2".to_string()],
                &BibliographyLayout::default()
            ),
            "entry1\n\nentry2"
        );
        assert_eq!(
            fmt.entry(
                "id1",
                "content".to_string(),
                None,
                &ProcEntryMetadata::default()
            ),
            "content"
        );
    }

    #[test]
    fn entry_slots_default_fuses_marker_and_body_with_no_separator() {
        let fmt = DummyFormat;
        assert_eq!(
            fmt.entry_slots(
                Some("[1]".to_string()),
                "J. Smith".to_string(),
                &BibliographyLayout::default()
            ),
            "[1]J. Smith"
        );
        assert_eq!(
            fmt.entry_slots(None, "J. Smith".to_string(), &BibliographyLayout::default()),
            "J. Smith"
        );
    }

    #[test]
    fn entry_slots_default_ignores_second_field_align() {
        let fmt = DummyFormat;
        let layout = BibliographyLayout {
            second_field_align: Some(citum_schema::options::SecondFieldAlign::Flush),
            hanging_indent: true,
        };
        assert_eq!(
            fmt.entry_slots(Some("[1]".to_string()), "J. Smith".to_string(), &layout),
            "[1]J. Smith",
            "a format that doesn't override entry_slots must render identically \
             whether or not the style declares second-field-align"
        );
    }

    #[test]
    fn entry_slots_default_preserves_preformatted_typst_marker() {
        use crate::render::typst::Typst;

        let fmt = Typst;
        assert_eq!(
            fmt.entry_slots(
                Some(r"\[1\]".to_string()),
                "Title Text".to_string(),
                &BibliographyLayout::default(),
            ),
            r"\[1\]Title Text"
        );
    }

    #[test]
    fn semantic_affixes_use_each_output_formats_text_escaping() {
        let punctuation = DelimiterPunctuation::Comma;

        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::plain::PlainText,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "<&value"
        );
        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::html::Html,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "&lt;&amp;value"
        );
        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::latex::Latex,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "<\\&value"
        );
        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::typst::Typst,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "\\<&value"
        );
        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::markdown::Markdown,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "\\<\\&value"
        );
        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::djot::Djot,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "<&value"
        );
        assert_eq!(
            apply_punctuation_affixes(
                &crate::render::org::OrgOutputFormat,
                Some((&punctuation, "<&")),
                "value".to_string(),
                None,
            ),
            "<&value"
        );
    }

    #[rstest]
    #[case::latin_comma(DelimiterPunctuation::Comma, ScriptClass::Latin, Some(','), " ")]
    #[case::cjk_comma_has_no_tail(DelimiterPunctuation::Comma, ScriptClass::Cjk, Some('，'), "")]
    #[case::custom_period_matches_semantic_period_under_latin(
        DelimiterPunctuation::Custom(". ".to_string()),
        ScriptClass::Latin,
        Some('.'),
        " ",
    )]
    #[case::custom_empty_has_no_core(
        DelimiterPunctuation::Custom(String::new()),
        ScriptClass::Latin,
        None,
        ""
    )]
    #[case::custom_ampersand_space_led_core_is_not_terminal_punctuation(
        DelimiterPunctuation::Custom(" & ".to_string()),
        ScriptClass::Latin,
        Some(' '),
        "& ",
    )]
    fn realized_punctuation_decomposes_core_and_tail(
        #[case] punctuation: DelimiterPunctuation,
        #[case] script: ScriptClass,
        #[case] expected_core: Option<char>,
        #[case] expected_tail: &str,
    ) {
        let realized = realize_punctuation_decomposed(
            &punctuation,
            script,
            None,
            PunctuationPosition::Separator,
        );

        assert_eq!(realized.core(), expected_core);
        assert_eq!(realized.tail(), expected_tail);
    }

    #[test]
    fn realized_punctuation_french_colon_has_no_movable_core() {
        // The parity case from `docs/specs/PUNCTUATION_REALIZATION.md` §2: a
        // locale-supplied realization can lead with a non-breaking space
        // rather than the mark's own glyph, so `core()` — which mirrors
        // `chars().next()` exactly — returns the NBSP, not `:`. Downstream
        // `matches!(core(), Some('.' | ','))` movement checks correctly treat
        // this the same as no leading punctuation at all.
        let realization = citum_schema::options::PunctuationRealization {
            colon: Some("\u{00A0}: ".to_string()),
            ..Default::default()
        };
        let punctuation = DelimiterPunctuation::Colon;

        let realized = realize_punctuation_decomposed(
            &punctuation,
            ScriptClass::Latin,
            Some(&realization),
            PunctuationPosition::Separator,
        );

        assert_eq!(realized.text(), "\u{00A0}: ");
        assert_eq!(realized.core(), Some('\u{00A0}'));
        assert!(!matches!(realized.core(), Some('.' | ',')));
    }

    #[test]
    fn realized_punctuation_is_empty_and_into_owned_detach_from_the_input() {
        let borrowed = RealizedPunctuation::new(Cow::Borrowed(""));
        assert!(borrowed.is_empty());

        let source = String::from(", ");
        let realized = RealizedPunctuation::new(Cow::Borrowed(source.as_str()));
        let owned = realized.into_owned();
        drop(source);

        assert_eq!(owned.text(), ", ");
        assert_eq!(owned.core(), Some(','));
    }
}
