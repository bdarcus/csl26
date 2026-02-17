use clap::{Parser, Subcommand, ValueEnum};
use csln_core::locale::RawLocale;
use csln_core::reference::InputReference;
use csln_core::{InputBibliography, Locale, Style};
use csln_processor::{
    io::{load_bibliography, load_citations},
    processor::document::djot::DjotParser,
    render::{djot::Djot, html::Html, latex::Latex, plain::PlainText},
    Citation, CitationItem, DocumentFormat, Processor,
};
#[cfg(feature = "schema")]
use schemars::schema_for;
use serde::Serialize;
use std::error::Error;
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
    Citations,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate JSON schema for CSLN models
    #[cfg(feature = "schema")]
    Schema {
        /// Data type (style, bib, locale, citations)
        #[arg(index = 1, value_enum)]
        r#type: Option<DataType>,

        /// Output directory to export all schemas
        #[arg(short, long)]
        out_dir: Option<PathBuf>,
    },
    /// Process a bibliography and citations
    Process {
        /// Path to the references file (CSLN YAML/JSON or CSL-JSON)
        #[arg(index = 1)]
        references: PathBuf,

        /// Path to the style YAML file
        #[arg(index = 2)]
        style: PathBuf,

        /// Path to the citations file (CSLN YAML/JSON)
        #[arg(short = 'c', long)]
        citations: Option<PathBuf>,

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
        #[arg(short = 'k', long, value_delimiter = ',')]
        keys: Option<Vec<String>>,

        /// Show reference keys/IDs in output (default: false)
        #[arg(long)]
        show_keys: bool,

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
    /// Process a full document
    Doc {
        /// Path to the document file
        #[arg(index = 1)]
        document: PathBuf,

        /// Path to the references file
        #[arg(index = 2)]
        references: PathBuf,

        /// Path to the style YAML file
        #[arg(index = 3)]
        style: PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value_t = Format::Plain)]
        format: Format,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Format {
    Plain,
    Html,
    Djot,
    Latex,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Plain => write!(f, "plain"),
            Format::Html => write!(f, "html"),
            Format::Djot => write!(f, "djot"),
            Format::Latex => write!(f, "latex"),
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\nError: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        #[cfg(feature = "schema")]
        Commands::Schema { r#type, out_dir } => {
            if let Some(dir) = out_dir {
                fs::create_dir_all(&dir)?;
                let types = [
                    (DataType::Style, "style.json"),
                    (DataType::Bib, "bib.json"),
                    (DataType::Locale, "locale.json"),
                    (DataType::Citations, "citations.json"),
                ];
                for (t, filename) in types {
                    let schema = match t {
                        DataType::Style => schema_for!(Style),
                        DataType::Bib => schema_for!(InputBibliography),
                        DataType::Locale => schema_for!(RawLocale),
                        DataType::Citations => schema_for!(csln_core::Citations),
                    };
                    let path = dir.join(filename);
                    fs::write(&path, serde_json::to_string_pretty(&schema)?)?;
                }
                println!("Schemas exported to {}", dir.display());
            } else if let Some(t) = r#type {
                let schema = match t {
                    DataType::Style => schema_for!(Style),
                    DataType::Bib => schema_for!(InputBibliography),
                    DataType::Locale => schema_for!(RawLocale),
                    DataType::Citations => schema_for!(csln_core::Citations),
                };
                println!("{}", serde_json::to_string_pretty(&schema)?);
            } else {
                return Err("Specify a type (style, bib, locale, citation) or --out-dir".into());
            }
        }
        Commands::Process {
            references,
            style,
            citations,
            format,
            mut bib,
            mut cite,
            keys,
            show_keys,
            json,
            no_semantics,
        } => {
            // Default behavior: show both if neither is specified
            if !bib && !cite {
                bib = true;
                cite = true;
            }

            // Load style
            let style_bytes = fs::read(&style)?;
            let style_ext = style.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

            let mut style_obj: Style = match style_ext {
                "cbor" => serde_cbor::from_slice(&style_bytes)?,
                "json" => serde_json::from_slice(&style_bytes)?,
                _ => serde_yaml::from_slice(&style_bytes)?,
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
            let bibliography = load_bibliography(&references)?;

            // Load citations if provided
            let input_citations = if let Some(ref path) = citations {
                Some(load_citations(path)?)
            } else {
                None
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
                print_json(
                    &processor,
                    &style_name,
                    cite,
                    bib,
                    &item_ids,
                    input_citations,
                    show_keys,
                );
            } else {
                match format {
                    Format::Plain => {
                        print_human_safe::<PlainText>(
                            &processor,
                            &style_name,
                            cite,
                            bib,
                            &item_ids,
                            input_citations,
                            show_keys,
                        );
                    }
                    Format::Html => {
                        print_human_safe::<Html>(
                            &processor,
                            &style_name,
                            cite,
                            bib,
                            &item_ids,
                            input_citations,
                            show_keys,
                        );
                    }
                    Format::Djot => {
                        print_human_safe::<Djot>(
                            &processor,
                            &style_name,
                            cite,
                            bib,
                            &item_ids,
                            input_citations,
                            show_keys,
                        );
                    }
                    Format::Latex => {
                        print_human_safe::<Latex>(
                            &processor,
                            &style_name,
                            cite,
                            bib,
                            &item_ids,
                            input_citations,
                            show_keys,
                        );
                    }
                }
            }
        }
        Commands::Validate { path } => {
            let bytes = fs::read(&path)?;
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
                Err(e) => return Err(format!("Validation failed: {}", e).into()),
            }
        }
        Commands::Convert {
            input,
            output,
            r#type,
        } => {
            let input_bytes = fs::read(&input)?;
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
                } else if stem.contains("cite") || stem.contains("citation") {
                    DataType::Citations
                } else if stem.len() == 5 && stem.contains('-') {
                    // e.g. en-US
                    DataType::Locale
                } else {
                    DataType::Style
                }
            });

            match data_type {
                DataType::Style => {
                    let style: Style = deserialize_any(&input_bytes, input_ext)?;
                    let out_bytes = serialize_any(&style, output_ext)?;
                    fs::write(&output, out_bytes)?;
                }
                DataType::Bib => {
                    let bib_obj = load_bibliography(&input)?;
                    // Convert internal Bibliography (IndexMap) back to InputBibliography
                    let references: Vec<InputReference> =
                        bib_obj.into_iter().map(|(_, r)| r).collect();
                    let input_bib = InputBibliography {
                        references,
                        ..Default::default()
                    };
                    let out_bytes = serialize_any(&input_bib, output_ext)?;
                    fs::write(&output, out_bytes)?;
                }
                DataType::Locale => {
                    let locale: RawLocale = deserialize_any(&input_bytes, input_ext)?;
                    let out_bytes = serialize_any(&locale, output_ext)?;
                    fs::write(&output, out_bytes)?;
                }
                DataType::Citations => {
                    let citations: csln_core::citation::Citations =
                        deserialize_any(&input_bytes, input_ext)?;
                    let out_bytes = serialize_any(&citations, output_ext)?;
                    fs::write(&output, out_bytes)?;
                }
            }
            println!("Converted {} to {}", input.display(), output.display());
        }
        Commands::Tree { path: _ } => {
            return Err("The 'tree' command is not yet implemented.".into());
        }
        Commands::Doc {
            document,
            references,
            style,
            format,
        } => {
            // Load style
            let style_bytes = fs::read(&style)?;
            let style_obj: Style = serde_yaml::from_slice(&style_bytes)?;

            // Load bibliography
            let bibliography = load_bibliography(&references)?;

            // Load document
            let doc_content = fs::read_to_string(&document)?;

            // Determine locales directory
            let locales_dir = find_locales_dir(style.to_str().unwrap_or("."));

            // Create processor
            let processor = if let Some(ref locale_id) = style_obj.info.default_locale {
                let locale = Locale::load(locale_id, &locales_dir);
                Processor::with_locale(style_obj, bibliography, locale)
            } else {
                Processor::new(style_obj, bibliography)
            };

            let parser = DjotParser;

            let doc_format = match format {
                Format::Plain => DocumentFormat::Plain,
                Format::Html => DocumentFormat::Html,
                Format::Djot => DocumentFormat::Djot,
                Format::Latex => DocumentFormat::Latex,
            };

            let output = match format {
                Format::Plain => {
                    processor.process_document::<_, PlainText>(&doc_content, &parser, doc_format)
                }
                Format::Html => {
                    processor.process_document::<_, Html>(&doc_content, &parser, doc_format)
                }
                Format::Djot => {
                    processor.process_document::<_, Djot>(&doc_content, &parser, doc_format)
                }
                Format::Latex => {
                    processor.process_document::<_, Latex>(&doc_content, &parser, doc_format)
                }
            };

            println!("{}", output);
        }
    }
    Ok(())
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

