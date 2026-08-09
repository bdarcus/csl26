/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citum Processor
//!
//! This crate provides the core citation and bibliography processing functionality
//! for the Citum project. It takes style definitions, bibliographic data, and
//! citation information and produces formatted output.
//!
//! The processor is designed to be pluggable with different renderers and supports
//! advanced features like disambiguation, sorting, and localization.
//!
//! For document-level integrations, use [`format_document`] with a
//! [`FormatDocumentRequest`]. The request carries a [`StyleInput`] and
//! [`RefsInput`], so callers can pass local style paths, inline style YAML, local
//! bibliography paths, inline bibliography YAML, or inline JSON references
//! without constructing a [`Processor`] directly.

//! # Example
//!
//! ```rust
//! use citum_engine::{
//!     Bibliography, Citation, CitationItem, CitationSpec, Config, Contributor,
//!     ContributorForm, ContributorList, ContributorRole, DateForm, DateValue,
//!     Monograph, MonographType, MultilingualString, Processing, Processor, Reference,
//!     Rendering, StructuredName, Style, StyleInfo, TemplateComponent, TemplateContributor,
//!     TemplateDate, TemplateDateVariable, Title, WrapPunctuation,
//! };
//!
//! // Create a simple style using native Citum types
//! let style = Style {
//!     info: StyleInfo {
//!         title: Some("Simple".to_string()),
//!         id: Some("simple".into()),
//!         ..Default::default()
//!     },
//!     options: Some(Config {
//!         processing: Some(Processing::AuthorDate),
//!         ..Default::default()
//!     }),
//!     citation: Some(CitationSpec {
//!         template: Some(vec![
//!             TemplateComponent::Contributor(TemplateContributor {
//!                 contributor: ContributorRole::Author.into(),
//!                 form: ContributorForm::Short,
//!                 rendering: Rendering::default(),
//!                 ..Default::default()
//!             }),
//!             TemplateComponent::Date(TemplateDate {
//!                 date: TemplateDateVariable::Issued,
//!                 form: DateForm::Year,
//!                 rendering: Rendering::default(),
//!                 ..Default::default()
//!             }),
//!         ].into()),
//!         wrap: Some(WrapPunctuation::Parentheses.into()),
//!         ..Default::default()
//!     }),
//!     ..Default::default()
//! };
//!
//! // Create a bibliography using native Citum reference data
//! let mut bib = Bibliography::new();
//! let reference = Reference::Monograph(Box::new(Monograph {
//!     id: Some("kuhn1962".into()),
//!     r#type: MonographType::Book,
//!     title: Some(Title::Single("The Structure of Scientific Revolutions".to_string())),
//!     author: Some(Contributor::ContributorList(ContributorList(vec![
//!         Contributor::StructuredName(StructuredName {
//!             family: MultilingualString::Simple("Kuhn".to_string()),
//!             given: MultilingualString::Simple("Thomas".to_string()),
//!             suffix: None,
//!             dropping_particle: None,
//!             non_dropping_particle: None,
//!         }),
//!     ]))),
//!     issued: DateValue::new("1962".to_string()),
//!     ..Default::default()
//! }));
//! bib.insert("kuhn1962".to_string(), reference);
//!
//! // Create processor and render
//! let processor = Processor::new(style, bib);
//! let citation = Citation {
//!     id: Some("c1".into()),
//!     items: vec![CitationItem { id: "kuhn1962".to_string(), ..Default::default() }],
//!     ..Default::default()
//! };
//! let result = processor.process_citation(&citation).unwrap();
//! assert_eq!(result, "(Kuhn, 1962)");
//! ```

/// Interactive document-level API for batch citation formatting.
pub mod api;
/// Error types returned by citation and bibliography processing.
pub mod error;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod grouping;
/// Citation, bibliography, sorting, and document processing logic.
pub mod processor;
pub mod reference;
/// Output-format renderers and string conversion helpers.
pub mod render;
mod sort_partitioning;
mod sort_support;
/// Bibliography and citation reference sorting.
pub mod sorting;
/// Template value resolution and formatting helpers.
pub mod values;

pub use api::{
    BibliographyBlockRequest, BibliographyEntry, CitationInsertPosition, CitationOccurrence,
    CitationOccurrenceItem, DocumentOptions, DocumentSession, DocumentSessionError, EntryMetadata,
    FormatDocumentError, FormatDocumentRequest, FormatDocumentResult, FormattedBibliography,
    FormattedBibliographyBlock, FormattedCitation, OpenSessionResult, OutputFormatKind,
    PreviewCitationResult, RefsInput, SessionMutationResult, StyleInput, Warning, WarningLevel,
    apply_style_overrides, format_document, format_document_with_resolver,
    format_document_with_style,
};
pub use citum_schema::options::{Config, Processing};
pub use citum_schema::reference::{
    Contributor, ContributorList, DateValue, Monograph, MonographType, MultilingualString,
    StructuredName, Title,
};
pub use citum_schema::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable as TemplateDateVariable, Rendering,
    TemplateComponent, TemplateContributor, TemplateDate, WrapPunctuation,
};
pub use citum_schema::{CitationSpec, Style, StyleInfo};
pub use error::ProcessorError;
pub use processor::document::DocumentFormat;
pub use processor::{ProcessedReferences, Processor};
pub use reference::{Bibliography, Citation, CitationItem, Reference};
pub use render::{ProcTemplate, ProcTemplateComponent, citation_to_string, refs_to_string};
pub use sorting::ReferenceSorter;
pub use values::{ComponentValues, ProcHints, ProcValues, RenderContext, RenderOptions};

// Re-export Locale from citum_schema for convenience
pub use citum_schema::locale::Locale;
