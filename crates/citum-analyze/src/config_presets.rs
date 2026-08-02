/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Corpus-wide config-preset discovery: which per-concern config shapes are unnamed?
//!
//! Walks all independent CSL styles, extracts the per-concern config blocks that
//! migration populates (`contributors`, `dates`, `titles`, `locators`), and checks
//! each observed shape against the named presets in `citum-schema-style`.
//!
//! Only **unnamed** shapes — recurring ≥ [`MIN_FREQUENCY`] times and matching no
//! existing preset exactly — are emitted. These are candidates for new named presets
//! in `citum-schema-style`. Matched shapes are tallied in the per-concern summary so
//! coverage context is preserved even though individual matches are suppressed.
//!
//! Invoke via `citum-analyze <styles_dir> --config-presets [--json]`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write as _;
use std::path::Path;

use walkdir::WalkDir;

use citum_migrate::OptionsExtractor;
use citum_schema::options::{
    ContributorConfig, DateConfig, LocatorConfig, LocatorPreset, TitlesConfig,
};
use citum_schema::presets::{ContributorPreset, DatePreset, TitlePreset};
use csl_legacy::parser::parse_style;

/// Minimum number of distinct styles an unnamed config shape must appear in to be reported.
const MIN_FREQUENCY: u32 = 3;

/// Maximum example style slugs per candidate entry.
const MAX_EXAMPLES: usize = 5;

/// Maximum top-level field distance for a candidate to report a [`PresetCandidate::nearest_preset`].
///
/// Above this distance the closest preset is not a meaningful comparison point, so the field is
/// left `None` rather than suggesting an unrelated preset.
const MAX_NEAREST_DISTANCE: usize = 3;

/// A recurring config shape that does not match any existing named preset.
#[derive(Debug, serde::Serialize)]
pub struct PresetCandidate {
    /// Number of corpus styles sharing this exact config shape.
    pub corpus_count: u32,
    /// Share of this concern's unmatched styles accounted for by this shape.
    pub share_of_unmatched: f64,
    /// Up to [`MAX_EXAMPLES`] style slugs that use this config.
    pub example_styles: Vec<String>,
    /// Serialized config block: the literal shape a new preset would encode.
    pub canonical_config: serde_json::Value,
    /// Name of the closest existing named preset by top-level field distance, if any preset is
    /// within [`MAX_NEAREST_DISTANCE`] fields.
    pub nearest_preset: Option<String>,
    /// Top-level field keys where this shape differs from `nearest_preset`.
    pub differing_fields: Vec<String>,
}

/// Per-concern summary: preset coverage count and unnamed-candidate list.
#[derive(Debug, serde::Serialize)]
pub struct ConcernReport {
    /// Concern name: `"contributors"`, `"dates"`, `"titles"`, or `"locators"`.
    pub concern: String,
    /// Styles whose non-default config for this concern matched a named preset exactly.
    pub matched_style_count: u32,
    /// Styles with a non-default config that did not match any preset.
    pub unmatched_style_count: u32,
    /// Unnamed shapes above [`MIN_FREQUENCY`], ranked by corpus count descending.
    pub candidates: Vec<PresetCandidate>,
}

/// Full config-preset analysis report.
#[derive(Debug, Default, serde::Serialize)]
pub struct ConfigPresetReport {
    /// Total CSL styles analyzed.
    pub total_analyzed: u32,
    /// Styles that failed to parse or load options.
    pub parse_errors: u32,
    /// Per-concern results in order: contributors, dates, titles, locators.
    pub concerns: Vec<ConcernReport>,
}

/// Run the config-preset analysis and emit the report to stdout or stderr.
pub fn run_config_presets(styles_dir: &str, json_output: bool) {
    let report = analyze_config_presets(Path::new(styles_dir));
    if json_output {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => writeln!(std::io::stdout(), "{json}").unwrap_or(()),
            Err(err) => eprintln!("Error: serializing config-preset report: {err}"),
        }
    } else {
        print_config_preset_report(&report);
    }
}

