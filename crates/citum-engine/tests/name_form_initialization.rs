/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in test, benchmark, and example code."
)]

/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

mod common;
use common::*;

use citum_engine::Processor;
use citum_schema::{
    CitationSpec, Style, StyleInfo,
    citation::{Citation, CitationItem},
    options::{Config, ContributorConfig, NameForm},
};
use rstest::rstest;

/// Build a test style with configurable name-form and initialize-with options.
fn build_name_form_test_style(
    name_form: Option<NameForm>,
    initialize_with: Option<String>,
) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Name Form Test".to_string()),
            id: Some("name-form-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            contributors: Some(ContributorConfig {
                name_form,
                initialize_with,
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Test matrix: (name_form, initialize_with, expected_output)
#[rstest]
#[case(Some(NameForm::Full), None, "John Samuel Doe")]
#[case(Some(NameForm::Initials), Some("".to_string()), "JS Doe")]
#[case(Some(NameForm::Initials), Some(" ".to_string()), "J S Doe")]
#[case(Some(NameForm::Initials), Some(". ".to_string()), "J. S. Doe")]
#[case(None, None, "John Samuel Doe")] // Default is Full
#[case(None, Some("".to_string()), "John Samuel Doe")] // initialize-with alone has no effect without name-form: initials
fn name_form_controls_given_name_rendering(
    #[case] name_form: Option<NameForm>,
    #[case] initialize_with: Option<String>,
    #[case] expected_output: &str,
) {
    let style = build_name_form_test_style(name_form, initialize_with);
    let bibliography = citum_schema::bib_map![
        "kuhn-1962" => make_book(
            "kuhn-1962",
            "Doe",
            "John Samuel",
            1962,
            "The Structure of Scientific Revolutions",
        )
    ];

    let processor = Processor::new(style, bibliography);
    let citation = Citation {
        items: vec![CitationItem {
            id: "kuhn-1962".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let rendered = processor
        .process_citation(&citation)
        .expect("Failed to render citation");

    // Extract just the author name part (before any dates)
    let author_part = rendered
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        author_part, expected_output,
        "Name form + initialize-with combination incorrect"
    );
}
