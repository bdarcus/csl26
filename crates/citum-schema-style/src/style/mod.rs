/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style model, metadata, validation, and inheritance internals.

mod diagnostics;
mod metadata;
mod model;
pub(crate) mod overlay;
mod resolution;
mod sections;
mod validation;

pub use metadata::{CitationField, StyleInfo, StyleLink, StylePerson, StyleSource};
pub use model::{Style, StyleDocumentError, StyleDocumentFormat};
pub use resolution::check_citum_version;
pub use sections::{
    BibliographySpec, CitationCollapse, CitationSpec, NoteStartTextCase, SameAuthorCollapse,
    YearSuffixCollapse,
};
pub use validation::SchemaWarning;
