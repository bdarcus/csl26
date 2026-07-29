/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Batch Migration Tester — runs every CSL 1.0 style in a directory through
//! the full migrate + engine pipeline in-process and reports pass/fail rates.
//!
//! Running in-process (no `cargo run` spawns) makes a full-corpus pass take
//! seconds rather than hours.
//!
//! Usage: `citum-batch-test <styles_dir> [--verbose] [--sample N] [--json]`

#![allow(missing_docs, reason = "bin crate")]

use std::collections::HashMap;
use std::env;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use citum_migrate::{OptionsExtractor, compilation, provenance::ProvenanceTracker};
use citum_schema::{BibliographySpec, CitationSpec, Style};

/// Parsed CLI arguments for `citum-batch-test`.
struct CliArgs {
    /// Path to the directory containing `.csl` files.
    styles_dir: String,
    /// Print each style's result individually.
    verbose: bool,
    /// Emit final results as JSON.
    json_output: bool,
    /// Run on a pseudo-random sample of this size instead of the full corpus.
    sample_size: Option<usize>,
}

fn parse_cli(args: &[String]) -> CliArgs {
    CliArgs {
        styles_dir: args.get(1).cloned().unwrap_or_default(),
        verbose: args.iter().any(|a| a == "--verbose"),
        json_output: args.iter().any(|a| a == "--json"),
        sample_size: args
            .iter()
            .position(|a| a == "--sample")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok()),
    }
}

fn collect_styles(styles_dir: &str, sample_size: Option<usize>) -> Vec<PathBuf> {
    let mut styles: Vec<_> = WalkDir::new(styles_dir)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "csl"))
        .map(|e| e.path().to_path_buf())
        .collect();

    if let Some(n) = sample_size {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        styles.sort_by(|a, b| {
            let mut ha = DefaultHasher::new();
            let mut hb = DefaultHasher::new();
            a.hash(&mut ha);
            b.hash(&mut hb);
            ha.finish().cmp(&hb.finish())
        });
        styles.truncate(n);
    }

    styles
}

/// Aggregated results across all tested styles.
#[derive(Default, serde::Serialize)]
struct BatchResults {
    total: usize,
    migration_success: usize,
    migration_failed: usize,
    processor_failed: usize,
    yaml_invalid: usize,
    migration_errors: HashMap<String, usize>,
    processor_errors: HashMap<String, usize>,
    yaml_errors: HashMap<String, usize>,
}

/// Per-style test outcome.
enum TestResult {
    /// Migrate + schema roundtrip + engine instantiation all passed.
    Success,
    /// Legacy CSL parse or migrate compilation failed.
    MigrationFailed(String),
    /// Assembled `Style` could not be round-tripped through `serde_yaml`.
    YamlInvalid(String),
    /// Engine `Processor` could not be instantiated from the migrated style.
    ProcessorFailed(String),
}

/// Test a single CSL style file in-process.
///
/// 1. Parses the legacy CSL XML with `csl_legacy::parser::parse_style`.
/// 2. Runs the migrate XML compilation pipeline (`compile_from_xml`).
/// 3. Assembles a minimal `citum_schema::Style` and round-trips it through `serde_yaml`.
/// 4. Instantiates `citum_engine::Processor::new` to verify engine acceptance.
fn test_style(path: &Path) -> TestResult {
    // Step 1: parse legacy XML
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return TestResult::MigrationFailed(format!("read: {e}")),
    };
    let doc = match roxmltree::Document::parse(&content) {
        Ok(d) => d,
        Err(e) => return TestResult::MigrationFailed(format!("xml parse: {e}")),
    };
    let legacy = match csl_legacy::parser::parse_style(doc.root_element()) {
        Ok(s) => s,
        Err(e) => return TestResult::MigrationFailed(format!("csl parse: {e}")),
    };

    // Step 2: migrate in-process
    let opts = OptionsExtractor::extract_migration_options(&legacy);
    let mut options = opts.options;
    let tracker = ProvenanceTracker::new(false);
    let output = compilation::compile_from_xml(&legacy, &mut options, false, &tracker);

    // Step 3: assemble minimal Style and validate YAML roundtrip
    let style = Style {
        options: Some(options),
        bibliography: Some(BibliographySpec {
            template: if output.bibliography.is_empty() {
                None
            } else {
                Some(output.bibliography)
            },
            ..Default::default()
        }),
        citation: Some(CitationSpec {
            template: if output.citation.is_empty() {
                None
            } else {
                Some(output.citation)
            },
            ..Default::default()
        }),
        ..Default::default()
    };
    let yaml = match serde_yaml::to_string(&style) {
        Ok(y) => y,
        Err(e) => return TestResult::YamlInvalid(format!("serialize: {e}")),
    };
    if let Err(e) = Style::from_yaml_str(&yaml) {
        return TestResult::YamlInvalid(e.to_string());
    }

    // Step 4: engine instantiation check
    let bib: citum_engine::Bibliography = indexmap::IndexMap::new();
    let catch = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        citum_engine::Processor::new(style, bib)
    }));
    if let Err(e) = catch {
        let msg = e
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| e.downcast_ref::<&str>().copied())
            .unwrap_or("panic")
            .to_string();
        return TestResult::ProcessorFailed(msg);
    }

    TestResult::Success
}

