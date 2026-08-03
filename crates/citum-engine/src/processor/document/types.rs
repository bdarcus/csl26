/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Shared document-processing types and parser contracts.

use crate::Citation;
use citum_schema::Style;
use citum_schema::grouping::BibliographyGroup;
use citum_schema::locale::Locale;
use citum_schema::options::bibliography::{
    BibliographyPartitionHeading, BibliographyPartitionKind, BibliographyPartitionMode,
    BibliographySortPartitioning,
};
use citum_schema::options::scoped::{
    BibliographyLabelMode, BibliographyLabelWrap, RepeatedAuthorRendering,
};
use citum_schema::options::{IntegralNameMemoryConfig, OrgAbbreviationMemoryConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

/// Describes where a parsed citation appears in the source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationPlacement {
    /// The citation marker appears in prose and should become a generated note
    /// reference for note styles.
    InlineProse,
    /// The citation marker appears inside a manually authored footnote
    /// definition and should render in place.
    ManualFootnote {
        /// The source footnote label that identifies the manual note block.
        label: String,
    },
}

/// Structural citation scope metadata derived from the source document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CitationStructure {
    /// The resolved chapter-level scope key for this citation location.
    pub chapter_scope: String,
    /// The resolved section-level scope key for this citation location.
    pub section_scope: String,
}

/// Document-level integral-name override parsed from frontmatter.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DocumentIntegralNameOverride {
    /// Whether the integral-name policy is enabled for this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Where name-memory resets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<citum_schema::options::IntegralNameScope>,
    /// Which document contexts participate in the policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<citum_schema::options::IntegralNameContexts>,
    /// The contributor form used after the first mention.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subsequent_form: Option<citum_schema::options::SubsequentNameForm>,
}

impl DocumentIntegralNameOverride {
    /// Apply this frontmatter override to a base integral-name configuration.
    ///
    /// Returns `None` if the override explicitly disables the policy. Otherwise,
    /// returns a new configuration where non-null fields from the override
    /// supersede fields from the base.
    pub(super) fn apply_to(
        &self,
        base: Option<&IntegralNameMemoryConfig>,
    ) -> Option<IntegralNameMemoryConfig> {
        if self.enabled == Some(false) {
            return None;
        }

        let mut result = base.cloned().unwrap_or_default();
        if self.scope.is_some() {
            result.scope = self.scope;
        }
        if self.contexts.is_some() {
            result.contexts = self.contexts;
        }
        if self.subsequent_form.is_some() {
            result.subsequent_form = self.subsequent_form;
        }
        Some(result)
    }
}

/// Document-level org-abbreviation-memory override parsed from frontmatter.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DocumentOrgAbbreviationOverride {
    /// Whether org-abbreviation-memory is enabled for this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Where name-memory resets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<citum_schema::options::IntegralNameScope>,
    /// Which document contexts participate in the policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<citum_schema::options::IntegralNameContexts>,
}

/// Per-document multilingual presentation overrides.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DocumentMultilingualOverride {
    /// Semantic punctuation-width preset for this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punctuation_width: Option<citum_schema::options::PunctuationWidth>,
}

