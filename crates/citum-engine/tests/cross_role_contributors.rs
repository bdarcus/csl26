/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "Panicking is appropriate in integration tests."
)]

use citum_engine::Processor;
use citum_schema::{
    Style,
    citation::{Citation, CitationItem},
    reference::InputReference,
};
use indexmap::IndexMap;

const STYLE_HEADER: &str = r#"
info:
  id: cross-role-test
  title: Cross-role test
options:
  contributors:
    delimiter: ", "
    and: symbol
    name-form: initials
    initialize-with: ". "
"#;

fn render_bibliography(template: &str, references: &str) -> String {
    let style = Style::from_yaml_str(&format!(
        "{STYLE_HEADER}\nbibliography:\n  template:\n{}",
        indent(template, 4)
    ))
    .expect("cross-role style should parse");
    let references: Vec<InputReference> =
        serde_yaml::from_str(references).expect("cross-role references should parse");
    let bibliography = references
        .into_iter()
        .map(|reference| {
            let id = reference
                .id()
                .expect("reference id should exist")
                .to_string();
            (id, reference)
        })
        .collect::<IndexMap<_, _>>();
    Processor::new(style, bibliography).render_bibliography()
}

fn processor(style: &str, references: &str) -> Processor {
    let style = Style::from_yaml_str(style).expect("cross-role style should parse");
    let references: Vec<InputReference> =
        serde_yaml::from_str(references).expect("cross-role references should parse");
    let bibliography = references
        .into_iter()
        .map(|reference| {
            let id = reference
                .id()
                .expect("reference id should exist")
                .to_string();
            (id, reference)
        })
        .collect::<IndexMap<_, _>>();
    Processor::new(style, bibliography)
}

fn indent(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .trim()
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn document_order_uses_one_conjunction_across_distinct_roles() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    labels: individual
    roles:
      writer:
        label: {term: writer, form: long, placement: suffix, text-case: capitalize-first, wrap: parentheses}
      director:
        label: {term: director, form: long, placement: suffix, text-case: capitalize-first, wrap: parentheses}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer]
      contributor: {family: Kogen, given: Jay}
    - roles: [writer]
      contributor: {family: Wolodarsky, given: Wallace}
    - roles: [director]
      contributor: {family: Kirkland, given: Mark}
"#,
    );

    assert_eq!(
        rendered,
        "Kogen, J. (Writer), Wolodarsky, W. (Writer), & Kirkland, M. (Director)"
    );
}

#[test]
fn explicit_multi_role_entry_uses_authored_combined_term() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    labels: individual
    roles:
      writer:
        label: {term: writer-director, form: long, placement: suffix, text-case: title, wrap: parentheses}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, director]
      contributor: {family: Whedon, given: Joss}
"#,
    );

    assert_eq!(rendered, "Whedon, J. (Writer & Director)");
}

#[test]
fn partial_cross_role_identity_combines_only_the_explicit_shared_entry() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    labels: individual
    roles:
      writer:
        label: {term: writer, form: long, placement: suffix, text-case: title, prefix: " (", suffix: ")"}
      director:
        label: {term: director, form: long, placement: suffix, text-case: title, prefix: " (", suffix: ")"}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, director]
      contributor: {family: Whedon, given: Joss}
    - roles: [writer]
      contributor: {family: Minear, given: Tim}
    - roles: [director]
      contributor: {family: Greenwalt, given: David}
"#,
    );

    assert_eq!(
        rendered,
        "Whedon, J. (Writer & Director), Minear, T. (Writer), & Greenwalt, D. (Director)"
    );
}

#[test]
fn equal_separate_native_entries_do_not_imply_shared_identity() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    labels: none
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer]
      contributor: {family: Doe, given: Jane}
    - roles: [director]
      contributor: {family: Doe, given: Jane}
"#,
    );

    assert_eq!(rendered, "Doe, J. & Doe, J.");
}

