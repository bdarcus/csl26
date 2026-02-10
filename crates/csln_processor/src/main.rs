/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSLN Processor CLI
//!
//! Renders citations and bibliographies using CSLN styles.

use clap::{Parser, ValueEnum};
use csl_legacy::csl_json::Reference as LegacyReference;
use csln_core::reference::InputReference;
use csln_core::{InputBibliography, Locale, Style};
use csln_processor::render::djot::Djot;
use csln_processor::render::html::Html;
use csln_processor::render::plain::PlainText;
use csln_processor::{Bibliography, Citation, CitationItem, Processor, Reference};
use std::collections::HashMap;
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

    /// Path to the references file (CSLN YAML/JSON or CSL-JSON)
    #[arg(short, long)]
    references: Option<PathBuf>,

    /// Specific citation keys to render (comma-separated)
    #[arg(short, long, value_delimiter = ',')]
    keys: Option<Vec<String>>,

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

    // Load bibliography
    let bibliography = if let Some(ref path) = args.references {
        load_bibliography(path)
    } else {
        create_test_bibliography()
    };

    // Determine citation keys
    let item_ids = if let Some(ref keys) = args.keys {
        keys.clone()
    } else {
        bibliography.keys().cloned().collect()
    };

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
        print_json(&processor, &style_name, args.cite, args.bib, &item_ids);
    } else {
        match args.format {
            Format::Plain => {
                print_human::<PlainText>(&processor, &style_name, args.cite, args.bib, &item_ids);
            }
            Format::Html => {
                print_human::<Html>(&processor, &style_name, args.cite, args.bib, &item_ids);
            }
            Format::Djot => {
                print_human::<Djot>(&processor, &style_name, args.cite, args.bib, &item_ids);
            }
        }
    }
}

