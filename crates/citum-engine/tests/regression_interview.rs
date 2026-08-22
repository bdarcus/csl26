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
use citum_schema::reference::{
    Contributor, ContributorEntry, ContributorRole, DateValue, InputReference, Monograph,
    MonographType, Publisher, StructuredName, Title,
};
use indexmap::IndexMap;

#[test]
fn test_apa_interview_fidelity_regression() {
    let style = load_style("styles/embedded/apa-7th.yaml");

    // Create the interview reference using native Citum structs
    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("sr-interview".into()),
        r#type: MonographType::Interview,
        title: Some(Title::Single("Thinking in Public".to_string())),
        author: Some(Contributor::StructuredName(StructuredName {
            family: "Arendt".into(),
            given: "Hannah".into(),
            ..Default::default()
        })),
        contributors: vec![ContributorEntry {
            roles: ContributorRole::Interviewer.into(),
            contributor: Contributor::StructuredName(StructuredName {
                family: "Young-Bruehl".into(),
                given: "Elisabeth".into(),
                ..Default::default()
            }),
            gender: None,
        }],
        issued: DateValue::new("1975".to_string()),
        publisher: Some(Publisher {
            name: "Schocken Books".into(),
            place: None,
        }),
        ..Default::default()
    }));

    let mut bib = IndexMap::new();
    bib.insert("sr-interview".to_string(), reference);

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // APA expected output for interview
    assert_eq!(
        result,
        "Arendt, H. (1975). Thinking in Public (E. Young-Bruehl, Interviewer) Schocken Books."
    );
}

#[test]
fn test_chicago_author_date_interview_moves_period_inside_quote() {
    // Regression for the nested-group delimiter dynamics bug: the
    // `interview:` bibliography variant's outer group (title wrap:quotes,
    // date, interviewer, ...) has its own `delimiter: ". "`, joined by
    // `Renderer::render_group_component_with_format`
    // (processor/rendering/grouped/core.rs) — a *separate* group-join
    // implementation from `TemplateGroup::values` (values/list.rs). Only
    // fixing the latter left this path emitting `Intelligence". 2023.`
    // instead of `Intelligence.” 2023.` (period outside the closing quote).
    let style = load_style("styles/embedded/chicago-author-date-18th.yaml");

    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("bengio-interview".into()),
        r#type: MonographType::Interview,
        title: Some(Title::Single(
            "The Future of Artificial Intelligence".to_string(),
        )),
        author: Some(Contributor::StructuredName(StructuredName {
            family: "Bengio".into(),
            given: "Yoshua".into(),
            ..Default::default()
        })),
        contributors: vec![ContributorEntry {
            roles: ContributorRole::Interviewer.into(),
            contributor: Contributor::StructuredName(StructuredName {
                family: "Colbert".into(),
                given: "Stephen".into(),
                ..Default::default()
            }),
            gender: None,
        }],
        issued: DateValue::new("2023-11-10".to_string()),
        ..Default::default()
    }));

    let mut bib = IndexMap::new();
    bib.insert("bengio-interview".to_string(), reference);

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Chicago's interview bibliography variant routes the interviewer record
    // through the author-first branch before rendering the title and date.
    assert_eq!(
        result,
        "Bengio, Yoshua. 2023. \u{201C}The Future of Artificial Intelligence.\u{201D} Interview by Stephen Colbert. November 10."
    );
}
