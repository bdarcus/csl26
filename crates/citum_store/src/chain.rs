/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Standard ChainResolver construction for platform-aware style resolution.

use crate::{
    StoreConfig, StoreResolver, platform_config_dir, platform_data_dir,
    resolver::{
        ChainResolver, EmbeddedResolver, FileLocaleResolver, FileResolver, GitResolver,
        HttpResolver, RegistryResolver, ResolverError, StyleResolver,
    },
};
use citum_schema::{Locale, Style};
use std::{error::Error, fs, path::Path, path::PathBuf};

type DynStyleResolver = dyn StyleResolver<Style = Style, Locale = Locale>;

/// Construct a `ChainResolver` with the standard platform resolver stack.
///
/// Order: FileResolver → StoreResolver → HttpResolver → GitResolver →
/// registry resolvers (local + configured + embedded default) → EmbeddedResolver.
///
/// # Errors
///
/// Returns an error if the current working directory cannot be determined.
pub fn build_standard_chain() -> Result<ChainResolver, Box<dyn Error + Send + Sync>> {
    Ok(ChainResolver::new(standard_resolvers()?))
}

/// Construct a `ChainResolver` that also looks up locales in `<style_dir>/locales/`
/// before consulting the user store and embedded data.
///
/// Use when a style was loaded from a file path and you want the same
/// sibling-`locales/` behavior the standalone-disk workflow had.
///
/// # Errors
///
/// Returns an error if the current working directory cannot be determined.
pub fn build_chain_with_file_locale_dir(
    locales_dir: PathBuf,
) -> Result<ChainResolver, Box<dyn Error + Send + Sync>> {
    let mut resolvers: Vec<Box<DynStyleResolver>> =
        vec![Box::new(FileLocaleResolver::new(locales_dir))];
    resolvers.extend(standard_resolvers()?);
    Ok(ChainResolver::new(resolvers))
}

fn standard_resolvers() -> Result<Vec<Box<DynStyleResolver>>, Box<dyn Error + Send + Sync>> {
    let mut resolvers: Vec<Box<DynStyleResolver>> = vec![Box::new(FileResolver)];

    if let Some(data_dir) = platform_data_dir()
        && data_dir.exists()
    {
        let config = StoreConfig::load().unwrap_or_default();
        resolvers.push(Box::new(StoreResolver::new(
            data_dir,
            config.store_format(),
        )));
    }

    if let Some(http) = HttpResolver::from_platform_cache_dir() {
        resolvers.push(Box::new(http));
    }

    if let Some(git) = GitResolver::from_platform_cache_dir() {
        resolvers.push(Box::new(git));
    }

    resolvers.extend(registry_resolvers()?);

    resolvers.push(Box::new(EmbeddedResolver));

    Ok(resolvers)
}

/// Resolve `id` against `chain`, returning the embedded `en-US` baseline if
/// the chain produces no locale.
///
/// Hosts that want strict error semantics should call
/// [`ChainResolver::resolve_locale`] directly and inspect the error.
#[must_use]
pub fn load_locale_or_default(
    chain: &dyn StyleResolver<Style = Style, Locale = Locale>,
    id: &str,
) -> Locale {
    match chain.resolve_locale(id) {
        Ok(locale) => locale.resolved_for(id),
        Err(ResolverError::LocaleNotFound(_)) => Locale::en_us().resolved_for(id),
        // For other errors (read/parse failure on a found file) we still
        // return the en-US baseline so a single bad file cannot brick the
        // render path. Callers needing stricter behavior should use the
        // chain directly.
        Err(_) => Locale::en_us().resolved_for(id),
    }
}

#[derive(serde::Deserialize)]
struct RegistrySourceRecord {
    name: String,
}

fn registry_sources_path() -> Option<std::path::PathBuf> {
    platform_config_dir().map(|dir| dir.join("registry-sources.json"))
}

fn configured_registry_path(name: &str) -> Option<std::path::PathBuf> {
    platform_config_dir().map(|dir| dir.join("registries").join(format!("{name}.yaml")))
}

