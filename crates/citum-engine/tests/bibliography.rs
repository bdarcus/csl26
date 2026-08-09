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

mod common;
use citum_schema::reference::ClassExtension;
use common::*;

use citum_engine::{
    Citation, CitationItem, Processor, render::html::Html, render::latex::Latex,
    render::plain::PlainText, render::typst::Typst,
};
use citum_schema::{
    BibliographySpec, CitationSpec, Style, StyleInfo,
    options::{
        AndOptions, ArticleJournalBibliographyConfig, ArticleJournalNoPageFallback,
        BibliographyLabelMode, BibliographyLabelWrap, BibliographyOptions,
        BibliographyPartitionHeading, BibliographyPartitionKind, BibliographyPartitionMode,
        BibliographySortPartitioning, Config, ContributorConfig, DelimiterPrecedesLast,
        DemoteNonDroppingParticle, DisplayAsSort, LinkAnchor, LinkTarget, LinksConfig,
        MultilingualConfig, MultilingualMode, Processing, ProcessingCustom, Sort, SortKey,
        SortSpec, SortingConfig, SortingMultilingualMode,
    },
    reference::{
        Contributor, ContributorList, DateValue, InputReference, Monograph, MonographType,
        Numbering, NumberingType, Serial, SerialComponent, SerialComponentType, SerialType,
        StructuredName, Title, WorkRelation,
        contributor::MultilingualName,
        types::{ArchiveInfo, EprintInfo, MultilingualComplex, MultilingualString},
    },
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, DelimiterPunctuation,
        NumberVariable, Rendering, SimpleVariable, TemplateComponent, TemplateConditionField,
        TemplateContributor, TemplateDate, TemplateGroup, TemplateGroupCondition, TemplateNumber,
        TemplateTitle, TemplateVariable, TitleForm, TitleType,
    },
};
use indexmap::IndexMap;
use rstest::rstest;
use std::collections::HashMap;
use std::fs;
use url::Url;

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
        bibliography: Some(BibliographySpec {
            options: Some(citum_schema::BibliographyOptions {
                label_mode: Some(citum_schema::options::BibliographyLabelMode::Numeric),
                label_separator: Some(". ".to_string()),
                ..Default::default()
            }),
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_date!(Issued, Year, prefix = " (", suffix = ")"),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn chicago_author_date_uses_comma_before_conference_pages_without_volume_or_issue() {
    let style = load_style("styles/embedded/chicago-author-date-18th.yaml");
    let bibliography = citum_io::load_bibliography(
        &project_root()
            .join("tests/fixtures/style-regressions/chicago-author-date-conference-pages.json"),
    )
    .expect("Chicago conference punctuation fixture should load");

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "chicago-conference-no-volume-pages".to_string(),
    ]);

    assert_eq!(
        rendered.trim(),
        "Mikolov, Tomas. 2013. “Distributed Representations of Words and Phrases.” _Proceedings of NIPS 2013_, 3111–19."
    );
}

fn build_sorted_style(sort: Vec<SortSpec>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Sorted Test".to_string()),
            id: Some("sort-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                sort: Some(citum_schema::options::SortEntry::Explicit(Sort {
                    template: sort,
                    shorten_names: false,
                    render_substitutions: false,
                })),
                ..Default::default()
            })),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_date!(Issued, Year, prefix = " "),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// `Sort::group_sort()` has no group-sort equivalent for `CitationNumber`
/// (citation-number sorting is registry order by definition), so a style
/// whose config-level sort names it should still resolve and render, but
/// the style-load-time compat scan should flag the key as unsupported
/// rather than silently keeping registry order unremarked.
#[test]
fn citation_number_bibliography_sort_key_produces_a_compat_warning() {
    let style = build_sorted_style(vec![SortSpec {
        key: SortKey::CitationNumber,
        ascending: true,
    }]);
    let processor = Processor::new(style, IndexMap::new());

    let warnings = citum_engine::api::unknown_enum_warnings(&processor);

    assert!(
        warnings
            .iter()
            .any(|w| w.code == "citation_number_sort_not_supported"),
        "expected a warning for the citation-number bibliography sort key, got: {warnings:?}"
    );
}

fn build_title_year_sorted_style(sort: Vec<SortSpec>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Title Year Sorted Test".to_string()),
            id: Some("title-year-sort-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                sort: Some(citum_schema::options::SortEntry::Explicit(Sort {
                    template: sort,
                    shorten_names: false,
                    render_substitutions: false,
                })),
                ..Default::default()
            })),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_title!(Primary, prefix = ". "),
                    citum_schema::tc_date!(Issued, Year, prefix = " "),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_partition_style(partition_yaml: &str, groups_yaml: &str) -> Style {
    fn indent_block(block: &str) -> String {
        block
            .trim_matches('\n')
            .lines()
            .map(|line| {
                if line.is_empty() {
                    String::new()
                } else {
                    format!("  {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    let partition_yaml = indent_block(partition_yaml);
    let groups_yaml = indent_block(groups_yaml);
    let yaml = format!(
        r#"
info:
  id: partition-test
  title: Partition Test
  default-locale: en-US
bibliography:
{partition_yaml}
{groups_yaml}
  sort:
    template:
      - key: title
  template:
    - title: primary
"#
    );

    serde_yaml::from_str(&yaml).expect("partition style should parse")
}

fn partition_reference(id: &str, title: &str, language: Option<&str>) -> InputReference {
    typed_partition_reference(id, "book", title, language)
}

fn typed_partition_reference(
    id: &str,
    ref_type: &str,
    title: &str,
    language: Option<&str>,
) -> InputReference {
    let mut fixture = serde_json::json!({
        "id": id,
        "type": ref_type,
        "title": title
    });
    if let Some(language) = language {
        fixture["language"] = serde_json::json!(language);
    }

    let legacy: csl_legacy::csl_json::Reference =
        serde_json::from_value(fixture).expect("partition fixture should parse");
    legacy.into()
}

fn script_partition_bibliography() -> IndexMap<String, InputReference> {
    IndexMap::from([
        (
            "latin".to_string(),
            partition_reference("latin", "Alpha", Some("en")),
        ),
        (
            "cyrl".to_string(),
            partition_reference("cyrl", "Бета", Some("ru")),
        ),
        (
            "hani".to_string(),
            partition_reference("hani", "東京", Some("ja")),
        ),
    ])
}

fn language_partition_bibliography() -> IndexMap<String, InputReference> {
    IndexMap::from([
        (
            "en".to_string(),
            partition_reference("en", "Alpha", Some("en")),
        ),
        (
            "ru".to_string(),
            partition_reference("ru", "Beta", Some("ru")),
        ),
        (
            "ja".to_string(),
            partition_reference("ja", "Gamma", Some("ja")),
        ),
    ])
}

fn romanized_partition_reference(
    id: &str,
    family: &str,
    sort_as: Option<&str>,
    title: &str,
) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        author: Some(Contributor::ContributorList(ContributorList(vec![
            Contributor::Multilingual(MultilingualName {
                original: StructuredName {
                    family: family.into(),
                    given: "Test".into(),
                    ..Default::default()
                },
                lang: Some("ru".into()),
                sort_as: sort_as.map(str::to_string),
                transliterations: HashMap::new(),
                translations: HashMap::new(),
            }),
        ]))),
        issued: DateValue::new("1900".to_string()),
        ..Default::default()
    }))
}

fn romanized_partition_bibliography() -> IndexMap<String, InputReference> {
    IndexMap::from([
        (
            "latin".to_string(),
            romanized_partition_reference("latin", "Smith", None, "Latin title"),
        ),
        (
            "pushkin".to_string(),
            romanized_partition_reference("pushkin", "Пушкин", Some("Zulu"), "Pushkin title"),
        ),
        (
            "tolstoy".to_string(),
            romanized_partition_reference("tolstoy", "Толстой", Some("Aardvark"), "Tolstoy title"),
        ),
    ])
}

fn language_partition_reference(
    id: &str,
    family: &str,
    given: &str,
    title: &str,
    language: &str,
) -> InputReference {
    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": id,
        "type": "book",
        "title": title,
        "language": language,
        "author": [{
            "family": family,
            "given": given
        }]
    }))
    .expect("language partition fixture should parse");
    legacy.into()
}

fn language_partition_substitute_style() -> Style {
    let mut style = make_style_with_substitute(Some("———".to_string()));
    style.info.title = Some("Partition Substitute Test".to_string());
    style.info.id = Some("partition-substitute-test".into());
    style
        .bibliography
        .as_mut()
        .expect("partition substitute style should have bibliography")
        .options
        .get_or_insert_with(BibliographyOptions::default)
        .sort_partitioning = Some(BibliographySortPartitioning {
        by: BibliographyPartitionKind::Language,
        mode: BibliographyPartitionMode::Sections,
        order: vec!["ru".to_string(), "en".to_string()],
        headings: HashMap::from([
            (
                "ru".to_string(),
                BibliographyPartitionHeading::Literal {
                    literal: "Russian".to_string(),
                },
            ),
            (
                "en".to_string(),
                BibliographyPartitionHeading::Literal {
                    literal: "English".to_string(),
                },
            ),
        ]),
        unknown_fields: std::collections::BTreeMap::new(),
    });
    style
        .bibliography
        .as_mut()
        .expect("partition substitute style should have bibliography")
        .template = Some(
        vec![
            citum_schema::tc_contributor!(Author, Long),
            citum_schema::tc_title!(Primary, prefix = ". "),
        ]
        .into(),
    );
    style
}

#[test]
fn given_no_partitioning_when_rendering_mixed_script_bibliography_then_single_collator_order_is_preserved()
 {
    announce_behavior(
        "Mixed-script bibliographies preserve the existing single-collator order unless partitioning is enabled.",
    );
    let style = build_partition_style("", "");
    let processor = Processor::new(style, script_partition_bibliography());

    assert_eq!(processor.render_bibliography(), "Alpha\n\nБета\n\n東京");
}

#[test]
#[cfg(feature = "icu")]
fn given_script_sort_only_partitioning_when_rendering_flat_bibliography_then_partition_order_precedes_title_sort()
 {
    announce_behavior(
        "Script partitioning in sort-only mode renders one flat bibliography ordered by configured script blocks.",
    );
    let style = build_partition_style(
        r#"
options:
  sort-partitioning:
    by: script
    mode: sort-only
    order: [Cyrl, Latn, Hani]
"#,
        "",
    );
    let processor = Processor::new(style, script_partition_bibliography());

    assert_eq!(processor.render_bibliography(), "Бета\n\nAlpha\n\n東京");
}

#[test]
#[cfg(feature = "icu")]
fn given_per_script_sorting_shorthand_when_no_partitioning_then_script_sort_only_is_applied() {
    announce_behavior(
        "The multilingual per-script sorting shorthand expands to script sort-only partitioning when no explicit partitioning block exists.",
    );
    let shorthand_style = build_partition_style(
        r#"
options:
  sorting:
    multilingual: per-script
"#,
        "",
    );
    let explicit_style = build_partition_style(
        r#"
options:
  sort-partitioning:
    by: script
    mode: sort-only
"#,
        "",
    );

    let shorthand =
        Processor::new(shorthand_style, script_partition_bibliography()).render_bibliography();
    let explicit =
        Processor::new(explicit_style, script_partition_bibliography()).render_bibliography();

    assert_eq!(shorthand, explicit);
    assert_eq!(shorthand, "Бета\n\n東京\n\nAlpha");
}

#[test]
#[cfg(feature = "icu")]
fn given_per_script_shorthand_and_explicit_partitioning_then_explicit_partitioning_wins() {
    announce_behavior(
        "An explicit sort-partitioning block is authoritative over the per-script sorting shorthand.",
    );
    let style = build_partition_style(
        r#"
options:
  sorting:
    multilingual: per-script
  sort-partitioning:
    by: script
    mode: sort-only
    order: [Latn, Cyrl, Hani]
"#,
        "",
    );
    let processor = Processor::new(style, script_partition_bibliography());

    assert_eq!(processor.render_bibliography(), "Alpha\n\nБета\n\n東京");
}

