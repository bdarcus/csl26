/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in tests."
)]

use citum_migrate::{compilation, fixups::normalize_author_date_locator_citation_component};
use citum_schema::template::{
    ContributorLabelMode, ContributorMergeOrder, ContributorRole, DateVariable, Rendering,
    SimpleVariable, TemplateComponent, TemplateDate, TemplateVariable,
};
use csl_legacy::{
    model::{CslNode, Formatting, Group, Layout, Text},
    parser::parse_style,
};
use roxmltree::Document;

#[test]
fn compile_from_xml_emits_ordered_localized_templates_with_default() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>localized-test</title><id>https://example.org/localized-test</id></info>
  <citation>
    <layout locale="en-US"><text variable="title"/></layout>
    <layout locale="zh-CN"><text variable="publisher"/></layout>
    <layout><text variable="DOI"/></layout>
  </citation>
  <bibliography>
    <layout locale="en-US"><text variable="title"/></layout>
    <layout locale="zh-CN"><text variable="publisher"/></layout>
    <layout><text variable="DOI"/></layout>
  </bibliography>
</style>
"#,
    );
    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);

    let output = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    let citation_locales = output
        .citation_locales
        .clone()
        .expect("citation locale branches should be emitted");
    assert_eq!(citation_locales.len(), 3);
    assert_eq!(
        citation_locales
            .first()
            .and_then(|branch| branch.locale.as_deref()),
        Some(&["en-US".to_string()][..])
    );
    assert_eq!(
        citation_locales
            .get(1)
            .and_then(|branch| branch.locale.as_deref()),
        Some(&["zh-CN".to_string()][..])
    );
    assert_eq!(
        citation_locales.get(2).and_then(|branch| branch.default),
        Some(true)
    );
    let citation = citum_schema::CitationSpec {
        template: Some(output.citation.clone()),
        locales: Some(citation_locales),
        ..Default::default()
    };
    assert_eq!(
        citation
            .resolve_localized_template(Some("en-US"))
            .expect("exact locale should resolve")
            .locale
            .as_deref(),
        Some("en-US")
    );
    assert_eq!(
        citation
            .resolve_localized_template(Some("en-GB"))
            .expect("primary language should resolve")
            .locale
            .as_deref(),
        Some("en-US")
    );
    assert_eq!(
        citation
            .resolve_localized_template(Some("fr-FR"))
            .expect("default branch should resolve")
            .locale,
        None
    );
    assert_eq!(
        output
            .bibliography_locales
            .as_ref()
            .expect("bibliography locale branches should be emitted")
            .len(),
        3
    );
    assert!(!output.unsupported_localized_layouts);
}

#[test]
fn compile_from_xml_keeps_identical_localized_template_branch() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>identical-test</title><id>https://example.org/identical-test</id></info>
  <citation>
    <layout locale="zh-CN"><text variable="title"/></layout>
    <layout><text variable="title"/></layout>
  </citation>
</style>
"#,
    );
    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);

    let output = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    assert_eq!(
        output
            .citation_locales
            .as_ref()
            .expect("identical locale branch should not be deduplicated")
            .len(),
        2
    );
}

#[test]
fn compile_from_xml_conventional_layout_has_no_localized_branches() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>conventional-test</title><id>https://example.org/conventional-test</id></info>
  <citation><layout><text variable="title"/></layout></citation>
</style>
"#,
    );
    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);

    let output = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    assert!(output.citation_locales.is_none());
    assert!(output.bibliography_locales.is_none());
    assert!(!output.unsupported_localized_layouts);
}

#[test]
fn compile_from_xml_diagnoses_locale_specific_layout_wrapper() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>wrapper-test</title><id>https://example.org/wrapper-test</id></info>
  <citation>
    <layout locale="zh-CN" prefix="（" suffix="）"><text variable="title"/></layout>
    <layout prefix="(" suffix=")"><text variable="title"/></layout>
  </citation>
</style>
"#,
    );
    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);

    let output = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    assert!(output.unsupported_localized_layouts);
}

