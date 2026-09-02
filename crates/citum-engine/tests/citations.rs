/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in test, benchmark, and example code."
)]

/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

mod common;
use citum_schema::reference::ClassExtension;
use common::*;
use rstest::rstest;

use citum_engine::{Processor, render::html::Html};
use citum_schema::{
    CitationOptions, CitationSpec, Style, StyleInfo,
    citation::{Citation, CitationItem, CitationMode, IntegralNameState},
    grouping::{GroupSort, GroupSortEntry, GroupSortKey, SortKey as GroupSortKeyType},
    options::{
        AndOptions, CitationLabelMode, Config, ContributorConfig, DateConfig, DateFallbackConfig,
        DateFallbackPreset, DelimiterPrecedesLast, DisplayAsSort, GivennameRule,
        IntegralNameContexts, IntegralNameMemoryConfig, IntegralNameScope, MultilingualConfig,
        MultilingualMode, NameForm, Processing, ProcessingCustom, ReferenceTypeName,
        ShortenListOptions, SubsequentNameForm, Substitute, SubstituteConfig,
        SubstituteTitleQuoteMode, TitleCategory, TitleRendering, TitlesConfig,
        TwoNameDelimiterPolicy,
    },
    reference::{DateValue, InputReference, Monograph, MonographType, Title},
};

// --- Helper Functions ---

fn build_numeric_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Numeric Test".to_string()),
            id: Some("numeric-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            options: Some(citum_schema::CitationOptions {
                label_mode: Some(citum_schema::options::CitationLabelMode::Numeric),
                ..Default::default()
            }),
            wrap: Some(citum_schema::template::WrapPunctuation::Brackets.into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_title_year_citation_style(sort: Vec<GroupSortKey>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Title Year Citation Sort Test".to_string()),
            id: Some("title-year-citation-sort-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            date_fallback: Some(DateFallbackConfig::Policy(
                DateFallbackPreset::Standard.config(),
            )),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            options: Some(CitationOptions {
                label_mode: Some(CitationLabelMode::None),
                ..Default::default()
            }),
            sort: Some(GroupSortEntry::Explicit(GroupSort { template: sort })),
            template: Some(
                vec![
                    citum_schema::tc_title!(Primary),
                    citum_schema::tc_date!(Issued, Year),
                ]
                .into(),
            ),
            delimiter: Some(" ".into()),
            multi_cite_delimiter: Some("; ".into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_integral_name_style() -> Style {
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
                template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
                ..Default::default()
            })),
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Short),
                    citum_schema::tc_date!(
                        Issued,
                        Year,
                        wrap = citum_schema::template::WrapPunctuation::Parentheses
                    ),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_author_date_style_with_givenname_rule(rule: GivennameRule) -> Style {
    let mut style = common::build_author_date_style(true, true, true, Some(3), Some(1));

    if let Some(options) = style.options.as_mut()
        && let Some(Processing::Custom(custom)) = options.processing.as_mut()
        && let Some(disambiguate) = custom.disambiguate.as_mut()
    {
        disambiguate.givenname_rule = rule;
    }

    style
}

fn make_undated_book(id: &str, family: &str, given: &str, title: &str) -> InputReference {
    let mut reference = make_book(id, family, given, 2020, title);
    if let ClassExtension::Monograph(monograph) = reference.extension_mut() {
        monograph.issued = DateValue::new(String::new());
    }
    reference
}

fn by_cite_scope_fixture() -> Vec<InputReference> {
    vec![
        make_book_multi_author(
            "ASTHMA-A",
            vec![
                ("Asthma", "Albert"),
                ("Bronchitis", "Brandon"),
                ("Cold", "Crispin"),
            ],
            1990,
            "Book A",
        ),
        make_book_multi_author(
            "ASTHMA-B",
            vec![
                ("Asthma", "Albert"),
                ("Bronchitis", "Edward"),
                ("Cold", "Crispin"),
            ],
            1990,
            "Book B",
        ),
        make_book_multi_author(
            "DROPSY-A",
            vec![
                ("Dropsy", "Devon"),
                ("Enteritis", "Edward"),
                ("Fever", "Xavier"),
            ],
            2000,
            "Book C",
        ),
        make_book_multi_author(
            "DROPSY-B",
            vec![
                ("Dropsy", "Devon"),
                ("Enteritis", "Frank"),
                ("Fever", "Yves"),
            ],
            2000,
            "Book D",
        ),
    ]
}

fn processor_for_givenname_rule(rule: GivennameRule) -> Processor {
    let mut bibliography = indexmap::IndexMap::new();
    for reference in by_cite_scope_fixture() {
        let id = reference.id().expect("fixture reference id").to_string();
        bibliography.insert(id, reference);
    }

    Processor::new(
        build_author_date_style_with_givenname_rule(rule),
        bibliography,
    )
}

fn process_citation_ids(processor: &Processor, ids: &[&str]) -> String {
    processor
        .process_citation(&Citation {
            items: ids
                .iter()
                .map(|id| CitationItem {
                    id: (*id).to_string(),
                    ..Default::default()
                })
                .collect(),
            mode: CitationMode::NonIntegral,
            ..Default::default()
        })
        .expect("citation should render")
}

#[rstest]
#[case::literal_default(None, "John Smith, and Jane Jones")]
#[case::apa_suppression(
    Some(TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst),
    "John Smith and Jane Jones"
)]
fn given_two_names_when_rendering_a_citation_then_policy_controls_the_delimiter(
    #[case] policy: Option<TwoNameDelimiterPolicy>,
    #[case] expected: &str,
) {
    let style = Style {
        info: StyleInfo {
            title: Some("Two Author Citation And Test".to_string()),
            id: Some("two-author-citation-and-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            contributors: Some(ContributorConfig {
                and: Some(AndOptions::Text),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                two_name_delimiter_policy: policy,
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        make_book_multi_author(
            "item1",
            vec![("Smith", "John"), ("Jones", "Jane")],
            2020,
            "Title",
        ),
    );
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, expected);
}

fn make_authorless_book(id: &str, title: &str) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        ..Default::default()
    }))
}

fn substitute_title_style(title_quote: Option<SubstituteTitleQuoteMode>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Substitute Title Quote Test".to_string()),
            id: Some("substitute-title-quote-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            substitute: Some(SubstituteConfig::Explicit(Substitute {
                candidates: Some(citum_schema::options::SubstituteCandidates::Candidates(
                    vec![citum_schema::options::SubstituteKey::Title],
                )),
                title_quote,
                ..Default::default()
            })),
            titles: Some(TitlesConfig {
                monograph: Some(TitleRendering {
                    quote: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn authorless_book_cited_by_title_in_citation_is_not_quoted_when_category_quoting_is_enabled() {
    // With `title-quote: by-category`, a substituted book title defers to the
    // style's `titles.monograph` config (quote: false here), matching how
    // APA/citeproc-js render an authorless book by title (italicized, not
    // quoted). See div-011 in docs/adjudication/DIVERGENCE_REGISTER.md.
    let style = substitute_title_style(Some(SubstituteTitleQuoteMode::ByCategory));
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("item1".to_string(), make_authorless_book("item1", "Title"));
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, "Title");
}

#[test]
fn authorless_book_cited_by_title_in_citation_defaults_to_unconditional_quoting() {
    // Unset `title-quote` preserves the historical behavior: the substituted
    // title is quoted unconditionally in citation context regardless of the
    // reference's title-category config, so existing styles' output does
    // not change by default.
    let style = substitute_title_style(None);
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("item1".to_string(), make_authorless_book("item1", "Title"));
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, "\u{201c}Title\u{201d}");
}

fn make_authorless_monograph(id: &str, title: &str, kind: MonographType) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: kind,
        title: Some(Title::Single(title.to_string())),
        ..Default::default()
    }))
}

/// Mirrors elsevier-harvard-core.yaml's citation-scoped `by-category` config
/// (SUBSTITUTED_VALUE_FORMATTING.md): book/report/graphic/motion-picture/song/
/// statute map to `monograph` (emph); `thesis` is explicitly overridden away
/// from the engine's hardcoded book/thesis/report default so it stays
/// quoted, matching elsevier-harvard.csl's real `author-short` rule; the
/// `component` and `default` categories both declare `quote: true` so no
/// reachable type silently falls through to plain text.
fn elsevier_harvard_style_substitute_title_style() -> Style {
    let mut type_mapping = std::collections::HashMap::new();
    type_mapping.insert(ReferenceTypeName::new("book"), TitleCategory::Monograph);
    type_mapping.insert(ReferenceTypeName::new("report"), TitleCategory::Monograph);
    type_mapping.insert(ReferenceTypeName::new("thesis"), TitleCategory::Default);

    Style {
        info: StyleInfo {
            title: Some("Elsevier Harvard Substitute Title Quote Test".to_string()),
            id: Some("elsevier-harvard-substitute-title-quote-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            substitute: Some(SubstituteConfig::Explicit(Substitute {
                candidates: Some(citum_schema::options::SubstituteCandidates::Candidates(
                    vec![citum_schema::options::SubstituteKey::Title],
                )),
                title_quote: Some(SubstituteTitleQuoteMode::ByCategory),
                ..Default::default()
            })),
            titles: Some(TitlesConfig {
                type_mapping: Some(type_mapping),
                monograph: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                component: Some(TitleRendering {
                    quote: Some(true),
                    ..Default::default()
                }),
                default: Some(TitleRendering {
                    quote: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[rstest]
#[case::book_maps_to_monograph_and_italicizes(MonographType::Book, "_Title_")]
#[case::report_maps_to_monograph_and_italicizes(MonographType::Report, "_Title_")]
#[case::thesis_overridden_away_from_monograph_stays_quoted(
    MonographType::Thesis,
    "\u{201c}Title\u{201d}"
)]
#[case::post_falls_to_component_bucket_and_quotes(MonographType::Post, "\u{201c}Title\u{201d}")]
#[case::interview_falls_to_default_bucket_and_quotes(
    MonographType::Interview,
    "\u{201c}Title\u{201d}"
)]
fn given_elsevier_harvard_shaped_by_category_config_when_type_varies_then_quoting_matches_the_type_mapping(
    #[case] kind: MonographType,
    #[case] expected: &str,
) {
    // Regression coverage for the elsevier-harvard flip in
    // SUBSTITUTED_VALUE_FORMATTING.md stage 2: `book`/`report` (explicit
    // `type-mapping` to `monograph`) italicize; `thesis` (explicitly
    // overridden to `default`, since the engine's hardcoded type-class
    // table would otherwise route it to `monograph` too) and `post` (the
    // engine's hardcoded `component` bucket) and `interview` (falls to
    // `default`, in neither hardcoded bucket) all stay quoted -- matching
    // real elsevier-harvard.csl's `author-short` macro, oracle-verified
    // against citeproc-js in the spec.
    let style = elsevier_harvard_style_substitute_title_style();
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        make_authorless_monograph("item1", "Title", kind),
    );
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, expected);
}

/// Mirrors taylor-and-francis-council-of-science-editors-author-date-core.yaml's
/// citation-scoped `by-category` config (SUBSTITUTED_VALUE_FORMATTING.md):
/// `title-quote: by-category` with no `titles:` block at all. Unlike
/// elsevier-harvard, this style's real `author`/`author-short` macros are
/// unconditionally plain (bare `<text variable="title" form="short"/>`, no
/// type branching) -- so the absence of a `titles:` config is the correct,
/// complete fix, not a coverage gap: no category ever resolves, so nothing
/// is ever quoted or emphasized, matching the oracle-verified real rule.
fn cse_style_substitute_title_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("T&F CSE Substitute Title Quote Test".to_string()),
            id: Some("cse-substitute-title-quote-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            substitute: Some(SubstituteConfig::Explicit(Substitute {
                candidates: Some(citum_schema::options::SubstituteCandidates::Candidates(
                    vec![citum_schema::options::SubstituteKey::Title],
                )),
                title_quote: Some(SubstituteTitleQuoteMode::ByCategory),
                ..Default::default()
            })),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[rstest]
#[case::book(MonographType::Book)]
#[case::report(MonographType::Report)]
#[case::post(MonographType::Post)]
#[case::interview(MonographType::Interview)]
fn given_cse_shaped_by_category_config_with_no_titles_block_when_type_varies_then_title_always_renders_plain(
    #[case] kind: MonographType,
) {
    let style = cse_style_substitute_title_style();
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        make_authorless_monograph("item1", "Title", kind),
    );
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, "Title");
}

fn integral_name_state_overrides_processor_memory() {
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Book A"),
    );
    let processor = Processor::new(build_integral_name_style(), bibliography);

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
            .expect("first should render"),
        "John Smith"
    );
    assert_eq!(
        processor
            .process_citation(&subsequent)
            .expect("subsequent should render"),
        "Smith"
    );
}

fn absent_memory_block_does_not_rewrite_subsequent_name_state() {
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Book A"),
    );
    let mut style = build_integral_name_style();
    style.options.as_mut().unwrap().integral_name_memory = None;
    let processor = Processor::new(style, bibliography);

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
            .process_citation(&subsequent)
            .expect("should render"),
        "John Smith"
    );
}

// --- Disambiguation Scenarios ---

/// Test year suffix disambiguation with alphabetical title sorting.
fn disambiguation_same_author_same_year_titles_follow_title_order() {
    let input = vec![
        make_book("item1", "Smith", "John", 2020, "Alpha"),
        make_book("item2", "Smith", "John", 2020, "Beta"),
    ];
    let citation_items = vec![vec!["item1", "item2"]];
    let expected = "Smith, (2020a), (2020b)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test the upstream `YearSuffixAtTwoLevels` disambiguation cascade.
#[allow(
    clippy::too_many_lines,
    reason = "test functions naturally exceed 100 lines"
)]
fn disambiguation_two_level_author_collisions_get_distinct_suffixes() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Smith", "John"), ("Jones", "John"), ("Brown", "John")],
            1986,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![("Smith", "John"), ("Jones", "John"), ("Brown", "John")],
            1986,
            "Book B",
        ),
        make_book_multi_author(
            "ITEM-3",
            vec![
                ("Smith", "John"),
                ("Jones", "John"),
                ("Brown", "John"),
                ("Green", "John"),
            ],
            1986,
            "Book C",
        ),
        make_book_multi_author(
            "ITEM-4",
            vec![
                ("Smith", "John"),
                ("Jones", "John"),
                ("Brown", "John"),
                ("Green", "John"),
            ],
            1986,
            "Book D",
        ),
    ];

    let mut style = build_author_date_style(true, true, false, Some(3), Some(1));
    style.options = Some(Config {
        processing: Some(Processing::Custom(ProcessingCustom {
            base: None,
            disambiguate: Some(citum_schema::options::Disambiguation {
                year_suffix: true,
                names: true,
                add_givenname: false,
                givenname_rule: GivennameRule::default(),
            }),
            ..Default::default()
        })),
        contributors: Some(ContributorConfig {
            display_as_sort: Some(DisplayAsSort::First),
            initialize_with: Some(String::new()),
            shorten: Some(ShortenListOptions {
                min: 3,
                use_first: 1,
                ..Default::default()
            }),
            and: Some(AndOptions::Symbol),
            delimiter_precedes_last: Some(DelimiterPrecedesLast::Never),
            ..Default::default()
        }),
        ..Default::default()
    });
    style.citation = Some(CitationSpec {
        sort: build_author_date_style(true, true, false, Some(3), Some(1))
            .citation
            .as_ref()
            .and_then(|spec| spec.sort.clone()),
        template: Some(
            vec![
                citum_schema::tc_contributor!(Author, Short),
                citum_schema::tc_date!(
                    Issued,
                    Year,
                    wrap = citum_schema::template::WrapPunctuation::Parentheses
                ),
            ]
            .into(),
        ),
        delimiter: Some(" ".into()),
        multi_cite_delimiter: Some("; ".into()),
        ..Default::default()
    });

    let mut bibliography = indexmap::IndexMap::new();
    for item in input {
        if let Some(id) = item.id() {
            bibliography.insert(id.to_string(), item);
        }
    }

    let processor = Processor::new(style, bibliography);
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "ITEM-1".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-2".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-3".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-4".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let result = processor
        .process_citation(&citation)
        .expect("Failed to process two-level year-suffix disambiguation citation");
    assert_eq!(
        result,
        "Smith, Jones & Brown (1986a); Smith, Jones & Brown (1986b); Smith, Jones, Brown, et al. (1986a); Smith, Jones, Brown, et al. (1986b)"
    );
}

