/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::Serialize;
use std::path::PathBuf;

const CLAP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
    name = "citum",
    author,
    version,
    about = "Modern, performant, and multilingual citation, bibliography, and document processor",
    long_about = "Citum is a Rust-based, declarative citation styling system.\n\n\
                  Styles are expressed as YAML templates and options, then rendered\n\
                  by a type-safe processor.\n\n\
                  EXAMPLES:\n  \
                  Render a document:\n    \
                  citum render doc input.djot -b refs.json -s apa-7th\n\n  \
                  Render references (human-readable):\n    \
                  citum render refs -b refs.json -s apa-7th\n\n  \
                  Check style and bibliography:\n    \
                  citum check -s apa-7th -b refs.json\n\n  \
                  Convert a style to binary CBOR:\n    \
                  citum convert style style.yaml -o style.cbor\n\n  \
                  Search available styles:\n    \
                  citum style search apa\n\n\
                  Run 'citum <COMMAND> --help' for more detailed examples and options.",
    styles = CLAP_STYLES,
    arg_required_else_help = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum DataType {
    Style,
    Bib,
    Locale,
    Citations,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum RenderMode {
    Bib,
    Cite,
    Both,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum InputFormat {
    Djot,
    Markdown,
}

/// Valid target types for JSON schema export.
#[cfg(feature = "schema")]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum SchemaType {
    /// Citation style schema
    Style,
    /// Bibliography input schema
    Bib,
    /// Locale schema
    Locale,
    /// Citation input schema
    Citation,
    /// Style registry schema
    Registry,
    /// Abbreviation map schema
    #[value(name = "abbrev-map")]
    AbbrevMap,
    /// JSON-RPC server method parameter schemas
    Server,
    /// Declarative BibLaTeX/CSL-JSON ↔ Citum type and field mapping tables
    /// (not a JSON Schema — a data dump consumed by
    /// `scripts/build-data-model-reference.js` to generate
    /// `docs/reference/BIBLATEX_MAPPING.md` and
    /// `docs/reference/generated/CSL_JSON_MAPPING.md`).
    #[value(name = "type-map")]
    TypeMap,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum OutputFormat {
    Plain,
    Html,
    Djot,
    /// CommonMark output; pipe to pandoc or any CommonMark-aware tool
    Markdown,
    Latex,
    Typst,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plain => write!(f, "plain"),
            OutputFormat::Html => write!(f, "html"),
            OutputFormat::Djot => write!(f, "djot"),
            OutputFormat::Markdown => write!(f, "markdown"),
            OutputFormat::Latex => write!(f, "latex"),
            OutputFormat::Typst => write!(f, "typst"),
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Render documents or references
    Render {
        #[command(subcommand)]
        command: RenderCommands,
    },

    /// Validate style, bibliography, and citations files
    #[command(
        about = "Validate style, bibliography, and citations files",
        long_about = "Perform schema validation on input files.\n\n\
                      Citum checks the syntax and structure of style (YAML/JSON/CBOR),\n\
                      bibliography (JSON/YAML), and citation files against their\n\
                      respective schemas. Use this to ensure your data is compatible\n\
                      before processing.\n\n\
                      EXAMPLES:\n  \
                      Validate a style and its bibliography:\n    \
                      citum check -s apa-7th -b refs.json\n\n  \
                      Validate and output detailed results as JSON:\n    \
                      citum check -s apa-7th -b refs.json --json"
    )]
    Check(CheckArgs),

    /// Convert styles, references, locales, and citations
    #[command(
        about = "Convert styles, references, locales, and citations",
        long_about = "Convert between native Citum formats and legacy bibliography formats.\n\n\
                      Use subcommands to make conversion intent explicit.\n\n\
                      EXAMPLES:\n  \
                      Convert references from BibLaTeX to native YAML:\n    \
                      citum convert refs refs.bib -o refs.yaml\n\n  \
                      Convert references from native YAML to RIS:\n    \
                      citum convert refs refs.yaml -o refs.ris\n\n  \
                      Convert a style from YAML to binary CBOR:\n    \
                      citum convert style style.yaml -o style.cbor"
    )]
    Convert {
        #[command(subcommand)]
        command: ConvertCommands,
    },

    /// Manage and inspect the style registry
    #[command(
        about = "Manage style registry sources",
        long_about = "Manage style registry sources used for style discovery and resolution.\n\n\
                      Registries map style names and aliases to available styles.\n\n\
                      EXAMPLES:\n  \
                      List configured registries:\n    \
                      citum registry list\n\n  \
                      Add an institutional registry:\n    \
                      citum registry add https://styles.example.org/citum-registry.yaml --name example\n\n  \
                      Resolve a style name or alias:\n    \
                      citum registry resolve apa"
    )]
    Registry {
        #[command(subcommand)]
        command: RegistryCommands,
    },

    /// Find, inspect, install, remove, and lint citation styles
    #[command(
        about = "Find, inspect, install, remove, and lint citation styles",
        long_about = "Work with citation styles by task: search the catalog, inspect a style,\n\
                      install a style, remove an installed style, or lint a style file.\n\n\
                      EXAMPLES:\n  \
                      Search styles:\n    \
                      citum style search chicago\n\n  \
                      Install a style without copying a full ID:\n    \
                      citum style add chicago\n\n  \
                      List embedded styles only:\n    \
                      citum style list --source embedded"
    )]
    Style {
        #[command(subcommand)]
        command: StyleCommands,
    },

    /// List, install, remove, and lint locale files
    #[command(
        about = "List, install, remove, and lint locale files",
        long_about = "Manage installed locales and validate locale authoring files.\n\n\
                      EXAMPLES:\n  \
                      List installed locales:\n    \
                      citum locale list --source installed\n\n  \
                      Install a locale:\n    \
                      citum locale add locales/en-US.yaml\n\n  \
                      Lint a locale:\n    \
                      citum locale lint locales/en-US.yaml"
    )]
    Locale {
        #[command(subcommand)]
        command: LocaleCommands,
    },

    /// Diagnose local Citum configuration, cache, and installed resources
    Doctor {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Generate JSON schema for Citum models
    #[cfg(feature = "schema")]
    Schema(SchemaArgs),

    /// Export language type bindings for Citum schema types
    #[cfg(feature = "typescript")]
    Bindings(BindingsArgs),

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
#[command(
    about = "Render documents or references",
    long_about = "Render documents or references using a specified citation style.\n\n\
                  Citum supports two primary rendering modes:\n\
                  - doc: Process a full document (Djot or Markdown) with integrated citations.\n\
                  - refs: Direct rendering of a bibliography file for debugging\n\
                    or inspection.\n\n\
                  Run 'citum render <COMMAND> --help' for specific examples."
)]
pub(crate) enum RenderCommands {
    /// Render a full document with citations and bibliography
    #[command(
        about = "Render a full document with citations and bibliography",
        long_about = "Process a full document with citations and bibliography.\n\n\
                      Citum parses the input document (default: Djot) for citations,\n\
                      matches them against the provided bibliography, and renders\n\
                      the final output in various formats (Plain, HTML, Latex, etc.).\n\n\
                      EXAMPLES:\n  \
                      Render to HTML:\n    \
                      citum render doc manuscript.djot -b refs.json -s apa-7th -f html\n\n  \
                      Render Markdown with Pandoc-style citations:\n    \
                      citum render doc manuscript.md --input-format markdown -b refs.json -s apa-7th\n\n  \
                      Render to Typst, then PDF (typst CLI required):\n    \
                      citum render doc manuscript.djot -b refs.json -s apa-7th -f typst -o paper.typ\n    \
                      typst compile paper.typ"
    )]
    Doc(RenderDocArgs),

    /// Render references/citations directly
    #[command(
        about = "Render references/citations directly",
        long_about = "Directly render a set of references and/or citations from files.\n\n\
                      This command is useful for inspecting how a style renders\n\
                      specific entries or testing bibliography grouping logic.\n\n\
                      INPUT FORMATS (--bibliography):\n  \
                      The --bibliography flag accepts:\n    \
                      - Citum YAML (.yaml, .yml) — native Citum reference format\n    \
                      - Citum JSON (.json)        — native Citum reference format (auto-detected by content)\n    \
                      - Citum CBOR (.cbor)        — native Citum reference format (binary)\n    \
                      - CSL-JSON (.json)          — legacy CSL-JSON (auto-detected by content)\n    \
                      Use 'citum convert refs' to convert BibLaTeX or RIS files first.\n\n\
                      EXAMPLES:\n  \
                      Render bibliography entries (APA 7th style):\n    \
                      citum render refs -b refs.json -s apa-7th\n\n  \
                      Render specific citations with keys:\n    \
                      citum render refs -b refs.json -s apa-7th -m cite\n    \
                      -k Doe2020,Smith2021\n\n  \
                      Output as JSON with human-readable rendered text:\n    \
                      citum render refs -b refs.json -s apa-7th --json"
    )]
    Refs(RenderRefsArgs),
}