#[test]
#[cfg(feature = "icu")]
fn given_romanized_sorting_with_explicit_script_partitioning_then_romanized_order_applies_within_partitions()
 {
    announce_behavior(
        "Romanized sorting composes with explicit script partitioning by applying hidden sort keys inside each script partition.",
    );
    let style = Style {
        info: StyleInfo {
            title: Some("Romanized Partition Test".to_string()),
            id: Some("romanized-partition-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            sorting: Some(SortingConfig {
                multilingual: Some(SortingMultilingualMode::Romanized),
                ..Default::default()
            }),
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                sort: Some(citum_schema::options::SortEntry::Explicit(Sort {
                    template: vec![SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    }],
                    shorten_names: false,
                    render_substitutions: false,
                })),
                ..Default::default()
            })),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                sort_partitioning: Some(BibliographySortPartitioning {
                    by: BibliographyPartitionKind::Script,
                    mode: BibliographyPartitionMode::SortOnly,
                    order: vec!["Cyrl".to_string(), "Latn".to_string()],
                    headings: HashMap::new(),
                    unknown_fields: Default::default(),
                }),
                ..Default::default()
            }),
            template: Some(vec![citum_schema::tc_title!(Primary)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let processor = Processor::new(style, romanized_partition_bibliography());

    assert_eq!(
        processor.render_bibliography(),
        "Tolstoy title\n\nPushkin title\n\nLatin title"
    );
}

#[test]
fn given_language_sort_only_partitioning_when_rendering_flat_bibliography_then_effective_language_order_is_used()
 {
    announce_behavior(
        "Language partitioning uses reference language before the normal bibliography sort chain.",
    );
    let style = build_partition_style(
        r#"
options:
  sort-partitioning:
    by: language
    mode: sort-only
    order: [ru, en, ja]
"#,
        "",
    );
    let processor = Processor::new(style, language_partition_bibliography());

    assert_eq!(processor.render_bibliography(), "Beta\n\nAlpha\n\nGamma");
}

#[test]
#[cfg(feature = "icu")]
fn given_script_section_partitioning_when_rendering_grouped_bibliography_then_configured_headings_are_used()
 {
    announce_behavior(
        "Script partitioning in sections mode renders automatic grouped bibliography sections with configured headings.",
    );
    let style = build_partition_style(
        r#"
options:
  sort-partitioning:
    by: script
    mode: sections
    order: [Cyrl, Latn, Hani]
    headings:
      Cyrl: { literal: "Cyrillic" }
      Latn: { literal: "Latin" }
      Hani: { literal: "Han" }
"#,
        "",
    );
    let processor = Processor::new(style, script_partition_bibliography());

    assert_eq!(
        processor.render_grouped_bibliography_with_format_standalone::<PlainText>(),
        "## Cyrillic\n\nБета\n\n## Latin\n\nAlpha\n\n## Han\n\n東京"
    );
}

#[test]
#[cfg(feature = "icu")]
fn given_explicit_groups_when_partition_sections_are_enabled_then_manual_groups_remain_authoritative()
 {
    announce_behavior(
        "Explicit bibliography groups disable automatic partition sections while retaining partition-aware order inside unsorted groups.",
    );
    let style = build_partition_style(
        r#"
options:
  sort-partitioning:
    by: script
    mode: sort-and-sections
    order: [Cyrl, Latn, Hani]
    headings:
      Cyrl: { literal: "Cyrillic" }
      Latn: { literal: "Latin" }
      Hani: { literal: "Han" }
"#,
        r#"
groups:
  - id: manual
    heading: { literal: "Manual Group" }
    selector: {}
"#,
    );
    let processor = Processor::new(style, script_partition_bibliography());

    let output = processor.render_grouped_bibliography_with_format_standalone::<PlainText>();

    // Manual groups gate wins; auto-partition section headings must not appear
    assert_eq!(output, "Бета\n\nAlpha\n\n東京");
    assert!(
        !output.contains("Cyrillic"),
        "auto-partition heading 'Cyrillic' must not appear when manual groups are configured: {output}"
    );
    assert!(
        !output.contains("Latin"),
        "auto-partition heading 'Latin' must not appear when manual groups are configured: {output}"
    );
    assert!(
        !output.contains("Han"),
        "auto-partition heading 'Han' must not appear when manual groups are configured: {output}"
    );
}

#[test]
fn disabled_manual_groups_render_every_reference_flat() {
    announce_behavior(
        "Disabling a retained manual groups block renders the complete standalone bibliography without group headings.",
    );
    let style = build_partition_style(
        "",
        r#"
groups-enabled: false
groups:
  - id: disabled-books
    heading: { literal: "Disabled Books" }
    selector:
      type: book
"#,
    );
    let bibliography = IndexMap::from([
        (
            "book".to_string(),
            typed_partition_reference("book", "book", "Alpha Book", Some("en")),
        ),
        (
            "article".to_string(),
            typed_partition_reference("article", "article-journal", "Beta Article", Some("en")),
        ),
    ]);
    let processor = Processor::new(style, bibliography);

    assert_eq!(
        processor.render_grouped_bibliography_with_format_standalone::<PlainText>(),
        "Alpha Book\n\nBeta Article"
    );
}

#[test]
fn grouped_live_run_remains_all_references() {
    announce_behavior(
        "Grouped rendering against a live run includes the whole library even when only one reference has been cited.",
    );
    let style = build_partition_style("", "");
    let bibliography = IndexMap::from([
        (
            "cited".to_string(),
            typed_partition_reference("cited", "book", "Alpha Cited", Some("en")),
        ),
        (
            "uncited".to_string(),
            typed_partition_reference("uncited", "article-journal", "Beta Uncited", Some("en")),
        ),
    ]);
    let processor = Processor::new(style, bibliography);
    let citation = Citation {
        items: vec![CitationItem {
            id: "cited".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut run = processor.begin_run();
    processor
        .process_citation_with_format::<PlainText>(&citation, &mut run)
        .expect("citation should render");
    let run = run.finalize();

    assert_eq!(
        processor.render_grouped_bibliography_with_format::<PlainText>(&run),
        "Alpha Cited\n\nBeta Uncited"
    );
}

#[test]
fn disabled_manual_groups_fall_through_to_partition_sections() {
    announce_behavior(
        "Disabling manual groups leaves automatic bibliography partition sections active.",
    );
    let style = build_partition_style(
        r#"
options:
  sort-partitioning:
    by: language
    mode: sections
    order: [ru, en]
    headings:
      ru: { literal: "Russian" }
      en: { literal: "English" }
"#,
        r#"
groups-enabled: false
groups:
  - id: disabled
    heading: { literal: "Disabled Manual Group" }
    selector: {}
"#,
    );
    let processor = Processor::new(style, language_partition_bibliography());

    assert_eq!(
        processor.render_grouped_bibliography_with_format_standalone::<PlainText>(),
        "## Russian\n\nBeta\n\n## English\n\nAlpha\n\nGamma"
    );
}

#[test]
fn language_partition_sections_reset_subsequent_author_substitution_per_section() {
    announce_behavior(
        "Partition sections render subsequent-author substitution within a section only, resetting the chain at each new partition.",
    );
    let style = language_partition_substitute_style();
    let bibliography = IndexMap::from([
        (
            "ru-first".to_string(),
            language_partition_reference("ru-first", "Smith", "John", "Alpha", "ru"),
        ),
        (
            "ru-second".to_string(),
            language_partition_reference("ru-second", "Smith", "John", "Beta", "ru"),
        ),
        (
            "en-only".to_string(),
            language_partition_reference("en-only", "Smith", "John", "Gamma", "en"),
        ),
    ]);
    let processor = Processor::new(style, bibliography);

    assert_eq!(
        processor.render_grouped_bibliography_with_format_standalone::<PlainText>(),
        "## Russian\n\nSmith, John. Alpha.\n\n———. Beta.\n\n## English\n\nSmith, John. Gamma."
    );
}

fn build_container_title_short_style(title_type: TitleType) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Container Title Short Test".to_string()),
            id: Some("container-title-short-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Group(TemplateGroup {
                    group: vec![
                        TemplateComponent::Variable(TemplateVariable {
                            variable: SimpleVariable::ContainerTitleShort,
                            ..Default::default()
                        }),
                        TemplateComponent::Title(TemplateTitle {
                            title: title_type.clone(),
                            form: Some(TitleForm::Short),
                            ..Default::default()
                        }),
                        TemplateComponent::Title(TemplateTitle {
                            title: title_type,
                            form: Some(TitleForm::Long),
                            ..Default::default()
                        }),
                    ],
                    delimiter: Some(DelimiterPunctuation::Slash),
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_group_with_suppressed_child_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Grouped Suppression Test".to_string()),
            id: Some("grouped-suppression-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Group(TemplateGroup {
                    group: vec![
                        TemplateComponent::Variable(TemplateVariable {
                            variable: SimpleVariable::Url,
                            rendering: Rendering {
                                suppress: Some(true),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        TemplateComponent::Title(TemplateTitle {
                            title: TitleType::Primary,
                            ..Default::default()
                        }),
                    ],
                    delimiter: Some(DelimiterPunctuation::Slash),
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_status_bibliography_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Status Test".to_string()),
            id: Some("status-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        ..Default::default()
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Status,
                        rendering: Rendering {
                            prefix: Some(". ".into()),
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

fn build_article_journal_no_page_fallback_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Article Journal Fallback Test".to_string()),
            id: Some("article-journal-fallback-test".into()),
            ..Default::default()
        },
        options: Some(Config::default()),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                article_journal: Some(ArticleJournalBibliographyConfig {
                    no_page_fallback: Some(ArticleJournalNoPageFallback::Doi),
                    ..Default::default()
                }),
                separator: Some(", ".into()),
                ..Default::default()
            }),
            template: Some(
                vec![
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentSerial,
                        ..Default::default()
                    }),
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![
                            TemplateComponent::Date(TemplateDate {
                                date: DateVariable::Issued,
                                form: DateForm::Year,
                                ..Default::default()
                            }),
                            TemplateComponent::Number(TemplateNumber {
                                number: NumberVariable::Volume,
                                ..Default::default()
                            }),
                            TemplateComponent::Number(TemplateNumber {
                                number: NumberVariable::Issue,
                                rendering: Rendering {
                                    prefix: Some("(".into()),
                                    suffix: Some(")".into()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                            TemplateComponent::Number(TemplateNumber {
                                number: NumberVariable::Pages,
                                rendering: Rendering {
                                    prefix: Some("pp. ".into()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                        ],
                        delimiter: Some(DelimiterPunctuation::Comma),
                        ..Default::default()
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Doi,
                        rendering: Rendering {
                            prefix: Some("DOI:".into()),
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

fn build_anonymous_entry_policy_style() -> Style {
    let style_yaml = r#"
info:
  title: Anonymous Entry Policy Test
  id: anonymous-entry-policy-test
bibliography:
  options:
    anonymous-entries: notes-only
  type-variants:
    entry-dictionary:
      - contributor: author
        form: long
      - variable: version
      - title: primary
      - title: parent-monograph
      - date: issued
        form: year
      - variable: doi
        prefix: "https://doi.org/"
      - variable: url
    entry-encyclopedia:
      - contributor: author
        form: long
      - title: primary
      - title: parent-monograph
      - date: issued
        form: year
      - variable: doi
        prefix: "https://doi.org/"
      - variable: url
"#;

    serde_yaml::from_str(style_yaml).expect("style should parse")
}

fn build_bibliography_entry_link_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Bibliography Entry Link Test".to_string()),
            id: Some("bibliography-entry-link-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                links: Some(LinksConfig {
                    url: Some(true),
                    target: Some(LinkTarget::Url),
                    anchor: Some(LinkAnchor::Entry),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            template: Some(vec![citum_schema::tc_title!(Primary)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_bibliography_local_note_sort_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Bibliography Local Note Sort Test".to_string()),
            id: Some("bibliography-local-note-sort-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                processing: Some(Processing::Note),
                label_mode: Some(citum_schema::options::BibliographyLabelMode::Numeric),
                label_separator: Some(". ".to_string()),
                ..Default::default()
            }),
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_title!(Primary, prefix = ". "),
                    citum_schema::tc_date!(Issued, Year, prefix = " "),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_bibliography_local_numeric_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Bibliography Local Numeric Test".to_string()),
            id: Some("bibliography-local-numeric-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                processing: Some(Processing::Numeric),
                label_mode: Some(citum_schema::options::BibliographyLabelMode::Numeric),
                label_separator: Some(". ".to_string()),
                ..Default::default()
            }),
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_numeric_citation_style_with_bibliography_local_note_sort() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Numeric Citation Local Note Sort Test".to_string()),
            id: Some("numeric-citation-local-note-sort-test".into()),
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
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                processing: Some(Processing::Note),
                label_mode: Some(citum_schema::options::BibliographyLabelMode::Numeric),
                label_separator: Some(". ".to_string()),
                ..Default::default()
            }),
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_title!(Primary, prefix = ". "),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_inline_article_journal_detail_group_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Inline Article Journal Detail Group Test".to_string()),
            id: Some("inline-article-journal-detail-group-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Contributor(citum_schema::template::TemplateContributor {
                        contributor: citum_schema::template::ContributorRole::Author.into(),
                        form: citum_schema::template::ContributorForm::Long,
                        rendering: Rendering {
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentSerial,
                        rendering: Rendering {
                            emph: Some(true),
                            suffix: Some(". ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![
                            TemplateComponent::Number(TemplateNumber {
                                number: NumberVariable::Volume,
                                ..Default::default()
                            }),
                            TemplateComponent::Group(TemplateGroup {
                                group: vec![
                                TemplateComponent::Number(TemplateNumber {
                                    number: NumberVariable::Issue,
                                    ..Default::default()
                                }),
                                TemplateComponent::Date(TemplateDate {
                                    date: DateVariable::Issued,
                                    form: DateForm::YearMonth,
                                    rendering: Rendering {
                                        wrap: Some(
                                            citum_schema::template::WrapPunctuation::Parentheses
                                                .into(),
                                        ),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                            ],
                                delimiter: Some(DelimiterPunctuation::Space),
                                ..Default::default()
                            }),
                            TemplateComponent::Number(TemplateNumber {
                                number: NumberVariable::Pages,
                                rendering: Rendering {
                                    prefix: Some("pp. ".into()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }),
                        ],
                        delimiter: Some(DelimiterPunctuation::Comma),
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

fn build_archive_eprint_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Archive and Eprint Test".to_string()),
            id: Some("archive-eprint-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_title!(Primary, suffix = ". "),
                    variable_component(SimpleVariable::ArchiveName, None, None),
                    variable_component(SimpleVariable::ArchiveCollection, Some(", "), None),
                    variable_component(SimpleVariable::ArchiveCollectionId, Some(", "), None),
                    variable_component(SimpleVariable::ArchiveSeries, Some(", Series "), None),
                    variable_component(SimpleVariable::ArchiveBox, Some(", Box "), None),
                    variable_component(SimpleVariable::ArchiveFolder, Some(", Folder "), None),
                    variable_component(SimpleVariable::ArchiveItem, Some(", Item "), None),
                    variable_component(SimpleVariable::ArchiveLocation, Some(", "), None),
                    variable_component(SimpleVariable::ArchivePlace, Some(", "), None),
                    variable_component(SimpleVariable::ArchiveUrl, Some(", "), None),
                    variable_component(SimpleVariable::EprintServer, Some(", "), None),
                    variable_component(SimpleVariable::EprintId, Some(":"), None),
                    variable_component(SimpleVariable::EprintClass, Some(" ["), Some("]")),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_archive_location_fallback_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Archive Location Fallback Test".to_string()),
            id: Some("archive-location-fallback-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_title!(Primary, suffix = ". "),
                    variable_component(SimpleVariable::ArchiveName, None, None),
                    variable_component(SimpleVariable::ArchiveCollection, Some(", "), None),
                    variable_component(SimpleVariable::ArchiveLocation, Some(", "), None),
                    variable_component(SimpleVariable::ArchivePlace, Some(", "), None),
                    variable_component(SimpleVariable::ArchiveUrl, Some(", "), None),
                    variable_component(SimpleVariable::EprintServer, Some(", "), None),
                    variable_component(SimpleVariable::EprintId, Some(":"), None),
                    variable_component(SimpleVariable::EprintClass, Some(" ["), Some("]")),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn variable_component(
    variable: SimpleVariable,
    prefix: Option<&str>,
    suffix: Option<&str>,
) -> TemplateComponent {
    TemplateComponent::Variable(TemplateVariable {
        variable,
        rendering: Rendering {
            prefix: prefix.map(Into::into),
            suffix: suffix.map(Into::into),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn build_multilingual_archive_name_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Multilingual Archive Name Test".to_string()),
            id: Some("multilingual-archive-name-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            multilingual: Some(MultilingualConfig {
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Variable(TemplateVariable {
                    variable: SimpleVariable::ArchiveName,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_archive_eprint_reference() -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("archive-eprint-ref".into()),
        r#type: MonographType::Preprint,
        title: Some(Title::Single("Archive-Aware Preprint".to_string())),
        container: None,
        author: None,
        editor: None,
        translator: None,
        issued: DateValue::new("2026-02".to_string()),
        publisher: None,
        url: Some(Url::parse("https://arxiv.org/abs/2602.01234").expect("url should parse")),
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: None,
        medium: None,
        archive: None,
        archive_location: None,
        archive_info: Some(ArchiveInfo {
            name: Some(MultilingualString::Simple("Houghton Library".to_string())),
            place: Some("Cambridge, MA".into()),
            collection: Some("Ada Lovelace Papers".to_string()),
            collection_id: Some("MS Am 1280".to_string()),
            series: Some("Correspondence".to_string()),
            r#box: Some("12".to_string()),
            folder: Some("4".to_string()),
            item: Some("7".to_string()),
            url: Some(Url::parse("https://example.com/archive").expect("url should parse")),
            ..Default::default()
        }),
        eprint: Some(EprintInfo {
            id: "2602.01234".to_string(),
            server: "arxiv".to_string(),
            class: Some("cs.DL".to_string()),
        }),
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }))
}

fn make_multilingual_archive_name_reference() -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("archive-name-ref".into()),
        r#type: MonographType::Document,
        title: Some(Title::Single("Repository Record".to_string())),
        container: None,
        author: None,
        editor: None,
        translator: None,
        issued: DateValue::new("2024".to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: None,
        medium: None,
        archive: None,
        archive_location: None,
        archive_info: Some(ArchiveInfo {
            name: Some(MultilingualString::Complex(MultilingualComplex {
                original: "東京国立博物館".to_string(),
                lang: Some("ja".into()),
                sort_as: None,
                transliterations: HashMap::from([(
                    "ja-Latn-hepburn".to_string(),
                    "Tokyo National Museum".to_string(),
                )]),
                translations: HashMap::new(),
            })),
            ..Default::default()
        }),
        eprint: None,
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }))
}

fn make_historical_archive_reference() -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("dead-sea-scrolls-demo".into()),
        r#type: MonographType::Manuscript,
        title: Some(Title::Single("The Community Rule (1QS)".to_string())),
        container: None,
        author: None,
        editor: None,
        translator: None,
        issued: DateValue::new("-0099".to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: Some("manuscript-scroll".to_string()),
        medium: None,
        archive: None,
        archive_location: None,
        archive_info: Some(ArchiveInfo {
            name: Some(MultilingualString::Simple(
                "Israel Antiquities Authority".to_string(),
            )),
            location: Some("Shrine of the Book".to_string()),
            place: Some("Jerusalem".into()),
            ..Default::default()
        }),
        eprint: None,
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }))
}

fn bibliography_html_injects_sparse_indices_from_type_template() {
    let style_yaml = r#"
info:
  title: Indexed Bibliography Preview
  id: indexed-bibliography-preview
bibliography:
  type-variants:
    article-journal:
      - contributor: author
        form: long
      - variable: doi
        prefix: " "
      - title: primary
        prefix: ". "
"#;
    let style: Style = serde_yaml::from_str(style_yaml).expect("style should parse");

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-1",
        "type": "article-journal",
        "title": "Preview Article",
        "author": [{"family": "Smith", "given": "Jane"}]
    }))
    .expect("legacy fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-1".to_string(), legacy.into());

    let processor = Processor::new(style, bib).with_inject_ast_indices(true);
    let rendered = processor.render_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains(r#"class="citum-author" data-index="0""#),
        "author wrapper should carry the first type-template index: {rendered}"
    );
    assert!(
        rendered.contains(r#"class="citum-title" data-index="2""#),
        "title wrapper should carry the sparse third type-template index: {rendered}"
    );
    assert!(
        !rendered.contains(r#"data-index="1""#),
        "missing DOI output should preserve sparse template indices: {rendered}"
    );
}

#[test]
fn checked_in_archive_demo_style_renders_historical_manuscript_with_bc_suffix() {
    announce_behavior(
        "The checked-in archival demo style renders historical manuscript years with a BC suffix.",
    );

    let style_path = project_root()
        .join("examples")
        .join("archive-eprint-demo-style.yaml");
    let style_yaml = fs::read_to_string(&style_path).expect("demo style should load");
    let style: Style = serde_yaml::from_str(&style_yaml).expect("demo style should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "dead-sea-scrolls-demo".to_string(),
        make_historical_archive_reference(),
    );

    let rendered = Processor::new(style, bib).render_bibliography();
    assert!(
        rendered.contains(
            "The Community Rule (1QS). Manuscript scroll, 100 BC, Israel Antiquities Authority, Shrine of the Book, Jerusalem"
        ),
        "historical manuscript output should match the checked-in docs example: {rendered}"
    );
}

fn build_list_index_preview_style(use_type_template: bool) -> Style {
    let bibliography_yaml = if use_type_template {
        r#"
bibliography:
  type-variants:
    article-journal:
      - group:
          - contributor: author
            form: long
          - title: primary
            prefix: ". "
        delimiter: ", "
"#
    } else {
        r#"
bibliography:
  template:
    - group:
        - contributor: author
          form: long
        - title: primary
          prefix: ". "
      delimiter: ", "
"#
    };

    let yaml = format!(
        r#"
info:
  title: List Index Preview
  id: {}
{}
"#,
        if use_type_template {
            "list-index-preview-type"
        } else {
            "list-index-preview-default"
        },
        bibliography_yaml
    );

    serde_yaml::from_str(&yaml).expect("style should parse")
}

fn assert_list_preview_inherits_parent_index(use_type_template: bool) {
    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-1",
        "type": "article-journal",
        "title": "Preview Article",
        "author": [{"family": "Smith", "given": "Jane"}]
    }))
    .expect("legacy fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-1".to_string(), legacy.into());

    let processor = Processor::new(build_list_index_preview_style(use_type_template), bib)
        .with_inject_ast_indices(true);
    let rendered = processor.render_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains(r#"class="citum-author" data-index="0""#),
        "list-rendered author should inherit the parent top-level index: {rendered}"
    );
    assert!(
        rendered.contains(r#"class="citum-title" data-index="0""#),
        "list-rendered title should inherit the parent top-level index: {rendered}"
    );
}

fn make_article_journal_with_detail(
    id: &str,
    issued: &str,
    issue: Option<&str>,
    pages: Option<&str>,
    doi: Option<&str>,
) -> InputReference {
    let mut numbering = vec![Numbering {
        r#type: NumberingType::Volume,
        value: "12".to_string(),
    }];
    if let Some(i) = issue {
        numbering.push(Numbering {
            r#type: NumberingType::Issue,
            value: i.to_string(),
        });
    }

    InputReference::SerialComponent(Box::new(SerialComponent {
        id: Some(id.into()),
        r#type: SerialComponentType::Article,
        title: Some(Title::Single("Fallback Article".to_string())),
        author: Some(Contributor::StructuredName(StructuredName {
            family: "Doe".into(),
            given: "Jane".into(),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: None,
        })),
        translator: None,
        issued: DateValue::new(issued.to_string()),
        container: Some(WorkRelation::Embedded(Box::new(InputReference::Serial(
            Box::new(Serial {
                r#type: SerialType::AcademicJournal,
                title: Some(Title::Single("Journal of Fallbacks".to_string())),
                ..Default::default()
            }),
        )))),
        url: None,
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        doi: doi.map(str::to_string),
        ads_bibcode: None,
        pages: pages.map(str::to_string),
        numbering,
        genre: None,
        medium: None,
        archive_info: None,
        eprint: None,
        keywords: None,
        reviewed: None,
        original: None,
        ..Default::default()
    }))
}

struct EntryReferenceParams<'a> {
    id: &'a str,
    entry_type: &'a str,
    title: &'a str,
    container_title: &'a str,
    year: i32,
    doi: Option<&'a str>,
    url: Option<&'a str>,
    author: Option<(&'a str, &'a str)>,
}

fn make_entry_reference(params: EntryReferenceParams<'_>) -> InputReference {
    let mut fixture = serde_json::json!({
        "id": params.id,
        "type": params.entry_type,
        "title": params.title,
        "container-title": params.container_title,
        "issued": {"date-parts": [[params.year]]},
    });

    if let Some(doi) = params.doi {
        fixture["DOI"] = serde_json::json!(doi);
    }
    if let Some(url) = params.url {
        fixture["URL"] = serde_json::json!(url);
    }
    if let Some((family, given)) = params.author {
        fixture["author"] = serde_json::json!([{ "family": family, "given": given }]);
    }

    let legacy: csl_legacy::csl_json::Reference =
        serde_json::from_value(fixture).expect("entry fixture should parse");
    legacy.into()
}

fn build_processing_style(processing: Processing) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Processing Default Sort Test".to_string()),
            id: Some("processing-default-sort-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(processing),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_date!(Issued, Year, prefix = " "),
                    citum_schema::tc_title!(Primary, prefix = ". "),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_style_with_substitute(substitute: Option<String>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Subsequent Author Substitute Test".to_string()),
            id: Some("sub-test".into()),
            ..Default::default()
        },
        templates: None,
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                ..Default::default()
            }),
            ..Default::default()
        }),
        citation: None,
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                subsequent_author_substitute: substitute,
                entry_suffix: Some(".".into()),
                ..Default::default()
            }),
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_date!(Issued, Year),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_particle_book(
    id: &str,
    family: &str,
    given: &str,
    particle: Option<&str>,
) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(format!("Title {id}"))),
        container: None,
        author: Some(Contributor::StructuredName(StructuredName {
            family: family.into(),
            given: given.into(),
            suffix: None,
            dropping_particle: None,
            non_dropping_particle: particle.map(Into::into),
        })),
        editor: None,
        translator: None,
        issued: citum_schema::reference::DateValue::new("2000".to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: None,
        medium: None,
        archive_info: None,
        eprint: None,
        archive: None,
        archive_location: None,
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }))
}

fn make_editor_only_book(
    id: &str,
    title: &str,
    year: &str,
    family: &str,
    given: &str,
) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        container: None,
        author: None,
        editor: Some(Contributor::StructuredName(StructuredName {
            given: given.into(),
            family: family.into(),
            ..Default::default()
        })),
        translator: None,
        issued: DateValue::new(year.to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: None,
        medium: None,
        archive_info: None,
        eprint: None,
        archive: None,
        archive_location: None,
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }))
}

fn make_multi_editor_only_book(
    id: &str,
    title: &str,
    year: &str,
    editors: Vec<(&str, &str)>,
) -> InputReference {
    let editors = editors
        .into_iter()
        .map(|(family, given)| {
            Contributor::StructuredName(StructuredName {
                given: given.into(),
                family: family.into(),
                ..Default::default()
            })
        })
        .collect();

    InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        container: None,
        author: None,
        editor: Some(Contributor::ContributorList(
            citum_schema::reference::ContributorList(editors),
        )),
        translator: None,
        issued: DateValue::new(year.to_string()),
        publisher: None,
        url: None,
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: None,
        medium: None,
        archive_info: None,
        eprint: None,
        archive: None,
        archive_location: None,
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }))
}

fn build_editor_verb_prefix_style(title_suffix: Option<&str>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Editor Verb Prefix Test".to_string()),
            id: Some("editor-verb-prefix-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            contributors: Some(ContributorConfig {
                role: Some(citum_schema::options::contributors::RoleOptions {
                    preset: Some(citum_schema::options::contributors::RoleLabelPreset::VerbPrefix),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_title!(Primary, suffix = title_suffix.unwrap_or("")),
                    citum_schema::tc_contributor!(Editor, Long),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_bare_long_form_editor_style(role: Option<ContributorConfig>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Bare Long Form Editor Test".to_string()),
            id: Some("bare-long-form-editor-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            contributors: role,
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(vec![citum_schema::tc_contributor!(Editor, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn bare_long_form_editor_gets_no_label_by_default() {
    // With no label config, no role.omit, no configured role preset, and no
    // `role.defaults` bundle, a Long-form editor component renders bare: the
    // old engine-hardcoded " (ed.)" suffix was removed in favor of explicit
    // per-style `contributors.role.defaults` bundles. See div-012 in
    // docs/adjudication/DIVERGENCE_REGISTER.md and
    // docs/specs/ROLE_LABEL_DEFAULTS.md.
    let style = build_bare_long_form_editor_style(None);
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith");
}

#[test]
fn apa_role_label_defaults_bundle_restores_editor_suffix_in_bibliography() {
    // A style declaring `contributors.role.defaults: apa` opts back into the
    // abbreviated editor suffix in bibliography context.
    let style = build_bare_long_form_editor_style(Some(ContributorConfig {
        role: Some(citum_schema::options::contributors::RoleOptions {
            defaults: Some(citum_schema::options::contributors::RoleLabelDefaults::Apa),
            ..Default::default()
        }),
        ..Default::default()
    }));
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith (ed.)");
}

#[test]
fn per_role_preset_none_overrides_the_defaults_bundle() {
    // contributors.role.roles.<role>.preset: none wins over a declared
    // `role.defaults` bundle: configured presets are resolved before the
    // bundle, so a style can opt one role out of its bundle. See div-012
    // and docs/specs/ROLE_LABEL_DEFAULTS.md.
    let mut roles = HashMap::new();
    roles.insert(
        "editor".to_string(),
        citum_schema::options::contributors::RoleRendering {
            preset: Some(citum_schema::options::contributors::RoleLabelPreset::None),
            ..Default::default()
        },
    );
    let style = build_bare_long_form_editor_style(Some(ContributorConfig {
        role: Some(citum_schema::options::contributors::RoleOptions {
            roles: Some(roles),
            defaults: Some(citum_schema::options::contributors::RoleLabelDefaults::Apa),
            ..Default::default()
        }),
        ..Default::default()
    }));
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith");
}

#[test]
fn unrecognized_label_term_falls_back_to_the_component_own_role_term() {
    // resolve_explicit_label recognizes only "chair"/"editor"/"translator"
    // as label.term keys; any other string silently substitutes the
    // component's own role term instead of erroring. This test locks in
    // that render behavior is unchanged by the new unknown_role_label_term
    // warning (a diagnostic-only addition) -- the editor's own "(ed.)"-style
    // term is still used even though "not-a-real-role" is not recognized.
    let yaml = "info:\n  title: Test\nbibliography:\n  template:\n    - contributor: editor\n      form: long\n      label: {term: not-a-real-role, placement: suffix}\n";
    let style = citum_schema::Style::from_yaml_str(yaml).expect("style should parse");

    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith, ed.");
}

/// Format selector for rstest-parameterized format tests.
enum TestOutputFormat {
    Plain,
    Html,
    Latex,
    Typst,
}

fn render_bibliography_in_format(processor: &Processor, fmt: TestOutputFormat) -> String {
    match fmt {
        TestOutputFormat::Plain => {
            processor.render_bibliography_with_format_standalone::<PlainText>()
        }
        TestOutputFormat::Html => processor.render_bibliography_with_format_standalone::<Html>(),
        TestOutputFormat::Latex => processor.render_bibliography_with_format_standalone::<Latex>(),
        TestOutputFormat::Typst => processor.render_bibliography_with_format_standalone::<Typst>(),
    }
}

fn build_sentence_initial_emph_group_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Sentence Initial Emph Group Test".to_string()),
            id: Some("sentence-initial-emph-group-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Group(TemplateGroup {
                    group: vec![TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        rendering: Rendering {
                            emph: Some(true),
                            ..Default::default()
                        },
                        ..Default::default()
                    })],
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn make_editor_substitute_bibliography() -> indexmap::IndexMap<String, InputReference> {
    citum_schema::bib_map![
        "ancient-tale" => make_editor_only_book(
            "ancient-tale",
            "The Ancient Tale",
            "1850",
            "Grimm",
            "Jacob",
        ),
        "ipcc2023" => make_multi_editor_only_book(
            "ipcc2023",
            "Climate Change 2023: Synthesis Report",
            "2023",
            vec![("Lee", "Hoesung"), ("Romero", "Jose")],
        ),
    ]
}

fn make_name_particle_style(display_as_sort: DisplayAsSort) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Hyphenated Particle Test".to_string()),
            id: Some("hyphenated-particle-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                sort: Some(citum_schema::options::SortEntry::Explicit(Sort {
                    template: vec![SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    }],
                    shorten_names: false,
                    render_substitutions: false,
                })),
                ..Default::default()
            })),
            contributors: Some(ContributorConfig {
                display_as_sort: Some(display_as_sort),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::DisplayAndSort),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(vec![citum_schema::tc_contributor!(Author, Long)].into()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// --- Sorting Tests ---

fn sorting_by_author_orders_entries_alphabetically() {
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

fn sorting_by_year_places_earlier_years_first() {
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

fn sorting_empty_dates_pushes_undated_items_after_dated_ones() {
    // Upstream provenance: CSL fixture `date_SortEmptyDatesBibliography`.
    let style = build_title_year_sorted_style(vec![
        SortSpec {
            key: SortKey::Year,
            ascending: true,
        },
        SortSpec {
            key: SortKey::Title,
            ascending: true,
        },
    ]);

    fn make_undated_book(id: &str, title: &str) -> InputReference {
        let mut reference = make_book(id, "Smith", "Jane", 2000, title);
        if let ClassExtension::Monograph(monograph) = reference.extension_mut() {
            monograph.issued = citum_schema::reference::DateValue::new(String::new());
        }
        reference
    }

    let mut bib = indexmap::IndexMap::new();
    bib.insert("item1".to_string(), make_undated_book("item1", "BookA"));
    bib.insert(
        "item2".to_string(),
        make_book("item2", "Smith", "Jane", 2000, "BookB"),
    );
    bib.insert("item3".to_string(), make_undated_book("item3", "BookC"));
    bib.insert(
        "item4".to_string(),
        make_book("item4", "Smith", "Jane", 1999, "BookD"),
    );
    bib.insert("item5".to_string(), make_undated_book("item5", "BookE"));

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(result.find("BookD 1999").unwrap() < result.find("BookB 2000").unwrap());
    assert!(result.find("BookB 2000").unwrap() < result.find("BookA").unwrap());
    assert!(result.find("BookA").unwrap() < result.find("BookC").unwrap());
    assert!(result.find("BookC").unwrap() < result.find("BookE").unwrap());
}

fn container_title_short_uses_journal_abbreviation_when_present() {
    // Upstream provenance: CSL fixtures `bugreports_ContainerTitleShort` and
    // `variables_ContainerTitleShort`.
    let style = build_container_title_short_style(TitleType::ParentSerial);

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-1",
        "type": "article-journal",
        "title": "Ignored",
        "container-title": "Anonymous Journal",
        "journalAbbreviation": "Anon J"
    }))
    .expect("legacy fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-1".to_string(), legacy.into());

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor.render_bibliography(),
        "Anon J/Anon J/Anonymous Journal"
    );
}

fn container_title_short_prefers_explicit_short_field() {
    // Upstream provenance: CSL fixtures `bugreports_ContainerTitleShort` and
    // `variables_ContainerTitleShort`.
    let style = build_container_title_short_style(TitleType::ParentMonograph);

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-2",
        "type": "chapter",
        "title": "Ignored",
        "container-title": "Anonymous Journal One",
        "container-title-short": "Journal-1"
    }))
    .expect("legacy fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-2".to_string(), legacy.into());

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor.render_bibliography(),
        "Journal-1/Journal-1/Anonymous Journal One"
    );
}

#[test]
fn collection_title_component_renders_parent_series_title() {
    let style = Style {
        info: StyleInfo {
            title: Some("Collection Title Test".to_string()),
            id: Some("collection-title-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::CollectionTitle,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-SERIES-1",
        "type": "chapter",
        "title": "Ignored",
        "container-title": "Edited Book",
        "collection-title": "Studies in Examples"
    }))
    .expect("legacy fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-SERIES-1".to_string(), legacy.into());

    let processor = Processor::new(style, bib);
    assert_eq!(processor.render_bibliography(), "Studies in Examples");
}

#[test]
fn container_title_component_renders_monograph_or_serial_parent_title() {
    let style = Style {
        info: StyleInfo {
            title: Some("Container Title Test".to_string()),
            id: Some("container-title-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::ContainerTitle,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let chapter: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-CHAPTER-1",
        "type": "chapter",
        "title": "Ignored Chapter",
        "container-title": "Edited Book"
    }))
    .expect("legacy chapter fixture should parse");
    let article: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-ARTICLE-1",
        "type": "article-journal",
        "title": "Ignored Article",
        "container-title": "Example Journal"
    }))
    .expect("legacy article fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-CHAPTER-1".to_string(), chapter.into());
    bib.insert("ITEM-ARTICLE-1".to_string(), article.into());

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor.render_bibliography(),
        "Example Journal\n\nEdited Book"
    );
}

