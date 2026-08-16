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
    ResolutionError, Style, StyleDocumentFormat,
    locale::GeneralTerm,
    template::{DateVariable, SimpleVariable, TemplateComponent, TemplateVariant, TypeSelector},
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

#[derive(Clone, Copy, Debug)]
enum FallbackSurface {
    Citation,
    Bibliography,
}

impl FallbackSurface {
    fn key(self) -> &'static str {
        match self {
            Self::Citation => "citation",
            Self::Bibliography => "bibliography",
        }
    }

    fn template(self, style: &Style) -> Option<&[TemplateComponent]> {
        match self {
            Self::Citation => style.citation.as_ref()?.template.as_ref(),
            Self::Bibliography => style.bibliography.as_ref()?.template.as_ref(),
        }
        .and_then(TemplateVariant::as_template)
    }

    fn resolves_template(self, style: &Style) -> bool {
        match self {
            Self::Citation => style
                .citation
                .as_ref()
                .and_then(|spec| spec.resolve_template()),
            Self::Bibliography => style
                .bibliography
                .as_ref()
                .and_then(|spec| spec.resolve_template()),
        }
        .is_some()
    }
}

fn fallback_style_yaml(
    id: &str,
    extends: Option<&str>,
    surface: FallbackSurface,
    body: &str,
) -> String {
    let extends = extends.map_or_else(String::new, |parent| format!("extends: {parent}\n"));
    let body = body
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "version: \"0.44.0\"\ninfo: {{ id: {id} }}\n{extends}{}:\n{body}\n",
        surface.key()
    )
}

fn resolve_fallback_overlay(base_yaml: &str, overlay_yaml: &str) -> Result<Style, ResolutionError> {
    let base = parse_fallback_style(base_yaml);
    let overlay = parse_fallback_style(overlay_yaml);
    let mut visited = HashSet::new();
    overlay.try_into_resolved_recursive_with(Some(&make_resolver(base)), &mut visited)
}

