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

use citum_engine::Processor;
use citum_schema::{
    CitationSpec, Style, StyleInfo,
    locale::{GeneralTerm, TermForm},
    options::{Config, ContributorConfig, Processing, ShortenListOptions},
    template::{
        ContributorForm, ContributorRole, DateForm, DateVariable as TDateVar, TemplateComponent,
        TemplateContributor, TemplateDate, TemplateTerm,
    },
};

// --- Helper Functions ---

fn build_name_style(form: ContributorForm, shorten: Option<ShortenListOptions>) -> Style {
    Style {
        info: StyleInfo {
            title: Some("Name Test".to_string()),
            id: Some("name-test".into()),
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
                contributor: ContributorRole::Author.into(),
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
            id: Some("date-test".into()),
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

// --- Behavior Announcements ---

#[test]
fn test_name_rendering_basic() {
    announce_behavior("Long name form renders given-name then family name.");
    let style = build_name_style(ContributorForm::Long, None);

    let bib = citum_schema::bib_map!["item1" => make_book("item1", "Smith", "John", 2020, "Title")];
    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "John Smith"
    );
}

#[test]
fn test_name_rendering_short() {
    announce_behavior("Short name form renders family name only.");
    let style = build_name_style(ContributorForm::Short, None);

    let bib = citum_schema::bib_map!["item1" => make_book("item1", "Smith", "John", 2020, "Title")];
    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "Smith"
    );
}

#[test]
fn test_name_rendering_family_only() {
    announce_behavior("Family-only form renders surname without given name or particles.");
    let style = build_name_style(ContributorForm::FamilyOnly, None);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Gogh", "Vincent", 1888, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut()
        && let Some(citum_schema::reference::Contributor::StructuredName(n)) = &mut m.author
    {
        n.non_dropping_particle = Some("van".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "Gogh"
    );
}

#[test]
fn test_name_rendering_et_al() {
    announce_behavior("Name list is truncated with et al. when it exceeds the configured minimum.");
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
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "Smith et al."
    );
}

#[test]
fn test_name_rendering_particles() {
    announce_behavior("Non-dropping particles are included when rendering the long name form.");
    let style = build_name_style(ContributorForm::Long, None);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Gogh", "Vincent", 1888, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut()
        && let Some(citum_schema::reference::Contributor::StructuredName(n)) = &mut m.author
    {
        n.non_dropping_particle = Some("van".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "Vincent van Gogh"
    );
}

#[test]
fn test_name_rendering_corporate() {
    announce_behavior("Corporate names are rendered verbatim without inversion.");
    let style = build_name_style(ContributorForm::Long, None);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "", "", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.author = Some(citum_schema::reference::Contributor::SimpleName(
            citum_schema::reference::SimpleName {
                name: citum_schema::reference::MultilingualString::Simple(
                    "World Health Organization".to_string(),
                ),
                location: None,
                short_name: None,
            },
        ));
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "World Health Organization"
    );
}

#[test]
fn test_date_rendering_year() {
    announce_behavior("Year-only date form renders just the year.");
    let style = build_date_style(DateForm::Year);

    let bib = citum_schema::bib_map!["item1" => make_book("item1", "Smith", "J", 2020, "Title")];
    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "2020"
    );
}

#[test]
fn test_date_rendering_full() {
    announce_behavior("Full date form renders month, day, and year in locale order.");
    let style = build_date_style(DateForm::Full);

    let mut bib = indexmap::IndexMap::new();
    // EDTF: 2020-05-15
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("2020-05-15".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    // Default en-US full: "May 15, 2020"
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "May 15, 2020"
    );
}

#[test]
fn test_date_rendering_day_month_abbr_year() {
    announce_behavior("Day-month-abbreviated-year form renders in day-month-year order.");
    let style = build_date_style(DateForm::DayMonthAbbrYear);

    let mut bib = indexmap::IndexMap::new();
    // EDTF: 2020-05-15
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("2020-05-15".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    // Short term for May in English locale is usually "May", depending on fallback.
    // It's "day month year".
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "15 May 2020"
    );
}

#[test]
fn test_date_rendering_range() {
    announce_behavior("A date range renders with an en dash between start and end years.");
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    // EDTF range: 2020/2022
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("2020/2022".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    // Range en-dash: "2020–2022"
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "2020–2022"
    );
}

#[test]
fn chicago_18_condenses_edtf_year_ranges() {
    announce_behavior(
        "Chicago 18 condenses a closed EDTF year range using inclusive-number rules.",
    );
    let style = load_style("styles/embedded/chicago-author-date-18th.yaml");

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Smith", "J", 2021, "Title");
    if let ClassExtension::Monograph(monograph) = item.extension_mut() {
        monograph.issued = citum_schema::reference::DateValue::new("2021/2026".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "(Smith 2021–26)"
    );
}

#[test]
fn test_date_rendering_negative_year() {
    announce_behavior("Negative EDTF years render historical years with a locale era suffix.");
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("-0099".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "100 BC"
    );
}

#[test]
fn test_date_rendering_negative_full_date() {
    announce_behavior("Negative full dates render the historical BC year with month and day.");
    let style = build_date_style(DateForm::Full);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("-0043-03-15".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "March 15, 44 BC"
    );
}

#[test]
fn test_date_rendering_negative_range() {
    announce_behavior("Historical date ranges render BC years at both ends of the interval.");
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("-0099/-0043".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "100 BC–44 BC"
    );
}

#[test]
fn test_date_rendering_open_range() {
    announce_behavior(
        "An open date range renders with an en dash followed by a locale present term.",
    );
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    // EDTF open range: 2020/..
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new("2020/..".to_string());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    // Open range: "2020–present" (using locale term)
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "2020–present"
    );
}

#[test]
fn test_date_rendering_fallback() {
    announce_behavior("A missing date falls back to the configured no-date term.");
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    // Missing date
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.issued = citum_schema::reference::DateValue::new(String::new());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "n.d."
    );
}

#[test]
fn test_date_rendering_uses_created_when_issued_is_missing() {
    announce_behavior("A created date backfills issued rendering for compatibility.");
    let style = build_date_style(DateForm::Year);

    let mut bib = indexmap::IndexMap::new();
    let mut item = make_book("item1", "Smith", "J", 2020, "Title");
    if let ClassExtension::Monograph(m) = item.extension_mut() {
        m.created = citum_schema::reference::DateValue::new("1954-05-17".to_string());
        m.issued = citum_schema::reference::DateValue::new(String::new());
    }
    bib.insert("item1".to_string(), item);

    let processor = Processor::new(style, bib);
    assert_eq!(
        processor
            .process_citation(&citum_schema::cite!("item1"))
            .unwrap(),
        "1954"
    );
}
