/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    locale::{GeneralTerm, TermForm},
    options::{Config, Processing},
    template::{DateForm, DateVariable, TemplateComponent, TemplateDate, TemplateTerm},
    CitationSpec, Style, StyleInfo,
};
use csln_processor::Processor;

fn build_date_style(form: DateForm) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Date Test".to_string()),
            id: Some("date-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![TemplateComponent::Date(TemplateDate {
                date: DateVariable::Issued,
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