#[test]
fn compile_from_xml_preserves_locale_specific_type_variant() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>type-variant-test</title><id>https://example.org/type-variant-test</id></info>
  <citation><layout><text variable="title"/></layout></citation>
  <bibliography>
    <layout locale="zh-CN">
      <choose>
        <if type="book"><text variable="publisher"/></if>
        <else><text variable="title"/></else>
      </choose>
    </layout>
    <layout>
      <choose>
        <if type="book"><text variable="title"/></if>
        <else><text variable="title"/></else>
      </choose>
    </layout>
  </bibliography>
</style>
"#,
    );
    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);

    let output = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    let localized = output
        .bibliography_locales
        .as_ref()
        .and_then(|locales| locales.first())
        .expect("localized bibliography branch should be emitted");
    assert!(localized.type_variants.is_some());
    assert!(!output.unsupported_localized_layouts);
}

#[test]
fn author_date_locator_prefers_group_delimiter() {
    let layout = Layout {
        prefix: None,
        suffix: None,
        delimiter: None,
        children: vec![CslNode::Group(Group {
            delimiter: Some(", ".into()),
            prefix: None,
            suffix: None,
            children: vec![
                CslNode::Text(Text {
                    value: None,
                    variable: None,
                    macro_name: Some("author-short".to_string()),
                    term: None,
                    form: None,
                    prefix: None,
                    suffix: None,
                    quotes: None,
                    text_case: None,
                    strip_periods: None,
                    plural: None,
                    macro_call_order: None,
                    formatting: Formatting::default(),
                }),
                CslNode::Text(Text {
                    value: None,
                    variable: None,
                    macro_name: Some("issued-year".to_string()),
                    term: None,
                    form: None,
                    prefix: None,
                    suffix: None,
                    quotes: None,
                    text_case: None,
                    strip_periods: None,
                    plural: None,
                    macro_call_order: None,
                    formatting: Formatting::default(),
                }),
                CslNode::Text(Text {
                    value: None,
                    variable: None,
                    macro_name: Some("citation-locator".to_string()),
                    term: None,
                    form: None,
                    prefix: None,
                    suffix: None,
                    quotes: None,
                    text_case: None,
                    strip_periods: None,
                    plural: None,
                    macro_call_order: None,
                    formatting: Formatting::default(),
                }),
            ],
            macro_call_order: None,
            formatting: Formatting::default(),
        })],
    };
    let mut template = vec![
        TemplateComponent::Contributor(citum_schema::template::TemplateContributor {
            contributor: citum_schema::template::ContributorRole::Author.into(),
            form: citum_schema::template::ContributorForm::Short,
            name_order: Some(citum_schema::template::NameOrder::FamilyFirst),
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: citum_schema::template::DateForm::Year,
            rendering: Rendering {
                prefix: Some(", ".into()),
                ..Default::default()
            },
            ..Default::default()
        }),
        TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Locator,
            rendering: Rendering {
                prefix: Some(" ".into()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ];

    normalize_author_date_locator_citation_component(&layout, &[], &mut template);

    let locator = template
        .iter()
        .find_map(|component| match component {
            TemplateComponent::Variable(variable)
                if variable.variable == SimpleVariable::Locator =>
            {
                Some(variable)
            }
            _ => None,
        })
        .expect("locator component should exist");

    assert_eq!(locator.rendering.prefix.as_deref(), Some(", "));
}

#[test]
fn compile_from_xml_drops_the_citation_label_variable_from_the_template() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>label-test</title>
    <id>https://example.org/label-test</id>
  </info>
  <citation collapse="citation-number">
    <layout prefix="[" suffix="]" delimiter=", ">
      <text variable="citation-label"/>
    </layout>
  </citation>
</style>
"#,
    );

    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);
    let out = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    // The reference marker is processor-owned: the style declares `label-mode`
    // and the processor materializes it, so no component is emitted.
    // See docs/specs/REFERENCE_MARKERS.md.
    assert!(
        out.citation.is_empty(),
        "citation-label should not compile to a template component, got: {:?}",
        out.citation
    );
}