#[test]
fn disabled_combination_expands_an_explicit_multi_role_entry() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    labels: none
    combine-same-person: false
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, director]
      contributor: {family: Doe, given: Jane}
"#,
    );

    assert_eq!(rendered, "Doe, J. & Doe, J.");
}

#[test]
fn suppression_uses_non_empty_exact_identity_sets_in_both_rendering_contexts() {
    let style = r#"
info: {id: suppression-test, title: Suppression test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
    suppress:
      - role: translator
        when-identical-to: editor
citation:
  template:
    - contributor: editor
      form: long
      name-order: family-first
    - contributor: translator
      form: long
      name-order: family-first
      prefix: "; "
bibliography:
  template:
    - contributor: editor
      form: long
      name-order: family-first
    - contributor: translator
      form: long
      name-order: family-first
      prefix: "; "
"#;
    let references = r#"
- id: exact
  class: monograph
  type: book
  contributors:
    - roles: [editor, translator]
      contributor: {family: Doe, given: Jane}
"#;
    let processor = processor(style, references);
    let citation = Citation {
        items: vec![CitationItem {
            id: "exact".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    assert_eq!(processor.render_bibliography(), "Doe, J.");
    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("suppressed citation should render"),
        "Doe, J."
    );
}

#[test]
fn suppression_collapses_a_dependent_descriptor_group() {
    let style = r#"
info: {id: suppression-group-test, title: Suppression group test}
options:
  contributors:
    suppress:
      - role: performer
        when-identical-to: composer
bibliography:
  template:
    - contributor: composer
      form: long
    - group:
        - contributor: performer
          form: long
      prefix: " [Recorded by "
      suffix: "]"
"#;
    let references = r#"
- id: song
  class: audio-visual
  type: recording
  contributors:
    - roles: [composer, performer]
      contributor: {family: Doe, given: Jane}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Jane Doe"
    );
}

#[test]
fn suppression_does_not_fire_for_absent_or_partially_overlapping_roles() {
    let style = r#"
info: {id: suppression-boundary-test, title: Suppression boundary test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
    and: symbol
    suppress:
      - role: translator
        when-identical-to: editor
bibliography:
  template:
    - contributor: [editor, translator]
      form: long
      name-order: family-first
      merge: {labels: none}
"#;
    let references = r#"
- id: absent-comparison
  class: monograph
  type: book
  contributors:
    - roles: [translator]
      contributor: {family: Able, given: Amy}
- id: partial-overlap
  class: monograph
  type: book
  contributors:
    - roles: [editor, translator]
      contributor: {family: Baker, given: Bea}
    - roles: [editor]
      contributor: {family: Clark, given: Cora}
- id: equal-looking-separate
  class: monograph
  type: book
  contributors:
    - roles: [editor]
      contributor: {family: Doe, given: Jane}
    - roles: [translator]
      contributor: {family: Doe, given: Jane}
"#;
    let rendered = processor(style, references).render_bibliography();

    assert_eq!(
        rendered,
        "Able, A.\n\nBaker, B. & Clark, C.\n\nDoe, J. & Doe, J."
    );
}

#[test]
fn missing_combined_term_uses_component_role_conjunction() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, producer]
  form: long
  name-order: family-first
  merge:
    labels: individual
    role-conjunction: " / "
    roles:
      writer:
        label: {term: writer-producer, form: long, placement: suffix, wrap: parentheses}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, producer]
      contributor: {family: Doe, given: Jane}
"#,
    );

    assert_eq!(rendered, "Doe, J. (writer / producer)");
}

#[test]
fn role_order_collective_labels_pluralize_only_matching_role_runs() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    order: role
    labels: collective
    roles:
      writer:
        label: {term: writer, form: long, placement: suffix, text-case: capitalize-first, prefix: " (", suffix: ")"}
      director:
        label: {term: director, form: long, placement: suffix, text-case: capitalize-first, prefix: " (", suffix: ")"}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer]
      contributor: {family: Able, given: Amy}
    - roles: [director]
      contributor: {family: Baker, given: Bea}
    - roles: [writer]
      contributor: {family: Clark, given: Cora}
