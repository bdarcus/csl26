/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test/bench/bin crate")]
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

//! BDD tests for Template V3 variant inheritance and structural diffs.
//!
//! These tests verify that complex inheritance chains and structural overlays
//! are resolved deterministically and predictably in `merge_style_overlay`.

use citum_schema_style::{
    Style, StyleDocumentFormat,
    locale::GeneralTerm,
    template::{SimpleVariable, TemplateComponent, TypeSelector},
};
use rstest::rstest;
use std::collections::HashSet;

fn create_base_style() -> Style {
    let yaml = r#"
version: "0.44.0"
info:
  title: Base Style
  id: base
bibliography:
  template:
    - title: primary
    - variable: doi
    - variable: url
  type-variants:
    book:
      - title: primary
      - variable: publisher
      - variable: url
"#;
    Style::from_yaml_str(yaml).expect("valid base style")
}

#[rstest]
#[case::override_rendering(
    r#"
extends: base
bibliography:
  type-variants:
    book:
      modify:
        - match: { title: primary }
          emph: true
"#,
    |style: &Style| {
        let bib = style.bibliography.as_ref().unwrap();
        let book = bib.type_variants.as_ref().unwrap().get(&TypeSelector::Single("book".into())).unwrap();
        let template = book.as_template().unwrap();
        assert_eq!(template[0].rendering().emph, Some(true));
        assert!(matches!(template[0], TemplateComponent::Title(_)));
    }
)]
#[case::add_component_before(
    r#"
extends: base
bibliography:
  type-variants:
    book:
      add:
        - before: { title: primary }
          component: { term: in }
"#,
    |style: &Style| {
        let bib = style.bibliography.as_ref().unwrap();
        let book = bib.type_variants.as_ref().unwrap().get(&TypeSelector::Single("book".into())).unwrap();
        let template = book.as_template().unwrap();
        assert_eq!(template.len(), 4);
        assert!(matches!(template[0], TemplateComponent::Term(_)));
        if let TemplateComponent::Term(t) = &template[0] {
            assert_eq!(t.term, GeneralTerm::In);
        }
    }
)]
#[case::remove_component(
    r#"
extends: base
bibliography:
  type-variants:
    book:
      remove:
        - match: { variable: publisher }
"#,
    |style: &Style| {
        let bib = style.bibliography.as_ref().unwrap();
        let book = bib.type_variants.as_ref().unwrap().get(&TypeSelector::Single("book".into())).unwrap();
        let template = book.as_template().unwrap();
        assert_eq!(template.len(), 2);
        assert!(!template.iter().any(|c| matches!(c, TemplateComponent::Variable(v) if v.variable == SimpleVariable::Publisher)));
    }
)]
#[case::deep_inheritance_explicit_extends(
    r#"
extends: base
bibliography:
  type-variants:
    book:
      extends: book
      modify:
        - match: { variable: publisher }
          strong: true
"#,
    |style: &Style| {
        let bib = style.bibliography.as_ref().unwrap();
        let book = bib.type_variants.as_ref().unwrap().get(&TypeSelector::Single("book".into())).unwrap();
        let template = book.as_template().unwrap();

        assert_eq!(template.len(), 3);
        let pub_comp = &template[1];
        assert_eq!(pub_comp.rendering().strong, Some(true));
    }
)]
fn test_template_variant_inheritance(#[case] overlay_yaml: &str, #[case] assertion: fn(&Style)) {
    let base = create_base_style();
    let overlay = Style::from_yaml_str(overlay_yaml).expect("valid overlay style");

    struct MockResolver(Style);
    impl citum_resolver_api::StyleResolver for MockResolver {
        type Style = Style;
        type Locale = citum_schema_style::locale::Locale;

        fn resolve_style(&self, _uri: &str) -> Result<Style, citum_schema_style::ResolverError> {
            Ok(self.0.clone())
        }

        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<Self::Locale, citum_schema_style::ResolverError> {
            Err(citum_schema_style::ResolverError::LocaleNotFound(
                std::borrow::Cow::Owned(id.to_string()),
            ))
        }
    }

    let resolver = MockResolver(base.clone());
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&resolver), &mut visited)
        .expect("resolution should succeed");

    assertion(&resolved);
}

#[rstest]
#[case::cross_variant_extension(
    r#"
bibliography:
  template: [{ title: primary }]
  type-variants:
    book:
      modify: [{ match: { title: primary }, emph: true }]
    thesis:
      extends: book
      add: [{ after: { title: primary }, component: { variable: doi } }]