#[test]
fn compile_from_xml_preserves_all_names_variables_as_a_merged_component() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>multi-variable-names-test</title>
    <id>https://example.org/multi-variable-names-test</id>
  </info>
  <bibliography>
    <layout>
      <names variable="editor translator">
        <name and="text"/>
        <label form="short" prefix=", "/>
      </names>
    </layout>
  </bibliography>
</style>
"#,
    );

    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);
    let out = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);
    let contributor = out
        .bibliography
        .iter()
        .find_map(|component| match component {
            TemplateComponent::Contributor(contributor) => Some(contributor),
            _ => None,
        })
        .expect("multi-variable names should compile to a contributor component");
    let merge = contributor
        .merge
        .as_ref()
        .expect("multi-variable names should retain merge metadata");

    assert_eq!(
        contributor.contributor.as_slice(),
        &[ContributorRole::Editor, ContributorRole::Translator]
    );
    assert_eq!(merge.order, ContributorMergeOrder::Role);
    assert_eq!(merge.labels, ContributorLabelMode::Collective);
    assert!(merge.combine_same_person);
    assert_eq!(
        merge
            .roles
            .get(&ContributorRole::Editor)
            .and_then(|role| role.label.as_ref())
            .map(|label| label.term.as_str()),
        Some("editor")
    );
    assert_eq!(
        merge
            .roles
            .get(&ContributorRole::Translator)
            .and_then(|role| role.label.as_ref())
            .map(|label| label.term.as_str()),
        Some("translator")
    );

    let actual = serde_yaml::to_value(TemplateComponent::Contributor(contributor.clone()))
        .expect("compiled component should serialize");
    let expected: serde_yaml::Value = serde_yaml::from_str(
        r#"
contributor: [editor, translator]
form: long
merge:
  order: role
  labels: collective
  roles:
    editor:
      labels: collective
      label:
        term: editor
        form: short
        placement: suffix
        prefix: { mark: comma }
    translator:
      labels: collective
      label:
        term: translator
        form: short
        placement: suffix
        prefix: { mark: comma }
  combine-same-person: true
and: text
suppress: false
"#,
    )
    .expect("expected component YAML should parse");
    assert_eq!(actual, expected);
}

#[test]
fn compile_from_xml_maps_nested_position_chooses_into_citation_overrides() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="note">
  <info>
    <title>position-test</title>
    <id>https://example.org/position-test</id>
  </info>
  <citation>
    <layout>
      <group delimiter=" ">
        <text value="prefix"/>
        <choose>
          <if position="subsequent">
            <text variable="author"/>
          </if>
          <else-if position="first">
            <text variable="title"/>
          </else-if>
          <else>
            <date variable="issued">
              <date-part name="year"/>
            </date>
          </else>
        </choose>
        <choose>
          <if position="ibid-with-locator">
            <group delimiter=" ">
              <text term="ibid"/>
              <text variable="locator"/>
            </group>
          </if>
          <else-if position="ibid">
            <text term="ibid"/>
          </else-if>
          <else>
            <date variable="issued">
              <date-part name="year"/>
            </date>
          </else>
        </choose>
        <text value="suffix"/>
      </group>
    </layout>
  </citation>
</style>
"#,
    );

    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);
    let out = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    assert!(
        template_contains(&out.citation, &|component| matches!(
            component,
            TemplateComponent::Title(_)
        )),
        "explicit first-position branch should become part of the base citation template"
    );
    assert!(
        template_contains(&out.citation, &|component| matches!(
            component,
            TemplateComponent::Date(_)
        )),
        "fallback content from sibling chooses should remain in the base citation template"
    );

    assert_position_override_shapes(&out);
}

