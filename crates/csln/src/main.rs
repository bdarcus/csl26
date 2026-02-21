use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{Shell, generate};
use csln_core::locale::RawLocale;
use csln_core::reference::InputReference;
use csln_core::{InputBibliography, Locale, Style};
use csln_processor::{
    Bibliography, Citation, CitationItem, DocumentFormat, Processor,
    io::{load_bibliography, load_citations},
    processor::document::djot::DjotParser,
    render::{djot::Djot, html::Html, latex::Latex, plain::PlainText},
};
#[cfg(feature = "schema")]
use schemars::schema_for;
use serde::Serialize;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum RenderMode {
    Bib,
    Cite,
    Both,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum InputFormat {
    Djot,
    Markdown,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    Plain,
    Html,
    Djot,
    Latex,
    Typst,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Djot => write!(f, "djot"),
            OutputFormat::Latex => write!(f, "latex"),
            OutputFormat::Typst => write!(f, "typst"),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Render documents or references
    Render {
        #[command(subcommand)]
        command: RenderCommands,
    },

    /// Validate style, bibliography, and citations files
    Check(CheckArgs),

    /// Convert between CSLN formats (YAML, JSON, CBOR)
    Convert(ConvertArgs),

    /// List and inspect embedded (builtin) citation styles
    Styles {
        #[command(subcommand)]
        command: Option<StylesCommands>,
    },

    /// Generate JSON schema for CSLN models
    #[cfg(feature = "schema")]
    Schema(SchemaArgs),

    /// Generate shell completion scripts
    Completions {
        /// The shell to generate completions for
        shell: Shell,
    },

    /// Legacy alias for `render doc`
    #[command(hide = true)]
    Doc(LegacyDocArgs),

    /// Legacy alias for `check --style`
    #[command(hide = true)]
    Validate(LegacyValidateArgs),
}

#[derive(Subcommand)]
enum RenderCommands {
    /// Render a full document with citations and bibliography
    Doc(RenderDocArgs),

    /// Render references/citations directly
    Refs(RenderRefsArgs),
}

#[derive(Subcommand)]
enum StylesCommands {
    /// List all embedded (builtin) style names
    List,
}

#[derive(Args, Debug)]
struct RenderDocArgs {
    /// Path to input document
    #[arg(index = 1)]
    input: PathBuf,

    /// Style file path or builtin name (apa, mla, ieee, etc.)
    #[arg(short, long, required = true)]
    style: String,

    /// Path(s) to bibliography input files (repeat for multiple)
    #[arg(short, long, required = true, action = ArgAction::Append)]
    bibliography: Vec<PathBuf>,
    #[arg(short = 'c', long, action = ArgAction::Append)]
    citations: Vec<PathBuf>,

    /// Input document format
    #[arg(short = 'I', long = "input-format", value_enum, default_value_t = InputFormat::Djot)]
    input_format: InputFormat,

    /// Output format
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = OutputFormat::Plain
    )]
    format: OutputFormat,

    /// Write output to file (defaults to stdout)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Disable semantic classes (HTML spans, Djot attributes)
    #[arg(long)]
    no_semantics: bool,
}

#[derive(Args, Debug)]
struct RenderRefsArgs {
    /// Path(s) to bibliography input files (repeat for multiple)
    #[arg(short, long, required = true, action = ArgAction::Append)]
    bibliography: Vec<PathBuf>,

    /// Style file path or builtin name (apa, mla, ieee, etc.)
    #[arg(short, long, required = true)]
    style: String,

    /// Path(s) to citations input files (repeat for multiple)
    #[arg(short = 'c', long, action = ArgAction::Append)]
    citations: Vec<PathBuf>,

    /// Render mode
    #[arg(short = 'm', long, value_enum, default_value_t = RenderMode::Both)]
    mode: RenderMode,

    /// Specific reference keys to render (comma-separated)
    #[arg(short = 'k', long, value_delimiter = ',')]
    keys: Option<Vec<String>>,

    /// Show reference keys/IDs in human output
    #[arg(long)]
    show_keys: bool,

    /// Output as JSON
    #[arg(short = 'j', long)]
    json: bool,