"#,
    |style: &Style| {
        let bib = style.bibliography.as_ref().unwrap();
        let variants = bib.type_variants.as_ref().unwrap();

        let book = variants.get(&TypeSelector::Single("book".into())).unwrap().as_template().unwrap();
        assert_eq!(book[0].rendering().emph, Some(true));

        let thesis = variants.get(&TypeSelector::Single("thesis".into())).unwrap().as_template().unwrap();
        assert_eq!(thesis.len(), 2);
        assert_eq!(thesis[0].rendering().emph, Some(true));
        assert!(matches!(thesis[1], TemplateComponent::Variable(ref v) if v.variable == SimpleVariable::Doi));
    }
)]
fn test_intra_style_variant_extension(#[case] yaml: &str, #[case] assertion: fn(&Style)) {
    let style = Style::from_yaml_str(yaml).expect("valid style");
    let mut visited = HashSet::new();
    let resolved = style
        .try_into_resolved_recursive_with(None, &mut visited)
        .expect("resolution should succeed");
    assertion(&resolved);
}

#[test]
fn test_multiple_inheritance_levels() {
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
bibliography:
  template: [{ title: primary }]
  type-variants:
    book:
      modify: [{ match: { title: primary }, emph: true }]
"#;
    let mid_yaml = r#"
extends: base
info: { id: mid }
bibliography:
  type-variants:
    book:
      extends: book
      add: [{ after: { title: primary }, component: { variable: doi } }]
"#;
    let top_yaml = r#"
extends: mid
info: { id: top }
bibliography:
  type-variants:
    book:
      extends: book
      modify: [{ match: { variable: doi }, strong: true }]
"#;

    let base = Style::from_yaml_str(base_yaml).unwrap();
    let mid = Style::from_yaml_str(mid_yaml).unwrap();
    let top = Style::from_yaml_str(top_yaml).unwrap();

    struct MultiResolver {
        base: Style,
        mid: Style,
    }
    impl citum_resolver_api::StyleResolver for MultiResolver {
        type Style = Style;
        type Locale = citum_schema_style::locale::Locale;

        fn resolve_style(&self, uri: &str) -> Result<Style, citum_schema_style::ResolverError> {
            match uri {
                "base" => Ok(self.base.clone()),
                "mid" => Ok(self.mid.clone()),
                _ => Err(citum_schema_style::ResolverError::StyleNotFound(
                    std::borrow::Cow::Owned(uri.to_string()),
                )),
            }
        }

        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<Self::Locale, citum_schema_style::ResolverError> {
            Err(citum_schema_style::ResolverError::LocaleNotFound(
                std::borrow::Cow::Owned(id.to_string()),
            ))
        }
    }

    let resolver = MultiResolver { base, mid };
    let mut visited = HashSet::new();
    let resolved = top
        .try_into_resolved_recursive_with(Some(&resolver), &mut visited)
        .expect("deep resolution should succeed");

    let bib = resolved.bibliography.as_ref().unwrap();
    let book = bib
        .type_variants
        .as_ref()
        .unwrap()
        .get(&TypeSelector::Single("book".into()))
        .unwrap();
    let template = book.as_template().unwrap();

    assert_eq!(template.len(), 2);
    // Component 0: title (emph: true from base)
    assert_eq!(template[0].rendering().emph, Some(true));
    // Component 1: doi (strong: true from top)
    assert_eq!(template[1].rendering().strong, Some(true));
}

#[test]
fn test_overlay_preserves_base_type_variant_keys_not_in_overlay() {
    // Regression guard: overlay with only one type-variant key must not drop
    // base type-variant keys that are absent from the overlay.
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
bibliography:
  template:
    - title: primary
  type-variants:
    book:
      - title: primary
      - variable: publisher
    article-journal:
      - title: primary
      - variable: doi
"#;
    let overlay_yaml = r#"
extends: base
info: { id: overlay }
bibliography:
  type-variants:
    book:
      modify:
        - match: { title: primary }
          emph: true
"#;

    let base = Style::from_yaml_str(base_yaml).unwrap();
    let overlay = Style::from_yaml_str(overlay_yaml).unwrap();

    struct R(Style);
    impl citum_resolver_api::StyleResolver for R {
        type Style = Style;
        type Locale = citum_schema_style::locale::Locale;
        fn resolve_style(&self, _: &str) -> Result<Style, citum_schema_style::ResolverError> {
            Ok(self.0.clone())
        }
        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<Self::Locale, citum_schema_style::ResolverError> {
            Err(citum_schema_style::ResolverError::LocaleNotFound(
                std::borrow::Cow::Owned(id.to_string()),
            ))
        }
    }

    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&R(base)), &mut visited)
        .unwrap();

    let variants = resolved
        .bibliography
        .as_ref()
        .unwrap()
        .type_variants
        .as_ref()
        .unwrap();

    // The overlay only touched `book` — `article-journal` from base must survive.
    assert!(
        variants.contains_key(&TypeSelector::Single("article-journal".into())),
        "base article-journal variant dropped by overlay"
    );
    assert!(
        variants.contains_key(&TypeSelector::Single("book".into())),
        "book variant missing after overlay"
    );
}

