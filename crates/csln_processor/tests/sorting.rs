/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    options::{
        Config, ContributorConfig, DisplayAsSort, Processing, ProcessingCustom, Sort, SortKey,
        SortSpec,
    },
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate,
    },
    BibliographySpec, Style, StyleInfo,
};
use csln_processor::Processor;

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
