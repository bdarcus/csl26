/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn run_style_analyzer(styles_dir: &str, json_output: bool) {
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
pub struct StyleStats {
    pub total_styles: u32,
    pub parse_errors: Vec<String>,

    // Style-level attributes
    pub style_class: Counter,
    pub initialize_with: Counter,
    pub names_delimiter: Counter,
    pub name_as_sort_order: Counter,
    pub delimiter_precedes_last: Counter,
    pub and_option: Counter,
    pub demote_non_dropping_particle: Counter,
    pub page_range_format: Counter,

    // Citation attributes
    pub disambiguate_add_year_suffix: Counter,
    pub disambiguate_add_givenname: Counter,
    pub givenname_disambiguation_rule: Counter,
    pub citation_et_al_min: Counter,
    pub citation_et_al_use_first: Counter,

    // Bibliography attributes
    pub subsequent_author_substitute: Counter,
    pub bib_et_al_min: Counter,

    // Condition patterns (in choose blocks)
    pub condition_type: Counter,
    pub condition_variable: Counter,
    pub condition_is_numeric: Counter,
    pub condition_is_uncertain_date: Counter,
    pub condition_locator: Counter,
    pub condition_position: Counter,

    // Element usage
    pub element_names: Counter,
    pub element_date: Counter,
    pub element_text: Counter,
    pub element_number: Counter,
    pub element_label: Counter,
    pub element_group: Counter,
    pub element_choose: Counter,

    // Name element options
    pub name_form: Counter,
    pub name_initialize: Counter,
    pub name_initialize_with: Counter,

    // Date element options
    pub date_form: Counter,
    pub date_parts: Counter,

    // Unhandled attributes (for gap analysis)
    pub unhandled_style_attrs: Counter,
    pub unhandled_name_attrs: Counter,
}

pub type Counter = HashMap<String, u32>;

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
    println!(
        "=== CSL Style Analysis ===
"
    );
    println!("Total styles analyzed: {}", stats.total_styles);
    println!(
        "Parse errors: {}
",
        stats.parse_errors.len()
    );

    println!(
        "=== Style-Level Attributes ===
"
    );
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

    println!(
        "
=== Citation Attributes ===
"
    );
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

    println!(
        "
=== Bibliography Attributes ===
"
    );
    print_counter(
        "subsequent-author-substitute",
        &stats.subsequent_author_substitute,
    );
    print_counter("et-al-min (bibliography)", &stats.bib_et_al_min);

    println!(
        "
=== Condition Patterns ===
"
    );
    print_counter("type conditions", &stats.condition_type);
    print_counter("variable conditions", &stats.condition_variable);
    print_counter("is-numeric conditions", &stats.condition_is_numeric);
    print_counter(
        "is-uncertain-date conditions",
        &stats.condition_is_uncertain_date,
    );
    print_counter("position conditions", &stats.condition_position);

    println!(
        "
=== Name Element Options ===
"
    );
    print_counter("name form", &stats.name_form);
    print_counter("name initialize", &stats.name_initialize);
    print_counter("name initialize-with", &stats.name_initialize_with);

    println!(
        "
=== Date Element Options ===
"
    );
    print_counter("date form", &stats.date_form);
    print_counter("date-parts", &stats.date_parts);

    println!(
        "
=== Element Usage ===
"
    );
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
        println!(
            "
=== Unhandled Style Attributes (Gap Analysis) ===
"
        );
        print_counter("style-level", &stats.unhandled_style_attrs);
    }

    if !stats.unhandled_name_attrs.is_empty() {
        println!(
            "
=== Unhandled Name Attributes ===
"
        );
        print_counter("name element", &stats.unhandled_name_attrs);
    }

    if !stats.parse_errors.is_empty() {
        println!(
            "
=== Parse Errors ===
"
        );
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
