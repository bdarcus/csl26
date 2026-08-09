/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Forward-compatibility scenario corpus.
//!
//! Each scenario constructs a "future" style, reference, or locale that uses a
//! feature beyond what the current engine knows, then runs it through the same
//! load path real users hit. The actual outcome is captured as `Pass`,
//! `SoftDegrade`, or `HardFail` and compared against a checked-in snapshot.
//!
//! See `docs/specs/FORWARD_COMPATIBILITY.md` for the contract these cases
//! anchor and `crates/citum-engine/tests/snapshots/forward_compat_gaps.snap`
//! for the truth-of-record outcomes.
//!
//! The test passes when the observed outcomes match the snapshot exactly. It
//! fails only on drift in either direction — a gap that silently closed (good
//! — update the snapshot) or a new gap that appeared (review and decide). The
//! snapshot rows whose `declared != observed` are the punch list for the
//! follow-up implementation beans listed in the spec.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking is acceptable and often desired in tests."
)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use citum_schema::Style;
use citum_schema::reference::ReferenceClass;
use citum_schema_data::InputBibliography;

/// What a loader did when it met a "future" feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Parse succeeded and the feature was silently accepted (no warning channel today).
    Pass,
    /// Parse succeeded with a documented warning emitted through the
    /// compatibility channel.
    SoftDegrade,
    /// Parse returned an error and no artifact was produced.
    HardFail,
}

impl Outcome {
    fn as_str(self) -> &'static str {
        match self {
            Outcome::Pass => "Pass",
            Outcome::SoftDegrade => "SoftDegrade",
            Outcome::HardFail => "HardFail",
        }
    }
}

/// One row of the scope table in `docs/specs/FORWARD_COMPATIBILITY.md`.
struct Scenario {
    id: &'static str,
    category: &'static str,
    declared: Outcome,
    observed: fn() -> Outcome,
    follow_up: &'static str,
}

use citum_schema::locale::{GeneralTerm, TermForm};
use citum_schema::reference::{
    ClassExtension, CollectionType, ContributorRole, MonographComponentType, MonographType,
    SerialComponentType,
};
use citum_schema::template::{DateForm, TemplateComponent};

fn parse_style(yaml: &str) -> Outcome {
    match Style::from_yaml_str(yaml) {
        Ok(style) => {
            if style_has_unknowns(&style) {
                Outcome::SoftDegrade
            } else {
                Outcome::Pass
            }
        }
        Err(_) => Outcome::HardFail,
    }
}

fn style_has_unknowns(style: &Style) -> bool {
    // Capture-fields (rows 05/06: new option key, new top-level section)
    // delegate to the published engine walker so the test and `citum check`
    // share a single source of truth.
    if !citum_engine::api::collect_unknown_field_paths(style).is_empty() {
        return true;
    }
    // Templates remain opt-out (rows 11/12 HardFail); local enum walker
    // catches Unknown variants captured by the tolerant deserializer.
    if let Some(citation) = &style.citation
        && let Some(template) = &citation.template
        && template.as_template().is_some_and(template_has_unknowns)
    {
        return true;
    }
    if let Some(bib) = &style.bibliography
        && let Some(template) = &bib.template
        && template.as_template().is_some_and(template_has_unknowns)
    {
        return true;
    }
    if let Some(templates) = &style.templates {
        for template in templates.values() {
            if template_has_unknowns(template) {
                return true;
            }
        }
    }
    false
}