// Regression tests: explicit `field: ~` in overlay must clear the inherited Option value.
// The typed merge_options! treats None as "absent"; these tests guard that raw_yaml
// null-inspection clears the base before merge_options! runs.

fn make_resolver(
    base: Style,
) -> impl citum_resolver_api::StyleResolver<Style = Style, Locale = citum_schema_style::locale::Locale>
{
    struct R(Style);
    impl citum_resolver_api::StyleResolver for R {
        type Style = Style;
        type Locale = citum_schema_style::locale::Locale;
        fn resolve_style(&self, _: &str) -> Result<Style, citum_schema_style::ResolverError> {
            Ok(self.0.clone())
        }
        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<Self::Locale, citum_schema_style::ResolverError> {
            Err(citum_schema_style::ResolverError::LocaleNotFound(
                std::borrow::Cow::Owned(id.to_string()),
            ))
        }
    }
    R(base)
}

#[test]
fn test_explicit_null_clears_citation_flat_field() {
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
citation:
  prefix: "("
  suffix: ")"
"#;
    let overlay_yaml = r#"
extends: base
info: { id: overlay }
citation:
  prefix: ~
"#;
    let base = Style::from_yaml_str(base_yaml).unwrap();
    let overlay = Style::from_yaml_str(overlay_yaml).unwrap();
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
        .unwrap();

    let cit = resolved.citation.as_ref().unwrap();
    assert!(
        cit.prefix.is_none(),
        "explicit `prefix: ~` did not clear inherited prefix"
    );
    assert_eq!(
        cit.suffix.as_deref(),
        Some(")"),
        "suffix not in overlay must survive"
    );
}

#[test]
fn test_explicit_null_clears_citation_options() {
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
citation:
  options: {}
"#;
    let overlay_yaml = r#"
extends: base
info: { id: overlay }
citation:
  options: ~
"#;
    let base = Style::from_yaml_str(base_yaml).unwrap();
    // Verify base has non-None options before resolution.
    assert!(
        base.citation.as_ref().unwrap().options.is_some(),
        "base must have citation.options for this test"
    );

    let overlay = Style::from_yaml_str(overlay_yaml).unwrap();
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
        .unwrap();

    assert!(
        resolved.citation.as_ref().unwrap().options.is_none(),
        "explicit `citation.options: ~` did not clear inherited options"
    );
}

#[test]
fn test_explicit_null_clears_bibliography_template() {
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
bibliography:
  template:
    - title: primary
    - variable: doi
"#;
    let overlay_yaml = r#"
extends: base
info: { id: overlay }
bibliography:
  template: ~
"#;
    let base = Style::from_yaml_str(base_yaml).unwrap();
    let overlay = Style::from_yaml_str(overlay_yaml).unwrap();
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
        .unwrap();

    assert!(
        resolved.bibliography.as_ref().unwrap().template.is_none(),
        "explicit `bibliography.template: ~` did not clear inherited template"
    );
}

#[test]
fn test_explicit_null_clears_bibliography_options() {
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
bibliography:
  options: {}
"#;
    let overlay_yaml = r#"
extends: base
info: { id: overlay }
bibliography:
  options: ~
"#;
    let base = Style::from_yaml_str(base_yaml).unwrap();
    assert!(
        base.bibliography.as_ref().unwrap().options.is_some(),
        "base must have bibliography.options for this test"
    );

    let overlay = Style::from_yaml_str(overlay_yaml).unwrap();
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
        .unwrap();

    assert!(
        resolved.bibliography.as_ref().unwrap().options.is_none(),
        "explicit `bibliography.options: ~` did not clear inherited options"
    );
}

/// Shared parent for the nested-option deep-merge cases: a fully populated
/// `dates` block at both global and bibliography scope.
fn nested_options_base_yaml() -> &'static str {
    r#"