#[derive(Subcommand)]
pub(crate) enum ConvertCommands {
    /// Convert bibliography/reference files
    #[command(
        about = "Convert bibliography/reference files",
        long_about = "Convert bibliography/reference files between formats.\n\n\
                      INPUT FORMATS (--from):\n  \
                      citum-yaml    Citum native YAML (.yaml or .yml)\n  \
                      citum-json    Citum native JSON (.json; content-sniffed when --from is omitted)\n  \
                      citum-cbor    Citum native CBOR (.cbor)\n  \
                      csl-json      Legacy CSL-JSON (.json; content-sniffed when --from is omitted)\n  \
                      biblatex      BibLaTeX .bib file\n  \
                      ris           RIS (.ris) file\n\n\
                      OUTPUT FORMATS (--to):\n  \
                      Same variants as --from. Default output format is citum-yaml.\n\n\
                      EXAMPLES:\n  \
                      Convert BibLaTeX to Citum YAML:\n    \
                      citum convert refs thesis.bib -o refs.yaml\n\n  \
                      Convert RIS to Citum YAML:\n    \
                      citum convert refs export.ris -o refs.yaml\n\n  \
                      Convert CSL-JSON to Citum YAML:\n    \
                      citum convert refs legacy.json --from csl-json -o refs.yaml"
    )]
    Refs(ConvertRefsArgs),
    /// Convert style files between YAML/JSON/CBOR
    Style(ConvertTypedArgs),
    /// Convert citations files between YAML/JSON/CBOR
    Citations(ConvertTypedArgs),
    /// Convert locale files between YAML/JSON/CBOR
    Locale(ConvertTypedArgs),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum RefsFormat {
    #[value(name = "citum-yaml")]
    CitumYaml,
    #[value(name = "citum-json")]
    CitumJson,
    #[value(name = "citum-cbor")]
    CitumCbor,
    #[value(name = "csl-json")]
    CslJson,
    #[value(name = "biblatex")]
    Biblatex,
    #[value(name = "ris")]
    Ris,
}