impl DocumentOrgAbbreviationOverride {
    /// Apply this override to a base org-abbreviation-memory config.
    pub(super) fn apply_to(
        &self,
        base: Option<&OrgAbbreviationMemoryConfig>,
    ) -> Option<OrgAbbreviationMemoryConfig> {
        if self.enabled == Some(false) {
            return None;
        }
        let mut result = base.cloned().unwrap_or_default();
        if self.scope.is_some() {
            result.scope = self.scope;
        }
        if self.contexts.is_some() {
            result.contexts = self.contexts;
        }
        Some(result)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::DocumentIntegralNameOverride;
    use citum_schema::options::IntegralNameMemoryConfig;

    #[test]
    fn test_document_integral_name_override_enabled_false_disables_block() {
        let base = IntegralNameMemoryConfig::default();
        let override_config = DocumentIntegralNameOverride {
            enabled: Some(false),
            ..Default::default()
        };

        assert!(override_config.apply_to(Some(&base)).is_none());
    }

    #[test]
    fn test_document_bibliography_override_deserializes_label_mode() {
        use super::DocumentBibliographyOverride;
        use citum_schema::options::scoped::BibliographyLabelMode;

        let parsed: DocumentBibliographyOverride =
            serde_yaml::from_str("label-mode: numeric").unwrap();
        assert_eq!(parsed.label_mode, Some(BibliographyLabelMode::Numeric));
        assert!(parsed.repeated_author_rendering.is_none());
        assert!(parsed.sort_partitioning.is_none());
    }

    #[test]
    fn test_document_bibliography_override_deserializes_repeated_author() {
        use super::DocumentBibliographyOverride;
        use citum_schema::options::scoped::RepeatedAuthorRendering;

        let parsed: DocumentBibliographyOverride =
            serde_yaml::from_str("repeated-author-rendering: dash").unwrap();
        assert_eq!(
            parsed.repeated_author_rendering,
            Some(RepeatedAuthorRendering::Dash)
        );
    }

    #[test]
    fn test_document_options_override_deserializes_locale_and_bibliography() {
        use super::{DocumentBibliographyOverride, DocumentOptionsOverride};
        use citum_schema::options::scoped::BibliographyLabelWrap;

        let yaml = "locale: de-DE\nbibliography:\n  label-wrap: brackets\n";
        let parsed: DocumentOptionsOverride = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed.locale.as_deref(), Some("de-DE"));
        assert_eq!(
            parsed.bibliography,
            Some(DocumentBibliographyOverride {
                label_wrap: Some(BibliographyLabelWrap::Brackets),
                ..Default::default()
            })
        );
        assert!(parsed.integral_name_memory.is_none());
    }

    #[test]
    fn test_document_options_override_empty_is_default() {
        use super::DocumentOptionsOverride;

        let parsed: DocumentOptionsOverride = serde_yaml::from_str("{}").unwrap();
        assert_eq!(parsed, DocumentOptionsOverride::default());
    }

    #[test]
    fn test_document_options_override_applies_punctuation_width() {
        use super::DocumentOptionsOverride;
        use citum_schema::Style;
        use citum_schema::options::PunctuationWidth;

        let parsed: DocumentOptionsOverride =
            serde_yaml::from_str("multilingual:\n  punctuation-width: mixed\n").unwrap();
        let mut style: Style = serde_yaml::from_str("info: { id: test, title: Test }").unwrap();
        parsed.apply_bibliography_to(&mut style);
        assert_eq!(
            style
                .options
                .unwrap()
                .multilingual
                .unwrap()
                .punctuation_width,
            Some(PunctuationWidth::Mixed)
        );
    }

    #[test]
    fn test_document_options_override_unknown_field_errors() {
        use super::DocumentOptionsOverride;

        assert!(serde_yaml::from_str::<DocumentOptionsOverride>("unknown-field: true").is_err());
    }
}

/// Sparse per-document sort-partitioning overlay.
///
/// Absent fields inherit the style's existing sort-partitioning values.
/// When `by` is absent and the style has no sort-partitioning configured,
/// the override is a no-op.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DocumentSortPartitioningOverride {
    /// Partition key source. Required when no style-level partitioning exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<BibliographyPartitionKind>,
    /// Whether partitioning affects sorting, visible sections, or both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<BibliographyPartitionMode>,
    /// Preferred partition order. Unlisted partitions sort after listed ones.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order: Vec<String>,
    /// Optional headings for visible partition sections.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headings: HashMap<String, BibliographyPartitionHeading>,
}

/// Per-document bibliography presentation overrides.
///
/// All fields are optional. Absent fields inherit the style's defaults.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DocumentBibliographyOverride {
    /// Sparse multilingual bibliography partitioning override.
    ///
    /// Non-`None` fields are merged into the style's existing sort-partitioning;
    /// absent fields are inherited. When `by` is absent and the style has no
    /// sort-partitioning, the field is a no-op.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_partitioning: Option<DocumentSortPartitioningOverride>,
    /// Repeated-author rendering mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeated_author_rendering: Option<RepeatedAuthorRendering>,
    /// Bibliography label mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_mode: Option<BibliographyLabelMode>,
    /// Bibliography label wrap punctuation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_wrap: Option<BibliographyLabelWrap>,
}