#[test]
fn legal_case_parent_serial_uses_reporter_as_container_title() {
    let style = Style {
        info: StyleInfo {
            title: Some("Legal Reporter Parent Serial Test".to_string()),
            id: Some("legal-reporter-parent-serial-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Group(TemplateGroup {
                    group: vec![
                        TemplateComponent::Title(TemplateTitle {
                            title: TitleType::ParentSerial,
                            ..Default::default()
                        }),
                        TemplateComponent::Number(TemplateNumber {
                            number: NumberVariable::Volume,
                            ..Default::default()
                        }),
                        TemplateComponent::Number(TemplateNumber {
                            number: NumberVariable::Pages,
                            ..Default::default()
                        }),
                    ],
                    delimiter: Some(DelimiterPunctuation::Slash),
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-LEGAL-1",
        "type": "legal_case",
        "title": "Brown v. Board of Education",
        "authority": "U.S. Supreme Court",
        "volume": "347",
        "container-title": "U.S. Reports",
        "page": "483",
        "issued": { "date-parts": [[1954, 5, 17]] }
    }))
    .expect("legal case fixture should parse");

    let mut bib = indexmap::IndexMap::new();
    bib.insert("ITEM-LEGAL-1".to_string(), legacy.into());

    let processor = Processor::new(style, bib);
    assert_eq!(processor.render_bibliography(), "U.S. Reports/347/483");
}

fn suppressed_group_children_do_not_leave_stray_delimiters() {
    let style = build_group_with_suppressed_child_style();
    let mut reference = make_book("grouped", "Smith", "Jane", 2024, "Grouped Title");
    if let ClassExtension::Monograph(book) = reference.extension_mut() {
        book.url = Some(Url::parse("https://example.com/grouped").expect("url should parse"));
    }

    let mut bib = indexmap::IndexMap::new();
    bib.insert("grouped".to_string(), reference);

    let processor = Processor::new(style, bib);
    assert_eq!(processor.render_bibliography(), "Grouped Title");
}

fn status_variables_render_in_bibliography_templates() {
    let style = build_status_bibliography_style();
    let mut reference = make_book(
        "status",
        "Lexicographer",
        "A.",
        2025,
        "Oxford English Dictionary",
    );
    if let ClassExtension::Monograph(book) = reference.extension_mut() {
        book.status = Some("last modified".to_string());
    }

    let mut bib = indexmap::IndexMap::new();
    bib.insert("status".to_string(), reference);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor.render_bibliography(),
        "Oxford English Dictionary. last modified"
    );
}

