/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Store resolver for locating and managing user styles and locales.

use crate::format::StoreFormat;
use citum_schema::{Locale, Style, StyleRegistry};
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
#[cfg(feature = "http")]
use std::io::Read;
#[cfg(feature = "http")]
use std::net::IpAddr;
use std::path::{Path, PathBuf};
#[cfg(feature = "http")]
use std::time::{Duration, SystemTime};

#[cfg(feature = "http")]
use sha2::{Digest, Sha256};
#[cfg(feature = "http")]
use tempfile;

pub use citum_resolver_api::{ResolutionError, ResolverError, StyleResolver};

/// Parse a [`Style`] from bytes, routing through [`Style::from_document_bytes`]
/// so the raw value tree is always populated for null-aware overlay merging
/// (see `docs/specs/STYLE_INHERITANCE.md`), regardless of which resolver or
/// wire format the style arrived through.
fn parse_style_bytes(bytes: &[u8], format: StoreFormat) -> Result<Style, ResolverError> {
    let doc_format = match format {
        StoreFormat::Yaml => citum_schema::StyleDocumentFormat::Yaml,
        StoreFormat::Json => citum_schema::StyleDocumentFormat::Json,
        StoreFormat::Cbor => citum_schema::StyleDocumentFormat::Cbor,
    };
    Style::from_document_bytes(bytes, doc_format).map_err(|e| match e {
        citum_schema::StyleDocumentError::Yaml(err) => ResolverError::YamlError(err.to_string()),
        citum_schema::StyleDocumentError::Json(err) => ResolverError::JsonError(err),
        citum_schema::StyleDocumentError::Cbor(err) => ResolverError::CborError(err),
        citum_schema::StyleDocumentError::Validation(err) => {
            ResolverError::InvalidStyle(err.into())
        }
    })
}

/// Parse any other store-resolved type (locales) using plain format-dispatched
/// deserialization; only [`Style`] needs the raw-tree-preserving path.
fn parse_generic_bytes<T: DeserializeOwned>(
    bytes: &[u8],
    format: StoreFormat,
) -> Result<T, ResolverError> {
    match format {
        StoreFormat::Yaml => {
            serde_yaml::from_slice(bytes).map_err(|e| ResolverError::YamlError(e.to_string()))
        }
        StoreFormat::Json => serde_json::from_slice(bytes).map_err(ResolverError::JsonError),
        StoreFormat::Cbor => ciborium::de::from_reader(Cursor::new(bytes))
            .map_err(|e| ResolverError::CborError(e.to_string())),
    }
}

/// A resolver that searches a local directory for styles and locales.
pub struct FileResolver;

impl StyleResolver for FileResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        let path = if let Some(path_str) = uri.strip_prefix("file://") {
            PathBuf::from(path_str)
        } else {
            PathBuf::from(uri)
        };

        if path.is_file() {
            let content = fs::read(&path)?;
            let format = StoreFormat::detect(&path).unwrap_or(StoreFormat::Yaml);
            parse_style_bytes(&content, format)
        } else {
            Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))
        }
    }

    fn resolve_locale(&self, _id: &str) -> Result<Locale, ResolverError> {
        Err(ResolverError::LocaleNotFound(Cow::Borrowed(
            "file resolver only resolves styles",
        )))
    }
}

/// A resolver that loads locales from `<base_dir>/<id>.{yaml,yml,json,cbor}`.
///
/// Use this to expose a sibling-`locales/` directory that lives next to a
/// file-based style. Styles are not handled.
pub struct FileLocaleResolver {
    base_dir: PathBuf,
}

impl FileLocaleResolver {
    /// Create a resolver rooted at `base_dir`.
    #[must_use]
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

impl StyleResolver for FileLocaleResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        for ext in StoreFormat::all_extensions() {
            let path = self.base_dir.join(format!("{id}.{ext}"));
            if !path.is_file() {
                continue;
            }
            let bytes = fs::read(&path)?;
            let format = StoreFormat::detect(&path).unwrap_or(StoreFormat::Yaml);
            return match format {
                StoreFormat::Yaml => Locale::from_yaml_str(&String::from_utf8_lossy(&bytes))
                    .map_err(|e| ResolverError::YamlError(ToString::to_string(&e))),
                StoreFormat::Json => {
                    serde_json::from_slice(&bytes).map_err(ResolverError::JsonError)
                }
                StoreFormat::Cbor => ciborium::de::from_reader(Cursor::new(&bytes))
                    .map_err(|e| ResolverError::CborError(e.to_string())),
            };
        }
        Err(ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

/// A resolver that manages user-installed styles and locales in a platform data directory.
pub struct StoreResolver {
    data_dir: PathBuf,
    format: StoreFormat,
}

impl StyleResolver for StoreResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        StoreResolver::resolve_style(self, uri)
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        StoreResolver::resolve_locale(self, id)
    }
}

