/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style inheritance and resolver integration.

use std::collections::HashSet;

use crate::{ResolutionError, StyleInfo, StyleResolver, options, registry, style_base};

use super::Style;
use super::overlay::merge_style_overlay;

impl Style {
    /// Resolve this style into its final effective form by applying base inheritance.
    ///
    /// If the `extends` field is present, the base [`StyleBase`](style_base::StyleBase) is loaded
    /// and any explicit `options`, `citation`, or `bibliography` keys in the current
    /// style document are merged on top (taking ultimate precedence).
    ///
    /// Styles without a base still resolve Template V3 variants and scoped
    /// options, but do not merge any inherited style data.
    ///
    /// # Panics
    ///
    /// Panics when style resolution fails. Use [`Style::try_into_resolved`]
    /// to handle profile-contract and inheritance errors explicitly.
    #[must_use]
    #[allow(
        clippy::panic,
        reason = "Convenience API for infallible resolution contexts"
    )]
    pub fn into_resolved(self) -> Self {
        self.try_into_resolved()
            .unwrap_or_else(|err| panic!("style resolution failed: {err}"))
    }

    /// Resolve this style into its final effective form, returning validation errors.
    ///
    /// Unlike [`Style::into_resolved`], this preserves resolution failures as
    /// structured [`ResolutionError`] values.
    ///
    /// # Errors
    ///
    /// Returns an error when profile wrappers try to override template-bearing
    /// structure, when profile capability validation fails, or when inheritance
    /// loops are detected.
    pub fn try_into_resolved(self) -> Result<Self, ResolutionError> {
        self.try_into_resolved_with(None)
    }

    /// Resolve this style into its final effective form using an optional style resolver.
    ///
    /// The resolver is used for URI, registry, and remote `extends` references
    /// that cannot be satisfied by embedded bases alone.
    ///
    /// # Errors
    ///
    /// Returns an error when style inheritance or Template V3 variant
    /// resolution fails.
    pub fn try_into_resolved_with(
        self,
        resolver: Option<&StyleResolver>,
    ) -> Result<Self, ResolutionError> {
        self.try_into_resolved_recursive_with_depth(resolver, &mut HashSet::new(), 0)
    }

    /// Internal recursive resolver with loop protection.
    ///
    /// # Panics
    ///
    /// Panics when style resolution fails. Use
    /// [`Style::try_into_resolved_recursive`] to preserve errors.
    #[must_use]
    #[allow(
        clippy::panic,
        reason = "Convenience API for infallible resolution contexts"
    )]
    pub fn into_resolved_recursive(self, visited: &mut HashSet<String>) -> Self {
        self.try_into_resolved_recursive(visited)
            .unwrap_or_else(|err| panic!("style resolution failed: {err}"))
    }

    /// Internal recursive resolver with loop protection.
    ///
    /// # Errors
    ///
    /// Returns an error when profile wrappers violate the config-only
    /// contract, when profile capability validation fails, or when
    /// inheritance loops are detected.
    pub fn try_into_resolved_recursive(
        self,
        visited: &mut HashSet<String>,
    ) -> Result<Self, ResolutionError> {
        self.try_into_resolved_recursive_with(None, visited)
    }

    /// Internal recursive resolver with loop protection and optional external resolver.
    ///
    /// # Errors
    ///
    /// Returns an error when style inheritance or Template V3 variant
    /// resolution fails.
    pub fn try_into_resolved_recursive_with(
        self,
        resolver: Option<&StyleResolver>,
        visited: &mut HashSet<String>,
    ) -> Result<Self, ResolutionError> {
        self.try_into_resolved_recursive_with_depth(resolver, visited, 0)
    }

    /// Internal recursive resolver with depth limit.
    fn try_into_resolved_recursive_with_depth(
        self,
        resolver: Option<&StyleResolver>,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Result<Self, ResolutionError> {
        const MAX_DEPTH: usize = 5;

        // Reject root styles whose declared engine compat range we don't satisfy
        // before we waste any work on inheritance or template variants.
        let root_label = self
            .info
            .id
            .as_deref()
            .or(self.info.title.as_deref())
            .unwrap_or("<root>");
        check_citum_version(root_label, &self.info)?;

        let Some(base_ref) = self.extends.clone() else {
            let mut style = self;
            crate::template::resolve_style_template_variants(&mut style, None)?;
            options::scoped::apply_scoped_style_options(&mut style);
            style.validate_collapse_regime()?;
            return Ok(style);
        };

        if depth >= MAX_DEPTH {
            let uri = base_ref.key();
            return Err(ResolutionError::UriResolutionFailed {
                uri: uri.to_string(),
                reason: format!("inheritance chain exceeds maximum depth of {MAX_DEPTH}"),
            });
        }

        let key = base_ref.key().to_string();
        if visited.contains(&key) {
            return Err(ResolutionError::InheritanceLoop { base: key });
        }
        visited.insert(key);

        let is_profile = self.resolves_as_profile();
        let pin = self.extends_pin.clone();
        let mut effective = match base_ref {
            style_base::StyleReference::Base(base) => {
                if pin.is_some() {
                    return Err(ResolutionError::UriResolutionFailed {
                        uri: base.key().to_string(),
                        reason:
                            "extends-pin is only supported for URI-based parents (https://, cid:); \
                         builtin StyleBase parents are content-fixed already"
                                .to_string(),
                    });
                }
                base.try_resolve_with_visited(resolver, visited)?
            }
            style_base::StyleReference::Uri(ref uri) => {
                let base_style = resolve_style_reference_uri(uri, resolver)?;
                if let Some(ref expected) = pin {
                    verify_parent_pin(uri, &base_style, expected)?;
                }
                base_style.try_into_resolved_recursive_with_depth(resolver, visited, depth + 1)?
            }
        };
        if is_profile {
            self.validate_profile_shape()?;
        }

        let inherited_variants = crate::template::inherited_variant_context(&effective);
        merge_style_overlay(&mut effective, &self);
        effective.scoped_raw_options =
            std::mem::take(&mut effective.scoped_raw_options).merged_with_child(&self);
        crate::template::resolve_style_template_variants_with_overlay(
            &mut effective,
            inherited_variants.as_ref(),
            &self,
        )?;
        effective.version = self.version;
        effective.extends = self.extends;
        effective.extends_pin = self.extends_pin;
        effective.raw_yaml = self.raw_yaml;
        options::scoped::apply_scoped_style_options(&mut effective);
        if is_profile {
            effective.extends = None;
        }
        effective.validate_collapse_regime()?;

        Ok(effective)
    }
    fn style_kind(&self) -> Option<registry::StyleKind> {
        let id = self.info.id.as_deref()?;
        registry::StyleRegistry::load_default()
            .resolve(id)
            .and_then(|entry| entry.kind.clone())
    }

    fn resolves_as_profile(&self) -> bool {
        self.style_kind() == Some(registry::StyleKind::Profile)
    }
}

