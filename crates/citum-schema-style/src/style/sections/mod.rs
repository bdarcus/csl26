/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citation and bibliography style section specifications.

pub mod bibliography;
pub mod citation;

pub use bibliography::BibliographySpec;
pub use citation::{
    CitationCollapse, CitationSpec, NoteStartTextCase, SameAuthorCollapse, YearSuffixCollapse,
};
