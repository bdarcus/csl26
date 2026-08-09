/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Style query subcommands: list, search, info, cid, pin, validate, browse.
//!
//! [`dispatch`] routes every `StyleCommands` variant; install/remove live in
//! [`super::style_install`] and lint in [`super::lint`].

use super::CliResult;
use super::catalog::{
    CatalogSourceFilter, StyleCatalogPage, paginate_style_catalog_rows, print_style_catalog_rows,
    style_catalog_entries, style_row_matches_query,
};
use super::lint::run_lint_style;
use super::style_install::{CliStyleBrowserActions, run_style_add, run_style_remove};
use crate::args::{StyleCatalogFormat, StyleCommands};
use crate::style_browser::{StyleBrowserConfig, run_style_browser};
use crate::style_catalog::StyleCatalogRow;
use citum_schema::{Locale, Style};
use citum_store::{StoreConfig, StoreResolver, platform_data_dir};
use std::error::Error;
use std::io::{self, IsTerminal};

pub(super) fn run_style_list(
    source: &str,
    format: StyleCatalogFormat,
    limit: Option<usize>,
    offset: usize,
) -> CliResult {
    let source_filter = CatalogSourceFilter::parse(source)?;
    let rows = style_catalog_entries(source_filter)?;
    let (total, rows) = paginate_style_catalog_rows(rows, StyleCatalogPage { limit, offset });
    print_style_catalog_rows(&rows, total, &source_filter.label(), format)
}

pub(super) fn run_style_search(
    query: &str,
    source: &str,
    format: StyleCatalogFormat,
    limit: Option<usize>,
    offset: usize,
) -> CliResult {
    let source_filter = CatalogSourceFilter::parse(source)?;
    let rows: Vec<_> = style_catalog_entries(source_filter)?
        .into_iter()
        .filter(|row| style_row_matches_query(row, query))
        .collect();
    let (total, rows) = paginate_style_catalog_rows(rows, StyleCatalogPage { limit, offset });
    print_style_catalog_rows(&rows, total, &source_filter.label(), format)
}

