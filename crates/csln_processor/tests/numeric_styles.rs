/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    options::{Config, Processing},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, NumberVariable,
        Rendering, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
    },
    BibliographySpec, CitationSpec, Style, StyleInfo,
};
use csln_processor::Processor;

fn build_numeric_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Numeric Test".to_string()),
            id: Some("numeric-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::CitationNumber,
                rendering: Rendering::default(),
                ..Default::default()
            })]),
            wrap: Some(csln_core::template::WrapPunctuation::Brackets),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(vec![
                TemplateComponent::Number(TemplateNumber {
                    number: NumberVariable::CitationNumber,
                    rendering: Rendering {
                        suffix: Some(". ".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        prefix: Some(" (".to_string()),
                        suffix: Some(")".to_string()),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn test_numeric_citation() {
    let style = build_numeric_style();

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Title A"),
    );
    bib.insert(
        "item2".to_string(),
        make_book("item2", "Doe", "Jane", 2021, "Title B"),
    );

    let processor = Processor::new(style, bib);

    let citation1 = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let citation2 = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item2".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.process_citation(&citation1).unwrap(), "[1]");
    assert_eq!(processor.process_citation(&citation2).unwrap(), "[2]");
}

#[test]
fn test_numeric_bibliography() {
    let style = build_numeric_style();

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Title A"),
    );

    let processor = Processor::new(style, bib);

    // Must process citation to assign number
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    processor.process_citation(&citation).unwrap();

    let result = processor.render_bibliography();
    assert_eq!(result, "1. John Smith (2020)");
}
