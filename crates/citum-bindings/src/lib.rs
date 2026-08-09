/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citum cross-language bindings.
//!
//! Exposes a minimal, stable public API over `citum-engine` for use from
//! JavaScript/WASM and other languages via cdylib. No internal engine types
//! cross the public boundary — all inputs and outputs are plain strings.
//!
//! # WASM
//!
//! Enable the `wasm` feature to compile with `wasm-bindgen` annotations:
//!
//! ```toml
//! citum-bindings = { features = ["wasm"] }
//! ```
//!
//! # API
//!
//! Three functions cover the common citation management use cases:
//!
//! - [`render_citation`] — render a single citation
//! - [`render_bibliography`] — render a full bibliography
//! - [`validate_style`] — check a style for parse/schema errors

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use citum_engine::processor::Processor;
use citum_engine::render::html::Html as HtmlRenderer;
use citum_engine::{Citation, Reference};
#[cfg(feature = "wasm")]
use citum_engine::{
    CitationOccurrence, CitationOccurrenceItem, DocumentSession, OutputFormatKind, RefsInput,
    StyleInput,
};
use citum_schema::{CitationSpec, Style, TemplatePreset};
use indexmap::IndexMap;
use serde_json::Value;

/// Parse a Citum YAML style string, returning a structured error on failure.
fn parse_style(style_yaml: &str) -> Result<Style, String> {
    Style::from_yaml_str(style_yaml).map_err(|e| format!("Style parse error: {e}"))
}

/// Parse a single reference `Value`, upgrading legacy CSL-JSON to the Citum schema when possible.
fn parse_single_reference_value(
    key: &str,
    val: Value,
) -> Result<citum_schema::reference::InputReference, String> {
    if let Ok(r) = serde_json::from_value::<citum_schema::reference::InputReference>(val.clone()) {
        return Ok(r);
    }

    #[cfg(feature = "legacy-convert")]
    {
        if let Ok(legacy) = serde_json::from_value::<csl_legacy::csl_json::Reference>(val) {
            let r: citum_schema::reference::InputReference = legacy.into();
            return Ok(r);
        }
    }

    #[cfg(not(feature = "legacy-convert"))]
    {
        Err(format!("Failed to parse reference '{key}'"))
    }

    #[cfg(feature = "legacy-convert")]
    {
        Err(format!(
            "Failed to parse reference '{key}' as InputReference or CSL-JSON"
        ))
    }
}

/// Parse references JSON, accepting either an object map or an array, upgrading
/// legacy CSL-JSON to the Citum schema when possible.
fn parse_references(refs_json: &str) -> Result<IndexMap<String, Reference>, String> {
    let json_value: Value =
        serde_json::from_str(refs_json).map_err(|e| format!("Invalid JSON for references: {e}"))?;

    let mut mapped: IndexMap<String, Reference> = IndexMap::new();

    match json_value {
        Value::Object(obj) => {
            for (key, val) in obj {
                let r = parse_single_reference_value(&key, val)?;
                mapped.insert(key, r);
            }
        }
        Value::Array(arr) => {
            for (idx, val) in arr.into_iter().enumerate() {
                let Some(obj) = val.as_object() else {
                    return Err(format!(
                        "Failed to parse reference at index {idx}: expected an object"
                    ));
                };

                let id = match obj.get("id").and_then(|v| v.as_str()) {
                    Some(s) => s.to_string(),
                    None => {
                        return Err(format!(
                            "Failed to parse reference at index {}: missing string 'id' field",
                            idx
                        ));
                    }
                };

                // Use the entire element value for parsing, keyed by its `id`.
                let r = parse_single_reference_value(&id, Value::Object(obj.clone()))?;
                mapped.insert(id.clone(), r);
            }
        }
        _ => {
            return Err(
                "Invalid JSON for references: expected an object map or an array of objects"
                    .to_string(),
            );
        }
    }
    Ok(mapped)
}