fn parse_fallback_style(yaml: &str) -> Style {
    Style::from_yaml_str(yaml).expect("fallback conformance style should parse")
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_list_fallback_when_parsed_then_it_remains_a_full_template(
    #[case] surface: FallbackSurface,
) {
    let yaml = fallback_style_yaml(
        "full-template",
        None,
        surface,
        "template:\n  - title: primary\n  - variable: doi",
    );
    let style = Style::from_yaml_str(&yaml).expect("legacy list template should parse");

    let variant = match surface {
        FallbackSurface::Citation => style
            .citation
            .as_ref()
            .and_then(|spec| spec.template.as_ref()),
        FallbackSurface::Bibliography => style
            .bibliography
            .as_ref()
            .and_then(|spec| spec.template.as_ref()),
    };
    assert!(matches!(variant, Some(TemplateVariant::Full(template)) if template.len() == 2));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_parent_full_fallback_when_child_removes_component_then_diff_resolves_to_full(
    #[case] surface: FallbackSurface,
) {
    let base = fallback_style_yaml(
        "base",
        None,
        surface,
        "template:\n  - title: primary\n  - variable: doi",
    );
    let overlay = fallback_style_yaml(
        "overlay",
        Some("base"),
        surface,
        "template:\n  remove:\n    - match: { variable: doi }",
    );
    let resolved =
        resolve_fallback_overlay(&base, &overlay).expect("inherited fallback diff should resolve");
    let template = surface
        .template(&resolved)
        .expect("resolved fallback should be full");

    assert_eq!(template.len(), 1);
    assert!(matches!(template[0], TemplateComponent::Title(_)));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_parent_template_ref_when_child_diffs_fallback_then_reference_is_the_base(
    #[case] surface: FallbackSurface,
) {
    let base = fallback_style_yaml("base", None, surface, "template-ref: chicago-author-date");
    let overlay = fallback_style_yaml(
        "overlay",
        Some("base"),
        surface,
        "template:\n  remove:\n    - match: { date: issued }",
    );
    let resolved = resolve_fallback_overlay(&base, &overlay)
        .expect("a parent template reference should provide the diff base");
    let template = surface
        .template(&resolved)
        .expect("resolved fallback should be full");

    assert!(!template.iter().any(
        |component| matches!(component, TemplateComponent::Date(date) if date.date == DateVariable::Issued)
    ));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_root_fallback_diff_when_resolved_then_missing_base_is_structured_error(
    #[case] surface: FallbackSurface,
) {
    let yaml = fallback_style_yaml(
        "root-diff",
        None,
        surface,
        "template:\n  remove:\n    - match: { variable: doi }",
    );
    let error = Style::from_yaml_str(&yaml)
        .expect("diff syntax should parse")
        .try_into_resolved()
        .expect_err("root diff must not resolve without a base");

    assert!(matches!(
        error,
        ResolutionError::InvalidFallbackTemplateDiff { location, reason }
            if location == format!("root-diff.{}.template", surface.key())
                && reason == "no inherited fallback template is available"
    ));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_fallback_diff_extends_when_resolved_then_it_is_rejected(#[case] surface: FallbackSurface) {
    let yaml = fallback_style_yaml(
        "fallback-extends",
        None,
        surface,
        "template:\n  extends: book\n  remove:\n    - match: { variable: doi }",
    );
    let error = Style::from_yaml_str(&yaml)
        .expect("diff syntax should parse")
        .try_into_resolved()
        .expect_err("fallback extends must be rejected");

    assert!(matches!(
        error,
        ResolutionError::InvalidFallbackTemplateDiff { reason, .. }
            if reason.starts_with("extends is not allowed")
    ));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_template_ref_and_diff_in_one_section_when_resolved_then_conflict_is_rejected(
    #[case] surface: FallbackSurface,
) {
    let yaml = fallback_style_yaml(
        "conflicting-base",
        None,
        surface,
        "template-ref: chicago-author-date\ntemplate:\n  remove:\n    - match: { date: issued }",
    );
    let error = Style::from_yaml_str(&yaml)
        .expect("conflicting fields should parse independently")
        .try_into_resolved()
        .expect_err("same-section base selection must be rejected");

    assert!(matches!(
        error,
        ResolutionError::InvalidFallbackTemplateDiff { reason, .. }
            if reason == "template-ref and a template diff cannot be declared in the same section"
    ));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_parent_template_ref_when_child_clears_template_then_fallback_is_absent(
    #[case] surface: FallbackSurface,
) {
    let base = fallback_style_yaml("base", None, surface, "template-ref: chicago-author-date");
    let overlay = fallback_style_yaml("overlay", Some("base"), surface, "template: ~");
    let resolved = resolve_fallback_overlay(&base, &overlay)
        .expect("explicit null should clear an inherited fallback");

    assert!(!surface.resolves_template(&resolved));
}

#[rstest]
#[case::citation_missing(
    FallbackSurface::Citation,
    "template:\n  - title: primary",
    "template:\n  remove:\n    - match: { variable: doi }",
    false
)]
#[case::bibliography_missing(
    FallbackSurface::Bibliography,
    "template:\n  - title: primary",
    "template:\n  remove:\n    - match: { variable: doi }",
    false
)]
#[case::citation_ambiguous(
    FallbackSurface::Citation,
    "template:\n  - title: primary\n  - title: primary",
    "template:\n  remove:\n    - match: { title: primary }",
    true
)]
#[case::bibliography_ambiguous(
    FallbackSurface::Bibliography,
    "template:\n  - title: primary\n  - title: primary",
    "template:\n  remove:\n    - match: { title: primary }",
    true
)]
fn given_invalid_fallback_selector_when_resolved_then_existing_selector_error_is_preserved(
    #[case] surface: FallbackSurface,
    #[case] base_body: &str,
    #[case] overlay_body: &str,
    #[case] ambiguous: bool,
) {
    let base = fallback_style_yaml("base", None, surface, base_body);
    let overlay = fallback_style_yaml("overlay", Some("base"), surface, overlay_body);
    let error = resolve_fallback_overlay(&base, &overlay)
        .expect_err("invalid fallback selector must not resolve");

    assert!(
        matches!(
            error,
            ResolutionError::TemplateVariantAmbiguousAnchor { .. }
        ) == ambiguous
            && matches!(error, ResolutionError::TemplateVariantAnchorNotFound { .. }) != ambiguous
    );
}

#[test]
fn nested_citation_fallback_diff_uses_the_effective_outer_fallback() {
    let yaml = r#"
version: "0.44.0"
info: { id: nested-fallback }
citation:
  template:
    - title: primary
    - variable: publisher
  subsequent:
    integral:
      template:
        remove:
          - match: { variable: publisher }
"#;
    let resolved = Style::from_yaml_str(yaml)
        .expect("nested fallback diff should parse")
        .try_into_resolved()
        .expect("nested fallback diff should use the effective outer fallback");
    let template = resolved
        .citation
        .as_ref()
        .and_then(|citation| citation.subsequent.as_deref())
        .and_then(|subsequent| subsequent.integral.as_deref())
        .and_then(|integral| integral.template.as_ref())
        .and_then(TemplateVariant::as_template)
        .expect("nested fallback should resolve to full");

    assert_eq!(template.len(), 1);
    assert!(matches!(template[0], TemplateComponent::Title(_)));
}

#[test]
fn inherited_nested_citation_fallback_diff_uses_the_parent_mode_fallback() {
    let base = r#"
version: "0.44.0"
info: { id: base }
citation:
  template:
    - title: primary
  subsequent:
    template:
      - variable: doi
      - variable: url
"#;
    let overlay = r#"
version: "0.44.0"
extends: base
info: { id: overlay }
citation:
  subsequent:
    template:
      remove:
        - match: { variable: url }
"#;
    let resolved = resolve_fallback_overlay(base, overlay)
        .expect("nested diff should use the inherited mode-specific fallback");
    let template = resolved
        .citation
        .as_ref()
        .and_then(|citation| citation.subsequent.as_deref())
        .and_then(|subsequent| subsequent.template.as_ref())
        .and_then(TemplateVariant::as_template)
        .expect("nested fallback should resolve to full");

    assert_eq!(template.len(), 1);
    assert!(matches!(
        template[0],
        TemplateComponent::Variable(ref variable) if variable.variable == SimpleVariable::Doi
    ));
}