#[derive(Subcommand)]
pub(crate) enum RegistryCommands {
    /// List available style registries
    #[command(
        about = "List available style registries",
        long_about = "Display available style registries (embedded default,\n\
                      local citum-registry.yaml if present, and configured\n\
                      registry sources)."
    )]
    List {
        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Add a registry source from a local path or HTTP(S) URL
    Add {
        /// Registry YAML path or HTTP(S) URL
        source: String,
        /// Registry name; defaults to the file stem or URL host
        #[arg(long)]
        name: Option<String>,
    },

    /// Remove a configured registry source
    Remove {
        /// Registry name to remove
        name: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Refresh one configured registry or all registries
    Update {
        /// Registry name to refresh
        name: Option<String>,
        /// Refresh all configured registries
        #[arg(long)]
        all: bool,
    },

    /// Resolve a style name or alias to its canonical ID
    #[command(
        about = "Resolve a style name or alias to its canonical ID",
        long_about = "Look up a style by name or alias in the registry.\n\
                      Returns the canonical style ID and source (builtin or path)."
    )]
    Resolve {
        /// Style name or alias to resolve
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum StyleCommands {
    /// List styles in the style catalog
    List {
        /// Catalog source to include: all, embedded, installed, or registry:`<name>`
        #[arg(long, default_value = "all")]
        source: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
        /// Maximum number of rows to print
        #[arg(long)]
        limit: Option<usize>,
        /// Number of matching rows to skip
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Search styles in the style catalog
    Search {
        /// Search query matched against IDs, aliases, titles, descriptions, and fields
        query: String,
        /// Catalog source to include: all, embedded, installed, or registry:`<name>`
        #[arg(long, default_value = "all")]
        source: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
        /// Maximum number of rows to print
        #[arg(long)]
        limit: Option<usize>,
        /// Number of matching rows to skip
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    /// Show details for a style in the style catalog
    Info {
        /// Style ID or alias
        name: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
    },

    /// Browse styles interactively in the terminal
    Browse {
        /// Optional initial search query
        query: Option<String>,
        /// Catalog source to include: all, embedded, installed, or registry:`<name>`
        #[arg(long, default_value = "all")]
        source: String,
    },

    /// Install a style by search query, ID, path, or URL
    Add {
        /// Style search query, ID, path, or URL
        query: String,
        /// Non-interactive: fail on ambiguous queries
        #[arg(long)]
        yes: bool,
    },

    /// Remove an installed style
    Remove {
        /// Installed style ID or alias
        name: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Validate style authoring rules, optionally against a locale file
    #[command(
        about = "Validate style authoring rules",
        long_about = "Validate a style file or installed/builtin style, including locale-driven\n\
                      terms when --locale is provided."
    )]
    Lint(LintStyleArgs),

    /// Print the canonical CIDv1 for a style file or installed/builtin style
    #[command(
        about = "Print the canonical CIDv1 for a style",
        long_about = "Compute and print the CIDv1 (raw codec, sha-256, base32 lower) for a\n\
                      style. The output is the same value the schema layer computes when\n\
                      verifying an extends-pin: parse, then re-serialize to canonical YAML,\n\
                      then hash. So a child style's extends-pin will match this CID.\n\n\
                      Accepts a file path or an installed/builtin style name. Remote URIs\n\
                      (https://, git+https://, cid:) are not resolved here — fetch them\n\
                      first or pass the local cached copy."
    )]
    Cid {
        /// File path or installed/builtin style ID/alias
        target: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
    },

    /// Print a paste-ready extends + extends-pin pair for a parent style
    #[command(
        about = "Print extends + extends-pin for a parent",
        long_about = "Print a paste-ready YAML snippet binding extends: to a stable URI and\n\
                      extends-pin: to the style's canonical CID. The CID matches what the\n\
                      schema layer computes at extends-pin verification time, so the pair\n\
                      will verify out of the box.\n\n\
                      Accepts a file path or an installed/builtin style name; remote URIs\n\
                      (https://, git+https://, cid:) are not resolved here."
    )]
    Pin {
        /// File path or installed/builtin style ID/alias of the parent
        target: String,
        /// Override the URI emitted alongside the pin (default: file:// for local paths,
        /// hub.citum.org placeholder for catalog names)
        #[arg(long)]
        uri: Option<String>,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
    },

    /// Validate a style end-to-end (schema, extends, extends-pin, citum-version)
    #[command(
        about = "Validate a style end-to-end",
        long_about = "Load a style, resolve all extends chains, run schema validation, verify\n\
                      any extends-pin values, and check the citum-version requirement against\n\
                      the running engine. Exits non-zero on the first failure.\n\n\
                      Accepts a file path or an installed/builtin style name."
    )]
    Validate {
        /// File path or installed/builtin style ID/alias
        target: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum StyleCatalogFormat {
    Text,
    Json,
}

impl std::fmt::Display for StyleCatalogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StyleCatalogFormat::Text => write!(f, "text"),
            StyleCatalogFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum LocaleCommands {
    /// List installed or embedded locales
    List {
        /// Locale source to include: all, embedded, or installed
        #[arg(long, default_value = "all")]
        source: String,
        /// Output format
        #[arg(long, value_enum, default_value_t = StyleCatalogFormat::Text)]
        format: StyleCatalogFormat,
    },

    /// Install a locale file
    Add {
        /// Path to the locale file to install
        path: PathBuf,
    },

    /// Remove an installed locale
    Remove {
        /// Installed locale ID
        name: String,
        /// Skip the confirmation prompt
        #[arg(long)]
        yes: bool,
    },

    /// Validate a locale file's message syntax and alias targets
    Lint(LintLocaleArgs),
}

#[derive(Args, Debug)]
pub(crate) struct RenderDocArgs {
    /// Path to input document
    #[arg(index = 1)]
    pub(crate) input: PathBuf,

    /// Style file path or builtin name (apa, mla, ieee, etc.)
    #[arg(short, long, required = true)]
    pub(crate) style: String,

    /// Path(s) to bibliography input files (repeat for multiple)
    #[arg(short, long, required = true, action = ArgAction::Append)]
    pub(crate) bibliography: Vec<PathBuf>,
    /// Path(s) to citations input files (repeat for multiple)
    #[arg(short = 'c', long, action = ArgAction::Append)]
    pub(crate) citations: Vec<PathBuf>,

    /// Input document format
    #[arg(short = 'I', long = "input-format", value_enum, default_value_t = InputFormat::Djot)]
    pub(crate) input_format: InputFormat,

    /// Output format
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = OutputFormat::Plain
    )]
    pub(crate) format: OutputFormat,

    /// Write output to file (defaults to stdout)
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,

    /// Compile Typst output to PDF (not in v1 cargo-install builds; use
    /// `-f typst` then `typst compile`)
    #[arg(long, hide = true)]
    pub(crate) pdf: bool,

    /// Preserve generated Typst source next to the PDF output
    #[arg(long, hide = true)]
    pub(crate) typst_keep_source: bool,

    /// Locale ID (e.g. "de-DE", "fr-FR") to override the style's default locale
    #[arg(short = 'L', long)]
    pub(crate) locale: Option<String>,

    /// Path to JSON file containing ordered bibliography block requests
    /// (`Vec<BibliographyBlockRequest>`). Each block is a group selector with an id.
    #[arg(long = "bibliography-blocks")]
    pub(crate) bibliography_blocks: Option<std::path::PathBuf>,

    /// Disable semantic classes (HTML spans, Djot attributes)
    #[arg(long)]
    pub(crate) no_semantics: bool,
}