    /// Output format
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = OutputFormat::Plain
    )]
    format: OutputFormat,

    /// Write output to file (defaults to stdout)
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Disable semantic classes (HTML spans, Djot attributes)
    #[arg(long)]
    no_semantics: bool,
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Style file path or builtin name (apa, mla, ieee, etc.)
    #[arg(short, long)]
    style: Option<String>,

    /// Path(s) to bibliography input files (repeat for multiple)
    #[arg(short, long, action = ArgAction::Append)]
    bibliography: Vec<PathBuf>,

    /// Path(s) to citations input files (repeat for multiple)
    #[arg(short = 'c', long, action = ArgAction::Append)]
    citations: Vec<PathBuf>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[cfg(feature = "schema")]
#[derive(Args, Debug)]
struct SchemaArgs {
    /// Data type (style, bib, locale, citations)
    #[arg(index = 1, value_enum)]
    r#type: Option<DataType>,

    /// Output directory to export all schemas
    #[arg(short, long)]
    out_dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct ConvertArgs {
    /// Path to input file
    #[arg(index = 1)]
    input: PathBuf,

    /// Path to output file
    #[arg(short = 'o', long)]
    output: PathBuf,

    /// Data type (style, bib, locale, citations)
    #[arg(short = 't', long = "type", value_enum)]
    r#type: Option<DataType>,
}

#[derive(Args, Debug)]
struct LegacyDocArgs {
    /// Path to the document file
    #[arg(index = 1)]
    document: PathBuf,

    /// Path to the references file
    #[arg(index = 2)]
    references: PathBuf,

    /// Path to the style file
    #[arg(index = 3)]
    style: PathBuf,

    /// Output format
    #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Plain)]
    format: OutputFormat,
}

#[derive(Args, Debug)]
struct LegacyValidateArgs {
    /// Path to style file
    path: PathBuf,
}

#[derive(Serialize)]
struct CheckItem {
    kind: &'static str,
    path: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
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
        Commands::Render { command } => match command {
            RenderCommands::Doc(args) => run_render_doc(args),
            RenderCommands::Refs(args) => run_render_refs(args),
        },
        Commands::Check(args) => run_check(args),
        Commands::Convert(args) => run_convert(args),
        Commands::Styles { command } => match command.unwrap_or(StylesCommands::List) {
            StylesCommands::List => run_styles_list(),
        },
        #[cfg(feature = "schema")]
        Commands::Schema(args) => run_schema(args),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Doc(args) => {
            eprintln!(
                "Warning: `csln doc` is deprecated. Use `csln render doc` with positional input."
            );
            let doc_args = RenderDocArgs {
                input: args.document,
                style: args.style.display().to_string(),
                bibliography: vec![args.references],
                citations: Vec::new(),
                input_format: InputFormat::Djot,
                format: args.format,
                output: None,
                no_semantics: false,
            };
            run_render_doc(doc_args)
        }
        Commands::Validate(args) => {
            eprintln!("Warning: `csln validate` is deprecated. Use `csln check --style`.");
            run_check(CheckArgs {
                style: Some(args.path.display().to_string()),
                bibliography: Vec::new(),
                citations: Vec::new(),
                json: false,
            })
        }
    }
}

#[cfg(feature = "schema")]
fn run_schema(args: SchemaArgs) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = args.out_dir {
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
        return Ok(());
    }

    if let Some(t) = args.r#type {
        let schema = match t {
            DataType::Style => schema_for!(Style),
            DataType::Bib => schema_for!(InputBibliography),
            DataType::Locale => schema_for!(RawLocale),
            DataType::Citations => schema_for!(csln_core::Citations),
        };
        println!("{}", serde_json::to_string_pretty(&schema)?);
        return Ok(());
    }

    Err("Specify a type (style, bib, locale, citation) or --out-dir".into())
}