/// A resolver that checks for embedded styles and locales.
pub struct EmbeddedResolver;

impl StyleResolver for EmbeddedResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        citum_schema::embedded::get_embedded_style(uri)
            .ok_or_else(|| ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))?
            .map_err(|e| ResolverError::YamlError(ToString::to_string(&e)))
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        citum_schema::embedded::get_locale(id)
            .ok_or_else(|| ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

/// A resolver that looks up styles in a registry and delegates to the matching source.
pub struct RegistryResolver {
    registry: StyleRegistry,
    base_dir: Option<PathBuf>,
    #[cfg(feature = "http")]
    http: Option<HttpResolver>,
    #[cfg(feature = "http")]
    git: Option<GitResolver>,
}

impl RegistryResolver {
    /// Create a registry resolver for the given registry.
    #[must_use]
    pub fn new(registry: StyleRegistry) -> Self {
        Self {
            registry,
            base_dir: None,
            #[cfg(feature = "http")]
            http: None,
            #[cfg(feature = "http")]
            git: None,
        }
    }

    /// Set the base directory used for relative path-backed registry entries.
    #[must_use]
    pub fn with_base_dir(mut self, base_dir: PathBuf) -> Self {
        self.base_dir = Some(base_dir);
        self
    }

    /// Set the HTTP resolver used for URL-backed registry entries.
    #[cfg(feature = "http")]
    #[must_use]
    pub fn with_http(mut self, http: HttpResolver) -> Self {
        self.http = Some(http);
        self
    }

    /// Set the Git resolver used for `git+https://` backed registry entries.
    #[cfg(feature = "http")]
    #[must_use]
    pub fn with_git(mut self, git: GitResolver) -> Self {
        self.git = Some(git);
        self
    }
}

impl StyleResolver for RegistryResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        let entry = self
            .registry
            .resolve(uri)
            .ok_or_else(|| ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))?;

        if let Some(builtin) = &entry.builtin {
            return EmbeddedResolver.resolve_style(builtin);
        }

        if let Some(path) = &entry.path {
            let path = self
                .base_dir
                .as_ref()
                .map_or_else(|| path.clone(), |base| base.join(path));
            let path_str = path
                .to_str()
                .ok_or_else(|| ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))?;
            return FileResolver.resolve_style(path_str);
        }

        #[cfg(feature = "http")]
        if let Some(url) = &entry.url {
            if url.starts_with("git+") && GitResolver::parse_git_uri(url).is_none() {
                return Err(ResolverError::Denied {
                    uri: url.clone(),
                    reason: "only git+https:// URLs with safe relative file paths are allowed"
                        .to_string(),
                });
            }
            if let Some((_, _)) = GitResolver::parse_git_uri(url)
                && let Some(git) = &self.git
            {
                return git.resolve_style(url);
            }
            if let Some(http) = &self.http {
                return http.resolve_style(url);
            }
        }

        Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        Err(ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

/// A resolver that fetches styles from Git repositories via shallow clone.
///
/// URI format: `git+https://github.com/org/repo.git#path/to/style.yaml`
#[cfg(feature = "http")]
pub struct GitResolver {
    cache_dir: PathBuf,
    policy: RemoteFetchPolicy,
}

/// A resolver that fetches HTTP(S) style URLs and caches them on disk.
///
/// The underlying HTTP client is initialized lazily on first use so that
/// constructing a [`ChainResolver`] is safe inside async Tokio contexts.
#[cfg(feature = "http")]
pub struct HttpResolver {
    cache_dir: PathBuf,
    client: std::sync::OnceLock<Result<reqwest::blocking::Client, String>>,
    policy: RemoteFetchPolicy,
}

#[cfg(feature = "http")]
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

#[cfg(feature = "http")]
const HTTP_CACHE_MAX_AGE: Duration = Duration::from_hours(24);

/// Maximum number of remote bytes accepted for a style or registry fetch.
#[cfg(feature = "http")]
pub const DEFAULT_REMOTE_FETCH_MAX_BYTES: u64 = 2 * 1024 * 1024;

/// Network policy applied before fetching remote styles or registries.
#[cfg(feature = "http")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFetchPolicy {
    /// Whether plaintext `http://` URLs are allowed.
    pub allow_http: bool,
    /// Whether HTTP redirects are allowed.
    pub allow_redirects: bool,
    /// Maximum accepted response body size in bytes.
    pub max_bytes: u64,
    /// Optional exact-match hostname allowlist.
    pub allowed_hosts: Vec<String>,
}

#[cfg(feature = "http")]
impl Default for RemoteFetchPolicy {
    fn default() -> Self {
        Self {
            allow_http: false,
            allow_redirects: false,
            max_bytes: DEFAULT_REMOTE_FETCH_MAX_BYTES,
            allowed_hosts: Vec::new(),
        }
    }
}