version: "0.44.0"
info: { id: base }
options:
  dates:
    month: numeric
    uncertainty-marker: '?'
    approximation-marker: '['
    approximation-marker-suffix: ']'
    range-delimiter: "—"
bibliography:
  options:
    dates:
      month: numeric
      uncertainty-marker: '?'
      approximation-marker: '['
      approximation-marker-suffix: ']'
  template:
    - variable: doi
"#
}

#[rstest]
#[case::bibliography_scope_adds_one_field(
    r#"
extends: base
info: { id: overlay }
bibliography:
  options:
    dates:
      note-wrap: parentheses
"#,
    |resolved: &Style| {
        let dates = resolved
            .bibliography
            .as_ref()
            .expect("bibliography inherited")
            .options
            .as_ref()
            .expect("bibliography options inherited")
            .dates
            .as_ref()
            .expect("dates block present");
        assert!(dates.note_wrap.is_some(), "authored note-wrap must apply");
        assert_eq!(
            dates.approximation_marker.as_deref(),
            Some("["),
            "inherited sibling field must survive a partial override"
        );
        assert_eq!(dates.uncertainty_marker.as_deref(), Some("?"));
    }
)]
#[case::global_scope_scalar_replaces(
    r#"
extends: base
info: { id: overlay }
options:
  dates:
    range-delimiter: "-"
"#,
    |resolved: &Style| {
        let dates = resolved
            .options
            .as_ref()
            .expect("global options inherited")
            .dates
            .as_ref()
            .expect("dates block present");
        assert_eq!(dates.range_delimiter, "-", "authored scalar must replace");
        assert_eq!(
            dates.uncertainty_marker.as_deref(),
            Some("?"),
            "inherited sibling field must survive a scalar override"
        );
    }
)]
#[case::null_clears_one_optional_field(
    r#"
extends: base
info: { id: overlay }
options:
  dates:
    uncertainty-marker: ~
"#,
    |resolved: &Style| {
        let dates = resolved
            .options
            .as_ref()
            .expect("global options inherited")
            .dates
            .as_ref()
            .expect("dates block present");
        assert!(
            dates.uncertainty_marker.is_none(),
            "explicit null must clear the inherited field"
        );
        assert_eq!(
            dates.approximation_marker.as_deref(),
            Some("["),
            "sibling fields must survive a targeted null"
        );
    }
)]
#[case::preset_string_layers_over_inherited_block(
    r#"
extends: base
info: { id: overlay }
options:
  dates: numeric
"#,
    |resolved: &Style| {
        let dates = resolved
            .options
            .as_ref()
            .expect("global options inherited")
            .dates
            .as_ref()
            .expect("dates block present");
        assert_eq!(
            dates.month,
            citum_schema_style::options::MonthFormat::Numeric,
            "preset-defined field must apply"
        );
        assert_eq!(
            dates.approximation_marker_suffix.as_deref(),
            Some("]"),
            "optional field unset by the preset must inherit from the parent"
        );
        assert_eq!(
            dates.range_delimiter, "–",
            "non-optional preset fields fully determine their value"
        );
    }
)]
fn given_inherited_nested_options_when_child_partially_overrides_then_untouched_fields_survive(
    #[case] overlay_yaml: &str,
    #[case] assertion: fn(&Style),
) {
    let base = Style::from_yaml_str(nested_options_base_yaml()).expect("valid base style");
    let overlay = Style::from_yaml_str(overlay_yaml).expect("valid overlay style");
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
        .expect("resolution succeeds");
    assertion(&resolved);
}