"#,
    );

    assert_eq!(
        rendered,
        "Able, A., Clark, C. (Writers), & Baker, B. (Director)"
    );
}

#[test]
fn role_order_collective_verb_labels_prefix_each_role_run() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: verb
  merge:
    order: role
    labels: collective
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [director]
      contributor: {family: Baker, given: Bea}
    - roles: [writer]
      contributor: {family: Able, given: Amy}
"#,
    );

    assert_eq!(rendered, "Written by A. Able & directed by B. Baker");
}

#[test]
fn document_order_collective_labels_preserve_interleaved_runs() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  merge:
    labels: collective
    roles:
      writer:
        label: {term: writer, form: long, placement: suffix, text-case: capitalize-first, prefix: " (", suffix: ")"}
      director:
        label: {term: director, form: long, placement: suffix, text-case: capitalize-first, prefix: " (", suffix: ")"}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer]
      contributor: {family: Able, given: Amy}
    - roles: [director]
      contributor: {family: Baker, given: Bea}
    - roles: [writer]
      contributor: {family: Clark, given: Cora}
"#,
    );

    assert_eq!(
        rendered,
        "Able, A. (Writer), Baker, B. (Director), & Clark, C. (Writer)"
    );
}

#[test]
fn et_al_threshold_is_calculated_after_explicit_role_combination() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  shorten: {min: 4, use-first: 1}
  merge: {labels: none}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, director]
      contributor: {family: Able, given: Amy}
    - roles: [writer]
      contributor: {family: Baker, given: Bea}
    - roles: [director]
      contributor: {family: Clark, given: Cora}
"#,
    );

    assert_eq!(rendered, "Able, A., Baker, B., & Clark, C.");
}

#[test]
fn collective_label_decorates_the_selected_run_before_et_al_joining() {
    let rendered = render_bibliography(
        r#"
- contributor: [writer, director]
  form: long
  name-order: family-first
  shorten: {min: 3, use-first: 1}
  merge:
    labels: collective
    roles:
      writer:
        label: {term: writer, form: long, placement: suffix, text-case: title, prefix: " (", suffix: ")"}
"#,
        r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer]
      contributor: {family: Able, given: Amy}
    - roles: [writer]
      contributor: {family: Baker, given: Bea}
    - roles: [writer]
      contributor: {family: Clark, given: Cora}
"#,
    );

    assert_eq!(rendered, "Able, A. (Writers) et al.");
}

#[test]
fn editor_translator_uses_the_canonical_combined_locale_term() {
    let rendered = render_bibliography(
        r#"
- contributor: [editor, translator]
  form: long
  name-order: family-first
  merge:
    labels: collective
    roles:
      editor:
        label: {term: editor-translator, form: short, placement: suffix, prefix: ", "}
"#,
        r#"
- id: book
  class: monograph
  type: book
  contributors:
    - roles: [editor, translator]
      contributor: {family: Doe, given: Jane}
"#,
    );

    assert_eq!(rendered, "Doe, J., ed. & trans.");
}

#[test]
fn bibliography_author_sort_uses_nested_list_primary_from_type_variant() {
    let style = r#"
info: {id: merged-sort-test, title: Merged sort test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
bibliography:
  sort:
    template:
      - key: author
  template:
    - title: primary
  type-variants:
    broadcast:
      - group:
          - contributor: [writer, director]
            form: long
            name-order: family-first
            merge: {labels: none}
"#;
    let references = r#"
- id: first-input
  class: audio-visual
  type: broadcast
  title: Alpha title
  contributors:
    - roles: [writer]
      contributor: {family: Zulu, given: Zoe}
- id: second-input
  class: audio-visual
  type: broadcast
  title: Zulu title
  contributors:
    - roles: [director]
      contributor: {family: Able, given: Amy}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Able, A.\n\nZulu, Z."
    );
}