fn sorting_multiple_keys_applies_secondary_ordering_within_author_groups() {
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

fn author_date_processing_defaults_bibliography_to_author_date_title_order() {
    let style = build_processing_style(Processing::AuthorDate);

    // Different years: author-date-title puts 2020 (Zeta) before 2022 (Alpha).
    // author-title-date would put Alpha before Zeta (title-first within same author).
    // Only author-date-title produces Zeta < Alpha, making this a true discriminator.
    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "alpha".to_string(),
        make_book("alpha", "Smith", "Jane", 2022, "Alpha Work"),
    );
    bib.insert(
        "zeta".to_string(),
        make_book("zeta", "Smith", "Jane", 2020, "Zeta Work"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.find("Zeta Work").unwrap() < result.find("Alpha Work").unwrap(),
        "author-date-title: Zeta (2020) must sort before Alpha (2022) — year is second key"
    );
}

fn label_processing_defaults_bibliography_to_author_date_title_order() {
    use citum_schema::options::LabelConfig;
    let style = build_processing_style(Processing::Label(LabelConfig::default()));

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "alpha".to_string(),
        make_book("alpha", "Smith", "Jane", 2022, "Alpha Work"),
    );
    bib.insert(
        "zeta".to_string(),
        make_book("zeta", "Smith", "Jane", 2020, "Zeta Work"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.find("Zeta Work").unwrap() < result.find("Alpha Work").unwrap(),
        "label processing: Zeta (2020) must sort before Alpha (2022) — defaults to author-date-title"
    );
}

fn note_processing_defaults_bibliography_to_author_title_date_order() {
    let style = build_processing_style(Processing::Note);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "zeta".to_string(),
        make_book("zeta", "Smith", "Jane", 2020, "Zeta Work"),
    );
    bib.insert(
        "alpha".to_string(),
        make_book("alpha", "Smith", "Jane", 2022, "Alpha Work"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(result.find("Alpha Work").unwrap() < result.find("Zeta Work").unwrap());
}

// --- Substitution Tests ---

fn subsequent_author_substitute_replaces_repeated_author_lines() {
    let style = make_style_with_substitute(Some("———".to_string()));

    let bib = citum_schema::bib_map![
        "ref1" => make_book("ref1", "Smith", "John", 2020, "Book A"),
        "ref2" => make_book("ref2", "Smith", "John", 2021, "Book B"),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // ref1 comes first (2020), then ref2 (2021). ref2 should have substituted author.
    // Note: Implicit separator ". " + Implicit suffix "."
    let expected = "Smith, John. 2020.\n\n———. 2021.";
    assert_eq!(result, expected);
}

fn magic_subsequent_author_substitute_reuses_the_full_author_group() {
    // Upstream provenance: CSL fixture `magic_SubsequentAuthorSubstitute`.
    let style = Style {
        info: StyleInfo {
            title: Some("Magic Subsequent Author Substitute Test".to_string()),
            id: Some("magic-subsequent-author-substitute-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::AuthorDate),
            contributors: Some(ContributorConfig {
                and: Some(AndOptions::Text),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Never),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                subsequent_author_substitute: Some("———".to_string()),
                ..Default::default()
            }),
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    citum_schema::tc_title!(Primary, prefix = ", "),
                    citum_schema::tc_date!(Issued, Year, prefix = " (", suffix = ")"),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let bib = citum_schema::bib_map![
        "item-1" => make_book_multi_author("item-1", vec![("Smith", "John"), ("Roe", "Jane")], 2000, "Book A"),
        "item-2" => make_book_multi_author("item-2", vec![("Smith", "John"), ("Roe", "Jane")], 2001, "Book B"),
        "item-3" => make_book("item-3", "Smith", "John", 2002, "Book C"),
    ];

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();
    assert_eq!(
        result,
        "John Smith and Jane Roe, Book A (2000)\n\n———, Book B (2001)\n\nJohn Smith, Book C (2002)"
    );
}

fn subsequent_author_substitute_does_not_apply_to_different_authors() {
    let style = make_style_with_substitute(Some("———".to_string()));

    let bib = citum_schema::bib_map![
        "ref1" => make_book("ref1", "Smith", "John", 2020, "Book A"),
        "ref2" => make_book("ref2", "Doe", "Jane", 2021, "Book B"),
    ];

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // Doe comes before Smith alphabetically
    let expected = "Doe, Jane. 2021.\n\nSmith, John. 2020.";
    assert_eq!(result, expected);
}

fn hyphenated_non_dropping_particles_sort_correctly_in_sort_order() {
    // Upstream provenance: CSL fixture `name_HyphenatedNonDroppingParticle1`.
    let style = make_name_particle_style(DisplayAsSort::All);

    let bib = citum_schema::bib_map![
        "ITEM-1" => make_particle_book("ITEM-1", "One", "Alan", Some("al-")),
        "ITEM-2" => make_particle_book("ITEM-2", "Marple", "Mary", None),
        "ITEM-3" => make_particle_book("ITEM-3", "Participle", "Paul", None),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "Marple, Mary\n\nOne, Alan al-\n\nParticiple, Paul");
}

fn hyphenated_non_dropping_particles_render_correctly_in_display_order() {
    // Upstream provenance: CSL fixture `name_HyphenatedNonDroppingParticle2`.
    let style = make_name_particle_style(DisplayAsSort::None);

    let bib = citum_schema::bib_map![
        "ITEM-1" => make_particle_book("ITEM-1", "One", "Alan", Some("al-")),
        "ITEM-2" => make_particle_book("ITEM-2", "Marple", "Mary", None),
        "ITEM-3" => make_particle_book("ITEM-3", "Participle", "Paul", None),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "Mary Marple\n\nAlan al-One\n\nPaul Participle");
}

fn make_two_author_and_style(
    delimiter_precedes_last: DelimiterPrecedesLast,
    name_order: Option<citum_schema::template::NameOrder>,
) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Two Author And Test".to_string()),
            id: Some("two-author-and-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            contributors: Some(ContributorConfig {
                and: Some(AndOptions::Text),
                delimiter_precedes_last: Some(delimiter_precedes_last),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Contributor(
                    citum_schema::template::TemplateContributor {
                        contributor: citum_schema::template::ContributorRole::Author.into(),
                        form: citum_schema::template::ContributorForm::Long,
                        name_order,
                        ..Default::default()
                    },
                )]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn two_names_bibliography_given_first_order_never_uses_delimiter() {
    // Given-first bibliography name lists (e.g. an editor/chair group) never
    // use the delimiter before the conjunction, regardless of the declared
    // delimiter-precedes-last value -- there is no per-component override
    // for this option today, and real styles (APA) rely on this suppression
    // for correct given-first-name-list formatting (e.g. "F. A. Editor &
    // S. Editor" rather than "F. A. Editor, & S. Editor"). See div-013 in
    // docs/adjudication/DIVERGENCE_REGISTER.md.
    let style = make_two_author_and_style(
        DelimiterPrecedesLast::Always,
        Some(citum_schema::template::NameOrder::GivenFirst),
    );
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_book_multi_author("ITEM-1", vec![("Smith", "John"), ("Jones", "Jane")], 2020, "Title"),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith and Jane Jones");
}

#[test]
fn two_names_bibliography_contextual_omits_delimiter_for_two_names() {
    // CSL's "contextual" delimiter-precedes-last means "delimiter only when
    // 3 or more names are joined"; for exactly two names no delimiter should
    // precede the conjunction. Previously this was hardcoded to `true`.
    let style = make_two_author_and_style(DelimiterPrecedesLast::Contextual, None);
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_book_multi_author("ITEM-1", vec![("Smith", "John"), ("Jones", "Jane")], 2020, "Title"),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith and Jane Jones");
}

// --- Numeric Bibliography Tests ---

fn numeric_bibliography_uses_assigned_citation_numbers() {
    let style = build_numeric_style();

    let bib =
        citum_schema::bib_map!["item1" => make_book("item1", "Smith", "John", 2020, "Title A")];
    let processor = Processor::new(style, bib);
    // Must process citation to assign number
    processor
        .process_citation(&citum_schema::cite!("item1"))
        .unwrap();

    let result = processor.render_bibliography();
    assert_eq!(result, "1. John Smith (2020)");
}

fn anonymous_works_sort_by_explicit_title_key_strips_leading_articles() {
    // SortKey::Title directly (not Author-key fallback) — verifies the Title-key contract.
    // "A Zen Perspective on Method" stripped → "Zen" (Z).
    // "The Academic Press Guide" stripped → "Academic" (A).
    // Without stripping: A < T → "A Zen…" first (wrong). With stripping: A < Z → Academic first.
    let style = build_title_year_sorted_style(vec![SortSpec {
        key: SortKey::Title,
        ascending: true,
    }]);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "zen".to_string(),
        make_book("zen", "", "", 2020, "A Zen Perspective on Method"),
    );
    bib.insert(
        "academic".to_string(),
        make_book("academic", "", "", 2020, "The Academic Press Guide"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.find("The Academic Press Guide").unwrap()
            < result.find("A Zen Perspective").unwrap(),
        "SortKey::Title must strip leading articles: 'Academic' (A) before 'Zen' (Z). Got:\n{result}"
    );
}

fn anonymous_works_sort_by_title_ignoring_leading_articles() {
    let style = build_title_year_sorted_style(vec![
        SortSpec {
            key: SortKey::Author,
            ascending: true,
        },
        SortSpec {
            key: SortKey::Year,
            ascending: true,
        },
    ]);

    let mut bib = indexmap::IndexMap::new();
    // Anonymous work with "The" article should sort as "Chicago Manual"
    bib.insert(
        "anon1".to_string(),
        make_book("anon1", "", "", 2018, "The Chicago Manual of Style"),
    );
    // Another anonymous work starting with title after article
    bib.insert(
        "anon2".to_string(),
        make_book("anon2", "", "", 2015, "A Guide to Citation"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // "The Chicago..." (C) should come BEFORE "A Guide..." (G) when articles are stripped
    assert!(result.find("The Chicago").unwrap() < result.find("A Guide").unwrap());
}

fn anonymous_works_with_the_same_year_still_sort_by_year_first() {
    let style = build_sorted_style(vec![
        SortSpec {
            key: SortKey::Author,
            ascending: true,
        },
        SortSpec {
            key: SortKey::Year,
            ascending: true,
        },
    ]);

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "anon1".to_string(),
        make_book("anon1", "", "", 2020, "The Chicago Manual"),
    );
    bib.insert(
        "anon2".to_string(),
        make_book("anon2", "", "", 2020, "An Earlier Publication"),
    );
    bib.insert(
        "anon3".to_string(),
        make_book("anon3", "", "", 2019, "The Chicago Manual"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    // 2019 entry should come before 2020 entries
    assert!(result.find("2019").unwrap() < result.find("2020").unwrap());
}

fn citation_key_tiebreaker_produces_deterministic_output_when_all_keys_equal() {
    // Two entries with identical author, year, and title — all configured sort keys compare Equal.
    // The citation-key tiebreaker must resolve the tie deterministically; repeated renders must
    // produce byte-identical output.
    let style = build_sorted_style(vec![
        SortSpec {
            key: SortKey::Author,
            ascending: true,
        },
        SortSpec {
            key: SortKey::Year,
            ascending: true,
        },
        SortSpec {
            key: SortKey::Title,
            ascending: true,
        },
    ]);

    let mut bib = indexmap::IndexMap::new();
    // Inserted in Z-then-A key order; tiebreaker should produce A-then-Z regardless.
    bib.insert(
        "zzz-last".to_string(),
        make_book("zzz-last", "Smith", "Jane", 2020, "Identical Title"),
    );
    bib.insert(
        "aaa-first".to_string(),
        make_book("aaa-first", "Smith", "Jane", 2020, "Identical Title"),
    );

    let processor = Processor::new(style, bib);
    let first = processor.render_bibliography();
    let second = processor.render_bibliography();

    // Both entries must appear.
    assert!(
        first.matches("Smith").count() >= 2,
        "both entries should appear in the bibliography. Got:\n{first}"
    );

    // Tiebreaker must be stable: identical output across calls.
    assert_eq!(
        first, second,
        "citation-key tiebreaker must produce deterministic output across repeated renders"
    );
}

fn article_journal_with_pages_keeps_standard_detail_block() {
    let style = build_article_journal_no_page_fallback_style();
    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "article-with-pages".to_string(),
        make_article_journal_with_detail(
            "article-with-pages",
            "2024",
            Some("3"),
            Some("101-109"),
            Some("10.1234/fallback"),
        ),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "Journal of Fallbacks, 2024, 12, (3), pp. 101–109");
}

fn page_less_article_journal_swaps_detail_block_for_doi() {
    let style = build_article_journal_no_page_fallback_style();
    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "article-without-pages".to_string(),
        make_article_journal_with_detail(
            "article-without-pages",
            "2024",
            Some("3"),
            None,
            Some("10.1234/fallback"),
        ),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "Journal of Fallbacks, DOI:10.1234/fallback");
}

fn type_variant_article_journal_fallback_preserves_variant_precedence() {
    let style_yaml = r#"
info:
  title: Type Variant Regression
  id: type-variant-regression
bibliography:
  options:
    article-journal:
      no-page-fallback: doi
  template:
    - title: primary
      prefix: "DEFAULT "
  type-variants:
    article-journal:
      - contributor: author
        form: long
      - number: volume
        prefix: ", "
      - number: pages
        prefix: ": "
      - variable: doi
        prefix: " DOI: "
"#;
    let style: Style = serde_yaml::from_str(style_yaml).expect("style should parse");

    let without_pages: csl_legacy::csl_json::Reference =
        serde_json::from_value(serde_json::json!({
            "id": "without-pages",
            "type": "article-journal",
            "title": "Fallback Title",
            "author": [{"family": "Smith", "given": "Jane"}],
            "issued": {"date-parts": [[2020]]},
            "volume": "12",
            "DOI": "10.1000/no-pages"
        }))
        .expect("no-pages fixture should parse");
    let with_pages: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "with-pages",
        "type": "article-journal",
        "title": "Detailed Title",
        "author": [{"family": "Jones", "given": "Alex"}],
        "issued": {"date-parts": [[2021]]},
        "volume": "18",
        "page": "33-40",
        "DOI": "10.1000/with-pages"
    }))
    .expect("with-pages fixture should parse");

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("without-pages".to_string(), without_pages.into());
    bibliography.insert("with-pages".to_string(), with_pages.into());

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "without-pages".to_string(),
        "with-pages".to_string(),
    ]);

    assert!(
        rendered.contains("Smith DOI: 10.1000/no-pages"),
        "DOI fallback should keep the type-variant output: {rendered}"
    );
    assert!(
        rendered.contains("Alex Jones, 18: 33–40"),
        "page-bearing articles should retain the detail block from the type variant: {rendered}"
    );
    assert!(
        !rendered.contains("DEFAULT Fallback Title")
            && !rendered.contains("DEFAULT Detailed Title"),
        "matching type variants must take precedence over the default bibliography template: {rendered}"
    );
    assert!(
        !rendered.contains("10.1000/with-pages"),
        "standard-detail mode should still suppress DOI when pages are present: {rendered}"
    );
}

fn anonymous_entry_type_variants_reorder_online_entries_and_drop_print_fallback_rows() {
    let style = build_anonymous_entry_policy_style();

    let online_encyclopedia = make_entry_reference(EntryReferenceParams {
        id: "online-encyclopedia",
        entry_type: "entry-encyclopedia",
        title: "Stevie Nicks",
        container_title: "Wikipedia",
        year: 2025,
        doi: None,
        url: Some("https://en.wikipedia.org/w/index.php?title=Stevie_Nicks&oldid=1279222290"),
        author: None,
    });
    let print_dictionary = make_entry_reference(EntryReferenceParams {
        id: "print-dictionary",
        entry_type: "entry-dictionary",
        title: "hootenanny, n.",
        container_title: "Oxford English Dictionary",
        year: 1976,
        doi: None,
        url: None,
        author: None,
    });
    let print_encyclopedia = make_entry_reference(EntryReferenceParams {
        id: "print-encyclopedia",
        entry_type: "entry-encyclopedia",
        title: "Feathers",
        container_title: "Johnson's Universal Cyclopaedia",
        year: 1886,
        doi: None,
        url: None,
        author: None,
    });
    let authorful_encyclopedia = make_entry_reference(EntryReferenceParams {
        id: "authorful-encyclopedia",
        entry_type: "entry-encyclopedia",
        title: "Ellington, Duke",
        container_title: "Grove Music Online",
        year: 2013,
        doi: Some("10.1093/gmo/9781561592630.article.A2249397"),
        url: None,
        author: Some(("Piras", "Marcello")),
    });

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("print-dictionary".to_string(), print_dictionary);
    bibliography.insert("print-encyclopedia".to_string(), print_encyclopedia);
    bibliography.insert("online-encyclopedia".to_string(), online_encyclopedia);
    bibliography.insert("authorful-encyclopedia".to_string(), authorful_encyclopedia);

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "print-dictionary".to_string(),
        "print-encyclopedia".to_string(),
        "online-encyclopedia".to_string(),
        "authorful-encyclopedia".to_string(),
    ]);

    assert_eq!(
        rendered,
        "Marcello Piras. Ellington, Duke. Grove Music Online. 2013. https://doi.org/10.1093/gmo/9781561592630.article.A2249397\n\nWikipedia. 2025. Stevie Nicks. https://en.wikipedia.org/w/index.php?title=Stevie_Nicks&oldid=1279222290",
        "anonymous print-like rows should be suppressed; authorful and online entries should render"
    );
}