#[allow(
    clippy::panic,
    reason = "Multihash::wrap on a 32-byte SHA-256 digest is infallible by construction"
)]
fn schema_compute_style_cid(bytes: &[u8]) -> String {
    use cid::Cid;
    use multihash::Multihash;
    use sha2::{Digest, Sha256};

    const RAW_CODEC: u64 = 0x55;
    const SHA256_CODE: u64 = 0x12;

    let digest: [u8; 32] = Sha256::digest(bytes).into();
    let mh = Multihash::<64>::wrap(SHA256_CODE, &digest)
        .unwrap_or_else(|_| panic!("32-byte SHA-256 digest fits in Multihash<64>"));
    Cid::new_v1(RAW_CODEC, mh).to_string()
}

/// Normalize a CID-or-`cid:`-URI to its canonical lowercase string form.
fn schema_canonicalize_cid(s: &str) -> Result<String, ResolutionError> {
    use cid::Cid;
    let trimmed = s.strip_prefix("cid:").unwrap_or(s);
    let cid: Cid =
        trimmed
            .parse()
            .map_err(|err: cid::Error| ResolutionError::UriResolutionFailed {
                uri: s.to_string(),
                reason: format!("invalid CID '{s}': {err}"),
            })?;
    Ok(cid.to_string())
}