/// Per-document presentation overrides parsed from frontmatter `options:` block.
///
/// Eligible options are those that control how output looks without requiring the
/// processor to re-walk citations for disambiguation or sorting. All fields are
/// optional; absent fields inherit the style's defaults.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DocumentOptionsOverride {
    /// BCP 47 locale ID to use as the base locale for this document.
    ///
    /// Replaces the style's default locale entirely. Handled in the CLI layer;
    /// `apply_to` does not touch locale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Integral-name-memory override. Takes precedence over the legacy top-level
    /// `integral-name-memory:` frontmatter field when both are present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integral_name_memory: Option<DocumentIntegralNameOverride>,
    /// Org-abbreviation-memory override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_abbreviation_memory: Option<DocumentOrgAbbreviationOverride>,
    /// Bibliography presentation overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<DocumentBibliographyOverride>,
    /// Multilingual presentation overrides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multilingual: Option<DocumentMultilingualOverride>,
}

impl DocumentOptionsOverride {
    /// Apply bibliography overrides from this override block to a resolved style.
    ///
    /// Writes non-`None` bibliography fields into `style.bibliography.options`,
    /// then calls `apply_scoped_options` for structural changes. Label mode and
    /// label wrapping remain runtime bibliography presentation settings. Locale
    /// and integral-name-memory are handled by the pipeline and CLI layers.
    pub(super) fn apply_bibliography_to(&self, style: &mut Style) {
        if let Some(multilingual) = &self.multilingual
            && let Some(width) = multilingual.punctuation_width
        {
            style
                .options
                .get_or_insert_with(Default::default)
                .multilingual
                .get_or_insert_with(Default::default)
                .punctuation_width = Some(width);
        }
        let Some(bib_override) = &self.bibliography else {
            return;
        };
        let bib = style.bibliography.get_or_insert_with(Default::default);
        let opts = bib.options.get_or_insert_with(Default::default);
        if let Some(sp_override) = &bib_override.sort_partitioning {
            match opts.sort_partitioning.as_mut() {
                Some(existing) => {
                    if let Some(by) = sp_override.by {
                        existing.by = by;
                    }
                    if let Some(mode) = sp_override.mode {
                        existing.mode = mode;
                    }
                    if !sp_override.order.is_empty() {
                        existing.order = sp_override.order.clone();
                    }
                    if !sp_override.headings.is_empty() {
                        existing.headings = sp_override.headings.clone();
                    }
                }
                None => {
                    if let Some(by) = sp_override.by {
                        opts.sort_partitioning = Some(BibliographySortPartitioning {
                            by,
                            mode: sp_override.mode.unwrap_or_default(),
                            order: sp_override.order.clone(),
                            headings: sp_override.headings.clone(),
                            unknown_fields: Default::default(),
                        });
                    }
                }
            }
        }
        if let Some(rar) = bib_override.repeated_author_rendering {
            opts.repeated_author_rendering = Some(rar);
        }
        if let Some(lm) = bib_override.label_mode {
            opts.label_mode = Some(lm);
        }
        if let Some(lw) = bib_override.label_wrap {
            opts.label_wrap = Some(lw);
        }
        style.apply_scoped_options();
    }
}

/// A citation marker parsed from a document.
#[derive(Debug, Clone)]
pub struct ParsedCitation {
    /// Byte offset where the citation marker starts in the source document.
    pub start: usize,
    /// Byte offset immediately after the citation marker in the source document.
    pub end: usize,
    /// The parsed citation payload and its items.
    pub citation: Citation,
    /// Where the citation was found in the source document.
    pub placement: CitationPlacement,
    /// Structural scope metadata for this citation location.
    pub structure: CitationStructure,
}

#[derive(Debug, Clone)]
pub(crate) struct ManualNoteReference {
    pub label: String,
    pub start: usize,
}

/// A bibliography block found in a document by a format-specific adapter.
///
/// Stores the byte range of the block in the source so the pipeline can
/// replace it with rendered output, plus the group descriptor that controls
/// which entries are selected and how they are headed.
#[derive(Debug, Clone)]
pub struct BibliographyBlock {
    /// Byte offset of the block's opening marker in source.
    pub start: usize,
    /// Byte offset past the block's closing marker in source.
    pub end: usize,
    /// The bibliography group for this block.
    pub group: BibliographyGroup,
}