fn deserialize_any<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    ext: &str,
) -> Result<T, Box<dyn Error>> {
    match ext {
        "yaml" | "yml" => Ok(serde_yaml::from_slice(bytes)?),
        "json" => Ok(serde_json::from_slice(bytes)?),
        "cbor" => Ok(serde_cbor::from_slice(bytes)?),
        _ => Ok(serde_yaml::from_slice(bytes)?),
    }
}

fn serialize_any<T: Serialize>(obj: &T, ext: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    match ext {
        "yaml" | "yml" => Ok(serde_yaml::to_string(obj)?.into_bytes()),
        "json" => Ok(serde_json::to_string_pretty(obj)?.into_bytes()),
        "cbor" => Ok(serde_cbor::to_vec(obj)?),
        _ => Ok(serde_yaml::to_string(obj)?.into_bytes()),
    }
}

fn print_human_safe<F>(
    processor: &Processor,
    style_name: &str,
    show_cite: bool,
    show_bib: bool,
    item_ids: &[String],
    citations: Option<Vec<Citation>>,
    show_keys: bool,
) where
    F: csln_processor::render::format::OutputFormat<Output = String> + Send + Sync + 'static,
{
    use std::panic;

    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        print_human::<F>(
            processor, style_name, show_cite, show_bib, item_ids, citations, show_keys,
        );
    }));

    if result.is_err() {
        eprintln!("\nError: The processor encountered a critical error during rendering.");
        eprintln!("This is likely due to an unexpected character or data structure in the style or bibliography.");
        eprintln!("Please report this issue with the style and data used.");
    }
}