#[cfg(feature = "http")]
impl RemoteFetchPolicy {
    /// Return a policy suitable for local test servers.
    #[must_use]
    pub fn localhost_for_tests() -> Self {
        Self {
            allow_http: true,
            allow_redirects: false,
            max_bytes: DEFAULT_REMOTE_FETCH_MAX_BYTES,
            allowed_hosts: vec!["127.0.0.1".to_string(), "localhost".to_string()],
        }
    }

    /// Validate and parse a URL before a network request is attempted.
    ///
    /// # Errors
    ///
    /// Returns [`ResolverError::Denied`] when the URL violates this policy.
    pub fn validate_url(&self, uri: &str) -> Result<reqwest::Url, ResolverError> {
        let url = reqwest::Url::parse(uri).map_err(|err| ResolverError::Denied {
            uri: uri.to_string(),
            reason: format!("invalid URL: {err}"),
        })?;

        match url.scheme() {
            "https" => {}
            "http" if self.allow_http => {}
            "http" => {
                return Err(ResolverError::Denied {
                    uri: uri.to_string(),
                    reason: "plaintext HTTP is disabled by default".to_string(),
                });
            }
            scheme => {
                return Err(ResolverError::Denied {
                    uri: uri.to_string(),
                    reason: format!("unsupported URL scheme '{scheme}'"),
                });
            }
        }

        if !url.username().is_empty() || url.password().is_some() {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: "credentials in remote style URLs are not allowed".to_string(),
            });
        }

        if url.fragment().is_some() {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: "fragments in remote style URLs are not allowed".to_string(),
            });
        }

        let Some(host) = url.host_str() else {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: "remote URL has no host".to_string(),
            });
        };

        let host_is_allowlisted =
            !self.allowed_hosts.is_empty() && self.allowed_hosts.iter().any(|h| h == host);

        if !host_is_allowlisted && host.parse::<IpAddr>().is_ok_and(is_denied_ip_literal) {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: format!("IP literal host '{host}' is not allowed by default"),
            });
        }

        if !self.allowed_hosts.is_empty() && !host_is_allowlisted {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: format!("host '{host}' not in resolver allowlist"),
            });
        }

        Ok(url)
    }
}

#[cfg(feature = "http")]
fn is_denied_ip_literal(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

#[cfg(feature = "http")]
impl HttpResolver {
    /// Create a resolver using the provided cache directory.
    ///
    /// The HTTP client is not created here; it is initialized lazily on first use.
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            client: std::sync::OnceLock::new(),
            policy: RemoteFetchPolicy::default(),
        }
    }

    fn client(&self) -> Result<&reqwest::blocking::Client, ResolverError> {
        self.client
            .get_or_init(|| {
                let mut builder = reqwest::blocking::Client::builder().timeout(HTTP_TIMEOUT);
                if !self.policy.allow_redirects {
                    builder = builder.redirect(reqwest::redirect::Policy::none());
                }
                builder
                    .build()
                    .map_err(|err| format!("failed to build HTTP client: {err}"))
            })
            .as_ref()
            .map_err(|err| ResolverError::HttpError(err.clone()))
    }

    /// Set the complete remote fetch policy for this resolver.
    #[must_use]
    pub fn with_policy(mut self, policy: RemoteFetchPolicy) -> Self {
        self.policy = policy;
        self.client = std::sync::OnceLock::new();
        self
    }

    /// Return the active remote fetch policy.
    #[must_use]
    pub fn policy(&self) -> &RemoteFetchPolicy {
        &self.policy
    }

    fn read_limited_body(
        &self,
        uri: &str,
        mut response: reqwest::blocking::Response,
    ) -> Result<Vec<u8>, ResolverError> {
        let mut reader = response
            .by_ref()
            .take(self.policy.max_bytes.saturating_add(1));
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        if bytes.len() as u64 > self.policy.max_bytes {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: format!(
                    "remote response exceeds {} byte limit",
                    self.policy.max_bytes
                ),
            });
        }
        Ok(bytes)
    }

    fn fetch_validated_bytes(&self, uri: &str) -> Result<Vec<u8>, ResolverError> {
        let url = self.policy.validate_url(uri)?;
        let response =
            self.client()?
                .get(url)
                .send()
                .map_err(|err| ResolverError::NetworkError {
                    uri: uri.to_string(),
                    reason: err.to_string(),
                })?;
        if response.status().is_redirection() && !self.policy.allow_redirects {
            return Err(ResolverError::Denied {
                uri: uri.to_string(),
                reason: format!("redirect response {} is not allowed", response.status()),
            });
        }
        if !response.status().is_success() {
            return Err(ResolverError::HttpError(format!(
                "failed to fetch {uri}: status {}",
                response.status()
            )));
        }
        self.read_limited_body(uri, response)
    }

    /// Create a resolver using the platform cache directory.
    #[must_use]
    pub fn from_platform_cache_dir() -> Option<Self> {
        crate::platform_cache_dir().map(Self::new)
    }

    /// Set an allowlist of hosts for this resolver. Empty list allows all hosts.
    #[must_use]
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.policy.allowed_hosts = hosts;
        self.client = std::sync::OnceLock::new();
        self
    }

    /// Fetch raw bytes from a URL.
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.fetch_validated_bytes(url)
            .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
    }

    fn cache_path(&self, uri: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(uri.as_bytes());
        let hash = hasher.finalize();
        let mut hex = String::with_capacity(hash.len() * 2);
        for byte in hash {
            hex.push(hex_digit(byte >> 4));
            hex.push(hex_digit(byte & 0x0f));
        }
        self.cache_dir
            .join("styles")
            .join("http")
            .join(format!("{hex}.yaml"))
    }

    fn parse_style(uri: &str, bytes: &[u8]) -> Result<Style, ResolverError> {
        Style::from_yaml_bytes(bytes).map_err(|err| {
            ResolverError::YamlError(format!("failed to parse style fetched from {uri}: {err}"))
        })
    }

    fn cache_is_fresh(path: &Path) -> bool {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .and_then(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .map_err(std::io::Error::other)
            })
            .is_ok_and(|age| age < HTTP_CACHE_MAX_AGE)
    }

    fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), ResolverError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut tmp = tempfile::NamedTempFile::new_in(
            path.parent().unwrap_or_else(|| std::path::Path::new(".")),
        )?;
        std::io::Write::write_all(&mut tmp, bytes)?;
        tmp.persist(path).map_err(|e| ResolverError::Io(e.error))?;
        Ok(())
    }
}

