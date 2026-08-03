/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! The Citum style model.

use std::collections::HashMap;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};

#[allow(unused_imports, reason = "Referenced by intra-doc links.")]
use crate::ResolutionError;
use crate::style_base;
use crate::{BibliographySpec, CitationSpec, Config, SchemaVersion, StyleInfo, Template};

/// The new Citum Style model.
///
/// This is the target schema for Citum, featuring declarative options
/// and simple template components instead of procedural conditionals.
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub struct Style {
    /// Style schema version.
    #[serde(default)]
    pub version: SchemaVersion,
    /// Style metadata.
    #[serde(default)]
    pub info: StyleInfo,
    /// Named reusable templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<HashMap<String, Template>>,
    /// Global style options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Config>,
    /// Citation specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<CitationSpec>,
    /// Bibliography specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bibliography: Option<BibliographySpec>,
    /// Custom user-defined fields for extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
    /// Extends a base style, with optional local overrides.
    ///
    /// When present, the base [`StyleReference`](style_base::StyleReference) is resolved and the local
    /// overrides are merged before any further processing. Explicit `options`,
    /// `citation`, and `bibliography` keys at the same document level take
    /// precedence over the resolved base.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<style_base::StyleReference>,
    /// Optional content-addressed integrity pin for the parent style referenced
    /// by [`extends`](Self::extends).
    ///
    /// When present, the resolver verifies that the SHA-256 of the fetched
    /// parent matches this CIDv1 string before merging. Mismatches abort
    /// resolution with [`ResolutionError::IntegrityFailure`]. Absent means
    /// "no integrity check" — appropriate for `file://` parents under user
    /// control or trusted local registries.
    #[serde(rename = "extends-pin", skip_serializing_if = "Option::is_none")]
    pub extends_pin: Option<String>,
    /// Raw YAML captured when the style was loaded via [`Style::from_yaml_str`]
    /// or [`Style::from_yaml_bytes`]. Used during style resolution for
    /// null-aware overlay merging (e.g., `ibid: ~` correctly clears an
    /// inherited preset value). Absent in programmatically-constructed styles.
    #[cfg_attr(feature = "schema", schemars(skip))]
    #[serde(skip, default)]
    pub raw_yaml: Option<serde_yaml::Value>,
    /// Chain-merged authored `citation.options` / `bibliography.options`
    /// mappings, captured at parse time and maintained through `extends`
    /// resolution. Basis for the runtime scope cascade's field-level merge
    /// (see [`crate::options::cascade::ScopedRawOptions`]). Empty in
    /// programmatically-constructed styles, which fall back to the typed
    /// whole-block merge.
    ///
    /// Public like [`Self::raw_yaml`] and [`Self::unknown_fields`]: other
    /// workspace crates construct `Style` via `Style { .. } .. Default::default()`,
    /// which requires every field visible at the call site, so a `pub(crate)`
    /// field would break those construction sites rather than only external
    /// struct-literal callers.
    #[cfg_attr(feature = "schema", schemars(skip))]
    #[serde(skip, default)]
    pub scoped_raw_options: crate::options::cascade::ScopedRawOptions,
    /// Forward-compat: captures unknown keys when an older engine reads a
    /// style produced by a newer schema. Empty by default; treated as a
    /// SoftDegrade signal. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[serde(
        flatten,
        default,
        skip_serializing_if = "std::collections::BTreeMap::is_empty"
    )]
    #[cfg_attr(feature = "schema", schemars(skip))]
    pub unknown_fields: std::collections::BTreeMap<String, serde_yaml::Value>,
}

impl Style {
    /// Parse a Citum style from a YAML string, preserving raw YAML for
    /// null-aware overlay merging during base resolution.
    ///
    /// Preferred over `serde_yaml::from_str` when the style extends a base,
    /// so that `ibid: ~` and similar null overrides correctly clear inherited values.
    ///
    /// # Errors
    ///
    /// Returns a serde error if YAML parsing or deserialization fails.
    pub fn from_yaml_str(s: &str) -> Result<Self, serde_yaml::Error> {
        let raw: serde_yaml::Value = serde_yaml::from_str(s)?;
        Self::from_raw_value(raw).map_err(StyleDocumentError::into_yaml_error)
    }

