/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Public schema types for Citum styles, citations, references, and locales.

/// Compatibility facade merging data input types with style-specific logic.
#[allow(missing_docs, reason = "internal derives")]
pub mod citation {
    pub use crate::locale::locator::normalize_locator_text;
    pub use citum_schema_data::citation::*;
}

/// Bibliographic reference data types.
pub use citum_schema_data::reference;

/// Bibliography grouping and sorting specifications.
pub mod grouping;
/// Locale-specific terms and translations.
pub mod locale;
/// Style configuration options.
#[allow(missing_docs, reason = "internal derives")]
pub mod options;
/// Configuration presets for common styles.
pub mod presets;
/// Style base-inheritance mechanism (named compiled-in Style structs).
pub mod style_base;
/// Citation and bibliography template components.
#[allow(missing_docs, reason = "internal derives")]
pub mod template;
/// Style schema version and resource-limit constants.
pub mod version;

/// Embedded templates for priority styles (APA, Chicago, Vancouver, IEEE, Harvard).
pub mod embedded;

/// Style registry — discovery and alias resolution.
pub mod registry;

/// Declarative macros for AST and configurations.
pub mod macros;

/// Lint helpers for raw locales and styles.
pub mod lint;

mod style;

#[cfg(test)]
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
mod reference_multilingual_tests;

pub use citation::{
    Citation, CitationItem, CitationMode, Citations, IntegralNameState, LocatorType, Position,
};
pub use citum_schema_data::{InputBibliography, InputBibliographyInfo};
pub use grouping::{
    BibliographyGroup, CitedStatus, FieldMatcher, GroupHeading, GroupSelector, GroupSort,
    GroupSortEntry, GroupSortKey, NameSortOrder, SortKey,
};
pub use locale::Locale;
pub use options::TextCase;
pub use options::{BibliographyOptions, CitationOptions, Config, ReferenceTypeName, TitleCategory};
pub use presets::{ContributorPreset, DatePreset, SortPreset, SubstitutePreset, TitlePreset};
pub use registry::{RegistryEntry, StyleRegistry};
pub use style::{
    BibliographySpec, CitationCollapse, CitationField, CitationSpec, NoteStartTextCase,
    SameAuthorCollapse, SchemaWarning, Style, StyleDocumentError, StyleDocumentFormat, StyleInfo,
    StyleLink, StylePerson, StyleSource, YearSuffixCollapse, check_citum_version,
};
pub use style_base::StyleBase;
pub use template::{
    ContributorLabelMode, ContributorMerge, ContributorMergeOrder, ContributorMergeRole,
    ContributorRoles, LocalizedTemplateSpec, LocalizedTemplateVariants, Rendering,
    ResolvedLocalizedTemplate, Template, TemplateAddOperation, TemplateComponent,
    TemplateComponentSelector, TemplateContributor, TemplateDate, TemplateGroup,
    TemplateIdentifier, TemplateModifyOperation, TemplateNumber, TemplatePreset, TemplateReference,
    TemplateRemoveOperation, TemplateTerm, TemplateTitle, TemplateVariable, TemplateVariant,
    TemplateVariantDiff, TemplateVariants, TypeSelector, VerticalAlign, WrapConfig,
    WrapPunctuation,
};
pub use version::*;

/// Canonical style resolution interfaces and error types.
pub use citum_resolver_api::{ResolutionError, ResolverError};

/// Resolver interface used by schema-layer style inheritance.
pub type StyleResolver = dyn citum_resolver_api::StyleResolver<Style = Style, Locale = Locale>;
