/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citum reference data loading and parsing.
//!
//! Provides multi-format bibliography parsing (Citum YAML/JSON/CBOR, CSL-JSON,
//! BibLaTeX, RIS) without depending on `citum-engine`. Both `citum-engine` and
//! `citum-io` depend on this crate; surface crates (`citum-server`,
//! `citum-bindings`) may depend on it directly.
//!
//! BibLaTeX parsing is provided via [`formats::biblatex::load_biblatex`] and the
//! conversion helpers in [`formats::biblatex`].

use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use thiserror::Error;

pub mod formats;

pub use formats::biblatex;

pub use citum_schema::InputBibliography;
pub use citum_schema::reference::InputReference as Reference;

// TODO: rename Bibliography → RefsMap (or similar) to disambiguate from the
// rendered-bibliography concept in citum-schema-style (BibliographyConfig,
// BibliographyOptions, etc.). This is a workspace-wide rename; deferred to a
// follow-up PR.
/// A resolved map of reference records keyed by ID.
pub type Bibliography = IndexMap<String, Reference>;

/// Errors produced while loading or parsing reference data.
#[derive(Error, Debug)]
pub enum RefsError {
    /// Reading an input file from disk failed.
    #[error("File I/O error: {0}")]
    FileIO(#[from] std::io::Error),

    /// Parsing a named input failed with a message describing the problem.
    #[error("Parse error ({0}): {1}")]
    ParseError(String, String),
}

/// Bibliography formats supported by reference loading helpers.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RefsFormat {
    /// Native Citum bibliography encoded as YAML.
    CitumYaml,
    /// Native Citum bibliography encoded as JSON.
    CitumJson,
    /// Native Citum bibliography encoded as CBOR.
    CitumCbor,
    /// Legacy CSL-JSON bibliography.
    CslJson,
    /// BibLaTeX `.bib` bibliography.
    Biblatex,
    /// RIS bibliography.
    Ris,
}

/// Reference data loaded from input, including optional compound sets.
#[derive(Debug, Clone, Default)]
pub struct LoadedRefs {
    /// Parsed references keyed by ID.
    pub references: Bibliography,
    /// Optional compound sets keyed by set ID.
    pub sets: Option<IndexMap<String, Vec<String>>>,
}

/// Validate compound sets against a bibliography.
///
/// Checks that every set member ID exists in the bibliography, that no ID
/// appears in more than one set, and that no ID appears more than once within
/// the same set.
///
/// Returns `None` when `sets` is `None` or empty; otherwise returns the
/// validated sets.
///
/// # Errors
///
/// Returns `RefsError::ParseError` for unknown member IDs or duplicates.
pub fn validate_compound_sets(
    sets: Option<IndexMap<String, Vec<String>>>,
    bibliography: &Bibliography,
) -> Result<Option<IndexMap<String, Vec<String>>>, RefsError> {
    let sets = match sets {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(None),
    };

    let mut membership: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();

    for (set_id, members) in &sets {
        let mut seen_in_set = std::collections::HashSet::new();
        for member_id in members {
            if !bibliography.contains_key(member_id.as_str()) {
                return Err(RefsError::ParseError(
                    "BIBLIOGRAPHY".to_string(),
                    format!("Compound set '{set_id}' contains unknown id '{member_id}'"),
                ));
            }
            if !seen_in_set.insert(member_id.as_str()) {
                return Err(RefsError::ParseError(
                    "BIBLIOGRAPHY".to_string(),
                    format!(
                        "Reference '{member_id}' appears more than once in compound set '{set_id}'"
                    ),
                ));
            }
            if let Some(existing_set) = membership.insert(member_id.as_str(), set_id.as_str()) {
                return Err(RefsError::ParseError(
                    "BIBLIOGRAPHY".to_string(),
                    format!(
                        "Reference '{member_id}' appears in both compound sets '{existing_set}' and '{set_id}'"
                    ),
                ));
            }
        }
    }

    Ok(Some(sets))
}

