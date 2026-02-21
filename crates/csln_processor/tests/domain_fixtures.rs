/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use csln_core::Style;
use csln_core::citation::{Citation, CitationItem};
use csln_processor::Processor;
use csln_processor::io::load_bibliography;
use std::fs;
use std::path::{Path, PathBuf};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_style(path: &Path) -> Style {
    let bytes = fs::read(path).expect("style fixture should be readable");
    serde_yaml::from_slice(&bytes).expect("style fixture should parse")
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
fn test_legal_fixture_is_covered_in_processor_tests() {
    let root = project_root();
    let style = load_style(&root.join("styles/apa-7th.yaml"));
    let bibliography = load_bibliography(&root.join("tests/fixtures/references-legal.json"))
        .expect("legal fixture should parse");

    let processor = Processor::new(style, bibliography);
    let brown = processor
        .process_citation(&single_item_citation("brown1954"))
        .expect("brown citation should render");
    let civil = processor
        .process_citation(&single_item_citation("civilrights1964"))
        .expect("civil rights citation should render");
    let treaty = processor
        .process_citation(&single_item_citation("versailles1919"))
        .expect("treaty citation should render");
    let rendered_bib = processor.render_bibliography();

    assert!(brown.contains("Brown v. Board of Education"));
    assert!(civil.contains("Civil Rights Act of 1964"));
    assert!(treaty.contains("Treaty of Versailles"));
    assert!(rendered_bib.contains("U.S. Supreme Court"));
}

#[test]
fn test_scientific_fixture_is_covered_in_processor_tests() {
    let root = project_root();
    let style = load_style(&root.join("styles/apa-7th.yaml"));
    let bibliography = load_bibliography(&root.join("tests/fixtures/references-scientific.json"))
        .expect("scientific fixture should parse");

    let processor = Processor::new(style, bibliography);
    let patent = processor
        .process_citation(&single_item_citation("pavlovic2008"))
        .expect("patent citation should render");
    let dataset = processor
        .process_citation(&single_item_citation("irino2009"))
        .expect("dataset citation should render");
    let standard = processor
        .process_citation(&single_item_citation("ieee754-2008"))
        .expect("standard citation should render");
    let software = processor
        .process_citation(&single_item_citation("rcore2021"))
        .expect("software citation should render");
    let rendered_bib = processor.render_bibliography();

    assert!(patent.contains("Pavlovic"));
    assert!(dataset.contains("Irino") && dataset.contains("2009"));
    assert!(standard.contains("IEEE Standard for Floating-Point Arithmetic"));
    assert!(software.contains("R Core Team"));
    assert!(rendered_bib.contains("[Dataset]"));
    assert!(rendered_bib.contains("Patent No. 7,347,809"));
}

#[test]
fn test_multilingual_fixture_is_covered_in_processor_tests() {
    let root = project_root();
    let style = load_style(&root.join("styles/apa-7th.yaml"));
    let bibliography = load_bibliography(&root.join("tests/fixtures/references-multilingual.yaml"))
        .expect("multilingual fixture should parse");

    let processor = Processor::new(style, bibliography);
    let rendered_bib = processor.render_bibliography();

    assert!(rendered_bib.contains("Nguyễn"));
    assert!(rendered_bib.contains("Trần"));
    assert!(rendered_bib.contains("Nhà xuất bản"));
    assert!(rendered_bib.contains("Oxford University Press"));
}
