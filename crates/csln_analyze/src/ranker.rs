/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Statistics for parent style ranking.
#[derive(Default, serde::Serialize)]
pub struct ParentRankerStats {
    /// Total dependent styles analyzed
    pub total_dependent: u32,
    /// Total independent (parent) styles found
    pub total_independent: u32,
    /// Parse errors encountered
    pub parse_errors: Vec<String>,
    /// Filter applied (if any)
    pub format_filter: Option<String>,
    /// Parent styles ranked by dependent count
    pub parent_rankings: Vec<ParentRanking>,
    /// Citation format distribution
    pub format_distribution: HashMap<String, u32>,
}

/// A parent style and its usage statistics.
#[derive(serde::Serialize, Clone)]
pub struct ParentRanking {
    /// Parent style ID (usually a Zotero URL)
    pub parent_id: String,
    /// Extracted short name from the ID
    pub short_name: String,
    /// Number of dependent styles that reference this parent
    pub dependent_count: u32,
    /// Percentage of all dependents (for the filtered set)
    pub percentage: f64,
    /// Citation format (author-date, numeric, note, label)
    pub format: Option<String>,
    /// Fields/disciplines that use this parent
    pub fields: Vec<String>,
}

pub fn run_parent_ranker(styles_dir: &str, json_output: bool, format_filter: Option<&str>) {
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
        eprintln!("Dependent styles are typically in styles-legacy/dependent/");
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
    println!(
        "=== Parent Style Rankings ===
"
    );

    if let Some(ref filter) = stats.format_filter {
        println!(
            "Filter: citation-format = {}
",
            filter
        );
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
        println!(
            "=== Citation Format Distribution ===
"
        );
        let mut formats: Vec<_> = stats.format_distribution.iter().collect();
        formats.sort_by(|a, b| b.1.cmp(a.1));
        for (format, count) in formats {
            println!("  {:20} {:5}", format, count);
        }
        println!();
    }

    println!(
        "=== Top Parent Styles by Usage ===
"
    );
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
            "
... and {} more parent styles",
            stats.parent_rankings.len() - 50
        );
    }

    // Show top styles by format for prioritization
    println!(
        "
=== Priority Styles by Format ===
"
    );
    println!(
        "These parent styles should be prioritized for rendering development:
"
    );

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
        println!(
            "=== Parse Errors ===
"
        );
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