/// Test year suffix disambiguation with multiple identical references.
fn disambiguation_same_year_articles_increment_suffixes() {
    let input = vec![
        make_article("22", "Ylinen", "A", 1995, "Article A"),
        make_article("21", "Ylinen", "A", 1995, "Article B"),
        make_article("23", "Ylinen", "A", 1995, "Article C"),
    ];
    let citation_items = vec![vec!["22", "21", "23"]];
    let expected = "Ylinen, (1995a), (1995b), (1995c)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Guard against spurious given-name expansion: when all family-name collisions
/// are already resolved by different issued years, `add_givenname` must not add
/// initials or given names. The three Asthma refs differ in year (1885 vs 1980)
/// so they form distinct collision groups; no given-name expansion is triggered.
fn disambiguation_no_spurious_givenname_expansion_when_years_differ() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Asthma", "Albert"), ("Asthma", "Bridget")],
            1980,
            "Book A",
        ),
        make_book("ITEM-2", "Bronchitis", "Beauregarde", 1995, "Book B"),
        make_book("ITEM-3", "Asthma", "Albert", 1885, "Book C"),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2", "ITEM-3"]];
    // Sorted by author (Asthma, then Bronchitis) and year (1885, then 1980).
    // No given-name expansion: the 1885 and 1980 Asthma groups are distinct.
    let expected = "Asthma, (1885); Asthma, Asthma, (1980); Bronchitis, (1995)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: false,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// Positive §2 given-name expansion: two same-year books whose authors share a
