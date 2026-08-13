/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Interactive document-level API for batch citation formatting.
//!
//! This module provides the Tier 1 `format_document` API, enabling whole-document
//! citation and bibliography rendering with proper context (note positions, ibid
//! detection, disambiguation).

mod document;
pub mod forward_compat;
mod refs_input;
mod session;
mod style_input;
mod types;
mod warnings;

pub use document::{
    FormatDocumentError, FormatDocumentRequest, FormatDocumentResult, apply_style_overrides,
    format_document, format_document_with_resolver, format_document_with_style,
};
pub use forward_compat::{UnknownFieldPath, collect_unknown_field_paths};
pub use refs_input::RefsInput;
pub use session::{
    CitationInsertPosition, DocumentSession, DocumentSessionError, OpenSessionResult,
    PreviewCitationResult, SessionMutationResult,
};
pub use style_input::StyleInput;
pub use types::{
    AbbreviationMap, AnnotationFormat, AnnotationStyle, BibliographyBlockRequest,
    BibliographyEntry, CitationOccurrence, CitationOccurrenceItem, DocumentOptions, EntryMetadata,
    FormattedBibliography, FormattedBibliographyBlock, FormattedCitation, OutputFormatKind,
    Warning, WarningLevel,
};
pub use warnings::{
    bibliography_label_missing_separator_warnings, term_locale_fallback_warnings,
    unknown_enum_warnings, unknown_reference_class_warnings, unknown_reference_field_warnings,
};