/// Ensure a style has a materialized citation template suitable for preview rendering.
///
/// Supplies the APA citation preset when the citation section is absent and
/// forces a locator into the citation template when missing. An absent
/// bibliography remains absent so citation-only styles keep their authored
/// contract.
pub fn ensure_style_has_templates(style: &mut Style) {
    if style.citation.is_none() {
        style.citation = Some(CitationSpec {
            template_ref: Some(TemplatePreset::Apa.into()),
            ..Default::default()
        });
    }

    if let Some(ref mut citation) = style.citation {
        use citum_schema::template::{
            Rendering, SimpleVariable, TemplateComponent, TemplateVariable,
        };
        let mut template = citation.resolve_template().unwrap_or_default();
        let has_locator = template.iter().any(|c| {
            matches!(c, TemplateComponent::Variable(v) if v.variable == SimpleVariable::Locator)
        });
        if !has_locator {
            template.push(TemplateComponent::Variable(TemplateVariable {
                variable: SimpleVariable::Locator,
                rendering: Rendering {
                    prefix: Some(", ".into()),
                    ..Default::default()
                },
                ..Default::default()
            }));
            citation.template = Some(template.into());
            citation.template_ref = None;
        }
    }
}

/// Extract the `info` block from a YAML style string as JSON.
///
/// # Errors
///
/// Returns a string error if the YAML fails to parse or the info block cannot
/// be serialized to JSON.
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "getStyleMetadata"))]
pub fn get_style_metadata(style_yaml: &str) -> Result<String, String> {
    let style = parse_style(style_yaml)?;
    serde_json::to_string(&style.info).map_err(|e| format!("Serialization error: {e}"))
}

/// Materialize all template presets in a style and return the updated YAML.
///
/// # Errors
///
/// Returns a string error if the input YAML fails to parse or the materialized
/// style cannot be serialized back to YAML.
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "materializeStyle"))]
pub fn materialize_style(style_yaml: &str) -> Result<String, String> {
    let mut style = parse_style(style_yaml)?;
    ensure_style_has_templates(&mut style);
    use serde_yaml;
    serde_yaml::to_string(&style).map_err(|e| format!("YAML serialization error: {e}"))
}

/// Render a single citation to HTML.
///
/// - `style_yaml` — Citum style as YAML
/// - `refs_json` — bibliography as JSON object (`{id: Reference}`) or CSL-JSON array
/// - `citation_json` — a single [`Citation`] as JSON
/// - `mode` — optional mode override (e.g. `"Integral"`)
///
/// # Errors
///
/// Returns a string error on style/reference/citation parse failure, invalid
/// mode string, or engine rendering error.
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "renderCitation"))]
pub fn render_citation(
    style_yaml: &str,
    refs_json: &str,
    citation_json: &str,
    mode: Option<String>,
) -> Result<String, String> {
    let mut style = parse_style(style_yaml)?;
    ensure_style_has_templates(&mut style);
    let refs = parse_references(refs_json)?;
    let mut citation: Citation =
        serde_json::from_str(citation_json).map_err(|e| format!("Citation parse error: {e}"))?;
    if let Some(m) = mode {
        let m_enum =
            serde_json::from_str::<citum_schema::citation::CitationMode>(&format!("\"{m}\""))
                .map_err(|e| format!("Invalid citation mode '{m}': {e}"))?;
        citation.mode = m_enum;
    }
    let processor = Processor::new(style, refs);
    let mut run = processor.begin_run();
    processor
        .process_citation_with_format::<HtmlRenderer>(&citation, &mut run)
        .map_err(|e| format!("Render error: {e}"))
}

/// Render a full bibliography to HTML.
///
/// - `style_yaml` — Citum style as YAML
/// - `refs_json` — bibliography as JSON object or CSL-JSON array
///
/// # Errors
///
/// Returns a string error on style or reference parse failure.
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "renderBibliography"))]
pub fn render_bibliography(style_yaml: &str, refs_json: &str) -> Result<String, String> {
    let mut style = parse_style(style_yaml)?;
    ensure_style_has_templates(&mut style);
    let refs = parse_references(refs_json)?;
    let processor = Processor::new(style, refs);
    Ok(processor.render_bibliography_with_format_standalone::<HtmlRenderer>())
}