#[derive(Args, Debug)]
pub(crate) struct RenderRefsArgs {
    /// Path(s) to bibliography input files (repeat for multiple)
    #[arg(short, long, required = true, action = ArgAction::Append)]
    pub(crate) bibliography: Vec<PathBuf>,

    /// Style file path or builtin name (apa, mla, ieee, etc.)
    #[arg(short, long, required = true)]
    pub(crate) style: String,

    /// Locale ID (e.g. "es-ES", "fr-FR") to override the style's default locale
    #[arg(short = 'L', long)]
    pub(crate) locale: Option<String>,

    /// Path(s) to citations input files (repeat for multiple)
    #[arg(short = 'c', long, action = ArgAction::Append)]
    pub(crate) citations: Vec<PathBuf>,

    /// Render mode
    #[arg(short = 'm', long, value_enum, default_value_t = RenderMode::Both)]
    pub(crate) mode: RenderMode,

    /// Specific reference keys to render (comma-separated)
    #[arg(short = 'k', long, value_delimiter = ',')]
    pub(crate) keys: Option<Vec<String>>,

    /// Show reference keys/IDs in human output
    #[arg(long)]
    pub(crate) show_keys: bool,

    /// Output as JSON
    #[arg(short = 'j', long)]
    pub(crate) json: bool,

