/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Behaviour tests for numbering accessors.

use super::{InputReference, NumOrStr, NumberingType};

fn parse_reference(json: &str) -> InputReference {
    serde_json::from_str(json).expect("reference should parse")
}

#[test]
fn shorthand_numbering_accessors_cover_all_numbered_reference_variants() {
    let cases = [
        (
            "monograph",
            r#"{
                    "class": "monograph",
                    "type": "book",
                    "title": "Book",
                    "issued": "2024",
                    "volume": "1",
                    "issue": "2",
                    "edition": "3",
                    "number": "4"
                }"#,
        ),
        (
            "collection",
            r#"{
                    "class": "collection",
                    "type": "anthology",
                    "title": "Collection",
                    "issued": "2024",
                    "volume": "1",
                    "issue": "2",
                    "edition": "3",
                    "number": "4"
                }"#,
        ),
        (
            "collection-component",
            r#"{
                    "class": "collection-component",
                    "type": "chapter",
                    "title": "Chapter",
                    "issued": "2024",
                    "volume": "1",
                    "issue": "2",
                    "edition": "3",
                    "number": "4"
                }"#,
        ),
        (
            "serial-component",
            r#"{
                    "class": "serial-component",
                    "type": "article",
                    "title": "Article",
                    "issued": "2024",
                    "volume": "1",
                    "issue": "2",
                    "edition": "3",
                    "number": "4"
                }"#,
        ),
        (
            "classic",
            r#"{
                    "class": "classic",
                    "title": "Classic",
                    "issued": "2024",
                    "volume": "1",
                    "issue": "2",
                    "edition": "3",
                    "number": "4"
                }"#,
        ),
    ];

    for (label, json) in cases {
        let reference = parse_reference(json);

        assert_eq!(
            reference.volume(),
            Some(NumOrStr::Str("1".to_string())),
            "{label} should resolve volume"
        );
        assert_eq!(
            reference.issue(),
            Some(NumOrStr::Str("2".to_string())),
            "{label} should resolve issue"
        );
        assert_eq!(
            reference.edition(),
            Some("3".to_string()),
            "{label} should resolve edition"
        );
        assert_eq!(
            reference.number(),
            Some("4".to_string()),
            "{label} should resolve number"
        );
        assert_eq!(
            reference.report_number(),
            None,
            "{label} should not resolve report number"
        );
    }
}

#[test]
fn number_of_volumes_accessor_resolves_the_canonical_numbering_value() {
    let reference = parse_reference(
        r#"{
            "class": "monograph",
            "type": "book",
            "title": "A multi-volume work",
            "numbering": [
                { "type": "number-of-volumes", "value": "3 vols. in 9 bks." }
            ]
        }"#,
    );

    assert_eq!(
        reference.number_of_volumes(),
        Some("3 vols. in 9 bks.".to_string())
    );
    assert_eq!(
        reference.numbering_value(&NumberingType::Custom("number-of-volumes".to_string())),
        Some("3 vols. in 9 bks.".to_string())
    );
}

#[test]
fn report_number_accessor_stays_separate_from_generic_number() {
    let reference = parse_reference(
        r#"{
                "class": "monograph",
                "type": "report",
                "title": "Report",
                "issued": "2024",
                "numbering": [
                    { "type": "report", "value": "TR-42" }
                ]
            }"#,
    );

    assert_eq!(reference.number(), None);
    assert_eq!(reference.report_number(), Some("TR-42".to_string()));
}

#[test]
fn numbering_value_accessor_resolves_custom_numbering_without_changing_builtin_accessors() {
    let reference = parse_reference(
        r#"{
                "class": "monograph",
                "type": "book",
                "title": "Score",
                "issued": "2024",
                "numbering": [
                    { "type": "movement", "value": "II" }
                ]
            }"#,
    );

    assert_eq!(
        reference.numbering_value(&NumberingType::Custom("movement".to_string())),
        Some("II".to_string())
    );
    assert_eq!(reference.number(), None);
    assert_eq!(reference.report_number(), None);
}

#[test]
fn numbering_value_accessor_normalizes_manual_custom_numbering_keys() {
    let reference = parse_reference(
        r#"{
                "class": "monograph",
                "type": "book",
                "title": "Score",
                "issued": "2024",
                "numbering": [
                    { "type": "movement", "value": "II" }
                ]
            }"#,
    );

    assert_eq!(
        reference.numbering_value(&NumberingType::Custom("Movement".to_string())),
        Some("II".to_string())
    );
}

#[test]
fn collection_component_collection_number_bubbles_from_embedded_container() {
    let reference = parse_reference(
        r#"{
                "class": "collection-component",
                "type": "chapter",
                "title": "Chapter",
                "issued": "2024",
                "container": {
                    "class": "collection",
                    "type": "edited-book",
                    "title": "Series",
                    "issued": "2024",
                    "volume": "7"
                }
            }"#,
    );

    assert_eq!(reference.collection_number(), Some("7".to_string()));
    assert_eq!(
        reference.volume(),
        Some(NumOrStr::Str("7".to_string())),
        "volume should bubble from the embedded parent monograph"
    );
}

#[test]
fn collection_component_entry_encyclopedia_genre_preserves_ref_type() {
    let reference = parse_reference(
        r#"{
                "class": "collection-component",
                "type": "chapter",
                "title": "Renaissance Art and Culture",
                "genre": "entry-encyclopedia",
                "issued": "2022",
                "container": {
                    "class": "collection",
                    "type": "edited-book",
                    "title": "Encyclopedia of World History",
                    "issued": "2022"
                }
            }"#,
    );

    assert_eq!(reference.ref_type(), "entry-encyclopedia");
}