/// Load reference data from a file, including optional compound sets.
///
/// Supports Citum YAML/JSON/CBOR and CSL-JSON.
///
/// # Errors
///
/// Returns an error when the file cannot be read, cannot be parsed, or
/// compound sets are invalid.
pub fn load_refs_with_sets(path: &Path) -> Result<LoadedRefs, RefsError> {
    let bytes = fs::read(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    match ext {
        "cbor" => formats::native::parse_cbor_refs(&bytes),
        "json" => formats::native::parse_json_refs(&bytes),
        _ => {
            let content = String::from_utf8_lossy(&bytes);
            formats::native::parse_yaml_refs(&content)
        }
    }
}

/// Load references from a file.
///
/// Supports Citum YAML/JSON/CBOR and CSL-JSON.
///
/// # Errors
///
/// Returns an error when the file cannot be read, cannot be parsed, or
/// embedded compound-set metadata is invalid.
pub fn load_refs(path: &Path) -> Result<Bibliography, RefsError> {
    Ok(load_refs_with_sets(path)?.references)
}

/// Load and merge one or more bibliography files, preserving compound set metadata.
///
/// Entries from later files replace entries with the same ID from earlier files.
/// Compound set IDs must be unique across input files, and final membership is
/// validated against the merged bibliography.
///
/// # Errors
///
/// Returns an error when no paths are supplied, any file cannot be loaded, or
/// merged compound sets are invalid.
pub fn load_merged_refs(paths: &[PathBuf]) -> Result<LoadedRefs, RefsError> {
    if paths.is_empty() {
        return Err(RefsError::ParseError(
            "BIBLIOGRAPHY".to_string(),
            "At least one bibliography path is required.".to_string(),
        ));
    }

    let mut merged = Bibliography::new();
    let mut merged_sets = IndexMap::<String, Vec<String>>::new();
    for path in paths {
        let loaded = load_refs_with_sets(path)?;
        for (id, reference) in loaded.references {
            merged.insert(id, reference);
        }
        if let Some(sets) = loaded.sets {
            for (set_id, members) in sets {
                if merged_sets.contains_key(&set_id) {
                    return Err(RefsError::ParseError(
                        "BIBLIOGRAPHY".to_string(),
                        format!("Duplicate compound set id while merging: {set_id}"),
                    ));
                }
                merged_sets.insert(set_id, members);
            }
        }
    }

    let validated_sets =
        validate_compound_sets((!merged_sets.is_empty()).then_some(merged_sets), &merged)?;

    Ok(LoadedRefs {
        references: merged,
        sets: validated_sets,
    })
}

/// Load bibliography input in a specified native or legacy reference format.
///
/// # Errors
///
/// Returns an error when the file cannot be read or parsed as `format`.
pub fn load_input_refs(path: &Path, format: RefsFormat) -> Result<InputBibliography, RefsError> {
    match format {
        RefsFormat::CitumYaml => {
            let bytes = fs::read(path)?;
            formats::native::deserialize_any(&bytes, "yaml")
        }
        RefsFormat::CitumJson => {
            let bytes = fs::read(path)?;
            formats::native::load_citum_json(&bytes)
        }
        RefsFormat::CitumCbor => {
            let bytes = fs::read(path)?;
            formats::native::deserialize_any(&bytes, "cbor")
        }
        RefsFormat::CslJson => formats::csl_json::load_csl_json(path),
        RefsFormat::Biblatex => formats::biblatex::load_biblatex(path),
        RefsFormat::Ris => formats::ris::load_ris(path),
    }
}

/// Infer a bibliography input format from a path.
///
/// JSON inputs are content-sniffed to distinguish native Citum JSON from CSL-JSON.
///
/// # Errors
///
/// Returns an error when a JSON input cannot be read or parsed for detection.
pub fn infer_refs_input_format(path: &Path) -> Result<RefsFormat, RefsError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let fmt = match ext.to_ascii_lowercase().as_str() {
        "yaml" | "yml" => RefsFormat::CitumYaml,
        "cbor" => RefsFormat::CitumCbor,
        "bib" => RefsFormat::Biblatex,
        "ris" => RefsFormat::Ris,
        "json" => detect_json_refs_format(path)?,
        _ => RefsFormat::CitumYaml,
    };
    Ok(fmt)
}

/// Infer a bibliography output format from a path.
///
/// Unknown extensions default to native Citum YAML.
#[must_use]
pub fn infer_refs_output_format(path: &Path) -> RefsFormat {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "yaml" | "yml" => RefsFormat::CitumYaml,
        "cbor" => RefsFormat::CitumCbor,
        "bib" => RefsFormat::Biblatex,
        "ris" => RefsFormat::Ris,
        "json" => RefsFormat::CitumJson,
        _ => RefsFormat::CitumYaml,
    }
}

fn detect_json_refs_format(path: &Path) -> Result<RefsFormat, RefsError> {
    let bytes = fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| RefsError::ParseError("JSON".to_string(), e.to_string()))?;
    let array = value.as_array();
    let is_citum_array = array.is_some_and(|items| items.iter().any(|v| v.get("class").is_some()));
    let is_csl_array = array.is_some_and(|items| {
        items.iter().any(|v| {
            v.get("id").is_some()
                && v.get("type").is_some()
                && (v.get("title").is_some() || v.get("author").is_some())
        })
    });
    let is_citum_object = value.get("references").is_some();
    if is_csl_array && !is_citum_array && !is_citum_object {
        Ok(RefsFormat::CslJson)
    } else {
        Ok(RefsFormat::CitumJson)
    }
}
