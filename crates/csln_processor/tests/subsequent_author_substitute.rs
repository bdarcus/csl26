/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    options::{BibliographyConfig, Config, ContributorConfig, Processing},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate,
    },
    BibliographySpec, Style, StyleInfo,
};
use csln_processor::Processor;

fn make_style_with_substitute(substitute: Option<String>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Subsequent Author Substitute Test".to_string()),
            id: Some("sub-test".to_string()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            bibliography: Some(BibliographyConfig {
                subsequent_author_substitute: substitute,
                entry_suffix: Some(".".to_string()),
                ..Default::default()
            }),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(csln_core::options::DisplayAsSort::First),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: None,
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn test_subsequent_author_substitute() {
    let style = make_style_with_substitute(Some("———".to_string()));

    let mut bib = indexmap::IndexMap::new();
    let ref1 = make_book("ref1", "Smith", "John", 2020, "Book A");
    let ref2 = make_book("ref2", "Smith", "John", 2021, "Book B");

    bib.insert("ref1".to_string(), ref1);
    bib.insert("ref2".to_string(), ref2);

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // ref1 comes first (2020), then ref2 (2021). ref2 should have substituted author.
    // Note: Implicit separator ". " + Implicit suffix "."
    let expected = "Smith, John. 2020.\n\n———. 2021.";
    assert_eq!(result, expected);
}

#[test]
fn test_no_substitute_if_different() {
    let style = make_style_with_substitute(Some("———".to_string()));

    let mut bib = indexmap::IndexMap::new();
    let ref1 = make_book("ref1", "Smith", "John", 2020, "Book A");
    let ref2 = make_book("ref2", "Doe", "Jane", 2021, "Book B");

    bib.insert("ref1".to_string(), ref1);
    bib.insert("ref2".to_string(), ref2);

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Doe comes before Smith alphabetically
    let expected = "Doe, Jane. 2021.\n\nSmith, John. 2020.";
    assert_eq!(result, expected);
}