#[cfg(feature = "http")]
fn hex_digit(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => '?',
    }
}

#[cfg(feature = "http")]
impl StyleResolver for HttpResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        if !uri.starts_with("http://") && !uri.starts_with("https://") {
            return Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())));
        }
        let url = self.policy.validate_url(uri)?;

        let cache_path = self.cache_path(uri);
        if cache_path.is_file() && Self::cache_is_fresh(&cache_path) {
            let bytes = fs::read(&cache_path)?;
            return Self::parse_style(uri, &bytes);
        }

        let fetch_result = self.client()?.get(url).send();

        match fetch_result {
            Ok(response) => {
                if response.status().is_redirection() && !self.policy.allow_redirects {
                    if cache_path.is_file() {
                        let bytes = fs::read(&cache_path)?;
                        return Self::parse_style(uri, &bytes);
                    }
                    return Err(ResolverError::Denied {
                        uri: uri.to_string(),
                        reason: format!("redirect response {} is not allowed", response.status()),
                    });
                }

                if response.status() == reqwest::StatusCode::NOT_FOUND {
                    if cache_path.is_file() {
                        let bytes = fs::read(&cache_path)?;
                        return Self::parse_style(uri, &bytes);
                    }
                    return Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())));
                }

                if !response.status().is_success() {
                    if cache_path.is_file() {
                        let bytes = fs::read(&cache_path)?;
                        return Self::parse_style(uri, &bytes);
                    }
                    return Err(ResolverError::HttpError(format!(
                        "failed to fetch {uri}: HTTP {}",
                        response.status()
                    )));
                }

                match self.read_limited_body(uri, response) {
                    Ok(bytes) => {
                        let style = Self::parse_style(uri, &bytes)?;
                        Self::write_cache(&cache_path, &bytes)?;
                        Ok(style)
                    }
                    Err(err) => {
                        if matches!(err, ResolverError::Denied { .. }) {
                            return Err(err);
                        }
                        // Check for stale cache and serve if available
                        if cache_path.is_file() {
                            let bytes = fs::read(&cache_path)?;
                            Self::parse_style(uri, &bytes)
                        } else {
                            Err(ResolverError::NetworkError {
                                uri: uri.to_string(),
                                reason: err.to_string(),
                            })
                        }
                    }
                }
            }
            Err(err) => {
                // Check for stale cache and serve if available
                if cache_path.is_file() {
                    let bytes = fs::read(&cache_path)?;
                    Self::parse_style(uri, &bytes)
                } else {
                    Err(ResolverError::NetworkError {
                        uri: uri.to_string(),
                        reason: err.to_string(),
                    })
                }
            }
        }
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        Err(ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

#[cfg(feature = "http")]
impl GitResolver {
    /// Create a resolver using the provided cache directory.
    #[must_use]
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            policy: RemoteFetchPolicy::default(),
        }
    }

    /// Set the complete remote fetch policy for this resolver.
    #[must_use]
    pub fn with_policy(mut self, policy: RemoteFetchPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Create a resolver using the platform cache directory.
    #[must_use]
    pub fn from_platform_cache_dir() -> Option<Self> {
        crate::platform_cache_dir().map(Self::new)
    }

    fn cache_path(&self, uri: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(uri.as_bytes());
        let hash = hasher.finalize();
        let mut hex = String::with_capacity(hash.len() * 2);
        for byte in hash {
            hex.push(hex_digit(byte >> 4));
            hex.push(hex_digit(byte & 0x0f));
        }
        self.cache_dir
            .join("styles")
            .join("git")
            .join(format!("{hex}.yaml"))
    }

    fn parse_style(uri: &str, bytes: &[u8]) -> Result<Style, ResolverError> {
        Style::from_yaml_bytes(bytes).map_err(|err| {
            ResolverError::YamlError(format!("failed to parse style fetched from {uri}: {err}"))
        })
    }

    fn cache_is_fresh(path: &Path) -> bool {
        path.metadata()
            .and_then(|metadata| metadata.modified())
            .and_then(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .map_err(std::io::Error::other)
            })
            .is_ok_and(|age| age < HTTP_CACHE_MAX_AGE)
    }

    fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), ResolverError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut tmp = tempfile::NamedTempFile::new_in(
            path.parent().unwrap_or_else(|| std::path::Path::new(".")),
        )?;
        std::io::Write::write_all(&mut tmp, bytes)?;
        tmp.persist(path).map_err(|e| ResolverError::Io(e.error))?;
        Ok(())
    }

    /// Parse a git URI into repo URL and file path.
    pub fn parse_git_uri(uri: &str) -> Option<(String, String)> {
        if !uri.starts_with("git+https://") {
            return None;
        }
        let rest = uri.strip_prefix("git+")?;
        let (repo_part, file_path) = rest.split_once('#')?;
        if !is_safe_git_style_path(file_path) {
            return None;
        }
        Some((repo_part.to_string(), file_path.to_string()))
    }
}

