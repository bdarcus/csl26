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
    reason = "Panicking is acceptable and often desired in test code."
)]

//! BDD tests for the runtime scoped-options cascade's field-level merge.
//!
//! The global → citation/bibliography scope cascade merges nested option
//! blocks field-by-field using the chain-merged authored scope mappings
//! carried on the resolved style (`ScopedRawOptions`), falling back to the
//! typed whole-block merge when no trustworthy mapping is available.

use citum_schema_style::{
    Style,
    options::{Config, MonthFormat, TwoNameDelimiterPolicy},
    template::{WrapConfig, WrapPunctuation},
};
use rstest::rstest;
use std::collections::HashSet;

struct MockResolver(Style);
impl citum_resolver_api::StyleResolver for MockResolver {
    type Style = Style;
    type Locale = citum_schema_style::locale::Locale;

    fn resolve_style(&self, _uri: &str) -> Result<Style, citum_schema_style::ResolverError> {
        Ok(self.0.clone())
    }

    fn resolve_locale(&self, id: &str) -> Result<Self::Locale, citum_schema_style::ResolverError> {
        Err(citum_schema_style::ResolverError::LocaleNotFound(
            std::borrow::Cow::Owned(id.to_string()),
        ))
    }
}

fn resolve(style: Style, base: Option<Style>) -> Style {
    let mut visited = HashSet::new();
    match base {
        Some(base) => {
            let resolver = MockResolver(base);
            style
                .try_into_resolved_recursive_with(Some(&resolver), &mut visited)
                .expect("resolution should succeed")
        }
        None => style
            .try_into_resolved_recursive_with(None, &mut visited)
            .expect("resolution should succeed"),
    }
}

/// Effective bibliography-scope config of a resolved style.
fn bibliography_scope_config(style: &Style) -> Config {
    let base = style.options.clone().expect("style has global options");
    let bibliography_options = style
        .bibliography
        .as_ref()
        .and_then(|b| b.options.as_ref())
        .expect("style has bibliography options");
    bibliography_options.merged_with_raw(&base, style.scoped_raw_options.bibliography.as_ref())
}

/// Effective citation-scope config of a resolved style.
fn citation_scope_config(style: &Style) -> Config {
    let base = style.options.clone().expect("style has global options");
    let citation_options = style
        .citation
        .as_ref()
        .and_then(|c| c.options.as_ref())
        .expect("style has citation options");
    citation_options.merged_with_raw(&base, style.scoped_raw_options.citation.as_ref())
}

#[test]
fn given_scope_date_substitute_map_when_cascading_then_lists_merge_per_selector() {
    let style = resolve(
        Style::from_yaml_str(
            r#"
version: "0.44.0"
info:
  title: Scoped Date Substitute
  id: scoped-date-substitute
options:
  date-substitute: standard
bibliography:
  options:
    date-substitute:
      book: []
"#,
        )
        .expect("valid style"),
        None,
    );

    let policy = bibliography_scope_config(&style)
        .date_substitute
        .expect("merged date-substitute policy");

    assert!(policy.candidates_for("book").is_some_and(<[_]>::is_empty));
    assert_eq!(policy.candidates_for("report").map(<[_]>::len), Some(1));
    let selectors: Vec<String> = policy.entries().keys().map(ToString::to_string).collect();
    assert_eq!(selectors, ["default", "book"]);
}

const ROOT_STYLE_BIBLIOGRAPHY_SCOPE: &str = r#"
version: "0.44.0"
info:
  title: Scope Cascade Root
  id: scope-cascade-root
options:
  dates:
    month: numeric
    range-delimiter: '—'
bibliography:
  options:
    dates:
      note-wrap: parentheses
"#;

const ROOT_STYLE_CITATION_SCOPE: &str = r#"
version: "0.44.0"
info:
  title: Scope Cascade Root
  id: scope-cascade-root
options:
  dates:
    month: numeric
    range-delimiter: '—'
citation:
  options:
    dates:
      note-wrap: parentheses
"#;

const TWO_NAME_DELIMITER_SCOPE_CASCADE: &str = r#"
version: "0.44.0"
info:
  title: Two-name delimiter scope cascade
  id: two-name-delimiter-scope-cascade
options:
  contributors:
    delimiter-precedes-last: always
    two-name-delimiter-policy: suppress-in-citation-or-given-first
citation:
  options:
    contributors:
      two-name-delimiter-policy: follow-rule
bibliography:
  options:
    contributors:
      and: text
"#;

#[rstest]
#[case::citation_override(citation_scope_config, TwoNameDelimiterPolicy::FollowRule)]
#[case::bibliography_inherits(
    bibliography_scope_config,
    TwoNameDelimiterPolicy::SuppressInCitationOrGivenFirst
)]
fn given_scoped_contributor_policy_when_cascading_then_authored_scope_overrides_or_inherits(
    #[case] scope_config: fn(&Style) -> Config,
    #[case] expected: TwoNameDelimiterPolicy,
) {
    let style = resolve(
        Style::from_yaml_str(TWO_NAME_DELIMITER_SCOPE_CASCADE).expect("valid style"),
        None,
    );

    let contributors = scope_config(&style)
        .contributors
        .expect("merged contributor options");

    assert_eq!(contributors.two_name_delimiter_policy, Some(expected));
    assert_eq!(
        contributors.delimiter_precedes_last,
        Some(citum_schema_style::options::DelimiterPrecedesLast::Always)
    );
}