fn load_bibliography(path: &Path) -> Bibliography {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error reading references file: {}", e);
        std::process::exit(1);
    });

    let mut bib = indexmap::IndexMap::new();

    // 1. Try parsing as CSLN InputBibliography (YAML/JSON)
    if let Ok(input_bib) = serde_yaml::from_str::<InputBibliography>(&content) {
        for r in input_bib.references {
            if let Some(id) = r.id() {
                bib.insert(id.to_string(), r);
            }
        }
        return bib;
    }

    // 2. Try parsing as HashMap<String, InputReference> (YAML/JSON)
    // This is common for YAML bib files where keys are IDs.
    if let Ok(map) = serde_yaml::from_str::<HashMap<String, InputReference>>(&content) {
        for (key, mut r) in map {
            if r.id().is_none() {
                r.set_id(key.clone());
            }
            bib.insert(key, r);
        }
        return bib;
    }

    // 3. Try parsing as Vec<InputReference> (YAML/JSON)
    if let Ok(refs) = serde_yaml::from_str::<Vec<InputReference>>(&content) {
        for r in refs {
            if let Some(id) = r.id() {
                bib.insert(id.to_string(), r);
            }
        }
        return bib;
    }

    // 4. Fallback: Try parsing as Legacy CSL-JSON
    if let Ok(legacy_bib) = serde_json::from_str::<Vec<LegacyReference>>(&content) {
        for ref_item in legacy_bib {
            bib.insert(ref_item.id.clone(), Reference::from(ref_item));
        }
        return bib;
    }

    eprintln!("Error parsing references: could not parse as CSLN (YAML/JSON) or CSL-JSON");
    std::process::exit(1);
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

    // Use JSON-to-Reference to avoid construction verbosity
    // Values match CSLN schema structs (Monograph, SerialComponent, etc.)
    let items = vec![
        // ITEM-1: Kuhn journal article (SerialComponent)
        serde_json::json!({
            "type": "article",
            "id": "ITEM-1",
            "author": [{"family": "Kuhn", "given": "Thomas S."}],
            "title": "The Structure of Scientific Revolutions",
            "issued": "1962",
            "volume": "2",
            "issue": "2",
            "pages": "1-210",
            "parent": {
                "type": "academic-journal",
                "title": "International Encyclopedia of Unified Science"
            }
        }),
        // ITEM-2: Hawking book (Monograph)
        serde_json::json!({
            "type": "book",
            "id": "ITEM-2",
            "author": [{"family": "Hawking", "given": "Stephen"}],
            "title": "A Brief History of Time",
            "issued": "1988",
            "publisher": {"name": "Bantam Dell Publishing Group", "location": "New York"}
        }),
        // ITEM-3: Multi-author (SerialComponent)
        serde_json::json!({
            "type": "article",
            "id": "ITEM-3",
            "author": [
                {"family": "Doe", "given": "John"},
                {"family": "Smith", "given": "Jane"},
                {"family": "Public", "given": "Joe"}
            ],
            "title": "Collaboration in Science",
            "issued": "2020",
            "volume": "42",
            "pages": "123-145",
            "parent": {
                "type": "academic-journal",
                "title": "Journal of Important Things"
            }
        }),
        // ITEM-4: Many authors (Realistic: ATLAS Higgs boson paper)
        serde_json::json!({
            "type": "article",
            "id": "ITEM-4",
            "author": [
                {"family": "Aad", "given": "G."}, {"family": "Abajyan", "given": "T."},
                {"family": "Abbott", "given": "B."}, {"family": "Abdallah", "given": "J."},
                {"family": "Abdel Khalek", "given": "S."}, {"family": "Abdelalim", "given": "A. A."},
                {"family": "Abdesselam", "given": "A."}, {"family": "Abdinov", "given": "O."},
                {"family": "Abi", "given": "B."}, {"family": "Abolins", "given": "M."},
                {"family": "AbouZeid", "given": "O. S."}, {"family": "Abramowicz", "given": "H."},
                {"family": "Abreu", "given": "H."}, {"family": "Abulaiti", "given": "Y."},
                {"family": "Acharya", "given": "B. S."}, {"family": "Adamczyk", "given": "L."},
                {"family": "Adams", "given": "D. L."}, {"family": "Addy", "given": "T. N."},
                {"family": "Adelman", "given": "J."}, {"family": "Adomeit", "given": "S."},
                {"family": "Adye", "given": "T."}, {"family": "Agatonovic-Jovin", "given": "T."},
                {"family": "Aguilar-Saavedra", "given": "J. A."}, {"family": "Agustoni", "given": "M."},
                {"family": "Ahlen", "given": "S. P."}, {"family": "Ahmadov", "given": "F."},
                {"family": "Aielli", "given": "G."}, {"family": "Åkesson", "given": "T. P. A."}
            ],
            "title": "Observation of a new particle in the search for the Standard Model Higgs boson with the ATLAS detector at the LHC",
            "issued": "2012-09-17",
            "volume": "716",
            "issue": "1",
            "pages": "1-29",
            "parent": {
                "type": "academic-journal",
                "title": "Physics Letters B"
            },
            "doi": "10.1016/j.physletb.2012.08.020"
        }),
        // ITEM-5: Edited Book (Collection)
        serde_json::json!({
            "type": "edited-book",
            "id": "ITEM-5",
            "editor": [{"family": "Editor", "given": "Edward"}],
            "title": "The Edited Volume",
            "issued": "2015",
            "publisher": {"name": "Academic Press"}
        }),
        // ITEM-6: Chapter in Edited Book (CollectionComponent)
        serde_json::json!({
            "type": "chapter",
            "id": "ITEM-6",
            "author": [{"family": "Contributor", "given": "Charles"}],
            "title": "My Special Chapter",
            "issued": "2015",
            "pages": "45-67",
            "parent": {
                "type": "edited-book",
                "title": "The Edited Volume",
                "editor": [{"family": "Editor", "given": "Edward"}],
                "publisher": {"name": "Academic Press"},
                "issued": "2015"
            }
        }),
        // ITEM-7: Webpage (Monograph)
        serde_json::json!({
            "type": "webpage",
            "id": "ITEM-7",
            "title": "How to Cite Everything",
            "issued": "2023",
            "url": "https://example.com/how-to-cite",
            "publisher": {"name": "Citation Guides Online"}
        }),
        // ITEM-8: Organization Report (Monograph)
        serde_json::json!({
            "type": "report",
            "id": "ITEM-8",
            "author": {"name": "World Health Organization"},
            "title": "Global Health Report",
            "issued": "2022",
            "publisher": {"name": "WHO", "location": "Geneva"}
        }),
        // ITEM-9: Thesis (Monograph)
        serde_json::json!({
            "type": "thesis",
            "id": "ITEM-9",
            "author": [{"family": "Student", "given": "Sarah"}],
            "title": "Deep Learning for Citations",
            "genre": "PhD dissertation",
            "issued": "2021",
            "publisher": {"name": "University of Rust"}
        }),
        // ITEM-10: No Date (Monograph)
        serde_json::json!({
            "type": "book",
            "id": "ITEM-10",
            "author": [{"family": "Ancient", "given": "Aristotle"}],
            "title": "Poetics",
            "issued": ""
        }),
    ];

    for item in items {
        let id = item["id"].as_str().unwrap_or("unknown").to_string();
        let reference: Reference = serde_json::from_value(item).unwrap_or_else(|e| {
            panic!("Error parsing test item {}: {}", id, e);
        });
        if let Some(id) = reference.id() {
            bib.insert(id.to_string(), reference);
        }
    }

    bib
}

