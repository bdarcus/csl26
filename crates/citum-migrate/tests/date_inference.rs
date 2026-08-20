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

use citum_migrate::options_extractor::OptionsExtractor;
use citum_schema::options::MonthFormat;
use csl_legacy::model::{Citation, CslNode, Date, DatePart, Formatting, Info, Layout, Style};

fn announce_behavior(summary: &str) {
    tracing::debug!("behavior: {summary}");
}

fn make_style_with_date(form: Option<String>) -> Style {
    let date_part = DatePart {
        name: "month".to_string(),
        form,
        prefix: None,
        suffix: None,
    };

    let date_node = CslNode::Date(Date {
        variable: "issued".to_string(),
        form: None,
        prefix: None,
        suffix: None,
        delimiter: None,
        date_parts: None,
        text_case: None,
        parts: vec![date_part],
        macro_call_order: None,
        formatting: Formatting::default(),
    });

    Style {
        version: "1.0".to_string(),
        xmlns: "http://purl.org/net/xbiblio/csl".to_string(),
        class: "in-text".to_string(),
        default_locale: None,
        initialize_with: None,
        initialize_with_hyphen: None,
        names_delimiter: None,
        name_as_sort_order: None,
        sort_separator: None,
        delimiter_precedes_last: None,
        delimiter_precedes_et_al: None,
        demote_non_dropping_particle: None,
        and: None,
        page_range_format: None,
        info: Info::default(),
        locale: vec![],
        macros: vec![],
        citation: Citation {
            layout: Layout {
                prefix: None,
                suffix: None,
                delimiter: None,
                children: vec![date_node],
            },
            localized_layouts: Vec::new(),
            sort: None,
            collapse: None,
            cite_group_delimiter: None,
            year_suffix_delimiter: None,
            et_al_min: None,
            et_al_use_first: None,
            disambiguate_add_year_suffix: None,
            disambiguate_add_names: None,
            disambiguate_add_givenname: None,
            disambiguate_givenname_rule: None,
        },
        bibliography: None,
    }
}

#[test]
fn test_infer_short_month() {
    announce_behavior("A CSL short month form migrates to the short month format option.");
    let style = make_style_with_date(Some("short".to_string()));
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Short);
}

#[test]
fn test_infer_long_month() {
    announce_behavior("A CSL long month form migrates to the long month format option.");
    let style = make_style_with_date(Some("long".to_string()));
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Long);
}

#[test]
fn test_infer_numeric_month() {
    announce_behavior("A CSL numeric month form migrates to the numeric month format option.");
    let style = make_style_with_date(Some("numeric".to_string()));
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Numeric);
}

#[test]
fn test_infer_default_month() {
    announce_behavior(
        "A missing month form falls back to the long month format option during migration.",
    );
    let style = make_style_with_date(None); // Default is usually long
    let config = OptionsExtractor::extract(&style);

    assert!(config.dates.is_some());
    assert_eq!(config.dates.unwrap().month, MonthFormat::Long);
}
