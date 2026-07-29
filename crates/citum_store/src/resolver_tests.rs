/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Integration tests for `StoreResolver`.

use crate::{FileLocaleResolver, StoreFormat, StoreResolver, resolver::StyleResolver};
use std::fs;
use tempfile::TempDir;

/// Returns a minimal valid Citum style as YAML bytes.
fn minimal_style_yaml() -> &'static [u8] {
    include_bytes!("../../../styles/alpha.yaml")
}

/// Returns a minimal installed locale fixture as YAML bytes.
fn en_us_locale_yaml() -> &'static [u8] {
    b"locale: en-US\n"
}

/// Creates a temporary store directory and a resolver pointing at it.
fn make_resolver(format: StoreFormat) -> (TempDir, StoreResolver) {
    let dir = TempDir::new().expect("tempdir");
    let resolver = StoreResolver::new(dir.path().to_path_buf(), format);
    (dir, resolver)
}

#[test]
fn list_styles_empty_on_fresh_store() {
    let (_dir, resolver) = make_resolver(StoreFormat::Yaml);
    let styles = resolver.list_styles().expect("list_styles");
    assert!(styles.is_empty());
}

#[test]
fn install_and_list_yaml_style() {
    let (dir, resolver) = make_resolver(StoreFormat::Yaml);

    // Write the fixture to a temp file
    let src = dir.path().join("alpha.yaml");
    fs::write(&src, minimal_style_yaml()).unwrap();

    let name = resolver.install_style(&src).expect("install_style");
    assert_eq!(name, "alpha");

    let styles = resolver.list_styles().expect("list_styles");
    assert_eq!(styles, vec!["alpha"]);
}

#[test]
fn resolve_installed_yaml_style() {
    let (dir, resolver) = make_resolver(StoreFormat::Yaml);

    let src = dir.path().join("alpha.yaml");
    fs::write(&src, minimal_style_yaml()).unwrap();
    resolver.install_style(&src).expect("install_style");

    let style = resolver.resolve_style("alpha").expect("resolve_style");
    assert_eq!(style.info.title.as_deref(), Some("Alpha (biblatex-alpha)"));
}

#[test]
fn remove_installed_style() {
    let (dir, resolver) = make_resolver(StoreFormat::Yaml);

    let src = dir.path().join("alpha.yaml");
    fs::write(&src, minimal_style_yaml()).unwrap();
    resolver.install_style(&src).expect("install_style");
    resolver.remove_style("alpha").expect("remove_style");

    let styles = resolver.list_styles().expect("list_styles");
    assert!(styles.is_empty());
}

#[test]
fn resolve_missing_style_returns_error() {
    let (_dir, resolver) = make_resolver(StoreFormat::Yaml);
    assert!(resolver.resolve_style("nonexistent").is_err());
}

#[test]
fn remove_missing_style_returns_error() {
    let (_dir, resolver) = make_resolver(StoreFormat::Yaml);
    assert!(resolver.remove_style("nonexistent").is_err());
}

#[test]
fn install_and_resolve_json_style() {
    let (dir, resolver) = make_resolver(StoreFormat::Json);

    // Convert the YAML fixture to JSON via citum_schema
    let style: citum_schema::Style =
        serde_yaml::from_slice(minimal_style_yaml()).expect("parse yaml");
    let json = serde_json::to_vec(&style).expect("to json");

    let src = dir.path().join("alpha.json");
    fs::write(&src, &json).unwrap();
    resolver.install_style(&src).expect("install_style");

    let resolved = resolver.resolve_style("alpha").expect("resolve_style");
    assert_eq!(
        resolved.info.title.as_deref(),
        Some("Alpha (biblatex-alpha)")
    );
}

#[test]
fn install_and_resolve_cbor_style() {
    let (dir, resolver) = make_resolver(StoreFormat::Cbor);

    let style: citum_schema::Style =
        serde_yaml::from_slice(minimal_style_yaml()).expect("parse yaml");
    let mut cbor = Vec::new();
    ciborium::ser::into_writer(&style, &mut cbor).expect("to cbor");

    let src = dir.path().join("alpha.cbor");
    fs::write(&src, &cbor).unwrap();
    resolver.install_style(&src).expect("install_style");

    let resolved = resolver.resolve_style("alpha").expect("resolve_style");
    assert_eq!(
        resolved.info.title.as_deref(),
        Some("Alpha (biblatex-alpha)")
    );
}

