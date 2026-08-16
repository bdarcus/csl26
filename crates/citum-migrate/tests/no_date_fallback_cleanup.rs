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

use citum_migrate::{
    Compressor, MacroInliner, OptionsExtractor, TemplateCompiler, Upsampler,
    options_extractor::dates::extract_date_fallback,
};
use citum_schema::{
    locale::GeneralTerm,
    options::DateFallbackCandidate,
    template::{DateVariable, TemplateComponent},
};
use csl_legacy::parser::parse_style;
use roxmltree::Document;

fn parse_csl(xml: &str) -> Result<csl_legacy::model::Style, String> {
    let doc = Document::parse(xml).map_err(|err| err.to_string())?;
    parse_style(doc.root_element()).map_err(|err| err.clone())
}

#[test]
fn migration_drops_explicit_no_date_terms_when_issued_is_already_present() {
    let xml = r#"<style>
        <citation><layout><text variable="title"/></layout></citation>
        <bibliography>
            <layout>
                <group prefix="(" suffix=")">
                    <choose>
                        <if variable="issued">
                            <date variable="issued">
                                <date-part name="year"/>
                            </date>
                        </if>
                        <else>
                            <text term="no date" form="short"/>
                        </else>
                    </choose>
                </group>
            </layout>
        </bibliography>
    </style>"#;
    let style = parse_csl(xml).expect("legacy style should parse");
    let options = OptionsExtractor::extract(&style);
    let candidates = options
        .date_fallback
        .as_ref()
        .and_then(|policy| policy.rule_for(true, "book"))
        .and_then(|rule| rule.candidates())
        .expect("authored no-date branch should become an explicit policy");
    assert!(matches!(
        candidates.as_ref(),
        [DateFallbackCandidate::Message(_)]
    ));

    let inliner = MacroInliner::new(&style);
    let flattened = inliner
        .inline_bibliography(&style)
        .expect("bibliography should exist");
    let raw_bib = Upsampler::new().upsample_nodes(&flattened);
    let compressor = Compressor;
    let bib_ir = compressor.compress_nodes(raw_bib);
    let compiler = TemplateCompiler;
    let template = compiler.compile_bibliography(&bib_ir, false);

    assert!(template.iter().any(component_contains_issued_date));
    assert!(!template.iter().any(component_contains_no_date_term));
}

#[test]
fn migration_leaves_bare_issued_dates_without_a_fallback_policy() {
    let style = parse_csl(
        r#"<style>
            <citation><layout><date variable="issued"><date-part name="year"/></date></layout></citation>
        </style>"#,
    )
    .expect("legacy style should parse");

    assert!(OptionsExtractor::extract(&style).date_fallback.is_none());
}

#[test]
fn migration_extracts_ordered_alternative_date_candidates() {
    let style = parse_csl(
        r#"<style>
            <citation><layout><choose>
                <if variable="issued"><date variable="issued"><date-part name="year"/></date></if>
                <else-if variable="accessed"><date variable="accessed"><date-part name="year"/></date></else-if>
                <else><text term="no date" form="short"/></else>
            </choose></layout></citation>
        </style>"#,
    )
    .expect("legacy style should parse");

    let options = OptionsExtractor::extract(&style);
    let candidates = options
        .date_fallback
        .as_ref()
        .and_then(|policy| policy.rule_for(true, "book"))
        .and_then(|rule| rule.candidates())
        .expect("alternative chain should become a date fallback policy");
    assert!(matches!(
        candidates.as_ref(),
        [
            DateFallbackCandidate::Date(_),
            DateFallbackCandidate::Message(_)
        ]
    ));
}

#[test]
fn migration_marks_unrepresentable_date_fallbacks_unsupported() {
    let style = parse_csl(
        r#"<style>
            <citation><layout><choose>
                <if variable="issued"><date variable="issued"><date-part name="year"/></date></if>
                <else><text variable="title"/></else>
            </choose></layout></citation>
        </style>"#,
    )
    .expect("legacy style should parse");

    let extracted = OptionsExtractor::extract_migration_options(&style);
    assert!(extracted.unsupported_date_fallback);
    assert!(extracted.options.date_fallback.is_none());
}

#[test]
fn migration_drops_conflicting_date_fallback_policies() {
    let style = parse_csl(
        r#"<style>
            <citation><layout><choose>
                <if variable="issued"><date variable="issued"><date-part name="year"/></date></if>
                <else><text term="no date" form="short"/></else>
            </choose></layout></citation>
            <bibliography><layout><choose>
                <if variable="issued"><date variable="issued"><date-part name="year-month"/></date></if>
                <else><date variable="accessed"><date-part name="year"/></date></else>
            </choose></layout></bibliography>
        </style>"#,
    )
    .expect("legacy style should parse");

    let extraction = extract_date_fallback(&style);
    assert!(extraction.unsupported, "{extraction:?}");
    assert_eq!(extraction.policy, None);
}

#[test]
fn migration_rejects_nested_type_scoped_date_fallbacks() {
    let style = parse_csl(
        r#"<style>
            <citation><layout><choose>
                <if type="book">
                    <choose>
                        <if variable="issued"><date variable="issued"><date-part name="year"/></date></if>
                        <else><text term="no date" form="short"/></else>
                    </choose>
                </if>
                <else><text variable="title"/></else>
            </choose></layout></citation>
        </style>"#,
    )
    .expect("legacy style should parse");

    let extraction = extract_date_fallback(&style);
    assert!(extraction.unsupported, "{extraction:?}");
    assert_eq!(extraction.policy, None);
}

#[test]
fn migration_rejects_decorated_single_child_date_groups() {
    let style = parse_csl(
        r#"<style>
            <citation><layout><choose>
                <if variable="issued"><date variable="issued"><date-part name="year"/></date></if>
                <else>
                    <group prefix="[" suffix="]" font-style="italic">
                        <text term="no date" form="short"/>
                    </group>
                </else>
            </choose></layout></citation>
        </style>"#,
    )
    .expect("legacy style should parse");

    let extraction = extract_date_fallback(&style);
    assert!(extraction.unsupported, "{extraction:?}");
    assert_eq!(extraction.policy, None);
}

#[test]
fn migration_date_fallback_scan_terminates_on_cyclic_macros() {
    let style = parse_csl(
        r#"<style>
            <macro name="cycle-a"><text macro="cycle-b"/></macro>
            <macro name="cycle-b"><text macro="cycle-a"/></macro>
            <citation><layout><choose>
                <if variable="issued"><text macro="cycle-a"/></if>
                <else><text term="no date" form="short"/></else>
            </choose></layout></citation>
        </style>"#,
    )
    .expect("legacy style should parse");

    let extraction = extract_date_fallback(&style);
    assert_eq!(extraction.policy, None);
    assert!(!extraction.unsupported);
}

fn component_contains_issued_date(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Date(date) => date.date == DateVariable::Issued,
        TemplateComponent::Group(group) => group.group.iter().any(component_contains_issued_date),
        _ => false,
    }
}

fn component_contains_no_date_term(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Term(term) => term.term == GeneralTerm::NoDate,
        TemplateComponent::Group(group) => group.group.iter().any(component_contains_no_date_term),
        _ => false,
    }
}