#[test]
fn inherited_nested_citation_diff_does_not_bypass_an_unresolved_template_reference() {
    let base = r#"
version: "0.44.0"
info: { id: base }
citation:
  template:
    - title: primary
  subsequent:
    template-ref: https://example.com/citation-template
"#;
    let overlay = r#"
version: "0.44.0"
extends: base
info: { id: overlay }
citation:
  subsequent:
    template:
      remove:
        - match: { title: primary }
"#;
    let error = resolve_fallback_overlay(base, overlay)
        .expect_err("an unresolved explicit template reference must still own the diff base");

    assert!(matches!(
        error,
        ResolutionError::InvalidFallbackTemplateDiff { location, reason }
            if location == "overlay.citation.subsequent.template"
                && reason == "no inherited fallback template is available"
    ));
}

#[test]
fn nested_citation_null_clears_own_fallback_and_restores_outer_fallback() {
    let base = r#"
version: "0.44.0"
info: { id: base }
citation:
  template:
    - title: primary
  subsequent:
    template:
      - variable: doi
"#;
    let overlay = r#"
version: "0.44.0"
extends: base
info: { id: overlay }
citation:
  subsequent:
    template: ~
    type-variants:
      book:
        add:
          - after: { title: primary }
            component: { variable: publisher }
"#;
    let resolved = resolve_fallback_overlay(base, overlay)
        .expect("cleared nested fallback should restore the effective outer fallback");
    let subsequent = resolved
        .citation
        .as_ref()
        .and_then(|citation| citation.subsequent.as_deref())
        .expect("subsequent citation should remain");

    assert!(subsequent.template.is_none());
    let template = subsequent
        .type_variants
        .as_ref()
        .and_then(|variants| variants.get(&TypeSelector::Single("book".to_string())))
        .and_then(TemplateVariant::as_template)
        .expect("type diff should resolve against the restored outer fallback");
    assert_eq!(template.len(), 2);
    assert!(matches!(template[0], TemplateComponent::Title(_)));
    assert!(matches!(
        template[1],
        TemplateComponent::Variable(ref variable)
            if variable.variable == SimpleVariable::Publisher
    ));
}

#[rstest]
#[case::citation(FallbackSurface::Citation)]
#[case::bibliography(FallbackSurface::Bibliography)]
fn given_fallback_and_type_diffs_when_resolved_then_fallback_resolves_first(
    #[case] surface: FallbackSurface,
) {
    let base = fallback_style_yaml(
        "base",
        None,
        surface,
        "template:\n  - title: primary\n  - variable: doi",
    );
    let overlay = fallback_style_yaml(
        "overlay",
        Some("base"),
        surface,
        "template:\n  remove:\n    - match: { variable: doi }\ntype-variants:\n  book:\n    add:\n      - after: { title: primary }\n        component: { variable: publisher }",
    );
    let resolved = resolve_fallback_overlay(&base, &overlay)
        .expect("fallback should resolve before its type variant");
    let variants = match surface {
        FallbackSurface::Citation => resolved
            .citation
            .as_ref()
            .and_then(|spec| spec.type_variants.as_ref()),
        FallbackSurface::Bibliography => resolved
            .bibliography
            .as_ref()
            .and_then(|spec| spec.type_variants.as_ref()),
    }
    .expect("type variants should remain");
    let template = variants
        .get(&TypeSelector::Single("book".to_string()))
        .and_then(TemplateVariant::as_template)
        .expect("type diff should resolve to full");

    assert_eq!(template.len(), 2);
    assert!(matches!(template[0], TemplateComponent::Title(_)));
    assert!(matches!(
        template[1],
        TemplateComponent::Variable(ref variable)
            if variable.variable == SimpleVariable::Publisher
    ));
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
    uncertainty-marker: '?]'
    uncertainty-marker-prefix: '['
    approximation-marker: '['
    approximation-marker-suffix: ']'
    range-delimiter: "—"
bibliography:
  options:
    dates:
      month: numeric
      uncertainty-marker: '?]'
      uncertainty-marker-prefix: '['
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
        assert_eq!(dates.uncertainty_marker.as_deref(), Some("?]"));
        assert_eq!(
            dates.uncertainty_marker_prefix.as_deref(),
            Some("["),
            "inherited paired uncertainty prefix must survive a partial override"
        );
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
            Some("?]"),
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
        assert_eq!(
            dates.uncertainty_marker_prefix.as_deref(),
            Some("["),
            "paired uncertainty prefix must survive a null clearing only its own suffix field"
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
    candidates:
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
        substitute.candidates(),
        &[
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
