/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! `convert` subcommands: typed (style/locale/citations) and refs.

use super::CliResult;
use crate::args::{ConvertCommands, ConvertRefsArgs, ConvertTypedArgs, DataType};
use citum_io::{
    infer_refs_input_format as infer_engine_refs_input_format,
    infer_refs_output_format as infer_engine_refs_output_format, load_input_bibliography,
    write_output_bibliography,
};
use citum_schema::locale::RawLocale;
use citum_schema::{Style, StyleDocumentFormat};
use serde::Serialize;
use std::error::Error;
use std::fs;

pub(super) fn dispatch(command: ConvertCommands) -> CliResult {
    match command {
        ConvertCommands::Refs(args) => run_convert_refs(args),
        ConvertCommands::Style(args) => run_convert_typed(args, DataType::Style),
        ConvertCommands::Citations(args) => run_convert_typed(args, DataType::Citations),
        ConvertCommands::Locale(args) => run_convert_typed(args, DataType::Locale),
    }
}

/// Execute typed conversion subcommands (`style`, `locale`, `citations`).
fn run_convert_typed(args: ConvertTypedArgs, data_type: DataType) -> CliResult {
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

    match data_type {
        DataType::Style => {
            let doc_format = match input_ext {
                "json" => StyleDocumentFormat::Json,
                "cbor" => StyleDocumentFormat::Cbor,
                _ => StyleDocumentFormat::Yaml,
            };
            let style = Style::from_document_bytes(&input_bytes, doc_format)?;
            let out_bytes = serialize_any(&style, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
        DataType::Locale => {
            let locale: RawLocale = deserialize_any(&input_bytes, input_ext)?;
            let out_bytes = serialize_any(&locale, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
        DataType::Citations => {
            let citations: citum_schema::citation::Citations =
                deserialize_any(&input_bytes, input_ext)?;
            let out_bytes = serialize_any(&citations, output_ext)?;
            fs::write(&args.output, out_bytes)?;
        }
        DataType::Bib => {
            return Err("`convert bib` was replaced by `convert refs`.".into());
        }
    }

    println!(
        "Converted {} to {}",
        args.input.display(),
        args.output.display()
    );
    Ok(())
}

/// Execute `convert refs` for native and legacy bibliography formats.
fn run_convert_refs(args: ConvertRefsArgs) -> CliResult {
    let input_format = if let Some(f) = args.from {
        f.into()
    } else {
        infer_engine_refs_input_format(&args.input)?
    };
    let output_format = args
        .to
        .map(Into::into)
        .unwrap_or_else(|| infer_engine_refs_output_format(&args.output));

    let bibliography = load_input_bibliography(&args.input, input_format)?;
    write_output_bibliography(&bibliography, &args.output, output_format)?;

    println!(
        "Converted {} ({:?}) to {} ({:?})",
        args.input.display(),
        input_format,
        args.output.display(),
        output_format
    );
    Ok(())
}

/// Deserialise bytes into `T` using the format indicated by `ext`.
///
/// Recognised extensions: `yaml` / `yml`, `json`, `cbor`.  All other values
/// fall back to YAML.
fn deserialize_any<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    ext: &str,
) -> Result<T, Box<dyn Error>> {
    match ext {
        "yaml" | "yml" => Ok(serde_yaml::from_slice(bytes)?),
        "json" => Ok(serde_json::from_slice(bytes)?),
        "cbor" => Ok(ciborium::de::from_reader(std::io::Cursor::new(bytes))?),
        _ => Ok(serde_yaml::from_slice(bytes)?),
    }
}

/// Serialise `obj` to bytes using the format indicated by `ext`.
///
/// Recognised extensions: `yaml` / `yml` (default), `json`, `cbor`.
fn serialize_any<T: Serialize>(obj: &T, ext: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    match ext {
        "yaml" | "yml" => Ok(serde_yaml::to_string(obj)?.into_bytes()),
        "json" => Ok(serde_json::to_string_pretty(obj)?.into_bytes()),
        "cbor" => {
            let mut buf = Vec::new();
            ciborium::ser::into_writer(obj, &mut buf)?;
            Ok(buf)
        }
        _ => Ok(serde_yaml::to_string(obj)?.into_bytes()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "tests")]
mod tests {
    use super::*;
    use citum_io::RefsFormat as EngineRefsFormat;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_infer_refs_output_format_yaml() {
        assert!(matches!(
            infer_engine_refs_output_format(Path::new("refs.yaml")),
            EngineRefsFormat::CitumYaml
        ));
    }

    #[test]
    fn test_infer_refs_output_format_bib() {
        assert!(matches!(
            infer_engine_refs_output_format(Path::new("refs.bib")),
            EngineRefsFormat::Biblatex
        ));
    }

    #[test]
    fn test_infer_refs_output_format_ris() {
        assert!(matches!(
            infer_engine_refs_output_format(Path::new("refs.ris")),
            EngineRefsFormat::Ris
        ));
    }

    #[test]
    fn test_load_citum_json_bibliography_keeps_standalone_edited_book_as_book_type() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/references-humanities-note.json");

        let bibliography = load_input_bibliography(&fixture, EngineRefsFormat::CitumJson)
            .expect("fixture should parse as native Citum");
        let edited_book = bibliography
            .references
            .iter()
            .find(|reference| reference.id().as_deref() == Some("burke2010-ed"))
            .expect("edited book fixture should exist");

        assert_eq!(edited_book.ref_type(), "book");
    }

    #[test]
    fn test_load_citum_json_bibliography_preserves_hybrid_edited_book_url() {
        let bytes = br#"[
  {
    "id": "edited-book-1",
    "class": "collection",
    "type": "edited-book",
    "title": "Edited Book",
    "editor": [{"family": "Miller", "given": "Ruth"}],
    "issued": {"date-parts": [[2022]]},
    "publisher": "Example Press",
    "publisher-place": "Chicago",
    "URL": "https://example.com/edited-book"
  }
]"#;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("citum-hybrid-json-{now}.json"));
        std::fs::write(&path, bytes).expect("hybrid fixture should write");

        let bibliography = load_input_bibliography(&path, EngineRefsFormat::CitumJson)
            .expect("hybrid JSON should parse as native Citum");
        let edited_book = bibliography
            .references
            .iter()
            .find(|reference| reference.id().as_deref() == Some("edited-book-1"))
            .expect("edited book fixture should exist");

        assert_eq!(
            edited_book.url().as_ref().map(url::Url::as_str),
            Some("https://example.com/edited-book")
        );

        let _ = std::fs::remove_file(path);
    }
}
