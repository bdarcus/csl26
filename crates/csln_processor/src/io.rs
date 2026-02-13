/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use csl_legacy::csl_json::Reference as LegacyReference;
use csln_core::reference::InputReference;

use crate::{Bibliography, Reference};

/// Load a bibliography from a file given its path.
/// Supports CSLN YAML/JSON and CSL-JSON.
pub fn load_bibliography(path: &Path) -> Result<Bibliography, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Error reading references file: {}", e))?;

    let mut bib = indexmap::IndexMap::new();

    // 1. Try parsing as CSLN InputBibliography (YAML/JSON)
    if let Ok(input_bib) = serde_yaml::from_str::<csln_core::InputBibliography>(&content) {
        for r in input_bib.references {
            if let Some(id) = r.id() {
                bib.insert(id.to_string(), r);
            }
        }
        return Ok(bib);
    }

    // 2. Try parsing as HashMap<String, InputReference> (YAML/JSON)
    // This is common for YAML bib files where keys are IDs.
    if let Ok(map) = serde_yaml::from_str::<HashMap<String, InputReference>>(&content) {
        for (key, mut r) in map {
            if r.id().is_none() {
                r.set_id(key.clone());
            }
            bib.insert(key, r);
        }
        return Ok(bib);
    }

    // 3. Try parsing as Vec<InputReference> (YAML/JSON)
    if let Ok(refs) = serde_yaml::from_str::<Vec<InputReference>>(&content) {
        for r in refs {
            if let Some(id) = r.id() {
                bib.insert(id.to_string(), r);
            }
        }
        return Ok(bib);
    }

    // 4. Fallback: Try parsing as Legacy CSL-JSON
    if let Ok(legacy_bib) = serde_json::from_str::<Vec<LegacyReference>>(&content) {
        for ref_item in legacy_bib {
            bib.insert(ref_item.id.clone(), Reference::from(ref_item));
        }
        return Ok(bib);
    }

    Err("Error parsing references: could not parse as CSLN (YAML/JSON) or CSL-JSON".to_string())
}