/// A preset-string override for a scoped-option field (`contributors: springer`,
/// `substitute: standard`) must layer only the preset's own fields over the
/// inherited block, not whole-replace it — the preset target types resolve
/// eagerly at parse time (`deserialize_contributor_config`,
/// `deserialize_substitute_config`, etc.) precisely so the raw-YAML deep
/// merge sees a mapping to field-merge rather than a scalar to replace.
/// Regression coverage for a real bug found auditing `gb-t-7714-2025-*`:
/// `substitute: standard` was whole-replacing an inherited
/// `role-substitute` because `SubstituteConfig` lacked the eager-resolve
/// deserializer the other preset-target fields already had.
#[test]
fn preset_string_override_preserves_inherited_sibling_fields_not_covered_by_the_preset() {
    let base_yaml = r#"
version: "0.44.0"
info: { id: base }
options:
  contributors:
    demote-non-dropping-particle: never
  substitute:
    template:
      - editor
    role-substitute:
      container-author:
        - editor
        - collection-editor
"#;
    let overlay_yaml = r#"
extends: base
info: { id: overlay }
options:
  contributors: springer
  substitute: standard
"#;
    let base = Style::from_yaml_str(base_yaml).expect("valid base style");
    let overlay = Style::from_yaml_str(overlay_yaml).expect("valid overlay style");
    let mut visited = HashSet::new();
    let resolved = overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
        .expect("resolution succeeds");

    let options = resolved.options.as_ref().expect("global options inherited");

    let contributors = options
        .contributors
        .as_ref()
        .expect("contributors block present");
    assert_eq!(
        contributors.demote_non_dropping_particle,
        Some(citum_schema_style::options::DemoteNonDroppingParticle::Never),
        "the springer preset does not set demote-non-dropping-particle, so the \
         inherited value must survive"
    );
    assert_eq!(
        contributors.name_form,
        Some(citum_schema_style::options::NameForm::Initials),
        "fields the springer preset does define must still apply"
    );

    let substitute = options
        .substitute
        .as_ref()
        .expect("substitute block present")
        .resolve();
    assert_eq!(
        substitute.role_substitute.get("container-author"),
        Some(&vec!["editor".to_string(), "collection-editor".to_string()]),
        "the standard preset does not set role-substitute, so the inherited \
         chain must survive"
    );
    assert_eq!(
        substitute.template,
        vec![
            citum_schema_style::options::SubstituteKey::Editor,
            citum_schema_style::options::SubstituteKey::Title,
            citum_schema_style::options::SubstituteKey::Translator,
        ],
        "the standard preset's own template must apply"
    );
}

/// A JSON-authored child produces the same resolved style as its YAML
/// equivalent — the raw-presence basis deep merge reads from must be
/// format-neutral (`csl26-j3zy`), not YAML-specific.
#[test]
fn json_authored_child_deep_merges_identically_to_yaml_equivalent() {
    let base = Style::from_yaml_str(nested_options_base_yaml()).expect("valid base style");

    let yaml_overlay = Style::from_yaml_str(
        r#"
extends: base
info: { id: overlay }
bibliography:
  options:
    dates:
      note-wrap: parentheses
"#,
    )
    .expect("valid yaml overlay");

    let json_overlay = Style::from_document_bytes(
        br#"{
            "extends": "base",
            "info": { "id": "overlay" },
            "bibliography": { "options": { "dates": { "note-wrap": "parentheses" } } }
        }"#,
        StyleDocumentFormat::Json,
    )
    .expect("valid json overlay");

    let mut yaml_visited = HashSet::new();
    let yaml_resolved = yaml_overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base.clone())), &mut yaml_visited)
        .expect("yaml resolution succeeds");

    let mut json_visited = HashSet::new();
    let json_resolved = json_overlay
        .try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut json_visited)
        .expect("json resolution succeeds");

    let yaml_dates = yaml_resolved
        .bibliography
        .as_ref()
        .and_then(|b| b.options.as_ref())
        .and_then(|o| o.dates.as_ref())
        .expect("yaml dates present");
    let json_dates = json_resolved
        .bibliography
        .as_ref()
        .and_then(|b| b.options.as_ref())
        .and_then(|o| o.dates.as_ref())
        .expect("json dates present");

    assert_eq!(
        json_dates, yaml_dates,
        "a JSON-authored partial override must deep-merge identically to the \
         same override authored in YAML"
    );
    assert!(
        json_dates.note_wrap.is_some(),
        "authored note-wrap must apply"
    );
    assert_eq!(
        json_dates.approximation_marker.as_deref(),
        Some("["),
        "inherited sibling field must survive regardless of authoring format"
    );
}

/// Explicit `null` on a non-`Option` scalar that carries a serde default
/// (e.g. `dates.range-delimiter`, a `String` with `#[serde(default = ...)]`)
/// is a parse error, not a silent reset to the default — STYLE_INHERITANCE.md
/// rule 3. If this ever stops erroring, the raw deep-merge path would fall
/// back to the typed whole-field merge without anyone noticing.
#[test]
fn null_on_defaulted_non_option_scalar_is_a_parse_error() {
    let yaml = r#"
version: "0.44.0"
info: { id: base }
options:
  dates:
    range-delimiter: ~
"#;
    let result = Style::from_yaml_str(yaml);
    assert!(
        result.is_err(),
        "null on a non-Option scalar field must fail to parse, not silently \
         reset to the field's serde default"
    );
}
