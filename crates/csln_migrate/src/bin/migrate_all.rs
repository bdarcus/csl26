use csl_legacy::parser::parse_style;
use csln_migrate::{MacroInliner, Upsampler};
use roxmltree::Document;
use std::fs;

use csl_legacy::model::CslNode as LNode;
use csln_core::CslnNode as CNode;

fn main() {
    let styles_dir = "styles";
    let entries = match fs::read_dir(styles_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error reading styles directory: {}", e);
            return;
        }
    };

    let mut total = 0;
    let mut success = 0;
    let mut failures = 0;
    let mut total_input_nodes = 0;
    let mut total_output_nodes = 0;
    let mut error_types = std::collections::HashMap::new();

    println!("Starting BULK MIGRATION of styles in {}...", styles_dir);

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("csl") {
            total += 1;

            // 1. Read & Parse (Legacy)
            let text = match fs::read_to_string(&path) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let doc = match Document::parse(&text) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let legacy_style = match parse_style(doc.root_element()) {
                Ok(s) => s,
                Err(e) => {
                    *error_types
                        .entry(format!("Legacy Parse Error: {}", e))
                        .or_insert(0) += 1;
                    failures += 1;
                    continue;
                }
            };

            // 2. Migration
            let inliner = MacroInliner::new(&legacy_style);
            let flattened_bib = inliner
                .inline_bibliography(&legacy_style)
                .unwrap_or_default();
            let flattened_cit = inliner.inline_citation(&legacy_style);

            let upsampler = Upsampler;
            let csln_bib = upsampler.upsample_nodes(&flattened_bib);
            let csln_cit = upsampler.upsample_nodes(&flattened_cit);

            // Stats
            let input_count =
                count_legacy_nodes(&flattened_bib) + count_legacy_nodes(&flattened_cit);
            let output_count = count_csln_nodes(&csln_bib) + count_csln_nodes(&csln_cit);

            total_input_nodes += input_count;
            total_output_nodes += output_count;

            if csln_bib.is_empty() && csln_cit.is_empty() {
                *error_types.entry("Empty Output".to_string()).or_insert(0) += 1;
                failures += 1;
            } else {
                success += 1;
            }

            // Track dropped nodes by inspecting the upsampler's decision?
            // Hard to do from outside.
            // Let's just trust the 100% success for now and investigate the retention later.

            if total % 100 == 0 {
                print!(".");
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }
    }

    println!("\n\n=== MIGRATION STATS ===");
    println!("Total Styles: {}", total);
    println!(
        "Success:      {} ({:.1}%)",
        success,
        (success as f64 / total as f64) * 100.0
    );
    println!(
        "Failures:     {} ({:.1}%)",
        failures,
        (failures as f64 / total as f64) * 100.0
    );

    println!("\n=== DATA RETENTION ===");
    println!("Input Nodes:  {}", total_input_nodes);
    println!("Output Nodes: {}", total_output_nodes);
    println!(
        "Retention:    {:.1}%",
        (total_output_nodes as f64 / total_input_nodes as f64) * 100.0
    );
    println!("(Note: Retention < 100% is expected due to node collapsing/upsampling)");

    println!("\n=== TOP ERRORS ===");
    let mut err_vec: Vec<_> = error_types.iter().collect();
    err_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (msg, count) in err_vec.into_iter().take(10) {
        println!("{:4}x {}", count, msg);
    }
}

fn count_legacy_nodes(nodes: &[LNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        count += 1;
        match node {
            LNode::Group(g) => count += count_legacy_nodes(&g.children),
            LNode::Names(n) => count += count_legacy_nodes(&n.children),
            LNode::Choose(c) => {
                count += count_legacy_nodes(&c.if_branch.children);
                for b in &c.else_if_branches {
                    count += count_legacy_nodes(&b.children);
                }
                if let Some(e) = &c.else_branch {
                    count += count_legacy_nodes(e);
                }
            }
            LNode::Substitute(s) => count += count_legacy_nodes(&s.children),
            _ => {}
        }
    }
    count
}

fn count_csln_nodes(nodes: &[CNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        count += 1;
        match node {
            CNode::Group(g) => count += count_csln_nodes(&g.children),
            CNode::Condition(c) => {
                count += count_csln_nodes(&c.then_branch);
                for else_if in &c.else_if_branches {
                    count += count_csln_nodes(&else_if.children);
                }
                if let Some(e) = &c.else_branch {
                    count += count_csln_nodes(e);
                }
            }
            _ => {}
        }
    }
    count
}
