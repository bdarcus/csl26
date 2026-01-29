/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSLN Processor CLI
//!
//! Renders citations and bibliographies using CSLN styles.
//!
//! Usage: csln_processor <style.yaml> [--bib] [--cite]

use csln_core::Style;
use csln_processor::{
    Bibliography, Citation, CitationItem, DateVariable, Name, Processor, Reference, StringOrNumber,
};
use std::collections::HashMap;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: csln_processor <style.yaml> [--bib] [--cite] [--json]");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  csln_processor csln-new.yaml");
        eprintln!("  csln_processor csln-new.yaml --cite");
        std::process::exit(1);
    }

    let style_path = &args[1];
    let show_bib = args.contains(&"--bib".to_string()) || !args.contains(&"--cite".to_string());
    let show_cite = args.contains(&"--cite".to_string()) || !args.contains(&"--bib".to_string());
    let json_output = args.contains(&"--json".to_string());

    // Load style
    let style_content = match fs::read_to_string(style_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading style: {}", e);
            std::process::exit(1);
        }
    };

    let style: Style = match serde_yaml::from_str(&style_content) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error parsing style: {}", e);
            std::process::exit(1);
        }
    };

    // Create test bibliography (same items as oracle.js)
    let bibliography = create_test_bibliography();

    // Create processor
    let processor = Processor::new(style, bibliography);

    let style_name = std::path::Path::new(style_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| style_path.clone());

    if json_output {
        print_json(&processor, &style_name, show_cite, show_bib);
    } else {
        print_human(&processor, &style_name, show_cite, show_bib);
    }
}

fn create_test_bibliography() -> Bibliography {
    let mut bib = HashMap::new();

    // ITEM-1: Kuhn journal article
    bib.insert(
        "ITEM-1".to_string(),
        Reference {
            id: "ITEM-1".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            container_title: Some("International Encyclopedia of Unified Science".to_string()),
            issued: Some(DateVariable::year(1962)),
            volume: Some(StringOrNumber::String("2".to_string())),
            issue: Some(StringOrNumber::String("2".to_string())),
            publisher: Some("University of Chicago Press".to_string()),
            publisher_place: Some("Chicago".to_string()),
            doi: Some("10.1234/example".to_string()),
            ..Default::default()
        },
    );

    // ITEM-2: Hawking book
    bib.insert(
        "ITEM-2".to_string(),
        Reference {
            id: "ITEM-2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Hawking", "Stephen")]),
            title: Some("A Brief History of Time".to_string()),
            issued: Some(DateVariable::year(1988)),
            publisher: Some("Bantam Dell Publishing Group".to_string()),
            publisher_place: Some("New York".to_string()),
            ..Default::default()
        },
    );

    // ITEM-3: LeCun et al. article (3 authors - triggers et al.)
    bib.insert(
        "ITEM-3".to_string(),
        Reference {
            id: "ITEM-3".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ]),
            title: Some("Deep Learning".to_string()),
            container_title: Some("Nature".to_string()),
            issued: Some(DateVariable::year(2015)),
            volume: Some(StringOrNumber::String("521".to_string())),
            page: Some("436-444".to_string()),
            doi: Some("10.1038/nature14539".to_string()),
            ..Default::default()
        },
    );

    // ITEM-4: Ericsson chapter
    bib.insert(
        "ITEM-4".to_string(),
        Reference {
            id: "ITEM-4".to_string(),
            ref_type: "chapter".to_string(),
            author: Some(vec![Name::new("Ericsson", "K. Anders")]),
            editor: Some(vec![
                Name::new("Ericsson", "K. Anders"),
                Name::new("Charness", "Neil"),
                Name::new("Feltovich", "Paul J."),
                Name::new("Hoffman", "Robert R."),
            ]),
            title: Some("The Role of Deliberate Practice".to_string()),
            collection_title: Some(
                "The Cambridge Handbook of Expertise and Expert Performance".to_string(),
            ),
            issued: Some(DateVariable::year(2006)),
            publisher: Some("Cambridge University Press".to_string()),
            page: Some("683-703".to_string()),
            ..Default::default()
        },
    );

    // ITEM-5: World Bank report (corporate author)
    bib.insert(
        "ITEM-5".to_string(),
        Reference {
            id: "ITEM-5".to_string(),
            ref_type: "report".to_string(),
            author: Some(vec![Name::literal("World Bank")]),
            title: Some("World Development Report 2023".to_string()),
            issued: Some(DateVariable::year(2023)),
            publisher: Some("World Bank Group".to_string()),
            publisher_place: Some("Washington, DC".to_string()),
            ..Default::default()
        },
    );

    bib
}

fn print_human(processor: &Processor, style_name: &str, show_cite: bool, show_bib: bool) {
    println!("\n=== {} ===\n", style_name);

    let item_ids = ["ITEM-1", "ITEM-2", "ITEM-3", "ITEM-4", "ITEM-5"];

    if show_cite {
        println!("CITATIONS:");
        for id in &item_ids {
            let citation = Citation {
                id: Some(id.to_string()),
                items: vec![CitationItem {
                    id: id.to_string(),
                    ..Default::default()
                }],
            };
            match processor.process_citation(&citation) {
                Ok(text) => println!("  [{}] {}", id, text),
                Err(e) => println!("  [{}] ERROR: {}", id, e),
            }
        }
        println!();
    }

    if show_bib {
        println!("BIBLIOGRAPHY:");
        let bib_text = processor.render_bibliography();
        for line in bib_text.lines() {
            if !line.is_empty() {
                println!("  {}", line);
            }
        }
    }
}

fn print_json(processor: &Processor, style_name: &str, show_cite: bool, show_bib: bool) {
    use serde_json::json;

    let item_ids = ["ITEM-1", "ITEM-2", "ITEM-3", "ITEM-4"];

    let mut result = json!({
        "style": style_name,
        "items": item_ids.len()
    });

    if show_cite {
        let citations: Vec<_> = item_ids
            .iter()
            .map(|id| {
                let citation = Citation {
                    id: Some(id.to_string()),
                    items: vec![CitationItem {
                        id: id.to_string(),
                        ..Default::default()
                    }],
                };
                json!({
                    "id": id,
                    "text": processor.process_citation(&citation).unwrap_or_else(|e| e.to_string())
                })
            })
            .collect();
        result["citations"] = json!(citations);
    }

    if show_bib {
        let bib_text = processor.render_bibliography();
        let entries: Vec<_> = bib_text
            .split("\n\n")
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, entry)| {
                json!({
                    "id": item_ids.get(i).unwrap_or(&"unknown"),
                    "text": entry.trim()
                })
            })
            .collect();
        result["bibliography"] = json!({ "entries": entries });
    }

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
