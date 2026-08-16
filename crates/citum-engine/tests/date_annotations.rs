/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Integration coverage for the opaque date `note` (calendar-date
//! annotation) feature — `docs/specs/CALENDAR_DATE_ANNOTATIONS.md`. Exercises
//! the embedded GB/T 7714—2025 author-date style end to end: bibliography
//! renders the wrapped note, citations do not.

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in test code."
)]

mod common;
use common::announce_behavior;

use citum_engine::Processor;
use citum_io::load_bibliography;
use citum_schema::citation::{Citation, CitationItem};
use citum_schema::options::{BibliographyOptions, DateConfig};
use citum_schema::reference::{
    Contributor, ContributorEntry, ContributorRole, DateValue, InputReference, Monograph,
    MonographType, StructuredName, Title,
};
use citum_schema::template::{
    DateForm, DateVariable, TemplateComponent, TemplateDate, WrapConfig, WrapPunctuation,
};
use citum_schema::{BibliographySpec, Style, StyleInfo};
use indexmap::IndexMap;
use rstest::rstest;
use std::path::PathBuf;

/// Load the embedded zh-CN locale for GB/T fixtures.
///
/// `Processor::new` seeds a plain `Locale::en_us()` base locale; the real
/// `citum render` CLI resolves a style's `info.default-locale` (zh-CN for
/// the GB/T family) into an explicit locale via `create_processor`
/// (`citum-cli/src/style_resolver.rs`). Tests that construct a `Processor`
/// directly must do the same or a language-sensitive term (e.g. GB/T's
/// `佚名`/`Anon` anonymous-author term, csl26-6eak) resolves against the
/// wrong locale's default.
fn zh_cn_locale() -> citum_schema::Locale {
    let bytes = citum_schema::embedded::get_locale_bytes("zh-CN").expect("zh-CN must be embedded");
    citum_schema::Locale::from_yaml_str(std::str::from_utf8(bytes).expect("valid UTF-8"))
        .expect("zh-CN locale should parse")
}

/// Project root path resolver.
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Render one reference, by id, from the real pinned GB/T 7714—2025 corpus
/// (`tests/fixtures/test-items-library/gb-t-7714-2025.json` — the fixture
/// PR #1067's reviewer commented against, not synthetic stand-in data),
/// isolated to a single-entry bibliography so the assertion is a precise
/// full-string match rather than a substring search across the ~250-item
/// corpus.
fn render_pinned_gbt_entry(style_name: &str, ref_id: &str) -> String {
    let corpus_path = project_root().join("tests/fixtures/test-items-library/gb-t-7714-2025.json");
    let corpus = load_bibliography(&corpus_path).expect("pinned GB/T corpus should load");
    let reference = corpus
        .get(ref_id)
        .unwrap_or_else(|| panic!("pinned GB/T corpus should contain {ref_id}"))
        .clone();
    let bibliography = IndexMap::from([(ref_id.to_string(), reference)]);

    let style = citum_schema::embedded::get_embedded_style(style_name)
        .unwrap_or_else(|| panic!("{style_name} should be embedded"))
        .unwrap_or_else(|_| panic!("{style_name} should parse"))
        .into_resolved();

    Processor::with_locale(style, bibliography, zh_cn_locale()).render_bibliography()
}

/// Build a minimal book reference with an annotated `issued` date.
fn annotated_book(id: &str, title: &str, year: &str, note: &str) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        issued: DateValue {
            value: year.to_string(),
            note: Some(note.to_string()),
        },
        ..Default::default()
    }))
}

/// Same as `annotated_book`, with a shared author for disambiguation tests.
fn annotated_book_by(
    id: &str,
    title: &str,
    author_family: &str,
    year: &str,
    note: &str,
) -> InputReference {
    InputReference::Monograph(Box::new(Monograph {
        id: Some(id.into()),
        r#type: MonographType::Book,
        title: Some(Title::Single(title.to_string())),
        contributors: vec![ContributorEntry {
            roles: ContributorRole::Author.into(),
            contributor: Contributor::StructuredName(StructuredName {
                family: author_family.into(),
                ..Default::default()
            }),
            gender: None,
        }],
        issued: DateValue {
            value: year.to_string(),
            note: Some(note.to_string()),
        },
        ..Default::default()
    }))
}