    /// Output format
    #[arg(
        short,
        long,
        value_enum,
        default_value_t = OutputFormat::Plain
    )]
    pub(crate) format: OutputFormat,

    /// Write output to file (defaults to stdout)
    #[arg(short = 'o', long)]
    pub(crate) output: Option<PathBuf>,

    /// Disable semantic classes (HTML spans, Djot attributes)
    #[arg(long)]
    pub(crate) no_semantics: bool,

    /// Path to annotations file (JSON or YAML mapping ref IDs to annotation text)
    #[arg(long, value_name = "FILE")]
    pub(crate) annotations: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub(crate) struct CheckArgs {
    /// Style file path or builtin name (apa, mla, ieee, etc.)
    #[arg(short, long)]
    pub(crate) style: Option<String>,

    /// Path(s) to bibliography input files (repeat for multiple)
    #[arg(short, long, action = ArgAction::Append)]
    pub(crate) bibliography: Vec<PathBuf>,

    /// Path(s) to citations input files (repeat for multiple)
    #[arg(short = 'c', long, action = ArgAction::Append)]
    pub(crate) citations: Vec<PathBuf>,

    /// Output as JSON
    #[arg(long)]
    pub(crate) json: bool,

    /// Treat captured forward-compat `unknown_fields` paths as hard errors
    /// instead of warnings. See `docs/specs/FORWARD_COMPATIBILITY.md`.
    #[arg(long)]
    pub(crate) strict: bool,
}

