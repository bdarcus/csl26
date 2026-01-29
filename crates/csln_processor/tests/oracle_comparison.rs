/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Integration test comparing CSLN processor output with citeproc-js.

use csln_core::options::{
    AndOptions, ContributorConfig, DisplayAsSort, Processing, ShortenListOptions,
};
use csln_core::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
    TemplateComponent, TemplateContributor, TemplateDate, TemplateTitle, TitleType,
    WrapPunctuation,
};
use csln_core::{BibliographySpec, CitationSpec, Style, StyleInfo};
use csln_processor::{Citation, CitationItem, DateVariable, Name, Processor, Reference};
use std::collections::HashMap;

fn make_apa_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("APA 7th Edition".to_string()),
            id: Some("apa".to_string()),
            ..Default::default()
        },
        options: Some(csln_core::options::Config {
            processing: Some(Processing::AuthorDate),
            contributors: Some(ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 3,
                    use_first: 1,
                    ..Default::default()
                }),
                and: Some(AndOptions::Symbol),
                display_as_sort: Some(DisplayAsSort::First),
                initialize_with: Some(". ".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            options: None,
            template: vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Short,
                    name_order: None,
                    delimiter: None,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: None,
            template: vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    name_order: None,
                    delimiter: None,
                    rendering: Rendering::default(),
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        wrap: Some(WrapPunctuation::Parentheses),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    form: None,
                    rendering: Rendering {
                        emph: Some(true),
                        ..Default::default()
                    },
                    overrides: None,
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }),
        templates: None,
        ..Default::default()
    }
}

fn make_test_bibliography() -> HashMap<String, Reference> {
    let mut bib = HashMap::new();

    // Kuhn 1962
    bib.insert(
        "kuhn1962".to_string(),
        Reference {
            id: "kuhn1962".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            issued: Some(DateVariable::year(1962)),
            publisher: Some("University of Chicago Press".to_string()),
            publisher_place: Some("Chicago".to_string()),
            ..Default::default()
        },
    );

    // Multi-author article (triggers et al.)
    bib.insert(
        "lecun2015".to_string(),
        Reference {
            id: "lecun2015".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ]),
            title: Some("Deep learning".to_string()),
            container_title: Some("Nature".to_string()),
            issued: Some(DateVariable::year(2015)),
            volume: Some(csln_processor::StringOrNumber::Number(521)),
            page: Some("436-444".to_string()),
            ..Default::default()
        },
    );

    bib
}

#[test]
fn test_apa_single_author_citation() {
    let style = make_apa_style();
    let bib = make_test_bibliography();
    let processor = Processor::new(style, bib);

    let citation = Citation {
        id: Some("c1".to_string()),
        items: vec![CitationItem {
            id: "kuhn1962".to_string(),
            ..Default::default()
        }],
    };

    let result = processor.process_citation(&citation).unwrap();

    // Expected: (Kuhn, 1962) - matches citeproc-js APA output
    assert_eq!(result, "(Kuhn, 1962)");
}

#[test]
fn test_apa_multi_author_citation_et_al() {
    let style = make_apa_style();
    let bib = make_test_bibliography();
    let processor = Processor::new(style, bib);

    let citation = Citation {
        id: Some("c2".to_string()),
        items: vec![CitationItem {
            id: "lecun2015".to_string(),
            ..Default::default()
        }],
    };

    let result = processor.process_citation(&citation).unwrap();

    // Expected: (LeCun et al., 2015) - matches citeproc-js APA output for 3+ authors
    assert_eq!(result, "(LeCun et al., 2015)");
}

#[test]
fn test_apa_bibliography_entry() {
    let style = make_apa_style();
    let bib = make_test_bibliography();
    let processor = Processor::new(style, bib);

    let result = processor.render_bibliography();

    // Check Kuhn entry has correct format
    assert!(result.contains("Kuhn, T."), "Should have 'Kuhn, T.'");
    assert!(result.contains("(1962)"), "Should have '(1962)'");
    assert!(
        result.contains("_The Structure of Scientific Revolutions_"),
        "Should have italicized title"
    );
}

#[test]
fn test_reference_not_found_error() {
    let style = make_apa_style();
    let bib = make_test_bibliography();
    let processor = Processor::new(style, bib);

    let citation = Citation {
        id: Some("bad".to_string()),
        items: vec![CitationItem {
            id: "nonexistent".to_string(),
            ..Default::default()
        }],
    };

    let result = processor.process_citation(&citation);
    assert!(result.is_err());
}