#[test]
fn elsevier_harvard_entry_encyclopedia_uses_entry_template_instead_of_chapter_detail() {
    let style = load_style("styles/embedded/elsevier-harvard.yaml");
    let bibliography = citum_io::load_bibliography(
        &project_root().join("tests/fixtures/references-expanded.json"),
    )
    .expect("expanded bibliography should load");

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "ITEM-18".to_string(),
    ]);

    assert_eq!(
        rendered.trim(),
        "Vasari, G., 2022. Renaissance Art and Culture. Encyclopedia of World History."
    );
}

#[test]
fn apa_dataset_without_title_falls_back_to_bracketed_label_version_and_doi() {
    let style = load_style("styles/embedded/apa-7th.yaml");
    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "apa-titleless-dataset",
        "type": "dataset",
        "author": [{ "family": "Author", "given": "First A." }],
        "issued": { "date-parts": [[2013]] },
        "DOI": "10.1234/5678",
        "genre": "Untitled dataset",
        "version": "2.1",
        "language": "en"
    }))
    .expect("dataset fixture should parse");

    let mut bibliography = indexmap::IndexMap::new();
    bibliography.insert("apa-titleless-dataset".to_string(), legacy.into());

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "apa-titleless-dataset".to_string(),
    ]);

    assert_eq!(
        rendered.trim(),
        "Author, F. A. (2013). [Untitled dataset] (Version 2.1). https://doi.org/10.1234/5678"
    );
}

#[test]
fn apa_web_native_entries_render_without_retrieved_fallbacks() {
    let style = load_style("styles/embedded/apa-7th.yaml");
    let legacy_items = [
        serde_json::json!({
            "id": "6188419/IC98IKSD",
            "type": "webpage",
            "container-title": "Website title",
            "genre": "page type",
            "language": "en",
            "note": "part-number: 1\npart-title: Part title\neditor: Editor || A. A.",
            "title": "58 Web page",
            "URL": "https://example.com",
            "author": [{ "family": "Author", "given": "A. A." }],
            "translator": [{ "family": "Translator", "given": "A. A." }],
            "accessed": { "date-parts": [[2018, 7, 15]] },
            "issued": { "date-parts": [[2018]] }
        }),
        serde_json::json!({
            "id": "6188419/XA2MLUAS",
            "type": "post-weblog",
            "container-title": "Website title",
            "genre": "Type",
            "language": "en",
            "title": "59 Blog post",
            "URL": "https://example.com",
            "author": [{ "family": "Author", "given": "A. A." }],
            "accessed": { "date-parts": [[2018, 7, 15]] },
            "issued": { "date-parts": [[2018]] }
        }),
        serde_json::json!({
            "id": "6188419/HCFRWJZR",
            "type": "post",
            "container-title": "Website title",
            "genre": "Type",
            "language": "la",
            "title": "60 Forum post",
            "URL": "https://example.com",
            "author": [{ "family": "Author", "given": "A. A." }],
            "accessed": { "date-parts": [[2018, 7, 15]] },
            "issued": { "date-parts": [[2018]] }
        }),
    ];

    let bibliography = legacy_items
        .into_iter()
        .map(|value| {
            let legacy: csl_legacy::csl_json::Reference =
                serde_json::from_value(value).expect("fixture should parse");
            (legacy.id.clone(), legacy.into())
        })
        .collect();

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "6188419/IC98IKSD".to_string(),
        "6188419/XA2MLUAS".to_string(),
        "6188419/HCFRWJZR".to_string(),
    ]);

    assert_eq!(
        rendered,
        "Author, A. A. (2018a). 58 Web page: Pt. 1. Part title (A. A. Editor, ed.; A. A. Translator, Trans.) [Page type]. Website Title. https://example.com/\n\nAuthor, A. A. (2018b). 59 Blog post [Type]. Website Title. https://example.com/\n\nAuthor, A. A. (2018c). 60 Forum post [Type]. Website title. https://example.com/"
    );
}

#[test]
fn apa_magazine_and_newspaper_entries_keep_special_format_translators_and_direct_urls() {
    let style = load_style("styles/embedded/apa-7th.yaml");
    let legacy_items = [
        serde_json::json!({
            "id": "6188419/BXMWCMVJ",
            "type": "article-magazine",
            "container-title": "Journal Title",
            "ISSN": "0000-0000",
            "issue": "5",
            "language": "en",
            "note": "medium: special format\ngenre: type\nsection: department",
            "page": "1-100",
            "title": "15 Magazine article",
            "URL": "http://example.com",
            "volume": "32",
            "author": [{ "family": "Author", "given": "First A." }],
            "translator": [{ "family": "Translator", "given": "Third A." }],
            "issued": { "date-parts": [[2018, 7, 14]] }
        }),
        serde_json::json!({
            "id": "6188419/389M98AT",
            "type": "article-newspaper",
            "container-title": "Newspaper Title",
            "edition": "evening",
            "ISSN": "0000-0000",
            "language": "en",
            "note": "medium: Special format\ngenre: Type",
            "page": "1-100",
            "title": "17 Newspaper article",
            "URL": "http://example.com",
            "author": [{ "family": "Author", "given": "First A." }],
            "translator": [{ "family": "Translator", "given": "Third A." }],
            "issued": { "date-parts": [[2018, 7, 14]] }
        }),
    ];

    let bibliography = legacy_items
        .into_iter()
        .map(|value| {
            let legacy: csl_legacy::csl_json::Reference =
                serde_json::from_value(value).expect("fixture should parse");
            (legacy.id.clone(), legacy.into())
        })
        .collect();

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "6188419/BXMWCMVJ".to_string(),
        "6188419/389M98AT".to_string(),
    ]);

    assert_eq!(
        rendered,
        "Author, F. A. (2018a, July 14). 15 Magazine article (T. A. Translator, Trans.) [Type; Special format]. _Journal Title_, _32_(5), 1–100. http://example.com/\n\nAuthor, F. A. (2018b, July 14). 17 Newspaper article (T. A. Translator, Trans.) [Type; Special format]. _Newspaper Title_, 1–100. http://example.com/"
    );
}

#[test]
fn apa_structural_entries_use_component_packaging_instead_of_generic_fallbacks() {
    let style = load_style("styles/embedded/apa-7th.yaml");
    let legacy_items = [
        serde_json::json!({
            "id": "6188419/RYT8J733",
            "type": "report",
            "genre": "Technical report",
            "note": "container-title: Report title",
            "page": "126-145",
            "publisher": "Publisher",
            "publisher-place": "City, ST",
            "title": "24 Chapter in a report",
            "URL": "https://example.com",
            "author": [{ "family": "Chapter", "given": "Author M., Jr." }],
            "editor": [
                { "family": "Editor", "given": "First A." },
                { "family": "Editor", "given": "Second" }
            ],
            "number": "12345",
            "issued": { "date-parts": [[2016]] }
        }),
        serde_json::json!({
            "id": "6188419/Q2MWRA2D",
            "type": "entry-encyclopedia",
            "container-title": "Title of book: a subtitle",
            "DOI": "10.1234/5678",
            "edition": "2",
            "note": "original-date: 1901\noriginal-title: Original title\ncontainer-title-short: Title of book",
            "page": "123-128",
            "publisher": "Publisher",
            "publisher-place": "Place, ST",
            "title": "45 Encyclopedia entry",
            "URL": "http://example.com",
            "volume": "2",
            "translator": [{ "family": "Editor", "given": "S. S." }],
            "editor": [{ "family": "Editor", "given": "S. S." }],
            "author": [{ "family": "Author", "given": "First A." }],
            "issued": { "date-parts": [[2013]] }
        }),
        serde_json::json!({
            "id": "6188419/2G36L2LR",
            "type": "paper-conference",
            "container-title": "Proceedings",
            "DOI": "10.1234/5678",
            "note": "event-date: 2010",
            "page": "123-128",
            "publisher": "Publisher",
            "publisher-place": "Place, ST",
            "title": "56 Conference paper",
            "URL": "http://example.com",
            "volume": "2",
            "translator": [{ "family": "Editor", "given": "S. S." }],
            "editor": [{ "family": "Editor", "given": "S. S." }],
            "author": [{ "family": "Author", "given": "First A." }],
            "issued": { "date-parts": [[2013]] }
        }),
    ];

    let bibliography = legacy_items
        .into_iter()
        .map(|value| {
            let legacy: csl_legacy::csl_json::Reference =
                serde_json::from_value(value).expect("fixture should parse");
            (legacy.id.clone(), legacy.into())
        })
        .collect();

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "6188419/RYT8J733".to_string(),
        "6188419/Q2MWRA2D".to_string(),
        "6188419/2G36L2LR".to_string(),
    ]);

    assert_eq!(
        rendered,
        "Author, F. A. (2013a). 45 Encyclopedia entry (S. S. Editor, Trans.). In S. S. Editor, ed., _Title of book: a subtitle_ (2 ed., Vol. 2, pp. 123–128). Publisher. https://doi.org/10.1234/5678 http://example.com/\n\nAuthor, F. A. (2013b). 56 Conference paper (S. S. Editor, Trans.). In S. S. Editor, ed., _Proceedings_ (Vol. 2, pp. 123–128). Publisher. https://doi.org/10.1234/5678 http://example.com/\n\nChapter, A. M. J. (2016). 24 Chapter in a report. In F. A. Editor & S. Editor (eds.), _Report title_ (pp. 126–145). Publisher. https://example.com/"
    );
}

#[test]
fn apa_containerless_translated_chapter_avoids_rendering_an_empty_in_group() {
    let legacy = serde_json::json!({
        "id": "6188419/4JYXEPMY",
        "type": "chapter",
        "DOI": "10.1234/5678",
        "edition": "2",
        "language": "en",
        "note": "original-title: Original title\ncontainer-title-short: Title of book",
        "number-of-volumes": "3",
        "page": "123-128",
        "publisher": "Publisher",
        "publisher-place": "Place, ST",
        "title": "27a Book chapter",
        "URL": "http://example.com",
        "volume": "2",
        "translator": [{ "family": "Editor", "given": "S. S." }],
        "editor": [{ "family": "Editor", "given": "S. S." }],
        "author": [{ "family": "Author", "given": "First A." }],
        "issued": { "date-parts": [[2013]] },
        "original-date": { "date-parts": [[1901]] }
    });

    let rendered = render_structural_bibliography_case(legacy);

    assert_eq!(
        rendered,
        "Author, F. A. (2013). 27a Book chapter (S. S. Editor, Trans.). In S. S. Editor (ed.) (2 ed., Vol. 2, pp. 123–128). Publisher. https://doi.org/10.1234/5678 http://example.com/"
    );
}

struct StructuralBibliographyCase {
    legacy: serde_json::Value,
    expected_contains: &'static [&'static str],
    expected_omits: &'static [&'static str],
}

fn render_structural_bibliography_case(value: serde_json::Value) -> String {
    let style = load_style("styles/embedded/apa-7th.yaml");
    let legacy: csl_legacy::csl_json::Reference =
        serde_json::from_value(value).expect("fixture should parse");
    let id = legacy.id.clone();
    let bibliography = IndexMap::from([(id.clone(), legacy.into())]);
    let processor = Processor::new(style, bibliography);

    processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>([id])
        .lines()
        .find(|line| !line.trim().is_empty())
        .expect("expected one bibliography line")
        .to_string()
}

fn chapter_structural_case() -> StructuralBibliographyCase {
    StructuralBibliographyCase {
        legacy: serde_json::json!({
            "id": "6188419/2QK89T6V",
            "type": "chapter",
            "container-title": "Title of book: a subtitle",
            "DOI": "10.1234/5678",
            "edition": "2",
            "language": "en",
            "note": "original-title: Original title\ncontainer-title-short: Title of book\npart-number: 2",
            "number-of-volumes": "3",
            "page": "123-128",
            "publisher": "Publisher",
            "publisher-place": "Place, ST",
            "title": "27 Book chapter",
            "URL": "http://example.com",
            "volume": "2",
            "translator": [{ "family": "Editor", "given": "S. S." }],
            "editor": [{ "family": "Editor", "given": "S. S." }],
            "author": [{ "family": "Author", "given": "First A." }],
            "container-author": [{ "family": "Author", "given": "C." }],
            "issued": { "date-parts": [[2013]] }
        }),
        expected_contains: &["In C. Author", "Title of book: a subtitle"],
        expected_omits: &[", S. S. Editor, ed.,"],
    }
}

fn event_structural_case() -> StructuralBibliographyCase {
    StructuralBibliographyCase {
        legacy: serde_json::json!({
            "id": "6188419/QUB9VPFI",
            "type": "speech",
            "container-title": "Session title",
            "event-title": "Society for Industrial and Organizational Psychology conference",
            "genre": "Symposium",
            "language": "en",
            "note": "type: event",
            "publisher-place": "City, ST",
            "title": "33 Conference presentation is a session",
            "URL": "http://www.example.com",
            "author": [{ "family": "Author", "given": "First" }],
            "chair": [
                { "family": "Chair", "given": "First" },
                { "family": "Chair", "given": "Second" }
            ],
            "issued": { "date-parts": [[2013, 5]] }
        }),
        expected_contains: &[
            "2013",
            "May",
            "In F. Chair & S. Chair",
            "Session title [Symposium].",
        ],
        expected_omits: &[],
    }
}

fn preprint_structural_case() -> StructuralBibliographyCase {
    StructuralBibliographyCase {
        legacy: serde_json::json!({
            "id": "6188419/5VSYNLFP",
            "type": "article",
            "language": "en",
            "note": "Medium: Format",
            "number": "123445",
            "publisher": "PsyArXiv",
            "title": "9 Preprint with archive",
            "author": [{ "family": "Author", "given": "A. A." }],
            "editor": [{ "family": "Editor", "given": "A. A." }],
            "translator": [{ "family": "Translator", "given": "A. A." }],
            "issued": { "date-parts": [[2018]] }
        }),
        expected_contains: &["9 Preprint with archive", "No. 123445"],
        expected_omits: &["_9 Preprint with archive_"],
    }
}

#[rstest]
#[case::chapter(chapter_structural_case())]
#[case::event(event_structural_case())]
#[case::preprint(preprint_structural_case())]
fn given_an_apa_structural_fixture_when_rendering_bibliography_then_expected_components_survive(
    #[case] case: StructuralBibliographyCase,
) {
    let rendered = render_structural_bibliography_case(case.legacy);

    for needle in case.expected_contains {
        assert!(rendered.contains(needle), "{rendered}");
    }

    for needle in case.expected_omits {
        assert!(!rendered.contains(needle), "{rendered}");
    }
}

#[test]
fn apa_personal_communication_entries_do_not_render_in_bibliography() {
    let style = load_style("styles/embedded/apa-7th.yaml");
    let bibliography = IndexMap::from([
        (
            "ITEM-28".to_string(),
            InputReference::from(
                serde_json::from_str::<csl_legacy::csl_json::Reference>(
                    r#"{
                        "id": "ITEM-28",
                        "type": "personal_communication",
                        "title": "Discussion on Citum Schema Design",
                        "author": [{"family": "Smith", "given": "Patricia"}],
                        "issued": {"date-parts": [[2024, 2, 7]]},
                        "recipient": [{"family": "Darcus", "given": "Bruce"}]
                    }"#,
                )
                .expect("fixture should parse"),
            ),
        ),
        (
            "sr-recipient".to_string(),
            InputReference::from(
                serde_json::from_str::<csl_legacy::csl_json::Reference>(
                    r#"{
                        "id": "sr-recipient",
                        "type": "personal_communication",
                        "title": "Letter to Colleague",
                        "author": [{"family": "Morrison", "given": "Toni"}],
                        "recipient": [{"family": "Walker", "given": "Alice"}],
                        "issued": {"date-parts": [[1983, 5, 12]]},
                        "genre": "letter"
                    }"#,
                )
                .expect("fixture should parse"),
            ),
        ),
    ]);

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_selected_bibliography_with_format_standalone::<PlainText, _>([
        "ITEM-28".to_string(),
        "sr-recipient".to_string(),
    ]);

    assert!(rendered.trim().is_empty(), "{rendered}");
}

