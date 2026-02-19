use super::*;
use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference};
use csln_core::options::{
    AndOptions, ContributorConfig, DisplayAsSort, Processing, ShortenListOptions,
};
use csln_core::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, Rendering,
    TemplateComponent, TemplateContributor, TemplateDate, TemplateTitle, TitleType,
    WrapPunctuation,
};
use csln_core::{BibliographySpec, CitationSpec, StyleInfo};

fn make_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("APA".to_string()),
            id: Some("apa".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            substitute: Some(csln_core::options::SubstituteConfig::default()),
            contributors: Some(ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 3,
                    use_first: 1,
                    ..Default::default()
                }),
                and: Some(AndOptions::Symbol),
                display_as_sort: Some(DisplayAsSort::First),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            options: None,
            template: Some(vec![
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
            ]),
            wrap: Some(WrapPunctuation::Parentheses),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: None,
            template: Some(vec![
                TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author,
                    form: ContributorForm::Long,
                    name_order: None,
                    delimiter: None,
                    and: None,
                    rendering: Default::default(),
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
                        prefix: Some(". ".to_string()),
                        emph: Some(true),
                        ..Default::default()
                    },
                    overrides: None,
                    ..Default::default()
                }),
            ]),
            ..Default::default()
        }),
        templates: None,
        ..Default::default()
    }
}

fn make_note_style() -> Style {
    let mut style = make_style();
    style.options = Some(Config {
        processing: Some(Processing::Note),
        ..Default::default()
    });
    style
}

fn make_bibliography() -> Bibliography {
    let mut bib = Bibliography::new();
    bib.insert(
        "kuhn1962".to_string(),
        Reference::from(LegacyReference {
            id: "kuhn1962".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            issued: Some(DateVariable::year(1962)),
            ..Default::default()
        }),
    );

    bib
}

