/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and desired in test code."
)]

//! Verifies the worked native-YAML examples inlined into
//! `docs/reference/NATIVE_FORMAT.md` (via `scripts/build-data-model-reference.js`)
//! actually deserialize and round-trip, so a stale example fails CI instead of
//! silently drifting from the schema.

use std::fs;
use std::path::PathBuf;

use citum_schema::InputBibliography;
use citum_schema::reference::InputReference;

#[test]
fn test_data_model_examples_round_trip() {
    let mut examples_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    examples_dir.push("../../examples/data-model");

    let mut entries: Vec<PathBuf> = fs::read_dir(&examples_dir)
        .expect("Failed to read examples/data-model directory")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    entries.sort();

    assert!(
        !entries.is_empty(),
        "Expected at least one example under examples/data-model/"
    );

    for path in entries {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let content =
            fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {filename}"));

        let bib: InputBibliography = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"));
        assert_eq!(
            bib.references.len(),
            1,
            "{filename} should contain exactly one worked example reference"
        );

        let serialized = serde_yaml::to_string(&bib.references)
            .unwrap_or_else(|e| panic!("Failed to re-serialize {filename}: {e}"));
        let round_tripped: Vec<InputReference> = serde_yaml::from_str(&serialized)
            .unwrap_or_else(|e| panic!("Failed to re-parse serialized {filename}: {e}"));

        assert_eq!(
            bib.references, round_tripped,
            "{filename} did not round-trip through serialization"
        );
    }
}
