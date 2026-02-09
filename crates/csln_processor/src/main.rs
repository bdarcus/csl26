/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSLN Processor CLI
//!
//! Renders citations and bibliographies using CSLN styles.

use clap::{Parser, ValueEnum};
use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference, StringOrNumber};
use csln_core::{Locale, Style};
use csln_processor::render::djot::Djot;
use csln_processor::render::html::Html;
use csln_processor::render::plain::PlainText;
use csln_processor::{Bibliography, Citation, CitationItem, Processor, Reference};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the style YAML file
    style: PathBuf,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Format::Plain)]
    format: Format,

    /// Show bibliography (default if neither --bib nor --cite is specified)
    #[arg(long)]
    bib: bool,

    /// Show citations
    #[arg(long)]
    cite: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Disable semantic classes (HTML spans, Djot attributes)
    #[arg(long)]
    no_semantics: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Format {
    Plain,
    Html,
    Djot,
}

fn main() {
    let mut args = Args::parse();

    // Default behavior: show both if neither is specified
    if !args.bib && !args.cite {
        args.bib = true;
        args.cite = true;
    }

    // Load style
    let style_content = match fs::read_to_string(&args.style) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading style: {}", e);
            std::process::exit(1);
        }
    };

    let mut style: Style = match serde_yaml::from_str(&style_content) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error parsing style: {}", e);
            std::process::exit(1);
        }
    };

    if args.no_semantics {
        if let Some(ref mut options) = style.options {
            options.semantic_classes = Some(false);
        } else {
            style.options = Some(csln_core::options::Config {
                semantic_classes: Some(false),
                ..Default::default()
            });
        }
    }

    // Create test bibliography
    let bibliography = create_test_bibliography();

    // Determine locales directory
    let locales_dir = find_locales_dir(args.style.to_str().unwrap_or("."));

    // Create processor with locale support
    let processor = if let Some(ref locale_id) = style.info.default_locale {
        let locale = Locale::load(locale_id, &locales_dir);
        Processor::with_locale(style, bibliography, locale)
    } else {
        Processor::new(style, bibliography)
    };

    let style_name = args
        .style
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if args.json {
        print_json(&processor, &style_name, args.cite, args.bib);
    } else {
        match args.format {
            Format::Plain => {
                print_human::<PlainText>(&processor, &style_name, args.cite, args.bib);
            }
            Format::Html => {
                print_human::<Html>(&processor, &style_name, args.cite, args.bib);
            }
            Format::Djot => {
                print_human::<Djot>(&processor, &style_name, args.cite, args.bib);
            }
        }
    }
}

fn find_locales_dir(style_path: &str) -> PathBuf {
    let style_dir = Path::new(style_path).parent().unwrap_or(Path::new("."));
    let candidates = [
        style_dir.join("locales"),
        style_dir.join("../locales"),
        style_dir.join("../../locales"),
        PathBuf::from("locales"),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            return candidate.clone();
        }
    }
    PathBuf::from(".")
}

fn create_test_bibliography() -> Bibliography {
    let mut bib = indexmap::IndexMap::new();

    // ITEM-1: Kuhn journal article
    bib.insert(
        "ITEM-1".to_string(),
        Reference::from(LegacyReference {
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
        }),
    );

    // ITEM-2: Hawking book
    bib.insert(
        "ITEM-2".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-2".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Hawking", "Stephen")]),
            title: Some("A Brief History of Time".to_string()),
            issued: Some(DateVariable::year(1988)),
            publisher: Some("Bantam Dell Publishing Group".to_string()),
            publisher_place: Some("New York".to_string()),
            ..Default::default()
        }),
    );

    bib
}

fn print_human<F>(processor: &Processor, style_name: &str, show_cite: bool, show_bib: bool)
where
    F: csln_processor::render::format::OutputFormat<Output = String>,
{
    println!("\n=== {} ===\n", style_name);

    let item_ids = ["ITEM-1", "ITEM-2"];

    if show_cite {
        println!("CITATIONS (Non-Integral):");
        for id in &item_ids {
            let citation = Citation {
                id: Some(id.to_string()),
                items: vec![CitationItem {
                    id: id.to_string(),
                    ..Default::default()
                }],
                mode: csln_core::citation::CitationMode::NonIntegral,
                ..Default::default()
            };
            match processor.process_citation_with_format::<F>(&citation) {
                Ok(text) => println!("  [{}] {}", id, text),
                Err(e) => println!("  [{}] ERROR: {}", id, e),
            }
        }
        println!();
    }

    if show_bib {
        println!("BIBLIOGRAPHY:");
        let bib_text = processor.render_bibliography_with_format::<F>();
        for line in bib_text.lines() {
            if !line.is_empty() {
                println!("  {}", line);
            }
        }
    }
}

fn print_json(processor: &Processor, style_name: &str, show_cite: bool, show_bib: bool) {
    use serde_json::json;

    let item_ids = ["ITEM-1", "ITEM-2"];

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
                    ..Default::default()
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
