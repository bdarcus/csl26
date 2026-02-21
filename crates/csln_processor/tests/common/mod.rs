/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#![allow(dead_code)]

use csln_core::{
    CitationSpec, Style, StyleInfo,
    citation::{Citation, CitationItem, CitationMode},
    reference::{
        Contributor, ContributorList, EdtfString, InputReference as Reference, Monograph,
        MonographType, MultilingualString, Parent, Serial, SerialComponent, SerialComponentType,
        SerialType, StructuredName, Title,
    },
};
use csln_processor::Processor;

// --- Helper Functions for Test Data Construction ---

/// Create a native Reference for a book with minimal fields.
pub fn make_book(id: &str, family: &str, given: &str, year: i32, title: &str) -> Reference {
    csln_core::ref_book!(id, family, given, year, title)
}

/// Create a native Reference with multiple authors.
pub fn make_book_multi_author(
    id: &str,
    authors: Vec<(&str, &str)>,
    year: i32,
    title: &str,
) -> Reference {
    let author_list: Vec<Contributor> = authors
        .into_iter()
        .map(|(family, given)| {
            Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })
        })
        .collect();

    Reference::Monograph(Box::new(Monograph {
        id: Some(id.to_string()),
        r#type: MonographType::Book,
        title: Title::Single(title.to_string()),
        author: Some(Contributor::ContributorList(ContributorList(author_list))),
        editor: None,
        translator: None,
        issued: EdtfString(year.to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        note: None,
        isbn: None,
        doi: None,
        edition: None,
        report_number: None,
        collection_number: None,
        genre: None,
        medium: None,
        keywords: None,
        original_date: None,
        original_title: None,
    }))
}

/// Create a native Reference for an article-journal.
pub fn make_article(id: &str, family: &str, given: &str, year: i32, title: &str) -> Reference {
    csln_core::ref_article!(id, family, given, year, title)
}

/// Create a native Reference for an article-journal with multiple authors.
pub fn make_article_multi_author(
    id: &str,
    authors: Vec<(&str, &str)>,
    year: i32,
    title: &str,
) -> Reference {
    let author_list: Vec<Contributor> = authors
        .into_iter()
        .map(|(family, given)| {
            Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })
        })
        .collect();

    Reference::SerialComponent(Box::new(SerialComponent {
        id: Some(id.to_string()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Single(title.to_string())),
        author: Some(Contributor::ContributorList(ContributorList(author_list))),
        translator: None,
        issued: EdtfString(year.to_string()),
        parent: Parent::Embedded(Serial {
            r#type: SerialType::AcademicJournal,
            title: Title::Single(String::new()),
            editor: None,
            publisher: None,
            issn: None,
        }),
        url: None,
        accessed: None,
        language: None,
        note: None,
        doi: None,
        pages: None,
        volume: None,
        issue: None,
        genre: None,
        medium: None,
        keywords: None,
    }))
}