#[rstest]
#[case::bibliography_scope(ROOT_STYLE_BIBLIOGRAPHY_SCOPE, bibliography_scope_config)]
#[case::citation_scope(ROOT_STYLE_CITATION_SCOPE, citation_scope_config)]
fn given_partial_scope_dates_block_when_cascading_then_unwritten_fields_inherit_from_global(
    #[case] yaml: &str,
    #[case] scope_config: fn(&Style) -> Config,
) {
    // given: a root style whose scope-level dates block sets only note-wrap
    let style = resolve(Style::from_yaml_str(yaml).expect("valid style"), None);

    // when: computing the effective scope config
    let dates = scope_config(&style).dates.expect("merged dates present");

    // then: the authored scope field applies and every unwritten field
    // inherits the global value rather than reverting to serde defaults
    assert_eq!(
        dates.note_wrap,
        Some(WrapConfig::from(WrapPunctuation::Parentheses))
    );
    assert_eq!(dates.month, MonthFormat::Numeric);
    assert_eq!(dates.range_delimiter, "—");
}

#[rstest]
#[case::child_extends_global_dates(
    r#"
version: "0.44.0"
info:
  title: Scope Cascade Child
  id: scope-cascade-child
extends: scope-cascade-root
options:
  dates:
    uncertainty-marker: '!'
"#,
    Some("!")
)]
#[case::child_without_dates_override(
    r#"
version: "0.44.0"
info:
  title: Scope Cascade Child
  id: scope-cascade-child
extends: scope-cascade-root
"#,
    None
)]
fn given_wrapper_chain_when_child_changes_global_dates_then_change_reaches_bibliography_scope(
    #[case] child_yaml: &str,
    #[case] expected_uncertainty_marker: Option<&str>,
) {
    // given: a parent authoring a partial bibliography dates block, and a
    // child wrapper that only touches the global dates block
    let parent = Style::from_yaml_str(ROOT_STYLE_BIBLIOGRAPHY_SCOPE).expect("valid parent");
    let child = Style::from_yaml_str(child_yaml).expect("valid child");

    // when: resolving the chain and computing the bibliography-scope config
    let resolved = resolve(child, Some(parent));
    let dates = bibliography_scope_config(&resolved)
        .dates
        .expect("merged dates present");

    // then: the child's global-scope change propagates into the bibliography
    // scope (an eagerly materialized scope block would freeze the parent's
    // global values), while the parent's scope-level note-wrap still applies
    assert_eq!(
        dates.uncertainty_marker,
        expected_uncertainty_marker.map(str::to_string)
    );
    assert_eq!(dates.month, MonthFormat::Numeric);
    assert_eq!(
        dates.note_wrap,
        Some(WrapConfig::from(WrapPunctuation::Parentheses))
    );
}

#[rstest]
#[case::mutated_to_short(MonthFormat::Short)]
#[case::mutated_to_numeric(MonthFormat::Numeric)]
fn given_post_parse_mutation_when_cascading_then_typed_whole_block_merge_is_used(
    #[case] mutated_month: MonthFormat,
) {
    // given: a resolved style whose bibliography options were mutated after
    // parse (the scope block parses with the serde-default `Long`, so both
    // case values are real changes), so the capture no longer round-trips
    let mut style = resolve(
        Style::from_yaml_str(ROOT_STYLE_BIBLIOGRAPHY_SCOPE).expect("valid style"),
        None,
    );
    let options = style
        .bibliography
        .as_mut()
        .and_then(|b| b.options.as_mut())
        .expect("bibliography options present");
    options.dates.as_mut().expect("dates present").month = mutated_month.clone();

    // when: computing the effective bibliography-scope config
    let base = style.options.clone().expect("global options present");
    let bibliography_options = style
        .bibliography
        .as_ref()
        .and_then(|b| b.options.as_ref())
        .expect("bibliography options present");
    let raw_merged =
        bibliography_options.merged_with_raw(&base, style.scoped_raw_options.bibliography.as_ref());

    // then: the guard rejects the stale capture and the result is exactly the
    // typed whole-block merge
    let typed_merged = bibliography_options.merged_with(&base);
    assert_eq!(raw_merged, typed_merged);
    assert_eq!(
        raw_merged.dates.as_ref().expect("dates present").month,
        mutated_month
    );
}

#[rstest]
#[case::without_raw_mapping(None)]
#[case::non_mapping_raw(Some(serde_yaml::Value::Null))]
fn given_no_trustworthy_raw_mapping_when_cascading_then_typed_merge_applies(
    #[case] raw: Option<serde_yaml::Value>,
) {
    // given: options built programmatically, with no authored raw mapping
    let style = resolve(
        Style::from_yaml_str(ROOT_STYLE_BIBLIOGRAPHY_SCOPE).expect("valid style"),
        None,
    );
    let base = style.options.clone().expect("global options present");
    let bibliography_options = style
        .bibliography
        .as_ref()
        .and_then(|b| b.options.as_ref())
        .expect("bibliography options present");

    // when: merging without a raw scope mapping
    let merged = bibliography_options.merged_with_raw(&base, raw.as_ref());

    // then: the result is the typed whole-block merge (scope block wins whole)
    assert_eq!(merged, bibliography_options.merged_with(&base));
    assert_eq!(
        merged.dates.as_ref().expect("dates present").month,
        MonthFormat::Long
    );
}
