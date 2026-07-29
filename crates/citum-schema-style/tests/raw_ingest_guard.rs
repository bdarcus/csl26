/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Guards csl26-j3zy: `Style` must only ever be deserialized through
//! [`citum_schema_style::Style::from_yaml_str`],
//! [`citum_schema_style::Style::from_yaml_bytes`], or
//! [`citum_schema_style::Style::from_document_bytes`]
//! (`crates/citum-schema-style/src/style/model.rs`), so that `raw_yaml` is
//! always populated for null-aware `extends` overlay merging (see
//! `docs/specs/STYLE_INHERITANCE.md`).
//!
//! This forbids the turbofish form of a direct bypass
//! (`serde_yaml::from_slice::<Style>(..)`, `serde_yaml::from_slice::<citum_schema::Style>(..)`,
//! and siblings) anywhere else in the repo — the walk starts at the
//! workspace root, so it covers `fuzz/`, `tests/`, `examples/`, and
//! `scripts/` alongside `crates/`, skipping only hidden directories,
//! `target/`, and `node_modules/`. It cannot catch a bypass hidden behind
//! return-type inference
//! (`let style: Style = serde_yaml::from_slice(bytes)?;`) — that shape must
//! be caught by review. See `docs/guides/CODING_STANDARDS.md`.

#![allow(missing_docs, reason = "test crate")]
#![allow(
    clippy::string_slice,
    reason = "slice bounds come from `find` (pattern-end, always a char boundary) \
              and `char_indices` (always a char boundary by construction)"
)]

use std::path::Path;

/// Prefixes of a raw-format deserializer call whose turbofish type argument
/// must not name `Style`, however qualified (`Style`, `citum_schema::Style`,
/// `crate::Style`, `Vec<Style>`, ...).
const DESERIALIZER_PREFIXES: &[&str] = &[
    "serde_yaml::from_slice::<",
    "serde_yaml::from_str::<",
    "serde_yaml_ng::from_slice::<",
    "serde_yaml_ng::from_str::<",
    "serde_json::from_slice::<",
    "serde_json::from_str::<",
    "ciborium::de::from_reader::<",
];

/// Files allowed to reference the patterns above: the raw-preserving
/// constructors themselves, and this guard's own source (which must quote
/// the patterns to check for them).
const ALLOWED_FILE_SUFFIXES: &[&str] = &[
    "citum-schema-style/src/style/model.rs",
    "citum-schema-style/tests/raw_ingest_guard.rs",
];

#[test]
fn no_direct_turbofish_style_deserialization_outside_constructor() {
    let workspace_root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));

    let mut offenders = Vec::new();
    visit_rust_files(workspace_root, &mut |path, contents| {
        let path_str = path.to_string_lossy().replace('\\', "/");
        if ALLOWED_FILE_SUFFIXES
            .iter()
            .any(|suffix| path_str.ends_with(suffix))
        {
            return;
        }
        for prefix in DESERIALIZER_PREFIXES {
            if let Some(target) = turbofish_target_naming_style(contents, prefix) {
                offenders.push(format!("{path_str}: `{prefix}{target}>`"));
            }
        }
    });

    assert!(
        offenders.is_empty(),
        "found a direct turbofish Style deserialization outside \
         Style::from_document_bytes/from_yaml_bytes/from_yaml_str; raw_yaml \
         would be silently unpopulated on this load path, breaking \
         explicit-`null` extends clearing (see csl26-j3zy):\n{}",
        offenders.join("\n")
    );
}

/// If `contents` contains a call `<prefix><type-args>>(...)` whose type
/// arguments name the identifier `Style` (bare or qualified, e.g.
/// `citum_schema::Style` or `Vec<Style>`), return the type-argument text.
fn turbofish_target_naming_style<'a>(contents: &'a str, prefix: &str) -> Option<&'a str> {
    let mut search_from = 0;
    while let Some(idx) = contents[search_from..].find(prefix) {
        let start = search_from + idx + prefix.len();
        let rest = &contents[start..];
        let mut depth = 1i32;
        let mut end = None;
        for (i, ch) in rest.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            let segment = &rest[..end];
            let names_style = segment
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|token| token == "Style");
            if names_style {
                return Some(segment);
            }
        }
        search_from = start;
    }
    None
}

/// Directory names skipped everywhere in the walk: build output, vendored/
/// dependency trees, and VCS/tooling metadata — none of these hold workspace
/// source we need to check, and `target`/`node_modules` can be large.
const SKIPPED_DIR_NAMES: &[&str] = &["target", "node_modules"];

fn visit_rust_files(dir: &Path, visit: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let is_hidden = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('.'));
            let is_skipped = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| SKIPPED_DIR_NAMES.contains(&n));
            if is_hidden || is_skipped {
                continue;
            }
            visit_rust_files(&path, visit);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
            && let Ok(contents) = std::fs::read_to_string(&path)
        {
            visit(&path, &contents);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod self_test {
    use super::turbofish_target_naming_style;

    #[test]
    fn detects_bare_and_qualified_style_turbofish() {
        assert!(
            turbofish_target_naming_style(
                "serde_yaml::from_slice::<Style>(bytes)",
                "serde_yaml::from_slice::<"
            )
            .is_some()
        );
        assert!(
            turbofish_target_naming_style(
                "serde_yaml::from_slice::<citum_schema::Style>(bytes)",
                "serde_yaml::from_slice::<"
            )
            .is_some()
        );
        assert!(
            turbofish_target_naming_style(
                "serde_json::from_slice::<Vec<Style>>(bytes)",
                "serde_json::from_slice::<"
            )
            .is_some()
        );
    }

    #[test]
    fn ignores_unrelated_types_and_style_prefixed_names() {
        assert!(
            turbofish_target_naming_style(
                "serde_yaml::from_slice::<StyleInfo>(bytes)",
                "serde_yaml::from_slice::<"
            )
            .is_none()
        );
        assert!(
            turbofish_target_naming_style(
                "serde_json::from_slice::<Locale>(bytes)",
                "serde_json::from_slice::<"
            )
            .is_none()
        );
    }
}