    /// Apply scoped citation and bibliography option overrides to this style.
    ///
    /// Applies structural scoped options such as group delimiters, date position,
    /// title terminators, and repeated-author rendering. Label mode and label wrap
    /// remain runtime presentation settings and do not mutate authored templates.
    pub fn apply_scoped_options(&mut self) {
        crate::options::scoped::apply_scoped_style_options(self);
    }

    /// Merge a partial overlay style over this style in place; overlay fields win.
    ///
    /// Overlay merging is typed and matches `extends` inheritance for the fields it supports:
    /// - `info`, `templates`, `options`, and `custom` are merged (overlay wins for `Some` fields / keys).
    /// - `citation` / `bibliography` are deep-merged; explicit YAML `~` can clear inherited fields when
    ///   `overlay.raw_yaml` is populated (e.g. via `Style::from_yaml_bytes`).
    ///
    /// The caller is responsible for calling [`apply_scoped_options`](Self::apply_scoped_options)
    /// afterwards if structural scoped-option side-effects (date position, title
    /// terminator, etc.) are needed.
    pub fn apply_overlay(&mut self, overlay: &Style) {
        super::overlay::merge_style_overlay(self, overlay);
    }

    /// Parse a Citum style from YAML bytes, preserving raw YAML for
    /// null-aware overlay merging during preset resolution.
    ///
    /// # Errors
    ///
    /// Returns a serde error if YAML parsing or deserialization fails.
    pub fn from_yaml_bytes(bytes: &[u8]) -> Result<Self, serde_yaml::Error> {
        let raw: serde_yaml::Value = serde_yaml::from_slice(bytes)?;
        Self::from_raw_value(raw).map_err(StyleDocumentError::into_yaml_error)
    }

    /// Parse a Citum style from bytes in any [`StyleDocumentFormat`], preserving
    /// a format-neutral raw value tree for null-aware overlay merging.
    ///
    /// This is the canonical entry point for every style load path — file,
    /// store, registry, CLI conversion, and server resolution — so that
    /// explicit-`null` inherited-field clearing (see [`Style::apply_overlay`])
    /// behaves identically regardless of load path or wire format. JSON and
    /// YAML documents parse directly into the same generic value tree used by
    /// [`Style::from_yaml_bytes`]; CBOR documents are decoded the same way but
    /// are rejected if any map uses a non-string key, since the raw-tree
    /// presence lookups used by overlay merging key on string field names.
    ///
    /// # Errors
    ///
    /// Returns [`StyleDocumentError`] if the bytes cannot be decoded in the
    /// requested format, if a CBOR document contains a non-string map key, or
    /// if the decoded style fails schema or resource-limit validation.
    pub fn from_document_bytes(
        bytes: &[u8],
        format: StyleDocumentFormat,
    ) -> Result<Self, StyleDocumentError> {
        let raw: serde_yaml::Value = match format {
            StyleDocumentFormat::Yaml => serde_yaml::from_slice(bytes)?,
            StyleDocumentFormat::Json => serde_json::from_slice(bytes)?,
            StyleDocumentFormat::Cbor => {
                let raw: serde_yaml::Value = ciborium::de::from_reader(bytes)
                    .map_err(|e| StyleDocumentError::Cbor(e.to_string()))?;
                reject_non_string_keys(&raw).map_err(StyleDocumentError::Cbor)?;
                raw
            }
        };
        Self::from_raw_value(raw)
    }

    /// Shared tail of [`Style::from_yaml_str`], [`Style::from_yaml_bytes`], and
    /// [`Style::from_document_bytes`]: validate the raw tree, deserialize the
    /// typed style from it, stamp `raw_yaml`, then validate resource limits.
    ///
    /// The `serde_yaml::from_value` step always uses the real
    /// [`StyleDocumentError::Yaml`] variant (never collapsed to a string),
    /// regardless of which wire format the raw tree originated from — the
    /// tree is already unified into `serde_yaml::Value` by the time this
    /// runs, so this deserialize step is always a `serde_yaml` operation.
    fn from_raw_value(raw: serde_yaml::Value) -> Result<Self, StyleDocumentError> {
        super::diagnostics::validate_raw_style(&raw).map_err(StyleDocumentError::Validation)?;
        let mut style: Style = serde_yaml::from_value(raw.clone())?;
        style.raw_yaml = Some(raw);
        style.scoped_raw_options = crate::options::cascade::ScopedRawOptions::capture(&style);
        style
            .validate_resource_limits()
            .map_err(StyleDocumentError::Validation)?;
        Ok(style)
    }
}