#[test]
fn resolver_fallback_finds_any_format() {
    // Resolver configured for CBOR but only a YAML file exists — should still find it.
    let (dir, resolver) = make_resolver(StoreFormat::Cbor);

    let src = dir.path().join("alpha.yaml");
    fs::write(&src, minimal_style_yaml()).unwrap();
    // Install directly (bypassing the resolver so it lands as .yaml)
    let styles_dir = dir.path().join("styles");
    fs::create_dir_all(&styles_dir).unwrap();
    fs::copy(&src, styles_dir.join("alpha.yaml")).unwrap();

    let resolved = resolver.resolve_style("alpha").expect("fallback resolve");
    assert_eq!(
        resolved.info.title.as_deref(),
        Some("Alpha (biblatex-alpha)")
    );
}

#[test]
fn resolve_installed_yaml_locale() {
    let (dir, resolver) = make_resolver(StoreFormat::Yaml);
    let locales_dir = dir.path().join("locales");
    fs::create_dir_all(&locales_dir).unwrap();
    fs::write(locales_dir.join("en-US.yaml"), en_us_locale_yaml()).unwrap();

    let locale = resolver.resolve_locale("en-US").expect("resolve_locale");

    assert_eq!(locale.locale, "en-US");
}

#[test]
fn list_styles_ignores_unsupported_files_and_deduplicates_formats() {
    let (dir, resolver) = make_resolver(StoreFormat::Yaml);
    let styles_dir = dir.path().join("styles");
    fs::create_dir_all(&styles_dir).unwrap();
    fs::write(styles_dir.join("alpha.yaml"), minimal_style_yaml()).unwrap();
    fs::write(styles_dir.join("alpha.json"), "{}").unwrap();
    fs::write(styles_dir.join("notes.txt"), "not a style").unwrap();

    let styles = resolver.list_styles().expect("list_styles");

    assert_eq!(styles, vec!["alpha"]);
}

#[cfg(feature = "http")]
#[test]
fn git_resolver_parses_git_uri() {
    use crate::GitResolver;

    // Test URI parsing
    let uri =
        "git+https://github.com/citum/citum-external-registry-prototype.git#styles/example.yaml";
    let (repo, file) = GitResolver::parse_git_uri(uri).expect("parse_git_uri");

    assert_eq!(
        repo,
        "https://github.com/citum/citum-external-registry-prototype.git"
    );
    assert_eq!(file, "styles/example.yaml");
}

#[cfg(feature = "http")]
#[test]
fn git_resolver_rejects_plain_http_uri() {
    use crate::GitResolver;

    let uri = "git+http://github.com/citum/example.git#styles/example.yaml";

    assert!(GitResolver::parse_git_uri(uri).is_none());
}

#[cfg(feature = "http")]
#[test]
fn git_resolver_rejects_unsafe_style_paths() {
    use crate::GitResolver;

    assert!(
        GitResolver::parse_git_uri("git+https://github.com/citum/example.git#/etc/passwd")
            .is_none()
    );
    assert!(
        GitResolver::parse_git_uri("git+https://github.com/citum/example.git#../style.yaml")
            .is_none()
    );
    assert!(GitResolver::parse_git_uri("git+https://github.com/citum/example.git#").is_none());
}

#[cfg(feature = "http")]
#[test]
fn verifying_resolver_passes_through_when_pin_matches() {
    use crate::cid::compute_style_cid;
    use crate::resolver::{StyleResolver, VerifyingResolver};
    use citum_schema::Style;

    struct FixedResolver(Style);
    impl StyleResolver for FixedResolver {
        type Style = Style;
        type Locale = citum_schema::Locale;

        fn resolve_style(&self, _uri: &str) -> Result<Style, crate::resolver::ResolverError> {
            Ok(self.0.clone())
        }
        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<citum_schema::Locale, crate::resolver::ResolverError> {
            Err(crate::resolver::ResolverError::LocaleNotFound(
                id.to_string().into(),
            ))
        }
    }

    let yaml = b"info:\n  title: Test\n";
    let style: Style = serde_yaml::from_slice(yaml).unwrap();
    let canonical = serde_yaml::to_string(&style).unwrap();
    let pin = compute_style_cid(canonical.as_bytes());

    let inner = FixedResolver(style.clone());
    let verifying = VerifyingResolver::new(inner, Some(pin));
    let resolved = verifying
        .resolve_style("file:///dev/null")
        .expect("verifying pass-through");
    assert_eq!(resolved.info.title.as_deref(), Some("Test"));
}

