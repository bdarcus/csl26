/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSL Style Analyzer
//!
//! Analyzes CSL 1.0 styles in a directory to collect statistics
//! and identify patterns for guiding migration development.
//!
//! Usage:
//!   csln_analyze <styles_dir> [--json]              # Full style analysis
//!   csln_analyze <styles_dir> --rank-parents [--json] [--format <format>]
//!                                                    # Rank parent styles by dependent count
//!
//! The --rank-parents mode analyzes dependent styles to identify which parent
//! styles are most widely used. This helps prioritize rendering development.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

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
        run_parent_ranker(styles_dir, json_output, format_filter);
    } else {
        run_style_analyzer(styles_dir, json_output);
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
    eprintln!("  csln_analyze styles/");
    eprintln!("  csln_analyze styles/ --rank-parents");
    eprintln!("  csln_analyze styles/ --rank-parents --format author-date --json");
}

// ============================================================================
// Parent Style Ranker
// ============================================================================

/// Statistics for parent style ranking.
#[derive(Default, serde::Serialize)]
struct ParentRankerStats {
    /// Total dependent styles analyzed
    total_dependent: u32,
    /// Total independent (parent) styles found
    total_independent: u32,
    /// Parse errors encountered
    parse_errors: Vec<String>,
    /// Filter applied (if any)
    format_filter: Option<String>,
    /// Parent styles ranked by dependent count
    parent_rankings: Vec<ParentRanking>,
    /// Citation format distribution
    format_distribution: HashMap<String, u32>,
}

/// A parent style and its usage statistics.
#[derive(serde::Serialize, Clone)]
struct ParentRanking {
    /// Parent style ID (usually a Zotero URL)
    parent_id: String,
    /// Extracted short name from the ID
    short_name: String,
    /// Number of dependent styles that reference this parent
    dependent_count: u32,
    /// Percentage of all dependents (for the filtered set)
    percentage: f64,
    /// Citation format (author-date, numeric, note, label)
    format: Option<String>,
    /// Fields/disciplines that use this parent
    fields: Vec<String>,
}

fn run_parent_ranker(styles_dir: &str, json_output: bool, format_filter: Option<&str>) {
    let mut stats = ParentRankerStats {
        format_filter: format_filter.map(|s| s.to_string()),
        ..Default::default()
    };

    // Maps: parent_id -> (count, format, fields)
    let mut parent_counts: HashMap<String, (u32, Option<String>, Vec<String>)> = HashMap::new();

    // First, scan independent styles to get their format
    let independent_dir = Path::new(styles_dir);
    let mut independent_formats: HashMap<String, String> = HashMap::new();

    for entry in WalkDir::new(independent_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "csl")
                .unwrap_or(false)
        })
    {
        if let Ok(info) = extract_style_info(entry.path()) {
            stats.total_independent += 1;
            if let Some(format) = info.citation_format {
                let style_url = format!(
                    "http://www.zotero.org/styles/{}",
                    entry.path().file_stem().unwrap().to_string_lossy()
                );
                independent_formats.insert(style_url, format);
            }
        }
    }

    // Scan dependent styles directory
    let dependent_dir = Path::new(styles_dir).join("dependent");
    if !dependent_dir.exists() {
        eprintln!(
            "Warning: No 'dependent' subdirectory found in {}",
            styles_dir
        );
        eprintln!("Dependent styles are typically in styles/dependent/");
    }

    for entry in WalkDir::new(&dependent_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "csl")
                .unwrap_or(false)
        })
    {
        match extract_dependent_info(entry.path()) {
            Ok(info) => {
                // Track format distribution
                if let Some(ref fmt) = info.citation_format {
                    *stats.format_distribution.entry(fmt.clone()).or_insert(0) += 1;
                }

                // Apply format filter if specified
                if let Some(filter) = format_filter {
                    if info.citation_format.as_deref() != Some(filter) {
                        continue;
                    }
                }

                stats.total_dependent += 1;

                if let Some(parent_id) = info.parent_id {
                    let entry = parent_counts.entry(parent_id.clone()).or_insert_with(|| {
                        let format = independent_formats.get(&parent_id).cloned();
                        (0, format, Vec::new())
                    });
                    entry.0 += 1;
                    for field in info.fields {
                        if !entry.2.contains(&field) {
                            entry.2.push(field);
                        }
                    }
                }
            }
            Err(e) => {
                stats
                    .parse_errors
                    .push(format!("{}: {}", entry.path().display(), e));
            }
        }
    }

    // Build ranked list
    let mut rankings: Vec<ParentRanking> = parent_counts
        .into_iter()
        .map(|(parent_id, (count, format, mut fields))| {
            let short_name = parent_id
                .rsplit('/')
                .next()
                .unwrap_or(&parent_id)
                .to_string();
            fields.sort();
            fields.dedup();
            ParentRanking {
                parent_id,
                short_name,
                dependent_count: count,
                percentage: (count as f64 / stats.total_dependent.max(1) as f64) * 100.0,
                format,
                fields,
            }
        })
        .collect();

    // Sort by dependent count descending
    rankings.sort_by(|a, b| b.dependent_count.cmp(&a.dependent_count));
    stats.parent_rankings = rankings;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&stats).unwrap());
    } else {
        print_parent_rankings(&stats);
    }
}