/// Serialization format of a raw style document, used by
/// [`Style::from_document_bytes`] to select the right decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleDocumentFormat {
    /// YAML document.
    Yaml,
    /// JSON document.
    Json,
    /// CBOR document. Only string-keyed maps are supported.
    Cbor,
}

/// Error parsing a style document in any [`StyleDocumentFormat`].
#[derive(Debug)]
pub enum StyleDocumentError {
    /// Failure decoding a YAML document, or deserializing the typed [`Style`]
    /// from the generic raw tree — which applies regardless of whether that
    /// tree originated from YAML, JSON, or CBOR, since the tree is always
    /// unified into `serde_yaml::Value` before this step runs.
    Yaml(serde_yaml::Error),
    /// Failure decoding a JSON document.
    Json(serde_json::Error),
    /// Failure decoding a CBOR document, or a non-string map key was found.
    Cbor(String),
    /// The decoded style failed schema or resource-limit validation.
    Validation(String),
}

impl std::fmt::Display for StyleDocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StyleDocumentError::Yaml(e) => write!(f, "yaml error: {e}"),
            StyleDocumentError::Json(e) => write!(f, "json error: {e}"),
            StyleDocumentError::Cbor(e) => write!(f, "cbor error: {e}"),
            StyleDocumentError::Validation(e) => write!(f, "invalid style: {e}"),
        }
    }
}

impl std::error::Error for StyleDocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StyleDocumentError::Yaml(e) => Some(e),
            StyleDocumentError::Json(e) => Some(e),
            StyleDocumentError::Cbor(_) | StyleDocumentError::Validation(_) => None,
        }
    }
}

impl From<serde_yaml::Error> for StyleDocumentError {
    fn from(e: serde_yaml::Error) -> Self {
        StyleDocumentError::Yaml(e)
    }
}

impl From<serde_json::Error> for StyleDocumentError {
    fn from(e: serde_json::Error) -> Self {
        StyleDocumentError::Json(e)
    }
}

impl StyleDocumentError {
    /// Convert into a `serde_yaml::Error` for callers with a YAML-only
    /// public signature ([`Style::from_yaml_str`], [`Style::from_yaml_bytes`]).
    ///
    /// The `Yaml` variant unwraps directly, preserving the original error
    /// and its source chain. `Json`/`Cbor` cannot structurally occur on
    /// those callers' paths (they never decode JSON or CBOR), so those arms
    /// only exist to keep this conversion total; `Validation` has no
    /// underlying serde error to preserve, so it round-trips through
    /// [`serde::de::Error::custom`].
    fn into_yaml_error(self) -> serde_yaml::Error {
        match self {
            StyleDocumentError::Yaml(e) => e,
            StyleDocumentError::Json(e) => serde_yaml::Error::custom(e),
            StyleDocumentError::Cbor(msg) | StyleDocumentError::Validation(msg) => {
                serde_yaml::Error::custom(msg)
            }
        }
    }
}

/// Reject a raw value tree containing a mapping keyed by anything other than
/// a string, recursively. CBOR permits non-string map keys; the overlay
/// null-clear lookups in `style/overlay.rs` key on string field names, so a
/// non-string-keyed map would silently fail to match rather than error.
fn reject_non_string_keys(value: &serde_yaml::Value) -> Result<(), String> {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (key, val) in map {
                if !matches!(key, serde_yaml::Value::String(_)) {
                    return Err(format!(
                        "CBOR style document uses a non-string map key ({key:?}); \
                         only string-keyed maps are supported"
                    ));
                }
                reject_non_string_keys(val)?;
            }
            Ok(())
        }
        serde_yaml::Value::Sequence(seq) => seq.iter().try_for_each(reject_non_string_keys),
        serde_yaml::Value::Tagged(tagged) => reject_non_string_keys(&tagged.value),
        _ => Ok(()),
    }
}