#[test]
fn compile_from_xml_maps_mixed_note_position_tree_into_citation_overrides() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="note">
  <info>
    <title>mixed-note-position-test</title>
    <id>https://example.org/mixed-note-position-test</id>
  </info>
  <citation>
    <layout>
      <choose>
        <if position="subsequent">
          <group delimiter=", ">
            <text variable="author"/>
            <choose>
              <if match="any" variable="archive archive-place container-title DOI number publisher references URL"/>
              <else-if position="first" type="interview">
                <date variable="issued">
                  <date-part name="year"/>
                </date>
              </else-if>
              <else-if position="first" type="personal_communication">
                <text variable="publisher"/>
              </else-if>
            </choose>
          </group>
        </if>
        <else>
          <text variable="title"/>
        </else>
      </choose>
    </layout>
  </citation>
</style>
"#,
    );

    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);
    let out = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    assert!(
        out.citation
            .iter()
            .any(|component| matches!(component, TemplateComponent::Title(_))),
        "base citation template should still contain the first-citation title"
    );
    assert!(
        out.citation_overrides.subsequent.is_some(),
        "mixed note trees should now emit a subsequent override"
    );
    assert!(
        out.citation_overrides
            .subsequent
            .as_ref()
            .is_some_and(|template| template
                .iter()
                .any(|component| matches!(component, TemplateComponent::Contributor(_)))),
        "subsequent override should preserve the shortened-note contributor content"
    );
}

#[test]
fn compile_from_xml_truly_unsupported_mixed_position_tree_falls_back_without_overrides() {
    let legacy_style = parse_legacy_style(
        r#"
<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="note">
  <info>
    <title>unsupported-mixed-position-test</title>
    <id>https://example.org/unsupported-mixed-position-test</id>
  </info>
  <citation>
    <layout>
      <choose>
        <if variable="title">
          <text variable="title"/>
        </if>
        <else-if position="subsequent">
          <text variable="author"/>
        </else-if>
        <else>
          <text variable="publisher"/>
        </else>
      </choose>
    </layout>
  </citation>
</style>
"#,
    );

    let mut options = citum_schema::options::Config::default();
    let tracker = citum_migrate::provenance::ProvenanceTracker::new(false);
    let out = compilation::compile_from_xml(&legacy_style, &mut options, false, &tracker);

    assert!(
        !out.citation.is_empty(),
        "unsupported trees must still compile a base citation template"
    );
    assert!(
        out.citation_overrides.subsequent.is_none() && out.citation_overrides.ibid.is_none(),
        "unsupported trees should not emit partial position overrides"
    );
}

fn parse_legacy_style(xml: &str) -> csl_legacy::model::Style {
    let doc = Document::parse(xml).expect("test style XML should parse");
    parse_style(doc.root_element()).expect("legacy style parsing should succeed")
}

fn template_contains(
    components: &[TemplateComponent],
    predicate: &dyn Fn(&TemplateComponent) -> bool,
) -> bool {
    components.iter().any(|component| {
        predicate(component)
            || matches!(
                component,
                TemplateComponent::Group(group) if template_contains(&group.group, predicate)
            )
    })
}

fn assert_position_override_shapes(out: &compilation::XmlCompilationOutput) {
    let subsequent_template = out
        .citation_overrides
        .subsequent
        .as_ref()
        .expect("subsequent branch should be migrated");
    assert!(
        template_contains(subsequent_template, &|component| matches!(
            component,
            TemplateComponent::Contributor(_)
        )),
        "subsequent override should preserve author short-form branch"
    );
    assert!(
        template_contains(subsequent_template, &|component| matches!(
            component,
            TemplateComponent::Date(_)
        )),
        "sibling choose fallback content should remain in the subsequent override"
    );

    let ibid_template = out
        .citation_overrides
        .ibid
        .as_ref()
        .expect("ibid branch should be migrated");
    assert!(
        template_contains(ibid_template, &|component| matches!(
            component,
            TemplateComponent::Variable(variable) if variable.variable == SimpleVariable::Locator
        )),
        "merged ibid override should preserve locator-aware content"
    );
    assert!(
        template_contains(ibid_template, &|component| matches!(
            component,
            TemplateComponent::Term(term)
                if term.term == citum_schema::locale::GeneralTerm::Ibid
        )),
        "merged ibid override should still contain the ibid term"
    );
}