fn configured_registry_names() -> Vec<String> {
    let Some(path) = registry_sources_path() else {
        return Vec::new();
    };
    let Ok(bytes) = fs::read(path) else {
        return Vec::new();
    };
    let Ok(records) = serde_json::from_slice::<Vec<RegistrySourceRecord>>(&bytes) else {
        return Vec::new();
    };
    records.into_iter().map(|record| record.name).collect()
}

fn registry_resolvers() -> Result<Vec<Box<DynStyleResolver>>, Box<dyn Error + Send + Sync>> {
    let mut resolvers: Vec<Box<DynStyleResolver>> = Vec::new();

    let local_path = Path::new("citum-registry.yaml");
    if local_path.exists()
        && let Ok(registry) = citum_schema::StyleRegistry::load_from_file(local_path)
    {
        resolvers.push(Box::new(
            RegistryResolver::new(registry).with_base_dir(std::env::current_dir()?),
        ));
    }

    for name in configured_registry_names() {
        let Some(path) = configured_registry_path(&name) else {
            continue;
        };
        if !path.exists() {
            continue;
        }
        let Ok(registry) = citum_schema::StyleRegistry::load_from_file(&path) else {
            continue;
        };
        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let resolver = RegistryResolver::new(registry).with_base_dir(base_dir);
        let resolver = if let Some(http) = HttpResolver::from_platform_cache_dir() {
            resolver.with_http(http)
        } else {
            resolver
        };
        let resolver = if let Some(git) = GitResolver::from_platform_cache_dir() {
            resolver.with_git(git)
        } else {
            resolver
        };
        resolvers.push(Box::new(resolver));
    }

    // Wire StoreConfig.registries entries not yet tracked in registry-sources.json
    if let Ok(config) = StoreConfig::load() {
        let tracked_names = configured_registry_names();

        for registry_config in &config.registries {
            if tracked_names.contains(&registry_config.name) {
                continue;
            }

            if let Some(path) = configured_registry_path(&registry_config.name)
                && path.exists()
                && let Ok(registry) = citum_schema::StyleRegistry::load_from_file(&path)
            {
                let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
                let resolver = RegistryResolver::new(registry).with_base_dir(base_dir);
                let resolver = if let Some(http) = HttpResolver::from_platform_cache_dir() {
                    resolver.with_http(http)
                } else {
                    resolver
                };
                let resolver = if let Some(git) = GitResolver::from_platform_cache_dir() {
                    resolver.with_git(git)
                } else {
                    resolver
                };
                resolvers.push(Box::new(resolver));
                continue;
            }

            if let Some(http) = HttpResolver::from_platform_cache_dir()
                && let Ok(bytes) = http.fetch_bytes(&registry_config.url)
                && let Ok(registry) = serde_yaml::from_slice::<citum_schema::StyleRegistry>(&bytes)
            {
                if let Some(path) = configured_registry_path(&registry_config.name)
                    && let Some(parent) = path.parent()
                {
                    let _ = fs::create_dir_all(parent);
                    if let Ok(mut tmp) = tempfile::NamedTempFile::new_in(parent) {
                        let _ = std::io::Write::write_all(&mut tmp, &bytes);
                        let _ = tmp.persist(&path);
                    }
                }
                let base_dir = configured_registry_path(&registry_config.name)
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| Path::new(".").to_path_buf());
                let resolver = RegistryResolver::new(registry).with_base_dir(base_dir);
                let resolver = resolver.with_http(http);
                let resolver = if let Some(git) = GitResolver::from_platform_cache_dir() {
                    resolver.with_git(git)
                } else {
                    resolver
                };
                resolvers.push(Box::new(resolver));
            }
        }
    }

    let resolver = RegistryResolver::new(citum_schema::embedded::default_registry())
        .with_base_dir(std::env::current_dir()?);
    let resolver = if let Some(http) = HttpResolver::from_platform_cache_dir() {
        resolver.with_http(http)
    } else {
        resolver
    };
    let resolver = if let Some(git) = GitResolver::from_platform_cache_dir() {
        resolver.with_git(git)
    } else {
        resolver
    };
    resolvers.push(Box::new(resolver));

    Ok(resolvers)
}