#[cfg(feature = "http")]
fn is_safe_git_style_path(file_path: &str) -> bool {
    let path = Path::new(file_path);
    !file_path.is_empty()
        && !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

#[cfg(feature = "http")]
impl StyleResolver for GitResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        let (repo_url, file_path) = Self::parse_git_uri(uri).ok_or_else(|| {
            if uri.starts_with("git+") {
                ResolverError::Denied {
                    uri: uri.to_string(),
                    reason: "only git+https:// URLs with safe relative file paths are allowed"
                        .to_string(),
                }
            } else {
                ResolverError::StyleNotFound(Cow::Owned(uri.to_string()))
            }
        })?;
        self.policy.validate_url(&repo_url)?;

        let cache_path = self.cache_path(uri);
        if cache_path.is_file() && Self::cache_is_fresh(&cache_path) {
            let bytes = fs::read(&cache_path)?;
            return Self::parse_style(uri, &bytes);
        }

        // Create a temporary directory for the git clone
        let tmpdir = tempfile::TempDir::new().map_err(ResolverError::Io)?;

        let mut prepare = gix::prepare_clone(repo_url.clone(), tmpdir.path()).map_err(|e| {
            // Serve stale cache if available
            if cache_path.is_file() {
                return ResolverError::Io(std::io::Error::other("gix fail"));
            }
            ResolverError::NetworkError {
                uri: uri.to_string(),
                reason: format!("git clone failed to prepare: {e}"),
            }
        })?;

        // We only need a shallow clone of the default branch
        if let Some(depth) = std::num::NonZeroU32::new(1) {
            prepare = prepare.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(depth));
        }

        let (repo, _) = prepare
            .fetch_only(
                gix::progress::Discard,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .map_err(|e| {
                if cache_path.is_file() {
                    return ResolverError::Io(std::io::Error::other("gix fetch fail"));
                }
                ResolverError::NetworkError {
                    uri: uri.to_string(),
                    reason: format!("git fetch failed: {e}"),
                }
            })?;

        let bytes_result = (|| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            let head = repo
                .head()?
                .id()
                .ok_or("repository has no HEAD")?
                .object()?
                .into_commit();
            let tree = head.tree()?;
            let entry = tree
                .lookup_entry_by_path(&file_path)?
                .ok_or("file not found in repo")?;
            let blob = entry.object()?;
            Ok(blob.data.clone())
        })();

        match bytes_result {
            Ok(bytes) => {
                let style = Self::parse_style(uri, &bytes)?;
                Self::write_cache(&cache_path, &bytes)?;
                Ok(style)
            }
            Err(_) => {
                // Serve stale cache if available
                if cache_path.is_file() {
                    let bytes = fs::read(&cache_path)?;
                    Self::parse_style(uri, &bytes)
                } else {
                    Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))
                }
            }
        }
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        Err(ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

/// Default IPFS HTTP gateway used by [`CidResolver`] when none is configured.
#[cfg(feature = "http")]
pub const DEFAULT_CID_GATEWAY: &str = "https://dweb.link/ipfs/";

/// A resolver that fetches content-addressed Citum styles via an IPFS HTTP
/// gateway and verifies their integrity against the requested CID.
///
/// `CidResolver` accepts URIs of the form `cid:bafkrei…`. It rewrites the
/// URI to `<gateway>/<cid>`, delegates the HTTP fetch and disk caching to a
/// wrapped [`HttpResolver`], then re-hashes the response bytes and refuses to
/// hand back a [`Style`] whose CID does not match the requested one. Cache
/// entries for `cid:` URIs are immutable — they need not be revalidated.
#[cfg(feature = "http")]
pub struct CidResolver {
    gateway: String,
    http: HttpResolver,
}

#[cfg(feature = "http")]
impl CidResolver {
    /// Construct a `CidResolver` with an explicit gateway and HTTP backend.
    ///
    /// `gateway` should end with a trailing slash; `https://dweb.link/ipfs/`
    /// Construct a `CidResolver` with an explicit gateway and HTTP backend.
    #[must_use]
    pub fn new(gateway: String, http: HttpResolver) -> Self {
        let gateway = if gateway.ends_with('/') {
            gateway
        } else {
            format!("{gateway}/")
        };
        Self { gateway, http }
    }

    /// Construct a `CidResolver` using [`DEFAULT_CID_GATEWAY`] and the platform
    /// cache directory. Returns `None` when the platform has no cache dir.
    #[must_use]
    pub fn from_platform_cache_dir() -> Option<Self> {
        HttpResolver::from_platform_cache_dir()
            .map(|http| Self::new(DEFAULT_CID_GATEWAY.to_string(), http))
    }

    fn resolve_cid_uri(&self, uri: &str) -> Result<Style, ResolverError> {
        let cid_str = crate::cid::strip_cid_scheme(uri);
        let canonical = crate::cid::canonicalize_cid(cid_str)?;
        let gateway_url = format!("{}{canonical}", self.gateway);
        let bytes =
            self.http
                .fetch_bytes(&gateway_url)
                .map_err(|err| ResolverError::NetworkError {
                    uri: uri.to_string(),
                    reason: err.to_string(),
                })?;
        crate::cid::verify_cid(uri, &canonical, &bytes)?;
        Style::from_yaml_bytes(&bytes).map_err(|err| {
            ResolverError::YamlError(format!("failed to parse CID-resolved style {uri}: {err}"))
        })
    }
}

#[cfg(feature = "http")]
impl StyleResolver for CidResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        if !crate::cid::is_cid_uri(uri) {
            return Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())));
        }
        self.resolve_cid_uri(uri)
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        Err(ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

/// Middleware resolver that wraps another [`StyleResolver`] and verifies the
/// content of every successful resolution against an expected CID.
///
/// Used by Phase 3 `extends-pin` enforcement: the parent resolution path
/// constructs a `VerifyingResolver` whose `expected` is the child's
/// declared pin, then routes the parent fetch through it. When `expected`
/// is `None`, the wrapper is a no-op pass-through.
///
/// Note: this wrapper has to re-fetch raw bytes through `HttpResolver`'s
/// public `fetch_bytes` API to verify integrity, because the inner
/// `resolve_style` returns a parsed `Style` whose serialized form is not
/// guaranteed to be byte-identical to the source. Callers that care about
/// `extends-pin` enforcement should prefer [`fetch_and_verify_bytes`] for
/// HTTP/CID URIs and skip the trip through `Style`.
#[cfg(feature = "http")]
pub struct VerifyingResolver<R: StyleResolver> {
    inner: R,
    expected: Option<String>,
}

#[cfg(feature = "http")]
impl<R: StyleResolver> VerifyingResolver<R> {
    /// Wrap `inner` with an integrity check against `expected_cid`.
    ///
    /// `expected_cid` may include or omit the `cid:` scheme prefix.
    #[must_use]
    pub fn new(inner: R, expected_cid: Option<String>) -> Self {
        Self {
            inner,
            expected: expected_cid,
        }
    }

    /// Returns the inner resolver, consuming this wrapper.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

#[cfg(feature = "http")]
impl<R: StyleResolver<Style = Style, Locale = Locale>> StyleResolver for VerifyingResolver<R> {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        let style = self.inner.resolve_style(uri)?;
        if let Some(ref pin) = self.expected {
            // Best-effort verification: re-serialize and hash. This rejects
            // tampering at the trust boundary even if the canonical serializer
            // round-trip differs from the upstream bytes — at the cost of
            // false negatives for whitespace-only differences. Callers that
            // need exact-bytes verification should use
            // [`fetch_and_verify_bytes`] before parsing.
            let bytes = serde_yaml::to_string(&style).map_err(|err| {
                ResolverError::YamlError(format!("re-serialize for pin check: {err}"))
            })?;
            crate::cid::verify_cid(uri, pin, bytes.as_bytes())?;
        }
        Ok(style)
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        self.inner.resolve_locale(id)
    }
}

/// Fetch raw bytes for a remote URI and verify them against `expected_cid`
/// before returning. This is the canonical entry point for `extends-pin`
/// enforcement — it operates on bytes, not parsed structures, so a single
/// trailing newline cannot make verification falsely pass or fail.
///
/// Supported schemes: `https://`, `http://`, `cid:`. Unsupported schemes
/// (including `file://` and `git+https://`) return [`ResolverError::Denied`]
/// — pinning makes sense only for content-addressed or HTTPS-fetched bytes.
///
/// # Errors
///
/// - [`ResolverError::Denied`] for unsupported URI schemes.
/// - [`ResolverError::NetworkError`] when the HTTP fetch fails.
/// - [`ResolverError::IntegrityFailure`] when the bytes do not match
///   `expected_cid`.
#[cfg(feature = "http")]
pub fn fetch_and_verify_bytes(
    http: &HttpResolver,
    cid_resolver: &CidResolver,
    uri: &str,
    expected_cid: &str,
) -> Result<Vec<u8>, ResolverError> {
    let bytes = if crate::cid::is_cid_uri(uri) {
        let canonical = crate::cid::canonicalize_cid(crate::cid::strip_cid_scheme(uri))?;
        let gateway_url = format!("{}{canonical}", cid_resolver.gateway);
        http.fetch_bytes(&gateway_url)
            .map_err(|err| ResolverError::NetworkError {
                uri: uri.to_string(),
                reason: err.to_string(),
            })?
    } else if matches!(
        reqwest::Url::parse(uri).as_ref().map(reqwest::Url::scheme),
        Ok("https" | "http")
    ) {
        http.fetch_bytes(uri)
            .map_err(|err| ResolverError::NetworkError {
                uri: uri.to_string(),
                reason: err.to_string(),
            })?
    } else {
        return Err(ResolverError::Denied {
            uri: uri.to_string(),
            reason: "extends-pin only supports cid: and policy-approved HTTP(S) URIs".to_string(),
        });
    };
    crate::cid::verify_cid(uri, expected_cid, &bytes)?;
    Ok(bytes)
}

/// A resolver that chains multiple resolvers and tries them in order.
pub struct ChainResolver {
    resolvers: Vec<Box<dyn StyleResolver<Style = Style, Locale = Locale>>>,
}

impl ChainResolver {
    /// Create a new `ChainResolver` with the given list of resolvers.
    #[must_use]
    pub fn new(resolvers: Vec<Box<dyn StyleResolver<Style = Style, Locale = Locale>>>) -> Self {
        ChainResolver { resolvers }
    }
}

impl StyleResolver for ChainResolver {
    type Style = Style;
    type Locale = Locale;

    fn resolve_style(&self, uri: &str) -> Result<Style, ResolverError> {
        for resolver in &self.resolvers {
            match resolver.resolve_style(uri) {
                Ok(style) => return Ok(style),
                Err(ResolverError::StyleNotFound(_)) => {}
                Err(err) => return Err(err),
            }
        }
        Err(ResolverError::StyleNotFound(Cow::Owned(uri.to_string())))
    }

    fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        for resolver in &self.resolvers {
            match resolver.resolve_locale(id) {
                Ok(locale) => return Ok(locale),
                Err(ResolverError::LocaleNotFound(_)) => {}
                Err(err) => return Err(err),
            }
        }
        Err(ResolverError::LocaleNotFound(Cow::Owned(id.to_string())))
    }
}

