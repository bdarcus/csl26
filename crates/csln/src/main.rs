use clap::{Parser, Subcommand, ValueEnum};
use csln_core::{Locale, Style};
use csln_processor::{
    io::load_bibliography,
    render::{djot::Djot, html::Html, plain::PlainText},
    Citation, CitationItem, Processor,
};
use schemars::schema_for;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate JSON schema for CSLN styles
    Schema,
    /// Process a bibliography and citations
    Process {
        /// Path to the references file (CSLN YAML/JSON or CSL-JSON)
        #[arg(index = 1)]
        references: PathBuf,

        /// Path to the style YAML file
        #[arg(index = 2)]
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

        /// Specific citation keys to render (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        keys: Option<Vec<String>>,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Disable semantic classes (HTML spans, Djot attributes)
        #[arg(long)]
        no_semantics: bool,
    },
    /// Validate a CSLN style file
    Validate {
        /// Path to the style YAML/JSON file
        path: PathBuf,
    },
    /// Show the structure of a CSLN style
    Tree {
        /// Path to the style YAML/JSON file
        path: PathBuf,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Format {
    Plain,
    Html,
    Djot,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Plain => write!(f, "plain"),
            Format::Html => write!(f, "html"),
            Format::Djot => write!(f, "djot"),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Schema => {
            let schema = schema_for!(Style);
            println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        }
        Commands::Process {
            references,
            style,
            format,
            mut bib,
            mut cite,
            keys,
            json,
            no_semantics,
        } => {
            // Default behavior: show both if neither is specified
            if !bib && !cite {
                bib = true;
                cite = true;
            }

            // Load style
            let style_content = match fs::read_to_string(&style) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!("Error reading style: {}", e);
                    std::process::exit(1);
                }
            };

            let mut style_obj: Style = match serde_yaml::from_str(&style_content) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error parsing style: {}", e);
                    std::process::exit(1);
                }
            };

            if no_semantics {
                if let Some(ref mut options) = style_obj.options {
                    options.semantic_classes = Some(false);
                } else {
                    style_obj.options = Some(csln_core::options::Config {
                        semantic_classes: Some(false),
                        ..Default::default()
                    });
                }
            }

            // Load bibliography
            let bibliography = match load_bibliography(&references) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };

            // Determine citation keys
            let item_ids = if let Some(ref k) = keys {
                k.clone()
            } else {
                bibliography.keys().cloned().collect()
            };

            // Determine locales directory
            let locales_dir = find_locales_dir(style.to_str().unwrap_or("."));

            // Create processor with locale support
            let processor = if let Some(ref locale_id) = style_obj.info.default_locale {
                let locale = Locale::load(locale_id, &locales_dir);
                Processor::with_locale(style_obj, bibliography, locale)
            } else {
                Processor::new(style_obj, bibliography)
            };

            let style_name = style
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if json {
                print_json(&processor, &style_name, cite, bib, &item_ids);
            } else {
                match format {
                    Format::Plain => {
                        print_human::<PlainText>(&processor, &style_name, cite, bib, &item_ids);
                    }
                    Format::Html => {
                        print_human::<Html>(&processor, &style_name, cite, bib, &item_ids);
                    }
                    Format::Djot => {
                        print_human::<Djot>(&processor, &style_name, cite, bib, &item_ids);
                    }
                }
            }
        }
        Commands::Validate { path } => {
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            // Try parsing as Style
            match serde_yaml::from_str::<Style>(&content) {
                Ok(_) => println!("Reference style is valid."),
                Err(e) => {
                    eprintln!("Validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Tree { path: _ } => {
            eprintln!("The 'tree' command is not yet implemented.");
            std::process::exit(1);
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
