/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Reference types for the CSLN processor.
//!
//! This module re-exports types from csln_core (for citations) and csl_legacy
//! (for CSL-JSON bibliography data) for backward compatibility.
//!
//! For new data, prefer using `csln_core::reference::InputReference` which
//! provides a more type-safe model with EDTF date support.

// Re-export citation types from csln_core
pub use csln_core::citation::{Citation, CitationItem, CitationMode, LocatorType};

// Re-export reference types from csln_core
pub use csln_core::reference::{
    Contributor, ContributorList, EdtfString, FlatName, InputReference as Reference,
    MultilingualString, NumOrStr, SimpleName, StructuredName, Title,
};

/// A bibliography is a collection of references keyed by ID.
pub type Bibliography = indexmap::IndexMap<String, Reference>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csl_json() {
        let json = r#"{
            "id": "kuhn1962",
            "type": "book",
            "author": [{"family": "Kuhn", "given": "Thomas S."}],
            "title": "The Structure of Scientific Revolutions",
            "issued": {"date-parts": [[1962]]},
            "publisher": "University of Chicago Press",
            "publisher-place": "Chicago"
        }"#;

        let legacy: csl_legacy::csl_json::Reference = serde_json::from_str(json).unwrap();
        let reference: Reference = legacy.into();
        assert_eq!(reference.id().unwrap(), "kuhn1962");
        assert_eq!(reference.ref_type(), "book");
        // No longer direct access to family on Contributor
        if let Some(Contributor::ContributorList(list)) = reference.author()
            && let Contributor::StructuredName(name) = &list.0[0]
        {
            assert_eq!(name.family, MultilingualString::Simple("Kuhn".to_string()));
        }
    }
}
