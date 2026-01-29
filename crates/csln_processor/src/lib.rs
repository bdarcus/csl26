/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSLN Processor
//!
//! This crate provides the core citation and bibliography processing functionality
//! for the Citation Style Language Next (CSLN) project. It takes style definitions,
//! bibliographic data, and citation information and produces formatted output.
//!
//! The processor is designed to be pluggable with different renderers and supports
//! advanced features like disambiguation, sorting, and localization.
//!
//! # Example
//!
//! ```rust
//! use csln_processor::{Processor, Reference, Bibliography, Citation, CitationItem, Name, DateVariable};
//! use csln_core::Style;
//! use std::collections::HashMap;
//!
//! // Create a simple style
//! let style_yaml = r#"
//! info:
//!   title: Simple
//! options:
//!   processing: author-date
//! citation:
//!   template:
//!     - contributor: author
//!       form: short
//!     - date: issued
//!       form: year
//! bibliography:
//!   template:
//!     - contributor: author
//!       form: long
//!     - date: issued
//!       form: year
//! "#;
//! let style: Style = serde_yaml::from_str(style_yaml).unwrap();
//!
//! // Create a bibliography
//! let mut bib = HashMap::new();
//! bib.insert("kuhn1962".to_string(), Reference {
//!     id: "kuhn1962".to_string(),
//!     ref_type: "book".to_string(),
//!     author: Some(vec![Name::new("Kuhn", "Thomas")]),
//!     title: Some("The Structure of Scientific Revolutions".to_string()),
//!     issued: Some(DateVariable::year(1962)),
//!     ..Default::default()
//! });
//!
//! // Create processor and render
//! let processor = Processor::new(style, bib);
//! let citation = Citation {
//!     id: Some("c1".to_string()),
//!     items: vec![CitationItem { id: "kuhn1962".to_string(), ..Default::default() }],
//! };
//! let result = processor.process_citation(&citation).unwrap();
//! assert_eq!(result, "(Kuhn, 1962)");
//! ```

pub mod error;
pub mod processor;
pub mod reference;
pub mod render;
pub mod values;

pub use error::ProcessorError;
pub use processor::{ProcessedReferences, Processor};
pub use reference::{
    Bibliography, Citation, CitationItem, DateVariable, Name, Reference, StringOrNumber,
};
pub use render::{citation_to_string, refs_to_string, ProcTemplate, ProcTemplateComponent};
pub use values::{ComponentValues, ProcHints, ProcValues, RenderContext, RenderOptions};

// Re-export Locale from csln_core for convenience
pub use csln_core::locale::Locale;