/// Information extracted from a dependent style.
struct DependentInfo {
    parent_id: Option<String>,
    citation_format: Option<String>,
    fields: Vec<String>,
}

fn extract_dependent_info(path: &Path) -> Result<DependentInfo, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("parse error: {}", e))?;

    let root = doc.root_element();
    let mut parent_id = None;
    let mut citation_format = None;
    let mut fields = Vec::new();

    // Find info element
    for child in root.children() {
        if child.tag_name().name() == "info" {
            for info_child in child.children() {
                match info_child.tag_name().name() {
                    "link" => {
                        if info_child.attribute("rel") == Some("independent-parent") {
                            parent_id = info_child.attribute("href").map(|s| s.to_string());
                        }
                    }
                    "category" => {
                        if let Some(fmt) = info_child.attribute("citation-format") {
                            citation_format = Some(fmt.to_string());
                        }
                        if let Some(field) = info_child.attribute("field") {
                            fields.push(field.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(DependentInfo {
        parent_id,
        citation_format,
        fields,
    })
}

/// Information extracted from an independent style.
struct StyleInfo {
    citation_format: Option<String>,
}

fn extract_style_info(path: &Path) -> Result<StyleInfo, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;
    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("parse error: {}", e))?;

    let root = doc.root_element();
    let mut citation_format = None;

    for child in root.children() {
        if child.tag_name().name() == "info" {
            for info_child in child.children() {
                if info_child.tag_name().name() == "category" {
                    if let Some(fmt) = info_child.attribute("citation-format") {
                        citation_format = Some(fmt.to_string());
                    }
                }
            }
        }
    }

    Ok(StyleInfo { citation_format })
}

fn print_parent_rankings(stats: &ParentRankerStats) {
    println!("=== Parent Style Rankings ===\n");

    if let Some(ref filter) = stats.format_filter {
        println!("Filter: citation-format = {}\n", filter);
    }

    println!("Dependent styles analyzed: {}", stats.total_dependent);
    println!("Independent styles found: {}", stats.total_independent);
    println!(
        "Unique parent styles referenced: {}",
        stats.parent_rankings.len()
    );
    println!();

    // Format distribution
    if !stats.format_distribution.is_empty() && stats.format_filter.is_none() {
        println!("=== Citation Format Distribution ===\n");
        let mut formats: Vec<_> = stats.format_distribution.iter().collect();
        formats.sort_by(|a, b| b.1.cmp(a.1));
        for (format, count) in formats {
            println!("  {:20} {:5}", format, count);
        }
        println!();
    }

    println!("=== Top Parent Styles by Usage ===\n");
    println!(
        "{:4}  {:40} {:>8}  {:>6}  {:15}",
        "Rank", "Parent Style", "Count", "%", "Format"
    );
    println!("{}", "-".repeat(80));

    for (i, ranking) in stats.parent_rankings.iter().take(50).enumerate() {
        println!(
            "{:4}  {:40} {:>8}  {:>5.1}%  {:15}",
            i + 1,
            truncate(&ranking.short_name, 40),
            ranking.dependent_count,
            ranking.percentage,
            ranking.format.as_deref().unwrap_or("-")
        );
    }

    if stats.parent_rankings.len() > 50 {
        println!(
            "\n... and {} more parent styles",
            stats.parent_rankings.len() - 50
        );
    }

    // Show top styles by format for prioritization
    println!("\n=== Priority Styles by Format ===\n");
    println!("These parent styles should be prioritized for rendering development:\n");

    for format in ["author-date", "numeric", "note"] {
        let top_for_format: Vec<_> = stats
            .parent_rankings
            .iter()
            .filter(|r| r.format.as_deref() == Some(format))
            .take(5)
            .collect();

        if !top_for_format.is_empty() {
            println!("  {} styles:", format);
            for r in top_for_format {
                println!("    - {} ({} dependents)", r.short_name, r.dependent_count);
            }
            println!();
        }
    }

    if !stats.parse_errors.is_empty() {
        println!("=== Parse Errors ===\n");
        for (i, err) in stats.parse_errors.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, err);
        }
        if stats.parse_errors.len() > 5 {
            println!("  ... and {} more", stats.parse_errors.len() - 5);
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ============================================================================
// Original Style Analyzer (unchanged)
// ============================================================================

fn run_style_analyzer(styles_dir: &str, json_output: bool) {
    let mut stats = StyleStats::default();

    // Walk directory and analyze each .csl file
    for entry in WalkDir::new(styles_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "csl")
                .unwrap_or(false)
        })
    {
        if let Err(e) = analyze_style(entry.path(), &mut stats) {
            stats
                .parse_errors
                .push(format!("{}: {}", entry.path().display(), e));
        }
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&stats).unwrap());
    } else {
        print_stats(&stats);
    }
}

#[derive(Default, serde::Serialize)]
struct StyleStats {
    total_styles: u32,
    parse_errors: Vec<String>,

    // Style-level attributes
    style_class: Counter,
    initialize_with: Counter,
    names_delimiter: Counter,
    name_as_sort_order: Counter,
    delimiter_precedes_last: Counter,
    and_option: Counter,
    demote_non_dropping_particle: Counter,
    page_range_format: Counter,

    // Citation attributes
    disambiguate_add_year_suffix: Counter,
    disambiguate_add_givenname: Counter,
    givenname_disambiguation_rule: Counter,
    citation_et_al_min: Counter,
    citation_et_al_use_first: Counter,

    // Bibliography attributes
    subsequent_author_substitute: Counter,
    bib_et_al_min: Counter,

    // Condition patterns (in choose blocks)
    condition_type: Counter,
    condition_variable: Counter,
    condition_is_numeric: Counter,
    condition_is_uncertain_date: Counter,
    condition_locator: Counter,
    condition_position: Counter,

    // Element usage
    element_names: Counter,
    element_date: Counter,
    element_text: Counter,
    element_number: Counter,
    element_label: Counter,
    element_group: Counter,
    element_choose: Counter,

    // Name element options
    name_form: Counter,
    name_initialize: Counter,
    name_initialize_with: Counter,

    // Date element options
    date_form: Counter,
    date_parts: Counter,

    // Unhandled attributes (for gap analysis)
    unhandled_style_attrs: Counter,
    unhandled_name_attrs: Counter,
}

type Counter = HashMap<String, u32>;

fn analyze_style(path: &Path, stats: &mut StyleStats) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read error: {}", e))?;

    let doc = roxmltree::Document::parse(&content).map_err(|e| format!("parse error: {}", e))?;

    let root = doc.root_element();

    stats.total_styles += 1;

    // Analyze style-level attributes
    analyze_style_attrs(&root, stats);

    // Walk all nodes and collect statistics
    analyze_nodes(&root, stats);

    Ok(())
}

