/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! A reference is a bibliographic item, such as a book, article, or web page.
//! It is the basic unit of bibliographic data.

pub mod citeproc_markup;
pub mod contributor;
#[cfg(feature = "legacy-convert")]
pub mod conversion;
pub mod date;
pub mod types;

mod accessors;
mod classes;
mod ctors;
mod input;
mod serde_impl;

#[cfg(all(test, feature = "legacy-convert"))]
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
mod tests;

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
mod discriminator_tests;

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
mod normalize_tests;

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
mod numbering_tests;

pub use self::classes::{ClassExtension, ReferenceClass};
pub use self::contributor::{
    Contributor, ContributorEntry, ContributorGender, ContributorList, ContributorRole, FlatName,
    SimpleName, StructuredName,
};
pub use self::date::DateValue;
pub(crate) use self::input::EMPTY_FIELD_LANGUAGES;
pub use self::input::{IdentifierName, InputReference, SupplementaryIdentifiers, UnknownClassData};
pub use self::types::common::{
    FieldLanguageMap, LangID, MultilingualString, NumOrStr, Numbering, NumberingType, Place,
    Publisher, RefID, RichText, Title,
};
pub use self::types::legal::{Brief, Hearing, LegalCase, Regulation, Statute, Treaty};
pub use self::types::specialized::{
    AudioVisualType, AudioVisualWork, Classic, Dataset, Event, Patent, Software, Standard, WorkCore,
};
pub use self::types::structural::{
    Collection, CollectionComponent, CollectionType, Monograph, MonographComponentType,
    MonographType, Serial, SerialComponent, SerialComponentType, SerialType,
};

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "bindings")]
use specta::Type;

/// A relation to another bibliographic entity.
///
/// Untagged in serde to allow either an inline object or a string ID reference.
/// Used for both hierarchical (`container`) and associative (`original`, `reviewed`, `series`) links.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[cfg_attr(feature = "bindings", derive(Type))]
#[serde(untagged)]
pub enum WorkRelation {
    /// The target work is referenced by its ID (resolved at render time).
    Id(RefID),
    /// The target work is embedded inline.
    Embedded(Box<InputReference>),
}