fn bibliography_local_entry_links_apply_on_the_default_render_path() {
    let style = build_bibliography_entry_link_style();
    let reference = InputReference::Monograph(Box::new(Monograph {
        short_title: None,
        id: Some("linked-book".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Linked Book".to_string())),
        container: None,
        author: None,
        editor: None,
        translator: None,
        issued: DateValue::new("2024".to_string()),
        publisher: None,
        url: Some(Url::parse("https://example.com/linked-book").expect("valid url")),
        accessed: None,
        language: None,
        field_languages: Default::default(),
        note: None,
        isbn: None,
        doi: None,
        numbering: Default::default(),
        genre: None,
        medium: None,
        archive_info: None,
        eprint: None,
        archive: None,
        archive_location: None,
        keywords: None,
        original: None,
        ads_bibcode: None,
        ..Default::default()
    }));
    let bib = citum_schema::bib_map!["linked-book" => reference];

    let processor = Processor::new(style, bib);
    let rendered = processor.render_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains(r#"href="https://example.com/linked-book""#),
        "bibliography-local entry link should be applied on the default path: {rendered}"
    );
}

fn bibliography_local_processing_changes_default_bibliography_sort() {
    let style = build_bibliography_local_note_sort_style();
    let bib = citum_schema::bib_map![
        "book-b" => make_book("book-b", "Zimmer", "Zed", 2021, "Later Book"),
        "book-a" => make_book("book-a", "Adams", "Amy", 2020, "Earlier Book")
    ];

    let processor = Processor::new(style, bib);
    let rendered = processor.render_bibliography();

    assert!(
        rendered.find("Amy Adams").unwrap() < rendered.find("Zed Zimmer").unwrap(),
        "bibliography-local processing should drive default bibliography sort: {rendered}"
    );
}

fn bibliography_local_numeric_processing_assigns_bibliography_numbers() {
    let style = build_bibliography_local_numeric_style();
    let bib = citum_schema::bib_map![
        "book-a" => make_book("book-a", "Smith", "John", 2020, "Title A"),
        "book-b" => make_book("book-b", "Brown", "Beth", 2021, "Title B")
    ];

    let processor = Processor::new(style, bib);
    let rendered = processor.render_bibliography();

    assert_eq!(
        rendered, "1. John Smith\n\n2. Beth Brown",
        "bibliography-local numeric processing should assign bibliography numbers"
    );
}

fn numeric_citations_follow_bibliography_local_sort_when_assigning_numbers() {
    let style = build_numeric_citation_style_with_bibliography_local_note_sort();
    let bib = citum_schema::bib_map![
        "book-b" => make_book("book-b", "Zimmer", "Zed", 2021, "Later Book"),
        "book-a" => make_book("book-a", "Adams", "Amy", 2020, "Earlier Book")
    ];

    let processor = Processor::new(style, bib);
    let citation = processor
        .process_citation(&citum_schema::cite!("book-b"))
        .expect("citation should render");
    let bibliography = processor.render_bibliography();

    assert_eq!(
        citation, "[2]",
        "citation numbering should follow bibliography-local sort order"
    );
    assert!(
        bibliography.find("1. Amy Adams").unwrap() < bibliography.find("2. Zed Zimmer").unwrap(),
        "bibliography numbering should stay aligned with citation numbering: {bibliography}"
    );
}

#[test]
fn nested_inline_article_journal_detail_group_renders_issue_and_parenthesized_year_month() {
    let style = build_inline_article_journal_detail_group_style();

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "article-inline-detail".to_string(),
        make_article_journal_with_detail(
            "article-inline-detail",
            "2024-01",
            Some("3"),
            Some("1-12"),
            None,
        ),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.contains("12, 3 (January 2024), pp. 1–12"),
        "expected nested inline detail group rendering, got {result}"
    );
}

#[test]
fn nested_inline_article_journal_detail_group_suppresses_missing_issue_without_extra_spacing() {
    let style = build_inline_article_journal_detail_group_style();

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "article-inline-detail-no-issue".to_string(),
        make_article_journal_with_detail(
            "article-inline-detail-no-issue",
            "2024-01",
            None,
            Some("1-12"),
            None,
        ),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.contains("12, (January 2024), pp. 1–12"),
        "expected clean suppression around missing issue, got {result}"
    );
}

fn royal_society_of_chemistry_restores_legacy_page_less_doi_behavior() {
    let style = load_style("styles/royal-society-of-chemistry.yaml");
    let bib = citum_io::load_bibliography(
        &project_root().join("tests/fixtures/references-expanded.json"),
    )
    .expect("expanded bibliography should load");
    let processor = Processor::new(style, bib);
    let result = processor
        .render_selected_bibliography_with_format_standalone::<citum_engine::render::plain::PlainText, _>(
            vec!["ITEM-1".to_string()],
        );

    // The leading `1` is the reference marker: the CSL bibliography layout
    // renders citation-number for every entry (second-field-align="flush"),
    // which the style now declares once via bibliography label-mode rather
    // than authoring on six type-variants. See docs/specs/REFERENCE_MARKERS.md.
    assert_eq!(
        result,
        "1T. S. Kuhn, _International Encyclopedia of Unified Science_, DOI:10.1234/example."
    );
}

#[test]
fn editor_author_substitute_omits_verb_role_label_in_bibliography() {
    let mut style = load_style("styles/embedded/apa-7th.yaml");
    let config = style.options.get_or_insert_with(Default::default);
    let contributors = config.contributors.get_or_insert_with(Default::default);
    contributors.role = Some(citum_schema::options::contributors::RoleOptions {
        preset: Some(citum_schema::options::contributors::RoleLabelPreset::VerbPrefix),
        ..Default::default()
    });

    let bib = make_editor_substitute_bibliography();
    let processor = Processor::new(style, bib);
    let result = processor
        .render_selected_bibliography_with_format_standalone::<citum_engine::render::plain::PlainText, _>(
            vec!["ancient-tale".to_string(), "ipcc2023".to_string()],
        );

    assert!(
        result.contains("Grimm, J. (1850). _The Ancient Tale_"),
        "editor substitute should render as the effective author without a verb label: {result}"
    );
    assert!(
        result.contains("Lee, H., & Romero, J. (2023). _Climate Change 2023: Synthesis Report_"),
        "multi-editor substitute should render as names only when occupying the author slot: {result}"
    );
    assert!(
        !result.contains("edited by Jacob Grimm") && !result.contains("edited by Hoesung Lee"),
        "verb-prefix labels should not survive when editors substitute into the author slot: {result}"
    );
}

#[test]
fn editor_author_substitute_renders_comma_short_capitalized_label() {
    // given an editor-substitute style configured for comma+short labels with
    // a capitalize-first transform (the IEEE/AMA shape)
    let mut style = load_style("styles/embedded/apa-7th.yaml");
    let config = style.options.get_or_insert_with(Default::default);
    config.substitute = Some(citum_schema::options::SubstituteConfig::Explicit(
        citum_schema::options::Substitute {
            contributor_role_form: Some("short-comma".to_string()),
            contributor_role_case: Some(citum_schema::options::titles::TextCase::CapitalizeFirst),
            template: vec![citum_schema::options::SubstituteKey::Editor],
            ..Default::default()
        },
    ));

    let bib = make_editor_substitute_bibliography();
    let processor = Processor::new(style, bib);
    let result = processor
        .render_selected_bibliography_with_format_standalone::<citum_engine::render::plain::PlainText, _>(
            vec!["ancient-tale".to_string(), "ipcc2023".to_string()],
        );

    // then the substitute label is comma-joined, short, and capitalized
    assert!(
        result.contains("Grimm, J., Ed. (1850). _The Ancient Tale_"),
        "single editor substitute should render `, Ed.` (comma + short + capitalized): {result}"
    );
    assert!(
        result.contains("Lee, H., & Romero, J., Eds. (2023)."),
        "multi-editor substitute should render `, Eds.` (comma + short + capitalized): {result}"
    );
    assert!(
        !result.contains("(eds.)") && !result.contains("(Eds.)"),
        "the parenthesised short form must not appear for the comma preset: {result}"
    );
}

#[test]
fn sentence_initial_editor_verb_prefix_is_capitalized_in_bibliography() {
    let style = build_editor_verb_prefix_style(None);
    let mut bib = IndexMap::new();
    bib.insert(
        "editor-only".to_string(),
        make_editor_only_book("editor-only", "Collected Essays", "2001", "Grimm", "Jacob"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.contains("Collected Essays. Edited by Jacob Grimm"),
        "sentence-initial editor labels should capitalize after a period: {result}"
    );
}

#[test]
fn mid_sentence_editor_verb_prefix_remains_lowercase_in_bibliography() {
    let style = build_editor_verb_prefix_style(Some(":"));
    let mut bib = IndexMap::new();
    bib.insert(
        "editor-only".to_string(),
        make_editor_only_book("editor-only", "Collected Essays", "2001", "Grimm", "Jacob"),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert!(
        result.contains("Collected Essays: edited by Jacob Grimm"),
        "mid-sentence editor labels should stay lowercase: {result}"
    );
}

#[rstest]
#[case::plain(TestOutputFormat::Plain, "_the", "_The collected essays_")]
#[case::html(TestOutputFormat::Html, "<Em>the", "<em>The collected essays</em>")]
#[case::latex(TestOutputFormat::Latex, "\\Emph{the", "\\emph{The collected essays}")]
#[case::typst(TestOutputFormat::Typst, "#Emph[the", "#emph[The collected essays]")]
fn given_pre_formatted_emph_group_as_sentence_initial_when_rendering_bibliography_then_markup_is_not_corrupted(
    #[case] format: TestOutputFormat,
    #[case] must_not_contain: &str,
    #[case] must_contain: &str,
) {
    let style = build_sentence_initial_emph_group_style();
    let mut bib = IndexMap::new();
    bib.insert(
        "item".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("item".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("the collected essays".to_string())),
            ..Default::default()
        })),
    );
    let processor = Processor::new(style, bib);
    let rendered = render_bibliography_in_format(&processor, format);

    assert!(
        rendered.contains(must_contain),
        "sentence-initial group should capitalize first word without corrupting markup: {rendered}"
    );
    assert!(
        !rendered.contains(must_not_contain),
        "sentence-initial group must not corrupt markup tag names: {rendered}"
    );
}

#[test]
fn given_archive_info_and_eprint_when_rendering_bibliography_then_new_variables_resolve() {
    let style = build_archive_eprint_style();
    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "archive-eprint-ref".to_string(),
        make_archive_eprint_reference(),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(
        result,
        "Archive-Aware Preprint. Houghton Library, Ada Lovelace Papers, MS Am 1280, Series Correspondence, Box 12, Folder 4, Item 7, collection Ada Lovelace Papers (MS Am 1280), series Correspondence, box 12, folder 4, item 7, Cambridge, MA, https://example.com/archive, arxiv:2602.01234 [cs.DL]"
    );
}

#[test]
fn given_archive_location_override_when_rendering_bibliography_then_legacy_fallback_still_works() {
    let style = build_archive_location_fallback_style();
    let mut bib = indexmap::IndexMap::new();
    let mut reference = make_archive_eprint_reference();

    let ClassExtension::Monograph(monograph) = reference.extension_mut() else {
        panic!("archive test fixture should be a monograph");
    };
    monograph.id = Some("archive-eprint-location-ref".into());
    monograph.archive_info = Some(ArchiveInfo {
        name: Some(MultilingualString::Simple("Houghton Library".to_string())),
        place: Some("Cambridge, MA".into()),
        collection: Some("Ada Lovelace Papers".to_string()),
        location: Some("MS Am 1280, Box 12, Folder 4".to_string()),
        url: Some(Url::parse("https://example.com/archive").expect("url should parse")),
        ..Default::default()
    });
    bib.insert("archive-eprint-location-ref".to_string(), reference);

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(
        result,
        "Archive-Aware Preprint. Houghton Library, Ada Lovelace Papers, MS Am 1280, Box 12, Folder 4, Cambridge, MA, https://example.com/archive, arxiv:2602.01234 [cs.DL]"
    );
}

#[test]
fn given_multilingual_archive_name_when_rendering_then_name_mode_controls_archive_name() {
    let style = build_multilingual_archive_name_style();
    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "archive-name-ref".to_string(),
        make_multilingual_archive_name_reference(),
    );

    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "Tokyo National Museum");
}

#[test]
fn given_legacy_archive_fields_when_converting_then_archive_info_is_hydrated() {
    let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(serde_json::json!({
        "id": "ITEM-ARCHIVE-1",
        "type": "manuscript",
        "title": "Commonplace Book",
        "issued": { "date-parts": [[1720]] },
        "archive": "Bodleian Library",
        "archive_location": "MS Bodl. Or. 579, fol. 23r"
    }))
    .expect("legacy reference should parse");

    let reference: InputReference = legacy.into();
    if let ClassExtension::Monograph(m) = reference.extension() {
        assert_eq!(m.archive, Some("Bodleian Library".to_string()));
        assert_eq!(
            m.archive_location,
            Some("MS Bodl. Or. 579, fol. 23r".to_string())
        );

        let archive_info = m
            .archive_info
            .clone()
            .expect("archive info should be hydrated");
        assert_eq!(
            archive_info
                .name
                .expect("archive name should exist")
                .to_string(),
            "Bodleian Library"
        );
        assert_eq!(
            archive_info.location,
            Some("MS Bodl. Or. 579, fol. 23r".to_string())
        );
    } else {
        panic!("Expected Monograph");
    }
}

mod sorting {
    use super::announce_behavior;

    #[test]
    fn author_sorting_orders_entries_alphabetically() {
        announce_behavior(
            "A bibliography sorted by author should place entries in alphabetical family-name order.",
        );
        super::sorting_by_author_orders_entries_alphabetically();
    }

    #[test]
    fn year_sorting_places_earlier_years_first() {
        announce_behavior(
            "A bibliography sorted by year should place earlier years before later years.",
        );
        super::sorting_by_year_places_earlier_years_first();
    }

    #[test]
    fn empty_dates_move_undated_entries_after_dated_ones() {
        announce_behavior(
            "When sorting by year, undated bibliography entries should fall after the dated entries.",
        );
        super::sorting_empty_dates_pushes_undated_items_after_dated_ones();
    }

    #[test]
    fn multiple_sort_keys_apply_secondary_ordering_inside_author_groups() {
        announce_behavior(
            "Multiple bibliography sort keys should apply the secondary key within an author group.",
        );
        super::sorting_multiple_keys_applies_secondary_ordering_within_author_groups();
    }

    #[test]
    fn author_date_processing_uses_author_date_title_as_the_default_bibliography_sort() {
        announce_behavior(
            "Author-date processing should default bibliography ordering to author, then date, then title.",
        );
        super::author_date_processing_defaults_bibliography_to_author_date_title_order();
    }

    #[test]
    fn label_processing_uses_author_date_title_as_the_default_bibliography_sort() {
        announce_behavior(
            "Label processing should default bibliography ordering to author, then date, then title.",
        );
        super::label_processing_defaults_bibliography_to_author_date_title_order();
    }

    #[test]
    fn note_processing_uses_author_title_date_as_the_default_bibliography_sort() {
        announce_behavior(
            "Note-style processing should default bibliography ordering to author, then title, then date.",
        );
        super::note_processing_defaults_bibliography_to_author_title_date_order();
    }

    #[test]
    fn anonymous_works_sort_by_explicit_title_key_strips_leading_articles() {
        announce_behavior(
            "SortKey::Title strips leading articles so 'The Academic…' (A) sorts before 'A Zen…' (Z).",
        );
        super::anonymous_works_sort_by_explicit_title_key_strips_leading_articles();
    }

    #[test]
    fn anonymous_titles_ignore_leading_articles_during_sorting() {
        announce_behavior(
            "Anonymous bibliography entries should ignore leading articles like The or A when sorting by title.",
        );
        super::anonymous_works_sort_by_title_ignoring_leading_articles();
    }

    #[test]
    fn anonymous_same_year_entries_keep_years_in_order_before_tiebreaks() {
        announce_behavior(
            "Anonymous entries should still respect year ordering before applying same-year tiebreakers.",
        );
        super::anonymous_works_with_the_same_year_still_sort_by_year_first();
    }

    #[test]
    fn citation_key_tiebreaker_is_deterministic_when_all_sort_keys_compare_equal() {
        announce_behavior(
            "When all sort keys compare Equal, the citation-key tiebreaker must produce the same output on every render call.",
        );
        super::citation_key_tiebreaker_produces_deterministic_output_when_all_keys_equal();
    }
}

mod title_short_resolution {
    use super::announce_behavior;

    #[test]
    fn journal_abbreviations_populate_container_title_short() {
        announce_behavior(
            "A journal abbreviation should populate container-title-short in bibliography rendering.",
        );
        super::container_title_short_uses_journal_abbreviation_when_present();
    }

    #[test]
    fn explicit_container_title_short_fields_take_precedence() {
        announce_behavior(
            "An explicit container-title-short field should take precedence over the long container title.",
        );
        super::container_title_short_prefers_explicit_short_field();
    }

    #[test]
    fn suppressed_group_children_do_not_leave_stray_delimiters() {
        announce_behavior(
            "Suppressed children inside grouped bibliography components should not leave stray delimiters.",
        );
        super::suppressed_group_children_do_not_leave_stray_delimiters();
    }

    #[test]
    fn status_variables_render_in_bibliography_templates() {
        announce_behavior(
            "Bibliography templates should render status variables when the reference carries publication-status metadata.",
        );
        super::status_variables_render_in_bibliography_templates();
    }
}

mod substitution {
    use super::announce_behavior;

    #[test]
    fn repeated_authors_can_be_replaced_with_the_substitute_marker() {
        announce_behavior(
            "Repeated bibliography authors should be replaced by the configured subsequent-author substitute marker.",
        );
        super::subsequent_author_substitute_replaces_repeated_author_lines();
    }

    #[test]
    fn repeated_multi_author_groups_can_reuse_the_substitute_marker() {
        announce_behavior(
            "Repeated multi-author bibliography entries should reuse the substitute marker for the full repeated author group.",
        );
        super::magic_subsequent_author_substitute_reuses_the_full_author_group();
    }

    #[test]
    fn different_authors_do_not_trigger_the_substitute_marker() {
        announce_behavior(
            "Different bibliography authors should never trigger the subsequent-author substitute marker.",
        );
        super::subsequent_author_substitute_does_not_apply_to_different_authors();
    }
}

mod contributor_particles {
    use super::announce_behavior;

    #[test]
    fn hyphenated_particles_sort_correctly_in_sort_order() {
        announce_behavior(
            "Hyphenated non-dropping particles should sort correctly when contributor names are rendered in sort order.",
        );
        super::hyphenated_non_dropping_particles_sort_correctly_in_sort_order();
    }