fn template_has_unknowns(components: &[TemplateComponent]) -> bool {
    for component in components {
        match component {
            TemplateComponent::Term(t) => {
                if matches!(t.term, GeneralTerm::Unknown(_)) {
                    return true;
                }
                if let Some(TermForm::Unknown(_)) = &t.form {
                    return true;
                }
            }
            TemplateComponent::Contributor(c) => {
                if matches!(
                    c.contributor.as_single(),
                    Some(citum_schema::template::ContributorRole::Unknown(_))
                ) {
                    return true;
                }
            }
            TemplateComponent::Date(d) => {
                if matches!(d.form, DateForm::Unknown(_)) {
                    return true;
                }
            }
            TemplateComponent::Group(g) if template_has_unknowns(&g.group) => {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn parse_bibliography(yaml: &str) -> Outcome {
    match serde_yaml::from_str::<InputBibliography>(yaml) {
        Ok(bibliography) => {
            let has_unknown = bibliography.references.iter().any(|reference| {
                if matches!(reference.class(), ReferenceClass::Unknown(_)) {
                    return true;
                }
                let ext_unknown = match reference.extension() {
                    ClassExtension::Monograph(r) => {
                        matches!(r.r#type, MonographType::Unknown(_))
                            || !r.unknown_fields.is_empty()
                    }
                    ClassExtension::Collection(r) => {
                        matches!(r.r#type, CollectionType::Unknown(_))
                            || !r.unknown_fields.is_empty()
                    }
                    ClassExtension::CollectionComponent(r) => {
                        matches!(r.r#type, MonographComponentType::Unknown(_))
                            || !r.unknown_fields.is_empty()
                    }
                    ClassExtension::SerialComponent(r) => {
                        matches!(r.r#type, SerialComponentType::Unknown(_))
                            || !r.unknown_fields.is_empty()
                    }
                    _ => false,
                };

                ext_unknown
                    || reference
                        .all_contributor_entries()
                        .iter()
                        .any(|contributor| {
                            contributor
                                .roles
                                .as_slice()
                                .iter()
                                .any(|role| matches!(role, ContributorRole::Unknown(_)))
                        })
            });

            if has_unknown {
                Outcome::SoftDegrade
            } else {
                Outcome::Pass
            }
        }
        Err(_) => Outcome::HardFail,
    }
}

// ------------ Case fixtures ------------
//
// Every fixture is a self-contained YAML literal. The only "future" feature
// in each fixture is the one named by the case; everything else is valid
// against the current schema.

const STYLE_HEAD: &str =
    "version: \"0.51\"\ninfo:\n  id: forward-compat-fixture\n  title: Forward compat fixture\n";

fn case_attribute_enum_in_template() -> Outcome {
    // ContributorRole gains a hypothetical future role.
    let yaml = format!(
        "{STYLE_HEAD}bibliography:\n  template:\n    - contributor: future-contributor\n      form: long\n"
    );
    parse_style(&yaml)
}

fn case_attribute_enum_in_data() -> Outcome {
    // MonographType gains hypothetical `dance-performance`.
    let yaml = "references:\n  - id: perf2026\n    class: monograph\n    type: dance-performance\n    title: A Performance\n";
    parse_bibliography(yaml)
}

fn case_discriminator_class() -> Outcome {
    // InputReference gains a brand-new `class`.
    let yaml =
        "references:\n  - id: perf2026\n    class: dance-performance\n    title: A Performance\n";
    parse_bibliography(yaml)
}

fn case_locale_form() -> Outcome {
    // TermForm gains hypothetical `vocative`. Drives the typed enum via
    // TemplateTerm.form: Option<TermForm> on the style side — the raw
    // locale YAML uses string-keyed maps and does not exercise TermForm
    // deserialization.
    let yaml =
        format!("{STYLE_HEAD}bibliography:\n  template:\n    - term: page\n      form: vocative\n");
    parse_style(&yaml)
}

fn case_date_form() -> Outcome {
    // DateForm gains hypothetical `month-and-day`.
    let yaml = format!(
        "{STYLE_HEAD}bibliography:\n  template:\n    - date: issued\n      form: month-and-day\n"
    );
    parse_style(&yaml)
}

fn case_new_style_option_key() -> Outcome {
    // A new key inside a `deny_unknown_fields` option struct.
    let yaml = format!(
        "{STYLE_HEAD}options:\n  contributors:\n    future-key: true\nbibliography:\n  template:\n    - contributor: author\n      form: long\n"
    );
    parse_style(&yaml)
}

fn case_new_top_level_section() -> Outcome {
    // A brand-new top-level block on Style.
    let yaml = format!(
        "{STYLE_HEAD}experiments:\n  inline-author-disambiguation: true\nbibliography:\n  template:\n    - contributor: author\n      form: long\n"
    );
    parse_style(&yaml)
}

fn case_new_reference_field() -> Outcome {
    // A field that does not exist on Monograph. Everything else is a valid
    // Monograph so the only "future" element is `audience: scholarly`.
    let yaml = "references:\n  - id: smith2026\n    class: monograph\n    type: book\n    audience: scholarly\n";
    parse_bibliography(yaml)
}

fn case_new_locale_term_key() -> Outcome {
    // A style references a GeneralTerm key that the engine vocabulary
    // does not enumerate. Drives the typed enum via TemplateTerm.term:
    // GeneralTerm — the style-side lookup path the spec actually targets,
    // not the raw locale map.
    let yaml = format!(
        "{STYLE_HEAD}bibliography:\n  template:\n    - term: preprint-server\n      form: long\n"
    );
    parse_style(&yaml)
}

fn case_custom_namespace() -> Outcome {
    // Namespaced custom metadata — must Pass per the spec.
    let yaml = format!(
        "{STYLE_HEAD}bibliography:\n  template:\n    - contributor: author\n      form: long\n      custom:\n        publisher-x.house-format: true\n"
    );
    parse_style(&yaml)
}

fn case_version_only_signal() -> Outcome {
    // version: "99.0" on an otherwise-valid style. citum check is the warning
    // channel for this case; parsing alone succeeds (Pass). The SoftDegrade
    // is delivered later by `citum check`, not by the loader.
    let yaml = "version: \"99.0\"\ninfo:\n  id: forward-compat-fixture\n  title: Forward compat fixture\nbibliography:\n  template:\n    - contributor: author\n      form: long\n";
    parse_style(yaml)
}

fn case_template_grammar() -> Outcome {
    // Hypothetical new TemplateComponent variant (must HardFail; opt-out).
    let yaml = format!(
        "{STYLE_HEAD}bibliography:\n  template:\n    - loop: authors\n      body:\n        - contributor: author\n          form: long\n"
    );
    parse_style(&yaml)
}

fn case_template_required_field() -> Outcome {
    // We can't actually introduce a hypothetical required field from a test;
    // we exercise the dual — a known-good template with a typoed required
    // shape — to confirm the loader still rejects malformed templates
    // (the contract for this row is HardFail).
    let yaml = format!(
        "{STYLE_HEAD}bibliography:\n  template:\n    - variable:\n        not-a-real-shape: true\n"
    );
    parse_style(&yaml)
}

// ------------ Snapshot harness ------------

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "01-attr-enum-template",
            category: "Attribute enum in template",
            declared: Outcome::SoftDegrade,
            observed: case_attribute_enum_in_template,
            follow_up: "csl26-ld6e tolerant-enum-deserializer",
        },
        Scenario {
            id: "02-attr-enum-data",
            category: "Attribute enum in reference data",
            declared: Outcome::SoftDegrade,
            observed: case_attribute_enum_in_data,
            follow_up: "csl26-ld6e tolerant-enum-deserializer",
        },
        Scenario {
            id: "02b-discriminator-class",
            category: "Unknown InputReference class",
            declared: Outcome::SoftDegrade,
            observed: case_discriminator_class,
            follow_up: "csl26-odgh inputreference-discriminator-design",
        },
        Scenario {
            id: "03-locale-form",
            category: "Locale TermForm value",
            declared: Outcome::SoftDegrade,
            observed: case_locale_form,
            follow_up: "csl26-ld6e tolerant-enum-deserializer",
        },
        Scenario {
            id: "04-date-form",
            category: "DateForm value in template",
            declared: Outcome::SoftDegrade,
            observed: case_date_form,
            follow_up: "csl26-ld6e tolerant-enum-deserializer",
        },
        Scenario {
            id: "05-new-option-key",
            category: "New key inside style option struct",
            declared: Outcome::SoftDegrade,
            observed: case_new_style_option_key,
            follow_up: "csl26-0ksu capture-unknown-fields-wrapper",
        },
        Scenario {
            id: "06-new-top-level-section",
            category: "New top-level Style section",
            declared: Outcome::SoftDegrade,
            observed: case_new_top_level_section,
            follow_up: "csl26-0ksu capture-unknown-fields-wrapper",
        },
        Scenario {
            id: "07-new-reference-field",
            category: "New optional field on reference type",
            declared: Outcome::SoftDegrade,
            observed: case_new_reference_field,
            follow_up: "csl26-acfh reference-data-silent-acceptance",
        },
        Scenario {
            id: "08-new-locale-term-key",
            category: "New locale term key",
            declared: Outcome::SoftDegrade,
            observed: case_new_locale_term_key,
            follow_up: "csl26-o1z5 tolerant-locale-lookup",
        },
        Scenario {
            id: "09-custom-namespace",
            category: "Namespaced custom.* metadata (control)",
            declared: Outcome::Pass,
            observed: case_custom_namespace,
            follow_up: "—",
        },
        Scenario {
            id: "10-version-only-signal",
            category: "Style version bumped without other changes (control)",
            declared: Outcome::Pass,
            observed: case_version_only_signal,
            follow_up: "—",
        },
        Scenario {
            id: "11-template-grammar",
            category: "New TemplateComponent variant (opt-out)",
            declared: Outcome::HardFail,
            observed: case_template_grammar,
            follow_up: "—",
        },
        Scenario {
            id: "12-template-required-field",
            category: "Malformed template shape (opt-out)",
            declared: Outcome::HardFail,
            observed: case_template_required_field,
            follow_up: "—",
        },
    ]
}

fn render_report(rows: &[(String, Outcome, Outcome, String, String)]) -> String {
    let mut out = String::new();
    out.push_str("# Forward-compat gap snapshot\n");
    out.push_str("#\n");
    out.push_str("# Source: crates/citum-engine/tests/forward_compatibility.rs\n");
    out.push_str("# Spec:   docs/specs/FORWARD_COMPATIBILITY.md\n");
    out.push_str("#\n");
    out.push_str("# Format: id | category | declared | observed | gap? | follow-up\n");
    out.push_str(
        "# A row is a GAP when declared != observed. GAP rows roll up to follow-up beans.\n",
    );
    out.push_str("#\n\n");
    for (id, declared, observed, category, follow_up) in rows {
        let gap = if declared == observed { "ok " } else { "GAP" };
        writeln!(
            out,
            "{:<28} | {:<55} | declared={:<11} observed={:<11} {} | {}",
            id,
            category,
            declared.as_str(),
            observed.as_str(),
            gap,
            follow_up,
        )
        .unwrap();
    }
    out
}

#[test]
fn forward_compat_snapshot_matches() {
    let rows: Vec<_> = scenarios()
        .into_iter()
        .map(|s| {
            let observed = (s.observed)();
            (
                s.id.to_string(),
                s.declared,
                observed,
                s.category.to_string(),
                s.follow_up.to_string(),
            )
        })
        .collect();

    let actual = render_report(&rows);

    let snapshot_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join("forward_compat_gaps.snap");

    // Update-on-demand path: setting UPDATE_FORWARD_COMPAT_SNAPSHOT=1
    // rewrites the snapshot. Otherwise we assert equality.
    if std::env::var_os("UPDATE_FORWARD_COMPAT_SNAPSHOT").is_some() {
        fs::create_dir_all(snapshot_path.parent().unwrap()).unwrap();
        fs::write(&snapshot_path, &actual).expect("write snapshot");
        return;
    }

    let expected = fs::read_to_string(&snapshot_path).unwrap_or_else(|_| {
        panic!(
            "snapshot missing at {}. Run with UPDATE_FORWARD_COMPAT_SNAPSHOT=1 to create it.",
            snapshot_path.display()
        )
    });

    if expected != actual {
        // Show full diff via assert_eq! so reviewers can see exactly which
        // (declared, observed) tuple changed.
        assert_eq!(
            expected.trim_end(),
            actual.trim_end(),
            "Forward-compat snapshot drift detected. \
             If this drift is intentional (an engine change closed or moved a gap), \
             rerun with UPDATE_FORWARD_COMPAT_SNAPSHOT=1 and review the diff.",
        );
    }
}

/// Build an engine [`citum_engine::Bibliography`] from an `InputBibliography`
/// YAML literal, keyed by the given reference IDs in order.
fn bibliography_from_yaml(yaml: &str, ids: &[&str]) -> citum_engine::Bibliography {
    let parsed: InputBibliography = serde_yaml::from_str(yaml).expect("fixture parses");
    assert_eq!(
        ids.len(),
        parsed.references.len(),
        "fixture reference count must match the expected IDs"
    );
    ids.iter()
        .map(|id| (*id).to_string())
        .zip(parsed.references)
        .collect()
}

#[test]
fn unknown_reference_field_warnings_reports_captured_keys() {
    let yaml = r#"
references:
  - id: future-ref
    class: serial-component
    type: article
    lang: zh
    title: A future-shaped article
    parent:
      type: academic-journal
      title: A future-shaped journal
"#;
    let bibliography = bibliography_from_yaml(yaml, &["future-ref"]);

    let warnings = citum_engine::api::unknown_reference_field_warnings(&bibliography);

    assert_eq!(warnings.len(), 1);
    let warning = &warnings[0];
    assert_eq!(warning.code, "unknown_reference_field");
    assert_eq!(warning.ref_id.as_deref(), Some("future-ref"));
    assert_eq!(
        warning.message,
        "Reference 'future-ref' has unknown field(s): lang, parent; \
         these fields are ignored during rendering."
    );
}

#[test]
fn unknown_reference_field_warnings_is_silent_for_clean_references() {
    let yaml = r#"
references:
  - id: clean-ref
    class: serial-component
    type: article
    language: en
    title: A well-formed article
    container:
      class: serial
      type: academic-journal
      title: A well-formed journal
"#;
    let bibliography = bibliography_from_yaml(yaml, &["clean-ref"]);

    let warnings = citum_engine::api::unknown_reference_field_warnings(&bibliography);

    assert!(warnings.is_empty(), "clean reference must not warn");
}