fn run_styles_list() -> Result<(), Box<dyn Error>> {
    println!("Embedded (builtin) citation styles:");
    println!();
    println!("  {:<10} {:<40} {:<30}", "Alias", "Title", "Full Name");
    println!("  {}", "-".repeat(82));

    for name in csln_core::embedded::EMBEDDED_STYLE_NAMES {
        let style = csln_core::embedded::get_embedded_style(name)
            .ok_or_else(|| format!("failed to load builtin style: {}", name))??;

        let alias = csln_core::embedded::EMBEDDED_STYLE_ALIASES
            .iter()
            .find(|(_, full)| *full == *name)
            .map(|(a, _)| *a)
            .unwrap_or("-");

        let title = style.info.title.as_deref().unwrap_or("-");

        println!("  {:<10} {:<40} {:<30}", alias, truncate(title, 38), name);
    }

    println!();
    println!("Usage:");
    println!("  csln render refs -s <alias|name> -b refs.json");
    println!("  csln render doc <doc.dj> -s <alias|name> -b refs.json");
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn run_render_doc(args: RenderDocArgs) -> Result<(), Box<dyn Error>> {
    let style_obj = load_any_style(&args.style, args.no_semantics)?;
    let bibliography = load_merged_bibliography(&args.bibliography)?;

    if !args.citations.is_empty() {
        eprintln!(
            "Warning: --citations is currently ignored by `render doc`; citations are parsed from the input document."
        );
    }

    let processor = create_processor(style_obj, bibliography, &args.style);

    let doc_content = fs::read_to_string(&args.input)?;
    let output = match args.input_format {
        InputFormat::Djot => render_doc_with_output_format(
            &processor,
            &doc_content,
            args.format,
            DocumentInput::Djot,
        )?,
        InputFormat::Markdown => {
            return Err(
                "Input format `markdown` is not implemented yet. Use --input-format djot.".into(),
            );
        }
    };

    write_output(&output, args.output.as_ref())
}

fn run_render_refs(args: RenderRefsArgs) -> Result<(), Box<dyn Error>> {
    let style_obj = load_any_style(&args.style, args.no_semantics)?;
    let bibliography = load_merged_bibliography(&args.bibliography)?;

    let item_ids = if let Some(k) = args.keys.clone() {
        k
    } else {
        bibliography.keys().cloned().collect()
    };

    let input_citations = if args.citations.is_empty() {
        None
    } else {
        Some(load_merged_citations(&args.citations)?)
    };

    let processor = create_processor(style_obj, bibliography, &args.style);

    let style_name = {
        let path = Path::new(&args.style);
        if path.exists() {
            path.file_name()
                .map(|s: &std::ffi::OsStr| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            args.style.clone()
        }
    };

    let output = if args.json {
        render_refs_json(
            &processor,
            &style_name,
            args.mode,
            &item_ids,
            input_citations,
            args.format,
        )?
    } else {
        render_refs_human(
            &processor,
            &style_name,
            args.mode,
            &item_ids,
            input_citations,
            args.show_keys,
            args.format,
        )?
    };

    write_output(&output, args.output.as_ref())
}

fn create_processor(style: Style, bib: Bibliography, style_input: &str) -> Processor {
    if let Some(ref locale_id) = style.info.default_locale {
        let path = Path::new(style_input);
        let locale = if path.exists() && path.is_file() {
            // File-based style: search for locale on disk, fall back to embedded.
            let locales_dir = find_locales_dir(style_input);
            let disk_locale = Locale::load(locale_id, &locales_dir);
            if disk_locale.locale == *locale_id || locale_id == "en-US" {
                disk_locale
            } else {
                load_locale_builtin(locale_id)
            }
        } else {
            // Builtin style: use embedded locale directly.
            load_locale_builtin(locale_id)
        };
        Processor::with_locale(style, bib, locale)
    } else {
        Processor::new(style, bib)
    }
}

/// Load a style from a file path, or fallback to builtin name / alias.
fn load_any_style(style_input: &str, no_semantics: bool) -> Result<Style, Box<dyn Error>> {
    let path = Path::new(style_input);
    if path.exists() && path.is_file() {
        return load_style(path, no_semantics);
    }

    if let Some(res) = csln_core::embedded::get_embedded_style(style_input) {
        return res.map_err(|e| e.into());
    }

    // Fuzzy matching suggestion
    let suggestions: Vec<_> = csln_core::embedded::EMBEDDED_STYLE_NAMES
        .iter()
        .chain(
            csln_core::embedded::EMBEDDED_STYLE_ALIASES
                .iter()
                .map(|(a, _)| a),
        )
        .filter(|&&name| strsim::jaro_winkler(style_input, name) > 0.8)
        .collect();

    let mut msg = format!("style not found: '{}'", style_input);
    if !suggestions.is_empty() {
        msg.push_str("\n\nDid you mean one of these?");
        for s in suggestions {
            msg.push_str(&format!("\n  - {}", s));
        }
    } else {
        msg.push_str("\n\nUse `csln styles list` to see all available builtin styles.");
    }

    Err(msg.into())
}

fn run_check(args: CheckArgs) -> Result<(), Box<dyn Error>> {
    let mut checks = Vec::<CheckItem>::new();

    if let Some(style_input) = args.style {
        let status = match load_any_style(&style_input, false) {
            Ok(_) => CheckItem {
                kind: "style",
                path: style_input,
                ok: true,
                error: None,
            },
            Err(e) => CheckItem {
                kind: "style",
                path: style_input,
                ok: false,
                error: Some(e.to_string()),
            },
        };
        checks.push(status);
    }

    for path in args.bibliography {
        let display = path.display().to_string();
        let status = match load_bibliography(&path) {
            Ok(_) => CheckItem {
                kind: "bibliography",
                path: display,
                ok: true,
                error: None,
            },
            Err(e) => CheckItem {
                kind: "bibliography",
                path: display,
                ok: false,
                error: Some(e.to_string()),
            },
        };
        checks.push(status);
    }

    for path in args.citations {
        let display = path.display().to_string();
        let status = match load_citations(&path) {
            Ok(_) => CheckItem {
                kind: "citations",
                path: display,
                ok: true,
                error: None,
            },
            Err(e) => CheckItem {
                kind: "citations",
                path: display,
                ok: false,
                error: Some(e.to_string()),
            },
        };
        checks.push(status);
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        for check in &checks {
            if check.ok {
                println!("OK   {:<12} {}", check.kind, check.path);
            } else {
                println!("FAIL {:<12} {}", check.kind, check.path);
                if let Some(err) = &check.error {
                    println!("  -> {}", err);
                }
            }
        }
    }

    if checks.iter().any(|c| !c.ok) {
        return Err("One or more checks failed.".into());
    }

    Ok(())
}

fn run_convert(args: ConvertArgs) -> Result<(), Box<dyn Error>> {
    let input_bytes = fs::read(&args.input)?;
    let input_ext = args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("yaml");
    let output_ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("yaml");

    let data_type = args.r#type.unwrap_or_else(|| {
        let stem = args
            .input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if stem.contains("bib") || stem.contains("ref") {
            DataType::Bib
        } else if stem.contains("cite") || stem.contains("citation") {
            DataType::Citations
        } else if stem.len() == 5 && stem.contains('-') {
            DataType::Locale
        } else {
            DataType::Style
        }
    });

    match data_type {
        DataType::Style => {
            let style: Style = deserialize_any(&input_bytes, input_ext)?;
            let out_bytes = serialize_any(&style, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
        DataType::Bib => {
            let bib_obj = load_bibliography(&args.input)?;
            let references: Vec<InputReference> = bib_obj.into_iter().map(|(_, r)| r).collect();
            let input_bib = InputBibliography {
                references,
                ..Default::default()
            };
            let out_bytes = serialize_any(&input_bib, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
        DataType::Locale => {
            let locale: RawLocale = deserialize_any(&input_bytes, input_ext)?;
            let out_bytes = serialize_any(&locale, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
        DataType::Citations => {
            let citations: csln_core::citation::Citations =
                deserialize_any(&input_bytes, input_ext)?;
            let out_bytes = serialize_any(&citations, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
    }

    println!(
        "Converted {} to {}",
        args.input.display(),
        args.output.display()
    );
    Ok(())
}

enum DocumentInput {
    Djot,
}

fn render_doc_with_output_format(
    processor: &Processor,
    content: &str,
    output_format: OutputFormat,
    input_format: DocumentInput,
) -> Result<String, Box<dyn Error>> {
    let doc_format = to_document_format(output_format)?;

    match input_format {
        DocumentInput::Djot => {
            let parser = DjotParser;
            match output_format {
                OutputFormat::Plain => {
                    Ok(processor.process_document::<_, PlainText>(content, &parser, doc_format))
                }
                OutputFormat::Html => {
                    Ok(processor.process_document::<_, Html>(content, &parser, doc_format))
                }
                OutputFormat::Djot => {
                    Ok(processor.process_document::<_, Djot>(content, &parser, doc_format))
                }
                OutputFormat::Latex => {
                    Ok(processor.process_document::<_, Latex>(content, &parser, doc_format))
                }
                OutputFormat::Typst => Err(
                    "Output format `typst` is not implemented yet for document rendering.".into(),
                ),
            }
        }
    }
}

fn to_document_format(output_format: OutputFormat) -> Result<DocumentFormat, Box<dyn Error>> {
    match output_format {
        OutputFormat::Plain => Ok(DocumentFormat::Plain),
        OutputFormat::Html => Ok(DocumentFormat::Html),
        OutputFormat::Djot => Ok(DocumentFormat::Djot),
        OutputFormat::Latex => Ok(DocumentFormat::Latex),
        OutputFormat::Typst => {
            Err("Output format `typst` is not implemented yet for document rendering.".into())
        }
    }
}

fn render_refs_human(
    processor: &Processor,
    style_name: &str,
    mode: RenderMode,
    item_ids: &[String],
    citations: Option<Vec<Citation>>,
    show_keys: bool,
    output_format: OutputFormat,
) -> Result<String, Box<dyn Error>> {
    let show_cite = matches!(mode, RenderMode::Cite | RenderMode::Both);
    let show_bib = matches!(mode, RenderMode::Bib | RenderMode::Both);
    match output_format {
        OutputFormat::Plain => print_human_safe::<PlainText>(
            processor, style_name, show_cite, show_bib, item_ids, citations, show_keys,
        )
        .map_err(|e| e.into()),
        OutputFormat::Html => print_human_safe::<Html>(
            processor, style_name, show_cite, show_bib, item_ids, citations, show_keys,
        )
        .map_err(|e| e.into()),
        OutputFormat::Djot => print_human_safe::<Djot>(
            processor, style_name, show_cite, show_bib, item_ids, citations, show_keys,
        )
        .map_err(|e| e.into()),
        OutputFormat::Latex => print_human_safe::<Latex>(
            processor, style_name, show_cite, show_bib, item_ids, citations, show_keys,
        )
        .map_err(|e| e.into()),
        OutputFormat::Typst => {
            Err("Output format `typst` is not implemented yet for reference rendering.".into())
        }
    }
}

fn render_refs_json(
    processor: &Processor,
    style_name: &str,
    mode: RenderMode,
    item_ids: &[String],
    citations: Option<Vec<Citation>>,
    output_format: OutputFormat,
) -> Result<String, Box<dyn Error>> {
    let show_cite = matches!(mode, RenderMode::Cite | RenderMode::Both);
    let show_bib = matches!(mode, RenderMode::Bib | RenderMode::Both);
    match output_format {
        OutputFormat::Plain => print_json_with_format::<PlainText>(
            processor, style_name, show_cite, show_bib, item_ids, citations,
        ),
        OutputFormat::Html => print_json_with_format::<Html>(
            processor, style_name, show_cite, show_bib, item_ids, citations,
        ),
        OutputFormat::Djot => print_json_with_format::<Djot>(
            processor, style_name, show_cite, show_bib, item_ids, citations,
        ),
        OutputFormat::Latex => print_json_with_format::<Latex>(
            processor, style_name, show_cite, show_bib, item_ids, citations,
        ),
        OutputFormat::Typst => {
            Err("Output format `typst` is not implemented yet for JSON reference rendering.".into())
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

fn load_style(path: &Path, no_semantics: bool) -> Result<Style, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("yaml");

    let mut style_obj: Style = match ext {
        "cbor" => serde_cbor::from_slice(&bytes)?,
        "json" => serde_json::from_slice(&bytes)?,
        _ => serde_yaml::from_slice(&bytes)?,
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

    Ok(style_obj)
}

/// Load a locale from embedded bytes, falling back to en-US.
fn load_locale_builtin(locale_id: &str) -> Locale {
    if let Some(bytes) = csln_core::embedded::get_locale_bytes(locale_id) {
        let content = String::from_utf8_lossy(bytes);
        Locale::from_yaml_str(&content).unwrap_or_else(|_| Locale::en_us())
    } else {
        // Locale not bundled — fall back to the hardcoded en-US default.
        Locale::en_us()
    }
}

fn load_merged_bibliography(paths: &[PathBuf]) -> Result<Bibliography, Box<dyn Error>> {
    if paths.is_empty() {
        return Err("At least one --bibliography file is required.".into());
    }

    let mut merged = Bibliography::new();
    for path in paths {
        let loaded = load_bibliography(path)?;
        for (id, reference) in loaded {
            merged.insert(id, reference);
        }
    }

    Ok(merged)
}

fn load_merged_citations(paths: &[PathBuf]) -> Result<Vec<Citation>, Box<dyn Error>> {
    let mut merged = Vec::new();
    for path in paths {
        let loaded = load_citations(path)?;
        merged.extend(loaded);
    }
    Ok(merged)
}

fn write_output(output: &str, path: Option<&PathBuf>) -> Result<(), Box<dyn Error>> {
    if let Some(file) = path {
        fs::write(file, output)?;
    } else {
        println!("{}", output);
    }
    Ok(())
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
) -> Result<String, String>
where
    F: csln_processor::render::format::OutputFormat<Output = String> + Send + Sync + 'static,
{
    use std::panic;

    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        print_human::<F>(
            processor, style_name, show_cite, show_bib, item_ids, citations, show_keys,
        )
    }));

    match result {
        Ok(output) => Ok(output),
        Err(_) => Err(
            "The processor encountered a critical error during rendering. Please report this issue with the style and data used."
                .to_string(),
        ),
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
) -> String
where
    F: csln_processor::render::format::OutputFormat<Output = String>,
{
    let mut output = String::new();
    let _ = writeln!(output, "\n=== {} ===\n", style_name);

    if show_cite {
        if let Some(cite_list) = citations {
            let _ = writeln!(output, "CITATIONS (From file):");
            for (i, citation) in cite_list.iter().enumerate() {
                match processor.process_citation_with_format::<F>(citation) {
                    Ok(text) => {
                        if show_keys {
                            let _ = writeln!(
                                output,
                                "  [{}] {}",
                                citation.id.as_deref().unwrap_or(&format!("{}", i)),
                                text
                            );
                        } else {
                            let _ = writeln!(output, "  {}", text);
                        }
                    }
                    Err(e) => {
                        let _ = writeln!(
                            output,
                            "  [{}] ERROR: {}",
                            citation.id.as_deref().unwrap_or(&format!("{}", i)),
                            e
                        );
                    }
                }
            }
        } else {
            let _ = writeln!(output, "CITATIONS (Non-Integral):");
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
                            let _ = writeln!(output, "  [{}] {}", id, text);
                        } else {
                            let _ = writeln!(output, "  {}", text);
                        }
                    }
                    Err(e) => {
                        let _ = writeln!(output, "  [{}] ERROR: {}", id, e);
                    }
                }
            }
            let _ = writeln!(output);

            let _ = writeln!(output, "CITATIONS (Integral):");
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
                            let _ = writeln!(output, "  [{}] {}", id, text);
                        } else {
                            let _ = writeln!(output, "  {}", text);
                        }
                    }
                    Err(e) => {
                        let _ = writeln!(output, "  [{}] ERROR: {}", id, e);
                    }
                }
            }
        }
        let _ = writeln!(output);
    }

    if show_bib {
        let _ = writeln!(output, "BIBLIOGRAPHY:");
        let filter: HashSet<&str> = item_ids.iter().map(|id| id.as_str()).collect();
        let processed = processor.process_references();
        let mut rendered_entries = Vec::new();

        for entry in processed.bibliography {
            if filter.contains(entry.id.as_str()) {
                let text =
                    csln_processor::render::refs_to_string_with_format::<F>(vec![entry.clone()]);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if show_keys {
                        rendered_entries.push(format!("  [{}] {}", entry.id, trimmed));
                    } else {
                        rendered_entries.push(trimmed.to_string());
                    }
                }
            }
        }

        if show_keys {
            for entry in rendered_entries {
                let _ = writeln!(output, "{}", entry);
            }
        } else if !rendered_entries.is_empty() {
            let _ = writeln!(output, "{}", rendered_entries.join("\n\n"));
        }
    }

    output
}