/// family name but have different given names must have initials injected only
/// for the ambiguous pair; a third non-colliding author stays unexpanded.
fn disambiguation_givenname_expansion_resolves_same_year_family_name_collision() {
    let input = vec![
        make_book("ITEM-A", "Smith", "Alice", 2000, "Book A"),
        make_book("ITEM-B", "Smith", "Bob", 2000, "Book B"),
        make_book("ITEM-C", "Jones", "Carol", 2000, "Book C"),
    ];
    let citation_items = vec![vec!["ITEM-A", "ITEM-B", "ITEM-C"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        // Jones is unambiguous — stays bare. Both Smiths expand to initials.
        expected: "Jones, (2000); A Smith, (2000); B Smith, (2000)",
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: false,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// Row 138 regression: year-suffix letters must follow the bibliography
/// title sort key, not a raw lowercased title. Under raw lowercasing "An
/// Ecology…" sorts before "Biology…", matching the correct letter
/// assignment only by coincidence of case, not text. The real sort key
/// (`title_sort_key_with_options`) uses the literal title text — CSL has no
/// automatic leading-article stripping (csl26-rrsb) — so "An Ecology of
/// Rivers" ('A') sorts before "Biology of Lakes" ('B') and takes `a`;
/// Biology takes `b`. This style has no resolved `bibliography.sort`
/// (`build_author_date_style` sets `citation.sort` only), so the comparison
/// goes through `sort_group_for_year_suffix`'s no-sort fallback — byte-wise
/// `String::cmp` on the cached title key, not the locale collator.
fn disambiguation_year_suffix_follows_literal_title_order() {
    let input = vec![
        make_book("eco", "Garcia", "Maria", 2019, "An Ecology of Rivers"),
        make_book("bio", "Garcia", "Maria", 2019, "Biology of Lakes"),
    ];
    // Cite Biology first, Ecology second: Biology reads 2019b, Ecology 2019a.
    let citation_items = vec![vec!["bio", "eco"]];
    let expected = "Garcia, (2019b), (2019a)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

#[test]
fn disambiguation_year_suffix_applies_to_no_date_terms() {
    let input = vec![
        make_undated_book("alpha", "Smith", "Jane", "Alpha"),
        make_undated_book("beta", "Smith", "Jane", "Beta"),
    ];
    let citation_items = vec![vec!["alpha", "beta"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected: "Smith, (n.d.-a), (n.d.-b)",
        mode: "citation",
        disambiguate_year_suffix: true,
        disambiguate_names: false,
        disambiguate_givenname: false,
        et_al_min: None,
        et_al_use_first: None,
    });
}

#[test]
fn disambiguation_no_date_year_suffix_uses_configured_delimiter() {
    let mut style = common::build_author_date_style(true, false, false, None, None);
    if let Some(options) = style.options.as_mut() {
        options.dates = Some(DateConfig {
            no_date_year_suffix_delimiter: String::new(),
            ..Default::default()
        });
    }
    let bibliography = indexmap::indexmap! {
        "alpha".to_string() => make_undated_book("alpha", "Smith", "Jane", "Alpha"),
        "beta".to_string() => make_undated_book("beta", "Smith", "Jane", "Beta"),
    };
    let processor = Processor::new(style, bibliography);
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "alpha".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "beta".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let result = processor
        .process_citation(&citation)
        .expect("undated disambiguation citation should render");

    assert_eq!(result, "Smith, (n.d.a), (n.d.b)");
}

/// Row 114 regression: same-surname, different-given-name authors in the same
/// year ("A. Johnson 2020" / "B. Johnson 2020") must disambiguate by given-name
/// initials, never by a spurious year suffix — even when year-suffix
/// disambiguation is also enabled (as in apa-7th via the `author-date-full`
/// preset). Given-name expansion precedes year suffix in the cascade and resolves
/// the collision, so no `2020a`/`2020b` letter is added.
fn disambiguation_givenname_expansion_preferred_over_year_suffix() {
    let input = vec![
        make_book("alan", "Johnson", "Alan", 2020, "Cognition"),
        make_book("bea", "Johnson", "Beatrice", 2020, "Memory"),
    ];
    let citation_items = vec![vec!["alan", "bea"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        // Initials resolve the surname collision; no `2020a`/`2020b`.
        expected: "A Johnson, (2020); B Johnson, (2020)",
        mode: "citation",
        disambiguate_year_suffix: true,
        disambiguate_names: true,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// Row 173 regression (MLA): author-page styles set year-suffix off. When two
/// same-author/same-year works cannot be resolved by names or given names (the
/// author is identical), no suffix letter is emitted — modern-language-association
/// relies on its `disambiguate-only` short title instead. Guards against a
/// regression that reintroduces `2019a`/`2019b` for MLA.
fn disambiguation_year_suffix_off_emits_no_letter() {
    let input = vec![
        make_book("a", "Garcia", "Maria", 2019, "Rivers"),
        make_book("b", "Garcia", "Maria", 2019, "Lakes"),
    ];
    let citation_items = vec![vec!["a", "b"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        // year-suffix disabled: identical author/year render with no letter.
        expected: "Garcia, (2019), (2019)",
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: true,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// Row 114 (global, cross-citation): APA-7th uses `primary-name-with-initials`,
/// whose detection is global. Same-surname authors cited in *separate* citations
/// both receive first-author initials (APA §8.20) and no year suffix. The engine's
/// `by-cite` default would miss this — it only compares references appearing
/// together in one citation — which is why apa-7th sets the rule explicitly.
fn disambiguation_primary_name_initials_expand_globally_across_citations() {
    let input = vec![
        make_book("aj", "Johnson", "Alan", 2020, "Cognition"),
        make_book("bj", "Johnson", "Beatrice", 2020, "Memory"),
    ];
    let mut bibliography = indexmap::IndexMap::new();
    for reference in input {
        let id = reference.id().expect("fixture id").to_string();
        bibliography.insert(id, reference);
    }
    let processor = Processor::new(
        build_author_date_style_with_givenname_rule(GivennameRule::PrimaryNameWithInitials),
        bibliography,
    );

    // Each author is cited in its own citation; global detection still expands
    // both to initials, and neither gains a `2020a`/`2020b` suffix.
    assert_eq!(
        process_citation_ids(&processor, &["aj"]),
        "A Johnson, (2020)"
    );
    assert_eq!(
        process_citation_ids(&processor, &["bj"]),
        "B Johnson, (2020)"
    );
}

fn disambiguation_by_cite_givenname_expansion_is_document_wide() {
    let processor = processor_for_givenname_rule(GivennameRule::ByCite);

    let asthma = process_citation_ids(&processor, &["ASTHMA-A", "ASTHMA-B"]);
    // DROPSY-A collides with DROPSY-B (csl26-8nrt fixture) in the full bibliography,
    // even though DROPSY-B is never cited in this test — collision detection is
    // document-wide, not scoped to what a citation names.
    let dropsy = process_citation_ids(&processor, &["DROPSY-A"]);

    assert_eq!(
        asthma,
        "A Asthma, B Bronchitis, et al., (1990); A Asthma, E Bronchitis, et al., (1990)"
    );
    assert_eq!(dropsy, "D Dropsy, E Enteritis, et al., (2000)");
}

fn disambiguation_all_names_givenname_expansion_remains_global() {
    let processor = processor_for_givenname_rule(GivennameRule::AllNames);

    let dropsy = process_citation_ids(&processor, &["DROPSY-A"]);

    assert_eq!(dropsy, "D Dropsy, E Enteritis, et al., (2000)");
}

fn disambiguation_by_cite_solo_cite_still_expands() {
    let processor = processor_for_givenname_rule(GivennameRule::ByCite);

    // ASTHMA-A is in a global collision group with ASTHMA-B, cited alone here.
    // Collision detection is document-wide: a reference cited alone still expands
    // if it collides with a reference elsewhere in the bibliography (csl26-8nrt).
    let solo = process_citation_ids(&processor, &["ASTHMA-A"]);

    assert_eq!(solo, "A Asthma, B Bronchitis, et al., (1990)");
}

fn disambiguation_by_cite_mixed_groups_all_expand() {
    let processor = processor_for_givenname_rule(GivennameRule::ByCite);

    // ASTHMA-A/B collide with each other; DROPSY-A collides with DROPSY-B, which
    // isn't cited here at all. All three still expand — collision membership never
    // depends on what else appears in the same citation.
    let mixed = process_citation_ids(&processor, &["ASTHMA-A", "ASTHMA-B", "DROPSY-A"]);

    assert_eq!(
        mixed,
        "A Asthma, B Bronchitis, et al., (1990); A Asthma, E Bronchitis, et al., (1990); D Dropsy, E Enteritis, et al., (2000)"
    );
}

fn disambiguation_all_names_co_citation() {
    let processor = processor_for_givenname_rule(GivennameRule::AllNames);

    // Both DROPSY works collide globally; citing them together must expand both.
    let co = process_citation_ids(&processor, &["DROPSY-A", "DROPSY-B"]);

    assert_eq!(
        co,
        "D Dropsy, E Enteritis, et al., (2000); D Dropsy, F Enteritis, et al., (2000)"
    );
}

fn disambiguation_primary_name_givenname_expansion() {
    let processor = processor_for_givenname_rule(GivennameRule::PrimaryName);

    // ASTHMA-A and ASTHMA-B share the same primary author (A Asthma).  Expanding the
    // first author's given name cannot resolve the collision, so the cascade must fall
    // through to year-suffix.  The et-al expansion to two names is retained alongside
    // the suffix — the name form must not collapse back to one-name et-al.
    let asthma = process_citation_ids(&processor, &["ASTHMA-A", "ASTHMA-B"]);

    assert_eq!(
        asthma,
        "A Asthma, Bronchitis, et al., (1990a); A Asthma, Bronchitis, et al., (1990b)"
    );
}

/// §2.1 primary-name *success* path: when the primary authors' given names differ,
/// expansion resolves the collision and year-suffix must NOT be applied.
fn disambiguation_primary_name_givenname_expansion_resolves_distinct_primary_authors() {
    let mut bibliography = indexmap::IndexMap::new();
    for reference in [
        make_book("ALICE", "Smith", "Alice", 2000, "Book A"),
        make_book("BOB", "Smith", "Bob", 2000, "Book B"),
    ] {
        let id = reference.id().expect("fixture reference id").to_string();
        bibliography.insert(id, reference);
    }

    let processor = Processor::new(
        build_author_date_style_with_givenname_rule(GivennameRule::PrimaryName),
        bibliography,
    );

    let result = process_citation_ids(&processor, &["ALICE", "BOB"]);

    assert_eq!(result, "A Smith, (2000); B Smith, (2000)");
}

/// Test et-al expansion success: Name expansion disambiguates conflicting references.
fn disambiguation_et_al_conflicts_expand_names_when_that_resolves_them() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Smith", "John"), ("Brown", "John"), ("Jones", "John")],
            1980,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![
                ("Smith", "John"),
                ("Beefheart", "Captain"),
                ("Jones", "John"),
            ],
            1980,
            "Book B",
        ),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2"]];
    // Expected order updated for csl26-7u16's follow-up fix (author sort key
    // now compares every co-author, not just the first): the style's
    // explicit citation <sort> by author ties ITEM-1/ITEM-2 on their shared
    // first author, so it now correctly breaks on the second ("Beefheart" <
    // "Brown"), placing ITEM-2 first — verified directly against
    // `CSL.Engine` for this exact shape (same style structure: an explicit
    // author+issued citation sort, et-al-min 3, et-al-use-first 1,
    // disambiguate-add-names).
    let expected = "Smith, Beefheart, et al., (1980); Smith, Brown, et al., (1980)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: true,
        disambiguate_givenname: false,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

/// Test et-al expansion failure: Cascade to year suffix when name expansion fails.
fn disambiguation_et_al_conflicts_fall_back_to_year_suffixes() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Smith", "John"), ("Brown", "John"), ("Jones", "John")],
            1980,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![("Smith", "John"), ("Brown", "John"), ("Jones", "John")],
            1980,
            "Book B",
        ),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2"]];
    let expected = "Smith et al., (1980a), (1980b)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: true,
        disambiguate_names: true,
        disambiguate_givenname: false,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

/// Regression for csl26-8nrt: under the default `by-cite` givenname rule,
/// et-al-expansion depth (`disambiguate-add-names`, cascade strategy 1) must not
/// be affected by citation scope. `by-cite` only governs given-name expansion
/// (strategy 2); it has no authority over strategy 1's `min_names_to_show`.
///
/// ITEM-A and ITEM-B mirror the fixture that exposed the bug
/// (taylor-and-francis-council-of-science-editors-author-date's
/// `et-al-single-long-list`/`et-al-with-locator` citations, ITEM-29/ITEM-30 in
/// tests/fixtures/references-expanded.json): four authors each, same year,
/// diverging only at the third author. Three names are the minimum that tells
/// them apart, so both the oracle and a correct engine show exactly three
/// before `et al.` — whether the colliding reference is cited alone or
/// alongside its collider.
#[rstest]
#[case::cited_alone(
    vec![vec!["ITEM-A"]],
    "Smith, Lee, Kumar, et al., (2021)"
)]
#[case::cited_together(
    vec![vec!["ITEM-A", "ITEM-B"]],
    "Smith, Lee, Kumar, et al., (2021); Smith, Lee, Nguyen, et al., (2021)"
)]
fn given_a_by_cite_style_with_an_et_al_collision_when_a_colliding_reference_is_cited_then_the_global_expansion_depth_applies(
    #[case] citation_items: Vec<Vec<&str>>,
    #[case] expected: &str,
) {
    let input = vec![
        make_book_multi_author(
            "ITEM-A",
            vec![
                ("Smith", "John"),
                ("Lee", "Alice"),
                ("Kumar", "Priya"),
                ("Zhou", "Ming"),
            ],
            2021,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-B",
            vec![
                ("Smith", "John"),
                ("Lee", "Alice"),
                ("Nguyen", "Bao"),
                ("Patel", "Ravi"),
            ],
            2021,
            "Book B",
        ),
    ];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: true,
        disambiguate_givenname: true,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

/// Regression for csl26-h9jy and csl26-5753, mirroring the official CSL
/// test suite's `disambiguate_ByCiteMinimalGivennameExpandMinimalNames.txt`
/// fixture exactly (same names, same style shape: et-al-min 3,
/// et-al-use-first 1, by-cite, initials baseline). Two references share
/// their first and third authors verbatim (Albert Asthma; Charles/Curtis
/// Cold differs only at position 3, never shown); only the second author
/// (Brandon vs Biff Bronchitis) differs at a shown position, and both
/// reduce to the same initial ("B."), so initials alone don't resolve it —
/// full-name escalation is required.
///
/// `by-cite` tracks escalation per author position (csl26-5753): position 0
/// ("Albert Asthma", identical in both references) stays at its default
/// initials depth — escalating it to the full given name would gain
/// nothing, since both entries share the same person there — while
/// position 1 escalates all the way to the full given name, matching the
/// oracle's own minimal output.
fn disambiguation_givenname_escalation_is_minimal_per_position() {
    let input = vec![
        make_book_multi_author(
            "ambigs-16",
            vec![
                ("Asthma", "Albert"),
                ("Bronchitis", "Brandon"),
                ("Cold", "Charles"),
            ],
            1990,
            "Book M",
        ),
        make_book_multi_author(
            "ambigs-17",
            vec![
                ("Asthma", "Albert"),
                ("Bronchitis", "Biff"),
                ("Cold", "Curtis"),
            ],
            1990,
            "Book M",
        ),
    ];
    let citation_items = vec![vec!["ambigs-16", "ambigs-17"]];
    let expected =
        "A Asthma, Biff Bronchitis, et al., (1990); A Asthma, Brandon Bronchitis, et al., (1990)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: true,
        disambiguate_givenname: true,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

/// Companion to `disambiguation_givenname_escalation_is_minimal_per_position`,
/// same input, `GivennameRule::AllNames` instead of the default `ByCite`:
/// `all-names` has no per-position ceiling at all, so it escalates *every*
/// shown position to the full given name uniformly, including position 0
/// (which gains nothing from it — both entries share "Albert Asthma"
/// there). This locks in the fork the two rules take on identical input,
/// rather than leaving the distinction implicit.
fn disambiguation_all_names_givenname_escalation_is_uniform_not_positional() {
    let input = vec![
        make_book_multi_author(
            "ambigs-16",
            vec![
                ("Asthma", "Albert"),
                ("Bronchitis", "Brandon"),
                ("Cold", "Charles"),
            ],
            1990,
            "Book M",
        ),
        make_book_multi_author(
            "ambigs-17",
            vec![
                ("Asthma", "Albert"),
                ("Bronchitis", "Biff"),
                ("Cold", "Curtis"),
            ],
            1990,
            "Book M",
        ),
    ];
    let mut bibliography = indexmap::IndexMap::new();
    for reference in input {
        let id = reference.id().expect("fixture id").to_string();
        bibliography.insert(id, reference);
    }
    // `build_author_date_style_with_givenname_rule` already enables
    // `disambiguate-add-names`/`disambiguate-add-givenname` with
    // et-al-min 3, et-al-use-first 1 -- the same style shape as the
    // `ByCite` companion test above, just with the rule overridden.
    let style = build_author_date_style_with_givenname_rule(GivennameRule::AllNames);
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["ambigs-16", "ambigs-17"]);

    assert_eq!(
        result,
        "Albert Asthma, Biff Bronchitis, et al., (1990); Albert Asthma, Brandon Bronchitis, et al., (1990)"
    );
}

/// Regression for a code-review finding on csl26-5753's PR: `by-cite`'s
/// per-position search must examine every position the base citation
/// already renders, not just position 0, even when `disambiguate-add-names`
/// is disabled. `disambiguate-add-names` only forbids *growing* past the
/// base visible name count -- it doesn't gate examining names the style
/// already shows unconditionally (here, no `shorten` config at all means
/// every author is always shown; the same gap applies to a style with
/// `et-al-use-first: 2+`).
fn disambiguation_by_cite_examines_already_visible_positions_without_add_names() {
    let input = vec![
        make_book_multi_author(
            "ambigs-16",
            vec![("Asthma", "Albert"), ("Bronchitis", "Brandon")],
            1990,
            "Book M",
        ),
        make_book_multi_author(
            "ambigs-17",
            vec![("Asthma", "Albert"), ("Bronchitis", "Biff")],
            1990,
            "Book M",
        ),
    ];
    let citation_items = vec![vec!["ambigs-16", "ambigs-17"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        // No `shorten` config at all (et_al_min/et_al_use_first both None)
        // -- every author is always visible, so by-cite must be able to
        // escalate position 1 even though disambiguate-add-names is off.
        // Output order follows the resolved author sort key, not citation
        // order -- see the sibling tests above for the same pattern.
        expected: "A Asthma, Biff Bronchitis, (1990); A Asthma, Brandon Bronchitis, (1990)",
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: false,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// Regression for a code-review finding on csl26-5753's PR: falling back to
/// year-suffix must retain a `by-cite` search's last-attempted name count
/// and positional escalation for a residual that's still colliding *next
/// to a sibling that did split off* -- resetting it would silently
/// under-display relative to what the sibling shows. `ambigs-a`/`ambigs-c`
/// share all four authors verbatim (genuinely unresolvable by name; forces
/// year-suffix) while `ambigs-b` differs at author 3 and splits off
/// cleanly via `disambiguate-add-names` alone.
fn disambiguation_by_cite_unresolved_fallback_retains_search_state() {
    let input = vec![
        make_book_multi_author(
            "ambigs-a",
            vec![
                ("Smith", "John"),
                ("Lee", "Alice"),
                ("Kumar", "Priya"),
                ("Zhou", "Ming"),
            ],
            2021,
            "Book A",
        ),
        make_book_multi_author(
            "ambigs-b",
            vec![
                ("Smith", "John"),
                ("Lee", "Alice"),
                ("Owen", "Priya"),
                ("Zhou", "Ming"),
            ],
            2021,
            "Book B",
        ),
        make_book_multi_author(
            "ambigs-c",
            vec![
                ("Smith", "John"),
                ("Lee", "Alice"),
                ("Kumar", "Priya"),
                ("Zhou", "Ming"),
            ],
            2021,
            "Book C",
        ),
    ];
    let citation_items = vec![vec!["ambigs-a", "ambigs-b", "ambigs-c"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        // Output order follows the citation's resolved author sort key, not
        // citation-item order (same pattern as the sibling tests above).
        // The escalated given-name initials on the still-tied a/c pair are
        // retained from the search's last attempt rather than reset to
        // bare family names -- csl26-5753's PR review fix under test here
        // -- even though they don't distinguish a from c (that's what the
        // year suffix is for).
        expected: "J Smith, A Lee, P Kumar, M Zhou, (2021a); J Smith, A Lee, P Kumar, M Zhou, (2021b); Smith, Lee, Owen, et al., (2021)",
        mode: "citation",
        disambiguate_year_suffix: true,
        disambiguate_names: true,
        disambiguate_givenname: true,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

/// Regression for csl26-5753, mirroring the official CSL test suite's
/// `disambiguate_ByCiteGivennameExpandCrossNestedNames.txt` fixture exactly
/// (same names, same style shape: et-al-min 3, et-al-use-first 1, by-cite,
/// initials baseline). Three references share the same three family names
/// throughout (Doe/Roe/Jones), so strategy 1 (name-count growth alone)
/// never splits them — position 0 (John Doe) is identical everywhere and
/// never needs to escalate past its default initials depth, but positions
/// 1 and 2 (Roe, Jones) each need escalation for *different* pairs: Roe
/// alone separates ambigs-13 (Josephine) from the other two, and Jones
/// alone separates ambigs-12 (Robert) from ambigs-14 (Richard) — so
/// resolving the group requires both positions escalating jointly, with
/// ambigs-13 needing only 2 shown names (`et al.`) while ambigs-12/14 need
/// all 3 (showing all authors suppresses `et al.` even though et-al-min is
/// met).
fn disambiguation_givenname_escalation_splits_positions_independently_per_collision() {
    let input = vec![
        make_book_multi_author(
            "ambigs-12",
            vec![("Doe", "John"), ("Roe", "Jane"), ("Jones", "Robert")],
            1990,
            "Book C",
        ),
        make_book_multi_author(
            "ambigs-13",
            vec![("Doe", "John"), ("Roe", "Josephine"), ("Jones", "Robert")],
            1990,
            "Book B",
        ),
        make_book_multi_author(
            "ambigs-14",
            vec![("Doe", "John"), ("Roe", "Jane"), ("Jones", "Richard")],
            1990,
            "Book A",
        ),
    ];
    let citation_items = vec![vec!["ambigs-12", "ambigs-13", "ambigs-14"]];
    // Output order follows the citation's resolved author sort key (family,
    // then given), not citation-item registration order — same as
    // `disambiguation_initials_are_used_when_short_form_family_names_collide`
    // above: "Jane" sorts before "Josephine", and within the tied "Jane"
    // pair, "Richard" sorts before "Robert".
    let expected = "J Doe, Jane Roe, Richard Jones, (1990); J Doe, Jane Roe, Robert Jones, (1990); J Doe, Josephine Roe, et al., (1990)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: true,
        disambiguate_givenname: true,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

/// Regression for csl26-h9jy: `all-names-with-initials` and
/// `primary-name-with-initials` must cap given-name escalation at initials —
/// unlike the default `by-cite`/`all-names`/`primary-name` rules, they may
/// never escalate to the full given name to resolve a same-initial collision
/// (only these two `GivennameRule` variants say "(initials form)" in their
/// doc comments). A same-initial collision under the cap must fall through
/// to year-suffix instead.
fn disambiguation_with_initials_rule_caps_escalation_at_initials() {
    let input = vec![
        make_book("smith-thomas", "Smith", "Thomas", 2000, "Book A"),
        make_book("smith-ted", "Smith", "Ted", 2000, "Book B"),
    ];
    let mut bibliography = indexmap::IndexMap::new();
    for reference in input {
        let id = reference.id().expect("fixture id").to_string();
        bibliography.insert(id, reference);
    }
    let processor = Processor::new(
        build_author_date_style_with_givenname_rule(GivennameRule::AllNamesWithInitials),
        bibliography,
    );

    let result = process_citation_ids(&processor, &["smith-thomas", "smith-ted"]);

    // Suffix order follows the resolved bibliography sort (§3 of
    // DISAMBIGUATION.md), not citation order — not this test's concern; the
    // behavior under test is that no given name (not even an initial) is
    // retained once the collision falls through to year-suffix.
    assert_eq!(result, "Smith, (2000b); Smith, (2000a)");
}

/// Test given name expansion with initial form (`initialize_with`).
fn disambiguation_initials_are_used_when_short_form_family_names_collide() {
    let input = vec![
        make_book("ITEM-1", "Roe", "Jane", 2000, "Book A"),
        make_book("ITEM-2", "Doe", "John", 2000, "Book B"),
        make_book("ITEM-3", "Doe", "Aloysius", 2000, "Book C"),
        make_book("ITEM-4", "Smith", "Thomas", 2000, "Book D"),
        make_book("ITEM-5", "Smith", "Ted", 2000, "Book E"),
    ];
    let citation_items = vec![
        vec!["ITEM-1"],
        vec!["ITEM-2", "ITEM-3"],
        vec!["ITEM-4", "ITEM-5"],
    ];
    // Within the Doe collision group, output order follows the author sort
    // key (family, then given), not citation-item registration order: with
    // the family name tied, "Aloysius" sorts before "John".
    //
    // Thomas/Ted Smith share an initial ("T."), so initials alone don't
    // resolve the collision — the escalation ladder continues to the full
    // given name (csl26-h9jy), matching the official CSL test suite's
    // disambiguate_ByCiteGivennameShortFormInitializeWith fixture exactly
    // ("Thomas Smith; Ted Smith").
    let expected = "Roe, (2000)
A Doe, (2000); J Doe, (2000)
Ted Smith, (2000); Thomas Smith, (2000)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: false,
        disambiguate_names: false,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// When given-name expansion cannot resolve a collision (identical given name and
/// family name, different works), the cascade must fall through to year-suffix.
fn disambiguation_year_suffix_fallback_when_givenname_expansion_fails() {
    let input = vec![
        make_book("ITEM-4", "Smith", "Ted", 2000, "Book D"),
        make_book("ITEM-5", "Smith", "Ted", 2000, "Book E"),
    ];
    let citation_items = vec![vec!["ITEM-4", "ITEM-5"]];

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        // Same author/given-name on both items → no initials injected; year suffixes
        // are still assigned because the family-name collision group is unresolved.
        expected: "Smith, (2000a), (2000b)",
        mode: "citation",
        disambiguate_year_suffix: true,
        disambiguate_names: false,
        disambiguate_givenname: true,
        et_al_min: None,
        et_al_use_first: None,
    });
}

/// Test subsequent et-al: first cite shows full list; repeat cite applies `subsequent_min/use_first`.
fn subsequent_et_al_thresholds_shorten_the_repeat_citation() {
    use citum_schema::options::{Disambiguation, Processing, ProcessingCustom, ShortenListOptions};

    let authors = vec![("Doe", "John"), ("Smith", "Jane"), ("Jones", "Alice")];

    let item = make_book_multi_author("REF-1", authors, 2020, "A Multi-Author Book");
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("REF-1".to_string(), item);

    // Style: min=3 (show all on first cite), subsequent_min=1 + subsequent_use_first=1
    let style = Style {
        info: StyleInfo {
            title: Some("Subsequent Et-Al Test".to_string()),
            id: Some("subsequent-etal-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    year_suffix: false,
                    names: false,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                }),
                ..Default::default()
            })),
            contributors: Some(citum_schema::options::ContributorConfig {
                shorten: Some(ShortenListOptions {
                    // min=4: first citation has 3 names → below threshold → show all (no et al.)
                    min: 4,
                    use_first: 3,
                    // subsequent_min=2: repeat citation has 3 names → ≥ threshold → et al.
                    subsequent_min: Some(2),
                    subsequent_use_first: Some(1),
                    ..Default::default()
                }),
                initialize_with: Some(" ".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Short),
                    citum_schema::tc_date!(
                        Issued,
                        Year,
                        wrap = citum_schema::template::WrapPunctuation::Parentheses
                    ),
                ]
                .into(),
            ),
            multi_cite_delimiter: Some("; ".into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let processor = Processor::new(style, bibliography);

    let first_cite = Citation {
        items: vec![CitationItem {
            id: "REF-1".to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };
    let repeat_cite = Citation {
        items: vec![CitationItem {
            id: "REF-1".to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let results = processor
        .process_citations(&[first_cite, repeat_cite])
        .expect("citations should render");

    // First cite: all 3 authors visible (no et al.)
    assert_eq!(
        results[0], "Doe, Smith, Jones, (2020)",
        "First citation should show all three authors without et al."
    );

    // Subsequent cite: abbreviated to 1 author + et al. (subsequent_use_first=1)
    assert_eq!(
        results[1], "Doe et al., (2020)",
        "Subsequent citation should collapse to one author + et al."
    );
}

/// Year-suffix assignment under et-al truncation: when two distinct multi-author
/// lists are both collapsed to the same et-al prefix, the collision group still
/// receives distinct suffixes. Non-citation-order suffix assignment is verified
/// (2000b before 2000a because title B sorts before title A under the sort key).
fn disambiguation_year_suffix_assigned_when_et_al_truncation_leaves_collision() {
    let input = vec![
        make_article_multi_author(
            "ITEM-1",
            vec![
                ("Baur", "Bruno"),
                ("Fröberg", "Lars"),
                ("Baur", "Anette"),
                ("Guggenheim", "Richard"),
                ("Haase", "Martin"),
            ],
            2000,
            "Ultrastructure of snail grazing damage to calcicolous lichens",
        ),
        make_article_multi_author(
            "ITEM-2",
            vec![
                ("Baur", "Bruno"),
                ("Schileyko", "Anatoly A."),
                ("Baur", "Anette"),
            ],
            2000,
            "Ecological observations on Arianta aethiops aethiops",
        ),
        make_article("ITEM-3", "Doe", "John", 2000, "Some bogus title"),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2", "ITEM-3"]];
    let expected = "Baur et al., (2000b); Baur et al., (2000a); Doe, (2000)";

    run_test_case_native_with_options(common::TestCaseOptions {
        input: &input,
        citation_items: &citation_items,
        expected,
        mode: "citation",
        disambiguate_year_suffix: true,
        disambiguate_names: false,
        disambiguate_givenname: false,
        et_al_min: Some(3),
        et_al_use_first: Some(1),
    });
}

fn citation_scoped_contributor_shorten_applies_without_component_override() {
    let item = make_book_multi_author(
        "REF-1",
        vec![
            ("Doe", "John"),
            ("Smith", "Jane"),
            ("Jones", "Alex"),
            ("Brown", "Casey"),
        ],
        2020,
        "Scoped Shorten",
    );
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("REF-1".to_string(), item);

    let style = Style {
        info: StyleInfo {
            title: Some("Scoped contributor shorten".to_string()),
            id: Some("scoped-contributor-shorten".into()),
            ..Default::default()
        },
        citation: Some(CitationSpec {
            options: Some(CitationOptions {
                contributors: Some(ContributorConfig {
                    shorten: Some(ShortenListOptions {
                        min: 4,
                        use_first: 1,
                        and_others: citum_schema::options::AndOtherOptions::Text,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .process_citation(&Citation {
            items: vec![CitationItem {
                id: "REF-1".to_string(),
                ..Default::default()
            }],
            mode: CitationMode::NonIntegral,
            ..Default::default()
        })
        .expect("citation should render");

    assert_eq!(
        rendered, "John Doe et al",
        "citation-scoped shorten should apply without component override"
    );
}

/// Two works with identical two-author list and same issued year: both receive
/// year suffixes (a, b) sorted by title. Complements the single-author cases.
fn disambiguation_identical_two_author_year_pair_receives_year_suffixes() {
    let input = vec![
        make_book_multi_author(
            "ITEM-1",
            vec![("Doe", "John"), ("Roe", "Jane")],
            2000,
            "Book A",
        ),
        make_book_multi_author(
            "ITEM-2",
            vec![("Doe", "John"), ("Roe", "Jane")],
            2000,
            "Book B",
        ),
    ];
    let citation_items = vec![vec!["ITEM-1", "ITEM-2"]];
    let expected = "Doe, Roe, (2000a), (2000b)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test year suffix with 30 entries (base-26 suffix wrapping).
fn disambiguation_suffixes_continue_past_z() {
    let mut input = Vec::new();
    let mut citation_ids = Vec::new();

    for i in 1..=30 {
        input.push(make_book(
            &format!("ITEM-{i}"),
            "Smith",
            "John",
            1986,
            "Book",
        ));
        citation_ids.push(format!("ITEM-{i}"));
    }

    let citation_items = vec![
        citation_ids
            .iter()
            .map(std::string::String::as_str)
            .collect(),
    ];
    let expected = "Smith, (1986a), (1986b), (1986c), (1986d), (1986e), (1986f), (1986g), (1986h), (1986i), (1986j), (1986k), (1986l), (1986m), (1986n), (1986o), (1986p), (1986q), (1986r), (1986s), (1986t), (1986u), (1986v), (1986w), (1986x), (1986y), (1986z), (1986aa), (1986ab), (1986ac), (1986ad)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

// --- Numeric Citation Scenarios ---

fn numeric_style_single_reference_renders_bracketed_number() {
    let style = build_numeric_style();

    let bib = citum_schema::bib_map![
        "item1" => make_book("item1", "Smith", "John", 2020, "Title A"),
        "item2" => make_book("item2", "Doe", "Jane", 2021, "Title B"),
    ];
    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "[1]"
    );
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item2"))
            .unwrap(),
        "[2]"
    );
}

// --- Citation Sorting And Grouping Scenarios ---

/// Test basic multi-item citation sorting by author.
fn author_date_sorting_orders_cluster_by_author_then_year() {
    let input = vec![
        make_book("item1", "Kuhn", "Thomas", 1962, "Title A"),
        make_book("item2", "Hawking", "Stephen", 1988, "Title B"),
    ];
    // Kuhn then Hawking in input, should be Hawking then Kuhn in output
    let citation_items = vec![vec!["item1", "item2"]];
    let expected = "Hawking, (1988); Kuhn, (1962)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test grouped citation sorting by year.
fn group_sorting_orders_cluster_by_year_within_an_author_group() {
    let input = vec![
        make_book("item1", "Kuhn", "Thomas", 1970, "Title A"),
        make_book("item2", "Kuhn", "Thomas", 1962, "Title B"),
    ];
    // 1970 then 1962 in input, should be 1962 then 1970 in output
    let citation_items = vec![vec!["item1", "item2"]];
    let expected = "Kuhn, (1962), (1970)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test multi-item citation sorting with accented surnames.
#[cfg(feature = "icu")]
fn author_date_sorting_orders_cluster_with_unicode_surnames() {
    let input = vec![
        make_book("item1", "Zimring", "Craig", 2020, "Title A"),
        make_book("item2", "Ó Tuathail", "Gearóid", 1998, "Title B"),
        make_book("item3", "Çelik", "Zeynep", 1996, "Title C"),
    ];
    let citation_items = vec![vec!["item1", "item2", "item3"]];
    let expected = "Çelik, (1996); Ó Tuathail, (1998); Zimring, (2020)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

fn sorting_empty_dates_pushes_undated_items_to_the_end() {
    // Upstream provenance: CSL fixture `date_SortEmptyDatesCitation`.
    fn make_undated_book(id: &str, title: &str) -> InputReference {
        let mut reference = make_book(id, "Smith", "Jane", 2000, title);
        if let ClassExtension::Monograph(monograph) = reference.extension_mut() {
            monograph.issued = citum_schema::reference::DateValue::new(String::new());
        }
        reference
    }

    let style = build_title_year_citation_style(vec![
        GroupSortKey {
            key: GroupSortKeyType::Issued,
            ascending: true,
            order: None,
            sort_order: None,
        },
        GroupSortKey {
            key: GroupSortKeyType::Title,
            ascending: true,
            order: None,
            sort_order: None,
        },
    ]);

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("ITEM-1".to_string(), make_undated_book("ITEM-1", "BookA"));
    bibliography.insert(
        "ITEM-2".to_string(),
        make_book("ITEM-2", "Smith", "Jane", 2000, "BookB"),
    );
    bibliography.insert("ITEM-3".to_string(), make_undated_book("ITEM-3", "BookC"));
    bibliography.insert(
        "ITEM-4".to_string(),
        make_book("ITEM-4", "Smith", "Jane", 1999, "BookD"),
    );
    bibliography.insert("ITEM-5".to_string(), make_undated_book("ITEM-5", "BookE"));

    let processor = Processor::new(style, bibliography);
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "ITEM-1".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-2".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-3".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-4".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-5".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let result = processor
        .process_citation(&citation)
        .expect("Failed to process citation with empty-date sort");
    assert_eq!(
        result,
        "BookD 1999; BookB 2000; BookA n.d.; BookC n.d.; BookE n.d."
    );
}

/// Test that an explicit `citation.sort: [Author asc]` reorders a two-item cluster
/// submitted in reverse-alphabetical order.
fn explicit_citation_sort_by_author_reorders_cluster() {
    let style = build_title_year_citation_style(vec![GroupSortKey {
        key: GroupSortKeyType::Author,
        ascending: true,
        order: None,
        sort_order: None,
    }]);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "zorr".to_string(),
        make_book("zorr", "Zorro", "Robert", 2020, "Zorro Work"),
    );
    bib.insert(
        "adam".to_string(),
        make_book("adam", "Adams", "John", 2020, "Adams Work"),
    );

    let processor = Processor::new(style, bib);
    // Submitted in Z-then-A order; citation.sort: [Author asc] must produce A-then-Z.
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "zorr".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "adam".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let result = processor
        .process_citation(&citation)
        .expect("citation with explicit Author sort should process");

    let adams_pos = result.find("Adams Work").expect("Adams Work must appear");
    let zorro_pos = result.find("Zorro Work").expect("Zorro Work must appear");
    assert!(
        adams_pos < zorro_pos,
        "explicit citation.sort [Author asc] must reorder Z-then-A submission to A-then-Z. Got: {result}"
    );
}

// --- Multilingual contributor disambiguation (§4) ---

/// Verifies that `Contributor::Multilingual` entries participate in the
/// collision-key path. Two refs share the same *original* family name and issued
/// year → both receive a year suffix.
fn disambiguation_multilingual_contributors_collide_on_original_family_name() {
    let a = common::make_multilingual_book(common::MultilingualBookParams {
        id: "ml-a",
        original_family: "김",
        original_given: "철수",
        lang: "ko",
        translit_script: "Latn",
        translit_family: "Kim",
        translit_given: "Cheolsu",
        year: 2020,
        title: "Book A",
    });
    let b = common::make_multilingual_book(common::MultilingualBookParams {
        id: "ml-b",
        original_family: "김",
        original_given: "영희",
        lang: "ko",
        translit_script: "Latn",
        translit_family: "Kim",
        translit_given: "Yeonghui",
        year: 2020,
        title: "Book B",
    });

    let input = vec![a, b];
    let citation_items = vec![vec!["ml-a", "ml-b"]];

    // The collision key uses the original Korean family name "김"; both entries
    // form one group and receive year suffixes. Suffix assignment follows the
    // bibliography sort order of the group (family, then given): "영희"
    // (ml-b) sorts before "철수" (ml-a), so ml-b receives "a" and ml-a
    // receives "b" even though ml-a is cited first.
    run_test_case_native(
        &input,
        &citation_items,
        "김, (2020b); 김, (2020a)",
        "citation",
    );
}

/// DISAMBIGUATION.md §4: when the style renders transliterated names, the
/// collision key is built from that displayed transliteration rather than from
/// the distinct original-script names. With `add_givenname` enabled, the
/// cascade resolves the transliterated-family collision by adding initials
/// instead of falling through to year suffixes.
fn disambiguation_multilingual_key_uses_transliterated_display_name() {
    let input = vec![
        common::make_multilingual_book(common::MultilingualBookParams {
            id: "ml-tanaka-a",
            original_family: "田中",
            original_given: "太郎",
            lang: "ja",
            translit_script: "ja-Latn",
            translit_family: "Tanaka",
            translit_given: "Taro",
            year: 2020,
            title: "Book A",
        }),
        common::make_multilingual_book(common::MultilingualBookParams {
            id: "ml-tanaka-b",
            original_family: "谷中",
            original_given: "次郎",
            lang: "ja",
            translit_script: "ja-Latn",
            translit_family: "Tanaka",
            translit_given: "Jiro",
            year: 2020,
            title: "Book B",
        }),
    ];

    let mut style = common::build_author_date_style(true, false, true, None, None);
    style.options = Some(Config {
        multilingual: Some(MultilingualConfig {
            name_mode: Some(MultilingualMode::Transliterated),
            preferred_transliteration: Some(vec!["ja-Latn".to_string()]),
            ..Default::default()
        }),
        ..style.options.take().unwrap_or_default()
    });

    let bibliography = input
        .into_iter()
        .filter_map(|item| item.id().map(|id| (id.to_string(), item)))
        .collect();
    let processor = Processor::new(style, bibliography);
    let result = processor
        .process_citation(&Citation {
            items: vec![
                CitationItem {
                    id: "ml-tanaka-a".to_string(),
                    ..Default::default()
                },
                CitationItem {
                    id: "ml-tanaka-b".to_string(),
                    ..Default::default()
                },
            ],
            mode: CitationMode::NonIntegral,
            ..Default::default()
        })
        .expect("citation should render");

    assert_eq!(result, "T Tanaka, (2020); J Tanaka, (2020)");
}

// --- APA §8.15 Reprint Disambiguation ---

/// APA §8.15: three reprints — two originally 1926, one originally 1927 — all
/// published 1967. Year-suffix must follow the *published* year only, giving
/// `(1926/1967a) (1926/1967b) (1927/1967c)`.
///
/// Verifies that `compute_disamb_suffix` is gated to the `issued` component and
/// never applied to `original-published`.
fn apa_reprint_issued_year_only_suffix() {
    use citum_schema::reference::InputReference;

    // Three CSL-JSON reprints: same author, all issued 1967, original years differ.
    // Titles chosen so both 1926 originals sort before the 1927 one (A < B < Z).
    let make_reprint = |id: &str, orig_year: i32, title: &str| -> InputReference {
        let json = serde_json::json!({
            "id": id,
            "type": "book",
            "title": title,
            "author": [{ "family": "Freud", "given": "Sigmund" }],
            "issued": { "date-parts": [[1967]] },
            "original-date": { "date-parts": [[orig_year]] }
        });
        let legacy: csl_legacy::csl_json::Reference =
            serde_json::from_value(json).expect("reprint json parse");
        legacy.into()
    };

    let refs = vec![
        make_reprint("reprint-a", 1926, "Abriss der Psychoanalyse"),
        make_reprint("reprint-b", 1926, "Begriffsbestimmung"),
        make_reprint("reprint-c", 1927, "Zukunft einer Illusion"),
    ];

    // Style built from YAML — renders (original-published-year/issued-year) for each cite.
    // The slash group delimiter and parentheses wrap produce e.g. `(1926/1967a)`.
    let style: citum_schema::Style = serde_yaml::from_str(
        r"
info:
  title: APA Reprint Suffix Test
  id: test-apa-reprint
options:
  processing:
    disambiguate:
      year-suffix: true
      names: false
      add-givenname: false
citation:
  collapse: same-author
  multi-cite-delimiter: ' '
  template:
    - group:
      - date: original-published
        form: year
      - date: issued
        form: year
      delimiter: /
      wrap:
        punctuation: parentheses
",
    )
    .expect("reprint style parse");

    let mut bibliography = indexmap::IndexMap::new();
    for item in &refs {
        if let Some(id) = item.id() {
            bibliography.insert(id.to_string(), item.clone());
        }
    }

    let processor = Processor::new(style, bibliography);
    let citation = citum_schema::citation::Citation {
        items: refs
            .iter()
            .filter_map(|r| r.id())
            .map(|id| citum_schema::citation::CitationItem {
                id: id.to_string(),
                ..Default::default()
            })
            .collect(),
        mode: citum_schema::citation::CitationMode::NonIntegral,
        ..Default::default()
    };

    let result = processor
        .process_citation(&citation)
        .expect("Failed to process APA reprint citation");

    // Three reprints all get a year-suffix: (1926/1967a), (1926/1967b), (1927/1967c).
    // The suffix attaches to the issued year (1967), not the original-published year
    // (1926/1927). All three entries are in one collision group (keyed on issued year
    // only), so none is left without a suffix — unlike citeproc-js which would omit
    // the suffix on the 1927 entry because its rendered string differs.
    assert_eq!(result, "Freud, (1926/1967a), (1926/1967b), (1927/1967c)");
}

// --- Note Style Position Scenarios ---

fn chicago_notes_style_with_ibid() -> Style {
    let path = common::test_style_path("styles/embedded/chicago-notes-18th.yaml");
    let yaml = std::fs::read_to_string(&path).expect("Failed to read chicago-notes.yaml");
    let existing_ibid = "  ibid:\n    note-start-text-case: capitalize-first\n    suffix: \"\"\n    delimiter: \", \"\n    template:\n    - message: term.ibid\n    - variable: locator\n      suffix: .\n";
    let yaml = yaml.replacen(existing_ibid, "", 1);
    let marker = "citation:\n  delimiter: \"\"\n";
    assert!(
        yaml.contains(marker),
        "Chicago notes fixture marker is missing"
    );
    let yaml = yaml.replacen(
        marker,
        "citation:\n  delimiter: \"\"\n  ibid:\n    note-start-text-case: capitalize-first\n    suffix: \"\"\n    delimiter: \", \"\n    template:\n    - message: term.ibid\n    - variable: locator\n      suffix: .\n",
        1,
    );
    serde_yaml::from_str(&yaml).expect("Failed to parse Chicago notes ibid fixture")
}

fn chicago_notes_immediate_repeat_renders_compact_ibid() {
    let style = chicago_notes_style_with_ibid();

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    // First citation (full form)
    let first_citation = citum_schema::Citation {
        items: vec![citum_schema::citation::CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::First),
        ..Default::default()
    };

    let first_result = processor
        .process_citation(&first_citation)
        .expect("Failed to process first citation");
    assert_eq!(first_result, "John Smith, _A Great Book_ (1995).");

    // Second citation with Ibid position (should render "Ibid.")
    let ibid_citation = citum_schema::Citation {
        items: vec![citum_schema::citation::CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Ibid),
        ..Default::default()
    };

    let ibid_result = processor
        .process_citation(&ibid_citation)
        .expect("Failed to process ibid citation");
    assert_eq!(ibid_result, "Ibid.");
}

fn chicago_notes_prefixed_ibid_remains_mid_sentence() {
    let style = chicago_notes_style_with_ibid();

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    let ibid_citation = citum_schema::Citation {
        items: vec![citum_schema::citation::CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Ibid),
        prefix: Some("See".into()),
        ..Default::default()
    };

    let ibid_result = processor
        .process_citation(&ibid_citation)
        .expect("Failed to process prefixed ibid citation");
    assert!(
        ibid_result.contains("See ibid."),
        "prefixed ibid should remain mid-sentence lowercase: {ibid_result}"
    );
    assert!(
        !ibid_result.contains("See Ibid."),
        "prefixed ibid should not be capitalized as sentence-initial: {ibid_result}"
    );
}

fn chicago_notes_immediate_repeat_with_locator_keeps_the_locator() {
    let style = chicago_notes_style_with_ibid();

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    // Citation with IbidWithLocator position and locator
    let ibid_with_locator = citum_schema::Citation {
        items: vec![citum_schema::citation::CitationItem {
            id: "smith1995".to_string(),
            locator: Some(citum_schema::citation::CitationLocator::single(
                citum_schema::citation::LocatorType::Page,
                "45",
            )),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::IbidWithLocator),
        ..Default::default()
    };

    let result = processor
        .process_citation(&ibid_with_locator)
        .expect("Failed to process ibid with locator citation");
    assert!(
        result.contains("Ibid., 45"),
        "IbidWithLocator should contain lexical ibid: {result}"
    );
}

fn chicago_notes_non_immediate_repeat_uses_the_subsequent_short_form() {
    let path = common::test_style_path("styles/embedded/chicago-notes-18th.yaml");

    let yaml = std::fs::read_to_string(&path).expect("Failed to read chicago-notes.yaml");
    let style: citum_schema::Style =
        serde_yaml::from_str(&yaml).expect("Failed to parse chicago-notes.yaml");

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    // Subsequent citation (after another source in between)
    let subsequent_citation = citum_schema::Citation {
        items: vec![citum_schema::citation::CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Subsequent),
        ..Default::default()
    };

    let result = processor
        .process_citation(&subsequent_citation)
        .expect("Failed to process subsequent citation");
    assert_eq!(result, "Smith, _A Great Book_.");
}

fn chicago_notes_reprint_full_note_renders_original_publisher_metadata() {
    let path = common::test_style_path("styles/embedded/chicago-notes-18th.yaml");

    let yaml = std::fs::read_to_string(&path).expect("Failed to read chicago-notes.yaml");
    let style: citum_schema::Style =
        serde_yaml::from_str(&yaml).expect("Failed to parse chicago-notes.yaml");

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "reprint1994",
        "type": "book",
        "title": "Orientalism",
        "author": [{ "family": "Said", "given": "Edward W." }],
        "issued": { "date-parts": [[1994]] },
        "publisher": "Vintage Books",
        "publisher-place": "New York",
        "original-date": { "date-parts": [[1901]] },
        "original-publisher": "Old Press",
        "original-publisher-place": "Boston"
    }))
    .expect("failed to parse legacy reprint fixture");
    let id = legacy.id.clone();
    let bib = indexmap::IndexMap::from([(id.clone(), legacy.into())]);
    let processor = Processor::new(style, bib);

    let first_citation = citum_schema::Citation {
        items: vec![citum_schema::citation::CitationItem {
            id,
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::First),
        ..Default::default()
    };

    let rendered = processor
        .process_citation(&first_citation)
        .expect("Failed to process reprint citation");
    assert_eq!(
        rendered,
        "Edward W. Said, _Orientalism_ (1901) Old Press, Boston (Vintage Books, 1994)."
    );
}

fn note_styles_without_ibid_overrides_fall_back_to_subsequent() {
    let style = Style {
        info: StyleInfo {
            title: Some("Note Subsequent Fallback".to_string()),
            id: Some("note-subsequent-fallback".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Note),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            subsequent: Some(Box::new(CitationSpec {
                template: Some(vec![citum_schema::tc_contributor!(Author, Short)].into()),
                ..Default::default()
            })),
            ..Default::default()
        }),
        ..Default::default()
    };

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];
    let processor = Processor::new(style, bib);

    let subsequent = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Subsequent),
        ..Default::default()
    };
    let ibid = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Ibid),
        ..Default::default()
    };
    let ibid_with_locator = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            locator: Some(citum_schema::citation::CitationLocator::single(
                citum_schema::citation::LocatorType::Page,
                "45",
            )),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::IbidWithLocator),
        ..Default::default()
    };

    let subsequent_rendered = processor
        .process_citation(&subsequent)
        .expect("subsequent should render");
    let ibid_rendered = processor
        .process_citation(&ibid)
        .expect("ibid should render");
    let ibid_with_locator_rendered = processor
        .process_citation(&ibid_with_locator)
        .expect("ibid-with-locator should render");

    assert_eq!(
        ibid_rendered, subsequent_rendered,
        "Ibid should fall back to subsequent form when `citation.ibid` is absent"
    );
    assert_eq!(
        ibid_with_locator_rendered, subsequent_rendered,
        "IbidWithLocator should fall back to subsequent form when `citation.ibid` is absent"
    );
    assert!(
        !ibid_rendered.contains("Ibid"),
        "fallback should not force lexical ibid output"
    );
}

fn oscola_position_overrides_control_ibid_and_subsequent_forms() {
    use std::path::PathBuf;

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("styles/oscola.yaml");

    let yaml = std::fs::read_to_string(&path).expect("Failed to read oscola.yaml");
    let style: citum_schema::Style =
        serde_yaml::from_str(&yaml).expect("Failed to parse oscola.yaml");

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    let first = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::First),
        ..Default::default()
    };
    let subsequent = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Subsequent),
        ..Default::default()
    };
    let ibid = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Ibid),
        ..Default::default()
    };
    let ibid_with_locator = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            locator: Some(citum_schema::citation::CitationLocator::single(
                citum_schema::citation::LocatorType::Page,
                "45",
            )),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::IbidWithLocator),
        ..Default::default()
    };

    let first_rendered = processor
        .process_citation(&first)
        .expect("first cite should render");
    let subsequent_rendered = processor
        .process_citation(&subsequent)
        .expect("subsequent cite should render");
    let ibid_rendered = processor
        .process_citation(&ibid)
        .expect("ibid cite should render");
    let ibid_with_locator_rendered = processor
        .process_citation(&ibid_with_locator)
        .expect("ibid-with-locator cite should render");

    // oscola.yaml's citation.delimiter is a literal space; the title (with
    // its own scavenged `", "` prefix externalized as the author->title
    // join) and the year, wrapped in parentheses with no prefix of its
    // own, must still take that shared delimiter (csl26-475u) — confirmed
    // against citeproc-js via `node scripts/oracle.js styles-legacy/oscola.csl`.
    assert_eq!(
        first_rendered,
        "John Smith, \u{201C}_A Great Book_\u{201D} (1995)."
    );
    // oscola.yaml declares no `default-locale`, so it resolves to the
    // caller's locale — en-US here, whose `punctuation-in-quote: true`
    // grammar option now supplies the style default (csl26-8e75) and moves
    // the trailing period inside the closing quote, matching citeproc-js.
    assert_eq!(
        subsequent_rendered,
        "Smith, \u{201C}_A Great Book_.\u{201D}"
    );
    assert_eq!(ibid_rendered, "ibid.");
    assert_eq!(ibid_with_locator_rendered, "ibid p45.");
}

fn oscola_without_ibid_reuses_the_subsequent_form_for_immediate_repeats() {
    use std::path::PathBuf;

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("styles/oscola-no-ibid.yaml");

    let yaml = std::fs::read_to_string(&path).expect("Failed to read oscola-no-ibid.yaml");
    let style: citum_schema::Style =
        serde_yaml::from_str(&yaml).expect("Failed to parse oscola-no-ibid.yaml");

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    let subsequent = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Subsequent),
        ..Default::default()
    };
    let ibid = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Ibid),
        ..Default::default()
    };

    let subsequent_rendered = processor
        .process_citation(&subsequent)
        .expect("subsequent cite should render");
    let ibid_rendered = processor
        .process_citation(&ibid)
        .expect("ibid cite should render");

    assert_eq!(
        ibid_rendered, subsequent_rendered,
        "OSCOLA no-ibid should fall back to the subsequent form for immediate repeats"
    );
    assert!(
        !ibid_rendered.to_lowercase().contains("ibid"),
        "OSCOLA no-ibid should never render lexical ibid: {ibid_rendered}"
    );
}

fn thomson_reuters_subsequent_short_form_keeps_the_locator() {
    use std::path::PathBuf;

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("styles/thomson-reuters-legal-tax-and-accounting-australia.yaml");

    let yaml = std::fs::read_to_string(&path)
        .expect("Failed to read thomson-reuters-legal-tax-and-accounting-australia.yaml");
    let style: citum_schema::Style = serde_yaml::from_str(&yaml)
        .expect("Failed to parse thomson-reuters-legal-tax-and-accounting-australia.yaml");

    let bib = citum_schema::bib_map![
        "smith1995" => make_book("smith1995", "Smith", "John", 1995, "A Great Book"),
    ];

    let processor = Processor::new(style, bib);

    let first = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::First),
        ..Default::default()
    };
    let subsequent = Citation {
        items: vec![CitationItem {
            id: "smith1995".to_string(),
            locator: Some(citum_schema::citation::CitationLocator::single(
                citum_schema::citation::LocatorType::Page,
                "23",
            )),
            ..Default::default()
        }],
        position: Some(citum_schema::citation::Position::Subsequent),
        ..Default::default()
    };

    let first_rendered = processor
        .process_citation(&first)
        .expect("first cite should render");
    let subsequent_rendered = processor
        .process_citation(&subsequent)
        .expect("subsequent cite should render");

    // thomson-reuters-legal-tax-and-accounting-australia.yaml's
    // citation.delimiter is a literal space; the title and the
    // parenthesised year must still take it (csl26-475u).
    assert_eq!(
        first_rendered,
        "Smith, \u{201C}A Great Book\u{201D} (1995)."
    );
    assert_eq!(
        subsequent_rendered,
        "Smith, \u{201C}_A Great Book_\u{201D} at 23."
    );
}

// --- Grouped Citation Rendering Tests ---

fn grouped_author_date_mode_groups_items_by_author() {
    let input = vec![
        make_book("item1", "Smith", "John", 2020, "Book A"),
        make_book("item1b", "Smith", "John", 2021, "Book B"),
        make_book("item2", "Jones", "Jane", 2020, "Book C"),
    ];
    let citation_items = vec![vec!["item1", "item1b", "item2"]];
    // Grouped author-date clusters by author then year within group
    let expected = "Jones, (2020); Smith, (2020), (2021)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

fn grouped_numeric_mode_preserves_item_order() {
    let input = vec![
        make_book("item1", "Smith", "John", 2020, "Book A"),
        make_book("item2", "Jones", "Jane", 2021, "Book B"),
        make_book("item3", "Brown", "Bob", 2022, "Book C"),
    ];
    let citation_items = vec![vec!["item1", "item2", "item3"]];
    // Numeric in default author-date style still sorts by author
    let expected = "Brown, (2022); Jones, (2021); Smith, (2020)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

fn grouped_integral_mode_displays_first_author_only() {
    let input = vec![
        make_book("item1", "Smith", "John", 2020, "Book A"),
        make_book("item1b", "Smith", "John", 2021, "Book B"),
    ];
    let citation_items = vec![vec!["item1", "item1b"]];
    // Author-date style groups same-author works by year
    let expected = "Smith, (2020), (2021)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// given a citation template whose leading component is an author `group`
/// gated by `render-when: field-present: author`, when rendering a grouped
/// citation, then the author slot renders normally for a reference that
/// satisfies the condition, and renders empty — never falling back to
/// `reference.author()`, since the condition already established there is
/// none to promote — for a reference that fails it.
#[rstest]
#[case::render_when_satisfied("with-author")]
#[case::render_when_suppressed("without-author")]
fn grouped_citation_honors_render_when_on_leading_author_group(#[case] ref_id: &str) {
    let style_yaml = r#"
info:
  title: Render-When Grouping Test
  id: render-when-grouping-test
options:
  processing: author-date-full
citation:
  template:
    - group:
      - contributor: author
        form: short
      render-when:
        field-present: author
      delimiter: " "
    - date: issued
      form: year
      prefix: " "
"#;
    let style: Style = Style::from_yaml_str(style_yaml).expect("style should parse");

    let with_author = InputReference::Monograph(Box::new(Monograph {
        id: Some("with-author".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Book A".to_string())),
        author: Some(citum_schema::reference::Contributor::StructuredName(
            citum_schema::reference::StructuredName {
                family: citum_schema::reference::MultilingualString::Simple("Smith".to_string()),
                given: citum_schema::reference::MultilingualString::Simple("John".to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            },
        )),
        issued: DateValue::new("2020".to_string()),
        ..Default::default()
    }));

    let without_author = InputReference::Monograph(Box::new(Monograph {
        id: Some("without-author".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Book B".to_string())),
        author: None,
        issued: DateValue::new("2020".to_string()),
        ..Default::default()
    }));

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("with-author".to_string(), with_author);
    bibliography.insert("without-author".to_string(), without_author);

    let processor = Processor::new(style, bibliography);
    let mut run = processor.begin_run();
    let citation = Citation {
        items: vec![CitationItem {
            id: ref_id.to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let rendered = processor
        .process_citation_with_format::<citum_engine::render::plain::PlainText>(&citation, &mut run)
        .expect("citation should render");

    let expected = match ref_id {
        "with-author" => "Smith 2020",
        "without-author" => "2020",
        other => panic!("unexpected case id: {other}"),
    };
    assert_eq!(
        rendered.trim(),
        expected,
        "rendered citation for '{ref_id}' did not match; yaml:\n{style_yaml}"
    );
}

/// csl26-475u. given an MLA-shaped citation template — a leading
/// author+title group (title carrying its own `", "` prefix, scavenged to
/// join author->title externally) followed by a sibling `variable: locator`
/// with no prefix of its own — when rendering a citation with a locator
/// (reproduces even for a single, non-collapsed item), then a delimiter
/// should still separate the title from the locator.
///
/// Was broken because `filter_author_from_template` reported the scavenge
/// via a `strip_item_delimiter` flag that zeroed the *whole* item's shared
/// delimiter in `render_group_item_parts_with_format`, and
/// `citation_to_string_with_format` threads that one delimiter value
/// uniformly across every join in the item's template — so zeroing it to
/// avoid double-delimiting the externalized author->title join also
/// silently dropped the title->locator join.
///
/// Fixed by inverting the mechanism: `RenderedComponent` now records
/// `supplies_own_leading_separator` per component, and
/// `citation_to_string_with_format` skips the *shared* delimiter only for
/// parts that report it, deciding this after empty parts are already
/// dropped from `parts`. The shared delimiter itself is no longer zeroed
/// anywhere. Two earlier attempts instead *injected* a delimiter as a
/// static per-component `prefix`, decided from template shape alone before
/// rendering; both regressed other embedded styles (double-delimiting a
/// component that already declared its own `prefix`, and orphan commas
/// when a preceding component rendered empty and vanished from `parts`
/// before the static injection could know).
///
/// `supplies_own_leading_separator` classifies the component's *structured*
/// `DelimiterPunctuation` prefix value, not its realized rendered string —
/// classifying the rendered glyph breaks for CJK, where a semantic comma
/// realizes to `，` rather than `,` (caught in PR review; see
/// `prefix_supplies_own_leading_separator` in `render/component.rs`, and
/// `cjk_realized_semantic_prefix_is_recognized_as_self_separating` below
/// for the regression test). See the bean for the full investigation and
/// oracle/PR context.
fn item_template_delimits_a_bare_sibling_after_a_scavenged_leading_affix() {
    let style_yaml = r#"
info:
  title: MLA Locator Delimiter Repro
  id: mla-locator-delimiter-repro
options:
  processing: author-date-full
citation:
  template:
  - group:
    - contributor: author
      form: family-only
    - title: primary
      wrap:
        punctuation: quotes
      prefix: ", "
  - variable: locator
  wrap:
    punctuation: parentheses
  multi-cite-delimiter: "; "
"#;
    let style: Style = Style::from_yaml_str(style_yaml).expect("style should parse");

    let item31 = InputReference::Monograph(Box::new(Monograph {
        id: Some("ITEM-31".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(
            "Methods for Robust Climate Attribution".to_string(),
        )),
        author: Some(citum_schema::reference::Contributor::StructuredName(
            citum_schema::reference::StructuredName {
                family: citum_schema::reference::MultilingualString::Simple("Garcia".to_string()),
                given: citum_schema::reference::MultilingualString::Simple("Maria".to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            },
        )),
        issued: DateValue::new("2020".to_string()),
        ..Default::default()
    }));
    let item32 = InputReference::Monograph(Box::new(Monograph {
        id: Some("ITEM-32".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(
            "Methods for Probabilistic Climate Attribution".to_string(),
        )),
        author: Some(citum_schema::reference::Contributor::StructuredName(
            citum_schema::reference::StructuredName {
                family: citum_schema::reference::MultilingualString::Simple("Garcia".to_string()),
                given: citum_schema::reference::MultilingualString::Simple("Maria".to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            },
        )),
        issued: DateValue::new("2021".to_string()),
        ..Default::default()
    }));

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("ITEM-31".to_string(), item31);
    bibliography.insert("ITEM-32".to_string(), item32);

    let processor = Processor::new(style, bibliography);
    let mut run = processor.begin_run();
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "ITEM-31".to_string(),
                locator: Some(citum_schema::citation::CitationLocator::single(
                    citum_schema::citation::LocatorType::Page,
                    "100",
                )),
                ..Default::default()
            },
            CitationItem {
                id: "ITEM-32".to_string(),
                ..Default::default()
            },
        ],
        mode: CitationMode::NonIntegral,
        ..Default::default()
    };

    let rendered = processor
        .process_citation_with_format::<citum_engine::render::plain::PlainText>(&citation, &mut run)
        .expect("citation should render");

    assert_eq!(
        rendered,
        "(Garcia, \u{201c}Methods for Robust Climate Attribution,\u{201d} p. 100; \
         Garcia, \u{201c}Methods for Probabilistic Climate Attribution\u{201d})"
    );
}

/// csl26-475u follow-up (PR #1255 review). `supplies_own_leading_separator`
/// must classify the component's *structured* `DelimiterPunctuation` prefix,
/// not its realized glyph -- a semantic `{ mark: comma }` prefix realizes to
/// full-width `，` under CJK script, which an ASCII-only character test
/// would misclassify as "not a separator," reintroducing double-delimiting
/// for CJK output specifically. Given a citation item whose style opts into
/// CJK realization (`options.multilingual.realization-default: cjk`) and a
/// template component with its own `prefix: { mark: comma }`, the shared
/// item delimiter must be skipped ahead of that component: one full-width
/// comma at the join, not a doubled join (confirmed against the pre-fix
/// predicate: it renders `"测试, ，2020"` -- the shared delimiter and the
/// component's own separator both firing -- not the `"测试，2020"` this
/// asserts).
///
/// Uses `suppress_author: true` to isolate the item's own internal join
/// (title -> date, where `supplies_own_leading_separator` lives) from the
/// separate author->title external join, which has its own delimiter
/// resolution untouched by this fix.
fn cjk_realized_semantic_prefix_is_recognized_as_self_separating() {
    let style_yaml = r#"
info:
  title: CJK Separator Classification Repro
  id: cjk-separator-classification-repro
options:
  processing: author-date-full
  multilingual:
    realization-default: cjk
citation:
  template:
  - contributor: author
    form: family-only
  - title: primary
  - date: issued
    form: year
    prefix: { mark: comma }
  wrap:
    punctuation: parentheses
"#;
    let style: Style = Style::from_yaml_str(style_yaml).expect("style should parse");

    let item = InputReference::Monograph(Box::new(Monograph {
        id: Some("cjk-item".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("测试".to_string())),
        author: Some(citum_schema::reference::Contributor::StructuredName(
            citum_schema::reference::StructuredName {
                family: citum_schema::reference::MultilingualString::Simple("Wang".to_string()),
                given: citum_schema::reference::MultilingualString::Simple("Fang".to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            },
        )),
        language: Some("zh".into()),
        issued: DateValue::new("2020".to_string()),
        ..Default::default()
    }));

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("cjk-item".to_string(), item);

    let processor = Processor::new(style, bibliography);
    let mut run = processor.begin_run();
    let citation = Citation {
        items: vec![CitationItem {
            id: "cjk-item".to_string(),
            ..Default::default()
        }],
        mode: CitationMode::NonIntegral,
        // Isolate the item's own internal join (title -> date) from the
        // separate author->title external join, which has its own
        // unrelated delimiter resolution not touched by this fix.
        suppress_author: true,
        ..Default::default()
    };

    let rendered = processor
        .process_citation_with_format::<citum_engine::render::plain::PlainText>(&citation, &mut run)
        .expect("citation should render");

    assert_eq!(rendered, "\u{ff08}测试，2020\u{ff09}");
}

fn citation_html_injects_sparse_template_indices_when_enabled() {
    let style_yaml = r#"
info:
  title: Indexed Citation Preview
  id: indexed-citation-preview
citation:
  template:
    - title: primary
    - variable: doi
      prefix: ". "
    - variable: url
      prefix: " "
"#;
    let style: Style = serde_yaml::from_str(style_yaml).expect("style should parse");

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-1",
        "type": "book",
        "title": "Preview Book",
        "URL": "https://example.com/preview-book"
    }))
    .expect("legacy fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-1".to_string(), legacy.into());

    let processor = Processor::new(style, bib).with_inject_ast_indices(true);
    let citation = Citation {
        items: vec![CitationItem {
            id: "ITEM-1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let mut run = processor.begin_run();
    let rendered = processor
        .process_citation_with_format::<Html>(&citation, &mut run)
        .expect("citation should render");

    assert!(
        rendered.contains(r#"class="citum-title" data-index="0""#),
        "title wrapper should carry the first template index: {rendered}"
    );
    assert!(
        rendered.contains(r#"class="citum-url" data-index="2""#),
        "url wrapper should carry the sparse third template index: {rendered}"
    );
    assert!(
        !rendered.contains(r#"data-index="1""#),
        "missing DOI output should preserve sparse template indices: {rendered}"
    );
}

mod integral_name_memory {
    use super::announce_behavior;

    #[test]
    fn explicit_integral_name_state_overrides_processor_memory() {
        announce_behavior(
            "An explicit integral-name state should force full-form on first cite and short-form on repeat.",
        );
        super::integral_name_state_overrides_processor_memory();
    }

    #[test]
    fn absent_memory_block_does_not_rewrite_subsequent_name_state() {
        announce_behavior(
            "A style with no integral-name-memory block should leave Subsequent-state citations rendered in the integral template's natural form.",
        );
        super::absent_memory_block_does_not_rewrite_subsequent_name_state();
    }
}

mod disambiguation {
    use super::announce_behavior;

    #[test]
    fn same_author_same_year_titles_follow_title_order() {
        announce_behavior(
            "Two same-author, same-year works should receive year suffixes in title order.",
        );
        super::disambiguation_same_author_same_year_titles_follow_title_order();
    }

    #[test]
    fn two_level_author_collisions_get_distinct_suffixes() {
        announce_behavior(
            "Colliding author lists at multiple truncation levels should still end up with distinct year suffixes.",
        );
        super::disambiguation_two_level_author_collisions_get_distinct_suffixes();
    }

    #[test]
    fn same_year_articles_increment_suffixes() {
        announce_behavior(
            "Same-year articles should increment year suffixes a, b, c in citation order.",
        );
        super::disambiguation_same_year_articles_increment_suffixes();
    }

    #[test]
    fn no_spurious_givenname_expansion_when_years_differ() {
        announce_behavior(
            "When same-family-name refs already differ by year, add_givenname must not introduce spurious given-name expansion.",
        );
        super::disambiguation_no_spurious_givenname_expansion_when_years_differ();
    }

    #[test]
    fn givenname_expansion_resolves_same_year_family_name_collision() {
        announce_behavior(
            "Two same-year authors with the same family name but different given names must have initials injected for the ambiguous pair; unrelated authors stay unexpanded.",
        );
        super::disambiguation_givenname_expansion_resolves_same_year_family_name_collision();
    }

    #[test]
    fn year_suffix_follows_literal_title_order() {
        announce_behavior(
            "Year-suffix letters must follow the bibliography title sort key (literal text, no article stripping), not a raw lowercased comparison.",
        );
        super::disambiguation_year_suffix_follows_literal_title_order();
    }

    #[test]
    fn givenname_expansion_preferred_over_year_suffix() {
        announce_behavior(
            "Same-surname, different-given-name authors in one year must disambiguate by initials, not a spurious year suffix, even when year-suffix is also enabled.",
        );
        super::disambiguation_givenname_expansion_preferred_over_year_suffix();
    }

    #[test]
    fn year_suffix_off_emits_no_letter() {
        announce_behavior(
            "With year-suffix disabled (author-page styles like MLA), an unresolved same-author/same-year collision emits no suffix letter.",
        );
        super::disambiguation_year_suffix_off_emits_no_letter();
    }

    #[test]
    fn primary_name_initials_expand_globally_across_citations() {
        announce_behavior(
            "Under primary-name-with-initials (APA), same-surname authors cited in separate citations both gain first-author initials globally, with no year suffix.",
        );
        super::disambiguation_primary_name_initials_expand_globally_across_citations();
    }

    #[test]
    fn by_cite_givenname_expansion_is_document_wide() {
        announce_behavior(
            "By-cite given-name disambiguation compares against every reference in the document, not just the current citation — identical to all-names until a real per-position expansion ceiling exists (csl26-8nrt, csl26-5753).",
        );
        super::disambiguation_by_cite_givenname_expansion_is_document_wide();
    }

    #[test]
    fn all_names_givenname_expansion_remains_global() {
        announce_behavior(
            "All-names given-name disambiguation should keep document-wide expansion for affected name groups.",
        );
        super::disambiguation_all_names_givenname_expansion_remains_global();
    }

    #[test]
    fn by_cite_solo_cite_still_expands() {
        announce_behavior(
            "A solo by-cite citation of a reference that collides with another reference elsewhere in the document must still expand — collision detection is document-wide, never scoped to one citation (csl26-8nrt).",
        );
        super::disambiguation_by_cite_solo_cite_still_expands();
    }

    #[test]
    fn by_cite_mixed_groups_all_expand() {
        announce_behavior(
            "When a by-cite citation mixes two directly-colliding references with a third reference that collides with something cited elsewhere, all three still expand — collision membership never depends on what else is in the same citation (csl26-8nrt).",
        );
        super::disambiguation_by_cite_mixed_groups_all_expand();
    }

    #[test]
    fn all_names_co_citation_expands_both_colliders() {
        announce_behavior(
            "Citing two globally-colliding references together under all-names must expand both.",
        );
        super::disambiguation_all_names_co_citation();
    }

    #[test]
    fn primary_name_givenname_expansion_expands_first_author_only() {
        announce_behavior(
            "primary-name rule must expand the first author's given name; when that does not resolve the collision, year-suffix must be applied.",
        );
        super::disambiguation_primary_name_givenname_expansion();
    }

    #[test]
    fn primary_name_givenname_expansion_resolves_when_primary_authors_differ() {
        announce_behavior(
            "primary-name rule must resolve the collision via given-name expansion alone when the primary authors' given names differ, with no year-suffix applied.",
        );
        super::disambiguation_primary_name_givenname_expansion_resolves_distinct_primary_authors();
    }

    #[test]
    fn et_al_conflicts_expand_names_when_that_resolves_them() {
        announce_behavior(
            "When et al. creates a collision, name expansion should win if it can resolve the ambiguity.",
        );
        super::disambiguation_et_al_conflicts_expand_names_when_that_resolves_them();
    }

    #[test]
    fn et_al_conflicts_fall_back_to_year_suffixes() {
        announce_behavior(
            "When et al. collisions cannot be resolved by names alone, year suffixes should disambiguate the cites.",
        );
        super::disambiguation_et_al_conflicts_fall_back_to_year_suffixes();
    }

    #[test]
    fn givenname_escalation_is_minimal_per_position() {
        announce_behavior(
            "A same-initial collision escalates to the full given name only at the author position that needs it, matching citeproc-js's own per-position minimality (csl26-5753) — positions that stay identical between colliders keep their default initials depth.",
        );
        super::disambiguation_givenname_escalation_is_minimal_per_position();
    }

    #[test]
    fn by_cite_examines_already_visible_positions_without_add_names() {
        announce_behavior(
            "by-cite's per-position search examines every name position the base citation already shows, even when disambiguate-add-names is disabled — that flag only forbids growing past the base visible name count, not examining it.",
        );
        super::disambiguation_by_cite_examines_already_visible_positions_without_add_names();
    }

    #[test]
    fn by_cite_unresolved_fallback_retains_search_state() {
        announce_behavior(
            "When a by-cite collision remains unresolved next to a sibling that did split off via disambiguate-add-names, the year-suffix fallback retains the search's last-attempted name count and escalation instead of resetting it to the style's default.",
        );
        super::disambiguation_by_cite_unresolved_fallback_retains_search_state();
    }

    #[test]
    fn all_names_givenname_escalation_is_uniform_not_positional() {
        announce_behavior(
            "Unlike by-cite, all-names has no per-position escalation ceiling — on the same input it escalates every shown position uniformly, including one that gains nothing from it.",
        );
        super::disambiguation_all_names_givenname_escalation_is_uniform_not_positional();
    }

    #[test]
    fn givenname_escalation_splits_positions_independently_per_collision() {
        announce_behavior(
            "When several author positions could each resolve a different pairwise collision within a by-cite group, escalation targets each position independently — including varying how many names each colliding reference shows — rather than applying one uniform name count or depth to the whole group (csl26-5753).",
        );
        super::disambiguation_givenname_escalation_splits_positions_independently_per_collision();
    }

    #[test]
    fn with_initials_rule_caps_escalation_at_initials() {
        announce_behavior(
            "all-names-with-initials and primary-name-with-initials must never escalate a same-initial collision to the full given name — they fall back to year-suffix instead.",
        );
        super::disambiguation_with_initials_rule_caps_escalation_at_initials();
    }

    #[test]
    fn initials_are_used_when_short_form_family_names_collide() {
        announce_behavior(
            "Short-form family-name collisions should expand to initials when that is the configured fallback, escalating to the full given name when initials alone still collide.",
        );
        super::disambiguation_initials_are_used_when_short_form_family_names_collide();
    }

    #[test]
    fn year_suffix_fallback_when_givenname_expansion_fails() {
        announce_behavior(
            "When given-name expansion cannot resolve a collision (same given name and family name), year-suffix must be applied as the next cascade step.",
        );
        super::disambiguation_year_suffix_fallback_when_givenname_expansion_fails();
    }

    #[test]
    fn year_suffix_assigned_when_et_al_truncation_leaves_collision() {
        announce_behavior(
            "When et-al truncation collapses distinct author lists to the same prefix, the resulting collision group should still receive distinct year suffixes in title sort order.",
        );
        super::disambiguation_year_suffix_assigned_when_et_al_truncation_leaves_collision();
    }

    #[test]
    fn identical_two_author_year_pair_receives_year_suffixes() {
        announce_behavior(
            "Two works sharing the same two-author list and issued year should each receive a year suffix.",
        );
        super::disambiguation_identical_two_author_year_pair_receives_year_suffixes();
    }

    #[test]
    fn suffixes_continue_past_z() {
        announce_behavior(
            "Year suffix generation should continue past z without resetting or truncating.",
        );
        super::disambiguation_suffixes_continue_past_z();
    }

    #[test]
    fn multilingual_contributors_collide_on_original_family_name() {
        announce_behavior(
            "Multilingual contributors with matching original family names and the same issued year must form a collision group and receive year suffixes.",
        );
        super::disambiguation_multilingual_contributors_collide_on_original_family_name();
    }

    #[test]
    fn multilingual_key_uses_transliterated_display_name() {
        announce_behavior(
            "When the style renders transliterated names, distinct original-script names with the same transliteration must form a collision group.",
        );
        super::disambiguation_multilingual_key_uses_transliterated_display_name();
    }

    #[test]
    fn apa_reprint_year_suffix_attaches_to_issued_year_only() {
        announce_behavior(
            "APA §8.15 reprints with different original-dates should receive year-suffix on the \
             issued year only, producing (1926/1967a) (1926/1967b) (1927/1967c).",
        );
        super::apa_reprint_issued_year_only_suffix();
    }
}

mod contributor_scoping {
    use super::announce_behavior;

    #[test]
    fn citation_scoped_shorten_applies_without_component_override() {
        announce_behavior(
            "Citation-scoped contributor shortening should apply even when the template contributor has no explicit shorten block.",
        );
        super::citation_scoped_contributor_shorten_applies_without_component_override();
    }

    #[test]
    fn subsequent_et_al_thresholds_shorten_the_repeat_citation() {
        announce_behavior(
            "Subsequent-citation et al. thresholds should shorten a repeat citation more aggressively than the first cite.",
        );
        super::subsequent_et_al_thresholds_shorten_the_repeat_citation();
    }
}

mod numeric_style {
    use super::announce_behavior;

    #[test]
    fn single_reference_renders_bracketed_number() {
        announce_behavior(
            "A numeric citation style should render a single reference number in brackets.",
        );
        super::numeric_style_single_reference_renders_bracketed_number();
    }
}

mod sorting_and_grouping {
    use super::announce_behavior;

    #[test]
    fn author_date_sorting_orders_cluster_by_author_then_year() {
        announce_behavior(
            "Author-date citation clusters should sort entries by author and then by year.",
        );
        super::author_date_sorting_orders_cluster_by_author_then_year();
    }

    #[test]
    fn group_sorting_orders_cluster_by_year_within_an_author_group() {
        announce_behavior(
            "Grouped citation sorting should keep works together by author and then sort years within that group.",
        );
        super::group_sorting_orders_cluster_by_year_within_an_author_group();
    }

    #[test]
    #[cfg(feature = "icu")]
    fn author_date_sorting_orders_cluster_with_unicode_surnames() {
        announce_behavior(
            "Author-date citation clusters should sort accented surnames with Unicode-aware collation.",
        );
        super::author_date_sorting_orders_cluster_with_unicode_surnames();
    }

    #[test]
    fn empty_dates_push_undated_items_to_the_end() {
        announce_behavior(
            "Undated items should sort after dated items rather than interleaving with them.",
        );
        super::sorting_empty_dates_pushes_undated_items_to_the_end();
    }

    #[test]
    fn explicit_citation_sort_by_author_reorders_cluster() {
        announce_behavior(
            "An explicit citation.sort [Author asc] must reorder a cluster submitted in reverse-alpha order.",
        );
        super::explicit_citation_sort_by_author_reorders_cluster();
    }
}

mod note_style_positions {
    use super::announce_behavior;

    #[test]
    fn chicago_notes_immediate_repeat_renders_compact_ibid() {
        announce_behavior("An immediate Chicago note repeat should collapse to a compact ibid.");
        super::chicago_notes_immediate_repeat_renders_compact_ibid();
    }

    #[test]
    fn chicago_notes_immediate_repeat_with_locator_keeps_the_locator() {
        announce_behavior(
            "An immediate Chicago note repeat with a locator should keep the locator in the ibid form.",
        );
        super::chicago_notes_immediate_repeat_with_locator_keeps_the_locator();
    }

    #[test]
    fn chicago_notes_prefixed_ibid_remains_mid_sentence() {
        announce_behavior(
            "A prefixed Chicago ibid should stay lowercase because the note marker is no longer sentence-initial.",
        );
        super::chicago_notes_prefixed_ibid_remains_mid_sentence();
    }

    #[test]
    fn chicago_notes_non_immediate_repeat_uses_the_subsequent_short_form() {
        announce_behavior(
            "A non-immediate Chicago note repeat should use the shortened subsequent-note form instead of ibid.",
        );
        super::chicago_notes_non_immediate_repeat_uses_the_subsequent_short_form();
    }

    #[test]
    fn chicago_notes_reprint_full_note_renders_original_publisher_metadata() {
        announce_behavior(
            "A full Chicago note for a reprint should include original publisher metadata before the current publication details.",
        );
        super::chicago_notes_reprint_full_note_renders_original_publisher_metadata();
    }

    #[test]
    fn note_styles_without_ibid_overrides_fall_back_to_subsequent() {
        announce_behavior(
            "Note styles without ibid overrides should fall back to their normal subsequent-note form.",
        );
        super::note_styles_without_ibid_overrides_fall_back_to_subsequent();
    }

    #[test]
    fn oscola_position_overrides_control_ibid_and_subsequent_forms() {
        announce_behavior(
            "OSCOLA note-position overrides should decide when to emit ibid versus a subsequent short form.",
        );
        super::oscola_position_overrides_control_ibid_and_subsequent_forms();
    }

    #[test]
    fn oscola_without_ibid_reuses_the_subsequent_form_for_immediate_repeats() {
        announce_behavior(
            "When OSCOLA disables ibid, even immediate repeats should reuse the subsequent short form.",
        );
        super::oscola_without_ibid_reuses_the_subsequent_form_for_immediate_repeats();
    }

    #[test]
    fn thomson_reuters_subsequent_short_form_keeps_the_locator() {
        announce_behavior(
            "Thomson Reuters repeated notes should shorten the cite while preserving the locator.",
        );
        super::thomson_reuters_subsequent_short_form_keeps_the_locator();
    }

    // --- Regression Tests for Grouped Citation Rendering ---

    #[test]
    fn grouped_author_date_mode_groups_items_by_author() {
        announce_behavior(
            "Author-date grouped rendering should collapse multiple items with same author.",
        );
        super::grouped_author_date_mode_groups_items_by_author();
    }

    #[test]
    fn grouped_numeric_mode_preserves_item_order() {
        announce_behavior(
            "Numeric grouped rendering should maintain citation order without author collapse.",
        );
        super::grouped_numeric_mode_preserves_item_order();
    }

    #[test]
    fn grouped_integral_mode_displays_first_author_only() {
        announce_behavior(
            "Integral grouped rendering should display only the first item's author.",
        );
        super::grouped_integral_mode_displays_first_author_only();
    }

    #[test]
    fn item_template_delimits_a_bare_sibling_after_a_scavenged_leading_affix() {
        announce_behavior(
            "A citation with a locator must still separate a title-based item's own title \
             from its locator with a delimiter, even when that title's own leading affix \
             was scavenged to join the author to it externally.",
        );
        super::item_template_delimits_a_bare_sibling_after_a_scavenged_leading_affix();
    }

    #[test]
    fn cjk_realized_semantic_prefix_is_recognized_as_self_separating() {
        announce_behavior(
            "A component's own semantic-mark prefix must be recognized as a \
             separator by its structured mark, not its CJK-realized glyph, or the \
             shared item delimiter doubles it up under CJK realization.",
        );
        super::cjk_realized_semantic_prefix_is_recognized_as_self_separating();
    }

    #[test]
    fn disambiguate_only_title_suppressed_when_first_ref_note_number_is_present() {
        announce_behavior(
            "In a note style, a `disambiguate-only` title should be suppressed in a subsequent \
             citation when a first-reference-note-number is available — the note number \
             already identifies the work.",
        );
        super::disambiguate_only_title_suppressed_in_note_cross_ref_position();
    }

    #[test]
    fn disambiguate_only_title_kept_when_template_lacks_first_ref_note_number() {
        announce_behavior(
            "In a note style, a `disambiguate-only` title must be retained in a subsequent \
             citation when the template does not render a first-reference-note-number — \
             suppressing it would reintroduce ambiguity with no replacement identifier.",
        );
        super::disambiguate_only_title_kept_when_template_lacks_note_number();
    }
}

mod annotated_html_preview {
    use super::announce_behavior;

    #[test]
    fn citation_indices_stay_sparse_when_template_components_do_not_render() {
        announce_behavior(
            "Annotated citation HTML should preserve the original template indices when intermediate components do not render.",
        );
        super::citation_html_injects_sparse_template_indices_when_enabled();
    }
}

#[test]
fn test_personal_communication_citation_rendering_is_style_driven() {
    let bib_vec = serde_yaml::from_str::<Vec<InputReference>>(
        r#"
- id: oglethorpe-1733
  class: monograph
  type: personal-communication
  contributors:
  - role: author
    contributor: {given: James, family: Oglethorpe}
  - role: recipient
    contributor: {name: "the Trustees"}
  issued: '1733-01-13'
"#,
    )
    .unwrap();

    let mut bib = indexmap::IndexMap::new();
    for item in bib_vec {
        bib.insert(item.id().unwrap().to_string(), item);
    }

    // Mock APA 7th non-integral: (J. Oglethorpe, personal communication, January 13, 1733)
    let apa_style = Style {
        info: StyleInfo {
            title: Some("APA Personal Communication".to_string()),
            ..Default::default()
        },
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    citum_schema::template::TemplateComponent::Contributor(
                        citum_schema::template::TemplateContributor {
                            contributor: citum_schema::template::ContributorRole::Author.into(),
                            form: citum_schema::template::ContributorForm::Long,
                            name_order: Some(citum_schema::template::NameOrder::GivenFirst),
                            rendering: citum_schema::template::Rendering {
                                name_form: Some(NameForm::Initials),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ),
                    citum_schema::tc_term!(PersonalCommunication),
                    citum_schema::tc_date!(Issued, Full),
                ]
                .into(),
            ),
            delimiter: Some(", ".into()),
            wrap: Some(citum_schema::template::WrapPunctuation::Parentheses.into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let processor = Processor::new(apa_style, bib);
    let citation = Citation {
        items: vec![CitationItem {
            id: "oglethorpe-1733".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    let output = processor.process_citation(&citation).unwrap();
    assert_eq!(
        output,
        "(J. Oglethorpe, personal communication, January 13, 1733)"
    );
}

/// Two same-author, same-year works (disambiguated by title in first cite).
/// A subsequent citation carries a `first-reference-note-number` — the note
/// number supersedes the disambiguating short title, which must be suppressed.
fn disambiguate_only_title_suppressed_in_note_cross_ref_position() {
    // Note style: subsequent form shows Author + disambiguate_only title.
    // When a first-reference-note-number is available, the title is suppressed.
    let style: citum_schema::Style = serde_yaml::from_str(
        r"
info:
  title: Note Disambig-Only Test
  id: test-note-disambig-only
options:
  processing: note
citation:
  template:
    - contributor: author
      form: short
    - title: primary
      form: short
      disambiguate-only: true
  delimiter: ', '
  subsequent:
    template:
      - contributor: author
        form: short
      - title: primary
        form: short
        disambiguate-only: true
      - number: first-reference-note-number
        prefix: 'see n. '
    delimiter: ', '
",
    )
    .expect("style parse");

    // Two books: same author, same year → disambiguation assigns short titles
    let bib = citum_schema::bib_map![
        "rome" => make_book("rome", "Smith", "John", 2020, "A History of Rome"),
        "greece" => make_book("greece", "Smith", "John", 2020, "A History of Greece"),
    ];

    let processor = Processor::new(style, bib);

    let citations = vec![
        // Note 1: first cite of Rome
        citum_schema::citation::Citation {
            items: vec![citum_schema::citation::CitationItem {
                id: "rome".to_string(),
                ..Default::default()
            }],
            note_number: Some(1),
            ..Default::default()
        },
        // Note 2: first cite of Greece
        citum_schema::citation::Citation {
            items: vec![citum_schema::citation::CitationItem {
                id: "greece".to_string(),
                ..Default::default()
            }],
            note_number: Some(2),
            ..Default::default()
        },
        // Note 3: subsequent cite of Rome — should NOT show short title
        citum_schema::citation::Citation {
            items: vec![citum_schema::citation::CitationItem {
                id: "rome".to_string(),
                ..Default::default()
            }],
            note_number: Some(3),
            position: Some(citum_schema::citation::Position::Subsequent),
            ..Default::default()
        },
    ];

    let results = processor
        .process_citations(&citations)
        .expect("citations should render");

    assert_eq!(results.len(), 3, "expected three rendered citations");
    // First cites carry the disambiguating title; the subsequent cite drops it
    // in favour of the note-number identifier ("see n. 1").
    assert_eq!(
        results,
        vec![
            "Smith, A History of Rome".to_string(),
            "Smith, A History of Greece".to_string(),
            "Smith, see n. 1".to_string(),
        ]
    );
}

/// Regression guard for the over-suppression fix: when the subsequent template
/// does *not* render `first-reference-note-number`, suppressing the
/// `disambiguate-only` title would silently reintroduce ambiguity. The title
/// must therefore be kept. Mirrors the suppression fixture above but with the
/// `number: first-reference-note-number` component removed.
fn disambiguate_only_title_kept_when_template_lacks_note_number() {
    let style: citum_schema::Style = serde_yaml::from_str(
        r"
info:
  title: Note Disambig-Only No-Number Test
  id: test-note-disambig-only-no-number
options:
  processing: note
citation:
  template:
    - contributor: author
      form: short
    - title: primary
      form: short
      disambiguate-only: true
  delimiter: ', '
  subsequent:
    template:
      - contributor: author
        form: short
      - title: primary
        form: short
        disambiguate-only: true
    delimiter: ', '
",
    )
    .expect("style parse");

    let bib = citum_schema::bib_map![
        "rome" => make_book("rome", "Smith", "John", 2020, "A History of Rome"),
        "greece" => make_book("greece", "Smith", "John", 2020, "A History of Greece"),
    ];

    let processor = Processor::new(style, bib);

    let citations = vec![
        citum_schema::citation::Citation {
            items: vec![citum_schema::citation::CitationItem {
                id: "rome".to_string(),
                ..Default::default()
            }],
            note_number: Some(1),
            ..Default::default()
        },
        citum_schema::citation::Citation {
            items: vec![citum_schema::citation::CitationItem {
                id: "greece".to_string(),
                ..Default::default()
            }],
            note_number: Some(2),
            ..Default::default()
        },
        // Note 3: subsequent cite of Rome. Without a note-number identifier the
        // disambiguating title MUST be retained.
        citum_schema::citation::Citation {
            items: vec![citum_schema::citation::CitationItem {
                id: "rome".to_string(),
                ..Default::default()
            }],
            note_number: Some(3),
            position: Some(citum_schema::citation::Position::Subsequent),
            ..Default::default()
        },
    ];

    let results = processor
        .process_citations(&citations)
        .expect("citations should render");

    assert_eq!(
        results,
        vec![
            "Smith, A History of Rome".to_string(),
            "Smith, A History of Greece".to_string(),
            "Smith, A History of Rome".to_string(),
        ]
    );
}

// --- sentence_start capitalization ---

#[test]
fn test_sentence_start_capitalizes_lowercase_prefix() {
    // Given: an integral citation with a lowercase prefix and sentence_start true
    let style = build_author_date_style(false, false, false, None, None);
    let bib = citum_schema::bib_map![
        "smith2020" => make_book("smith2020", "Smith", "John", 2020, "A Book"),
    ];
    let processor = Processor::new(style, bib);

    let citation = Citation {
        mode: CitationMode::Integral,
        prefix: Some("see also".into()),
        sentence_start: true,
        items: vec![CitationItem {
            id: "smith2020".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    // When: rendered
    let result = processor.process_citation(&citation).expect("render");

    // Then: leading "s" of "see also" is capitalized
    assert!(
        result.starts_with("See also"),
        "expected 'See also …' but got: {result}"
    );
}

#[test]
fn test_sentence_start_noop_on_capitalized_author() {
    // Given: an integral citation with no prefix (author leads) and sentence_start true
    let style = build_author_date_style(false, false, false, None, None);
    let bib = citum_schema::bib_map![
        "smith2020" => make_book("smith2020", "Smith", "John", 2020, "A Book"),
    ];
    let processor = Processor::new(style, bib);

    let citation = Citation {
        mode: CitationMode::Integral,
        sentence_start: true,
        items: vec![CitationItem {
            id: "smith2020".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let without_flag = Citation {
        sentence_start: false,
        ..citation.clone()
    };

    // When: rendered with and without the flag
    let with_result = processor
        .process_citation(&citation)
        .expect("render with flag");
    let without_result = processor
        .process_citation(&without_flag)
        .expect("render without flag");

    // Then: output is identical (author "Smith" already starts with a capital)
    assert_eq!(
        with_result, without_result,
        "sentence_start should be a no-op when the cluster already starts with a capital"
    );
}

#[test]
fn test_sentence_start_false_leaves_output_unchanged() {
    // Given: a citation with a lowercase prefix and sentence_start false (default)
    let style = build_author_date_style(false, false, false, None, None);
    let bib = citum_schema::bib_map![
        "smith2020" => make_book("smith2020", "Smith", "John", 2020, "A Book"),
    ];
    let processor = Processor::new(style, bib);

    let citation = Citation {
        mode: CitationMode::Integral,
        prefix: Some("see also".into()),
        sentence_start: false,
        items: vec![CitationItem {
            id: "smith2020".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    // When: rendered
    let result = processor.process_citation(&citation).expect("render");

    // Then: "see also" remains lowercase
    assert!(
        result.starts_with("see also"),
        "expected 'see also …' (lowercase) but got: {result}"
    );
}

#[test]
fn role_label_defaults_bundle_never_fires_in_citation_context() {
    // Role labels are a bibliography-only convention in every examined style
    // guide (APA, MLA, Chicago, Vancouver/NLM) -- see div-012 in
    // docs/adjudication/DIVERGENCE_REGISTER.md and
    // docs/specs/ROLE_LABEL_DEFAULTS.md. Even with the APA defaults bundle
    // declared, a Long-form editor in a citation template renders bare.
    use citum_schema::reference::{Contributor, ContributorList, StructuredName};

    let style = Style {
        info: StyleInfo {
            title: Some("Role Label Defaults Citation Test".to_string()),
            id: Some("role-label-defaults-citation-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            contributors: Some(ContributorConfig {
                role: Some(citum_schema::options::contributors::RoleOptions {
                    defaults: Some(citum_schema::options::contributors::RoleLabelDefaults::Apa),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            options: Some(CitationOptions {
                label_mode: Some(CitationLabelMode::None),
                ..Default::default()
            }),
            template: Some(vec![citum_schema::tc_contributor!(Editor, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("item1".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Title".to_string())),
            editor: Some(Contributor::ContributorList(ContributorList(vec![
                Contributor::StructuredName(StructuredName {
                    given: "John".into(),
                    family: "Smith".into(),
                    ..Default::default()
                }),
            ]))),
            issued: DateValue::new("2020".to_string()),
            ..Default::default()
        })),
    );
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, "John Smith");
}

#[test]
fn leading_non_author_contributor_renders_once_in_grouped_citation() {
    // The grouped-citation fallback renders an "author part" from the first
    // grouping component of the template -- any contributor role, not just
    // author. The item-part template previously stripped only author
    // components, so a citation template leading with e.g. a translator
    // rendered its names twice ("Tr Translatorsen, Tr Translatorsen").
    // filter_author_from_template now strips the same leading contributor
    // the author part rendered. Bean csl26-7g1i.
    use citum_schema::reference::{Contributor, ContributorList, StructuredName};

    let style = Style {
        info: StyleInfo {
            title: Some("Leading Translator Citation Test".to_string()),
            id: Some("leading-translator-citation-test".into()),
            ..Default::default()
        },
        citation: Some(CitationSpec {
            template: Some(vec![citum_schema::tc_contributor!(Translator, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "item1".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("item1".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Title".to_string())),
            editor: Some(Contributor::ContributorList(ContributorList(vec![
                Contributor::StructuredName(StructuredName {
                    given: "Ed".into(),
                    family: "Editorsen".into(),
                    ..Default::default()
                }),
            ]))),
            translator: Some(Contributor::ContributorList(ContributorList(vec![
                Contributor::StructuredName(StructuredName {
                    given: "Tr".into(),
                    family: "Translatorsen".into(),
                    ..Default::default()
                }),
            ]))),
            issued: DateValue::new("2020".to_string()),
            ..Default::default()
        })),
    );
    let processor = Processor::new(style, bibliography);

    let result = process_citation_ids(&processor, &["item1"]);

    assert_eq!(result, "Tr Translatorsen");
}

/// **Given** an author-date style whose resolved bibliography sort has no
/// keys that distinguish a same-author/same-year collision group — either no
/// keys at all (the `citation-number` preset, which is how
/// `gb-t-7714-2025-author-date` reached this bug via its inherited numeric
/// base) or keys that tie for every member (an explicit `author`+`issued`
/// template, since both references below share both),
/// **When** each reference is cited,
/// **Then** the year-suffix letter follows the bibliography's registration
/// (registry) order — matching how `ReferenceSorter` renders that same tie,
/// since disambiguation hints are computed once for the whole processor and
/// shared by citation and bibliography rendering — not an independently
/// computed title order. csl26-m8la.
#[rstest]
#[case::citation_number_preset_empty_template(GroupSortEntry::Preset(
    citum_schema::SortPreset::CitationNumber
))]
#[case::explicit_author_issued_template(GroupSortEntry::Explicit(GroupSort {
    template: vec![
        GroupSortKey {
            key: GroupSortKeyType::Author,
            ascending: true,
            order: None,
            sort_order: None,
        },
        GroupSortKey {
            key: GroupSortKeyType::Issued,
            ascending: true,
            order: None,
            sort_order: None,
        },
    ],
}))]
fn given_a_tied_bibliography_sort_when_citing_then_year_suffixes_follow_registration_order(
    #[case] bibliography_sort: GroupSortEntry,
) {
    use citum_schema::reference::{Contributor, StructuredName};

    let style = Style {
        info: StyleInfo {
            title: Some("Year Suffix Registration Order Test".to_string()),
            id: Some("year-suffix-registration-order-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(citum_schema::options::Disambiguation {
                    names: false,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: true,
                }),
                ..Default::default()
            })),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Short),
                    citum_schema::tc_date!(
                        Issued,
                        Year,
                        wrap = citum_schema::template::WrapPunctuation::Parentheses
                    ),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        bibliography: Some(citum_schema::BibliographySpec {
            sort: Some(bibliography_sort),
            template: Some(vec![citum_schema::tc_title!(Primary)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    // Registered in an order that disagrees with title-alphabetical (which
    // would put "Alpha Report" first) — proving suffixes follow registration,
    // not a fresh title computation.
    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert(
        "gamma".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("gamma".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Gamma Report".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                given: "Same".into(),
                family: "Smith".into(),
                ..Default::default()
            })),
            issued: DateValue::new("2020".to_string()),
            ..Default::default()
        })),
    );
    bibliography.insert(
        "alpha".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("alpha".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Alpha Report".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                given: "Same".into(),
                family: "Smith".into(),
                ..Default::default()
            })),
            issued: DateValue::new("2020".to_string()),
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bibliography);

    // Both references share author and year, so their rendered citations
    // differ only by the year-suffix letter the disambiguator assigns. The
    // suffix must follow registration order ("gamma" then "alpha"): "a" then
    // "b" — not title order, which would put "Alpha Report" ("alpha") first.
    let gamma_rendered = processor
        .process_citation(&Citation::simple("gamma"))
        .expect("gamma citation should render");
    let alpha_rendered = processor
        .process_citation(&Citation::simple("alpha"))
        .expect("alpha citation should render");

    assert_eq!(gamma_rendered, "Smith, (2020a)");
    assert_eq!(alpha_rendered, "Smith, (2020b)");
}