fn analyze_style_attrs(node: &roxmltree::Node, stats: &mut StyleStats) {
    // Core attributes
    if let Some(v) = node.attribute("class") {
        *stats.style_class.entry(v.to_string()).or_insert(0) += 1;
    }

    // Name formatting
    if let Some(v) = node.attribute("initialize-with") {
        *stats.initialize_with.entry(format!("{:?}", v)).or_insert(0) += 1;
    }
    if let Some(v) = node.attribute("names-delimiter") {
        *stats.names_delimiter.entry(format!("{:?}", v)).or_insert(0) += 1;
    }
    if let Some(v) = node.attribute("name-as-sort-order") {
        *stats.name_as_sort_order.entry(v.to_string()).or_insert(0) += 1;
    }
    if let Some(v) = node.attribute("delimiter-precedes-last") {
        *stats
            .delimiter_precedes_last
            .entry(v.to_string())
            .or_insert(0) += 1;
    }
    if let Some(v) = node.attribute("and") {
        *stats.and_option.entry(v.to_string()).or_insert(0) += 1;
    }
    if let Some(v) = node.attribute("demote-non-dropping-particle") {
        *stats
            .demote_non_dropping_particle
            .entry(v.to_string())
            .or_insert(0) += 1;
    }
    if let Some(v) = node.attribute("page-range-format") {
        *stats.page_range_format.entry(v.to_string()).or_insert(0) += 1;
    }

    // Check for unhandled style-level attributes
    let known_attrs = [
        "xmlns",
        "version",
        "class",
        "default-locale",
        "initialize-with",
        "names-delimiter",
        "name-as-sort-order",
        "delimiter-precedes-last",
        "and",
        "demote-non-dropping-particle",
        "page-range-format",
        "sort-separator",
        "name-delimiter",
    ];
    for attr in node.attributes() {
        if !known_attrs.contains(&attr.name()) {
            *stats
                .unhandled_style_attrs
                .entry(attr.name().to_string())
                .or_insert(0) += 1;
        }
    }
}