#[test]
fn name_year_disambiguation_uses_the_list_primary_instead_of_semantic_author() {
    let style = r#"
info: {id: merged-disambiguation-test, title: Merged disambiguation test}
options:
  processing:
    disambiguate:
      names: false
      add-givenname: false
      year-suffix: true
  contributors:
    name-form: initials
    initialize-with: ". "
citation:
  multi-cite-delimiter: "; "
  template:
    - contributor: [writer, director]
      form: short
      merge: {labels: none}
    - date: issued
      form: year
      prefix: " "
"#;
    let references = r#"
- id: first
  class: audio-visual
  type: broadcast
  issued: "2020"
  contributors:
    - roles: [author]
      contributor: {family: Alpha, given: Alice}
    - roles: [writer, director]
      contributor: {family: Doe, given: Jane}
- id: second
  class: audio-visual
  type: broadcast
  issued: "2020"
  contributors:
    - roles: [author]
      contributor: {family: Zulu, given: Zoe}
    - roles: [writer, director]
      contributor: {family: Doe, given: Jane}
"#;
    let processor = processor(style, references);
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "first".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "second".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("merged-primary citation should render"),
        "Doe 2020a; Doe 2020b"
    );
}

fn apa_processor(references: &str) -> Processor {
    processor(
        include_str!("../../citum-schema-style/embedded/styles/apa-7th.yaml"),
        references,
    )
}