/// A bibliography-only style rendering `issued` twice per item — the front
/// occurrence bare, the second with `suppress-note: true` — with
/// `note-wrap: parentheses` configured. Mirrors the GB/T author-date shape
/// that motivated `csl26-gl0n` (a short front-matter year plus a redundant
/// later occurrence).
fn style_with_duplicated_issued_date() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Duplicated Issued Date".to_string()),
            ..Default::default()
        },
        bibliography: Some(BibliographySpec {
            options: Some(BibliographyOptions {
                dates: Some(DateConfig {
                    note_wrap: Some(WrapConfig {
                        punctuation: WrapPunctuation::Parentheses,
                        inner_prefix: None,
                        inner_suffix: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            template: Some(
                vec![
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::Issued,
                        form: DateForm::Year,
                        ..Default::default()
                    }),
                    TemplateComponent::Date(TemplateDate {
                        date: DateVariable::Issued,
                        form: DateForm::Year,
                        suppress_note: Some(true),
                        ..Default::default()
                    }),
                ]
                .into(),
            ),
            ..Default::default()
        }),
        ..Default::default()
    }
    .into_resolved()
}

#[test]
fn given_component_with_suppress_note_when_date_renders_twice_then_note_appears_once() {
    announce_behavior(
        "A template rendering the same date variable twice per item wraps the \
         calendar note on the occurrence without `suppress-note: true` only — \
         the second, redundant occurrence renders bare, even though the \
         section's `note-wrap` is configured and the date carries a note on \
         both renders (it's the same underlying date value) — csl26-gl0n.",
    );
    let style = style_with_duplicated_issued_date();
    let bibliography = IndexMap::from([(
        "minguo-1947".to_string(),
        annotated_book("minguo-1947", "戰後臺灣史", "1947", "民国三十六年"),
    )]);

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography();

    assert_eq!(rendered, "1947(民国三十六年). 1947");
}

fn single_item_citation(id: &str) -> Citation {
    Citation {
        items: vec![CitationItem {
            id: id.to_string(),
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn given_gb_t_author_date_style_when_rendering_bibliography_then_calendar_note_is_wrapped() {
    announce_behavior(
        "GB/T 7714 author-date wraps an issued date's opaque note (e.g. a Minguo \
         calendar annotation) in the bibliography, per the style's `note-wrap: \
         parentheses` bibliography-scoped `dates` option — csl26-epu6.",
    );
    let style = citum_schema::embedded::get_embedded_style("gb-t-7714-2025-author-date")
        .expect("gb-t-7714-2025-author-date should be embedded")
        .expect("gb-t-7714-2025-author-date should parse")
        .into_resolved();
    let bibliography = IndexMap::from([(
        "minguo-1947".to_string(),
        annotated_book("minguo-1947", "戰後臺灣史", "1947", "民国三十六年"),
    )]);

    let processor = Processor::with_locale(style, bibliography, zh_cn_locale());
    let rendered = processor.render_bibliography();

    assert_eq!(rendered, "戰後臺灣史，1947（民国三十六年）. [M].");
}

#[test]
fn given_gb_t_author_date_style_when_rendering_citation_then_calendar_note_is_absent() {
    announce_behavior(
        "GB/T 7714 author-date citations carry no `note-wrap` option (it is set only \
         under bibliography.options.dates), so an annotated date's note never appears \
         in citation output — bibliography-only scoping, csl26-epu6.",
    );
    let style = citum_schema::embedded::get_embedded_style("gb-t-7714-2025-author-date")
        .expect("gb-t-7714-2025-author-date should be embedded")
        .expect("gb-t-7714-2025-author-date should parse")
        .into_resolved();
    let bibliography = IndexMap::from([(
        "minguo-1947".to_string(),
        annotated_book("minguo-1947", "戰後臺灣史", "1947", "民国三十六年"),
    )]);

    let processor = Processor::new(style, bibliography);
    let citation = processor
        .process_citation(&single_item_citation("minguo-1947"))
        .expect("annotated-date citation should render");

    assert_eq!(citation, "（“戰後臺灣史”，1947）");
}

#[test]
fn given_two_refs_same_author_year_different_note_when_disambiguating_then_still_collide() {
    announce_behavior(
        "A date's opaque note never enters author-date collision grouping — two \
         references by the same author with the same issued year but different \
         calendar notes still collide and receive year-suffix disambiguators, exactly \
         as if neither carried a note. `year()`/`effective_issued_date` read only \
         `value`; `note` is invisible to `build_group_key` — csl26-epu6.",
    );
    let style = citum_schema::embedded::get_embedded_style("gb-t-7714-2025-author-date")
        .expect("gb-t-7714-2025-author-date should be embedded")
        .expect("gb-t-7714-2025-author-date should parse")
        .into_resolved();
    let bibliography = IndexMap::from([
        (
            "kang-a".to_string(),
            annotated_book_by("kang-a", "First Work", "Kang", "1947", "民国三十六年"),
        ),
        (
            "kang-b".to_string(),
            annotated_book_by("kang-b", "Second Work", "Kang", "1947", "不同的注释"),
        ),
    ]);

    let processor = Processor::new(style, bibliography);
    let rendered = processor.render_bibliography();

    assert_eq!(
        rendered,
        "Kang，1947a（民国三十六年）. First Work[M].\n\n\
         Kang，1947b（不同的注释）. Second Work[M]."
    );
}

/// **Given** the real pinned GB/T 7714—2025 corpus reference
/// `{ref_id}` (a Zotero-extra `issued: <year>（<note>）` note-field
/// override, converted via `annotated_issued_from_legacy`) and a GB/T
/// style with `note-wrap: parentheses`,
/// **When** its bibliography entry is rendered,
/// **Then** the entry contains the wrapped calendar annotation exactly as
/// it appears in the corpus — a regression guard grounded in the actual
/// fixture the feature was motivated by, not synthetic stand-in data.
#[rstest]
#[case::gb_t_7714_2025_author_date_minguo(
    "gb-t-7714-2025-author-date",
    "gbt7714.7.5.4.1:1",
    "1947（民国三十六年）. [M]."
)]
#[case::gb_t_7714_2025_author_date_kangxi(
    "gb-t-7714-2025-author-date",
    "gbt7714.7.5.4.1:2",
    "1705（康熙四十四年）. [M]."
)]
#[case::gb_t_7714_2025_numeric_minguo(
    "gb-t-7714-2025-numeric",
    "gbt7714.7.5.4.1:1",
    "[1][M]. 1947（民国三十六年）."
)]
#[case::gb_t_7714_2025_numeric_kangxi(
    "gb-t-7714-2025-numeric",
    "gbt7714.7.5.4.1:2",
    "[1][M]. 1705（康熙四十四年）."
)]
#[case::gb_t_7714_2025_note_minguo(
    "gb-t-7714-2025-note",
    "gbt7714.7.5.4.1:1",
    "[1][M]. 1947（民国三十六年）."
)]
#[case::gb_t_7714_2025_note_kangxi(
    "gb-t-7714-2025-note",
    "gbt7714.7.5.4.1:2",
    "[1][M]. 1705（康熙四十四年）."
)]
#[case::gb_t_7714_2025_author_date_qing_dynasty(
    "gb-t-7714-2025-author-date",
    "gbt7714.8.2.2:2",
    "王夫之，1865（清同治四年）. 宋论[M]. 刻本. 金陵：湘乡曾国荃."
)]
#[case::gb_t_7714_2025_author_date_guangxu_era(
    "gb-t-7714-2025-author-date",
    "gbt7714.8.12.3:1",
    "李鸿章，1887. 奏请上海道库洋务外销要款无款可筹仍拨药厘接济事：04-01-35-0399-039[A]. 北京：中国第一历史档案馆，1887（光绪十三年三月十三日）."
)]
#[case::gb_t_7714_2025_author_date_republic_era(
    "gb-t-7714-2025-author-date",
    "gbt7714.8.12.3:3",
    "中国人民解放军武汉市军事管制委员会接管国立武汉大学的文告，1949. [A/OL]. 武汉：武汉大学档案馆，1949（中华民国三十八年八月）. https://archive.whu.edu.cn/index/forwardView/20/51."
)]
fn given_pinned_gbt_record_when_rendering_bibliography_then_note_is_wrapped(
    #[case] style_name: &str,
    #[case] ref_id: &str,
    #[case] expected: &str,
) {
    let rendered = render_pinned_gbt_entry(style_name, ref_id);
    assert_eq!(rendered, expected);
}