#[allow(clippy::too_many_arguments)]
pub fn make_multilingual_book(
    id: &str,
    original_family: &str,
    original_given: &str,
    lang: &str,
    translit_script: &str,
    translit_family: &str,
    translit_given: &str,
    year: i32,
    title: &str,
) -> Reference {
    use csln_core::reference::contributor::MultilingualName;
    use std::collections::HashMap;

    let mut transliterations = HashMap::new();
    transliterations.insert(
        translit_script.to_string(),
        StructuredName {
            family: MultilingualString::Simple(translit_family.to_string()),
            given: MultilingualString::Simple(translit_given.to_string()),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        },
    );

    Reference::Monograph(Box::new(Monograph {
        id: Some(id.to_string()),
        r#type: MonographType::Book,
        title: Title::Single(title.to_string()),
        author: Some(Contributor::Multilingual(MultilingualName {
            original: StructuredName {
                family: MultilingualString::Simple(original_family.to_string()),
                given: MultilingualString::Simple(original_given.to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            },
            lang: Some(lang.to_string()),
            transliterations,
            translations: HashMap::new(),
        })),
        editor: None,
        translator: None,
        issued: EdtfString(year.to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        note: None,
        isbn: None,
        doi: None,
        edition: None,
        report_number: None,
        collection_number: None,
        genre: None,
        medium: None,
        keywords: None,
        original_date: None,
        original_title: None,
    }))
}

// --- Test Execution Helpers ---

/// Execute a test case with default disambiguation settings (year_suffix only).
pub fn run_test_case_native(
    input: &[Reference],
    citation_items: &[Vec<&str>],
    expected: &str,
    mode: &str,
) {
    run_test_case_native_with_options(
        input,
        citation_items,
        expected,
        mode,
        true,
        false,
        false,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
/// Execute a test case with custom disambiguation settings.
pub fn run_test_case_native_with_options(
    input: &[Reference],
    citation_items: &[Vec<&str>],
    expected: &str,
    mode: &str,
    disambiguate_year_suffix: bool,
    disambiguate_names: bool,
    disambiguate_givenname: bool,
    et_al_min: Option<u8>,
    et_al_use_first: Option<u8>,
) {
    // Create author-date style with customizable disambiguation options
    let style = build_author_date_style(
        disambiguate_year_suffix,
        disambiguate_names,
        disambiguate_givenname,
        et_al_min,
        et_al_use_first,
    );

    // Build bibliography from native references
    let mut bibliography = indexmap::IndexMap::new();
    for item in input.iter() {
        if let Some(id) = item.id() {
            bibliography.insert(id, item.clone());
        }
    }

    let processor = Processor::new(style, bibliography);

    if mode == "citation" {
        let mut results = Vec::new();

        for batch in citation_items {
            let items: Vec<CitationItem> = batch
                .iter()
                .map(|id| CitationItem {
                    id: id.to_string(),
                    ..Default::default()
                })
                .collect();

            let citation = Citation {
                items,
                mode: CitationMode::NonIntegral,
                ..Default::default()
            };

            let res = processor
                .process_citation(&citation)
                .expect("Failed to process citation");
            results.push(res);
        }

        let actual = results.join("\n");
        assert_eq!(actual.trim(), expected.trim(), "Citation output mismatch");
    } else if mode == "bibliography" {
        if !citation_items.is_empty() {
            for batch in citation_items {
                let items: Vec<CitationItem> = batch
                    .iter()
                    .map(|id| CitationItem {
                        id: id.to_string(),
                        ..Default::default()
                    })
                    .collect();
                let citation = Citation {
                    items,
                    ..Default::default()
                };
                processor.process_citation(&citation).ok();
            }
        }

        let actual = processor.render_bibliography();
        assert_eq!(
            actual.trim(),
            expected.trim(),
            "Bibliography output mismatch"
        );
    }
}

/// Build an author-date style with customizable disambiguation options.
pub fn build_author_date_style(
    disambiguate_year_suffix: bool,
    disambiguate_names: bool,
    disambiguate_givenname: bool,
    et_al_min: Option<u8>,
    et_al_use_first: Option<u8>,
) -> Style {
    use csln_core::options::{
        Config, ContributorConfig, Disambiguation, Processing, ProcessingCustom, ShortenListOptions,
    };
    use csln_core::template::WrapPunctuation;

    // Build disambiguation config
    let disambiguate = if disambiguate_year_suffix || disambiguate_names || disambiguate_givenname {
        Some(Disambiguation {
            year_suffix: disambiguate_year_suffix,
            names: disambiguate_names,
            add_givenname: disambiguate_givenname,
        })
    } else {
        None
    };

    // Build contributors config with et-al settings and initialize_with for initials
    let contributors = Some(ContributorConfig {
        shorten: if et_al_min.is_some() || et_al_use_first.is_some() {
            Some(ShortenListOptions {
                min: et_al_min.unwrap_or(3),
                use_first: et_al_use_first.unwrap_or(1),
                ..Default::default()
            })
        } else {
            None
        },
        initialize_with: Some(" ".to_string()),
        ..Default::default()
    });

    // Citation template: Author (Year)
    let citation_template = vec![
        csln_core::tc_contributor!(Author, Short),
        csln_core::tc_date!(Issued, Year, wrap = WrapPunctuation::Parentheses),
    ];

    Style {
        info: StyleInfo {
            title: Some("Author-Date Disambiguation Test".to_string()),
            id: Some("http://test.example/disambiguation".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                disambiguate,
                ..Default::default()
            })),
            contributors,
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            sort: Some(csln_core::grouping::GroupSort {
                template: vec![
                    csln_core::grouping::GroupSortKey {
                        key: csln_core::grouping::SortKey::Author,
                        ascending: true,
                        order: None,
                        sort_order: None,
                    },
                    csln_core::grouping::GroupSortKey {
                        key: csln_core::grouping::SortKey::Issued,
                        ascending: true,
                        order: None,
                        sort_order: None,
                    },
                ],
            }),
            template: Some(citation_template),
            multi_cite_delimiter: Some("; ".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }
}
