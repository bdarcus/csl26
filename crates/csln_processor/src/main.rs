/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! CSLN Processor CLI
//!
//! Renders citations and bibliographies using CSLN styles.
//!
//! Usage: csln_processor <style.yaml> [--bib] [--cite]

use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference, StringOrNumber};
use csln_core::{Locale, Style};
use csln_processor::{Bibliography, Citation, CitationItem, Processor, Reference};
use std::env;
use std::fs;
use std::path::Path;

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

    // Determine locales directory - look relative to the style file, then cwd
    let locales_dir = find_locales_dir(style_path);

    // Create processor with locale support
    let processor = if let Some(ref locale_id) = style.info.default_locale {
        let locale = Locale::load(locale_id, &locales_dir);
        Processor::with_locale(style, bibliography, locale)
    } else {
        Processor::new(style, bibliography)
    };

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

/// Find the locales directory by looking in common locations.
fn find_locales_dir(style_path: &str) -> std::path::PathBuf {
    // 1. Try relative to the style file (../locales or ../../locales)
    let style_dir = Path::new(style_path).parent().unwrap_or(Path::new("."));
    let candidates = [
        style_dir.join("locales"),
        style_dir.join("../locales"),
        style_dir.join("../../locales"),
        Path::new("locales").to_path_buf(),
        Path::new("../locales").to_path_buf(),
    ];

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            return candidate.clone();
        }
    }

    // Default to current directory if no locales found
    Path::new(".").to_path_buf()
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

    // ITEM-3: LeCun et al. article (3 authors - triggers et al.)
    bib.insert(
        "ITEM-3".to_string(),
        Reference::from(LegacyReference {
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
        }),
    );

    // ITEM-4: Ericsson chapter
    bib.insert(
        "ITEM-4".to_string(),
        Reference::from(LegacyReference {
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
            // Note: container-title is used for the book title in CSL-JSON for chapters
            container_title: Some(
                "The Cambridge Handbook of Expertise and Expert Performance".to_string(),
            ),
            issued: Some(DateVariable::year(2006)),
            publisher: Some("Cambridge University Press".to_string()),
            page: Some("683-703".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-5: World Bank report (corporate author)
    bib.insert(
        "ITEM-5".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-5".to_string(),
            ref_type: "report".to_string(),
            author: Some(vec![Name::literal("World Bank")]),
            title: Some("World Development Report 2023".to_string()),
            issued: Some(DateVariable::year(2023)),
            publisher: Some("World Bank Group".to_string()),
            publisher_place: Some("Washington, DC".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-6: Two-author book
    bib.insert(
        "ITEM-6".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-6".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![
                Name::new("Weinberg", "Gerald M."),
                Name::new("Freedman", "Daniel P."),
            ]),
            title: Some("The Psychology of Computer Programming".to_string()),
            issued: Some(DateVariable::year(1971)),
            publisher: Some("Van Nostrand Reinhold".to_string()),
            publisher_place: Some("New York".to_string()),
            edition: Some(StringOrNumber::String(
                "Silver Anniversary Edition".to_string(),
            )),
            ..Default::default()
        }),
    );

    // ITEM-7: 8-author article (tests et-al)
    bib.insert(
        "ITEM-7".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-7".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("Vaswani", "Ashish"),
                Name::new("Shazeer", "Noam"),
                Name::new("Parmar", "Niki"),
                Name::new("Uszkoreit", "Jakob"),
                Name::new("Jones", "Llion"),
                Name::new("Gomez", "Aidan N."),
                Name::new("Kaiser", "Lukasz"),
                Name::new("Polosukhin", "Illia"),
            ]),
            title: Some("Attention Is All You Need".to_string()),
            container_title: Some("Advances in Neural Information Processing Systems".to_string()),
            issued: Some(DateVariable::year(2017)),
            volume: Some(StringOrNumber::String("30".to_string())),
            page: Some("5998-6008".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-8: Kuhn 1970 (tests disambiguation with ITEM-1)
    bib.insert(
        "ITEM-8".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-8".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("Scientific Paradigms and Normal Science".to_string()),
            container_title: Some("Philosophy of Science".to_string()),
            issued: Some(DateVariable::year(1970)),
            volume: Some(StringOrNumber::String("37".to_string())),
            issue: Some(StringOrNumber::String("1".to_string())),
            page: Some("1-13".to_string()),
            doi: Some("10.1086/288273".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-9: Smith, John (tests disambiguation with ITEM-10)
    bib.insert(
        "ITEM-9".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-9".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("Smith", "John"),
                Name::new("Anderson", "Mary"),
            ]),
            title: Some("Climate Change and Extreme Weather Events".to_string()),
            container_title: Some("Nature Climate Change".to_string()),
            issued: Some(DateVariable::year(2020)),
            volume: Some(StringOrNumber::String("10".to_string())),
            page: Some("850-855".to_string()),
            doi: Some("10.1038/s41558-020-0871-4".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-10: Smith, Jane (tests disambiguation with ITEM-9)
    bib.insert(
        "ITEM-10".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-10".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("Smith", "Jane"),
                Name::new("Williams", "Robert"),
            ]),
            title: Some("Machine Learning for Climate Prediction".to_string()),
            container_title: Some("Environmental Research Letters".to_string()),
            issued: Some(DateVariable::year(2020)),
            volume: Some(StringOrNumber::String("15".to_string())),
            issue: Some(StringOrNumber::String("11".to_string())),
            page: Some("114042".to_string()),
            doi: Some("10.1088/1748-9326/abc123".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-11: Thesis
    bib.insert(
        "ITEM-11".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-11".to_string(),
            ref_type: "thesis".to_string(),
            author: Some(vec![Name::new("Chen", "Wei")]),
            title: Some("Neural Networks for Natural Language Understanding".to_string()),
            issued: Some(DateVariable::year(2019)),
            publisher: Some("Stanford University".to_string()),
            genre: Some("PhD thesis".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-12: Conference paper
    bib.insert(
        "ITEM-12".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-12".to_string(),
            ref_type: "paper-conference".to_string(),
            author: Some(vec![
                Name::new("Mikolov", "Tomas"),
                Name::new("Sutskever", "Ilya"),
                Name::new("Chen", "Kai"),
                Name::new("Corrado", "Greg"),
                Name::new("Dean", "Jeff"),
            ]),
            title: Some("Distributed Representations of Words and Phrases".to_string()),
            container_title: Some("Proceedings of NIPS 2013".to_string()),
            issued: Some(DateVariable::year(2013)),
            page: Some("3111-3119".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-13: Webpage
    bib.insert(
        "ITEM-13".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-13".to_string(),
            ref_type: "webpage".to_string(),
            author: Some(vec![Name::literal("State of JS Team")]),
            title: Some("The State of JavaScript 2023".to_string()),
            issued: Some(DateVariable::year(2023)),
            url: Some("https://stateofjs.com/2023".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-14: Edited book
    bib.insert(
        "ITEM-14".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-14".to_string(),
            ref_type: "book".to_string(),
            editor: Some(vec![
                Name::new("Reis", "Harry T."),
                Name::new("Judd", "Charles M."),
            ]),
            title: Some("Handbook of Research Methods in Social Psychology".to_string()),
            issued: Some(DateVariable::year(2000)),
            publisher: Some("Cambridge University Press".to_string()),
            publisher_place: Some("Cambridge".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-15: No author (edge case)
    bib.insert(
        "ITEM-15".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-15".to_string(),
            ref_type: "article-journal".to_string(),
            author: None,
            title: Some("The Role of Theory in Research".to_string()),
            container_title: Some("Journal of Theoretical Psychology".to_string()),
            issued: Some(DateVariable::year(2018)),
            volume: Some(StringOrNumber::String("28".to_string())),
            issue: Some(StringOrNumber::String("3".to_string())),
            page: Some("201-215".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-16: Newspaper article
    bib.insert(
        "ITEM-16".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-16".to_string(),
            ref_type: "article-newspaper".to_string(),
            author: Some(vec![Name::new("Rodriguez", "Maria")]),
            title: Some("Major Breakthrough in Quantum Computing Announced".to_string()),
            container_title: Some("The New York Times".to_string()),
            issued: Some(DateVariable::full(2024, 3, 15)),
            section: Some("Science".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-17: Magazine article
    bib.insert(
        "ITEM-17".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-17".to_string(),
            ref_type: "article-magazine".to_string(),
            author: Some(vec![Name::new("Thompson", "Alexander")]),
            title: Some("The Art of Minimalism in Modern Design".to_string()),
            container_title: Some("Wired".to_string()),
            issued: Some(DateVariable::year_month(2023, 6)),
            volume: Some(StringOrNumber::String("31".to_string())),
            issue: Some(StringOrNumber::String("6".to_string())),
            page: Some("42-49".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-18: Encyclopedia entry
    bib.insert(
        "ITEM-18".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-18".to_string(),
            ref_type: "entry-encyclopedia".to_string(),
            author: Some(vec![Name::new("Vasari", "Giorgio")]),
            title: Some("Renaissance Art and Culture".to_string()),
            container_title: Some("Encyclopedia of World History".to_string()),
            issued: Some(DateVariable::year(2022)),
            publisher: Some("Oxford University Press".to_string()),
            volume: Some(StringOrNumber::String("5".to_string())),
            page: Some("234-256".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-19: Dataset
    bib.insert(
        "ITEM-19".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-19".to_string(),
            ref_type: "dataset".to_string(),
            author: Some(vec![Name::literal(
                "NASA Goddard Institute for Space Studies",
            )]),
            title: Some("Global Temperature Anomalies 1880-2023".to_string()),
            issued: Some(DateVariable::year(2024)),
            publisher: Some("NASA".to_string()),
            url: Some("https://data.giss.nasa.gov/gistemp/".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-20: Legal case
    bib.insert(
        "ITEM-20".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-20".to_string(),
            ref_type: "legal_case".to_string(),
            title: Some("Brown v. Board of Education".to_string()),
            issued: Some(DateVariable::full(1954, 5, 17)),
            authority: Some("U.S. Supreme Court".to_string()),
            volume: Some(StringOrNumber::String("347".to_string())),
            container_title: Some("U.S. Reports".to_string()),
            page: Some("483".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-21: Patent
    bib.insert(
        "ITEM-21".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-21".to_string(),
            ref_type: "patent".to_string(),
            author: Some(vec![
                Name::new("Johnson", "David"),
                Name::new("Lee", "Sarah"),
            ]),
            title: Some("Method for Efficient Data Compression".to_string()),
            issued: Some(DateVariable::full(2021, 7, 13)),
            number: Some("US 11,043,211 B2".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-22: Motion picture
    bib.insert(
        "ITEM-22".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-22".to_string(),
            ref_type: "motion_picture".to_string(),
            author: Some(vec![Name::new("Lumière", "Louis")]),
            title: Some("The Arrival of a Train at La Ciotat Station".to_string()),
            issued: Some(DateVariable::year(1896)),
            genre: Some("Short film".to_string()),
            medium: Some("film".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-23: Broadcast
    bib.insert(
        "ITEM-23".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-23".to_string(),
            ref_type: "broadcast".to_string(),
            author: Some(vec![Name::new("Sagan", "Carl")]),
            title: Some("The Universe in a Nutshell".to_string()),
            issued: Some(DateVariable::full(1980, 9, 28)),
            container_title: Some("Cosmos: A Spacetime Odyssey".to_string()),
            number: Some("1".to_string()),
            medium: Some("Television".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-24: Interview
    bib.insert(
        "ITEM-24".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-24".to_string(),
            ref_type: "interview".to_string(),
            author: Some(vec![Name::new("Bengio", "Yoshua")]),
            title: Some("The Future of Artificial Intelligence".to_string()),
            issued: Some(DateVariable::full(2023, 11, 10)),
            url: Some("https://example.com/interview".to_string()),
            interviewer: Some(vec![Name::new("Colbert", "Stephen")]),
            medium: Some("Video interview".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-25: Book with translator
    bib.insert(
        "ITEM-25".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-25".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kafka", "Franz")]),
            title: Some("Metamorphosis".to_string()),
            issued: Some(DateVariable::year(1915)),
            publisher: Some("Kurt Wolff Verlag".to_string()),
            publisher_place: Some("Leipzig".to_string()),
            translator: Some(vec![Name::new("Wyllie", "David")]),
            ..Default::default()
        }),
    );

    // ITEM-26: Spinoza (tests older dates)
    bib.insert(
        "ITEM-26".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-26".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Spinoza", "Baruch")]),
            title: Some("Éthique".to_string()),
            issued: Some(DateVariable::year(1677)),
            publisher: Some("Jan Rieuwertsz".to_string()),
            publisher_place: Some("Amsterdam".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-27: Report with editor
    bib.insert(
        "ITEM-27".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-27".to_string(),
            ref_type: "report".to_string(),
            author: Some(vec![Name::literal("United Nations Environment Programme")]),
            editor: Some(vec![Name::new("Díaz", "Sandra")]),
            title: Some("State of the World's Biodiversity".to_string()),
            issued: Some(DateVariable::year(2019)),
            publisher: Some("UNEP".to_string()),
            publisher_place: Some("Nairobi".to_string()),
            genre: Some("Assessment report".to_string()),
            ..Default::default()
        }),
    );

    // ITEM-28: Personal communication
    bib.insert(
        "ITEM-28".to_string(),
        Reference::from(LegacyReference {
            id: "ITEM-28".to_string(),
            ref_type: "personal_communication".to_string(),
            author: Some(vec![Name::new("Smith", "Patricia")]),
            title: Some("Discussion on CSLN Schema Design".to_string()),
            issued: Some(DateVariable::full(2024, 2, 7)),
            recipient: Some(vec![Name::new("Darcus", "Bruce")]),
            ..Default::default()
        }),
    );

    bib
}

fn print_human(processor: &Processor, style_name: &str, show_cite: bool, show_bib: bool) {
    println!("\n=== {} ===\n", style_name);

    let item_ids = [
        "ITEM-1", "ITEM-2", "ITEM-3", "ITEM-4", "ITEM-5", "ITEM-6", "ITEM-7", "ITEM-8", "ITEM-9",
        "ITEM-10", "ITEM-11", "ITEM-12", "ITEM-13", "ITEM-14", "ITEM-15", "ITEM-16", "ITEM-17",
        "ITEM-18", "ITEM-19", "ITEM-20", "ITEM-21", "ITEM-22", "ITEM-23", "ITEM-24", "ITEM-25",
        "ITEM-26", "ITEM-27", "ITEM-28",
    ];

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
            match processor.process_citation(&citation) {
                Ok(text) => println!("  [{}] {}", id, text),
                Err(e) => println!("  [{}] ERROR: {}", id, e),
            }
        }
        println!();

        println!("CITATIONS (Integral/Narrative):");
        for id in &item_ids {
            let citation = Citation {
                id: Some(id.to_string()),
                items: vec![CitationItem {
                    id: id.to_string(),
                    ..Default::default()
                }],
                mode: csln_core::citation::CitationMode::Integral,
                ..Default::default()
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

    let item_ids = [
        "ITEM-1", "ITEM-2", "ITEM-3", "ITEM-4", "ITEM-5", "ITEM-6", "ITEM-7", "ITEM-8", "ITEM-9",
        "ITEM-10", "ITEM-11", "ITEM-12", "ITEM-13", "ITEM-14", "ITEM-15", "ITEM-16", "ITEM-17",
        "ITEM-18", "ITEM-19", "ITEM-20", "ITEM-21", "ITEM-22", "ITEM-23", "ITEM-24", "ITEM-25",
        "ITEM-26", "ITEM-27", "ITEM-28",
    ];

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