/// Structured output from a document parser.
#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    /// Citation markers discovered in source order.
    pub citations: Vec<ParsedCitation>,
    /// Manual footnote labels in the order they appear in the document.
    pub manual_note_order: Vec<String>,
    pub(crate) manual_note_references: Vec<ManualNoteReference>,
    pub(crate) manual_note_labels: HashSet<String>,
    /// Bibliography blocks found in the document, in source order.
    pub bibliography_blocks: Vec<BibliographyBlock>,
    /// Bibliography groups from YAML frontmatter.
    pub frontmatter_groups: Option<Vec<citum_schema::grouping::BibliographyGroup>>,
    /// Integral-name-memory override from YAML frontmatter (legacy top-level field).
    pub frontmatter_integral_name_memory: Option<DocumentIntegralNameOverride>,
    /// Org-abbreviation-memory override from YAML frontmatter (legacy top-level field).
    ///
    /// Superseded by `options.org_abbreviation_memory` when both are present.
    pub frontmatter_org_abbreviation_memory: Option<DocumentOrgAbbreviationOverride>,
    /// Full options override from YAML frontmatter `options:` block.
    ///
    /// When `options.integral_name_memory` is `Some`, it takes precedence over
    /// the legacy top-level `frontmatter_integral_name_memory`.
    /// When `options.org_abbreviation_memory` is `Some`, it takes precedence over
    /// the legacy top-level `frontmatter_org_abbreviation_memory`.
    pub frontmatter_options: Option<DocumentOptionsOverride>,
    /// Non-empty when the frontmatter `---` block failed to deserialize.
    ///
    /// The pipeline treats this as a hard error: it prints the message and
    /// exits rather than proceeding silently without frontmatter data.
    pub frontmatter_error: Option<String>,
    /// Byte offset where the document body starts (past any frontmatter).
    pub body_start: usize,
}

/// A trait for document parsers that can identify citations.
///
/// Each implementation is a format-specific adapter (Djot, Markdown, etc.).
/// Adapters are responsible for source-syntax citation parsing, note discovery,
/// frontmatter extraction, bibliography block detection, and HTML finalization.
/// The shared pipeline consumes the resulting [`ParsedDocument`] without
/// inspecting format-specific internals.
pub trait CitationParser {
    /// Parse the document into citation placements and note metadata.
    fn parse_document(&self, content: &str, locale: &Locale) -> ParsedDocument;

    /// Finalize rendered document markup as HTML.
    ///
    /// Called after citation strings have been spliced back into the source
    /// markup and the result must be converted to HTML. The default
    /// implementation is a pass-through: it returns the markup unchanged.
    /// Format-specific adapters that require a markup-to-HTML conversion step
    /// (e.g. Djot via `jotdown`) must override this method.
    fn finalize_html_output(&self, rendered: &str) -> String {
        rendered.to_owned()
    }

    /// Convert raw document body markup (with NUL citation placeholder tokens)
    /// to the target output format.
    ///
    /// Called after citations have been replaced with NUL placeholder tokens.
    /// The default implementation returns the body unchanged (passthrough),
    /// which is correct for formats that have a downstream markup processor
    /// (HTML via `finalize_html_output`) or that emit verbatim markup (Plain,
    /// Djot, Markdown). Format-specific adapters that perform markup-to-target
    /// conversion for Typst/LaTeX must override this method.
    fn render_body_markup<F>(&self, body: &str, _fmt: &F) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        body.to_owned()
    }

    /// Find and extract citations from a document string.
    ///
    /// Returns a list of `(start_index, end_index, citation_model)` tuples.
    fn parse_citations(&self, content: &str, locale: &Locale) -> Vec<(usize, usize, Citation)> {
        self.parse_document(content, locale)
            .citations
            .into_iter()
            .map(|parsed| (parsed.start, parsed.end, parsed.citation))
            .collect()
    }
}

/// Document output format.
///
/// This governs the render target for citation strings and the generated
/// bibliography. It is independent of the document *input* format (Djot,
/// Markdown) and of in-field reference markup (Djot inline via
/// `render_djot_inline`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    /// Plain text (raw markup).
    Plain,
    /// Djot markup.
    Djot,
    /// Markdown markup.
    Markdown,
    /// HTML output.
    Html,
    /// LaTeX output.
    Latex,
    /// Typst output.
    Typst,
}