fn print_human<F>(
    processor: &Processor,
    style_name: &str,
    show_cite: bool,
    show_bib: bool,
    item_ids: &[String],
) where
    F: csln_processor::render::format::OutputFormat<Output = String>,
{
    println!("\n=== {} ===\n", style_name);

    if show_cite {
        println!("CITATIONS (Non-Integral):");
        for id in item_ids {
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

        println!("CITATIONS (Integral):");
        for id in item_ids {
            let citation = Citation {
                id: Some(id.to_string()),
                items: vec![CitationItem {
                    id: id.to_string(),
                    ..Default::default()
                }],
                mode: csln_core::citation::CitationMode::Integral,
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

fn print_json(
    processor: &Processor,
    style_name: &str,
    show_cite: bool,
    show_bib: bool,
    item_ids: &[String],
) {
    use serde_json::json;

    let mut result = json!({
        "style": style_name,
        "items": item_ids.len()
    });

    if show_cite {
        let non_integral: Vec<_> = item_ids
            .iter()
            .map(|id| {
                let citation = Citation {
                    id: Some(id.to_string()),
                    items: vec![CitationItem {
                        id: id.to_string(),
                        ..Default::default()
                    }],
                    mode: csln_core::citation::CitationMode::NonIntegral,
                    ..Default::default()
                };
                json!({
                    "id": id,
                    "text": processor.process_citation(&citation).unwrap_or_else(|e| e.to_string())
                })
            })
            .collect();

        let integral: Vec<_> = item_ids
            .iter()
            .map(|id| {
                let citation = Citation {
                    id: Some(id.to_string()),
                    items: vec![CitationItem {
                        id: id.to_string(),
                        ..Default::default()
                    }],
                    mode: csln_core::citation::CitationMode::Integral,
                    ..Default::default()
                };
                json!({
                    "id": id,
                    "text": processor.process_citation(&citation).unwrap_or_else(|e| e.to_string())
                })
            })
            .collect();

        result["citations"] = json!({
            "non-integral": non_integral,
            "integral": integral
        });
    }

    if show_bib {
        let bib_text = processor.render_bibliography();
        let entries: Vec<_> = bib_text
            .split("\n\n")
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, entry)| {
                json!({
                    "id": item_ids.get(i).unwrap_or(&"unknown".to_string()),
                    "text": entry.trim()
                })
            })
            .collect();
        result["bibliography"] = json!({ "entries": entries });
    }

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
