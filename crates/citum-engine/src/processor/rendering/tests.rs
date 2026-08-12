/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::*;
use crate::Processor;
use crate::processor::rendering::grouped::group_citation_items_by_author;
use citum_schema::citation::{Citation, CitationItem, CitationMode, IntegralNameState};
use citum_schema::options::{
    Config, IntegralNameContexts, IntegralNameMemoryConfig, IntegralNameScope, LocatorPreset,
    Processing, SubsequentNameForm,
};
use citum_schema::template::*;
use citum_schema::{CitationSpec, Style, StyleInfo};
use csl_legacy::csl_json::{
    DateVariable as LegacyDateVariable, Name, Reference as LegacyReference,
};

fn make_reference(
    id: &str,
    ref_type: &str,
    author: Option<(&str, &str)>,
    year: i32,
    title: &str,
) -> Reference {
    Reference::from(LegacyReference {
        id: id.to_string(),
        ref_type: ref_type.to_string(),
        author: author.map(|(family, given)| vec![Name::new(family, given)]),
        title: Some(title.to_string()),
        issued: Some(LegacyDateVariable::year(year)),
        ..Default::default()
    })
}

fn grouped_author_date_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Grouped Author Date".to_string()),
            id: Some("grouped-author-date".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author.into(),
                        form: ContributorForm::Short,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: citum_schema::template::DateVariable::Issued,
                        form: DateForm::Year,
                        rendering: Rendering {
                            prefix: Some(", ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            wrap: Some(WrapPunctuation::Parentheses.into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn explicit_author_year_group_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Explicit Author Year Group".to_string()),
            id: Some("explicit-author-year-group".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            locators: Some(LocatorPreset::Note.config()),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![
                            TemplateComponent::Contributor(TemplateContributor {
                                contributor: ContributorRole::Author.into(),
                                form: ContributorForm::Short,
                                rendering: Rendering::default(),
                                ..Default::default()
                            }),
                            TemplateComponent::Date(TemplateDate {
                                date: citum_schema::template::DateVariable::Issued,
                                form: DateForm::Year,
                                rendering: Rendering::default(),
                                ..Default::default()
                            }),
                        ],
                        delimiter: Some(DelimiterPunctuation::Space),
                        rendering: Rendering::default(),
                        render_when: None,
                        custom: None,
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Locator,
                        rendering: Rendering {
                            prefix: Some(", ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            wrap: Some(WrapPunctuation::Parentheses.into()),
            delimiter: Some("".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn explicit_author_year_group_with_locator_delimiter_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Explicit Author Year Group With Locator Delimiter".to_string()),
            id: Some("explicit-author-year-group-with-locator-delimiter".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![
                            TemplateComponent::Contributor(TemplateContributor {
                                contributor: ContributorRole::Author.into(),
                                form: ContributorForm::Short,
                                rendering: Rendering::default(),
                                ..Default::default()
                            }),
                            TemplateComponent::Date(TemplateDate {
                                date: citum_schema::template::DateVariable::Issued,
                                form: DateForm::Year,
                                rendering: Rendering::default(),
                                ..Default::default()
                            }),
                        ],
                        delimiter: Some(DelimiterPunctuation::Space),
                        rendering: Rendering::default(),
                        render_when: None,
                        custom: None,
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Locator,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            wrap: Some(WrapPunctuation::Parentheses.into()),
            delimiter: Some(", ".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn integral_name_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Integral Name Memory".to_string()),
            id: Some("integral-name-memory".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            integral_name_memory: Some(IntegralNameMemoryConfig {
                scope: Some(IntegralNameScope::Document),
                contexts: Some(IntegralNameContexts::BodyAndNotes),
                subsequent_form: Some(SubsequentNameForm::Short),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            integral: Some(Box::new(CitationSpec {
                template: Some(
                    vec![TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author.into(),
                        form: ContributorForm::Long,
                        rendering: Rendering::default(),
                        ..Default::default()
                    })]
                    .into(),
                ),
                ..Default::default()
            })),
            template: Some(
                vec![
                    TemplateComponent::Contributor(TemplateContributor {
                        contributor: ContributorRole::Author.into(),
                        form: ContributorForm::Short,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: citum_schema::template::DateVariable::Issued,
                        form: DateForm::Year,
                        rendering: Rendering {
                            wrap: Some(WrapPunctuation::Parentheses.into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn legal_case_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Legal Case Grouping".to_string()),
            id: Some("legal-case-grouping".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        form: None,
                        rendering: Rendering::default(),
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: citum_schema::template::DateVariable::Issued,
                        form: DateForm::Year,
                        rendering: Rendering {
                            prefix: Some(", ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            wrap: Some(WrapPunctuation::Parentheses.into()),
            multi_cite_delimiter: Some("; ".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Dates are exempt from `TemplateComponentTracker` dedup entirely — CSL
/// restricts variable-consumption tracking to cs:substitute (names only), so
/// a style writing `date: issued` twice (a short citation year up front, a
/// full precise date later) must have both render, regardless of form or
/// rendering context. Covers the case that motivated the fix (same form,
/// same context — the strongest case for "these might be an accidental
/// duplicate"), which still must not be suppressed.
#[test]
fn test_date_key_is_always_none() {
    let date1 = TemplateComponent::Date(TemplateDate {
        date: citum_schema::template::DateVariable::Issued,
        form: DateForm::Year,
        rendering: Rendering::default(),
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        links: None,
        custom: None,
    });

    // Same variable, same form, same (empty) context — the case most likely
    // to look like an accidental duplicate, yet still must not be treated
    // as one.
    let date2 = TemplateComponent::Date(TemplateDate {
        date: citum_schema::template::DateVariable::Issued,
        form: DateForm::Year,
        rendering: Rendering::default(),
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        links: None,
        custom: None,
    });

    // Same variable, different form (the GB/T author-date case: bare year up
    // front, full precise date later).
    let date3 = TemplateComponent::Date(TemplateDate {
        date: citum_schema::template::DateVariable::Issued,
        form: DateForm::YearMonthDay,
        rendering: Rendering::default(),
        fallback: None,
        suppress_note: None,
        suppress_disamb_suffix: None,
        links: None,
        custom: None,
    });

    assert_eq!(get_variable_key(&date1), None);
    assert_eq!(get_variable_key(&date2), None);
    assert_eq!(get_variable_key(&date3), None);
}

#[test]
fn test_substituted_contributor_keys_block_contextual_duplicate_components() {
    let mut tracker = TemplateComponentTracker::default();
    let translated_component = TemplateComponent::Contributor(TemplateContributor {
        contributor: ContributorRole::Translator.into(),
        form: ContributorForm::Long,
        rendering: Rendering {
            suffix: Some(", translator".into()),
            ..Default::default()
        },
        ..Default::default()
    });
    let translator_key =
        get_variable_key(&translated_component).expect("translator component should have a key");

    tracker.mark_rendered(None, Some("contributor:Translator"));

    assert!(tracker.should_skip(Some(&translator_key)));
}

#[test]
fn test_substituted_title_keys_preserve_primary_title_base() {
    let mut tracker = TemplateComponentTracker::default();
    let short_title = TemplateComponent::Title(TemplateTitle {
        title: TitleType::Primary,
        form: Some(TitleForm::Short),
        ..Default::default()
    });
    let long_title = TemplateComponent::Title(TemplateTitle {
        title: TitleType::Primary,
        form: Some(TitleForm::Long),
        ..Default::default()
    });

    let short_key = get_variable_key(&short_title).expect("short title should have a key");
    let long_key = get_variable_key(&long_title).expect("long title should have a key");

    assert_eq!(short_key, "title:Primary:Short");
    assert_eq!(long_key, "title:Primary:Long");

    tracker.mark_rendered(None, Some("title:Primary"));

    assert!(tracker.should_skip(Some(&short_key)));
    assert!(tracker.should_skip(Some(&long_key)));
}

#[test]
fn test_strip_author_component_nested_list() {
    let nested = TemplateComponent::Group(TemplateGroup {
        group: vec![
            TemplateComponent::Contributor(TemplateContributor {
                fallback: None,
                contributor: ContributorRole::Author.into(),
                form: ContributorForm::Short,
                and: None,
                shorten: None,
                label: None,
                merge: None,
                name_order: None,
                name_form: None,
                delimiter: None,
                sort_separator: None,
                links: None,
                gender: None,
                rendering: Rendering::default(),
                custom: None,
            }),
            TemplateComponent::Date(TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                form: DateForm::Year,
                rendering: Rendering::default(),
                fallback: None,
                suppress_note: None,
                suppress_disamb_suffix: None,
                links: None,
                custom: None,
            }),
        ],
        delimiter: Some(DelimiterPunctuation::Space),
        rendering: Rendering::default(),
        render_when: None,
        custom: None,
    });

    let filtered = strip_author_component(&nested).expect("list should remain");
    let TemplateComponent::Group(filtered_list) = filtered else {
        panic!("expected list");
    };

    assert_eq!(filtered_list.group.len(), 1);
    assert!(matches!(filtered_list.group[0], TemplateComponent::Date(_)));
}

#[test]
fn affix_content_normalizes_prefix_and_suffix_spacing() {
    let style = Style::default();
    let bibliography = Bibliography::new();
    let locale = Locale::default();
    let config = Config::default();
    let hints = HashMap::new();
    let citation_numbers = RwLock::new(HashMap::new());
    let compound_set_by_ref = HashMap::new();
    let compound_member_index = HashMap::new();
    let compound_sets = IndexMap::new();
    let renderer = Renderer::new(
        RendererResources {
            style: &style,
            bibliography: &bibliography,
            locale: &locale,
            config: Arc::new(config.clone()),
            bibliography_config: None,
            first_note_by_id: None,
        },
        &hints,
        &citation_numbers,
        CompoundRenderData {
            set_by_ref: &compound_set_by_ref,
            member_index: &compound_member_index,
            sets: &compound_sets,
        },
        true,
        false,
        None,
    );
    let fmt = crate::render::plain::PlainText;

    assert_eq!(
        renderer.affix_content(&fmt, "body".to_string(), Some("see"), Some("n. 2"), None),
        "see body n. 2"
    );
    assert_eq!(
        renderer.affix_content(&fmt, "body".to_string(), Some("see "), Some(", n. 2"), None),
        "see body, n. 2"
    );
}

#[test]
fn grouped_author_date_strips_leading_affix_from_tail_components() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "item1".to_string(),
        make_reference("item1", "book", Some(("Kuhn", "Thomas")), 1962, "Book A"),
    );
    bibliography.insert(
        "item2".to_string(),
        make_reference("item2", "book", Some(("Kuhn", "Thomas")), 1963, "Book B"),
    );
    let processor = Processor::new(grouped_author_date_style(), bibliography);

    let citation = Citation {
        items: vec![
            CitationItem {
                id: "item1".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "item2".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("grouped citation should render"),
        "(Kuhn, 1962, 1963)"
    );
}

#[test]
fn grouped_author_date_preserves_later_item_prefixes() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "item1".to_string(),
        make_reference("item1", "book", Some(("Kuhn", "Thomas")), 1962, "Book A"),
    );
    bibliography.insert(
        "item2".to_string(),
        make_reference("item2", "book", Some(("Kuhn", "Thomas")), 1963, "Book B"),
    );
    let processor = Processor::new(grouped_author_date_style(), bibliography);

    let citation = Citation {
        items: vec![
            CitationItem {
                id: "item1".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "item2".to_string(),
                prefix: Some("see".into()),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("grouped citation should preserve later item prefixes"),
        "(Kuhn, 1962, see 1963)"
    );
}

#[test]
fn explicit_author_year_group_uses_group_delimiter_after_author_strip() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "item1".to_string(),
        make_reference("item1", "book", Some(("Kuhn", "Thomas")), 1962, "Book A"),
    );
    let processor = Processor::new(explicit_author_year_group_style(), bibliography);

    let citation = Citation {
        items: vec![CitationItem {
            id: "item1".to_string(),
            locator: Some(citum_schema::citation::CitationLocator::single(
                citum_schema::citation::LocatorType::Page,
                "5",
            )),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("explicit author-year group should render"),
        "(Kuhn 1962, 5)"
    );
}

#[test]
fn explicit_author_year_group_keeps_tail_delimiter_for_locator() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "item1".to_string(),
        make_reference("item1", "book", Some(("Kuhn", "Thomas")), 1962, "Book A"),
    );
    let processor = Processor::new(
        explicit_author_year_group_with_locator_delimiter_style(),
        bibliography,
    );

    let citation = Citation {
        items: vec![CitationItem {
            id: "item1".to_string(),
            locator: Some(citum_schema::citation::CitationLocator::single(
                citum_schema::citation::LocatorType::Page,
                "5",
            )),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("explicit author-year group should render locator"),
        "(Kuhn 1962, p. 5)"
    );
}

#[test]
fn grouping_helper_matches_citation_wide_preserve_behavior() {
    let style = grouped_author_date_style();
    let config = style.options.clone().unwrap_or_default();
    let locale = Locale::default();
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "item1".to_string(),
        make_reference("item1", "book", Some(("Kuhn", "Thomas")), 1962, "Book A"),
    );
    bibliography.insert(
        "item2".to_string(),
        make_reference("item2", "book", Some(("Kuhn", "Thomas")), 1963, "Book B"),
    );
    bibliography.insert(
        "item3".to_string(),
        make_reference("item3", "book", Some(("Smith", "John")), 2020, "Book C"),
    );

    let mut hints = HashMap::new();
    hints.insert(
        "item3".to_string(),
        ProcHints {
            min_names_to_show: Some(2),
            ..Default::default()
        },
    );
    let citation_numbers = RwLock::new(HashMap::new());
    let compound_set_by_ref = HashMap::new();
    let compound_member_index = HashMap::new();
    let compound_sets = IndexMap::new();
    let renderer = Renderer::new(
        RendererResources {
            style: &style,
            bibliography: &bibliography,
            locale: &locale,
            config: Arc::new(config.clone()),
            bibliography_config: None,
            first_note_by_id: None,
        },
        &hints,
        &citation_numbers,
        CompoundRenderData {
            set_by_ref: &compound_set_by_ref,
            member_index: &compound_member_index,
            sets: &compound_sets,
        },
        true,
        false,
        None,
    );
    let items = vec![
        CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        },
        CitationItem {
            id: "item2".to_string(),
            ..Default::default()
        },
        CitationItem {
            id: "item3".to_string(),
            ..Default::default()
        },
    ];

    let groups = group_citation_items_by_author(&renderer, &items);

    assert_eq!(groups.len(), 3);
    assert_eq!(
        groups[0]
            .1
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["item1"]
    );
    assert_eq!(
        groups[1]
            .1
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["item2"]
    );
    assert_eq!(
        groups[2]
            .1
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["item3"]
    );
}

#[test]
fn explicit_integral_template_honors_integral_name_state() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "item1".to_string(),
        make_reference("item1", "book", Some(("Smith", "John")), 2020, "Book A"),
    );
    let processor = Processor::new(integral_name_style(), bibliography);

    let first = Citation {
        mode: CitationMode::Integral,
        items: vec![CitationItem {
            id: "item1".to_string(),
            integral_name_state: Some(IntegralNameState::First),
            ..Default::default()
        }],
        ..Default::default()
    };
    let subsequent = Citation {
        mode: CitationMode::Integral,
        items: vec![CitationItem {
            id: "item1".to_string(),
            integral_name_state: Some(IntegralNameState::Subsequent),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&first)
            .expect("first integral citation should render"),
        "John Smith"
    );
    assert_eq!(
        processor
            .process_citation(&subsequent)
            .expect("subsequent integral citation should render"),
        "Smith"
    );
}

#[test]
fn legal_cases_render_per_item_instead_of_grouped_year_compression() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "case1".to_string(),
        make_reference("case1", "legal-case", None, 1954, "Brown v. Board"),
    );
    bibliography.insert(
        "case2".to_string(),
        make_reference("case2", "legal-case", None, 1955, "Brown v. Board"),
    );
    let processor = Processor::new(legal_case_style(), bibliography);

    let citation = Citation {
        items: vec![
            CitationItem {
                id: "case1".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "case2".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let rendered = processor
        .process_citation(&citation)
        .expect("legal-case citation should render");
    assert!(
        rendered.contains("Brown v. Board"),
        "full legal-case title should be preserved in each item"
    );
    assert!(
        rendered.contains(';'),
        "legal-case items should remain separate within the citation"
    );
    assert!(
        !rendered.contains("1954, 1955"),
        "legal-case items should not collapse into grouped year compression"
    );
}

use std::str::FromStr;

#[allow(clippy::too_many_lines, reason = "integration test fixture setup")]
#[test]
fn test_type_specific_rendering() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "article1".to_string(),
        make_reference(
            "article1",
            "article-journal",
            Some(("Smith", "John")),
            2020,
            "Title A",
        ),
    );
    bibliography.insert(
        "book1".to_string(),
        make_reference("book1", "book", Some(("Doe", "Jane")), 2021, "Title B"),
    );

    let mut type_variants = IndexMap::new();
    // Article variant: Author (Short), Year
    type_variants.insert(
        TypeSelector::from_str("article-journal").unwrap(),
        vec![
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Author.into(),
                form: ContributorForm::Short,
                ..Default::default()
            }),
            TemplateComponent::Date(TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                form: DateForm::Year,
                rendering: Rendering {
                    prefix: Some(", ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ]
        .into(),
    );
    // Book variant: Author (Short), Title (Primary), Year
    type_variants.insert(
        TypeSelector::from_str("book").unwrap(),
        vec![
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Author.into(),
                form: ContributorForm::Short,
                ..Default::default()
            }),
            TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                rendering: Rendering {
                    emph: Some(true),
                    prefix: Some(", ".into()),
                    ..Default::default()
                },
                links: None,
                custom: None,
                ..Default::default()
            }),
            TemplateComponent::Date(TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                form: DateForm::Year,
                rendering: Rendering {
                    prefix: Some(", ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ]
        .into(),
    );

    let style = Style {
        info: StyleInfo {
            title: Some("Type Specific".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            type_variants: Some(type_variants),
            template: Some(
                vec![TemplateComponent::Variable(TemplateVariable {
                    variable: SimpleVariable::Locator,
                    rendering: Rendering::default(),
                    links: None,
                    custom: None,
                })]
                .into(),
            ),
            wrap: Some(WrapPunctuation::Parentheses.into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let processor = Processor::new(style, bibliography);

    let cite_article = Citation {
        items: vec![CitationItem {
            id: "article1".to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };
    let cite_book = Citation {
        items: vec![CitationItem {
            id: "book1".to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    assert_eq!(
        processor.process_citation(&cite_article).unwrap(),
        "(Smith, 2020)"
    );
    assert_eq!(
        processor.process_citation(&cite_book).unwrap(),
        "(Doe, _Title B_, 2021)"
    );
}

#[test]
fn ungrouped_chapter_citation_uses_entry_dictionary_variant_alias() {
    let mut bibliography = Bibliography::new();
    bibliography.insert(
        "chapter1".to_string(),
        make_reference(
            "chapter1",
            "chapter",
            Some(("Diderot", "Denis")),
            1751,
            "Encyclopedia Entry",
        ),
    );

    let mut type_variants = IndexMap::new();
    type_variants.insert(
        TypeSelector::from_str("entry-dictionary").unwrap(),
        vec![TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            ..Default::default()
        })]
        .into(),
    );

    let style = Style {
        info: StyleInfo {
            title: Some("Ungrouped Chapter Alias".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            type_variants: Some(type_variants),
            options: Some(citum_schema::CitationOptions {
                label_mode: Some(citum_schema::options::CitationLabelMode::Numeric),
                ..Default::default()
            }),
            wrap: Some(WrapPunctuation::Parentheses.into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let processor = Processor::new(style, bibliography);

    let citation = Citation {
        items: vec![CitationItem {
            id: "chapter1".to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("ungrouped chapter citation should render"),
        "(1, Encyclopedia Entry)"
    );
}

#[allow(clippy::too_many_lines, reason = "integration test fixture setup")]
#[test]
fn test_bibliography_type_specific_rendering() {
    use crate::processor::rendering::RendererResources;
    use citum_schema::BibliographySpec;

    let mut bibliography = Bibliography::new();
    let interview_ref = Reference::from(LegacyReference {
        id: "ref1".to_string(),
        ref_type: "interview".to_string(),
        author: Some(vec![Name::new("Arendt", "Hannah")]),
        title: Some("Thinking in Public".to_string()),
        issued: Some(LegacyDateVariable::year(1975)),
        interviewer: Some(vec![Name::new("Young-Bruehl", "Elisabeth")]),
        publisher: Some("Schocken Books".to_string()),
        ..Default::default()
    });
    bibliography.insert("ref1".to_string(), interview_ref);

    let mut type_variants = IndexMap::new();
    type_variants.insert(
        TypeSelector::from_str("interview").unwrap(),
        vec![
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Author.into(),
                form: ContributorForm::Long,
                name_order: Some(NameOrder::FamilyFirst),
                ..Default::default()
            }),
            TemplateComponent::Date(TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                form: DateForm::Year,
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses.into()),
                    prefix: Some(" ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                rendering: Rendering {
                    emph: Some(true),
                    prefix: Some(" ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Interviewer.into(),
                form: ContributorForm::Long,
                name_order: Some(NameOrder::FamilyFirst),
                rendering: Rendering {
                    wrap: Some(WrapPunctuation::Parentheses.into()),
                    prefix: Some(" ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
            TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Publisher,
                rendering: Rendering {
                    prefix: Some(". ".into()),
                    suffix: Some(".".into()),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ]
        .into(),
    );

    let style = Style {
        info: StyleInfo {
            title: Some("Bib Type Specific".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            type_variants: Some(type_variants),
            template: Some(vec![TemplateComponent::Title(TemplateTitle::default())].into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let locale = Locale::default(); // Note: default locale might not have "Interviewer" term set up in a way that auto-labels work if we don't specify them.
    // But we are specifying the interviewer component explicitly.
    let config = Config::default();
    let disambig = HashMap::new();
    let series_map = RwLock::new(HashMap::new());
    let set_by_ref = HashMap::new();
    let member_index = HashMap::new();
    let sets = IndexMap::new();

    let renderer = crate::processor::rendering::Renderer::new(
        RendererResources {
            style: &style,
            bibliography: &bibliography,
            locale: &locale,
            config: Arc::new(config.clone()),
            bibliography_config: None,
            first_note_by_id: None,
        },
        &disambig,
        &series_map,
        crate::processor::rendering::CompoundRenderData {
            set_by_ref: &set_by_ref,
            member_index: &member_index,
            sets: &sets,
        },
        false,
        false,
        None,
    );

    let reference = bibliography.get("ref1").unwrap();
    let proc_template = renderer
        .process_bibliography_entry_with_format::<crate::render::plain::PlainText>(reference, 1)
        .unwrap();
    let result = crate::render::bibliography::render_entry_body_with_format::<
        crate::render::plain::PlainText,
    >(&crate::render::component::ProcEntry {
        id: "ref1".to_string(),
        marker: None,
        template: proc_template,
        metadata: crate::render::format::ProcEntryMetadata::default(),
    });

    assert_eq!(
        result,
        "Arendt, Hannah (1975) _Thinking in Public_ (Young-Bruehl, Elisabeth). Schocken Books."
    );
}

fn render_single_bibliography_entry(style: Style, reference: Reference) -> String {
    let id = reference
        .id()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "ref1".to_string());
    let mut bibliography = Bibliography::new();
    bibliography.insert(id, reference);

    Processor::new(style, bibliography)
        .render_bibliography_with_format_standalone::<crate::render::plain::PlainText>()
}

fn bibliography_style_with_template(template: Vec<TemplateComponent>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Sentence Initial Bibliography".to_string()),
            ..Default::default()
        },
        bibliography: Some(citum_schema::BibliographySpec {
            template: Some(template.into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn sentence_initial_date_group_preserves_no_date_term_case() {
    let reference = Reference::from(LegacyReference {
        id: "no-date".to_string(),
        ref_type: "book".to_string(),
        author: Some(vec![Name::new("Forthcoming", "A.")]),
        title: Some("Foundations of Declarative Bibliography".to_string()),
        publisher: Some("University Press".to_string()),
        ..Default::default()
    });
    let style = bibliography_style_with_template(vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author.into(),
            form: ContributorForm::Long,
            name_order: Some(NameOrder::FamilyFirst),
            ..Default::default()
        }),
        TemplateComponent::Group(TemplateGroup {
            group: vec![TemplateComponent::Date(TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                form: DateForm::Year,
                ..Default::default()
            })],
            ..Default::default()
        }),
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            ..Default::default()
        }),
        TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Publisher,
            ..Default::default()
        }),
    ]);

    let result = render_single_bibliography_entry(style, reference);

    assert_eq!(
        result,
        "Forthcoming, A. n.d. Foundations of Declarative Bibliography. University Press"
    );
}

#[test]
fn sentence_initial_term_group_preserves_locale_term_case() {
    let reference = Reference::from(LegacyReference {
        id: "term-group".to_string(),
        ref_type: "book".to_string(),
        title: Some("Untitled".to_string()),
        ..Default::default()
    });
    let no_date_term = TemplateComponent::Term(TemplateTerm {
        term: citum_schema::locale::GeneralTerm::NoDate,
        form: Some(citum_schema::locale::TermForm::Short),
        ..Default::default()
    });

    for component in [
        no_date_term,
        TemplateComponent::Message(TemplateMessage {
            message: "term.no-date".to_string(),
            form: Some(citum_schema::locale::TermForm::Short),
            ..Default::default()
        }),
    ] {
        let style =
            bibliography_style_with_template(vec![TemplateComponent::Group(TemplateGroup {
                group: vec![
                    component,
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        ..Default::default()
                    }),
                ],
                delimiter: Some(DelimiterPunctuation::Space),
                ..Default::default()
            })]);

        let result = render_single_bibliography_entry(style, reference.clone());
        assert_eq!(result, "n.d. Untitled");
    }
}

#[test]
fn sentence_initial_group_still_capitalizes_leading_contributor_role_prose() {
    let reference = Reference::from(LegacyReference {
        id: "edited".to_string(),
        ref_type: "book".to_string(),
        editor: Some(vec![Name::new("Smith", "Ada")]),
        title: Some("Collected Sources".to_string()),
        ..Default::default()
    });
    let style = bibliography_style_with_template(vec![TemplateComponent::Group(TemplateGroup {
        group: vec![TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Editor.into(),
            form: ContributorForm::Verb,
            name_order: Some(NameOrder::GivenFirst),
            ..Default::default()
        })],
        ..Default::default()
    })]);

    let result = render_single_bibliography_entry(style, reference);

    assert_eq!(result, "Edited by Ada Smith");
}

// csl26-huuz (piece 3): a year-suffix letter attached to a fallback date
// candidate must land inside that candidate's own wrap, and a fallback
// chain that renders no text at all must still carry the letter standalone.
// `render_date_fallback_chain` (values/date.rs) is exercised only through
// the GB/T oracle otherwise, which is diagnostic (`count_toward_fidelity:
// false`) — these tests keep the two render paths covered in-repo.
mod fallback_chain_disamb_suffix {
    use super::*;
    use citum_schema::reference::{
        Contributor, DateValue, Monograph, MonographType, MultilingualString, StructuredName, Title,
    };

    fn make_monograph(id: &str, family: &str, issued: &str, accessed: Option<&str>) -> Reference {
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(format!("Title {id}"))),
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(String::new()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new(issued),
            accessed: accessed.map(DateValue::new),
            ..Default::default()
        }))
    }

    fn render_bibliography_entries(style: Style, references: Vec<Reference>) -> Vec<String> {
        let mut bibliography = Bibliography::new();
        for reference in references {
            let id = reference
                .id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| format!("ref{}", bibliography.len() + 1));
            bibliography.insert(id, reference);
        }
        Processor::new(style, bibliography)
            .render_bibliography_with_format_standalone::<crate::render::plain::PlainText>()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn author_then_date_style(date_component: TemplateDate) -> Style {
        bibliography_style_with_template(vec![
            TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Author.into(),
                form: ContributorForm::Long,
                name_order: Some(NameOrder::FamilyFirst),
                ..Default::default()
            }),
            TemplateComponent::Group(TemplateGroup {
                group: vec![TemplateComponent::Date(date_component)],
                ..Default::default()
            }),
            TemplateComponent::Title(TemplateTitle {
                title: TitleType::Primary,
                ..Default::default()
            }),
        ])
    }

    #[test]
    fn accessed_fallback_letter_lands_inside_bracket_wrap() {
        let style = author_then_date_style(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            fallback: Some(vec![TemplateComponent::Date(TemplateDate {
                date: DateVariable::Accessed,
                form: DateForm::Year,
                rendering: Rendering {
                    wrap: Some(WrapConfig {
                        punctuation: WrapPunctuation::Brackets,
                        inner_prefix: None,
                        inner_suffix: None,
                    }),
                    ..Default::default()
                },
                ..Default::default()
            })]),
            ..Default::default()
        });
        let references = vec![
            make_monograph("first", "Anon", "", Some("2020")),
            make_monograph("second", "Anon", "", Some("2019")),
        ];

        let lines = render_bibliography_entries(style, references);

        assert_eq!(
            lines,
            vec![
                "Anon. [2020a]. Title first".to_string(),
                String::new(),
                "Anon. [2019b]. Title second".to_string(),
            ],
            "the disambiguating letter must sit inside the accessed date's own \
             bracket wrap, not appended after it"
        );
    }

    #[test]
    fn empty_fallback_chain_still_renders_letter_standalone() {
        let style = author_then_date_style(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            fallback: Some(vec![]),
            ..Default::default()
        });
        let references = vec![
            make_monograph("first", "Anon", "", None),
            make_monograph("second", "Anon", "", None),
        ];

        let lines = render_bibliography_entries(style, references);

        assert_eq!(
            lines,
            vec![
                "Anon. a. Title first".to_string(),
                String::new(),
                "Anon. b. Title second".to_string(),
            ],
            "an explicit empty fallback list renders no date text, but the \
             collision group's letter must still render standalone rather \
             than being silently dropped"
        );
    }

    /// Mirrors GB/T 7714's real `article-journal,article-magazine`
    /// type-variant shape (`fallback: []`): an undated entry with no
    /// collision partner renders no date text whatsoever — not even the
    /// locale's no-date term — and an undated entry that *does* need
    /// disambiguation still gets its letter, rendered standalone in the
    /// same blank position. Answers the PR review question about this
    /// YAML's behavioral implication.
    #[test]
    fn empty_fallback_list_leaves_the_date_position_blank_with_or_without_disambiguation() {
        let style = author_then_date_style(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            fallback: Some(vec![]),
            ..Default::default()
        });
        let references = vec![
            make_monograph("solo", "Solo", "", None),
            make_monograph("first", "Anon", "", None),
            make_monograph("second", "Anon", "", None),
        ];

        let lines = render_bibliography_entries(style, references);

        assert_eq!(
            lines,
            vec![
                "Anon. a. Title first".to_string(),
                String::new(),
                "Anon. b. Title second".to_string(),
                String::new(),
                "Solo. Title solo".to_string(),
            ],
            "an undated entry with no collision partner renders no date \
             position at all; an undated entry needing disambiguation still \
             gets its letter, standalone, in that same blank position"
        );
    }
}
