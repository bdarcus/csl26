/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Batch Migration Tester
//!
//! Runs CSL 1.0 → CSLN migration on all styles and reports success rates.
//!
//! Usage: csln_batch_test <styles_dir> [--verbose] [--sample N]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: csln_batch_test <styles_dir> [--verbose] [--sample N]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --verbose    Show individual style results");
        eprintln!("  --sample N   Only test N random styles");
        eprintln!("  --json       Output as JSON");
        std::process::exit(1);
    }
    
    let styles_dir = &args[1];
    let verbose = args.contains(&"--verbose".to_string());
    let json_output = args.contains(&"--json".to_string());
    let sample_size: Option<usize> = args.iter()
        .position(|a| a == "--sample")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());
    
    // Collect all .csl files
    let mut styles: Vec<_> = WalkDir::new(styles_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "csl").unwrap_or(false))
        .map(|e| e.path().to_path_buf())
        .collect();
    
    // Sample if requested
    if let Some(n) = sample_size {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Deterministic shuffle based on filename hash
        styles.sort_by(|a, b| {
            let mut ha = DefaultHasher::new();
            let mut hb = DefaultHasher::new();
            a.hash(&mut ha);
            b.hash(&mut hb);
            ha.finish().cmp(&hb.finish())
        });
        styles.truncate(n);
    }
    
    let total = styles.len();
    let mut results = BatchResults::default();
    
    eprintln!("Testing {} styles...\n", total);
    
    for (i, path) in styles.iter().enumerate() {
        let result = test_style(path);
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        
        match &result {
            TestResult::Success => {
                results.migration_success += 1;
                if verbose {
                    eprintln!("[{}/{}] ✅ {}", i + 1, total, name);
                }
            }
            TestResult::MigrationFailed(err) => {
                results.migration_failed += 1;
                *results.migration_errors.entry(categorize_error(err)).or_insert(0) += 1;
                if verbose {
                    eprintln!("[{}/{}] ❌ {} - Migration: {}", i + 1, total, name, truncate(err, 60));
                }
            }
            TestResult::ProcessorFailed(err) => {
                results.processor_failed += 1;
                *results.processor_errors.entry(categorize_error(err)).or_insert(0) += 1;
                if verbose {
                    eprintln!("[{}/{}] ⚠️  {} - Processor: {}", i + 1, total, name, truncate(err, 60));
                }
            }
            TestResult::YamlInvalid(err) => {
                results.yaml_invalid += 1;
                *results.yaml_errors.entry(categorize_error(err)).or_insert(0) += 1;
                if verbose {
                    eprintln!("[{}/{}] ❌ {} - Invalid YAML: {}", i + 1, total, name, truncate(err, 60));
                }
            }
        }
        
        // Progress indicator every 100 styles
        if !verbose && (i + 1) % 100 == 0 {
            eprintln!("  Processed {}/{}", i + 1, total);
        }
    }
    
    results.total = total;
    
    if json_output {
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    } else {
        print_results(&results);
    }
}

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

enum TestResult {
    Success,
    MigrationFailed(String),
    ProcessorFailed(String),
    YamlInvalid(String),
}

fn test_style(path: &Path) -> TestResult {
    // Step 1: Run migration
    let migrate_output = Command::new("cargo")
        .args(["run", "-q", "--bin", "csln_migrate", "--"])
        .arg(path)
        .output();
    
    let migrate_output = match migrate_output {
        Ok(o) => o,
        Err(e) => return TestResult::MigrationFailed(format!("spawn error: {}", e)),
    };
    
    if !migrate_output.status.success() {
        let stderr = String::from_utf8_lossy(&migrate_output.stderr);
        return TestResult::MigrationFailed(stderr.to_string());
    }
    
    let yaml_content = String::from_utf8_lossy(&migrate_output.stdout);
    
    // Step 2: Validate YAML parses as Style
    let style_result: Result<csln_core::Style, _> = serde_yaml::from_str(&yaml_content);
    if let Err(e) = style_result {
        return TestResult::YamlInvalid(e.to_string());
    }
    
    // Step 3: Write to temp file and run processor
    let temp_path = std::env::temp_dir().join("csln_batch_test.yaml");
    if let Err(e) = fs::write(&temp_path, yaml_content.as_bytes()) {
        return TestResult::ProcessorFailed(format!("write error: {}", e));
    }
    
    let proc_output = Command::new("cargo")
        .args(["run", "-q", "--bin", "csln_processor", "--"])
        .arg(&temp_path)
        .output();
    
    let proc_output = match proc_output {
        Ok(o) => o,
        Err(e) => return TestResult::ProcessorFailed(format!("spawn error: {}", e)),
    };
    
    if !proc_output.status.success() {
        let stderr = String::from_utf8_lossy(&proc_output.stderr);
        return TestResult::ProcessorFailed(stderr.to_string());
    }
    
    // Clean up
    let _ = fs::remove_file(&temp_path);
    
    TestResult::Success
}

fn categorize_error(err: &str) -> String {
    // Extract meaningful error category from error message
    if err.contains("unknown attribute") {
        if let Some(attr) = err.split("unknown attribute: ").nth(1) {
            return format!("unknown attr: {}", attr.split_whitespace().next().unwrap_or("?"));
        }
    }
    if err.contains("Unknown top-level tag") {
        return "unknown top-level tag".to_string();
    }
    if err.contains("missing field") {
        if let Some(field) = err.split("missing field").nth(1) {
            return format!("missing field:{}", field.chars().take(20).collect::<String>());
        }
    }
    if err.contains("unknown variant") {
        return "unknown variant".to_string();
    }
    if err.contains("Error parsing style") {
        return "parse error".to_string();
    }
    
    // Truncate to first line
    err.lines().next().unwrap_or("unknown").chars().take(40).collect()
}

fn truncate(s: &str, max: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max {
        format!("{}...", &first_line[..max])
    } else {
        first_line.to_string()
    }
}

fn print_results(results: &BatchResults) {
    println!("\n=== Batch Migration Test Results ===\n");
    println!("Total styles tested: {}", results.total);
    println!();
    
    let success_rate = (results.migration_success as f64 / results.total as f64) * 100.0;
    println!("Migration + Processor Success: {} ({:.1}%)", results.migration_success, success_rate);
    println!("Migration Failed: {}", results.migration_failed);
    println!("YAML Invalid: {}", results.yaml_invalid);
    println!("Processor Failed: {}", results.processor_failed);
    
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
        println!("  {:5} - {}", count, error);
    }
    if items.len() > 10 {
        println!("  ... and {} more error types", items.len() - 10);
    }
}