impl StoreResolver {
    /// Create a new resolver with the given data directory and format.
    #[must_use]
    pub fn new(data_dir: PathBuf, format: StoreFormat) -> Self {
        StoreResolver { data_dir, format }
    }

    /// Resolve a style by ID.
    ///
    /// # Errors
    /// Returns an error if the style cannot be found or loaded.
    pub fn resolve_style(&self, id: &str) -> Result<Style, ResolverError> {
        self.resolve_item(id, "styles", parse_style_bytes)
    }

    /// Resolve a locale by ID.
    ///
    /// # Errors
    /// Returns an error if the locale cannot be found or loaded.
    pub fn resolve_locale(&self, id: &str) -> Result<Locale, ResolverError> {
        self.resolve_item(id, "locales", parse_generic_bytes)
    }

    /// List all installed styles.
    ///
    /// # Errors
    /// Returns an error if the styles directory cannot be read.
    pub fn list_styles(&self) -> Result<Vec<String>, ResolverError> {
        self.list_items("styles")
    }

    /// List all installed locales.
    ///
    /// # Errors
    /// Returns an error if the locales directory cannot be read.
    pub fn list_locales(&self) -> Result<Vec<String>, ResolverError> {
        self.list_items("locales")
    }

    /// Install a style from a source file.
    ///
    /// # Errors
    /// Returns an error if the source file cannot be read or the destination cannot be written.
    pub fn install_style(&self, source: &Path) -> Result<String, ResolverError> {
        self.install_item(source, "styles")
    }

