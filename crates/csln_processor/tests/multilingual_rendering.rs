/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    options::{Config, MultilingualConfig, MultilingualMode, Processing},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate,
    },
    CitationSpec, Style, StyleInfo,
};
use csln_processor::Processor;

fn build_ml_style(name_mode: MultilingualMode, preferred_script: Option<String>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Multilingual Test".to_string()),
            id: Some("ml-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            multilingual: Some(MultilingualConfig {
                name_mode: Some(name_mode),
                preferred_script,
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
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
            delimiter: Some(" ".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn test_multilingual_rendering_original() {
    let style = build_ml_style(MultilingualMode::Primary, None);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_multilingual_book(
            "item1", "東京", "太郎", "ja", "ja-Latn", "Tokyo", "Taro", 2020, "Title",
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
        "太郎 東京 2020"
    );
}

#[test]
fn test_multilingual_rendering_transliterated() {
    let style = build_ml_style(MultilingualMode::Transliterated, Some("Latn".to_string()));

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_multilingual_book(
            "item1", "東京", "太郎", "ja", "ja-Latn", "Tokyo", "Taro", 2020, "Title",
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
        "Taro Tokyo 2020"
    );
}

#[test]
fn test_multilingual_rendering_combined() {
    let style = build_ml_style(MultilingualMode::Combined, Some("Latn".to_string()));

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_multilingual_book(
            "item1", "東京", "太郎", "ja", "ja-Latn", "Tokyo", "Taro", 2020, "Title",
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

    // Note: Combined mode for names is currently transliterated only in resolve_multilingual_name
    assert_eq!(
        processor.process_citation(&citation).unwrap(),
        "Taro Tokyo 2020"
    );
}
