/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    options::{
        BibliographyConfig, Config, ContributorConfig, DisplayAsSort, Processing, ProcessingCustom,
        Sort, SortKey, SortSpec,
    },
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, NumberVariable,
        Rendering, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
    },
    BibliographySpec, CitationSpec, Style, StyleInfo,
};
use csln_processor::Processor;

// --- Helper Functions ---

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

fn build_sorted_style(sort: Vec<SortSpec>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Sorted Test".to_string()),
            id: Some("sort-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                sort: Some(Sort {
                    template: sort,
                    shorten_names: false,
                    render_substitutions: false,
                }),
                ..Default::default()
            })),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    ..Default::default()
                }),
                TemplateComponent::Date(TemplateDate {
                    date: TDateVar::Issued,
                    form: DateForm::Year,
                    rendering: Rendering {
                        prefix: Some(" ".to_string()),
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
                display_as_sort: Some(DisplayAsSort::First),
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

// --- Sorting Tests ---

#[test]
fn test_sorting_by_author() {
    let style = build_sorted_style(vec![SortSpec {
        key: SortKey::Author,
        ascending: true,
    }]);

    let mut bib = indexmap::IndexMap::new();
    bib.insert("z".to_string(), make_book("z", "Zoe", "Z", 2020, "Title Z"));
    bib.insert(
        "a".to_string(),
        make_book("a", "Adam", "A", 2020, "Title A"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Adam should come before Zoe
    assert!(result.find("Adam").unwrap() < result.find("Zoe").unwrap());
}

#[test]
fn test_sorting_by_year() {
    let style = build_sorted_style(vec![SortSpec {
        key: SortKey::Year,
        ascending: true,
    }]);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "J", 2022, "Title B"),
    );
    bib.insert(
        "item2".to_string(),
        make_book("item2", "Smith", "J", 2020, "Title A"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // 2020 should come before 2022
    assert!(result.find("2020").unwrap() < result.find("2022").unwrap());
}

#[test]
fn test_sorting_multiple_keys() {
    let style = build_sorted_style(vec![
        SortSpec {
            key: SortKey::Author,
            ascending: true,
        },
        SortSpec {
            key: SortKey::Year,
            ascending: false,
        },
    ]);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "J", 2020, "Title A"),
    );
    bib.insert(
        "item2".to_string(),
        make_book("item2", "Smith", "J", 2022, "Title B"),
    );
    bib.insert(
        "item3".to_string(),
        make_book("item3", "Adams", "A", 2021, "Title C"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Adams (2021) should be first
    // Then Smith (2022) - because descending year
    // Then Smith (2020)
    assert!(result.find("Adams").unwrap() < result.find("Smith, J 2022").unwrap());
    assert!(result.find("Smith, J 2022").unwrap() < result.find("Smith, J 2020").unwrap());
}

// --- Substitution Tests ---

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

// --- Numeric Bibliography Tests ---

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