#[cfg(feature = "http")]
#[test]
fn verifying_resolver_rejects_when_pin_mismatches() {
    use crate::resolver::{StyleResolver, VerifyingResolver};
    use citum_schema::Style;

    struct FixedResolver(Style);
    impl StyleResolver for FixedResolver {
        type Style = Style;
        type Locale = citum_schema::Locale;

        fn resolve_style(&self, _uri: &str) -> Result<Style, crate::resolver::ResolverError> {
            Ok(self.0.clone())
        }
        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<citum_schema::Locale, crate::resolver::ResolverError> {
            Err(crate::resolver::ResolverError::LocaleNotFound(
                id.to_string().into(),
            ))
        }
    }

    let yaml = b"info:\n  title: Test\n";
    let style: Style = serde_yaml::from_slice(yaml).unwrap();
    // Wrong pin: hash of unrelated content.
    let bogus_pin = crate::cid::compute_style_cid(b"different bytes entirely");
    let verifying = VerifyingResolver::new(FixedResolver(style), Some(bogus_pin));
    let err = verifying
        .resolve_style("file:///dev/null")
        .expect_err("must reject");
    assert!(
        matches!(err, crate::resolver::ResolverError::IntegrityFailure { .. }),
        "expected IntegrityFailure, got {err:?}"
    );
}

#[test]
fn resolver_error_variants_format_distinctly() {
    use crate::resolver::ResolverError;

    let denied = ResolverError::Denied {
        uri: "https://evil.example.com/x.yaml".to_string(),
        reason: "host not in allowlist".to_string(),
    };
    assert_eq!(
        denied.to_string(),
        "host not in resolver allowlist: https://evil.example.com/x.yaml (host not in allowlist)"
    );

    let net = ResolverError::NetworkError {
        uri: "https://example.org/y.yaml".to_string(),
        reason: "connection refused".to_string(),
    };
    assert_eq!(
        net.to_string(),
        "network error fetching https://example.org/y.yaml: connection refused"
    );

    let version = ResolverError::VersionMismatch {
        uri: "https://example.org/z.yaml".to_string(),
        required: ">=99.0.0".to_string(),
        declared: "0.38.0".to_string(),
    };
    assert_eq!(
        version.to_string(),
        "engine version mismatch for https://example.org/z.yaml: engine requires >=99.0.0, style declares 0.38.0"
    );

    let integrity = ResolverError::IntegrityFailure {
        uri: "cid:bafkreiabc".to_string(),
        expected: "bafkreiabc".to_string(),
        actual: "bafkreidef".to_string(),
    };
    assert_eq!(
        integrity.to_string(),
        "integrity failure for cid:bafkreiabc: expected bafkreiabc, got bafkreidef"
    );
}

#[test]
fn file_locale_resolver_finds_yaml() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("xx-XX.yaml"), b"locale: xx-XX\n").unwrap();
    let resolver = FileLocaleResolver::new(dir.path().to_path_buf());

    let locale = resolver.resolve_locale("xx-XX").expect("resolve");
    assert_eq!(locale.locale, "xx-XX");
}

#[test]
fn file_locale_resolver_finds_yml_alias() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("xx-XX.yml"), b"locale: xx-XX\n").unwrap();
    let resolver = FileLocaleResolver::new(dir.path().to_path_buf());

    let locale = resolver.resolve_locale("xx-XX").expect("resolve");
    assert_eq!(locale.locale, "xx-XX");
}

#[test]
fn file_locale_resolver_missing_returns_not_found() {
    let dir = TempDir::new().expect("tempdir");
    let resolver = FileLocaleResolver::new(dir.path().to_path_buf());
    assert!(resolver.resolve_locale("nope").is_err());
}

#[test]
fn file_locale_resolver_does_not_resolve_styles() {
    let dir = TempDir::new().expect("tempdir");
    let resolver = FileLocaleResolver::new(dir.path().to_path_buf());
    assert!(resolver.resolve_style("apa").is_err());
}