fn print_json_with_format<F>(
    processor: &Processor,
    style_name: &str,
    show_cite: bool,
    show_bib: bool,
    item_ids: &[String],
    citations: Option<Vec<Citation>>,
) -> Result<String, Box<dyn Error>>
where
    F: csln_processor::render::format::OutputFormat<Output = String>,
{
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
                        "text": processor
                            .process_citation_with_format::<F>(c)
                            .unwrap_or_else(|e| e.to_string())
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
                        "text": processor
                            .process_citation_with_format::<F>(&citation)
                            .unwrap_or_else(|e| e.to_string())
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
                        "text": processor
                            .process_citation_with_format::<F>(&citation)
                            .unwrap_or_else(|e| e.to_string())
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
        let filter: HashSet<&str> = item_ids.iter().map(|id| id.as_str()).collect();
        let processed = processor.process_references();
        let entries: Vec<_> = processed
            .bibliography
            .into_iter()
            .filter(|entry| filter.contains(entry.id.as_str()))
            .map(|entry| {
                let text =
                    csln_processor::render::refs_to_string_with_format::<F>(vec![entry.clone()]);
                json!({
                    "id": entry.id,
                    "text": text.trim()
                })
            })
            .collect();

        result["bibliography"] = json!({ "entries": entries });
    }

    Ok(serde_json::to_string_pretty(&result)?)
}
