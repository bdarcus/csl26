/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use std::fs;
use std::path::Path;

use csl_legacy::csl_json::Reference as LegacyReference;
use csln_core::InputBibliography;
use csln_core::reference::InputReference;

use crate::{Bibliography, Citation, ProcessorError, Reference};

/// Load a list of citations from a file.
/// Supports CSLN YAML/JSON.
pub fn load_citations(path: &Path) -> Result<Vec<Citation>, ProcessorError> {
    let bytes = fs::read(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    match ext {
        "json" => {
            // Check for syntax errors first
            let _: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| ProcessorError::ParseError("JSON".to_string(), e.to_string()))?;

            if let Ok(citations) = serde_json::from_slice::<Vec<Citation>>(&bytes) {
                return Ok(citations);
            }
            match serde_json::from_slice::<Citation>(&bytes) {
                Ok(citation) => Ok(vec![citation]),
                Err(e) => Err(ProcessorError::ParseError(
                    "JSON".to_string(),
                    e.to_string(),
                )),
            }
        }
        _ => {
            let content = String::from_utf8_lossy(&bytes);
            // Check for syntax errors first
            let _: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| ProcessorError::ParseError("YAML".to_string(), e.to_string()))?;

            if let Ok(citations) = serde_yaml::from_str::<Vec<Citation>>(&content) {
                return Ok(citations);
            }
            match serde_yaml::from_str::<Citation>(&content) {
                Ok(citation) => Ok(vec![citation]),
                Err(e) => Err(ProcessorError::ParseError(
                    "YAML".to_string(),
                    e.to_string(),
                )),
            }
        }
    }
}

/// Load a bibliography from a file given its path.
/// Supports CSLN YAML/JSON/CBOR and CSL-JSON.
pub fn load_bibliography(path: &Path) -> Result<Bibliography, ProcessorError> {
    let bytes = fs::read(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    let mut bib = indexmap::IndexMap::new();

    // Try parsing as CSLN formats
    match ext {
        "cbor" => match serde_cbor::from_slice::<InputBibliography>(&bytes) {
            Ok(input_bib) => {
                for r in input_bib.references {
                    if let Some(id) = r.id() {
                        bib.insert(id.to_string(), r);
                    }
                }
                Ok(bib)
            }
            Err(e) => Err(ProcessorError::ParseError(
                "CBOR".to_string(),
                e.to_string(),
            )),
        },
        "json" => {
            // Check for syntax errors first
            let _: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| ProcessorError::ParseError("JSON".to_string(), e.to_string()))?;

            // Try CSL-JSON (Vec<LegacyReference>)
            if let Ok(legacy_bib) = serde_json::from_slice::<Vec<LegacyReference>>(&bytes) {
                for ref_item in legacy_bib {
                    bib.insert(ref_item.id.clone(), Reference::from(ref_item));
                }
                return Ok(bib);
            }
            // Try CSLN JSON (InputBibliography)
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

            // If all failed, return the error from the most likely format (CSLN JSON)
            match serde_json::from_slice::<InputBibliography>(&bytes) {
                Ok(_) => unreachable!(),
                Err(e) => Err(ProcessorError::ParseError(
                    "JSON".to_string(),
                    e.to_string(),
                )),
            }
        }
        _ => {
            // YAML/Fallback
            let content = String::from_utf8_lossy(&bytes);

            // Check for syntax errors first
            let _: serde_yaml::Value = serde_yaml::from_str(&content)
                .map_err(|e| ProcessorError::ParseError("YAML".to_string(), e.to_string()))?;

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

            // If all failed, return error from CSLN YAML
            match serde_yaml::from_str::<InputBibliography>(&content) {
                Ok(_) => unreachable!(),
                Err(e) => Err(ProcessorError::ParseError(
                    "YAML".to_string(),
                    e.to_string(),
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_citations_preserves_locator_labels() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/citations-expanded.json");
        let citations = load_citations(&path).expect("citations fixture should parse");
        let with_locator = citations
            .iter()
            .find(|c| c.id.as_deref() == Some("with-locator"))
            .expect("with-locator citation should exist");

        assert_eq!(with_locator.items.len(), 1);
        assert_eq!(
            with_locator.items[0].label,
            Some(csln_core::citation::LocatorType::Page)
        );
        assert_eq!(with_locator.items[0].locator.as_deref(), Some("23"));
    }
}