fn analyze_config_presets(styles_dir: &Path) -> ConfigPresetReport {
    let entries: Vec<_> = WalkDir::new(styles_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "csl"))
        .collect();
    let corpus_size = entries.len();
    eprintln!("Analyzing {corpus_size} styles for config-preset gaps...");

    // concern name → (canonical_key → (count, examples, display value))
    let mut concern_maps: [HashMap<String, (u32, Vec<String>, serde_json::Value)>; 4] =
        Default::default();

    let mut total_analyzed = 0u32;
    let mut parse_errors = 0u32;

    for (i, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let slug = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let Ok(content) = std::fs::read_to_string(path) else {
            parse_errors += 1;
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&content) else {
            parse_errors += 1;
            continue;
        };
        let Ok(legacy) = parse_style(doc.root_element()) else {
            parse_errors += 1;
            continue;
        };

        // Options extraction: XML attribute parsing only — no compile or engine call.
        let config = OptionsExtractor::extract_migration_options(&legacy).options;
        total_analyzed += 1;

        if let Some(v) = config.contributors
            && v != ContributorConfig::default()
        {
            accumulate(&mut concern_maps[0], &slug, &v);
        }
        if let Some(v) = config.dates
            && v != DateConfig::default()
        {
            accumulate(&mut concern_maps[1], &slug, &v);
        }
        if let Some(v) = config.titles
            && v != TitlesConfig::default()
        {
            accumulate(&mut concern_maps[2], &slug, &v);
        }
        if let Some(v) = config.locators
            && v != LocatorConfig::default()
        {
            accumulate(&mut concern_maps[3], &slug, &v);
        }

        if (i + 1) % 500 == 0 {
            eprintln!("  {}/{corpus_size}", i + 1);
        }
    }
    eprintln!("  done ({corpus_size} styles).");

    let [contributor_map, date_map, title_map, locator_map] = concern_maps;

    let concerns = vec![
        build_concern("contributors", contributor_map, &contributor_named_keys()),
        build_concern("dates", date_map, &date_named_keys()),
        build_concern("titles", title_map, &title_named_keys()),
        build_concern("locators", locator_map, &locator_named_keys()),
    ];

    ConfigPresetReport {
        total_analyzed,
        parse_errors,
        concerns,
    }
}

/// Accumulate a serializable config value into a frequency map.
fn accumulate(
    map: &mut HashMap<String, (u32, Vec<String>, serde_json::Value)>,
    slug: &str,
    value: &impl serde::Serialize,
) {
    let raw = serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
    let normalized = normalize_shape(&sort_json_keys(&raw));
    let key = normalized.to_string();
    let entry = map.entry(key).or_insert((0, Vec::new(), normalized));
    entry.0 += 1;
    if entry.1.len() < MAX_EXAMPLES {
        entry.1.push(slug.to_string());
    }
}

/// Build a [`ConcernReport`] by comparing accumulated keys against the named-preset set.
fn build_concern(
    name: &str,
    counts: HashMap<String, (u32, Vec<String>, serde_json::Value)>,
    named_presets: &[(String, serde_json::Value)],
) -> ConcernReport {
    let named_keys: HashSet<String> = named_presets.iter().map(|(_, v)| v.to_string()).collect();

    let mut matched = 0u32;
    let mut unmatched = 0u32;
    let mut pending = Vec::new();

    for (key, (count, examples, canonical_config)) in counts {
        if named_keys.contains(&key) {
            matched += count;
        } else {
            unmatched += count;
            if count >= MIN_FREQUENCY {
                pending.push((count, examples, canonical_config));
            }
        }
    }

    let mut candidates: Vec<PresetCandidate> = pending
        .into_iter()
        .map(|(count, examples, canonical_config)| {
            let (nearest_preset, differing_fields) =
                nearest_preset(&canonical_config, named_presets);
            PresetCandidate {
                corpus_count: count,
                share_of_unmatched: if unmatched == 0 {
                    0.0
                } else {
                    f64::from(count) / f64::from(unmatched)
                },
                example_styles: examples,
                canonical_config,
                nearest_preset,
                differing_fields,
            }
        })
        .collect();
    // Tie-break on the canonical shape string: `counts` is a HashMap, so candidates with equal
    // corpus_count would otherwise print in an arbitrary, run-to-run-unstable order.
    candidates.sort_by(|a, b| {
        b.corpus_count.cmp(&a.corpus_count).then_with(|| {
            a.canonical_config
                .to_string()
                .cmp(&b.canonical_config.to_string())
        })
    });

    ConcernReport {
        concern: name.to_string(),
        matched_style_count: matched,
        unmatched_style_count: unmatched,
        candidates,
    }
}