#[cfg(feature = "http")]
#[test]
fn load_locale_or_default_returns_en_us_on_miss() {
    use crate::resolver::ChainResolver;

    let dir = TempDir::new().expect("tempdir");
    let chain = ChainResolver::new(vec![Box::new(FileLocaleResolver::new(
        dir.path().to_path_buf(),
    ))]);

    let locale = crate::load_locale_or_default(&chain, "missing-locale");
    assert_eq!(locale.locale, "en-US");
}

#[cfg(feature = "http")]
#[test]
fn load_locale_or_default_uses_first_match() {
    use crate::resolver::ChainResolver;

    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("xx-XX.yaml"), b"locale: xx-XX\n").unwrap();
    let chain = ChainResolver::new(vec![Box::new(FileLocaleResolver::new(
        dir.path().to_path_buf(),
    ))]);

    let locale = crate::load_locale_or_default(&chain, "xx-XX");
    assert_eq!(locale.locale, "xx-XX");
}

// Regression tests for csl26-j3zy: a child style resolved through
// `StoreResolver` (in any wire format) must clear an explicit-`null`
// inherited field identically to the same child loaded directly from a
// file, because `merge_style_overlay` reads null-clear intent from the
// *child's* raw value tree, not from typed `Option::None`.
mod store_raw_ingest_regression {
    use super::*;
    use citum_schema::Style;
    use std::collections::HashSet;

    const BASE_YAML: &str = r#"
version: "0.44.0"
info: { id: base }
citation:
  options: {}
"#;

    /// Resolver stub that always hands back `base` for `extends: base`,
    /// mirroring the fixture pattern citum-schema-style's own
    /// `bdd_inheritance` null-clear regression tests use.
    struct BaseOnly(Style);

    impl citum_resolver_api::StyleResolver for BaseOnly {
        type Style = Style;
        type Locale = citum_schema::Locale;

        fn resolve_style(&self, _uri: &str) -> Result<Style, citum_resolver_api::ResolverError> {
            Ok(self.0.clone())
        }

        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<Self::Locale, citum_resolver_api::ResolverError> {
            Err(citum_resolver_api::ResolverError::LocaleNotFound(
                id.to_string().into(),
            ))
        }
    }

    /// Child document as a generic JSON value carrying an explicit
    /// `citation.options: null` — the wire-format-neutral shape each test
    /// below transcodes into its target store format.
    fn child_value() -> serde_json::Value {
        serde_json::json!({
            "extends": "base",
            "info": { "id": "child" },
            "citation": { "options": serde_json::Value::Null }
        })
    }

    fn assert_options_cleared(resolved: &Style) {
        assert!(
            resolved.citation.as_ref().unwrap().options.is_none(),
            "explicit `citation.options: null` did not clear inherited options"
        );
    }

    fn resolve_child_through_store(format: StoreFormat, bytes: &[u8], ext: &str) -> Style {
        let (dir, resolver) = make_resolver(format);
        let styles_dir = dir.path().join("styles");
        fs::create_dir_all(&styles_dir).unwrap();
        fs::write(styles_dir.join(format!("child.{ext}")), bytes).unwrap();

        let base = citum_schema::Style::from_yaml_str(BASE_YAML).expect("valid base style");
        assert!(
            base.citation.as_ref().unwrap().options.is_some(),
            "base must have citation.options for this test"
        );

        let child = resolver.resolve_style("child").expect("resolve_style");
        let mut visited = HashSet::new();
        child
            .try_into_resolved_recursive_with(Some(&BaseOnly(base)), &mut visited)
            .expect("child resolves against base")
    }

    #[test]
    fn store_resolved_yaml_child_null_clear_matches_file_loaded() {
        let bytes = serde_yaml::to_string(&child_value()).unwrap().into_bytes();
        let resolved = resolve_child_through_store(StoreFormat::Yaml, &bytes, "yaml");
        assert_options_cleared(&resolved);
    }

    #[test]
    fn store_resolved_json_child_null_clear_matches_file_loaded() {
        let bytes = serde_json::to_vec(&child_value()).unwrap();
        let resolved = resolve_child_through_store(StoreFormat::Json, &bytes, "json");
        assert_options_cleared(&resolved);
    }

    #[test]
    fn store_resolved_cbor_child_null_clear_matches_file_loaded() {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(&child_value(), &mut bytes).unwrap();
        let resolved = resolve_child_through_store(StoreFormat::Cbor, &bytes, "cbor");
        assert_options_cleared(&resolved);
    }
}
