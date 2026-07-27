/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Contract test: every CSL 1.0.2 item type in
//! [`csl_legacy::csl_json::CSL_TYPES`] converts to an `InputReference`
//! whose `ref_type()` is a faithful round trip, not a silent collapse into
//! the generic document/monograph fallback. See
//! `docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md` for the canonicalization
//! rules and the rationale behind each intentional divergence documented on
//! [`super::CSL_TYPE_MAP`], which this test asserts against.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking is acceptable and often desired in tests."
)]

use super::*;
use csl_legacy::csl_json::CSL_TYPES;
use serde_json::json;

#[test]
fn structured_circa_csl_dates_convert_to_approximate_edtf() {
    // A `c1988`-shaped *literal* is CSL's copyright-year convention (GB/T
    // 7714 §7.5.4.3), not circa — see `copyright_year_from_legacy` and
    // `docs/specs/DATE_MODEL.md`. Only the structured `circa: true` flag
    // maps to EDTF approximate (`~`).
    let structured: DateValue = csl_legacy::csl_json::DateVariable {
        date_parts: Some(vec![vec![1988]]),
        circa: Some(true),
        ..Default::default()
    }
    .into();

    assert_eq!(structured.value, "1988~");
}

/// Build the minimal legacy reference the contract test converts for a
/// given CSL type: an id, the type under test, a title, and an issued
/// year. This is deliberately the *smallest* shape a real CSL-JSON export
/// would carry. A type that needs more than this to avoid the generic
/// document/monograph fallback is either a genuine routing gap (fix the
/// converter, not this helper) or a documented, shape-dependent
/// divergence (see the comments on [`super::CSL_TYPE_MAP`] and the spec).
fn minimal_reference(ref_type: &str) -> csl_legacy::csl_json::Reference {
    csl_legacy::csl_json::Reference {
        id: format!("contract-{ref_type}"),
        ref_type: ref_type.to_string(),
        title: Some("Contract Test Title".to_string()),
        issued: Some(csl_legacy::csl_json::DateVariable {
            date_parts: Some(vec![vec![2024]]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn every_csl_1_0_2_type_has_an_expectation_table_entry() {
    for csl_type in CSL_TYPES {
        assert!(
            CSL_TYPE_MAP.iter().any(|row| &row.csl_type == csl_type),
            "CSL_TYPES entry `{csl_type}` has no entry in `CSL_TYPE_MAP`; \
             every CSL 1.0.2 type must be covered"
        );
    }
    assert_eq!(
        CSL_TYPE_MAP.len(),
        CSL_TYPES.len(),
        "CSL_TYPE_MAP size has drifted from CSL_TYPES; add or remove \
         an entry so the two stay in lockstep"
    );
}

#[test]
fn every_csl_1_0_2_type_round_trips_through_ref_type() {
    // Collect every mismatch instead of stopping at the first one: a
    // routing regression usually breaks more than one type at a time, and
    // seeing the whole set in one run is what turns this into a fast
    // diagnostic instead of a bisection exercise (see the epic's problem
    // statement in bean csl26-cvfy).
    let failures: Vec<String> = CSL_TYPE_MAP
        .iter()
        .filter_map(|row| {
            let legacy = minimal_reference(row.csl_type);
            let actual = InputReference::from(legacy).ref_type();
            (actual != row.citum_ref_type).then(|| {
                format!(
                    "{}: expected `{}`, got `{actual}`",
                    row.csl_type, row.citum_ref_type
                )
            })
        })
        .collect();

    assert!(
        failures.is_empty(),
        "CSL type round-trip mismatches (see docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md):\n{}",
        failures.join("\n")
    );
}

/// Regression test for the `TLIB-SEL-MAP-1` fixture case
/// (`tests/fixtures/references-expanded.json`): a `map` reference whose
/// user-supplied genre uses a capitalized export label (`"Map"`, as
/// Zotero emits) must still round-trip to the canonical lowercase `map`,
/// not fall through the genre back-map into `document`. The back-map has
/// matched genre case-insensitively since before the routing closure;
/// this pins that behavior for all genre-discriminated document types.
#[test]
fn map_with_capitalized_export_genre_still_round_trips_as_map() {
    let mut legacy = minimal_reference("map");
    legacy.genre = Some("Map".to_string());

    let converted = InputReference::from(legacy);

    assert_eq!(converted.ref_type(), "map");
}

/// Regression test for the `chi-manuscript` fixture case (bean
/// `csl26-shco`): a reference with top-level `"type": "manuscript"` and a
/// note-field override `"type: collection"` must round-trip as
/// `collection`, not silently collapse into the generic `document`
/// fallback. CSL 1.0.2's `collection` is an *archival* collection
/// (author, archive, archive-place), so it converts to the archival
/// monograph/document shape — which carries those fields — with a
/// genre-discriminated round trip, not to the editorial
/// `ClassExtension::Collection` (anthology/proceedings), which has no
/// author or archive fields and would drop them. See
/// `docs/specs/CSL_TYPE_CONVERSION_CONTRACT.md`.
#[test]
fn manuscript_with_recognized_collection_note_override_converts_to_collection() {
    let mut legacy = minimal_reference("manuscript");
    legacy.note = Some("type: collection".to_string());
    legacy.archive = Some("University of Georgia Library".to_string());
    legacy.parse_note_field_hacks();

    assert_eq!(legacy.ref_type, "collection");

    let converted = InputReference::from(legacy);

    assert_eq!(converted.ref_type(), "collection");
    let crate::reference::ClassExtension::Monograph(monograph) = converted.extension() else {
        panic!(
            "expected the archival Monograph shape for a `collection` conversion, got `{}`",
            converted.ref_type()
        );
    };
    assert_eq!(
        monograph.archive.as_deref(),
        Some("University of Georgia Library"),
        "archival fields must survive the collection conversion"
    );
}

#[test]
fn book_volume_title_note_field_survives_as_monograph_metadata() {
    let mut legacy = minimal_reference("book");
    legacy.note = Some("volume-title: History of Science".to_string());
    legacy.parse_note_field_hacks();

    let converted = InputReference::from(legacy);
    let crate::reference::ClassExtension::Monograph(monograph) = converted.extension() else {
        panic!("book must convert to a Monograph");
    };

    assert_eq!(
        monograph.volume_title.as_deref(),
        Some("History of Science")
    );
    assert!(monograph.container.is_none());
}

#[test]
fn map_scale_and_dimensions_survive_as_monograph_metadata() {
    let mut legacy = minimal_reference("map");
    legacy.note = Some("dimensions: 128cm×84cm".to_string());
    legacy.extra.insert("scale".to_string(), json!("1:25000"));
    legacy.parse_note_field_hacks();

    let converted = InputReference::from(legacy);
    let crate::reference::ClassExtension::Monograph(monograph) = converted.extension() else {
        panic!("map must convert to a Monograph");
    };

    assert_eq!(monograph.scale.as_deref(), Some("1:25000"));
    assert_eq!(monograph.size.as_deref(), Some("128cm×84cm"));
}

#[test]
fn standalone_article_version_note_field_survives_on_preprint() {
    let mut legacy = minimal_reference("article");
    legacy.note = Some("version: 2".to_string());
    legacy.parse_note_field_hacks();

    let converted = InputReference::from(legacy);
    assert_eq!(converted.ref_type(), "preprint");
    let crate::reference::ClassExtension::Monograph(monograph) = converted.extension() else {
        panic!("standalone article must convert to a preprint Monograph");
    };

    assert_eq!(monograph.version.as_deref(), Some("2"));
}
