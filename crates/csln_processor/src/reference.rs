/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Reference types for the CSLN processor.
//!
//! This module re-exports the CSL-JSON reference model from `csl_legacy`
//! for backward compatibility with existing CSL-JSON data.
//!
//! For new data, prefer using `csln_core::reference::InputReference` which
//! provides a more type-safe model with EDTF date support.

// Re-export CSL-JSON types from csl_legacy for backward compatibility
pub use csl_legacy::csl_json::{
    Bibliography, Citation, CitationItem, DateVariable, Name, Reference, StringOrNumber,
};

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

        let reference: Reference = serde_json::from_str(json).unwrap();
        assert_eq!(reference.id, "kuhn1962");
        assert_eq!(reference.ref_type, "book");
        assert_eq!(
            reference.author.as_ref().unwrap()[0].family,
            Some("Kuhn".to_string())
        );
        assert_eq!(reference.issued.as_ref().unwrap().year_value(), Some(1962));
    }

    #[test]
    fn test_date_variable() {
        let date = DateVariable::year(2023);
        assert_eq!(date.year_value(), Some(2023));
        assert_eq!(date.month_value(), None);

        let date = DateVariable::year_month(2023, 6);
        assert_eq!(date.year_value(), Some(2023));
        assert_eq!(date.month_value(), Some(6));
    }
}
