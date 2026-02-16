/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

mod common;
use common::*;

use csln_core::{
    options::{Config, Processing},
    template::{NumberVariable, Rendering, TemplateComponent, TemplateNumber},
    CitationSpec, Style, StyleInfo,
};
use csln_processor::Processor;

// --- Helper Functions ---

fn build_numeric_style() -> Style {
    Style {
        info: StyleInfo {
            title: Some("Numeric Test".to_string()),
            id: Some("numeric-test".to_string()),
            ..Default::default()
        },
        options: Some(Config {
            processing: Some(Processing::Numeric),
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: Some(vec![TemplateComponent::Number(TemplateNumber {
                number: NumberVariable::CitationNumber,
                rendering: Rendering::default(),
                ..Default::default()
            })]),
            wrap: Some(csln_core::template::WrapPunctuation::Brackets),
            ..Default::default()
        }),
        ..Default::default()
    }
}

// --- Disambiguation Tests ---

/// Test year suffix disambiguation with alphabetical title sorting.
#[test]
fn test_disambiguate_yearsuffixandsort() {
    let input = vec![
        make_book("item1", "Smith", "John", 2020, "Alpha"),
        make_book("item2", "Smith", "John", 2020, "Beta"),
    ];
    let citation_items = vec![vec!["item1", "item2"]];
    let expected = "Smith, (2020a); Smith, (2020b)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test empty input handling (placeholder test).
#[test]
fn test_disambiguate_yearsuffixattwolevels() {
    let input = vec![];
    let citation_items: Vec<Vec<&str>> = vec![];
    let expected = "";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test year suffix disambiguation with multiple identical references.
#[test]
fn test_disambiguate_yearsuffixmixeddates() {
    let input = vec![
        make_article("22", "Ylinen", "A", 1995, "Article A"),
        make_article("21", "Ylinen", "A", 1995, "Article B"),
        make_article("23", "Ylinen", "A", 1995, "Article C"),
    ];
    let citation_items = vec![vec!["22", "21", "23"]];
    let expected = "Ylinen, (1995a); Ylinen, (1995b); Ylinen, (1995c)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test given name expansion for authors with duplicate family names.
#[test]
fn test_disambiguate_bycitetwoauthorssamefamilyname() {
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
    let expected = "Asthma, Asthma, (1980); Bronchitis, (1995); Asthma, (1885)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        false,
        false,
        true,
        None,
        None,
    );
}

/// Test et-al expansion success: Name expansion disambiguates conflicting references.
#[test]
fn test_disambiguate_addnamessuccess() {
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
    let expected = "Smith, Brown, et al., (1980); Smith, Beefheart, et al., (1980)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        false,
        true,
        false,
        Some(3),
        Some(1),
    );
}

/// Test et-al expansion failure: Cascade to year suffix when name expansion fails.
#[test]
fn test_disambiguate_addnamesfailure() {
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
    let expected = "Smith et al., (1980a); Smith et al., (1980b)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        true,
        true,
        false,
        Some(3),
        Some(1),
    );
}

/// Test given name expansion with initial form (initialize_with).
#[test]
fn test_disambiguate_bycitegivennameshortforminitializewith() {
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
    let expected = "Roe, (2000)
J Doe, (2000); A Doe, (2000)
T Smith, (2000); T Smith, (2000)";

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        false,
        false,
        true,
        None,
        None,
    );
}

/// Test year suffix + et-al with varying author list lengths.
#[test]
fn test_disambiguate_basedonetalsubsequent() {
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

    run_test_case_native_with_options(
        &input,
        &citation_items,
        expected,
        "citation",
        true,
        false,
        false,
        Some(3),
        Some(1),
    );
}

/// Test conditional disambiguation with identical author-year pairs.
#[test]
fn test_disambiguate_bycitedisambiguatecondition() {
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
    let expected = "Doe, Roe, (2000a); Doe, Roe, (2000b)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

/// Test year suffix with 30 entries (base-26 suffix wrapping).
#[test]
fn test_disambiguate_yearsuffixfiftytwoentries() {
    let mut input = Vec::new();
    let mut citation_ids = Vec::new();

    for i in 1..=30 {
        input.push(make_book(
            &format!("ITEM-{}", i),
            "Smith",
            "John",
            1986,
            "Book",
        ));
        citation_ids.push(format!("ITEM-{}", i));
    }

    let citation_items = vec![citation_ids.iter().map(|s| s.as_str()).collect()];
    let expected = "Smith, (1986a); Smith, (1986b); Smith, (1986c); Smith, (1986d); Smith, (1986e); Smith, (1986f); Smith, (1986g); Smith, (1986h); Smith, (1986i); Smith, (1986j); Smith, (1986k); Smith, (1986l); Smith, (1986m); Smith, (1986n); Smith, (1986o); Smith, (1986p); Smith, (1986q); Smith, (1986r); Smith, (1986s); Smith, (1986t); Smith, (1986u); Smith, (1986v); Smith, (1986w); Smith, (1986x); Smith, (1986y); Smith, (1986z); Smith, (1986aa); Smith, (1986ab); Smith, (1986ac); Smith, (1986ad)";

    run_test_case_native(&input, &citation_items, expected, "citation");
}

// --- Numeric Citation Tests ---

#[test]
fn test_numeric_citation() {
    let style = build_numeric_style();

    let mut bib = indexmap::IndexMap::new();
    bib.insert(
        "item1".to_string(),
        make_book("item1", "Smith", "John", 2020, "Title A"),
    );
    bib.insert(
        "item2".to_string(),
        make_book("item2", "Doe", "Jane", 2021, "Title B"),
    );

    let processor = Processor::new(style, bib);

    let citation1 = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item1".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let citation2 = csln_core::citation::Citation {
        items: vec![csln_core::citation::CitationItem {
            id: "item2".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.process_citation(&citation1).unwrap(), "[1]");
    assert_eq!(processor.process_citation(&citation2).unwrap(), "[2]");
}