    /// Install a locale from a source file.
    ///
    /// # Errors
    /// Returns an error if the source file cannot be read or the destination cannot be written.
    pub fn install_locale(&self, source: &Path) -> Result<String, ResolverError> {
        self.install_item(source, "locales")
    }

    /// Remove an installed style by ID.
    ///
    /// # Errors
    /// Returns an error if the style cannot be found or removed.
    pub fn remove_style(&self, id: &str) -> Result<(), ResolverError> {
        self.remove_item(id, "styles")
    }

    /// Remove an installed locale by ID.
    ///
    /// # Errors
    /// Returns an error if the locale cannot be found or removed.
    pub fn remove_locale(&self, id: &str) -> Result<(), ResolverError> {
        self.remove_item(id, "locales")
    }

    // --- Internal Helpers ---

    fn resolve_item<T>(
        &self,
        id: &str,
        category: &str,
        parse: impl Fn(&[u8], StoreFormat) -> Result<T, ResolverError>,
    ) -> Result<T, ResolverError> {
        let items_dir = self.data_dir.join(category);
        if !items_dir.exists() {
            return Err(match category {
                "styles" => ResolverError::StyleNotFound(id.to_string().into()),
                _ => ResolverError::LocaleNotFound(id.to_string().into()),
            });
        }

        // Try exact match with current format extension first
        let path = items_dir.join(format!("{}.{}", id, self.format.extension()));
        if path.is_file() {
            return self.load_item_at(&path, &parse);
        }

        // Fallback: probe every accepted extension (yaml + yml + json + cbor)
        for ext in StoreFormat::all_extensions() {
            let path = items_dir.join(format!("{id}.{ext}"));
            if path.is_file() {
                return self.load_item_at(&path, &parse);
            }
        }

        Err(match category {
            "styles" => ResolverError::StyleNotFound(id.to_string().into()),
            _ => ResolverError::LocaleNotFound(id.to_string().into()),
        })
    }