fn print_human<F>(
    processor: &Processor,
    style_name: &str,
    show_cite: bool,
    show_bib: bool,
    item_ids: &[String],
    citations: Option<Vec<Citation>>,
    show_keys: bool,
) where
    F: csln_processor::render::format::OutputFormat<Output = String>,
{
    println!("\n=== {} ===\n", style_name);

    if show_cite {
        if let Some(cite_list) = citations {
            println!("CITATIONS (From file):");
            for (i, citation) in cite_list.iter().enumerate() {
                match processor.process_citation_with_format::<F>(citation) {
                    Ok(text) => {
                        if show_keys {
                            println!(
                                "  [{}] {}",
                                citation.id.as_deref().unwrap_or(&format!("{}", i)),
                                text
                            );
                        } else {
                            println!("  {}", text);
                        }
                    }
                    Err(e) => println!(
                        "  [{}] ERROR: {}",
                        citation.id.as_deref().unwrap_or(&format!("{}", i)),
                        e
                    ),
                }
            }
        } else {
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
                    Ok(text) => {
                        if show_keys {
                            println!("  [{}] {}", id, text);
                        } else {
                            println!("  {}", text);
                        }
                    }
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
                    Ok(text) => {
                        if show_keys {
                            println!("  [{}] {}", id, text);
                        } else {
                            println!("  {}", text);
                        }
                    }
                    Err(e) => println!("  [{}] ERROR: {}", id, e),
                }
            }
        }
        println!();
    }

    if show_bib {
        println!("BIBLIOGRAPHY:");
        if show_keys {
            let processed = processor.process_references();
            for entry in processed.bibliography {
                // Render this single entry
                let text =
                    csln_processor::render::refs_to_string_with_format::<F>(vec![entry.clone()]);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    println!("  [{}] {}", entry.id, trimmed);
                }
            }
        } else {
            let text = processor.render_bibliography_with_format::<F>();
            println!("{}", text);
        }
    }
}

fn print_json(
    processor: &Processor,
    style_name: &str,
    show_cite: bool,
    show_bib: bool,
    item_ids: &[String],
    citations: Option<Vec<Citation>>,
    _show_keys: bool,
) {
    use serde_json::json;

    let mut result = json!({
        "style": style_name,
        "items": item_ids.len()
    });

    if show_cite {
        if let Some(cite_list) = citations {
            let rendered: Vec<_> = cite_list
                .iter()
                .map(|c| {
                    json!({
                        "id": c.id,
                        "text": processor.process_citation(c).unwrap_or_else(|e| e.to_string())
                    })
                })
                .collect();
            result["citations"] = json!(rendered);
        } else {
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