    #[test]
    fn hyphenated_particles_render_correctly_in_display_order() {
        announce_behavior(
            "Hyphenated non-dropping particles should stay attached correctly when contributor names are rendered in display order.",
        );
        super::hyphenated_non_dropping_particles_render_correctly_in_display_order();
    }
}

mod article_journal_no_page_fallback {
    use super::announce_behavior;

    #[test]
    fn journal_articles_with_pages_keep_the_standard_detail_block() {
        announce_behavior(
            "A bibliography article-journal entry with pages should keep the standard detail block and suppress the DOI fallback path.",
        );
        super::article_journal_with_pages_keeps_standard_detail_block();
    }

    #[test]
    fn page_less_journal_articles_can_swap_the_detail_block_for_a_doi() {
        announce_behavior(
            "A bibliography article-journal entry without pages should swap its normal year-volume-pages detail block for DOI output when the style opts in.",
        );
        super::page_less_article_journal_swaps_detail_block_for_doi();
    }

    #[test]
    fn matching_type_variants_keep_precedence_when_article_journal_fallbacks_apply() {
        announce_behavior(
            "A matching article-journal type variant should keep precedence over the default bibliography template even when the no-page DOI fallback toggles between detail and DOI output.",
        );
        super::type_variant_article_journal_fallback_preserves_variant_precedence();
    }

    #[test]
    fn royal_society_of_chemistry_restores_the_legacy_page_less_doi_behavior() {
        announce_behavior(
            "The Royal Society of Chemistry bibliography should restore the legacy page-less journal behavior by rendering DOI instead of the standard detail block.",
        );
        super::royal_society_of_chemistry_restores_legacy_page_less_doi_behavior();
    }
}

mod anonymous_entry_type_variants {
    use super::announce_behavior;

    #[test]
    fn chicago_like_anonymous_entries_prefer_online_container_rows_and_drop_print_fallbacks() {
        announce_behavior(
            "Chicago-like anonymous dictionary and encyclopedia bibliography variants should keep online entries in container-led order while omitting the print-style fallback rows that the oracle does not emit.",
        );
        super::anonymous_entry_type_variants_reorder_online_entries_and_drop_print_fallback_rows();
    }
}

mod numeric_styles {
    use super::announce_behavior;

    #[test]
    fn numeric_bibliographies_use_the_assigned_citation_number() {
        announce_behavior(
            "A numeric bibliography should reuse the citation number assigned during citation rendering.",
        );
        super::numeric_bibliography_uses_assigned_citation_numbers();
    }

    #[test]
    fn bibliography_local_numeric_processing_assigns_numbers_on_the_default_path() {
        announce_behavior(
            "Bibliography-local numeric processing should assign citation numbers on the default bibliography render path even when the top-level style is not numeric.",
        );
        super::bibliography_local_numeric_processing_assigns_bibliography_numbers();
    }

    #[test]
    fn numeric_citations_follow_the_bibliography_local_sort_order() {
        announce_behavior(
            "Numeric citation numbering should stay aligned with bibliography-local sort rules when the bibliography overrides the processing family.",
        );
        super::numeric_citations_follow_bibliography_local_sort_when_assigning_numbers();
    }
}

mod local_overrides {
    use super::announce_behavior;

    #[test]
    fn bibliography_local_entry_links_apply_on_the_default_render_path() {
        announce_behavior(
            "Bibliography-local shared overrides should apply on the default bibliography render path.",
        );
        super::bibliography_local_entry_links_apply_on_the_default_render_path();
    }

    #[test]
    fn bibliography_local_processing_can_change_the_default_sort_order() {
        announce_behavior(
            "Bibliography-local processing should control the default bibliography sort order when no explicit bibliography sort is declared.",
        );
        super::bibliography_local_processing_changes_default_bibliography_sort();
    }
}