#[test]
fn test_process_citation() {
    let style = make_style();
    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    let citation = Citation {
        id: Some("c1".to_string()),
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = processor.process_citation(&citation).unwrap();
    assert_eq!(result, "(Kuhn, 1962)");
}

#[test]
fn test_normalize_note_context_assigns_missing_numbers() {
    let style = make_note_style();
    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    let citations = vec![
        Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
        Citation {
            id: Some("c2".to_string()),
            note_number: Some(7),
            items: vec![crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
        Citation {
            id: Some("c3".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
    ];

    let normalized = processor.normalize_note_context(&citations);
    assert_eq!(normalized[0].note_number, Some(1));
    assert_eq!(normalized[1].note_number, Some(7));
    assert_eq!(normalized[2].note_number, Some(8));
}

#[test]
fn test_process_citations_batch_api() {
    let style = make_style();
    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    let citations = vec![
        Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
        Citation {
            id: Some("c2".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
    ];

    let rendered = processor.process_citations(&citations).unwrap();
    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0], "(Kuhn, 1962)");
    assert_eq!(rendered[1], "(Kuhn, 1962)");
}

#[test]
fn test_citation_locator_label_renders_term() {
    let mut style = make_style();
    style.citation = Some(csln_core::CitationSpec {
        template: Some(vec![
            csln_core::TemplateComponent::Contributor(csln_core::template::TemplateContributor {
                contributor: ContributorRole::Author,
                form: ContributorForm::Short,
                ..Default::default()
            }),
            csln_core::TemplateComponent::Date(csln_core::template::TemplateDate {
                date: TDateVar::Issued,
                form: DateForm::Year,
                ..Default::default()
            }),
            csln_core::TemplateComponent::Variable(csln_core::template::TemplateVariable {
                variable: csln_core::template::SimpleVariable::Locator,
                ..Default::default()
            }),
        ]),
        wrap: Some(WrapPunctuation::Parentheses),
        delimiter: Some(", ".to_string()),
        ..Default::default()
    });

    let bib = make_bibliography();
    let processor = Processor::new(style, bib);
    let citation = Citation {
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            label: Some(csln_core::citation::LocatorType::Page),
            locator: Some("23".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let rendered = processor.process_citation(&citation).unwrap();
    assert_eq!(rendered, "(Kuhn, 1962, p. 23)");
}

#[test]
fn test_citation_locator_label_renders_term_with_loaded_locale() {
    use std::path::Path;

    let mut style = make_style();
    style.citation = Some(csln_core::CitationSpec {
        template: Some(vec![
            csln_core::TemplateComponent::Contributor(csln_core::template::TemplateContributor {
                contributor: ContributorRole::Author,
                form: ContributorForm::Short,
                ..Default::default()
            }),
            csln_core::TemplateComponent::Date(csln_core::template::TemplateDate {
                date: TDateVar::Issued,
                form: DateForm::Year,
                ..Default::default()
            }),
            csln_core::TemplateComponent::Variable(csln_core::template::TemplateVariable {
                variable: csln_core::template::SimpleVariable::Locator,
                ..Default::default()
            }),
        ]),
        wrap: Some(WrapPunctuation::Parentheses),
        delimiter: Some(", ".to_string()),
        ..Default::default()
    });

    let bib = make_bibliography();
    let locale = csln_core::locale::Locale::load("en-US", Path::new("locales"));
    let processor = Processor::with_locale(style, bib, locale);
    let citation = Citation {
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            label: Some(csln_core::citation::LocatorType::Page),
            locator: Some("23".to_string()),
            ..Default::default()
        }],
        ..Default::default()
    };

    let rendered = processor.process_citation(&citation).unwrap();
    assert_eq!(rendered, "(Kuhn, 1962, p. 23)");
}

#[test]
fn test_render_bibliography() {
    let style = make_style();
    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    let result = processor.render_bibliography();

    // Check it contains the key parts
    assert!(result.contains("Kuhn"));
    assert!(result.contains("(1962)"));
    assert!(result.contains("_The Structure of Scientific Revolutions_"));
}

#[test]
fn test_disambiguation_hints() {
    let style = make_style();
    let mut bib = make_bibliography();

    // Add another Kuhn 1962 reference to trigger disambiguation
    bib.insert(
        "kuhn1962b".to_string(),
        Reference::from(LegacyReference {
            id: "kuhn1962b".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Function of Measurement in Modern Physical Science".to_string()),
            issued: Some(DateVariable::year(1962)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);
    let hints = &processor.hints;

    // Both should have disambiguation condition true
    assert!(hints.get("kuhn1962").unwrap().disamb_condition);
    assert!(hints.get("kuhn1962b").unwrap().disamb_condition);
}

#[test]
fn test_disambiguation_givenname() {
    use csln_core::options::{
        Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
    };

    // Style with add-givenname enabled
    let mut style = make_style();
    style.options = Some(Config {
        processing: Some(Processing::Custom(ProcessingCustom {
            sort: Some(Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![
                    SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Year,
                        ascending: true,
                    },
                ],
            }),
            group: Some(Group {
                template: vec![SortKey::Author, SortKey::Year],
            }),
            disambiguate: Some(Disambiguation {
                names: true,
                add_givenname: true,
                year_suffix: true,
            }),
        })),
        contributors: Some(ContributorConfig {
            initialize_with: Some(". ".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    });

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "smith2020a".to_string(),
        Reference::from(LegacyReference {
            id: "smith2020a".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "John")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );
    bib.insert(
        "smith2020b".to_string(),
        Reference::from(LegacyReference {
            id: "smith2020b".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "Alice")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    let hints = &processor.hints;

    // Verify hints
    assert!(hints.get("smith2020a").unwrap().expand_given_names);
    assert!(hints.get("smith2020b").unwrap().expand_given_names);
    assert!(!hints.get("smith2020a").unwrap().disamb_condition); // No year suffix

    // Verify output
    let cit_a = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "smith2020a".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    let cit_b = processor
        .process_citation(&Citation {
            id: Some("c2".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "smith2020b".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    // Should expand to "J. Smith" and "A. Smith" (because initialized)
    assert!(cit_a.contains("J. Smith"));
    assert!(cit_b.contains("A. Smith"));
}

#[test]
fn test_disambiguation_add_names() {
    use csln_core::options::{
        Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
    };

    let mut style = make_style();
    style.options = Some(Config {
        processing: Some(Processing::Custom(ProcessingCustom {
            sort: Some(Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![
                    SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Year,
                        ascending: true,
                    },
                ],
            }),
            group: Some(Group {
                template: vec![SortKey::Author, SortKey::Year],
            }),
            disambiguate: Some(Disambiguation {
                names: true, // disambiguate-add-names
                add_givenname: false,
                year_suffix: true,
            }),
        })),
        contributors: Some(ContributorConfig {
            shorten: Some(ShortenListOptions {
                min: 2,
                use_first: 1,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    });

    let mut bib = indexmap::IndexMap::new();
    // Two works by Smith & Jones and Smith & Brown
    // Both would be "Smith et al. (2020)"
    bib.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![
                Name::new("Smith", "John"),
                Name::new("Jones", "Peter"),
            ]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );
    bib.insert(
        "ref2".to_string(),
        Reference::from(LegacyReference {
            id: "ref2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![
                Name::new("Smith", "John"),
                Name::new("Brown", "Alice"),
            ]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    // Verify hints
    assert_eq!(
        processor.hints.get("ref1").unwrap().min_names_to_show,
        Some(2)
    );
    assert_eq!(
        processor.hints.get("ref2").unwrap().min_names_to_show,
        Some(2)
    );

    // Verify output
    let cit_1 = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    let cit_2 = processor
        .process_citation(&Citation {
            id: Some("c2".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref2".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    // Should expand to "Smith, Jones" and "Smith, Brown" (no et al. because only 2 names)
    assert!(cit_1.contains("Smith") && cit_1.contains("Jones"));
    assert!(cit_2.contains("Smith") && cit_2.contains("Brown"));
}

#[test]
fn test_disambiguation_combined_expansion() {
    use csln_core::options::{
        Disambiguation, Group, Processing, ProcessingCustom, Sort, SortKey, SortSpec,
    };

    // This test simulates the "Sam Smith & Julie Smith" scenario but with
    // two items that remain ambiguous after name expansion alone.
    // Item 1: [Sam Smith, Julie Smith] 2020 -> "Smith & Smith" (base)
    // Item 2: [Sam Smith, Bob Smith] 2020   -> "Smith & Smith" (base)
    // Both would be "Smith et al." if min=3, but here they collide even as "Smith & Smith".
    // They need both expanded names AND expanded given names.

    let mut style = make_style();
    style.options = Some(Config {
        processing: Some(Processing::Custom(ProcessingCustom {
            sort: Some(Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![
                    SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Year,
                        ascending: true,
                    },
                ],
            }),
            group: Some(Group {
                template: vec![SortKey::Author, SortKey::Year],
            }),
            disambiguate: Some(Disambiguation {
                names: true,
                add_givenname: true,
                year_suffix: true,
            }),
        })),
        contributors: Some(ContributorConfig {
            shorten: Some(ShortenListOptions {
                min: 2,
                use_first: 1,
                ..Default::default()
            }),
            initialize_with: Some(". ".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    });

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "Sam"), Name::new("Smith", "Julie")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );
    bib.insert(
        "ref2".to_string(),
        Reference::from(LegacyReference {
            id: "ref2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "Sam"), Name::new("Smith", "Bob")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    // Verify output
    let cit_1 = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    let cit_2 = processor
        .process_citation(&Citation {
            id: Some("c2".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref2".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    // Should expand to "S. Smith & J. Smith" and "S. Smith & B. Smith"
    assert!(
        cit_1.contains("S. Smith") && cit_1.contains("J. Smith"),
        "Output was: {}",
        cit_1
    );
    assert!(
        cit_2.contains("S. Smith") && cit_2.contains("B. Smith"),
        "Output was: {}",
        cit_2
    );
}

#[test]
fn test_apa_titles_config() {
    use crate::reference::Reference;
    use csln_core::options::{Config, TitleRendering, TitlesConfig};
    use csln_core::template::{Rendering, TemplateTitle, TitleType};

    let config = Config {
        titles: Some(TitlesConfig {
            periodical: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            monograph: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            container_monograph: Some(TitleRendering {
                emph: Some(true),
                prefix: Some("In ".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let bib_template = vec![
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentMonograph,
            rendering: Rendering::default(),
            ..Default::default()
        }),
    ];

    let style = Style {
        options: Some(config),
        bibliography: Some(csln_core::BibliographySpec {
            template: Some(bib_template),
            ..Default::default()
        }),
        ..Default::default()
    };

    let references = vec![
        Reference::from(LegacyReference {
            id: "art1".to_string(),
            ref_type: "article-journal".to_string(),
            title: Some("A Title".to_string()),
            container_title: Some("Nature".to_string()),
            ..Default::default()
        }),
        Reference::from(LegacyReference {
            id: "ch1".to_string(),
            ref_type: "chapter".to_string(),
            title: Some("A Chapter".to_string()),
            container_title: Some("A Book".to_string()),
            ..Default::default()
        }),
        Reference::from(LegacyReference {
            id: "bk1".to_string(),
            ref_type: "book".to_string(),
            title: Some("A Global Book".to_string()),
            ..Default::default()
        }),
    ];

    let processor = Processor::new(
        style,
        references
            .into_iter()
            .map(|r| (r.id().unwrap().to_string(), r))
            .collect(),
    );

    let res = processor.render_bibliography();

    // Book Case: Primary title -> monograph category -> Italic, No "In "
    assert!(
        res.contains("_A Global Book_"),
        "Book title should be italicized: {}",
        res
    );
    assert!(
        !res.contains("In _A Global Book_"),
        "Book title should NOT have 'In ' prefix: {}",
        res
    );

    // Journal Article Case: ParentSerial -> periodical category -> Italic, No "In "
    assert!(
        res.contains("_Nature_"),
        "Journal title should be italicized: {}",
        res
    );
    assert!(
        !res.contains("In _Nature_"),
        "Journal title should NOT have 'In ' prefix: {}",
        res
    );

    // Chapter Case: ParentMonograph -> container_monograph category -> Italic, WITH "In "
    assert!(
        res.contains("In _A Book_"),
        "Chapter container title should have 'In ' prefix: {}",
        res
    );
}

#[test]
fn test_numeric_citation_numbers_with_repeated_refs() {
    // Citation numbers should remain stable once assigned.
    // Citing ref1, ref2, ref1 again should give numbers 1, 2, 1.
    use csln_core::options::{Config, Processing};
    use csln_core::template::{NumberVariable, TemplateNumber};
    use csln_core::CitationSpec;

    let style = Style {
        citation: Some(CitationSpec {
            wrap: Some(csln_core::template::WrapPunctuation::Brackets),
            template: Some(vec![TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::CitationNumber,
                ..Default::default()
            })]),
            ..Default::default()
        }),
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bib = Bibliography::new();
    bib.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            title: Some("First Book".to_string()),
            ..Default::default()
        }),
    );
    bib.insert(
        "ref2".to_string(),
        Reference::from(LegacyReference {
            id: "ref2".to_string(),
            ref_type: "book".to_string(),
            title: Some("Second Book".to_string()),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    // Cite ref1 first
    let cit1 = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    // Cite ref2 second
    let cit2 = processor
        .process_citation(&Citation {
            id: Some("c2".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref2".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    // Cite ref1 again - should get the SAME number as before
    let cit3 = processor
        .process_citation(&Citation {
            id: Some("c3".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(cit1, "[1]", "First citation of ref1 should be [1]");
    assert_eq!(cit2, "[2]", "First citation of ref2 should be [2]");
    assert_eq!(cit3, "[1]", "Second citation of ref1 should still be [1]");
}

#[test]
fn test_numeric_citation_numbers_follow_registry_order() {
    use csln_core::options::{Config, Processing};
    use csln_core::template::{NumberVariable, TemplateNumber};
    use csln_core::CitationSpec;

    let style = Style {
        citation: Some(CitationSpec {
            wrap: Some(csln_core::template::WrapPunctuation::Brackets),
            template: Some(vec![TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::CitationNumber,
                ..Default::default()
            })]),
            ..Default::default()
        }),
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bib = Bibliography::new();
    bib.insert(
        "ref1".to_string(),
        Reference::from(LegacyReference {
            id: "ref1".to_string(),
            ref_type: "book".to_string(),
            title: Some("First Book".to_string()),
            ..Default::default()
        }),
    );
    bib.insert(
        "ref2".to_string(),
        Reference::from(LegacyReference {
            id: "ref2".to_string(),
            ref_type: "book".to_string(),
            title: Some("Second Book".to_string()),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    let cit = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![crate::reference::CitationItem {
                id: "ref2".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        })
        .unwrap();

    assert_eq!(
        cit, "[2]",
        "Numeric citation number should follow bibliography registry order"
    );
}

#[test]
fn test_citation_grouping_same_author() {
    // Test that adjacent citations by the same author are collapsed:
    // (Kuhn 1962a, 1962b) instead of (Kuhn 1962a; Kuhn 1962b)
    let style = make_style();
    let mut bib = make_bibliography();

    // Add second Kuhn 1962 with different title (triggers year-suffix)
    bib.insert(
        "kuhn1962b".to_string(),
        Reference::from(LegacyReference {
            id: "kuhn1962b".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Function of Measurement in Modern Physical Science".to_string()),
            issued: Some(DateVariable::year(1962)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    // Cite both Kuhn works in one citation - should group
    let result = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![
                crate::reference::CitationItem {
                    id: "kuhn1962b".to_string(), // "Function..." comes first alphabetically -> a
                    ..Default::default()
                },
                crate::reference::CitationItem {
                    id: "kuhn1962".to_string(), // "Structure..." comes second -> b
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();

    // Should be grouped: "Kuhn, 1962a, 1962b" not "Kuhn, 1962a; Kuhn, 1962b"
    // Year suffix assigned by title order: "Function..." < "Structure..."
    assert!(
        result.contains("Kuhn, 1962a, 1962b") || result.contains("Kuhn, 1962b, 1962a"),
        "Same-author citations should be grouped. Got: {}",
        result
    );
    assert!(
        !result.contains("; Kuhn"),
        "Should not have semicolon between same-author citations. Got: {}",
        result
    );
}

#[test]
fn test_citation_grouping_different_authors() {
    // Different authors should NOT be grouped
    let style = make_style();
    let mut bib = make_bibliography();

    bib.insert(
        "smith2020".to_string(),
        Reference::from(LegacyReference {
            id: "smith2020".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "John")]),
            title: Some("Another Book".to_string()),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    let result = processor
        .process_citation(&Citation {
            id: Some("c1".to_string()),
            items: vec![
                crate::reference::CitationItem {
                    id: "kuhn1962".to_string(),
                    ..Default::default()
                },
                crate::reference::CitationItem {
                    id: "smith2020".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        })
        .unwrap();

    // Should have semicolon between different authors
    assert!(
        result.contains("Kuhn") && result.contains("Smith"),
        "Should contain both authors. Got: {}",
        result
    );
    assert!(
        result.contains("; "),
        "Different authors should be separated by semicolon. Got: {}",
        result
    );
}

#[test]
fn test_sort_anonymous_work_by_title() {
    // Anonymous works (no author) should sort by title, with leading articles stripped
    let style = make_style();
    let mut bib = indexmap::IndexMap::new();

    // Add references in wrong alphabetical order to test sorting
    bib.insert(
        "smith".to_string(),
        Reference::from(LegacyReference {
            id: "smith".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "John")]),
            title: Some("A Book".to_string()),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    // Anonymous work - should sort by "Role" (stripping "The")
    bib.insert(
        "anon".to_string(),
        Reference::from(LegacyReference {
            id: "anon".to_string(),
            ref_type: "article-journal".to_string(),
            author: None, // No author!
            title: Some("The Role of Theory".to_string()),
            issued: Some(DateVariable::year(2018)),
            ..Default::default()
        }),
    );

    bib.insert(
        "jones".to_string(),
        Reference::from(LegacyReference {
            id: "jones".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Jones", "Alice")]),
            title: Some("Another Book".to_string()),
            issued: Some(DateVariable::year(2019)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Order should be: Jones (J), anon/Role (R), Smith (S)
    let jones_pos = result.find("Jones").expect("Jones not found");
    let role_pos = result.find("Role of Theory").expect("Role not found");
    let smith_pos = result.find("Smith").expect("Smith not found");

    assert!(
        jones_pos < role_pos,
        "Jones should come before Role. Got:
{}",
        result
    );
    assert!(
        role_pos < smith_pos,
        "Role should come before Smith. Got:
{}",
        result
    );
}

#[test]
fn test_whole_entry_linking_html() {
    use crate::render::html::Html;
    use csln_core::options::{LinkAnchor, LinkTarget, LinksConfig};

    let mut style = make_style();
    style.options.as_mut().unwrap().links = Some(LinksConfig {
        target: Some(LinkTarget::Url),
        anchor: Some(LinkAnchor::Entry),
        ..Default::default()
    });

    let mut bib = Bibliography::new();
    bib.insert(
        "link1".to_string(),
        Reference::from(LegacyReference {
            id: "link1".to_string(),
            ref_type: "webpage".to_string(),
            title: Some("Linked Page".to_string()),
            url: Some("https://example.com".to_string()),
            issued: Some(DateVariable::year(2023)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography_with_format::<Html>();

    // The whole entry content should be wrapped in an <a> tag inside the entry div
    assert!(result.contains(r#"id="ref-link1""#));
    assert!(result.contains(r#"<a href="https://example.com/">"#));
    assert!(result.contains("Linked Page"));
}

#[test]
fn test_global_title_linking_html() {
    use crate::render::html::Html;
    use csln_core::options::{LinkAnchor, LinkTarget, LinksConfig};

    let mut style = make_style();
    style.options.as_mut().unwrap().links = Some(LinksConfig {
        target: Some(LinkTarget::Doi),
        anchor: Some(LinkAnchor::Title),
        ..Default::default()
    });

    let mut bib = Bibliography::new();
    bib.insert(
        "doi1".to_string(),
        Reference::from(LegacyReference {
            id: "doi1".to_string(),
            ref_type: "book".to_string(),
            title: Some("Linked Title".to_string()),
            doi: Some("10.1001/test".to_string()),
            issued: Some(DateVariable::year(2023)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography_with_format::<Html>();

    println!("Result: {}", result);

    // The title should be automatically hyperlinked because of global config.
    // Note: In this test, title substitutes for author, so it gets csln-author class.
    assert!(result.contains(r#"<span class="csln-author"><a href="https://doi.org/10.1001/test">"#));
    assert!(result.contains("Linked Title"));
}

#[test]
fn test_numeric_integral_citation_author_year() {
    use csln_core::options::Processing;

    let mut style = make_style();
    // Override to numeric style
    style.options = Some(Config {
        processing: Some(Processing::Numeric),
        ..Default::default()
    });

    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    // Integral mode citation - should render author-year instead of number
    let citation = Citation {
        id: Some("c1".to_string()),
        mode: csln_core::citation::CitationMode::Integral,
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = processor.process_citation(&citation).unwrap();
    // For numeric+integral, expect author + citation number (no outer parens)
    assert_eq!(result, "Kuhn [1]");
}

#[test]
fn test_numeric_non_integral_citation_number() {
    use csln_core::citation::CitationMode;
    use csln_core::options::Processing;

    let mut style = make_style();
    // Override to numeric style with citation number template
    style.options = Some(Config {
        processing: Some(Processing::Numeric),
        ..Default::default()
    });
    style.citation = Some(csln_core::CitationSpec {
        template: Some(vec![TemplateComponent::Number(
            csln_core::template::TemplateNumber {
                number: csln_core::template::NumberVariable::CitationNumber,
                form: None,
                rendering: Rendering::default(),
                ..Default::default()
            },
        )]),
        wrap: Some(WrapPunctuation::Brackets),
        ..Default::default()
    });

    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    // Non-integral mode citation - should render citation number
    let citation = Citation {
        id: Some("c1".to_string()),
        mode: CitationMode::NonIntegral,
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = processor.process_citation(&citation).unwrap();
    // For numeric+non-integral, expect number format: "[1]"
    assert_eq!(result, "[1]");
}

#[test]
fn test_numeric_integral_with_multiple_items() {
    use csln_core::options::Processing;

    let mut style = make_style();
    style.options = Some(Config {
        processing: Some(Processing::Numeric),
        ..Default::default()
    });

    let mut bib = make_bibliography();
    bib.insert(
        "smith2020".to_string(),
        Reference::from(LegacyReference {
            id: "smith2020".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Smith", "Jane")]),
            issued: Some(DateVariable::year(2020)),
            ..Default::default()
        }),
    );

    let processor = Processor::new(style, bib);

    // Integral mode with multiple items
    let citation = Citation {
        id: Some("c1".to_string()),
        mode: csln_core::citation::CitationMode::Integral,
        items: vec![
            crate::reference::CitationItem {
                id: "kuhn1962".to_string(),
                ..Default::default()
            },
            crate::reference::CitationItem {
                id: "smith2020".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let result = processor.process_citation(&citation).unwrap();
    // Should render both as author + citation number
    assert!(result.contains("Kuhn [1]"));
    assert!(result.contains("Smith [2]"));
}

#[test]
fn test_citation_visibility_modifiers() {
    use csln_core::citation::CitationMode;
    use csln_core::citation::ItemVisibility;

    let style = make_style();
    let bib = make_bibliography();
    let processor = Processor::new(style, bib);

    // 1. Suppress Author
    let cit_suppress = Citation {
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            visibility: ItemVisibility::SuppressAuthor,
            ..Default::default()
        }],
        ..Default::default()
    };
    let res_suppress = processor.process_citation(&cit_suppress).unwrap();
    // Default APA style: (Kuhn, 1962). Suppress Author: (1962).
    assert_eq!(res_suppress, "(1962)");

    // 2. Integral Citation
    let cit_integral = Citation {
        mode: CitationMode::Integral,
        items: vec![crate::reference::CitationItem {
            id: "kuhn1962".to_string(),
            visibility: ItemVisibility::Default,
            ..Default::default()
        }],
        ..Default::default()
    };
    let res_integral = processor.process_citation(&cit_integral).unwrap();
    // Integral mode for author-date styles: Kuhn (1962)
    assert_eq!(res_integral, "Kuhn (1962)");
}

#[test]
fn test_bibliography_per_group_disambiguation() {
    use csln_core::grouping::{
        BibliographyGroup, DisambiguationScope, FieldMatcher, GroupHeading, GroupSelector,
    };

    let mut style = make_style();

    // Configure two groups, each with its own disambiguation scope
    style.bibliography.as_mut().unwrap().groups = Some(vec![
        BibliographyGroup {
            id: "group1".to_string(),
            heading: Some(GroupHeading::Literal {
                literal: "Group 1".to_string(),
            }),
            selector: GroupSelector {
                field: Some({
                    let mut map = HashMap::new();
                    map.insert("note".to_string(), FieldMatcher::Exact("g1".to_string()));
                    map
                }),
                ..Default::default()
            },
            sort: None,
            template: None,
            disambiguate: Some(DisambiguationScope::Locally),
        },
        BibliographyGroup {
            id: "group2".to_string(),
            heading: Some(GroupHeading::Literal {
                literal: "Group 2".to_string(),
            }),
            selector: GroupSelector {
                field: Some({
                    let mut map = HashMap::new();
                    map.insert("note".to_string(), FieldMatcher::Exact("g2".to_string()));
                    map
                }),
                ..Default::default()
            },
            sort: None,
            template: None,
            disambiguate: Some(DisambiguationScope::Locally),
        },
    ]);

    // Set up references that would normally disambiguate globally
    let mut bib = Bibliography::new();
    // Two Kuhn 1962 in Group 1
    bib.insert(
        "r1".to_string(),
        Reference::from(LegacyReference {
            id: "r1".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas")]),
            issued: Some(DateVariable::year(1962)),
            title: Some("B title".to_string()),
            note: Some("g1".to_string()),
            ..Default::default()
        }),
    );
    bib.insert(
        "r2".to_string(),
        Reference::from(LegacyReference {
            id: "r2".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas")]),
            issued: Some(DateVariable::year(1962)),
            title: Some("A title".to_string()),
            note: Some("g1".to_string()),
            ..Default::default()
        }),
    );
    // Same name/year in Group 2 - should RESTART suffixes if locally disambiguated
    bib.insert(
        "r3".to_string(),
        Reference::from(LegacyReference {
            id: "r3".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas")]),
            issued: Some(DateVariable::year(1962)),
            title: Some("C title".to_string()),
            note: Some("g2".to_string()),
            ..Default::default()
        }),
    );
    bib.insert(
        "r4".to_string(),
        Reference::from(LegacyReference {
            id: "r4".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas")]),
            issued: Some(DateVariable::year(1962)),
            title: Some("D title".to_string()),
            note: Some("g2".to_string()),
            ..Default::default()
        }),
    );

    // Ensure year-suffix is enabled in style
    style.options.as_mut().unwrap().processing = Some(csln_core::options::Processing::Custom(
        csln_core::options::ProcessingCustom {
            disambiguate: Some(csln_core::options::Disambiguation {
                year_suffix: true,
                ..Default::default()
            }),
            ..Default::default()
        },
    ));

    let processor = Processor::new(style, bib);
    let result =
        processor.render_grouped_bibliography_with_format::<crate::render::plain::PlainText>();

    assert!(result.contains("Group 2"));
    // Group 2 should have its own 1962a and 1962b
    let count_a = result.matches("1962a").count();
    assert_eq!(
        count_a, 2,
        "1962a should appear in both groups if disambiguated locally. Output: {}",
        result
    );
}

#[test]
fn test_group_heading_localized_uses_processor_locale() {
    use csln_core::grouping::{BibliographyGroup, GroupHeading, GroupSelector};

    let mut style = make_style();
    style.bibliography.as_mut().unwrap().groups = Some(vec![BibliographyGroup {
        id: "all".to_string(),
        heading: Some(GroupHeading::Localized {
            localized: HashMap::from([
                ("en-US".to_string(), "English Sources".to_string()),
                ("vi".to_string(), "Tài liệu tiếng Việt".to_string()),
            ]),
        }),
        selector: GroupSelector::default(),
        sort: None,
        template: None,
        disambiguate: None,
    }]);

    let mut locale = csln_core::Locale::en_us();
    locale.locale = "vi-VN".to_string();

    let processor = Processor::with_locale(style, make_bibliography(), locale);
    let output =
        processor.render_grouped_bibliography_with_format::<crate::render::plain::PlainText>();

    assert!(output.contains("# Tài liệu tiếng Việt"));
}

#[test]
fn test_group_heading_term_resolves_from_locale() {
    use csln_core::grouping::{BibliographyGroup, GroupHeading, GroupSelector};
    use csln_core::locale::{GeneralTerm, TermForm};

    let mut style = make_style();
    style.bibliography.as_mut().unwrap().groups = Some(vec![BibliographyGroup {
        id: "all".to_string(),
        heading: Some(GroupHeading::Term {
            term: GeneralTerm::And,
            form: Some(TermForm::Long),
        }),
        selector: GroupSelector::default(),
        sort: None,
        template: None,
        disambiguate: None,
    }]);

    let processor = Processor::new(style, make_bibliography());
    let output =
        processor.render_grouped_bibliography_with_format::<crate::render::plain::PlainText>();

    assert!(output.contains("# and"));
}