/// Find the closest named preset to `shape` by top-level field distance.
///
/// Distance is the number of top-level keys present in exactly one of the two objects, plus one
/// for each shared key whose values differ. Returns `None` when no preset is within
/// [`MAX_NEAREST_DISTANCE`], since an unrelated preset is not a useful comparison point.
fn nearest_preset(
    shape: &serde_json::Value,
    named_presets: &[(String, serde_json::Value)],
) -> (Option<String>, Vec<String>) {
    // Only object shapes are comparable field-by-field; every canonical_config in practice is
    // one (all Config structs serialize to JSON objects), but a non-object must never be treated
    // as "close" to a preset by falling through the distance check below.
    if !shape.is_object() {
        return (None, Vec::new());
    }

    let mut best: Option<(usize, &str, Vec<String>)> = None;

    for (name, preset_shape) in named_presets {
        let diff = top_level_diff(shape, preset_shape);
        if diff.len() > MAX_NEAREST_DISTANCE {
            continue;
        }
        if best.as_ref().is_none_or(|(dist, ..)| diff.len() < *dist) {
            best = Some((diff.len(), name.as_str(), diff));
        }
    }

    match best {
        Some((_, name, fields)) => (Some(name.to_string()), fields),
        None => (None, Vec::new()),
    }
}

/// Field keys that differ between two top-level JSON objects (present in only one, or with
/// unequal values). A non-object input is never comparable field-by-field, so it always exceeds
/// [`MAX_NEAREST_DISTANCE`] rather than reading as a deceptively small 1-field diff.
fn top_level_diff(a: &serde_json::Value, b: &serde_json::Value) -> Vec<String> {
    let (Some(a), Some(b)) = (a.as_object(), b.as_object()) else {
        return (0..=MAX_NEAREST_DISTANCE)
            .map(|i| format!("<non-object-{i}>"))
            .collect();
    };
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .cloned()
        .collect()
}

/// Strip object keys whose value is JSON `null`.
///
/// A `null` here means "not configured at this level" for an `Option<T>` field — it carries no
/// signal distinct from the field being absent, and named presets never emit it (they set
/// `Some(default)` explicitly, e.g. `ContributorPreset` always sets `delimiter`). Left unstripped,
/// an extracted config that is otherwise identical to a preset but has one `null` optional field
/// can never match that preset by JSON string equality. See
/// `ContributorConfig::is_default_contributor_delimiter` for the field that motivated this.
fn normalize_shape(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let new_map: serde_json::Map<_, _> = map
                .iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), normalize_shape(v)))
                .collect();
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_shape).collect())
        }
        other => other.clone(),
    }
}

/// Recursively sort JSON object keys for a stable canonical representation.
fn sort_json_keys(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<_, _> = map.iter().collect();
            let new_map: serde_json::Map<_, _> = sorted
                .into_iter()
                .map(|(k, v)| (k.clone(), sort_json_keys(v)))
                .collect();
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(sort_json_keys).collect())
        }
        other => other.clone(),
    }
}

// ── Preset enumerators ──────────────────────────────────────────────────────
//
// These lists enumerate all current variants of each preset enum so the report
// can reverse-match observed configs against them. Keep in sync with the enum
// definitions in `citum-schema-style/src/presets.rs` and
// `citum-schema-style/src/options/locators.rs`.

/// Serialize `(name, config)` pairs into `(kebab-case name, normalized canonical shape)` pairs.
///
/// Driven by each preset enum's own `ALL` const rather than a hand-listed array, so a preset
/// variant can never be silently missing from analyzer matching — see the enums' `ALL` docs.
fn preset_keys<P: serde::Serialize>(
    presets: impl IntoIterator<Item = (P, impl serde::Serialize)>,
) -> Vec<(String, serde_json::Value)> {
    presets
        .into_iter()
        .map(|(name, config)| {
            let name_json = serde_json::to_value(&name).unwrap_or(serde_json::Value::Null);
            let name = name_json.as_str().unwrap_or_default().to_string();
            let raw = serde_json::to_value(&config).unwrap_or(serde_json::Value::Null);
            (name, normalize_shape(&sort_json_keys(&raw)))
        })
        .collect()
}