#[allow(clippy::cognitive_complexity, reason = "macro-heavy output code")]
fn run_batch(cli: CliArgs) {
    let styles = collect_styles(&cli.styles_dir, cli.sample_size);
    let total = styles.len();
    let mut results = BatchResults::default();

    eprintln!("Testing {total} styles...");

    for (i, path) in styles.iter().enumerate() {
        let result = test_style(path);
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        match &result {
            TestResult::Success => {
                results.migration_success += 1;
                if cli.verbose {
                    eprintln!("[{}/{}] ✅ {}", i + 1, total, name);
                }
            }
            TestResult::MigrationFailed(err) => {
                results.migration_failed += 1;
                *results
                    .migration_errors
                    .entry(categorize_error(err))
                    .or_insert(0) += 1;
                if cli.verbose {
                    eprintln!(
                        "[{}/{}] ❌ {} — Migration: {}",
                        i + 1,
                        total,
                        name,
                        truncate(err, 60)
                    );
                }
            }
            TestResult::ProcessorFailed(err) => {
                results.processor_failed += 1;
                *results
                    .processor_errors
                    .entry(categorize_error(err))
                    .or_insert(0) += 1;
                if cli.verbose {
                    eprintln!(
                        "[{}/{}] ⚠️  {} — Processor: {}",
                        i + 1,
                        total,
                        name,
                        truncate(err, 60)
                    );
                }
            }
            TestResult::YamlInvalid(err) => {
                results.yaml_invalid += 1;
                *results
                    .yaml_errors
                    .entry(categorize_error(err))
                    .or_insert(0) += 1;
                if cli.verbose {
                    eprintln!(
                        "[{}/{}] ❌ {} — Invalid YAML: {}",
                        i + 1,
                        total,
                        name,
                        truncate(err, 60)
                    );
                }
            }
        }

        if !cli.verbose && (i + 1) % 100 == 0 {
            eprintln!("  Processed {}/{}", i + 1, total);
        }
    }

    results.total = total;

    if cli.json_output {
        match serde_json::to_string_pretty(&results) {
            Ok(json) => writeln!(std::io::stdout(), "{json}").unwrap_or(()),
            Err(err) => eprintln!("Error: serializing batch results: {err}"),
        }
    } else {
        print_results(&results);
    }
}

#[allow(clippy::cognitive_complexity, reason = "macro-heavy output code")]
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: citum-batch-test <styles_dir> [--verbose] [--sample N] [--json]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --verbose    Show individual style results");
        eprintln!("  --sample N   Only test N pseudo-random styles");
        eprintln!("  --json       Output final results as JSON");
        std::process::exit(1);
    }
    run_batch(parse_cli(&args));
}

fn categorize_error(err: &str) -> String {
    if err.contains("unknown attribute")
        && let Some(attr) = err.split("unknown attribute: ").nth(1)
    {
        return format!(
            "unknown attr: {}",
            attr.split_whitespace().next().unwrap_or("?")
        );
    }
    if err.contains("Unknown top-level tag") {
        return "unknown top-level tag".to_string();
    }
    if err.contains("missing field")
        && let Some(field) = err.split("missing field").nth(1)
    {
        return format!(
            "missing field:{}",
            field.chars().take(20).collect::<String>()
        );
    }
    if err.contains("unknown variant") {
        return "unknown variant".to_string();
    }
    if err.contains("Error parsing style") {
        return "parse error".to_string();
    }
    err.lines()
        .next()
        .unwrap_or("unknown")
        .chars()
        .take(40)
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.chars().count() > max {
        format!("{}…", first_line.chars().take(max).collect::<String>())
    } else {
        first_line.to_string()
    }
}

#[allow(clippy::cognitive_complexity, reason = "macro-heavy output code")]
fn print_results(results: &BatchResults) {
    println!("\n=== Batch Migration Test Results ===\n");
    println!("Total styles tested: {}", results.total);
    println!();
    let success_rate = if results.total > 0 {
        (results.migration_success as f64 / results.total as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "Migration + Engine Success: {} ({:.1}%)",
        results.migration_success, success_rate
    );
    println!("Migration Failed:          {}", results.migration_failed);
    println!("YAML Invalid:              {}", results.yaml_invalid);
    println!("Processor Failed:          {}", results.processor_failed);

    if !results.migration_errors.is_empty() {
        println!("\n--- Migration Errors ---");
        print_error_summary(&results.migration_errors);
    }
    if !results.yaml_errors.is_empty() {
        println!("\n--- YAML Validation Errors ---");
        print_error_summary(&results.yaml_errors);
    }
    if !results.processor_errors.is_empty() {
        println!("\n--- Processor Errors ---");
        print_error_summary(&results.processor_errors);
    }
}

fn print_error_summary(errors: &HashMap<String, usize>) {
    let mut items: Vec<_> = errors.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));
    for (error, count) in items.iter().take(10) {
        println!("  {count:5} - {error}");
    }
    if items.len() > 10 {
        println!("  … and {} more error types", items.len() - 10);
    }
}