#[test]
fn apa_episode_promotes_writer_and_director_from_native_roles() {
    let references =
        include_str!("../../../tests/fixtures/audiovisual/apa-episode-cross-role.yaml");
    let processor = apa_processor(references);

    assert_eq!(
        processor.render_bibliography(),
        "Barris, K. (Writer & Director). (2017, January 11). Lemons [TV series episode]."
    );
    assert_eq!(
        processor
            .process_citation(&Citation {
                items: vec![CitationItem {
                    id: "lemons".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .expect("APA episode citation should render"),
        "(Barris, 2017)"
    );
}

#[test]
fn apa_film_promotes_directors_with_a_collective_plural_label() {
    let references = include_str!("../../../tests/fixtures/audiovisual/apa-film-directors.yaml");

    assert_eq!(
        apa_processor(references).render_bibliography(),
        "Docter, P., & Del Carmen, R. (Directors). (2015). _Inside out_ [Film]. Walt Disney Pictures; Pixar Animation Studios."
    );
}

#[test]
fn native_override_precedes_legacy_alias_and_uses_available_merged_roles() {
    let style = r#"
info: {id: primary-override-test, title: Primary override test}
options:
  substitute:
    candidates: [editor, title, translator]
    overrides:
      episode:
        - contributor: [writer, director]
      broadcast:
        - contributor: producer
  contributors:
    name-form: initials
    initialize-with: ". "
    merge: {labels: none}
bibliography:
  template:
    - contributor: author
      form: long
      name-order: family-first
"#;
    let references = r#"
- id: partial
  class: audio-visual
  type: episode
  title: Partial roles
  contributors:
    - role: author
      contributor: {family: Semantic, given: Alice}
    - role: writer
      contributor: {family: Writer, given: Wendy}
    - role: producer
      contributor: {family: Producer, given: Paul}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Writer, W."
    );
}

#[test]
fn effective_primary_override_drives_sorting_and_disambiguation() {
    let style = r#"
info: {id: primary-semantic-test, title: Primary semantic test}
options:
  processing:
    disambiguate: {names: false, add-givenname: false, year-suffix: true}
  substitute:
    overrides:
      episode:
        - contributor: [writer, director]
  contributors:
    name-form: initials
    initialize-with: ". "
    merge: {labels: none}
citation:
  multi-cite-delimiter: "; "
  template:
    - contributor: author
      form: short
    - date: issued
      form: year
      prefix: " "
bibliography:
  sort: author-date-title
  template:
    - contributor: author
      form: long
      name-order: family-first
"#;
    let references = r#"
- id: zulu-input
  class: audio-visual
  type: episode
  issued: "2020"
  contributors:
    - role: author
      contributor: {family: Alpha, given: Alice}
    - roles: [writer, director]
      contributor: {family: Doe, given: Jane}
- id: able-input
  class: audio-visual
  type: episode
  issued: "2020"
  contributors:
    - role: author
      contributor: {family: Zulu, given: Zoe}
    - roles: [writer, director]
      contributor: {family: Doe, given: Jane}
- id: sort-zulu
  class: audio-visual
  type: episode
  contributors:
    - role: author
      contributor: {family: Able, given: Amy}
    - role: writer
      contributor: {family: Zulu, given: Zoe}
- id: sort-able
  class: audio-visual
  type: episode
  contributors:
    - role: author
      contributor: {family: Zulu, given: Zoe}
    - role: writer
      contributor: {family: Able, given: Amy}
"#;
    let processor = processor(style, references);
    let citation = Citation {
        items: vec![
            CitationItem {
                id: "zulu-input".to_string(),
                ..Default::default()
            },
            CitationItem {
                id: "able-input".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    assert_eq!(
        processor.render_bibliography(),
        "Able, A.\n\nDoe, J.\n\nDoe, J.\n\nZulu, Z."
    );
    assert_eq!(
        processor
            .process_citation(&citation)
            .expect("effective-primary citation should render"),
        "Doe 2020a; Doe 2020b"
    );
}

// --- Review-fix regression tests ---
//
// NOTE: `role_substitute_suppression_*` below exercise
// `is_role_suppressed_by_substitute` (ROLE_SUBSTITUTE_FALLBACK.md): a
// fallback role is suppressed elsewhere in the template exactly when its
// configured *primary* role has its own data (see
// `crates/citum-engine/src/values/contributor/substitute.rs`). This is the
// opposite polarity of "primary role empty, filled by the fallback" — that
// case is already handled by the unrelated rendered-variable tracker
// (`TemplateComponentTracker`), not by this suppression check.

#[test]
fn role_substitute_suppression_excludes_role_from_merged_list_when_primary_role_present() {
    let style = r#"
info: {id: role-substitute-merge-present-test, title: Role substitute merge present test}
options:
  substitute:
    role-substitute:
      container-author: [editor]
  contributors:
    name-form: initials
    initialize-with: ". "
    and: symbol
bibliography:
  template:
    - contributor: [editor, translator]
      form: long
      name-order: family-first
      merge: {labels: none}
"#;
    let references = r#"
- id: primary-present
  class: monograph
  type: book
  contributors:
    - roles: [container-author]
      contributor: {family: Container, given: Cara}
    - roles: [editor]
      contributor: {family: Edit, given: Ed}
    - roles: [translator]
      contributor: {family: Trans, given: Tara}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Trans, T."
    );
}

#[test]
fn role_substitute_suppression_retains_role_in_merged_list_when_primary_role_absent() {
    let style = r#"
info: {id: role-substitute-merge-absent-test, title: Role substitute merge absent test}
options:
  substitute:
    role-substitute:
      container-author: [editor]
  contributors:
    name-form: initials
    initialize-with: ". "
    and: symbol
bibliography:
  template:
    - contributor: [editor, translator]
      form: long
      name-order: family-first
      merge: {labels: none}
"#;
    let references = r#"
- id: primary-absent
  class: monograph
  type: book
  contributors:
    - roles: [editor]
      contributor: {family: Edit, given: Ed}
    - roles: [translator]
      contributor: {family: Trans, given: Tara}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Edit, E. & Trans, T."
    );
}

#[test]
fn subsequent_author_substitute_uses_type_override_for_contributor_matching() {
    // `writer` (not `director`) is deliberately chosen: `Reference::author()`
    // already has a built-in Film/Episode compatibility fallback to
    // `director` (`citum-schema-data/src/reference/accessors.rs`), which
    // would make both references resolve a shared primary contributor
    // through that pre-existing path alone and mask whether the matcher
    // actually consults `options.substitute.overrides`. `writer` carries no
    // such built-in fallback, so a match here can only come from the
    // type override.
    let style = r#"
info: {id: matcher-override-test, title: Matcher override test}
options:
  substitute:
    overrides:
      film:
        - contributor: writer
  contributors:
    name-form: initials
    initialize-with: ". "
bibliography:
  options:
    subsequent-author-substitute: "———"
  sort:
    template:
      - key: title
  template:
    - contributor: author
      form: long
      name-order: family-first
    - title: primary
      prefix: ", "
"#;
    let references = r#"
- id: alpha-film
  class: audio-visual
  type: film
  title: Alpha Film
  contributors:
    - roles: [writer]
      contributor: {family: Doe, given: Jane}
- id: beta-film
  class: audio-visual
  type: film
  title: Beta Film
  contributors:
    - roles: [writer]
      contributor: {family: Doe, given: Jane}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Doe, J., Alpha Film\n\n———, Beta Film"
    );
}

#[test]
fn subsequent_author_substitute_never_matches_via_title_fallback() {
    let style = r#"
info: {id: matcher-title-test, title: Matcher title test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
bibliography:
  options:
    subsequent-author-substitute: "———"
  sort:
    template:
      - key: issued
  template:
    - contributor: author
      form: long
      name-order: family-first
    - date: issued
      form: year
      prefix: " ("
      suffix: ")"
"#;
    let references = r#"
- id: first
  class: monograph
  type: book
  title: Same Title
  issued: "2020"
- id: second
  class: monograph
  type: book
  title: Same Title
  issued: "2021"
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Same Title (2020)\n\nSame Title (2021)"
    );
}

#[test]
fn role_omit_suppresses_a_configured_structural_label_preset() {
    let style = r#"
info: {id: role-omit-scalar-test, title: Role omit scalar test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
    role:
      defaults: apa
      omit: [director]
bibliography:
  template:
    - contributor: director
      form: long
      name-order: family-first
"#;
    let references = r#"
- id: kubrick-film
  class: audio-visual
  type: film
  contributors:
    - roles: [director]
      contributor: {family: Kubrick, given: Stanley}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Kubrick, S."
    );
}

#[test]
fn configured_structural_label_preset_renders_when_role_is_not_omitted() {
    let style = r#"
info: {id: role-show-scalar-test, title: Role show scalar test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
    role:
      defaults: apa
bibliography:
  template:
    - contributor: director
      form: long
      name-order: family-first
"#;
    let references = r#"
- id: kubrick-film
  class: audio-visual
  type: film
  contributors:
    - roles: [director]
      contributor: {family: Kubrick, given: Stanley}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Kubrick, S. (Director)"
    );
}

#[test]
fn role_omit_suppresses_a_configured_structural_label_for_a_combined_merged_entry() {
    let style = r#"
info: {id: role-omit-merged-test, title: Role omit merged test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
    role:
      defaults: apa
      omit: [writer]
bibliography:
  template:
    - contributor: [writer, director]
      form: long
      name-order: family-first
      merge: {labels: individual}
"#;
    let references = r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, director]
      contributor: {family: Whedon, given: Joss}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Whedon, J."
    );
}

#[test]
fn configured_structural_label_renders_for_a_combined_merged_entry_when_role_is_not_omitted() {
    let style = r#"
info: {id: role-show-merged-test, title: Role show merged test}
options:
  contributors:
    name-form: initials
    initialize-with: ". "
    role:
      defaults: apa
bibliography:
  template:
    - contributor: [writer, director]
      form: long
      name-order: family-first
      merge: {labels: individual}
"#;
    let references = r#"
- id: episode
  class: audio-visual
  type: broadcast
  contributors:
    - roles: [writer, director]
      contributor: {family: Whedon, given: Joss}
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Whedon, J. (Writer & Director)"
    );
}

#[test]
fn empty_editor_list_falls_through_to_title_substitute() {
    let style = r#"
info: {id: empty-editor-substitute-test, title: Empty editor substitute test}
options:
  substitute:
    candidates: [editor, title]
  contributors:
    name-form: initials
    initialize-with: ". "
bibliography:
  template:
    - contributor: author
      form: long
      name-order: family-first
"#;
    let references = r#"
- id: empty-editor
  class: monograph
  type: book
  title: Substituted Title
  editor: []
"#;

    assert_eq!(
        processor(style, references).render_bibliography(),
        "Substituted Title"
    );
}

#[test]
fn merged_sort_key_falls_through_to_effective_primary_when_merged_component_is_empty() {
    let style = r#"
info: {id: merged-sort-fallback-test, title: Merged sort fallback test}
options:
  substitute:
    candidates: [editor, title]
  contributors:
    name-form: initials
    initialize-with: ". "
bibliography:
  sort:
    template:
      - key: author
  template:
    - contributor: [writer, director]
      form: long
      merge: {labels: none}
    - title: primary
"#;
    let references = r#"
- id: zulu-input
  class: monograph
  type: book
  title: Zulu Book
  contributors:
    - roles: [editor]
      contributor: {family: Able, given: Amy}
- id: able-input
  class: monograph
  type: book
  title: Able Book
  contributors:
    - roles: [editor]
      contributor: {family: Zulu, given: Zoe}
"#;

    // zulu-input's editor ("Able, Amy") sorts before able-input's editor
    // ("Zulu, Zoe"), even though the titles alone would sort the opposite
    // way — proving the sort key fell through to the effective-primary
    // resolver instead of stopping at the empty merged list.
    assert_eq!(
        processor(style, references).render_bibliography(),
        "Zulu Book\n\nAble Book"
    );
}

#[test]
fn migrated_names_merge_labels_every_declared_role_not_just_the_first() {
    // Regression for the CSL `<names variable="editor translator"><label
    // form="short" prefix=", "/></names>` migration shape: the converter
    // must label EVERY declared role (editor and translator), not only
    // `roles.first()`. Different people in each role must each keep their
    // own role label; a shared person combining both roles still resolves
    // the locale's authored `editor-translator` combined term.
    let rendered = render_bibliography(
        r#"
- contributor: [editor, translator]
  form: long
  name-order: family-first
  merge:
    order: role
    labels: collective
    roles:
      editor:
        label: {term: editor, form: short, placement: suffix, prefix: ", "}
      translator:
        label: {term: translator, form: short, placement: suffix, prefix: ", "}
"#,
        r#"
- id: different-people
  class: monograph
  type: book
  contributors:
    - roles: [editor]
      contributor: {family: Editor, given: Edna}
    - roles: [translator]
      contributor: {family: Translator, given: Tara}
"#,
    );
    assert_eq!(rendered, "Editor, E., ed. & Translator, T., trans.");

    let rendered_same_person = render_bibliography(
        r#"
- contributor: [editor, translator]
  form: long
  name-order: family-first
  merge:
    order: role
    labels: collective
    roles:
      editor:
        label: {term: editor, form: short, placement: suffix, prefix: ", "}
      translator:
        label: {term: translator, form: short, placement: suffix, prefix: ", "}
"#,
        r#"
- id: same-person
  class: monograph
  type: book
  contributors:
    - roles: [editor, translator]
      contributor: {family: Both, given: Blake}
"#,
    );
    assert_eq!(rendered_same_person, "Both, B., ed. & trans.");
}