fn contributor_named_keys() -> Vec<(String, serde_json::Value)> {
    preset_keys(ContributorPreset::ALL.iter().map(|p| (p, p.config())))
}

fn date_named_keys() -> Vec<(String, serde_json::Value)> {
    preset_keys(DatePreset::ALL.iter().map(|p| (p, p.config())))
}

fn title_named_keys() -> Vec<(String, serde_json::Value)> {
    preset_keys(TitlePreset::ALL.iter().map(|p| (p, p.config())))
}

fn locator_named_keys() -> Vec<(String, serde_json::Value)> {
    preset_keys(LocatorPreset::ALL.iter().map(|p| (p, p.config())))
}

// ── Human-readable output ───────────────────────────────────────────────────

#[allow(clippy::cognitive_complexity, reason = "macro-heavy output code")]
fn print_config_preset_report(report: &ConfigPresetReport) {
    println!("=== Config-Preset Discovery Report ===\n");
    println!("Styles analyzed: {}", report.total_analyzed);
    println!("Parse errors:    {}", report.parse_errors);
    println!();

    for concern in &report.concerns {
        println!("=== {} ===", concern.concern.to_ascii_uppercase());
        println!(
            "  {} styles matched existing presets, {} styles unmatched",
            concern.matched_style_count, concern.unmatched_style_count
        );
        if concern.candidates.is_empty() {
            println!("  (no unnamed shapes ≥ {MIN_FREQUENCY} styles above threshold)");
        } else {
            println!(
                "  {} unnamed shapes (≥ {MIN_FREQUENCY} styles):\n",
                concern.candidates.len()
            );
            println!("  {:>6}  {:>7}  Examples", "Styles", "Share");
            println!("  {}", "-".repeat(60));
            for (rank, c) in concern.candidates.iter().enumerate() {
                let examples = c.example_styles.join(", ");
                let examples_trunc = if examples.chars().count() > 44 {
                    format!("{}…", examples.chars().take(43).collect::<String>())
                } else {
                    examples
                };
                println!(
                    "  {:>6}  {:>6.1}%  {}",
                    c.corpus_count,
                    c.share_of_unmatched * 100.0,
                    examples_trunc
                );
                let config_str = serde_json::to_string(&c.canonical_config)
                    .unwrap_or_else(|_| String::from("{?}"));
                let config_trunc = if config_str.chars().count() > 100 {
                    format!("{}…", config_str.chars().take(99).collect::<String>())
                } else {
                    config_str
                };
                println!("         Config[{}]: {config_trunc}", rank + 1);
                match (&c.nearest_preset, c.differing_fields.is_empty()) {
                    (Some(name), false) => {
                        println!(
                            "         Nearest preset: {name} (differs: {})",
                            c.differing_fields.join(", ")
                        );
                    }
                    (Some(name), true) => println!("         Nearest preset: {name} (identical?)"),
                    (None, _) => println!("         Nearest preset: none within range"),
                }
                println!();
            }
        }
        println!();
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_shape_strips_null_keys() {
        let shape = serde_json::json!({
            "delimiter": null,
            "and": "text",
            "shorten": { "min": 3, "use-first": null },
        });

        let normalized = normalize_shape(&shape);

        assert_eq!(
            normalized,
            serde_json::json!({
                "and": "text",
                "shorten": { "min": 3 },
            })
        );
    }

    #[test]
    fn test_build_concern_matches_null_delimiter_variant_of_a_preset() {
        // Regression for the fragmentation bug: an extracted config that differs from a preset
        // only by an explicit `delimiter: null` (vs. the preset's omitted default) must match.
        let apa_shape = normalize_shape(&sort_json_keys(
            &serde_json::to_value(ContributorPreset::Apa.config()).unwrap(),
        ));
        let mut extracted = apa_shape.as_object().unwrap().clone();
        extracted.insert("delimiter".to_string(), serde_json::Value::Null);
        let extracted_shape =
            normalize_shape(&sort_json_keys(&serde_json::Value::Object(extracted)));

        let mut counts = HashMap::new();
        counts.insert(
            extracted_shape.to_string(),
            (5u32, vec!["some-style".to_string()], extracted_shape),
        );

        let report = build_concern("contributors", counts, &contributor_named_keys());

        assert_eq!(report.matched_style_count, 5);
        assert_eq!(report.unmatched_style_count, 0);
        assert!(report.candidates.is_empty());
    }

    #[test]
    fn test_build_concern_matches_locator_numeric_preset_shape() {
        // The locator candidate this bean added (author-date + strip-label-periods) must now
        // resolve as `numeric`, not appear as an unmatched candidate.
        let numeric_shape = normalize_shape(&sort_json_keys(
            &serde_json::to_value(LocatorPreset::Numeric.config()).unwrap(),
        ));

        let mut counts = HashMap::new();
        counts.insert(
            numeric_shape.to_string(),
            (120u32, vec!["some-journal".to_string()], numeric_shape),
        );

        let report = build_concern("locators", counts, &locator_named_keys());

        assert_eq!(report.matched_style_count, 120);
        assert_eq!(report.unmatched_style_count, 0);
        assert!(report.candidates.is_empty());
    }

    #[test]
    fn test_build_concern_matches_title_default_only_shapes() {
        let emphasis_shape = normalize_shape(&sort_json_keys(
            &serde_json::to_value(TitlePreset::EmphasisAll.config()).unwrap(),
        ));
        let title_case_shape = normalize_shape(&sort_json_keys(
            &serde_json::to_value(TitlePreset::TitleCase.config()).unwrap(),
        ));

        let mut counts = HashMap::new();
        counts.insert(
            emphasis_shape.to_string(),
            (830u32, vec!["style-a".to_string()], emphasis_shape),
        );
        counts.insert(
            title_case_shape.to_string(),
            (208u32, vec!["style-b".to_string()], title_case_shape),
        );

        let report = build_concern("titles", counts, &title_named_keys());

        assert_eq!(report.matched_style_count, 830 + 208);
        assert_eq!(report.unmatched_style_count, 0);
        assert!(report.candidates.is_empty());
    }

    #[test]
    fn test_nearest_preset_reports_preset_one_field_away() {
        let author_date_shape = normalize_shape(&sort_json_keys(
            &serde_json::to_value(LocatorPreset::AuthorDate.config()).unwrap(),
        ));
        let mut candidate = author_date_shape.as_object().unwrap().clone();
        candidate.insert("strip-label-periods".to_string(), serde_json::json!(true));
        let candidate_shape = serde_json::Value::Object(candidate);

        let named = vec![("author-date".to_string(), author_date_shape)];
        let (nearest, differing) = nearest_preset(&candidate_shape, &named);

        assert_eq!(nearest, Some("author-date".to_string()));
        assert_eq!(differing, vec!["strip-label-periods".to_string()]);
    }

    #[test]
    fn test_nearest_preset_returns_none_when_every_preset_is_far() {
        let unrelated_shape = serde_json::json!({
            "a": 1, "b": 2, "c": 3, "d": 4, "e": 5,
        });
        let named = vec![("author-date".to_string(), serde_json::json!({ "a": 0 }))];

        let (nearest, differing) = nearest_preset(&unrelated_shape, &named);

        assert_eq!(nearest, None);
        assert!(differing.is_empty());
    }

    #[test]
    fn test_nearest_preset_ignores_non_object_shapes() {
        // Regression: a non-object shape must never read as a deceptively small 1-field diff
        // and get reported as "close" to an unrelated preset.
        let non_object_shape = serde_json::json!(null);
        let named = vec![("author-date".to_string(), serde_json::json!({ "a": 0 }))];

        let (nearest, differing) = nearest_preset(&non_object_shape, &named);

        assert_eq!(nearest, None);
        assert!(differing.is_empty());
    }

    #[test]
    fn test_named_keys_functions_never_panic() {
        // ALL is what the analyzer's named_keys functions iterate; a variant missing its config()
        // dispatch arm would panic here rather than silently vanishing from the analyzer.
        for (name, _) in contributor_named_keys() {
            assert!(!name.is_empty());
        }
        for (name, _) in date_named_keys() {
            assert!(!name.is_empty());
        }
        for (name, _) in title_named_keys() {
            assert!(!name.is_empty());
        }
        for (name, _) in locator_named_keys() {
            assert!(!name.is_empty());
        }
    }
}