    fn load_item_at<T>(
        &self,
        path: &Path,
        parse: &impl Fn(&[u8], StoreFormat) -> Result<T, ResolverError>,
    ) -> Result<T, ResolverError> {
        let content = fs::read(path)?;
        let format = StoreFormat::detect(path).unwrap_or(self.format);
        parse(&content, format)
    }

    fn list_items(&self, category: &str) -> Result<Vec<String>, ResolverError> {
        let items_dir = self.data_dir.join(category);
        if !items_dir.exists() {
            return Ok(Vec::new());
        }

        let mut names = BTreeSet::new();
        for entry in fs::read_dir(items_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file()
                && StoreFormat::detect(&path).is_some()
                && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            {
                names.insert(name.to_string());
            }
        }
        Ok(names.into_iter().collect())
    }

    // Helper: install an item from a source file.
    fn install_item(&self, source: &Path, category: &str) -> Result<String, ResolverError> {
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ResolverError::InvalidStyle("no filename".into()))?;

        let category_dir = self.data_dir.join(category);
        fs::create_dir_all(&category_dir)?;

        // Detect format from source, or use configured default
        let source_format = StoreFormat::detect(source).unwrap_or(self.format);
        let dest_path = category_dir.join(format!("{}.{}", name, source_format.extension()));

        fs::copy(source, dest_path)?;
        Ok(name.to_string())
    }

    fn remove_item(&self, id: &str, category: &str) -> Result<(), ResolverError> {
        let items_dir = self.data_dir.join(category);
        if !items_dir.exists() {
            return Err(match category {
                "styles" => ResolverError::StyleNotFound(id.to_string().into()),
                _ => ResolverError::LocaleNotFound(id.to_string().into()),
            });
        }

        // Search and remove all matching files for this ID
        let mut found = false;
        for ext in StoreFormat::all_extensions() {
            let path = items_dir.join(format!("{id}.{ext}"));
            if path.exists() {
                fs::remove_file(path)?;
                found = true;
            }
        }

        if !found {
            return Err(match category {
                "styles" => ResolverError::StyleNotFound(id.to_string().into()),
                _ => ResolverError::LocaleNotFound(id.to_string().into()),
            });
        }

        Ok(())
    }
}
