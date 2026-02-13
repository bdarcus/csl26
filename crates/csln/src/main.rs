use clap::{Parser, Subcommand, ValueEnum};
use csln_core::locale::RawLocale;
use csln_core::reference::InputReference;
use csln_core::{InputBibliography, Locale, Style};
use csln_processor::{
    io::load_bibliography,
    render::{djot::Djot, html::Html, plain::PlainText},
    Citation, CitationItem, Processor,
};
use schemars::schema_for;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum DataType {
    Style,
    Bib,
    Locale,
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
    /// Convert between CSLN formats (YAML, JSON, CBOR)
    Convert {
        /// Path to the input file
        input: PathBuf,

        /// Path to the output file
        #[arg(short, long)]
        output: PathBuf,

        /// Data type (style, bib, locale)
        #[arg(short, long, value_enum)]
        r#type: Option<DataType>,
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
            let style_bytes = match fs::read(&style) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error reading style: {}", e);
                    std::process::exit(1);
                }
            };

            let style_ext = style.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

            let mut style_obj: Style = match style_ext {
                "cbor" => match serde_cbor::from_slice(&style_bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error parsing CBOR style: {}", e);
                        std::process::exit(1);
                    }
                },
                "json" => match serde_json::from_slice(&style_bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error parsing JSON style: {}", e);
                        std::process::exit(1);
                    }
                },
                _ => match serde_yaml::from_slice(&style_bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error parsing YAML style: {}", e);
                        std::process::exit(1);
                    }
                },
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
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

            // Try parsing as Style
            let res = match ext {
                "cbor" => serde_cbor::from_slice::<Style>(&bytes)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                "json" => serde_json::from_slice::<Style>(&bytes)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
                _ => serde_yaml::from_slice::<Style>(&bytes)
                    .map(|_| ())
                    .map_err(|e| e.to_string()),
            };

            match res {
                Ok(_) => println!("Reference style is valid."),
                Err(e) => {
                    eprintln!("Validation failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Convert {
            input,
            output,
            r#type,
        } => {
            let input_bytes = fs::read(&input).expect("Failed to read input file");
            let input_ext = input.extension().and_then(|e| e.to_str()).unwrap_or("yaml");
            let output_ext = output
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("yaml");

            // Detect data type if not provided
            let data_type = r#type.unwrap_or_else(|| {
                let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if stem.contains("bib") || stem.contains("ref") {
                    DataType::Bib
                } else if stem.len() == 5 && stem.contains('-') {
                    // e.g. en-US
                    DataType::Locale
                } else {
                    DataType::Style
                }
            });

            match data_type {
                DataType::Style => {
                    let style: Style = deserialize_any(&input_bytes, input_ext);
                    let out_bytes = serialize_any(&style, output_ext);
                    fs::write(&output, out_bytes).expect("Failed to write output");
                }
                DataType::Bib => {
                    let bib_obj = load_bibliography(&input).expect("Failed to load bibliography");
                    // Convert internal Bibliography (IndexMap) back to InputBibliography
                    let references: Vec<InputReference> =
                        bib_obj.into_iter().map(|(_, r)| r).collect();
                    let input_bib = InputBibliography {
                        references,
                        ..Default::default()
                    };
                    let out_bytes = serialize_any(&input_bib, output_ext);
                    fs::write(&output, out_bytes).expect("Failed to write output");
                }
                DataType::Locale => {
                    let locale: RawLocale = deserialize_any(&input_bytes, input_ext);
                    let out_bytes = serialize_any(&locale, output_ext);
                    fs::write(&output, out_bytes).expect("Failed to write output");
                }
            }
            println!("Converted {} to {}", input.display(), output.display());
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

fn deserialize_any<T: serde::de::DeserializeOwned>(bytes: &[u8], ext: &str) -> T {
    match ext {
        "yaml" | "yml" => serde_yaml::from_slice(bytes).expect("Failed to parse YAML"),
        "json" => serde_json::from_slice(bytes).expect("Failed to parse JSON"),
        "cbor" => serde_cbor::from_slice(bytes).expect("Failed to parse CBOR"),
        _ => serde_yaml::from_slice(bytes).expect("Failed to parse YAML (fallback)"),
    }
}

fn serialize_any<T: Serialize>(obj: &T, ext: &str) -> Vec<u8> {
    match ext {
        "yaml" | "yml" => serde_yaml::to_string(obj)
            .expect("Failed to serialize YAML")
            .into_bytes(),
        "json" => serde_json::to_string_pretty(obj)
            .expect("Failed to serialize JSON")
            .into_bytes(),
        "cbor" => serde_cbor::to_vec(obj).expect("Failed to serialize CBOR"),
        _ => serde_yaml::to_string(obj)
            .expect("Failed to serialize YAML (fallback)")
            .into_bytes(),
    }
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