/// Verify that the parent style's serialized form matches `expected_pin`.
///
/// Re-serializes the parsed `Style` to its canonical YAML form and computes
/// its CIDv1. Mismatch produces [`ResolutionError::IntegrityFailure`]. This
/// is a best-effort check at the schema layer; for byte-exact verification
/// of the originally-fetched bytes, route through
/// `citum_store::fetch_and_verify_bytes` before parsing.
fn verify_parent_pin(uri: &str, parent: &Style, expected_pin: &str) -> Result<(), ResolutionError> {
    let expected = schema_canonicalize_cid(expected_pin)?;
    let bytes =
        serde_yaml::to_string(parent).map_err(|err| ResolutionError::UriResolutionFailed {
            uri: uri.to_string(),
            reason: format!("re-serialize for extends-pin verification: {err}"),
        })?;
    let actual = schema_compute_style_cid(bytes.as_bytes());
    if actual == expected {
        Ok(())
    } else {
        Err(ResolutionError::IntegrityFailure {
            uri: uri.to_string(),
            expected,
            actual,
        })
    }
}

/// Apply an `info.citum-version` requirement check against the running
/// engine version (CARGO_PKG_VERSION).
///
/// Returns `Ok(())` when the field is absent or the requirement is satisfied;
/// returns [`ResolutionError::VersionMismatch`] otherwise.
///
/// # Errors
///
/// Returns [`ResolutionError::VersionMismatch`] when the running engine
/// version does not satisfy the style's declared `citum-version` requirement.
/// Returns [`ResolutionError::UriResolutionFailed`] when the requirement
/// string itself fails to parse as a semver `VersionReq`, or when the
/// running engine's `CARGO_PKG_VERSION` cannot be parsed (this latter case
/// is structurally impossible at runtime but is preserved as a typed error
/// rather than a panic).
pub fn check_citum_version(uri: &str, info: &StyleInfo) -> Result<(), ResolutionError> {
    let Some(req_str) = info.citum_version.as_ref() else {
        return Ok(());
    };
    let req =
        semver::VersionReq::parse(req_str).map_err(|err| ResolutionError::UriResolutionFailed {
            uri: uri.to_string(),
            reason: format!("invalid `info.citum-version` requirement '{req_str}': {err}"),
        })?;
    let engine_str = env!("CARGO_PKG_VERSION");
    let engine =
        semver::Version::parse(engine_str).map_err(|err| ResolutionError::UriResolutionFailed {
            uri: uri.to_string(),
            reason: format!("unparseable engine version `{engine_str}`: {err}"),
        })?;
    if req.matches(&engine) {
        Ok(())
    } else {
        Err(ResolutionError::VersionMismatch {
            uri: uri.to_string(),
            required: req_str.clone(),
            declared: engine_str.to_string(),
        })
    }
}

fn resolve_style_reference_uri(
    uri: &str,
    resolver: Option<&StyleResolver>,
) -> Result<Style, ResolutionError> {
    if let Some(resolver) = resolver {
        let style = resolver
            .resolve_style(uri)
            .map_err(|e| ResolutionError::from_resolver_error(uri, e))?;
        check_citum_version(uri, &style.info)?;
        return Ok(style);
    }

    let Some(raw_path) = uri.strip_prefix("file://") else {
        return Err(ResolutionError::UriResolutionFailed {
            uri: uri.to_string(),
            reason: "unsupported scheme; an external style resolver is required".to_string(),
        });
    };
    let path = std::path::Path::new(raw_path);
    let bytes = std::fs::read(path).map_err(|e| ResolutionError::UriResolutionFailed {
        uri: uri.to_string(),
        reason: e.to_string(),
    })?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");
    let doc_format = match ext {
        "cbor" => super::model::StyleDocumentFormat::Cbor,
        "json" => super::model::StyleDocumentFormat::Json,
        _ => super::model::StyleDocumentFormat::Yaml,
    };
    let style = Style::from_document_bytes(&bytes, doc_format).map_err(|e| {
        ResolutionError::UriResolutionFailed {
            uri: uri.to_string(),
            reason: e.to_string(),
        }
    })?;
    check_citum_version(uri, &style.info)?;
    Ok(style)
}
