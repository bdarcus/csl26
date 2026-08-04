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

//! Tests for bibliography base-template layout-order preservation.
//!
//! The base template must follow the DEFAULT (else) branch order of the CSL
//! layout, take component shape from the default branch occurrence, and keep
//! type-conditional output (even behind nested variable conditions)
//! suppressed by default.

use citum_migrate::{Compressor, MacroInliner, TemplateCompiler, Upsampler};
use citum_schema::template::TemplateComponent;
use csl_legacy::parser::parse_style;
use roxmltree::Document;

fn announce_behavior(summary: &str) {
    tracing::debug!("behavior: {summary}");
}

fn parse_csl(xml: &str) -> Result<csl_legacy::model::Style, String> {
    let doc = Document::parse(xml).map_err(|err| err.to_string())?;
    parse_style(doc.root_element()).map_err(|err| err.clone())
}

fn compile_bibliography_base(xml: &str) -> Vec<TemplateComponent> {
    let style = parse_csl(xml).expect("legacy style should parse");
    let inliner = MacroInliner::new(&style);
    let flattened = inliner
        .inline_bibliography(&style)
        .expect("bibliography should exist");
    let raw_bib = Upsampler::new().upsample_nodes(&flattened);
    let bib_ir = Compressor.compress_nodes(raw_bib);
    let (base_template, _) = TemplateCompiler.compile_bibliography_with_types(&bib_ir, true);
    base_template
}

/// Short identification key for a component, for order assertions.
fn component_key(component: &TemplateComponent) -> String {
    match component {
        TemplateComponent::Contributor(c) => c.contributor.as_single().map_or_else(
            || format!("contributor:{:?}", c.contributor.as_slice()),
            |role| format!("contributor:{role:?}"),
        ),
        TemplateComponent::Date(d) => format!("date:{:?}", d.date),
        TemplateComponent::Title(t) => format!("title:{:?}", t.title),
        TemplateComponent::Number(n) => format!("number:{:?}", n.number),
        TemplateComponent::Variable(v) => format!("variable:{:?}", v.variable),
        TemplateComponent::Term(t) => format!("term:{:?}", t.term),
        TemplateComponent::Group(_) => "group".to_string(),
        _ => "other".to_string(),
    }
}

fn is_suppressed(component: &TemplateComponent) -> bool {
    let suppress = match component {
        TemplateComponent::Contributor(c) => c.rendering.suppress,
        TemplateComponent::Date(d) => d.rendering.suppress,
        TemplateComponent::Title(t) => t.rendering.suppress,
        TemplateComponent::Number(n) => n.rendering.suppress,
        TemplateComponent::Variable(v) => v.rendering.suppress,
        TemplateComponent::Term(t) => t.rendering.suppress,
        TemplateComponent::Group(g) => g.rendering.suppress,
        _ => None,
    };
    suppress == Some(true)
}

/// A numeric-journal layout shaped like brazilian-journal-of-psychiatry: a
/// type choose whose ELSE branch carries the article-journal rendering, plus a
/// type-gated access macro.
const VANCOUVER_LIKE_XML: &str = r#"<style class="in-text">
    <macro name="author">
        <names variable="author"/>
    </macro>
    <macro name="access">
        <choose>
            <if type="webpage" match="any">
                <choose>
                    <if variable="URL">
                        <text term="available at" suffix=" "/>
                        <text variable="URL"/>
                    </if>
                </choose>
            </if>
        </choose>
    </macro>
    <citation><layout><text variable="citation-number"/></layout></citation>
    <bibliography>
        <layout>
            <text variable="citation-number"/>
            <text macro="author" suffix=". "/>
            <choose>
                <if type="book" match="any">
                    <text variable="title"/>
                    <group delimiter=" ">
                        <label variable="volume" form="short"/>
                        <text variable="volume"/>
                    </group>
                    <text variable="publisher"/>
                </if>
                <else>
                    <text variable="title"/>
                    <text variable="container-title" form="short"/>
                    <date variable="issued"><date-part name="year"/></date>
                    <text variable="volume"/>
                    <text variable="page"/>
                </else>
            </choose>
            <text macro="access"/>
        </layout>
    </bibliography>
</style>"#;

#[test]
fn base_template_visible_components_follow_default_branch_order() {
    announce_behavior(
        "The visible base-template components follow the CSL default (else) branch order, not the earliest occurrence across type branches.",
    );
    let base_template = compile_bibliography_base(VANCOUVER_LIKE_XML);

    let visible_keys: Vec<String> = base_template
        .iter()
        .filter(|component| !is_suppressed(component))
        .map(component_key)
        .collect();

    assert_eq!(
        visible_keys,
        // The reference marker is no longer a template component; the style
        // declares label-mode and the processor materializes it.
        // See docs/specs/REFERENCE_MARKERS.md.
        vec![
            "contributor:Author",
            "title:Primary",
            "title:ContainerTitle",
            "date:Issued",
            "number:Volume",
            "number:Pages",
        ],
        "visible base-template order should match the default branch layout order"
    );
}

#[test]
fn type_gated_access_components_stay_suppressed_in_base_template() {
    announce_behavior(
        "Components behind a variable condition nested inside a type condition remain type-conditional: the access macro must not leak 'available at' into every entry.",
    );
    let base_template = compile_bibliography_base(VANCOUVER_LIKE_XML);

    let leaked: Vec<String> = base_template
        .iter()
        .filter(|component| !is_suppressed(component))
        .map(component_key)
        .filter(|key| key.contains("AvailableAt") || key.contains("variable:Url"))
        .collect();

    assert!(
        leaked.is_empty(),
        "access-macro components must be suppressed in the base template, found visible: {leaked:?}"
    );
}

#[test]
fn base_component_shape_comes_from_default_branch_occurrence() {
    announce_behavior(
        "When a variable renders with a label in a type branch but bare in the default branch, the base component takes the bare default shape.",
    );
    let base_template = compile_bibliography_base(VANCOUVER_LIKE_XML);

    let volume = base_template
        .iter()
        .find_map(|component| match component {
            TemplateComponent::Number(n)
                if matches!(n.number, citum_schema::template::NumberVariable::Volume) =>
            {
                Some(n)
            }
            _ => None,
        })
        .expect("base template should contain a volume component");

    assert_eq!(
        volume.label_form, None,
        "volume label from the book branch must not leak into the default shape"
    );
}

#[test]
fn csl_section_and_collection_title_compile_to_distinct_citum_components() {
    announce_behavior(
        "CSL section migrates to a simple section variable, while collection-title migrates to the collection-title title surface rather than a parent monograph title.",
    );
    let xml = r#"<style class="in-text">
        <citation><layout><text variable="citation-number"/></layout></citation>
        <bibliography>
            <layout>
                <text variable="section"/>
                <text variable="collection-title"/>
            </layout>
        </bibliography>
    </style>"#;

    let base_template = compile_bibliography_base(xml);
    let visible_keys: Vec<String> = base_template
        .iter()
        .filter(|component| !is_suppressed(component))
        .map(component_key)
        .collect();

    assert_eq!(
        visible_keys,
        vec!["variable:Section", "title:CollectionTitle"],
        "section and collection-title should keep distinct Citum semantics"
    );
}