/// Validate a Citum style string.
///
/// # Errors
///
/// Returns a string error describing the parse or schema validation failure.
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "validateStyle"))]
pub fn validate_style(style_yaml: &str) -> Result<(), String> {
    Style::from_yaml_str(style_yaml)
        .map(|_| ())
        .map_err(|e| format!("Style parse error: {e}"))
}

/// Format a complete document's citations and bibliography in one call.
///
/// Takes a JSON-encoded `FormatDocumentRequest` and returns a JSON-encoded
/// `FormatDocumentResult`. In WASM, the resolver chain is unavailable —
/// `StyleInput::Id` and `StyleInput::Uri` variants return an error; use
/// `StyleInput::Yaml` (preferred) or `StyleInput::Path` (if filesystem
/// access is available in the host).
///
/// # Errors
///
/// Returns a string error on request JSON parse failure, style resolution failure,
/// or engine rendering error.
#[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "formatDocument"))]
pub fn format_document(request_json: &str) -> Result<String, String> {
    let request: citum_engine::FormatDocumentRequest =
        serde_json::from_str(request_json).map_err(|e| format!("Invalid request JSON: {}", e))?;
    let result =
        citum_engine::format_document(request).map_err(|e| format!("Format error: {}", e))?;
    serde_json::to_string(&result).map_err(|e| format!("Result serialization failed: {}", e))
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "DocumentSession")]
/// WASM wrapper around the engine's stateful document session.
pub struct WasmDocumentSession {
    inner: DocumentSession,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen(js_class = "DocumentSession")]
impl WasmDocumentSession {
    /// Create a stateful document session from style YAML and optional refs JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the style YAML or refs JSON cannot be parsed.
    #[wasm_bindgen(constructor)]
    pub fn new(style_yaml: &str, refs_json: Option<String>) -> Result<WasmDocumentSession, String> {
        let mut style = parse_style(style_yaml)?;
        ensure_style_has_templates(&mut style);
        let mut inner = DocumentSession::new(
            style,
            StyleInput::Yaml(style_yaml.to_string()),
            None,
            OutputFormatKind::Html,
            None,
        );
        if let Some(refs_json) = refs_json {
            inner
                .put_references(refs_input_from_json(&refs_json)?)
                .map_err(|e| e.to_string())?;
        }
        Ok(Self { inner })
    }

    /// Replace the session's full reference set from a JSON object.
    ///
    /// # Errors
    ///
    /// Returns an error if the refs JSON cannot be parsed.
    pub fn put_references(&mut self, refs_json: &str) -> Result<(), String> {
        self.inner
            .put_references(refs_input_from_json(refs_json)?)
            .map_err(|e| e.to_string())
    }