pub(super) fn run_style_info(name: &str, format: StyleCatalogFormat) -> CliResult {
    let rows = style_catalog_entries(CatalogSourceFilter::All)?;
    let row = rows
        .into_iter()
        .find(|row| row.id == name || row.aliases.iter().any(|alias| alias == name))
        .ok_or_else(|| format!("style not found: {name}"))?;

    // Load the unresolved Style so the CID matches what extends-pin verification
    // computes. Best-effort: catalog entries may exist without a loadable backing
    // style (e.g. registry entries pointing at unfetched URLs); in that case we
    // omit the CID block.
    let unresolved = load_unresolved_style(&row.id).ok();
    let cid_string = unresolved.as_ref().and_then(|style| {
        let canonical = serde_yaml::to_string(style).ok()?;
        Some(citum_store::cid::compute_style_cid(canonical.as_bytes()))
    });
    let citum_version = unresolved
        .as_ref()
        .and_then(|style| style.info.citum_version.clone());

    if format == StyleCatalogFormat::Json {
        let mut value = serde_json::to_value(&row)?;
        if let Some(map) = value.as_object_mut() {
            if let Some(cid) = &cid_string {
                map.insert("cid".to_string(), serde_json::Value::String(cid.clone()));
            }
            if let Some(req) = &citum_version {
                map.insert(
                    "citum-version".to_string(),
                    serde_json::Value::String(req.clone()),
                );
            }
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("ID:       {}", row.id);
    println!("Title:    {}", row.title.as_deref().unwrap_or("-"));
    println!("Source:   {}", row.source);
    println!(
        "Aliases:  {}",
        if row.aliases.is_empty() {
            "-".to_string()
        } else {
            row.aliases.join(", ")
        }
    );
    if let Some(description) = row.description {
        println!("Summary:  {description}");
    }
    if !row.fields.is_empty() {
        println!("Fields:   {}", row.fields.join(", "));
    }
    if let Some(url) = row.url {
        println!("URL:      {url}");
    }
    if let Some(req) = &citum_version {
        println!("Citum:    {req}");
    }
    if let Some(cid) = &cid_string {
        println!("CID:      {cid}");
        println!("Pin:      extends-pin: {cid}");
    }
    Ok(())
}

/// Compute and print the CIDv1 of a style, identified by file path or
/// catalog name.
pub(super) fn run_style_cid(target: &str, format: StyleCatalogFormat) -> CliResult {
    let bytes = read_target_bytes(target)?;
    let cid = citum_store::cid::compute_style_cid(&bytes);

    if format == StyleCatalogFormat::Json {
        let value = serde_json::json!({
            "target": target,
            "cid": cid,
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{cid}");
    }
    Ok(())
}

/// Print a paste-ready `extends:` + `extends-pin:` block for a parent style.
pub(super) fn run_style_pin(
    target: &str,
    uri_override: Option<&str>,
    format: StyleCatalogFormat,
) -> CliResult {
    let bytes = read_target_bytes(target)?;
    let cid = citum_store::cid::compute_style_cid(&bytes);
    let uri = uri_override
        .map(str::to_string)
        .unwrap_or_else(|| derive_pin_uri(target));

    if format == StyleCatalogFormat::Json {
        let value = serde_json::json!({
            "extends": uri,
            "extends-pin": format!("cid:{cid}"),
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("extends: {uri}");
        println!("extends-pin: cid:{cid}");
    }
    Ok(())
}

/// Validate a style end-to-end: parse, resolve `extends`, run lint warnings,
/// verify any `extends-pin`, and check `citum-version`.
pub(super) fn run_style_validate(
    target: &str,
    format: StyleCatalogFormat,
    include_resolved: bool,
) -> CliResult {
    if include_resolved && format != StyleCatalogFormat::Json {
        return Err("--include-resolved requires --format json".into());
    }
    let style = load_unresolved_style(target)?;
    let warnings = style.validate();
    // try_into_resolved walks extends, runs extends-pin verification, and
    // applies the citum-version check on every URI-resolved parent — this is
    // the same code path the engine uses at render time.
    let mut resolved = style
        .clone()
        .try_into_resolved_with(Some(&build_chain_resolver()?))?;
    resolved.extends = None;
    let canonical = serde_yaml::to_string(&style)?;
    let cid = citum_store::cid::compute_style_cid(canonical.as_bytes());

    if format == StyleCatalogFormat::Json {
        let value = style_validation_json(target, &warnings, &cid, &resolved, include_resolved)?;
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("OK   {target}");
        println!("CID  {cid}");
        if let Some(ref req) = resolved.info.citum_version {
            println!("Citum {req}");
        }
        for warning in &warnings {
            println!("warn {warning}");
        }
    }
    Ok(())
}

fn style_validation_json(
    target: &str,
    warnings: &[citum_schema::SchemaWarning],
    cid: &str,
    resolved: &Style,
    include_resolved: bool,
) -> Result<serde_json::Value, serde_json::Error> {
    let mut value = serde_json::json!({
        "target": target,
        "ok": warnings.is_empty(),
        "warnings": warnings.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "cid": cid,
        "citum-version": resolved.info.citum_version,
    });
    if include_resolved && let Some(object) = value.as_object_mut() {
        object.insert(
            "resolved-style".to_string(),
            serde_json::to_value(resolved)?,
        );
    }
    Ok(value)
}

/// Read canonical Style bytes for a target argument that may be a file path or
/// catalog name.
///
/// Always returns `serde_yaml::to_string(&parsed_style).into_bytes()`. This is
/// the same byte sequence the schema layer hashes when verifying an
/// `extends-pin`, so a CID computed from these bytes will round-trip a child
/// style's pin verification — which would not be true if we hashed the raw
/// on-disk bytes (whitespace, comments, key ordering would all perturb the
/// CID).
fn read_target_bytes(target: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let style = load_unresolved_style(target)?;
    Ok(serde_yaml::to_string(&style)?.into_bytes())
}

/// Load a Style without recursively resolving its `extends` chain.
///
/// File paths are parsed via [`Style::from_yaml_bytes`]. Installed/builtin
/// names go through the local resolver chain (file → store → embedded). The
/// returned Style is the just-deserialized form, mirroring what
/// `verify_parent_pin` sees at the schema layer.
pub(super) fn load_unresolved_style(target: &str) -> Result<Style, Box<dyn Error>> {
    use citum_store::resolver::{ChainResolver, EmbeddedResolver, FileResolver, StyleResolver};

    let path = std::path::Path::new(target);
    if path.is_file() {
        let bytes = std::fs::read(path)?;
        return Ok(Style::from_yaml_bytes(&bytes)?);
    }

    let mut resolvers: Vec<Box<dyn StyleResolver<Style = Style, Locale = Locale>>> =
        vec![Box::new(FileResolver)];
    if let Some(data_dir) = platform_data_dir()
        && data_dir.exists()
    {
        let cfg = StoreConfig::load().unwrap_or_default();
        resolvers.push(Box::new(StoreResolver::new(data_dir, cfg.store_format())));
    }
    resolvers.push(Box::new(EmbeddedResolver));
    let chain = ChainResolver::new(resolvers);
    Ok(chain.resolve_style(target)?)
}

/// Build a citum_schema::StyleResolver chain for use in `style validate`.
///
/// Mirrors the resolver chain `load_any_style` constructs but returns a
/// trait-object suitable for passing into
/// `Style::try_into_resolved_with(Some(&...))`. CidResolver is intentionally
/// omitted — `style validate` operates on local bytes and should not silently
/// reach across the network during a "is my file OK?" check.
fn build_chain_resolver()
-> Result<impl citum_resolver_api::StyleResolver<Style = Style, Locale = Locale>, Box<dyn Error>> {
    use citum_store::resolver::{ChainResolver, EmbeddedResolver, FileResolver, StyleResolver};

    let mut resolvers: Vec<Box<dyn StyleResolver<Style = Style, Locale = Locale>>> =
        vec![Box::new(FileResolver)];
    if let Some(data_dir) = platform_data_dir()
        && data_dir.exists()
    {
        let cfg = StoreConfig::load().unwrap_or_default();
        resolvers.push(Box::new(StoreResolver::new(data_dir, cfg.store_format())));
    }
    resolvers.push(Box::new(EmbeddedResolver));
    Ok(ChainResolver::new(resolvers))
}

/// Best-effort URI derivation for `style pin`: prefers an absolute `file://`
/// URL for real paths, falls back to the catalog name as a placeholder so the
/// user knows to replace it with a hosting URL before publishing.
fn derive_pin_uri(target: &str) -> String {
    let path = std::path::Path::new(target);
    if path.is_file() {
        let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        // `Url::from_file_path` builds a spec-compliant URI (correct slash
        // direction on Windows, percent-encoded reserved characters). It only
        // fails for relative paths, so the lossy fallback covers the
        // can't-canonicalize edge case.
        url::Url::from_file_path(&abs)
            .map(|u| u.to_string())
            .unwrap_or_else(|()| format!("file://{}", abs.to_string_lossy()))
    } else {
        format!("https://hub.citum.org/styles/{target}.yaml")
    }
}

pub(super) fn dispatch(command: StyleCommands) -> CliResult {
    match command {
        StyleCommands::List {
            source,
            format,
            limit,
            offset,
        } => run_style_list(&source, format, limit, offset),
        StyleCommands::Search {
            query,
            source,
            format,
            limit,
            offset,
        } => run_style_search(&query, &source, format, limit, offset),
        StyleCommands::Info { name, format } => run_style_info(&name, format),
        StyleCommands::Browse { query, source } => run_style_browse(query.as_deref(), &source),
        StyleCommands::Add { query, yes } => run_style_add(&query, yes),
        StyleCommands::Remove { name, yes } => run_style_remove(&name, yes),
        StyleCommands::Lint(args) => run_lint_style(args),
        StyleCommands::Cid { target, format } => run_style_cid(&target, format),
        StyleCommands::Pin {
            target,
            uri,
            format,
        } => run_style_pin(&target, uri.as_deref(), format),
        StyleCommands::Validate {
            target,
            format,
            include_resolved,
        } => run_style_validate(&target, format, include_resolved),
    }
}

pub(super) fn run_style_browse(query: Option<&str>, source: &str) -> CliResult {
    let source_filter = CatalogSourceFilter::parse(source)?;
    let all_rows = style_catalog_entries(source_filter)?;
    if !io::stdin().is_terminal() {
        let rows: Vec<StyleCatalogRow> = all_rows
            .into_iter()
            .filter(|row| query.is_none_or(|q| style_row_matches_query(row, q)))
            .take(20)
            .collect();
        print_style_catalog_rows(
            &rows,
            rows.len(),
            &source_filter.label(),
            StyleCatalogFormat::Text,
        )?;
        return Ok(());
    }

    let mut actions = CliStyleBrowserActions;
    run_style_browser(
        StyleBrowserConfig {
            rows: all_rows,
            initial_query: query.unwrap_or("").to_string(),
            source_label: source_filter.label(),
        },
        &mut actions,
    )
}

#[cfg(test)]
mod tests {
    use super::style_validation_json;
    use citum_schema::Style;

    #[test]
    fn validation_json_includes_resolved_style_only_when_requested() {
        let parsed = Style::from_yaml_str(
            r#"
info:
  title: Resolved style
citation:
  template:
    - title: primary
"#,
        );
        assert!(parsed.is_ok(), "style fixture must parse");
        let Some(parsed) = parsed.ok() else {
            return;
        };
        let result = parsed.try_into_resolved();
        assert!(result.is_ok(), "style fixture must parse and resolve");
        let Some(style) = result.ok() else {
            return;
        };

        let compact = style_validation_json("style.yaml", &[], "cid", &style, false);
        let with_resolved = style_validation_json("style.yaml", &[], "cid", &style, true);
        assert!(compact.is_ok(), "compact JSON must serialize");
        assert!(with_resolved.is_ok(), "resolved JSON must serialize");
        let (Some(compact), Some(with_resolved)) = (compact.ok(), with_resolved.ok()) else {
            return;
        };

        assert!(compact.get("resolved-style").is_none());
        assert_eq!(
            with_resolved.pointer("/resolved-style/citation/template/0/title"),
            Some(&serde_json::json!("primary"))
        );
    }
}