fn analyze_nodes(node: &roxmltree::Node, stats: &mut StyleStats) {
    let tag = node.tag_name().name();

    match tag {
        "citation" => {
            if let Some(v) = node.attribute("disambiguate-add-year-suffix") {
                *stats
                    .disambiguate_add_year_suffix
                    .entry(v.to_string())
                    .or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("disambiguate-add-givenname") {
                *stats
                    .disambiguate_add_givenname
                    .entry(v.to_string())
                    .or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("givenname-disambiguation-rule") {
                *stats
                    .givenname_disambiguation_rule
                    .entry(v.to_string())
                    .or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("et-al-min") {
                *stats.citation_et_al_min.entry(v.to_string()).or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("et-al-use-first") {
                *stats
                    .citation_et_al_use_first
                    .entry(v.to_string())
                    .or_insert(0) += 1;
            }
        }
        "bibliography" => {
            if let Some(v) = node.attribute("subsequent-author-substitute") {
                *stats
                    .subsequent_author_substitute
                    .entry(format!("{:?}", v))
                    .or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("et-al-min") {
                *stats.bib_et_al_min.entry(v.to_string()).or_insert(0) += 1;
            }
        }
        "if" | "else-if" => {
            // Analyze condition patterns
            if let Some(v) = node.attribute("type") {
                for t in v.split_whitespace() {
                    *stats.condition_type.entry(t.to_string()).or_insert(0) += 1;
                }
            }
            if let Some(v) = node.attribute("variable") {
                for t in v.split_whitespace() {
                    *stats.condition_variable.entry(t.to_string()).or_insert(0) += 1;
                }
            }
            if let Some(v) = node.attribute("is-numeric") {
                for t in v.split_whitespace() {
                    *stats.condition_is_numeric.entry(t.to_string()).or_insert(0) += 1;
                }
            }
            if let Some(v) = node.attribute("is-uncertain-date") {
                for t in v.split_whitespace() {
                    *stats
                        .condition_is_uncertain_date
                        .entry(t.to_string())
                        .or_insert(0) += 1;
                }
            }
            if let Some(v) = node.attribute("locator") {
                for t in v.split_whitespace() {
                    *stats.condition_locator.entry(t.to_string()).or_insert(0) += 1;
                }
            }
            if let Some(v) = node.attribute("position") {
                for t in v.split_whitespace() {
                    *stats.condition_position.entry(t.to_string()).or_insert(0) += 1;
                }
            }
        }
        "names" => {
            *stats.element_names.entry("count".to_string()).or_insert(0) += 1;
        }
        "name" => {
            if let Some(v) = node.attribute("form") {
                *stats.name_form.entry(v.to_string()).or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("initialize") {
                *stats.name_initialize.entry(v.to_string()).or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("initialize-with") {
                *stats
                    .name_initialize_with
                    .entry(format!("{:?}", v))
                    .or_insert(0) += 1;
            }

            // Check for unhandled name attributes
            let known = [
                "form",
                "initialize",
                "initialize-with",
                "initialize-with-hyphen",
                "and",
                "delimiter",
                "delimiter-precedes-last",
                "delimiter-precedes-et-al",
                "et-al-min",
                "et-al-use-first",
                "et-al-subsequent-min",
                "et-al-subsequent-use-first",
                "name-as-sort-order",
                "sort-separator",
                "prefix",
                "suffix",
                "font-variant",
                "font-style",
                "font-weight",
                "text-decoration",
                "vertical-align",
            ];
            for attr in node.attributes() {
                if !known.contains(&attr.name()) {
                    *stats
                        .unhandled_name_attrs
                        .entry(attr.name().to_string())
                        .or_insert(0) += 1;
                }
            }
        }
        "date" => {
            *stats.element_date.entry("count".to_string()).or_insert(0) += 1;
            if let Some(v) = node.attribute("form") {
                *stats.date_form.entry(v.to_string()).or_insert(0) += 1;
            }
            if let Some(v) = node.attribute("date-parts") {
                *stats.date_parts.entry(v.to_string()).or_insert(0) += 1;
            }
        }
        "text" => {
            *stats.element_text.entry("count".to_string()).or_insert(0) += 1;
        }
        "number" => {
            *stats.element_number.entry("count".to_string()).or_insert(0) += 1;
        }
        "label" => {
            *stats.element_label.entry("count".to_string()).or_insert(0) += 1;
        }
        "group" => {
            *stats.element_group.entry("count".to_string()).or_insert(0) += 1;
        }
        "choose" => {
            *stats.element_choose.entry("count".to_string()).or_insert(0) += 1;
        }
        _ => {}
    }

    // Recurse into children
    for child in node.children() {
        if child.is_element() {
            analyze_nodes(&child, stats);
        }
    }
}

fn print_stats(stats: &StyleStats) {
    println!("=== CSL Style Analysis ===\n");
    println!("Total styles analyzed: {}", stats.total_styles);
    println!("Parse errors: {}\n", stats.parse_errors.len());

    println!("=== Style-Level Attributes ===\n");
    print_counter("class", &stats.style_class);
    print_counter("initialize-with", &stats.initialize_with);
    print_counter("names-delimiter", &stats.names_delimiter);
    print_counter("name-as-sort-order", &stats.name_as_sort_order);
    print_counter("delimiter-precedes-last", &stats.delimiter_precedes_last);
    print_counter("and", &stats.and_option);
    print_counter(
        "demote-non-dropping-particle",
        &stats.demote_non_dropping_particle,
    );
    print_counter("page-range-format", &stats.page_range_format);

    println!("\n=== Citation Attributes ===\n");
    print_counter(
        "disambiguate-add-year-suffix",
        &stats.disambiguate_add_year_suffix,
    );
    print_counter(
        "disambiguate-add-givenname",
        &stats.disambiguate_add_givenname,
    );
    print_counter(
        "givenname-disambiguation-rule",
        &stats.givenname_disambiguation_rule,
    );
    print_counter("et-al-min (citation)", &stats.citation_et_al_min);

    println!("\n=== Bibliography Attributes ===\n");
    print_counter(
        "subsequent-author-substitute",
        &stats.subsequent_author_substitute,
    );
    print_counter("et-al-min (bibliography)", &stats.bib_et_al_min);

    println!("\n=== Condition Patterns ===\n");
    print_counter("type conditions", &stats.condition_type);
    print_counter("variable conditions", &stats.condition_variable);
    print_counter("is-numeric conditions", &stats.condition_is_numeric);
    print_counter(
        "is-uncertain-date conditions",
        &stats.condition_is_uncertain_date,
    );
    print_counter("position conditions", &stats.condition_position);

    println!("\n=== Name Element Options ===\n");
    print_counter("name form", &stats.name_form);
    print_counter("name initialize", &stats.name_initialize);
    print_counter("name initialize-with", &stats.name_initialize_with);

    println!("\n=== Date Element Options ===\n");
    print_counter("date form", &stats.date_form);
    print_counter("date-parts", &stats.date_parts);

    println!("\n=== Element Usage ===\n");
    println!(
        "  names:  {}",
        stats.element_names.get("count").unwrap_or(&0)
    );
    println!(
        "  date:   {}",
        stats.element_date.get("count").unwrap_or(&0)
    );
    println!(
        "  text:   {}",
        stats.element_text.get("count").unwrap_or(&0)
    );
    println!(
        "  number: {}",
        stats.element_number.get("count").unwrap_or(&0)
    );
    println!(
        "  label:  {}",
        stats.element_label.get("count").unwrap_or(&0)
    );
    println!(
        "  group:  {}",
        stats.element_group.get("count").unwrap_or(&0)
    );
    println!(
        "  choose: {}",
        stats.element_choose.get("count").unwrap_or(&0)
    );

    if !stats.unhandled_style_attrs.is_empty() {
        println!("\n=== Unhandled Style Attributes (Gap Analysis) ===\n");
        print_counter("style-level", &stats.unhandled_style_attrs);
    }

    if !stats.unhandled_name_attrs.is_empty() {
        println!("\n=== Unhandled Name Attributes ===\n");
        print_counter("name element", &stats.unhandled_name_attrs);
    }

    if !stats.parse_errors.is_empty() {
        println!("\n=== Parse Errors ===\n");
        for (i, err) in stats.parse_errors.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, err);
        }
        if stats.parse_errors.len() > 10 {
            println!("  ... and {} more", stats.parse_errors.len() - 10);
        }
    }
}

fn print_counter(name: &str, counter: &Counter) {
    if counter.is_empty() {
        return;
    }

    let total: u32 = counter.values().sum();
    println!("{}: {} occurrences", name, total);

    // Sort by count descending
    let mut items: Vec<_> = counter.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1));

    for (value, count) in items.iter().take(8) {
        let pct = (**count as f64 / total as f64) * 100.0;
        println!("  {:40} {:5} ({:.1}%)", value, count, pct);
    }
    if items.len() > 8 {
        println!("  ... and {} more values", items.len() - 8);
    }
    println!();
}