#[derive(Args, Debug)]
pub(crate) struct LintLocaleArgs {
    /// Path to locale file
    #[arg(index = 1)]
    pub(crate) path: PathBuf,
}

#[derive(Args, Debug)]
pub(crate) struct LintStyleArgs {
    /// Style file path or builtin name
    #[arg(index = 1)]
    pub(crate) style: String,

    /// Locale file used for validation
    #[arg(long, required = true)]
    pub(crate) locale: PathBuf,
}

#[cfg(feature = "schema")]
#[derive(Args, Debug)]
pub(crate) struct SchemaArgs {
    /// Data type to export
    #[arg(index = 1, value_enum)]
    pub(crate) r#type: Option<SchemaType>,

    /// Output directory to export all schemas
    #[arg(short, long)]
    pub(crate) out_dir: Option<PathBuf>,
}

#[cfg(feature = "typescript")]
#[derive(Args, Debug)]
pub(crate) struct BindingsArgs {
    /// Output directory for generated type definition files
    #[arg(short, long)]
    pub(crate) out_dir: PathBuf,
}

#[derive(Args, Debug)]
pub(crate) struct ConvertTypedArgs {
    /// Path to input file
    #[arg(index = 1)]
    pub(crate) input: PathBuf,

    /// Path to output file
    #[arg(short = 'o', long)]
    pub(crate) output: PathBuf,
}

#[derive(Args, Debug)]
pub(crate) struct ConvertRefsArgs {
    /// Path to input bibliography file
    #[arg(index = 1)]
    pub(crate) input: PathBuf,

    /// Path to output bibliography file
    #[arg(short = 'o', long)]
    pub(crate) output: PathBuf,

    /// Input format (auto-detected from extension; .json inputs are content-sniffed to distinguish citum-json from csl-json)
    #[arg(long, value_enum)]
    pub(crate) from: Option<RefsFormat>,

    /// Output format (auto-detected from extension if omitted; defaults to citum-yaml)
    #[arg(long, value_enum)]
    pub(crate) to: Option<RefsFormat>,
}

#[derive(Args, Debug)]
pub(crate) struct LegacyDocArgs {
    /// Path to the document file
    #[arg(index = 1)]
    pub(crate) document: PathBuf,

    /// Path to the references file
    #[arg(index = 2)]
    pub(crate) references: PathBuf,

    /// Path to the style file
    #[arg(index = 3)]
    pub(crate) style: PathBuf,

    /// Output format
    #[arg(short = 'f', long, value_enum, default_value_t = OutputFormat::Plain)]
    pub(crate) format: OutputFormat,
}

#[derive(Args, Debug)]
pub(crate) struct LegacyValidateArgs {
    /// Path to style file
    pub(crate) path: PathBuf,
}

#[derive(Serialize)]
pub(crate) struct CheckItem {
    pub(crate) kind: &'static str,
    pub(crate) path: String,
    pub(crate) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) warnings: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}
