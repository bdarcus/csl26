/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use std::fs;
use std::path::Path;

use csl_legacy::csl_json::Reference as LegacyReference;
use csln_core::reference::InputReference;
use csln_core::InputBibliography;

use crate::{Bibliography, Citation, Reference};

/// Load a list of citations from a file.
/// Supports CSLN YAML/JSON.
pub fn load_citations(path: &Path) -> Result<Vec<Citation>, String> {
    let bytes = fs::read(path).map_err(|e| format!("Error reading citations file: {}", e))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    match ext {
        "json" => {
            if let Ok(citations) = serde_json::from_slice::<Vec<Citation>>(&bytes) {
                return Ok(citations);
            }
            if let Ok(citation) = serde_json::from_slice::<Citation>(&bytes) {
                return Ok(vec![citation]);
            }
        }
        _ => {
            let content = String::from_utf8_lossy(&bytes);
            if let Ok(citations) = serde_yaml::from_str::<Vec<Citation>>(&content) {
                return Ok(citations);
            }
            if let Ok(citation) = serde_yaml::from_str::<Citation>(&content) {
                return Ok(vec![citation]);
            }
        }
    }

    Err("Error parsing citations: could not parse as CSLN Citation(s) (YAML/JSON)".to_string())
}

/// Load a bibliography from a file given its path.
/// Supports CSLN YAML/JSON/CBOR and CSL-JSON.
pub fn load_bibliography(path: &Path) -> Result<Bibliography, String> {
    let bytes = fs::read(path).map_err(|e| format!("Error reading references file: {}", e))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    let mut bib = indexmap::IndexMap::new();

    // Try parsing as CSLN formats
    match ext {
        "cbor" => {
            if let Ok(input_bib) = serde_cbor::from_slice::<InputBibliography>(&bytes) {
                for r in input_bib.references {
                    if let Some(id) = r.id() {
                        bib.insert(id.to_string(), r);
                    }
                }
                return Ok(bib);
            }
        }
        "json" => {
            // CSL-JSON is Vec<LegacyReference>
            if let Ok(legacy_bib) = serde_json::from_slice::<Vec<LegacyReference>>(&bytes) {
                for ref_item in legacy_bib {
                    bib.insert(ref_item.id.clone(), Reference::from(ref_item));
                }
                return Ok(bib);
            }
            // Also try CSLN JSON
            if let Ok(input_bib) = serde_json::from_slice::<InputBibliography>(&bytes) {
                for r in input_bib.references {
                    if let Some(id) = r.id() {
                        bib.insert(id.to_string(), r);
                    }
                }
                return Ok(bib);
            }

            // Try IndexMap of LegacyReference (preserves insertion order from JSON)
            if let Ok(map) =
                serde_json::from_slice::<indexmap::IndexMap<String, serde_json::Value>>(&bytes)
            {
                let mut found = false;
                for (id, val) in map {
                    if let Ok(ref_item) = serde_json::from_value::<LegacyReference>(val) {
                        let mut r = Reference::from(ref_item);
                        if r.id().is_none() {
                            r.set_id(id.clone());
                        }
                        bib.insert(id, r);
                        found = true;
                    }
                }
                if found {
                    return Ok(bib);
                }
            }
        }
        _ => {
            // YAML/Fallback
            let content = String::from_utf8_lossy(&bytes);
            if let Ok(input_bib) = serde_yaml::from_str::<InputBibliography>(&content) {
                for r in input_bib.references {
                    if let Some(id) = r.id() {
                        bib.insert(id.to_string(), r);
                    }
                }
                return Ok(bib);
            }

            // Try parsing as IndexMap<String, serde_yaml::Value> (YAML/JSON, preserves order)
            if let Ok(map) =
                serde_yaml::from_str::<indexmap::IndexMap<String, serde_yaml::Value>>(&content)
            {
                let mut found = false;
                for (key, val) in map {
                    if let Ok(mut r) = serde_yaml::from_value::<InputReference>(val.clone()) {
                        if r.id().is_none() {
                            r.set_id(key.clone());
                        }
                        bib.insert(key, r);
                        found = true;
                    } else if let Ok(ref_item) = serde_yaml::from_value::<LegacyReference>(val) {
                        let mut r = Reference::from(ref_item);
                        if r.id().is_none() {
                            r.set_id(key.clone());
                        }
                        bib.insert(key, r);
                        found = true;
                    }
                }
                if found {
                    return Ok(bib);
                }
            }

            // Try parsing as Vec<InputReference> (YAML/JSON)
            if let Ok(refs) = serde_yaml::from_str::<Vec<InputReference>>(&content) {
                for r in refs {
                    if let Some(id) = r.id() {
                        bib.insert(id.to_string(), r);
                    }
                }
                return Ok(bib);
            }
        }
    }

    Err(
        "Error parsing references: could not parse as CSLN (YAML/JSON/CBOR) or CSL-JSON"
            .to_string(),
    )
}