    /// Replace the full ordered citation list and return a session mutation result JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if citations JSON is invalid, rendering fails, or the
    /// result cannot be serialized.
    pub fn insert_citations_batch(&mut self, citations_json: &str) -> Result<String, String> {
        let citations: Vec<CitationOccurrence> = serde_json::from_str(citations_json)
            .map_err(|e| format!("Invalid citations JSON: {e}"))?;
        let result = self
            .inner
            .insert_citations_batch(citations)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result).map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Insert one citation and return a session mutation result JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if request JSON is invalid, rendering fails, or the
    /// result cannot be serialized.
    pub fn insert_citation(
        &mut self,
        citation_json: &str,
        position_json: Option<String>,
    ) -> Result<String, String> {
        let citation = parse_json(citation_json, "citation")?;
        let position = parse_optional_json(position_json, "position")?;
        let result = self
            .inner
            .insert_citation(citation, position)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result).map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Update one citation and return a session mutation result JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if request JSON is invalid, the citation is missing,
    /// rendering fails, or the result cannot be serialized.
    pub fn update_citation(
        &mut self,
        citation_id: &str,
        citation_json: &str,
        position_json: Option<String>,
    ) -> Result<String, String> {
        let citation = parse_json(citation_json, "citation")?;
        let position = parse_optional_json(position_json, "position")?;
        let result = self
            .inner
            .update_citation(citation_id, citation, position)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result).map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Delete one citation and return a session mutation result JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the citation is missing, rendering fails, or the
    /// result cannot be serialized.
    pub fn delete_citation(&mut self, citation_id: &str) -> Result<String, String> {
        let result = self
            .inner
            .delete_citation(citation_id)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result).map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Render a citation preview without mutating session state.
    ///
    /// # Errors
    ///
    /// Returns an error if request JSON is invalid, rendering fails, or the
    /// result cannot be serialized.
    pub fn preview_citation(
        &self,
        items_json: &str,
        position_json: Option<String>,
    ) -> Result<String, String> {
        let items: Vec<CitationOccurrenceItem> = parse_json(items_json, "items")?;
        let position = parse_optional_json(position_json, "position")?;
        let result = self
            .inner
            .preview_citation(items, None, position)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result).map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Return the current formatted citations as a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the result cannot be serialized.
    pub fn get_citations(&self) -> Result<String, String> {
        serde_json::to_string(&serde_json::json!({
            "formatted_citations": self.inner.get_citations()
        }))
        .map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Return the current bibliography as a JSON string.
    ///
    /// # Errors
    ///
    /// Returns an error if the result cannot be serialized.
    pub fn get_bibliography(&self) -> Result<String, String> {
        serde_json::to_string(&serde_json::json!({
            "bibliography": self.inner.get_bibliography()
        }))
        .map_err(|e| format!("Result serialization failed: {e}"))
    }

    /// Explicitly dispose of the session object.
    pub fn dispose(self) {}
}

#[cfg(feature = "wasm")]
fn refs_input_from_json(refs_json: &str) -> Result<RefsInput, String> {
    let refs = serde_json::from_str(refs_json).map_err(|e| format!("Invalid refs JSON: {e}"))?;
    Ok(RefsInput::Json(refs))
}

#[cfg(feature = "wasm")]
fn parse_json<T>(json: &str, label: &str) -> Result<T, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    serde_json::from_str(json).map_err(|e| format!("Invalid {label} JSON: {e}"))
}

#[cfg(feature = "wasm")]
fn parse_optional_json<T>(json: Option<String>, label: &str) -> Result<Option<T>, String>
where
    T: for<'de> serde::Deserialize<'de>,
{
    json.map(|value| parse_json(&value, label)).transpose()
}

/// Export all Citum schema types as TypeScript type definitions to a file.
///
/// Writes a `.d.ts` file to `out_path` containing type definitions for all
/// annotated public schema types (references, citations, bibliography input).
///
/// Typically called via `citum bindings --out-dir <dir>`, which writes
/// to `<out-dir>/citum.d.ts`.
///
/// # Errors
///
/// Returns a [`specta_typescript::Error`] if the output file cannot be written
/// or a type registration is invalid.
#[cfg(feature = "typescript")]
#[cfg(not(target_arch = "wasm32"))]
pub fn export_typescript(out_path: &std::path::Path) -> Result<(), specta_typescript::Error> {
    use citum_schema::citation::{
        CitationItem, CitationLocator, CitationMode, LocatorType, LocatorValue,
    };
    use citum_schema::reference::{Contributor, InputReference};
    use citum_schema::{Citation, InputBibliography};
    use specta::TypeCollection;
    use specta_typescript::Typescript;

    let types = TypeCollection::default()
        .register::<InputReference>()
        .register::<InputBibliography>()
        .register::<Citation>()
        .register::<CitationItem>()
        .register::<CitationLocator>()
        .register::<CitationMode>()
        .register::<LocatorType>()
        .register::<LocatorValue>()
        .register::<Contributor>();

    Typescript::default().export_to(out_path, &types)
}