#[test]
fn original_published_date_variable_renders_when_reference_has_original_date() {
    let style = Style {
        info: StyleInfo {
            title: Some("Original-date test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::OriginalPublished,
                        form: DateForm::Year,
                        rendering: Rendering {
                            prefix: Some("(".into()),
                            suffix: Some(") ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::Issued,
                        form: DateForm::Year,
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        form: Some(TitleForm::Long),
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("gatsby".into()),
        title: Some(Title::Single("The Great Gatsby".to_string())),
        issued: DateValue::new("1992".to_string()),
        original: Some(WorkRelation::Embedded(Box::new(InputReference::Monograph(
            Box::new(Monograph {
                id: Some("gatsby-orig".into()),
                issued: DateValue::new("1925".to_string()),
                ..Default::default()
            }),
        )))),
        ..Default::default()
    }));

    let bibliography = IndexMap::from([("gatsby".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(["gatsby".to_string()])
        .trim()
        .to_string();

    assert_eq!(rendered, "(1925) 1992. The Great Gatsby");
}

#[test]
fn original_published_date_variable_renders_for_patent_references() {
    let style = Style {
        info: StyleInfo {
            title: Some("Original-date patent test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::OriginalPublished,
                        form: DateForm::Year,
                        rendering: Rendering {
                            prefix: Some("(".into()),
                            suffix: Some(") ".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::Issued,
                        form: DateForm::Year,
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        form: Some(TitleForm::Long),
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference: InputReference = serde_json::from_str(
        r#"{
            "class": "patent",
            "id": "patent",
            "title": "Improved Widget",
            "patent-number": "US-123",
            "issued": "1992",
            "original": {
                "class": "monograph",
                "type": "book",
                "id": "patent-orig",
                "issued": "1901"
            }
        }"#,
    )
    .unwrap();

    let bibliography = IndexMap::from([("patent".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(["patent".to_string()])
        .trim()
        .to_string();

    assert_eq!(rendered, "(1901) 1992. Improved Widget");
}

/// Builds an `original` sub-reference JSON fragment with an optional publisher
/// name and/or publisher place, matching the shape produced by the CMOS 18
/// reprint fixtures (e.g. a reprint with only an original publisher, or only
/// an original publisher place).
fn original_publisher_fragment(
    original_publisher: Option<&str>,
    original_publisher_place: Option<&str>,
) -> serde_json::Value {
    let mut original = serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "orig",
        "issued": "1900",
    });
    let publisher = match (original_publisher, original_publisher_place) {
        (Some(name), Some(place)) => Some(serde_json::json!({ "name": name, "place": place })),
        (Some(name), None) => Some(serde_json::json!({ "name": name })),
        (None, Some(place)) => Some(serde_json::json!({ "name": "", "place": place })),
        (None, None) => None,
    };
    if let Some(publisher) = publisher {
        original
            .as_object_mut()
            .expect("original fragment is an object")
            .insert("publisher".to_string(), publisher);
    }
    original
}

#[rstest]
#[case::original_publisher_present(Some("Kodansha"), None, "10.1000/marker-doi")]
#[case::original_publisher_place_present_alone(
    None,
    Some("Oxford"),
    "https://example.test/marker-url"
)]
#[case::neither_present(None, None, "")]
fn given_original_publication_fields_when_a_bibliography_group_checks_field_presence_then_it_renders_conditionally(
    #[case] original_publisher: Option<&str>,
    #[case] original_publisher_place: Option<&str>,
    #[case] expected: &str,
) {
    let style = Style {
        info: StyleInfo {
            title: Some("Original-publication condition test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![TemplateComponent::Variable(TemplateVariable {
                            variable: SimpleVariable::Doi,
                            ..Default::default()
                        })],
                        render_when: Some(TemplateGroupCondition {
                            field_present: Some(TemplateConditionField::OriginalPublisher),
                            field_absent: None,
                        }),
                        ..Default::default()
                    }),
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![TemplateComponent::Variable(TemplateVariable {
                            variable: SimpleVariable::Url,
                            ..Default::default()
                        })],
                        render_when: Some(TemplateGroupCondition {
                            field_present: Some(TemplateConditionField::OriginalPublisherPlace),
                            field_absent: Some(TemplateConditionField::OriginalPublisher),
                        }),
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "primary",
        "title": "Primary Work",
        "issued": "1995",
        "doi": "10.1000/marker-doi",
        "url": "https://example.test/marker-url",
        "original": original_publisher_fragment(original_publisher, original_publisher_place),
    }))
    .unwrap();

    let bibliography = IndexMap::from([("primary".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(
            ["primary".to_string()],
        )
        .trim()
        .to_string();

    assert_eq!(rendered, expected);
}

#[rstest]
#[case::original_published_present(Some("1920"), "")]
#[case::original_published_absent(None, "10.1000/marker-doi")]
fn given_a_field_absent_only_condition_when_the_probed_field_is_present_or_absent_then_the_group_renders_conditionally(
    #[case] original_published: Option<&str>,
    #[case] expected: &str,
) {
    let style = Style {
        info: StyleInfo {
            title: Some("Field-absent-only condition test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Group(TemplateGroup {
                    group: vec![TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::Doi,
                        ..Default::default()
                    })],
                    render_when: Some(TemplateGroupCondition {
                        field_present: None,
                        field_absent: Some(TemplateConditionField::OriginalPublished),
                    }),
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut reference_json = serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "primary",
        "title": "Primary Work",
        "issued": "1995",
        "doi": "10.1000/marker-doi",
    });
    if let Some(issued) = original_published {
        reference_json.as_object_mut().unwrap().insert(
            "original".to_string(),
            serde_json::json!({
                "class": "monograph",
                "type": "book",
                "id": "orig",
                "issued": issued,
            }),
        );
    }
    let reference: InputReference = serde_json::from_value(reference_json).unwrap();

    let bibliography = IndexMap::from([("primary".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(
            ["primary".to_string()],
        )
        .trim()
        .to_string();

    assert_eq!(rendered, expected);
}

#[rstest]
#[case::title_present_and_publisher_absent(
    true,
    None,
    Some("Oxford"),
    "10.1000/marker-doi, Oxford"
)]
#[case::title_present_and_publisher_present(
    true,
    Some("Kodansha"),
    Some("Oxford"),
    "10.1000/marker-doi"
)]
#[case::title_absent(false, None, Some("Oxford"), "")]
fn given_nested_render_when_conditions_when_outer_and_inner_fields_vary_then_each_group_is_evaluated_independently(
    #[case] original_title_present: bool,
    #[case] original_publisher: Option<&str>,
    #[case] original_publisher_place: Option<&str>,
    #[case] expected: &str,
) {
    let style = Style {
        info: StyleInfo {
            title: Some("Nested render-when condition test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Group(TemplateGroup {
                    group: vec![
                        TemplateComponent::Variable(TemplateVariable {
                            variable: SimpleVariable::Doi,
                            ..Default::default()
                        }),
                        TemplateComponent::Group(TemplateGroup {
                            group: vec![TemplateComponent::Variable(TemplateVariable {
                                variable: SimpleVariable::OriginalPublisherPlace,
                                ..Default::default()
                            })],
                            render_when: Some(TemplateGroupCondition {
                                field_present: None,
                                field_absent: Some(TemplateConditionField::OriginalPublisher),
                            }),
                            ..Default::default()
                        }),
                    ],
                    render_when: Some(TemplateGroupCondition {
                        field_present: Some(TemplateConditionField::OriginalTitle),
                        field_absent: None,
                    }),
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut original = original_publisher_fragment(original_publisher, original_publisher_place);
    if original_title_present {
        original
            .as_object_mut()
            .expect("original fragment is an object")
            .insert("title".to_string(), serde_json::json!("Original Title"));
    }

    let reference: InputReference = serde_json::from_value(serde_json::json!({
        "class": "monograph",
        "type": "book",
        "id": "primary",
        "title": "Primary Work",
        "issued": "1995",
        "doi": "10.1000/marker-doi",
        "original": original,
    }))
    .unwrap();

    let bibliography = IndexMap::from([("primary".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(
            ["primary".to_string()],
        )
        .trim()
        .to_string();

    assert_eq!(rendered, expected);
}

#[test]
fn original_title_variable_renders_the_original_language_title() {
    let style = Style {
        info: StyleInfo {
            title: Some("Original-title test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        form: Some(TitleForm::Long),
                        ..Default::default()
                    }),
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Original,
                        form: Some(TitleForm::Long),
                        rendering: Rendering {
                            prefix: Some(" / ".into()),
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
    };

    let reference: InputReference = serde_json::from_str(
        r#"{
            "class": "monograph",
            "type": "book",
            "id": "memory-police",
            "title": "The Memory Police",
            "issued": "2020",
            "original": {
                "class": "monograph",
                "type": "book",
                "id": "memory-police-orig",
                "title": "Hisoyaka na kesshō",
                "issued": "1994"
            }
        }"#,
    )
    .unwrap();

    let bibliography = IndexMap::from([("memory-police".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>([
            "memory-police".to_string()
        ])
        .trim()
        .to_string();

    assert_eq!(rendered, "The Memory Police / Hisoyaka na kesshō");
}

#[rstest]
#[case::with_capitalize_first_case(
    Some(citum_schema::options::titles::TextCase::CapitalizeFirst),
    "Reprinted with a new preface"
)]
#[case::without_a_case_override(None, "reprinted with a new preface")]
fn given_a_number_component_with_free_text_when_a_text_case_override_is_set_then_it_is_applied(
    #[case] text_case: Option<citum_schema::options::titles::TextCase>,
    #[case] expected: &str,
) {
    let style = Style {
        info: StyleInfo {
            title: Some("Number text-case test".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Number(TemplateNumber {
                    number: NumberVariable::Edition,
                    rendering: Rendering {
                        text_case,
                        ..Default::default()
                    },
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference: InputReference = serde_json::from_str(
        r#"{
            "class": "monograph",
            "type": "book",
            "id": "reprint",
            "title": "A Reprinted Work",
            "issued": "2000",
            "edition": "reprinted with a new preface"
        }"#,
    )
    .unwrap();

    let bibliography = IndexMap::from([("reprint".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor
        .render_selected_bibliography_with_format_standalone::<PlainText, _>(
            ["reprint".to_string()],
        )
        .trim()
        .to_string();

    assert_eq!(rendered, expected);
}

mod annotated_html_preview {
    use super::announce_behavior;
    use rstest::rstest;

    #[test]
    fn bibliography_type_templates_preserve_sparse_component_indices() {
        announce_behavior(
            "Annotated bibliography HTML should preserve original type-template indices when intermediate components do not render.",
        );
        super::bibliography_html_injects_sparse_indices_from_type_template();
    }

    #[rstest]
    #[case::default_template(false, "default bibliography template")]
    #[case::type_template(true, "matching bibliography type-template")]
    fn given_a_top_level_list_when_rendering_annotated_html_then_list_children_inherit_the_parent_index(
        #[case] use_type_template: bool,
        #[case] source: &str,
    ) {
        announce_behavior(&format!(
            "Annotated bibliography HTML should map list-rendered child wrappers back to the parent top-level index when rendering via {source}."
        ));
        super::assert_list_preview_inherits_parent_index(use_type_template);
    }
}

#[test]
fn archive_hierarchy_assembled_when_location_absent() {
    let style = Style {
        info: StyleInfo {
            title: Some("Archive Test".to_string()),
            id: Some("archive-test".into()),
            default_locale: Some("en-US".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        ..Default::default()
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::ArchiveLocation,
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("test-archive".into()),
        title: Some(Title::Single("Test Archival Item".to_string())),
        archive_info: Some(ArchiveInfo {
            collection: Some("Foo Papers".to_string()),
            collection_id: Some("MS-123".to_string()),
            series: Some("Correspondence".to_string()),
            r#box: Some("3".to_string()),
            folder: Some("4".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }));

    let bibliography = IndexMap::from([("test-archive".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography().trim().to_string();

    let expected = "collection Foo Papers (MS-123), series Correspondence, box 3, folder 4";
    assert!(
        rendered.contains(expected),
        "expected assembled archive location '{expected}' in output: {rendered}"
    );
}

#[test]
fn archive_location_string_bypasses_assembly() {
    let style = Style {
        info: StyleInfo {
            title: Some("Archive Test".to_string()),
            id: Some("archive-test".into()),
            default_locale: Some("en-US".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        ..Default::default()
                    }),
                    TemplateComponent::Variable(TemplateVariable {
                        variable: SimpleVariable::ArchiveLocation,
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("test-archive-override".into()),
        title: Some(Title::Single("Test Archival Item".to_string())),
        archive_info: Some(ArchiveInfo {
            location: Some("Custom Location String".to_string()),
            collection: Some("Foo Papers".to_string()),
            series: Some("Correspondence".to_string()),
            r#box: Some("3".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    }));

    let bibliography = IndexMap::from([("test-archive-override".to_string(), reference)]);
    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography().trim().to_string();

    // Should return the location string unchanged, not assemble from structured fields
    assert!(
        rendered.contains("Custom Location String"),
        "expected location string in output: {rendered}"
    );
    assert!(
        !rendered.contains("Foo Papers"),
        "structured fields should not appear when location overrides: {rendered}"
    );
    assert!(
        !rendered.contains("Correspondence"),
        "structured fields should not appear when location overrides: {rendered}"
    );
}

#[test]
fn processor_renders_bibliography_annotations() {
    use citum_engine::Processor;
    use citum_engine::render::plain::PlainText;
    use citum_io::AnnotationStyle;
    use citum_schema::reference::{InputReference, Monograph, MonographType, RefID, Title};
    use indexmap::IndexMap;
    use std::collections::HashMap;

    let mut bib = IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some(RefID("ref1".to_string())),
            r#type: MonographType::Book,
            title: Some(Title::Single("Test Book".to_string())),
            ..Default::default()
        })),
    );

    let style = citum_schema::Style {
        info: citum_schema::StyleInfo {
            title: Some("Test Style".to_string()),
            ..Default::default()
        },
        bibliography: Some(citum_schema::BibliographySpec {
            template: Some(vec![citum_schema::tc_title!(Primary)].into()),
            ..Default::default()
        }),
        ..Default::default()
    };

    let processor = Processor::new(style, bib);

    let mut annotations = HashMap::new();
    annotations.insert("ref1".to_string(), "This is an annotation.".to_string());

    let annotation_style = AnnotationStyle::default();

    let rendered = processor
        .render_bibliography_with_format_and_annotations_standalone::<PlainText>(
            Some(&annotations),
            Some(&annotation_style),
        );

    assert_eq!(rendered, "Test Book\n\nThis is an annotation.");
}

#[test]
fn given_all_refs_in_single_group_when_rendered_then_heading_is_suppressed() {
    announce_behavior(
        "When all references fall into a single bibliography group and no unassigned references exist, the group heading is suppressed.",
    );

    let style = build_partition_style(
        "",
        r#"
groups:
  - id: single
    heading: { literal: "Secondary Sources Section Heading" }
    selector: {}
"#,
    );

    let mut bib = IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("ref1".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("First Book".to_string())),
            ..Default::default()
        })),
    );
    bib.insert(
        "ref2".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("ref2".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Second Book".to_string())),
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bib);
    let rendered = processor.render_grouped_bibliography_with_format_standalone::<PlainText>();

    assert_eq!(rendered, "First Book\n\nSecond Book");
}

#[test]
fn given_refs_split_across_two_groups_when_rendered_then_both_headings_appear() {
    announce_behavior(
        "When references are split across two bibliography groups, both group headings are shown.",
    );

    let style = build_partition_style(
        "",
        r#"
groups:
  - id: primary
    heading: { literal: "Primary Sources Section" }
    selector:
      type: book
  - id: secondary
    heading: { literal: "Secondary Sources Section" }
    selector: {}
"#,
    );

    let mut bib = IndexMap::new();
    bib.insert(
        "ref1".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("ref1".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("First Book".to_string())),
            ..Default::default()
        })),
    );
    bib.insert(
        "ref2".to_string(),
        InputReference::Monograph(Box::new(Monograph {
            id: Some("ref2".into()),
            r#type: MonographType::Manuscript,
            title: Some(Title::Single("An Archival Manuscript".to_string())),
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bib);
    let rendered = processor.render_grouped_bibliography_with_format_standalone::<PlainText>();

    assert_eq!(
        rendered,
        "## Primary Sources Section\n\nFirst Book\n\n## Secondary Sources Section\n\nAn Archival Manuscript"
    );
}

#[test]
fn given_grouped_html_bibliography_when_journal_article_rendered_then_container_title_uses_html_italic()
 {
    announce_behavior(
        "When a journal article is rendered in a grouped HTML bibliography, the container (journal) title must appear as <em>…</em>, not as Djot _…_ markup leaking from the PlainText fast-path.",
    );

    // APA-7 has emph:true for container (periodical) titles; add a catch-all group
    // so the grouped rendering code path is exercised.
    let mut style = load_style("styles/embedded/apa-7th.yaml");
    style.bibliography.as_mut().unwrap().groups =
        Some(vec![citum_schema::grouping::BibliographyGroup {
            id: "all".to_string(),
            heading: None,
            selector: citum_schema::grouping::GroupSelector::default(),
            sort: None,
            template: None,
            disambiguate: None,
        }]);

    let mut bib = IndexMap::new();
    bib.insert(
        "art1".to_string(),
        InputReference::SerialComponent(Box::new(SerialComponent {
            id: Some("art1".into()),
            r#type: SerialComponentType::Article,
            title: Some(Title::Single("A Study of Things".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                family: "Smith".into(),
                given: "John".into(),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new("2020".to_string()),
            container: Some(WorkRelation::Embedded(Box::new(InputReference::Serial(
                Box::new(Serial {
                    r#type: SerialType::AcademicJournal,
                    title: Some(Title::Single("Journal of Testing".to_string())),
                    ..Default::default()
                }),
            )))),
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bib);
    let rendered = processor.render_grouped_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains("<em>Journal of Testing</em>"),
        "grouped HTML bibliography should render container title with <em> tags, not Djot markup: {rendered}"
    );
    assert!(
        !rendered.contains("_Journal of Testing_"),
        "grouped HTML bibliography must not leak PlainText Djot markup into HTML output: {rendered}"
    );
}

#[test]
fn given_grouped_html_bibliography_when_title_has_inline_djot_markup_then_markup_is_rendered_as_html()
 {
    announce_behavior(
        "When a bibliography entry's title contains inline Djot markup (e.g. _word_), \
         a grouped HTML bibliography must render it as <em>word</em>, not as literal underscores.",
    );

    let mut style = load_style("styles/embedded/apa-7th.yaml");
    style.bibliography.as_mut().unwrap().groups =
        Some(vec![citum_schema::grouping::BibliographyGroup {
            id: "all".to_string(),
            heading: None,
            selector: citum_schema::grouping::GroupSelector::default(),
            sort: None,
            template: None,
            disambiguate: None,
        }]);

    let mut bib = IndexMap::new();
    bib.insert(
        "art1".to_string(),
        InputReference::SerialComponent(Box::new(SerialComponent {
            id: Some("art1".into()),
            r#type: SerialComponentType::Article,
            // Title with within-field Djot italic markup
            title: Some(Title::Single("The Role of _in vitro_ Studies".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                family: "Smith".into(),
                given: "John".into(),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new("2022".to_string()),
            container: Some(WorkRelation::Embedded(Box::new(InputReference::Serial(
                Box::new(Serial {
                    r#type: SerialType::AcademicJournal,
                    title: Some(Title::Single("Journal of Testing".to_string())),
                    ..Default::default()
                }),
            )))),
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bib);
    let rendered = processor.render_grouped_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains("<em>in vitro</em>"),
        "within-field Djot italic in title must render as <em> in grouped HTML bibliography: {rendered}"
    );
    // The data-title attribute intentionally holds the raw input string; check that
    // Djot underscores do not leak into rendered text content (outside attributes).
    assert!(
        !rendered.contains(">_in vitro_<"),
        "within-field Djot markup must not appear as literal underscores in HTML text content: {rendered}"
    );
}

fn quoted_title_style(grouped: bool) -> Style {
    let mut style = Style {
        info: StyleInfo {
            title: Some("Quoted Title Test".to_string()),
            id: Some("quoted-title-test".into()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    rendering: Rendering {
                        quote: Some(true),
                        ..Default::default()
                    },
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };

    if grouped {
        style.bibliography.as_mut().unwrap().groups =
            Some(vec![citum_schema::grouping::BibliographyGroup {
                id: "all".to_string(),
                heading: None,
                selector: citum_schema::grouping::GroupSelector::default(),
                sort: None,
                template: None,
                disambiguate: None,
            }]);
    }

    style
}

fn title_with_inner_quotes_bibliography() -> IndexMap<String, InputReference> {
    let mut bib = IndexMap::new();
    bib.insert(
        "art1".to_string(),
        InputReference::SerialComponent(Box::new(SerialComponent {
            id: Some("art1".into()),
            r#type: SerialComponentType::Article,
            title: Some(Title::Single("The \"Parmenides\" dialogue".to_string())),
            issued: DateValue::new("2022".to_string()),
            ..Default::default()
        })),
    );
    bib
}

#[test]
fn given_quoted_title_with_inner_quotes_when_html_bibliography_rendered_then_inner_quotes_alternate()
 {
    announce_behavior(
        "When a title component applies quote=true around a title containing inline quotes, the title renderer must render the inner quotes as nested quote marks before the component wrapper is applied.",
    );

    let processor = Processor::new(
        quoted_title_style(false),
        title_with_inner_quotes_bibliography(),
    );
    let rendered = processor.render_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains("“The ‘Parmenides’ dialogue”"),
        "quoted title should alternate outer and inner quote marks: {rendered}"
    );
    assert!(
        !rendered.contains("“The “Parmenides” dialogue”"),
        "quoted title must not use outer quote marks for the nested title quote: {rendered}"
    );
}

#[test]
fn given_quoted_title_with_inner_quotes_when_grouped_html_bibliography_rendered_then_inner_quotes_alternate()
 {
    announce_behavior(
        "Grouped bibliography rendering must preserve the same nested quote-depth context as normal bibliography rendering.",
    );

    let processor = Processor::new(
        quoted_title_style(true),
        title_with_inner_quotes_bibliography(),
    );
    let rendered = processor.render_grouped_bibliography_with_format_standalone::<Html>();

    assert!(
        rendered.contains("“The ‘Parmenides’ dialogue”"),
        "grouped quoted title should alternate outer and inner quote marks: {rendered}"
    );
    assert!(
        !rendered.contains("“The “Parmenides” dialogue”"),
        "grouped quoted title must not use outer quote marks for the nested title quote: {rendered}"
    );
}

#[test]
fn given_multilingual_ref_when_rendering_html_then_data_attrs_match_displayed_form() {
    use citum_schema::reference::contributor::MultilingualName;
    use citum_schema::reference::types::LangID;

    // Style mirrors apa-7th multilingual config: combined titles, transliterated names
    let style = Style {
        info: StyleInfo {
            title: Some("Multilingual Metadata Test".to_string()),
            id: Some("multilingual-metadata-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            multilingual: Some(MultilingualConfig {
                title_mode: Some(MultilingualMode::Combined),
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    ..Default::default()
                })]
                .into(),
            ),
            groups: Some(vec![citum_schema::grouping::BibliographyGroup {
                id: "all".to_string(),
                heading: None,
                selector: citum_schema::grouping::GroupSelector::default(),
                sort: None,
                template: None,
                disambiguate: None,
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let mut bib = IndexMap::new();
    bib.insert(
        "tanaka2019".to_string(),
        InputReference::SerialComponent(Box::new(SerialComponent {
            id: Some("tanaka2019".into()),
            r#type: SerialComponentType::Article,
            title: Some(Title::Multilingual(MultilingualComplex {
                original: "引用の社会的機能：学術知識の構築における参照実践".to_string(),
                lang: None,
                sort_as: None,
                transliterations: HashMap::from([(
                    "ja-Latn".to_string(),
                    "In'yo no shakaiteki kino: Gakujutsu chishiki".to_string(),
                )]),
                translations: HashMap::from([(
                    LangID("en".to_string()),
                    "The Social Function of Citation: Reference Practices in Academic Knowledge"
                        .to_string(),
                )]),
            })),
            author: Some(Contributor::ContributorList(
                citum_schema::reference::ContributorList(vec![Contributor::Multilingual(
                    MultilingualName {
                        original: StructuredName {
                            family: "田中".into(),
                            given: "由紀".into(),
                            ..Default::default()
                        },
                        lang: None,
                        sort_as: None,
                        transliterations: HashMap::from([(
                            "ja-Latn".to_string(),
                            StructuredName {
                                family: "Tanaka".into(),
                                given: "Yuki".into(),
                                ..Default::default()
                            },
                        )]),
                        translations: HashMap::default(),
                    },
                )]),
            )),
            issued: DateValue::new("2019".to_string()),
            container: None,
            ..Default::default()
        })),
    );

    let processor = Processor::new(style, bib);
    let rendered = processor.render_grouped_bibliography_with_format_standalone::<Html>();

    // data-title must be set to the combined (transliterated [translated]) form on the attribute
    assert!(
        rendered.contains(
            r#"data-title="In'yo no shakaiteki kino: Gakujutsu chishiki [The Social Function of Citation: Reference Practices in Academic Knowledge]""#
        ),
        "data-title attribute must use combined transliterated+translated form, not original CJK: {rendered}"
    );
    // data-author must use the transliterated family name
    assert!(
        rendered.contains(r#"data-author="Tanaka" data-year="2019""#),
        "data-author must use transliterated name, not original CJK: {rendered}"
    );
}

#[test]
fn explicit_label_affixes_override_placement_defaults() {
    // RoleLabel prefix/suffix mirror CSL 1.0 cs:label affixes: a suffix
    // placement normally joins with ", ", but an explicit " ("/")" pair
    // wraps the term in parentheses (elsevier's " (Eds.)" shape, with
    // text-case capitalizing the locale's lowercase "eds.").
    let yaml = "info:\n  title: Label Affix Test\nbibliography:\n  template:\n    - contributor: editor\n      form: long\n      label:\n        term: editor\n        form: short\n        placement: suffix\n        prefix: \" (\"\n        suffix: \")\"\n        text-case: capitalize-first\n";
    let style = citum_schema::Style::from_yaml_str(yaml).expect("style should parse");
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John"), ("Doe", "Jane")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith, Jane Doe (Eds.)");
}

#[test]
fn explicit_label_wrap_uses_structural_punctuation_and_suffix_spacing() {
    let yaml = "info:\n  title: Label Wrap Test\nbibliography:\n  template:\n    - contributor: editor\n      form: long\n      label:\n        term: editor\n        form: short\n        placement: suffix\n        wrap: parentheses\n        text-case: capitalize-first\n";
    let style = citum_schema::Style::from_yaml_str(yaml).expect("style should parse");
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John"), ("Doe", "Jane")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "John Smith, Jane Doe (Eds.)");
}

#[test]
fn explicit_label_wrap_preserves_prefix_placement_spacing() {
    let yaml = "info:\n  title: Prefix Label Wrap Test\nbibliography:\n  template:\n    - contributor: editor\n      form: long\n      label:\n        term: editor\n        form: short\n        placement: prefix\n        wrap: parentheses\n        text-case: capitalize-first\n";
    let style = citum_schema::Style::from_yaml_str(yaml).expect("style should parse");
    let bib = citum_schema::bib_map![
        "ITEM-1" => make_multi_editor_only_book("ITEM-1", "Title", "2020", vec![("Smith", "John"), ("Doe", "Jane")]),
    ];
    let processor = Processor::new(style, bib);
    let result = processor.render_bibliography();

    assert_eq!(result, "(Eds.) John Smith, Jane Doe");
}

#[rstest]
#[case::period(BibliographyLabelWrap::Period, "1.Thomas Kuhn")]
#[case::brackets(BibliographyLabelWrap::Brackets, "[1]Thomas Kuhn")]
fn given_numeric_label_mode_when_label_wrap_varies_then_label_renders_flush_against_author(
    #[case] label_wrap: BibliographyLabelWrap,
    #[case] expected: &str,
) {
    // Regression test for csl26-ecwd: `label-mode: numeric` used to insert
    // the auto-generated citation-number as a bare top-level list item, so
    // the bibliography's `separator` leaked between the label and the next
    // component ("1. Kuhn" instead of the oracle-correct "1.Kuhn").
    let style = Style {
        info: StyleInfo {
            title: Some("Numeric Label Wrap Test".to_string()),
            id: Some("numeric-label-wrap-test".into()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                label_mode: Some(BibliographyLabelMode::Numeric),
                label_wrap: Some(label_wrap),
                separator: Some(". ".into()),
                ..Default::default()
            }),
            template: Some(
                vec![TemplateComponent::Contributor(TemplateContributor {
                    contributor: ContributorRole::Author.into(),
                    form: ContributorForm::Long,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    };
    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("kuhn-1962".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(
            "The Structure of Scientific Revolutions".to_string(),
        )),
        author: Some(Contributor::StructuredName(StructuredName {
            given: "Thomas".into(),
            family: "Kuhn".into(),
            ..Default::default()
        })),
        issued: DateValue::new("1962".to_string()),
        ..Default::default()
    }));
    let bib = citum_schema::bib_map!["kuhn-1962" => reference];
    let processor = Processor::new(style, bib);

    let result = processor.render_bibliography();

    assert_eq!(result, expected);
}

/// Regression test for the `Date`-key engine fix (`get_variable_key` in
/// `citum-engine/src/processor/rendering/mod.rs`): `date: issued` must
/// render independently each time it appears in a template regardless of
/// form, since dates are exempt from `TemplateComponentTracker` dedup — CSL
/// restricts variable-consumption tracking to cs:substitute (names only).
/// Found while implementing GB/T 7714 author-date, which needs a bare
/// citation year up front and a full precise date later in the same entry.
#[test]
fn duplicate_issued_date_with_different_forms_both_render() {
    announce_behavior(
        "The same date variable renders independently every time it appears in a template, regardless of form.",
    );
    let yaml = "info:\n  title: Dual Date Form Test\nbibliography:\n  template:\n    - date: issued\n      form: year\n    - date: issued\n      form: year-month-day\n      prefix: ' ('\n      suffix: ')'\n";
    let style = citum_schema::Style::from_yaml_str(yaml).expect("style should parse");
    let reference = InputReference::Monograph(Box::new(Monograph {
        id: Some("dual-date-ref".into()),
        r#type: MonographType::Book,
        title: Some(Title::Single("Dual Date Test".to_string())),
        issued: DateValue::new("2020-03-15".to_string()),
        ..Default::default()
    }));
    let bib = citum_schema::bib_map!["dual-date-ref" => reference];
    let processor = Processor::new(style, bib);

    let result = processor.render_bibliography();

    assert_eq!(result, "2020 (2020, March 15)");
}
