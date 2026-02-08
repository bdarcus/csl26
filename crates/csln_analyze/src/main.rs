/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSL Style Analyzer
//!
//! Analyzes CSL 1.0 styles in a directory to collect statistics
//! and identify patterns for guiding migration development.

mod analyzer;
mod ranker;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let styles_dir = &args[1];
    let json_output = args.contains(&"--json".to_string());
    let rank_parents = args.contains(&"--rank-parents".to_string());

    // Check for format filter (--format author-date, --format numeric, etc.)
    let format_filter = args
        .iter()
        .position(|a| a == "--format")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    if rank_parents {
        ranker::run_parent_ranker(styles_dir, json_output, format_filter);
    } else {
        analyzer::run_style_analyzer(styles_dir, json_output);
    }
}

fn print_usage() {
    eprintln!("CSL Style Analyzer");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  csln_analyze <styles_dir> [--json]");
    eprintln!("      Analyze all .csl files and report feature statistics.");
    eprintln!();
    eprintln!("  csln_analyze <styles_dir> --rank-parents [--json] [--format <format>]");
    eprintln!("      Rank parent styles by how many dependent styles reference them.");
    eprintln!(
        "      Use --format to filter by citation format (author-date, numeric, note, label)."
    );
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  csln_analyze styles-legacy/");
    eprintln!("  csln_analyze styles-legacy/ --rank-parents");
    eprintln!("  csln_analyze styles-legacy/ --rank-parents --format author-date --json");
}
