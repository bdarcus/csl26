/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    CitationSpec, Style, StyleInfo,
    locale::{GeneralTerm, TermForm},
    options::{Config, ContributorConfig, Processing, ShortenListOptions},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, TemplateComponent,
        TemplateContributor, TemplateDate, TemplateTerm,
    },
};
use csln_processor::Processor;

// --- Helper Functions ---

fn build_name_style(form: ContributorForm, shorten: Option<ShortenListOptions>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Name Test".to_string()),
            id: Some("name-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            contributors: Some(ContributorConfig {
                shorten,
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Author,
                form,
                ..Default::default()
            })]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_date_style(form: DateForm) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Date Test".to_string()),
            id: Some("date-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![TemplateComponent::Date(TemplateDate {
                date: TDateVar::Issued,
                form,
                fallback: Some(vec![TemplateComponent::Term(TemplateTerm {
                    term: GeneralTerm::NoDate,
                    form: Some(TermForm::Short),
                    ..Default::default()
                })]),
                ..Default::default()
            })]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// --- Name Rendering Tests ---

#[test]
fn test_name_rendering_basic() {
    let style = build_name_style(ContributorForm::Long, None);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Title"),
    );

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.process_citation(&citation).unwrap(), "John Smith");
}

#[test]
fn test_name_rendering_short() {
    let style = build_name_style(ContributorForm::Short, None);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Title"),
    );

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.process_citation(&citation).unwrap(), "Smith");
}

#[test]
fn test_name_rendering_et_al() {
    let style = build_name_style(
        ContributorForm::Short,
        Some(ShortenListOptions {
            min: 3,
            use_first: 1,
            ..Default::default()
        }),
    );

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book_multi_author(
            "item1",
            vec![("Smith", "John"), ("Doe", "Jane"), ("Brown", "Bob")],
            2020,
            "Title",
        ),
    );

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(
        processor.process_citation(&citation).unwrap(),
        "Smith et al."
    );
}

#[test]
fn test_name_rendering_particles() {
    let style = build_name_style(ContributorForm::Long, None);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Gogh", "Vincent", 1888, "Title");
    if let csln_core::reference::InputReference::Monograph(m) = &mut item
        && let Some(csln_core::reference::Contributor::StructuredName(n)) = &mut m.author
    {
        n.non_dropping_particle = Some("van".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(
        processor.process_citation(&citation).unwrap(),
        "Vincent van Gogh"
    );
}

#[test]
fn test_name_rendering_corporate() {
    let style = build_name_style(ContributorForm::Long, None);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "", "", 2020, "Title");
    if let csln_core::reference::InputReference::Monograph(m) = &mut item {
        m.author = Some(csln_core::reference::Contributor::SimpleName(
            csln_core::reference::SimpleName {
                name: csln_core::reference::MultilingualString::Simple(
                    "World Health Organization".to_string(),
                ),
                location: None,
            },
        ));
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(
        processor.process_citation(&citation).unwrap(),
        "World Health Organization"
    );
}

// --- Date Rendering Tests ---

#[test]
fn test_date_rendering_year() {
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "J", 2020, "Title"),
    );

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.process_citation(&citation).unwrap(), "2020");
}

#[test]
fn test_date_rendering_full() {
    let style = build_date_style(DateForm::Full);

    let mut bib = indexmap::IndexMap::new();
    // EDTF: 2020-05-15
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let csln_core::reference::InputReference::Monograph(m) = &mut item {
        m.issued = csln_core::reference::EdtfString("2020-05-15".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Default en-US full: "May 15, 2020"
    assert_eq!(
        processor.process_citation(&citation).unwrap(),
        "May 15, 2020"
    );
}

#[test]
fn test_date_rendering_range() {
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    // EDTF range: 2020/2022
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let csln_core::reference::InputReference::Monograph(m) = &mut item {
        m.issued = csln_core::reference::EdtfString("2020/2022".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Range en-dash: "2020–2022"
    assert_eq!(processor.process_citation(&citation).unwrap(), "2020–2022");
}

#[test]
fn test_date_rendering_open_range() {
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    // EDTF open range: 2020/..
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let csln_core::reference::InputReference::Monograph(m) = &mut item {
        m.issued = csln_core::reference::EdtfString("2020/..".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    // Open range: "2020–present" (using locale term)
    assert_eq!(
        processor.process_citation(&citation).unwrap(),
        "2020–present"
    );
}

#[test]
fn test_date_rendering_fallback() {
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    // Missing date
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let csln_core::reference::InputReference::Monograph(m) = &mut item {
        m.issued = csln_core::reference::EdtfString("".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    let citation = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.process_citation(&citation).unwrap(), "n.d.");
}
