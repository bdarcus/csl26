/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use std::borrow::Cow;
use thiserror::Error;

/// Error type for style resolution operations at the store layer.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ResolverError {
    /// An underlying I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// The style content is malformed or invalid for the target version.
    #[error("invalid style: {0}")]
    InvalidStyle(Cow<'static, str>),
    /// The requested style could not be located.
    #[error("style not found: {0}")]
    StyleNotFound(Cow<'static, str>),
    /// The requested locale could not be located.
    #[error("locale not found: {0}")]
    LocaleNotFound(Cow<'static, str>),
    /// Failure during YAML deserialization.
    #[error("yaml error: {0}")]
    YamlError(String),
    /// Failure during JSON deserialization.
    #[cfg(feature = "serde_json")]
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// Failure during CBOR deserialization.
    #[error("cbor error: {0}")]
    CborError(String),
    /// A network failure occurred during HTTP fetch.
    #[cfg(feature = "http")]
    #[error("http error: {0}")]
    HttpError(String),
    /// A failure occurred during a Git operation.
    #[cfg(feature = "http")]
    #[error("git error: {0}")]
    GitError(String),
    /// The URI host or origin is not in the resolver's allowlist.
    #[error("host not in resolver allowlist: {uri} ({reason})")]
    Denied {
        /// The URI that was denied.
        uri: String,
        /// Reason for the denial.
        reason: String,
    },
    /// The style's `citum-version` is not compatible with the running engine.
    #[error(
        "engine version mismatch for {uri}: engine requires {required}, style declares {declared}"
    )]
    VersionMismatch {
        /// URI of the style with the incompatible version.
        uri: String,
        /// Version requirement declared by the style.
        required: String,
        /// Version of the running engine.
        declared: String,
    },
    /// The content does not match the expected integrity hash.
    #[error("integrity failure for {uri}: expected {expected}, got {actual}")]
    IntegrityFailure {
        /// URI of the content that failed verification.
        uri: String,
        /// The expected CIDv1 string.
        expected: String,
        /// The CID computed from the actual content bytes.
        actual: String,
    },
    /// Generic network or transport failure.
    #[error("network error fetching {uri}: {reason}")]
    NetworkError {
        /// URI of the failed fetch.
        uri: String,
        /// Reason for the network failure.
        reason: String,
    },
}

/// Error type for style resolution and inheritance processing at the schema layer.
#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ResolutionError {
    /// A `profile` style attempted to override template-bearing structure.
    #[error("profile styles may not override template-bearing field `{location}`")]
    InvalidProfileOverride {
        /// Human-readable location hint.
        location: String,
    },
    /// An inheritance loop was detected.
    #[error("inheritance loop detected at base `{base}`")]
    InheritanceLoop {
        /// Base key that closed the cycle.
        base: String,
    },
    /// A `file://` URI could not be resolved.
    #[error("failed to resolve URI `{uri}`: {reason}")]
    UriResolutionFailed {
        /// The URI that failed to resolve.
        uri: String,
        /// Reason for failure.
        reason: String,
    },
    /// A Template V3 variant references a missing parent variant.
    #[error("template variant `{location}` extends missing variant `{selector}`")]
    MissingTemplateVariantParent {
        /// Human-readable location hint.
        location: String,
        /// Parent selector that could not be found.
        selector: String,
    },
    /// A Template V3 variant parent chain contains a cycle.
    #[error("template variant inheritance loop at `{location}` through `{selector}`")]
    TemplateVariantCycle {
        /// Human-readable location hint.
        location: String,
        /// Selector that closed the cycle.
        selector: String,
    },
    /// A Template V3 operation matched no components.
    #[error("template variant operation in `{location}` matched no component")]
    TemplateVariantAnchorNotFound {
        /// Human-readable location hint.
        location: String,
    },
    /// A Template V3 operation matched more than one component.
    #[error("template variant operation in `{location}` matched multiple components")]
    TemplateVariantAmbiguousAnchor {
        /// Human-readable location hint.
        location: String,
    },
    /// A Template V3 add operation does not define exactly one anchor.
    #[error(
        "template variant add operation in `{location}` must specify exactly one of before/after"
    )]
    InvalidTemplateVariantAdd {
        /// Human-readable location hint.
        location: String,
    },
    /// A fallback-template diff violates its inheritance contract.
    #[error("invalid fallback-template diff in `{location}`: {reason}")]
    InvalidFallbackTemplateDiff {
        /// Style-aware location of the invalid fallback template.
        location: String,
        /// Reason the fallback diff cannot be resolved.
        reason: String,
    },
    /// The fetched parent style's content did not hash to the value declared
    /// in `extends-pin`.
    #[error("extends-pin integrity check failed for `{uri}`: expected {expected}, got {actual}")]
    IntegrityFailure {
        /// URI of the parent that failed integrity verification.
        uri: String,
        /// CID declared in the child's `extends-pin`.
        expected: String,
        /// CID computed from the bytes the resolver actually returned.
        actual: String,
    },
    /// The fetched style declares a `citum-version` requirement that the
    /// running engine does not satisfy.
    #[error("style `{uri}` requires citum-version `{required}`; running engine is `{declared}`")]
    VersionMismatch {
        /// URI whose `citum-version` requirement was unsatisfiable.
        uri: String,
        /// `citum-version` requirement declared by the style's `info` block.
        required: String,
        /// Version of the running engine.
        declared: String,
    },
    /// A `citation.collapse` value is not licensed for the style's processing
    /// regime, e.g. `same-author` on a `Numeric` style or `citation-number`
    /// on an `AuthorDate` style. See `docs/specs/SAME_AUTHOR_COLLAPSE.md` §6.
    #[error("`{collapse}` collapse is not valid for `{processing}` processing in `{location}`")]
    IncoherentCollapseRegime {
        /// Human-readable location hint (e.g. `"citation.collapse"`).
        location: String,
        /// The declared collapse value, formatted for the error message.
        collapse: String,
        /// The style's resolved processing regime, formatted for the error message.
        processing: String,
    },
}

impl ResolutionError {
    /// Convert a [`ResolverError`] into a [`ResolutionError`] for a specific URI.
    #[must_use]
    pub fn from_resolver_error(uri: &str, err: ResolverError) -> Self {
        match err {
            ResolverError::IntegrityFailure {
                expected, actual, ..
            } => ResolutionError::IntegrityFailure {
                uri: uri.into(),
                expected,
                actual,
            },
            ResolverError::VersionMismatch {
                required, declared, ..
            } => ResolutionError::VersionMismatch {
                uri: uri.into(),
                required,
                declared,
            },
            _ => ResolutionError::UriResolutionFailed {
                uri: uri.into(),
                reason: err.to_string(),
            },
        }
    }
}
