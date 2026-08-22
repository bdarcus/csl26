/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Document-level batch formatting API.

use crate::api::AnnotationStyle;
use crate::error::ProcessorError;
use crate::processor::Processor;
use crate::reference::Citation;
use crate::render::djot::Djot;
use crate::render::format::OutputFormat;
use crate::render::html::Html;
use crate::render::latex::Latex;
use crate::render::markdown::Markdown;
use crate::render::plain::PlainText;
use crate::render::typst::Typst;
use citum_schema::Style;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::warnings::{
    bibliography_label_missing_separator_warnings, term_locale_fallback_warnings,
    unknown_enum_warnings, unknown_reference_class_warnings, unknown_reference_field_warnings,
};
use super::{
    BibliographyEntry, CitationOccurrence, DocumentOptions, EntryMetadata, FormattedBibliography,
    FormattedBibliographyBlock, FormattedCitation, OutputFormatKind, RefsInput, StyleInput,
    Warning, WarningLevel,
};

/// A request to format a complete document's citations and bibliography.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDocumentRequest {
    /// The style to use (may be resolved locally or by an adapter).
    pub style: StyleInput,
    /// Optional partial-style overlay (YAML or JSON) merged over the resolved base
    /// style for this request only.
    ///
    /// Accepts any subset of the style YAML schema — e.g. just `options.contributors`
    /// to change `and`/et-al behaviour, or a full citation spec. Uses the same
    /// null-aware, typed-merge semantics as `extends` inheritance: supplied fields
    /// win over base style fields; an explicit `~` (null) value clears an inherited
    /// field. The base style is never mutated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_overrides: Option<String>,
    /// Optional locale override as a BCP 47 language tag (e.g. `en-US`).
    /// When omitted or set to en-US the engine uses its built-in en-US locale;
    /// other locales emit a warning and fall back to en-US until adapter-side
    /// locale resolution is wired through.
    pub locale: Option<String>,
    /// Output format (plain, html, djot, latex, typst). Defaults to plain
    /// when omitted from the request.
    #[serde(default)]
    pub output_format: OutputFormatKind,
    /// Reference input as a local path, inline YAML, inline JSON, or legacy bare map.
    pub refs: RefsInput,
    /// Ordered citations as they appear in the document.
    pub citations: Vec<CitationOccurrence>,
    /// Ordered sectional bibliography blocks to render after citations.
    #[serde(default)]
    pub bibliography_blocks: Vec<super::BibliographyBlockRequest>,
    /// Optional document-level configuration.
    pub document_options: Option<DocumentOptions>,
    /// Reference IDs to include in the bibliography without emitting an in-text citation.
    ///
    /// Nocite entries appear in `bibliography.entries` (and match `CitedStatus::Visible`
    /// selectors for grouped / block bibliographies) but produce no `formatted_citations`
    /// entry. This matches standard citeproc / Pandoc `nocite` semantics.
    ///
    /// IDs absent from `refs` are ignored and trigger a `nocite_missing_ref` warning.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nocite: Vec<String>,
}

/// The result of formatting a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatDocumentResult {
    /// Formatted citations in document order.
    pub formatted_citations: Vec<FormattedCitation>,
    /// Formatted bibliography.
    pub bibliography: FormattedBibliography,
    /// Rendered bibliography blocks, in request order.
    pub bibliography_blocks: Vec<FormattedBibliographyBlock>,
    /// Non-fatal warnings encountered during processing.
    pub warnings: Vec<Warning>,
}

/// Errors that can occur during document formatting.
#[derive(thiserror::Error, Debug)]
pub enum FormatDocumentError {
    /// The style ID or URI requires a resolver chain not available in the engine.
    #[error("Unresolved style input: {0}")]
    UnresolvedInput(String),
    /// Failed to parse the style YAML.
    #[error("Style parse error: {0}")]
    StyleParse(String),
    /// Failed to read or locate the style file.
    #[error("Style path error: {0}")]
    StylePath(String),
    /// Failed to read a local refs input path.
    #[error("Refs input path error: {0}")]
    RefsInputPath(String),
    /// Failed to parse refs input data.
    #[error("Refs input parse error: {0}")]
    RefsInputParse(String),
    /// The processor encountered an error during rendering.
    #[error("Processing error: {0}")]
    Processing(#[from] ProcessorError),
    /// Style inheritance (`extends`) could not be resolved.
    #[error("Style resolution error: {0}")]
    StyleResolution(String),
}

/// Parse a partial-style overlay (YAML or JSON) and merge it over `style` in place.
///
/// Called internally by `format_document_with_style`; also available to surface crates
/// (e.g. `citum-server`) that pre-resolve the style before handing it to the processor.
///
/// Uses the same null-aware, typed-merge semantics as `extends` inheritance.
/// Calls `apply_scoped_options` after the merge so that overlay fields that affect
/// structural scoped options (date_position, repeated_author_rendering, etc.) take
/// effect in the same way they do during normal style resolution.
///
/// # Errors
///
/// Returns `FormatDocumentError::StyleParse` if the overlay cannot be parsed.
pub fn apply_style_overrides(
    style: &mut Style,
    overlay_src: &str,
) -> Result<(), FormatDocumentError> {
    let overlay = Style::from_yaml_bytes(overlay_src.as_bytes()).map_err(|e| {
        FormatDocumentError::StyleParse(format!("Failed to parse style_overrides: {e}"))
    })?;
    style.apply_overlay(&overlay);
    style.apply_scoped_options();
    Ok(())
}

/// Format a complete document's citations and bibliography (convenience wrapper).
///
/// This function resolves the style locally using `StyleInput::resolve_local`.
/// For styles requiring a resolver chain (Id or Uri), use `format_document_with_style`
/// after pre-resolving.
///
/// # Errors
///
/// Returns an error if the style cannot be resolved, parsed, or if rendering fails.
pub fn format_document(
    request: FormatDocumentRequest,
) -> Result<FormatDocumentResult, FormatDocumentError> {
    let style = request.style.resolve_local()?;
    format_document_with_style(style, request)
}

/// Format a document, resolving the style through an injected resolver.
///
/// `Yaml` is parsed inline; `Id`, `Uri`, and `Path` are delegated to
/// `resolver.resolve_style`. This lets WASM/FFI callers supply their own
/// resolver chain without pre-resolving the style themselves.
///
/// # Errors
///
/// Returns an error if the resolver fails, the style cannot be parsed, or
/// if rendering fails.
pub fn format_document_with_resolver(
    request: FormatDocumentRequest,
    resolver: &citum_schema::StyleResolver,
) -> Result<FormatDocumentResult, FormatDocumentError> {
    let style = match &request.style {
        StyleInput::Yaml(_) => request.style.resolve_local()?,
        StyleInput::Id(value) | StyleInput::Uri(value) | StyleInput::Path(value) => resolver
            .resolve_style(value)
            .map_err(|e| FormatDocumentError::UnresolvedInput(e.to_string()))?,
    };
    // Fully resolve any `extends` chain via the injected resolver, then clear
    // `extends` so the processor's later `into_resolved()` call needs no
    // resolver. Mirrors `citum-server`'s `load_style`.
    let mut resolved = style
        .try_into_resolved_with(Some(resolver))
        .map_err(|e| FormatDocumentError::StyleResolution(e.to_string()))?;
    resolved.extends = None;
    format_document_with_style(resolved, request)
}

/// Format a document using an already-resolved style.
///
/// This is the primary entry point for adapters (citum-server, citum-bindings)
/// that have a resolver chain and can pre-resolve style IDs and URIs.
///
/// # Errors
///
/// Returns an error if rendering fails.
#[allow(
    clippy::too_many_lines,
    reason = "match arms grow one-to-one with format variants"
)]
pub fn format_document_with_style(
    style: Style,
    request: FormatDocumentRequest,
) -> Result<FormatDocumentResult, FormatDocumentError> {
    let mut warnings = Vec::new();

    // Apply per-request style overrides (merge over the resolved base style).
    let mut style = style;
    if let Some(src) = &request.style_overrides {
        apply_style_overrides(&mut style, src)?;
    }

    // Locale: the engine has no resolver chain for non-en-US locales.
    // Adapters with a citum_store dep can pre-resolve and call
    // Processor::with_locale directly; for now, emit a warning when a
    // non-en-US tag is requested and fall back to en-US.
    if let Some(tag) = &request.locale
        && !tag.is_empty()
        && !tag.eq_ignore_ascii_case("en-us")
    {
        warnings.push(Warning {
            level: WarningLevel::Warning,
            code: "locale_fallback".to_string(),
            citation_id: None,
            ref_id: None,
            message: format!(
                "Requested locale '{tag}' could not be loaded by the engine; falling back to en-US. Adapter-side locale resolution is not yet wired through."
            ),
        });
    }

    let bibliography = request.refs.resolve_local()?;
    let mut processor = Processor::new(style, bibliography);
    warnings.extend(unknown_reference_class_warnings(&processor.bibliography));
    warnings.extend(unknown_reference_field_warnings(&processor.bibliography));
    warnings.extend(unknown_enum_warnings(&processor));
    warnings.extend(term_locale_fallback_warnings(&processor));
    warnings.extend(bibliography_label_missing_separator_warnings(&processor));

    if let Some(opts) = &request.document_options {
        // Rebuild the processor with the document-level integral-name override
        // before applying scalar field mutations (show_semantics etc.) so that
        // those mutations are not lost when the processor is reconstructed.
        if let Some(new_proc) = processor
            .processor_with_document_integral_name_override(opts.integral_name_memory.as_ref())
        {
            processor = new_proc;
        }
        if let Some(show_semantics) = opts.show_semantics {
            processor.show_semantics = show_semantics;
        }
        if let Some(inject_ast) = opts.inject_ast_indices {
            processor.set_inject_ast_indices(inject_ast);
        }
        if let Some(abbr_map) = opts.abbreviation_map.clone() {
            processor.abbreviation_map = Some(abbr_map);
        }
    }

    // Convert citations, recording missing-ref warnings and dropping items
    // whose reference IDs are absent from the bibliography. Citations with no
    // surviving items are kept as empty placeholders so the output preserves
    // input order and length.
    let mut citations: Vec<Citation> = Vec::new();
    for occ in request.citations {
        let mut citation: Citation = occ.into();
        citation.items.retain(|item| {
            if processor.bibliography.contains_key(&item.id) {
                true
            } else {
                warnings.push(Warning {
                    level: WarningLevel::Warning,
                    code: "missing_ref".to_string(),
                    citation_id: citation.id.clone(),
                    ref_id: Some(item.id.clone()),
                    message: format!("Reference '{}' not found in bibliography", item.id),
                });
                false
            }
        });
        citations.push(citation);
    }

    // Annotate integral-name First/Subsequent state from the processor's
    // effective config (no document structure available; all citations share
    // document scope). Safe no-op when no memory config is present.
    processor.annotate_flat_integral_name_states(&mut citations);

    // One run threads registration (citation numbers, cite-order, dynamic
    // compound groups, first-note tracking) across citations and nocite
    // registration below, then finalizes once for bibliography rendering.
    let mut run = processor.begin_run();

    // Process citations
    let formatted_citations = match request.output_format {
        OutputFormatKind::Plain => format_by_kind::<PlainText>(&processor, &citations, &mut run)?,
        OutputFormatKind::Html => format_by_kind::<Html>(&processor, &citations, &mut run)?,
        OutputFormatKind::Djot => format_by_kind::<Djot>(&processor, &citations, &mut run)?,
        OutputFormatKind::Latex => format_by_kind::<Latex>(&processor, &citations, &mut run)?,
        OutputFormatKind::Typst => format_by_kind::<Typst>(&processor, &citations, &mut run)?,
        OutputFormatKind::Markdown => format_by_kind::<Markdown>(&processor, &citations, &mut run)?,
    };

    // Register nocite IDs: validate against bibliography, warn on missing, then add
    // to cited_ids so they appear in bibliography.entries but produce no citation text.
    let nocite_ids: Vec<String> = request
        .nocite
        .iter()
        .filter_map(|id| {
            if processor.bibliography.contains_key(id) {
                Some(id.clone())
            } else {
                warnings.push(Warning {
                    level: WarningLevel::Warning,
                    code: "nocite_missing_ref".to_string(),
                    citation_id: None,
                    ref_id: Some(id.clone()),
                    message: format!("Nocite reference '{id}' not found in bibliography"),
                });
                None
            }
        })
        .collect();
    processor.register_nocite_ids(nocite_ids, &mut run);

    // Registration is complete: finalize so bibliography rendering below sees
    // the full cite-order state from every citation processed above.
    let run = run.finalize();

    // Process bibliography
    let bibliography = match request.output_format {
        OutputFormatKind::Plain => format_bibliography::<PlainText>(
            &processor,
            request.output_format,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Html => format_bibliography::<Html>(
            &processor,
            request.output_format,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Djot => format_bibliography::<Djot>(
            &processor,
            request.output_format,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Latex => format_bibliography::<Latex>(
            &processor,
            request.output_format,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Typst => format_bibliography::<Typst>(
            &processor,
            request.output_format,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Markdown => format_bibliography::<Markdown>(
            &processor,
            request.output_format,
            request.document_options.as_ref(),
            &run,
        )?,
    };

    // Process bibliography blocks
    let bibliography_blocks = match request.output_format {
        OutputFormatKind::Plain => format_bibliography_blocks::<PlainText>(
            &processor,
            &request.bibliography_blocks,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Html => format_bibliography_blocks::<Html>(
            &processor,
            &request.bibliography_blocks,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Djot => format_bibliography_blocks::<Djot>(
            &processor,
            &request.bibliography_blocks,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Latex => format_bibliography_blocks::<Latex>(
            &processor,
            &request.bibliography_blocks,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Typst => format_bibliography_blocks::<Typst>(
            &processor,
            &request.bibliography_blocks,
            request.document_options.as_ref(),
            &run,
        )?,
        OutputFormatKind::Markdown => format_bibliography_blocks::<Markdown>(
            &processor,
            &request.bibliography_blocks,
            request.document_options.as_ref(),
            &run,
        )?,
    };

    Ok(FormatDocumentResult {
        formatted_citations,
        bibliography,
        bibliography_blocks,
        warnings,
    })
}

/// Process citations and return formatted text.
pub(crate) fn format_by_kind<F>(
    processor: &Processor,
    citations: &[Citation],
    run: &mut crate::processor::RunState,
) -> Result<Vec<FormattedCitation>, FormatDocumentError>
where
    F: OutputFormat<Output = String>,
{
    let texts = processor.process_citations_with_format::<F>(citations, run)?;

    let formatted = citations
        .iter()
        .zip(texts.iter())
        .map(|(citation, text)| {
            let ref_ids = citation.items.iter().map(|item| item.id.clone()).collect();
            FormattedCitation {
                id: citation.id.clone().unwrap_or_default(),
                text: text.clone(),
                ref_ids,
            }
        })
        .collect();

    Ok(formatted)
}

/// Format the bibliography by output kind, restricted to the document's cited set.
///
/// Only references that appear in `processor.cited_ids` — either via an in-text
/// citation or via a `nocite` registration — are included in the output. Delegates
/// to [`Processor::render_document_bibliography`], the unified facade that ensures
/// both `content` and `entries` are computed from the same cited subset so
/// subsequent-author substitution stays consistent.
pub(crate) fn format_bibliography<F>(
    processor: &Processor,
    format_kind: OutputFormatKind,
    doc_opts: Option<&DocumentOptions>,
    run: &crate::processor::FinalizedRun,
) -> Result<FormattedBibliography, FormatDocumentError>
where
    F: OutputFormat<Output = String>,
{
    let (annotations, annotation_style) = annotation_options(doc_opts);
    let doc_bib = processor.render_document_bibliography::<F>(
        true,
        if annotations.is_empty() {
            None
        } else {
            Some(&annotations)
        },
        annotation_style.as_ref(),
        run,
    );
    let entries = doc_bib
        .entries
        .into_iter()
        .map(|entry| {
            proc_entry_to_bibliography_entry::<F>(
                entry,
                if annotations.is_empty() {
                    None
                } else {
                    Some(&annotations)
                },
                annotation_style.as_ref(),
            )
        })
        .collect();
    Ok(FormattedBibliography {
        format: format_kind,
        content: doc_bib.content,
        entries,
    })
}

/// Format ordered sectional bibliography blocks.
///
/// Threads a single `assigned` dedup set through all blocks so each reference
/// appears in only one block. Renders entries with annotations if configured.
pub(crate) fn format_bibliography_blocks<F>(
    processor: &Processor,
    requests: &[super::BibliographyBlockRequest],
    doc_opts: Option<&DocumentOptions>,
    run: &crate::processor::FinalizedRun,
) -> Result<Vec<super::FormattedBibliographyBlock>, FormatDocumentError>
where
    F: OutputFormat<Output = String>,
{
    if requests.is_empty() {
        return Ok(Vec::new());
    }

    let (annotations, annotation_style) = annotation_options(doc_opts);
    let groups: Vec<_> = requests.iter().map(|r| r.group.clone()).collect();
    let rendered = processor.render_document_bibliography_blocks::<F>(
        &groups,
        if annotations.is_empty() {
            None
        } else {
            Some(&annotations)
        },
        annotation_style.as_ref(),
        run,
    );

    Ok(requests
        .iter()
        .zip(rendered)
        .map(|(req, rg)| super::FormattedBibliographyBlock {
            id: req.id.clone(),
            heading: rg.heading,
            content: rg.body,
            entries: rg
                .entries
                .into_iter()
                .map(|entry| {
                    proc_entry_to_bibliography_entry::<F>(
                        entry,
                        if annotations.is_empty() {
                            None
                        } else {
                            Some(&annotations)
                        },
                        annotation_style.as_ref(),
                    )
                })
                .collect(),
        })
        .collect())
}

/// Extract annotation map and style from document options.
fn annotation_options(
    doc_opts: Option<&DocumentOptions>,
) -> (HashMap<String, String>, Option<AnnotationStyle>) {
    if let Some(opts) = doc_opts
        && let Some(anns) = &opts.annotations
    {
        let style = opts.annotation_format.as_ref().map(|fmt| AnnotationStyle {
            format: fmt.clone(),
        });
        return (anns.clone(), style);
    }
    (HashMap::new(), None)
}

/// Convert a processor entry to a bibliography entry with annotations.
fn proc_entry_to_bibliography_entry<F>(
    entry: crate::render::ProcEntry,
    annotations: Option<&HashMap<String, String>>,
    annotation_style: Option<&AnnotationStyle>,
) -> BibliographyEntry
where
    F: OutputFormat<Output = String>,
{
    let text = crate::render::bibliography::refs_to_string_slice_with_format::<F>(
        std::slice::from_ref(&entry),
        annotations,
        annotation_style,
    );
    let metadata = EntryMetadata {
        author: entry.metadata.author.unwrap_or_default(),
        year: entry.metadata.year.unwrap_or_default(),
        title: entry.metadata.title.unwrap_or_default(),
    };
    BibliographyEntry {
        id: entry.id,
        text,
        metadata,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code uses assertions and panic"
)]
mod tests {
    use super::*;
    use crate::api::CitationOccurrenceItem;
    use crate::reference::Bibliography;
    use crate::{
        Config, ContributorForm, ContributorRole, DateForm, Processing, Rendering,
        TemplateComponent, TemplateContributor, TemplateDate, TemplateDateVariable,
        WrapPunctuation,
    };
    use citum_schema::data::citation::CitationMode;
    use citum_schema::options::{AndOptions, ContributorConfig};
    use citum_schema::reference::{DateValue, InputReference, Monograph, MonographType, Title};
    use citum_schema::template::{TemplateTitle, TitleType};
    use citum_schema::{BibliographySpec, CitationSpec, StyleInfo};
    use std::collections::HashMap;

    fn make_test_style() -> Style {
        Style {
            info: StyleInfo {
                title: Some("Test Style".to_string()),
                id: Some("test".into()),
                ..Default::default()
            },
            options: Some(Config {
                processing: Some(Processing::AuthorDate),
                ..Default::default()
            }),
            citation: Some(CitationSpec {
                template: Some(
                    vec![
                        TemplateComponent::Contributor(TemplateContributor {
                            contributor: ContributorRole::Author.into(),
                            form: ContributorForm::Short,
                            rendering: Rendering::default(),
                            ..Default::default()
                        }),
                        TemplateComponent::Date(TemplateDate {
                            date: TemplateDateVariable::Issued,
                            form: DateForm::Year,
                            rendering: Rendering::default(),
                            ..Default::default()
                        }),
                    ]
                    .into(),
                ),
                wrap: Some(WrapPunctuation::Parentheses.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_test_bibliography() -> RefsInput {
        let mut refs = Bibliography::new();
        refs.insert(
            "smith2020".to_string(),
            InputReference::Monograph(Box::new(Monograph {
                id: Some("smith2020".into()),
                r#type: MonographType::Book,
                title: Some(Title::Single("Sample Work".to_string())),
                issued: DateValue::new("2020".to_string()),
                ..Default::default()
            })),
        );
        RefsInput::Json(serde_json::to_value(refs).unwrap())
    }

    fn make_markup_bibliography() -> RefsInput {
        let mut refs = Bibliography::new();
        refs.insert(
            "art1".to_string(),
            InputReference::Monograph(Box::new(Monograph {
                id: Some("art1".into()),
                r#type: MonographType::Book,
                title: Some(Title::Single(
                    "_Homo sapiens_ and *modern* world".to_string(),
                )),
                issued: DateValue::new("2023".to_string()),
                ..Default::default()
            })),
        );
        RefsInput::Json(serde_json::to_value(refs).unwrap())
    }

    #[test]
    fn format_document_with_style_empty_citations() {
        let style = make_test_style();
        let refs = make_test_bibliography();
        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document_with_style(style, request);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.formatted_citations.len(), 0);
    }

    #[test]
    fn format_document_html_bibliography_entries_preserve_inline_markup() {
        let mut style = make_test_style();
        style.bibliography = Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        });

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Html,
            refs: make_markup_bibliography(),
            citations: vec![],
            bibliography_blocks: Vec::new(),
            document_options: None,
            // Use nocite to include art1 in the bibliography without an in-text citation;
            // the test is validating bibliography HTML rendering, not citation rendering.
            nocite: vec!["art1".to_string()],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert_eq!(
            result.bibliography.entries[0].text, result.bibliography.content,
            "single-entry bibliography should mirror the full bibliography payload"
        );
        assert!(
            result.bibliography.entries[0].text.contains(
                "<span class=\"citum-title\"><em>Homo sapiens</em> and <b>modern</b> world</span>"
            ),
            "per-entry HTML should preserve inline markup for Djot-bearing titles"
        );
    }

    #[test]
    fn format_document_missing_ref_warning() {
        let style = make_test_style();
        let refs = make_test_bibliography();

        let citation_occ = CitationOccurrence {
            id: "cite1".to_string(),
            items: vec![CitationOccurrenceItem {
                id: "unknown_ref".to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: None,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        };

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![citation_occ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document_with_style(style, request);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert!(res.warnings.iter().any(|w| w.code == "missing_ref"));
    }

    #[test]
    fn format_document_unknown_reference_class_warning() {
        let style = make_test_style();
        let mut refs = Bibliography::new();
        let unknown_ref: InputReference = serde_json::from_str(
            r#"{
                "class": "dance-performance",
                "id": "pina2011",
                "title": "Pina",
                "issued": "2011",
                "venue": "Berlin"
            }"#,
        )
        .expect("unknown class should parse through the compatibility path");
        refs.insert("pina2011".to_string(), unknown_ref);

        let citation_occ = CitationOccurrence {
            id: "cite1".to_string(),
            items: vec![CitationOccurrenceItem {
                id: "pina2011".to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: None,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        };

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs: RefsInput::Json(serde_json::to_value(refs).unwrap()),
            citations: vec![citation_occ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document_with_style(style, request).unwrap();
        let warning = result
            .warnings
            .iter()
            .find(|w| w.code == "unknown_reference_class")
            .expect("unknown class warning should be emitted");
        assert_eq!(warning.ref_id.as_deref(), Some("pina2011"));
        assert!(warning.message.contains("dance-performance"));
    }

    #[test]
    fn format_document_yaml_style_input() {
        let style = make_test_style();
        let yaml_style = serde_yaml::to_string(&style).expect("serialize test style");

        let mut refs = Bibliography::new();
        refs.insert(
            "test2024".to_string(),
            InputReference::Monograph(Box::new(Monograph {
                id: Some("test2024".into()),
                r#type: MonographType::Book,
                title: Some(Title::Single("Test Work".to_string())),
                issued: DateValue::new("2024".to_string()),
                ..Default::default()
            })),
        );

        let citation_occ = CitationOccurrence {
            id: "c1".to_string(),
            items: vec![CitationOccurrenceItem {
                id: "test2024".to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: None,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        };

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml(yaml_style),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs: RefsInput::Json(serde_json::to_value(refs).unwrap()),
            citations: vec![citation_occ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document(request);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.formatted_citations.len(), 1);
        assert!(!res.formatted_citations[0].text.is_empty());
    }

    #[test]
    fn format_document_uri_input_unresolved() {
        let request = FormatDocumentRequest {
            style: StyleInput::Uri("https://example.com/style.yaml".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs: RefsInput::Json(serde_json::Value::Object(Default::default())),
            citations: vec![],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document(request);
        match result {
            Err(FormatDocumentError::UnresolvedInput(_)) => {
                // Expected
            }
            _ => panic!("Expected UnresolvedInput error"),
        }
    }

    /// A minimal resolver that returns a fixed style for any ID.
    struct MockResolver(Style);

    impl citum_resolver_api::StyleResolver for MockResolver {
        type Style = Style;
        type Locale = citum_schema::locale::Locale;

        fn resolve_style(&self, _uri: &str) -> Result<Style, citum_schema::ResolverError> {
            Ok(self.0.clone())
        }

        fn resolve_locale(
            &self,
            id: &str,
        ) -> Result<citum_schema::locale::Locale, citum_schema::ResolverError> {
            Err(citum_schema::ResolverError::LocaleNotFound(
                std::borrow::Cow::Owned(id.to_string()),
            ))
        }
    }

    #[test]
    fn format_document_with_resolver_injects_style_for_id_input() {
        let style = make_test_style();
        let resolver = MockResolver(style);
        let refs = make_test_bibliography();

        let citation_occ = CitationOccurrence {
            id: "c1".to_string(),
            items: vec![CitationOccurrenceItem {
                id: "smith2020".to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: None,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        };

        let request = FormatDocumentRequest {
            style: StyleInput::Id("any-id".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![citation_occ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        // Without a resolver, the same Id input must be rejected.
        match format_document(request.clone()) {
            Err(FormatDocumentError::UnresolvedInput(_)) => {}
            other => panic!("expected UnresolvedInput without resolver, got: {other:?}"),
        }

        // With the injected resolver it must succeed.
        let result = format_document_with_resolver(request, &resolver);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
        let res = result.unwrap();
        assert_eq!(res.formatted_citations.len(), 1);
        assert!(
            !res.formatted_citations[0].text.is_empty(),
            "formatted citation text should not be empty"
        );
    }

    /// Build an author-date style whose citation template renders contributor short form.
    fn make_two_author_style() -> Style {
        Style {
            info: StyleInfo {
                title: Some("Override Test Style".to_string()),
                id: Some("override-test".into()),
                ..Default::default()
            },
            options: Some(Config {
                processing: Some(Processing::AuthorDate),
                // Explicitly set `and: text` so the override to `symbol` is observable
                // in rendered output without relying on any default connector.
                contributors: Some(ContributorConfig {
                    and: Some(AndOptions::Text),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            citation: Some(CitationSpec {
                template: Some(
                    vec![
                        TemplateComponent::Contributor(TemplateContributor {
                            contributor: ContributorRole::Author.into(),
                            form: ContributorForm::Short,
                            rendering: Rendering::default(),
                            ..Default::default()
                        }),
                        TemplateComponent::Date(TemplateDate {
                            date: TemplateDateVariable::Issued,
                            form: DateForm::Year,
                            rendering: Rendering {
                                prefix: Some(", ".into()),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    ]
                    .into(),
                ),
                wrap: Some(WrapPunctuation::Parentheses.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Build a refs input with a two-author book so the "and" connector is exercised.
    ///
    /// Uses inline YAML (the reliably tested deserialization path) rather than
    /// round-tripping through `serde_json::to_value` which may not preserve the
    /// contributor tagged-enum layout the engine expects.
    fn make_two_author_refs() -> RefsInput {
        RefsInput::Yaml(
            r#"duo2024:
  class: monograph
  id: duo2024
  type: book
  title: Duo Work
  issued: "2024"
  author:
    - family: Smith
      given: Alice
    - family: Jones
      given: Bob
"#
            .to_string(),
        )
    }

    /// Helper: produce a single-item citation occurrence for a given ref id.
    fn cite(ref_id: &str) -> CitationOccurrence {
        CitationOccurrence {
            id: "c1".to_string(),
            items: vec![CitationOccurrenceItem {
                id: ref_id.to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: None,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        }
    }

    #[test]
    fn style_overrides_and_symbol_changes_rendered_output() {
        let base_style = make_two_author_style();
        let refs = make_two_author_refs();

        // given: base style produces a citation containing "and"
        let request_base = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs: refs.clone(),
            citations: vec![cite("duo2024")],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };
        let result_base = format_document_with_style(base_style.clone(), request_base).unwrap();
        let text_base = &result_base.formatted_citations[0].text;
        assert!(
            text_base.contains("and"),
            "base style should use text 'and' connector, got: {text_base:?}"
        );

        // when: style_overrides switches connector to symbol "&"
        let request_override = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: Some("options:\n  contributors:\n    and: symbol\n".to_string()),
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![cite("duo2024")],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };
        let result_override =
            format_document_with_style(base_style.clone(), request_override).unwrap();
        let text_override = &result_override.formatted_citations[0].text;
        assert!(
            text_override.contains('&'),
            "overridden style should use '&' connector, got: {text_override:?}"
        );

        // then: base style struct is untouched — still has Text, not Symbol
        let base_and = base_style
            .options
            .as_ref()
            .and_then(|o| o.contributors.as_ref())
            .and_then(|c| c.and.as_ref());
        assert!(
            matches!(base_and, Some(&AndOptions::Text)),
            "base style must not be mutated; expected And::Text, got: {base_and:?}"
        );
    }

    #[test]
    fn style_overrides_invalid_yaml_returns_parse_error() {
        let style = make_test_style();
        let refs = make_test_bibliography();

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: Some("{ unclosed yaml: [".to_string()),
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        match format_document_with_style(style, request) {
            Err(FormatDocumentError::StyleParse(msg)) => {
                assert!(
                    msg.contains("style_overrides"),
                    "error message should mention style_overrides, got: {msg}"
                );
            }
            other => panic!("expected StyleParse error, got: {other:?}"),
        }
    }

    #[test]
    fn apply_style_overrides_merges_option_field() {
        let mut style = make_test_style();
        apply_style_overrides(&mut style, "options:\n  contributors:\n    and: symbol\n")
            .expect("apply_style_overrides should succeed");

        let and_option = style
            .options
            .as_ref()
            .and_then(|o| o.contributors.as_ref())
            .and_then(|c| c.and.as_ref());
        assert!(
            matches!(and_option, Some(&AndOptions::Symbol)),
            "expected And::Symbol after override, got: {and_option:?}"
        );
    }

    // --- integral_name_memory wiring ---

    /// Build a style that has integral-name memory configured with scope=Document,
    /// contexts=BodyAndNotes, subsequent_form=Short, and an integral sub-template
    /// that renders the author in Long (given + family) form.
    fn make_integral_name_style() -> Style {
        use citum_schema::options::{
            IntegralNameContexts, IntegralNameMemoryConfig, IntegralNameScope, SubsequentNameForm,
        };
        Style {
            info: StyleInfo {
                title: Some("Integral Name Memory Test".to_string()),
                id: Some("integral-name-memory-test".into()),
                ..Default::default()
            },
            options: Some(Config {
                processing: Some(Processing::AuthorDate),
                integral_name_memory: Some(IntegralNameMemoryConfig {
                    scope: Some(IntegralNameScope::Document),
                    contexts: Some(IntegralNameContexts::BodyAndNotes),
                    subsequent_form: Some(SubsequentNameForm::Short),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            citation: Some(CitationSpec {
                integral: Some(Box::new(CitationSpec {
                    template: Some(
                        vec![TemplateComponent::Contributor(TemplateContributor {
                            contributor: ContributorRole::Author.into(),
                            form: ContributorForm::Long,
                            rendering: Rendering::default(),
                            ..Default::default()
                        })]
                        .into(),
                    ),
                    ..Default::default()
                })),
                template: Some(
                    vec![
                        TemplateComponent::Contributor(TemplateContributor {
                            contributor: ContributorRole::Author.into(),
                            form: ContributorForm::Short,
                            rendering: Rendering::default(),
                            ..Default::default()
                        }),
                        TemplateComponent::Date(TemplateDate {
                            date: TemplateDateVariable::Issued,
                            form: DateForm::Year,
                            rendering: Rendering::default(),
                            ..Default::default()
                        }),
                    ]
                    .into(),
                ),
                wrap: Some(WrapPunctuation::Parentheses.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_smith_refs() -> RefsInput {
        RefsInput::Yaml(
            r#"smith2020:
  class: monograph
  id: smith2020
  type: book
  title: Smith Book
  issued: "2020"
  author:
    - family: Smith
      given: John
"#
            .to_string(),
        )
    }

    fn make_integral_occ(id: &str, ref_id: &str) -> CitationOccurrence {
        CitationOccurrence {
            id: id.to_string(),
            items: vec![CitationOccurrenceItem {
                id: ref_id.to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: Some(citum_schema::data::citation::CitationMode::Integral),
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        }
    }

    #[test]
    fn document_options_integral_name_memory_first_full_then_short() {
        use crate::processor::document::DocumentIntegralNameOverride;

        let style = make_integral_name_style();
        let refs = make_smith_refs();

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![
                make_integral_occ("c1", "smith2020"),
                make_integral_occ("c2", "smith2020"),
            ],
            bibliography_blocks: Vec::new(),
            document_options: Some(DocumentOptions {
                integral_name_memory: Some(DocumentIntegralNameOverride {
                    enabled: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            nocite: vec![],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert!(
            !result
                .warnings
                .iter()
                .any(|w| w.code == "integral_name_memory_not_applied"),
            "stale warning must not appear: {:?}",
            result.warnings
        );
        assert_eq!(
            result.formatted_citations[0].text, "John Smith",
            "first integral cite should render full name form"
        );
        assert_eq!(
            result.formatted_citations[1].text, "Smith",
            "second integral cite of same author should render short form"
        );
    }

    #[test]
    fn document_options_integral_name_memory_disabled_keeps_full_form() {
        use crate::processor::document::DocumentIntegralNameOverride;

        let style = make_integral_name_style();
        let refs = make_smith_refs();

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![
                make_integral_occ("c1", "smith2020"),
                make_integral_occ("c2", "smith2020"),
            ],
            bibliography_blocks: Vec::new(),
            document_options: Some(DocumentOptions {
                integral_name_memory: Some(DocumentIntegralNameOverride {
                    enabled: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            nocite: vec![],
        };

        let result = format_document_with_style(style, request).expect("should render");

        // With memory disabled both occurrences should render the natural integral
        // template form (Long = "John Smith") without any subsequent rewrite.
        assert_eq!(
            result.formatted_citations[0].text, "John Smith",
            "first integral cite: {}",
            result.formatted_citations[0].text
        );
        assert_eq!(
            result.formatted_citations[1].text, "John Smith",
            "second integral cite should also be full when memory is disabled"
        );
    }

    #[test]
    fn style_native_integral_name_memory_applied_without_document_override() {
        // Style has integral_name_memory in its own options; no document_options
        // override is supplied. The flat API must still annotate First/Subsequent.
        let style = make_integral_name_style();
        let refs = make_smith_refs();

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![
                make_integral_occ("c1", "smith2020"),
                make_integral_occ("c2", "smith2020"),
            ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert_eq!(
            result.formatted_citations[0].text, "John Smith",
            "first integral cite should render full name form"
        );
        assert_eq!(
            result.formatted_citations[1].text, "Smith",
            "second integral cite should render short form from style-native config"
        );
    }

    #[test]
    fn format_document_bibliography_blocks_ordered_with_dedup() {
        use citum_schema::grouping::CitedStatus;
        use citum_schema::grouping::{BibliographyGroup, GroupSelector};

        let mut style = make_test_style();
        style.bibliography = Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        });
        let mut refs = Bibliography::new();
        refs.insert(
            "smith2020".to_string(),
            InputReference::Monograph(Box::new(Monograph {
                id: Some("smith2020".into()),
                r#type: MonographType::Book,
                title: Some(Title::Single("Sample Work".to_string())),
                issued: DateValue::new("2020".to_string()),
                ..Default::default()
            })),
        );
        refs.insert(
            "jones2019".to_string(),
            InputReference::Monograph(Box::new(Monograph {
                id: Some("jones2019".into()),
                r#type: MonographType::Book,
                title: Some(Title::Single("Another Work".to_string())),
                issued: DateValue::new("2019".to_string()),
                ..Default::default()
            })),
        );

        let make_block = |id: &str| crate::BibliographyBlockRequest {
            id: id.to_string(),
            group: BibliographyGroup {
                id: id.to_string(),
                selector: GroupSelector {
                    cited: Some(CitedStatus::Any),
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs: RefsInput::Json(serde_json::to_value(refs).unwrap()),
            citations: vec![],
            bibliography_blocks: vec![make_block("block-a"), make_block("block-b")],
            document_options: None,
            nocite: vec![],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert_eq!(result.bibliography_blocks.len(), 2, "both blocks returned");
        assert_eq!(result.bibliography_blocks[0].id, "block-a");
        assert_eq!(result.bibliography_blocks[1].id, "block-b");

        let block_a_count = result.bibliography_blocks[0].entries.len();
        let block_b_count = result.bibliography_blocks[1].entries.len();

        assert_eq!(block_a_count, 2, "block-a captures both refs");
        assert_eq!(
            block_b_count, 0,
            "block-b is empty: dedup set prevents re-assignment from block-a"
        );
    }

    // --- nocite tests ---

    /// A ref listed only in `nocite` must appear in the bibliography but produce
    /// no `formatted_citations` entry (standard citeproc nocite semantics).
    #[test]
    fn nocite_ref_in_bibliography_not_in_formatted_citations() {
        let mut style = make_test_style();
        // A bibliography template is required for entries to be produced.
        style.bibliography = Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        });
        let refs = make_test_bibliography(); // contains "smith2020"

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec!["smith2020".to_string()],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert_eq!(
            result.formatted_citations.len(),
            0,
            "nocite refs must not produce a formatted citation"
        );
        assert_eq!(
            result.bibliography.entries.len(),
            1,
            "nocite ref must appear in bibliography entries"
        );
        assert_eq!(
            result.bibliography.entries[0].id, "smith2020",
            "bibliography entry id should match nocite ref"
        );
        assert!(
            !result.bibliography.content.is_empty(),
            "bibliography content must be non-empty for nocite ref"
        );
        assert!(
            result.warnings.is_empty(),
            "no warnings expected: {:?}",
            result.warnings
        );
    }

    /// An ID listed in `nocite` that is absent from `refs` must emit a
    /// `nocite_missing_ref` warning and not appear in the bibliography.
    #[test]
    fn nocite_missing_ref_emits_warning() {
        let style = make_test_style();
        let refs = make_test_bibliography();

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec!["does_not_exist".to_string()],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert_eq!(
            result.bibliography.entries.len(),
            0,
            "absent nocite ref must not produce a bibliography entry"
        );
        let warning = result
            .warnings
            .iter()
            .find(|w| w.code == "nocite_missing_ref")
            .expect("nocite_missing_ref warning should be emitted");
        assert_eq!(
            warning.ref_id.as_deref(),
            Some("does_not_exist"),
            "warning ref_id should name the absent nocite key"
        );
    }

    /// A nocite ref must sort alongside the cited ref when both are present
    /// (i.e., citation status does not affect bibliography sort order).
    #[test]
    fn nocite_ref_sorts_alongside_cited_ref() {
        let mut style = make_test_style();
        style.bibliography = Some(BibliographySpec {
            template: Some(
                vec![TemplateComponent::Title(TemplateTitle {
                    title: TitleType::Primary,
                    ..Default::default()
                })]
                .into(),
            ),
            ..Default::default()
        });

        let citation_occ = CitationOccurrence {
            id: "c1".to_string(),
            items: vec![CitationOccurrenceItem {
                id: "duo2024".to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode: None,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        };

        // Two refs: duo2024 (cited via citation_occ) + smith2020 (nocite-only).
        let combined_refs = RefsInput::Yaml(
            r#"duo2024:
  class: monograph
  id: duo2024
  type: book
  title: Duo Work
  issued: "2024"
  author:
    - family: Smith
      given: Alice
    - family: Jones
      given: Bob
smith2020:
  class: monograph
  id: smith2020
  type: book
  title: Smith Work
  issued: "2020"
  author:
    - family: Smith
      given: Alex
"#
            .to_string(),
        );

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml("dummy".to_string()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs: combined_refs,
            citations: vec![citation_occ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec!["smith2020".to_string()],
        };

        let result = format_document_with_style(style, request).expect("should render");

        assert_eq!(result.formatted_citations.len(), 1, "one in-text citation");
        assert_eq!(
            result.bibliography.entries.len(),
            2,
            "both cited and nocite refs must appear in the bibliography"
        );
        let ids: Vec<&str> = result
            .bibliography
            .entries
            .iter()
            .map(|e| e.id.as_str())
            .collect();
        assert!(
            ids.contains(&"duo2024"),
            "cited ref must be in bibliography: {ids:?}"
        );
        assert!(
            ids.contains(&"smith2020"),
            "nocite ref must be in bibliography: {ids:?}"
        );
    }

    fn apa_style_path() -> String {
        use std::path::PathBuf;
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("styles/embedded/apa-7th.yaml")
            .to_str()
            .unwrap()
            .to_string()
    }

    fn chicago_notes_path() -> String {
        use std::path::PathBuf;
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("crates/citum-engine/tests/fixtures/styles/chicago-notes-18th.yaml")
            .to_str()
            .unwrap()
            .to_string()
    }

    fn chicago_notes_style_without_ibid() -> String {
        let yaml = std::fs::read_to_string(chicago_notes_path())
            .expect("Chicago notes fixture should be readable");
        let ibid = "  ibid:\n    note-start-text-case: capitalize-first\n    suffix: \"\"\n    delimiter: \", \"\n    template:\n    - message: term.ibid\n    - variable: locator\n      suffix: .\n";
        yaml.replacen(ibid, "", 1)
    }

    fn make_citation_occ(id: &str, ref_id: &str, mode: Option<CitationMode>) -> CitationOccurrence {
        CitationOccurrence {
            id: id.to_string(),
            items: vec![CitationOccurrenceItem {
                id: ref_id.to_string(),
                locator: None,
                prefix: None,
                suffix: None,
                integral_name_state: None,
                org_abbreviation_state: None,
            }],
            mode,
            note_number: None,
            suppress_author: None,
            grouped: None,
            prefix: None,
            suffix: None,
            sentence_start: None,
        }
    }

    #[test]
    fn format_document_author_date_mixed_citation_modes_order_preserved() {
        let refs = RefsInput::Json(serde_json::json!({
            "smith2020": {
                "id": "smith2020",
                "class": "monograph",
                "type": "book",
                "title": "Sample Work",
                "author": [{"family": "Smith", "given": "John"}],
                "issued": "2020"
            }
        }));

        let request = FormatDocumentRequest {
            style: StyleInput::Path(apa_style_path()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![
                make_citation_occ("cite-integral", "smith2020", Some(CitationMode::Integral)),
                make_citation_occ(
                    "cite-non-integral",
                    "smith2020",
                    Some(CitationMode::NonIntegral),
                ),
            ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document(request).expect("format_document should succeed");

        assert_eq!(
            result.formatted_citations.len(),
            2,
            "both citations should be returned"
        );
        assert_eq!(
            result.formatted_citations[0].id, "cite-integral",
            "document order must be preserved"
        );
        assert_eq!(
            result.formatted_citations[1].id, "cite-non-integral",
            "document order must be preserved"
        );

        let integral = &result.formatted_citations[0].text;
        let non_integral = &result.formatted_citations[1].text;

        assert!(
            !integral.starts_with('('),
            "integral citation should place author name outside parentheses: {integral:?}"
        );
        assert!(
            integral.contains("Smith"),
            "integral citation should contain author name: {integral:?}"
        );
        assert!(
            non_integral.starts_with('('),
            "non-integral citation should be fully parenthetical: {non_integral:?}"
        );
    }

    #[test]
    fn format_document_note_style_repeat_citations_use_subsequent_form() {
        let refs = RefsInput::Json(serde_json::json!({
            "smith1995": {
                "id": "smith1995",
                "class": "monograph",
                "type": "book",
                "title": "A Great Book",
                "author": [{"family": "Smith", "given": "John"}],
                "issued": "1995"
            }
        }));

        let request = FormatDocumentRequest {
            style: StyleInput::Yaml(chicago_notes_style_without_ibid()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![
                make_citation_occ("cite-1", "smith1995", None),
                make_citation_occ("cite-2", "smith1995", None),
                make_citation_occ("cite-3", "smith1995", None),
            ],
            bibliography_blocks: Vec::new(),
            document_options: None,
            nocite: vec![],
        };

        let result = format_document(request).expect("format_document should succeed");

        assert_eq!(result.formatted_citations.len(), 3);

        let first = &result.formatted_citations[0].text;
        let second = &result.formatted_citations[1].text;
        let third = &result.formatted_citations[2].text;

        assert!(
            first.contains("Smith"),
            "first citation should render full form: {first:?}"
        );
        assert!(
            second.contains("Smith") && second.contains("A Great Book"),
            "immediate repeat should render the subsequent form: {second:?}"
        );
        assert!(
            third.contains("Smith") && third.contains("A Great Book"),
            "third repeat should also render the subsequent form: {third:?}"
        );
    }

    #[test]
    fn format_document_annotations_appear_in_bibliography() {
        let refs = RefsInput::Json(serde_json::json!({
            "smith2020": {
                "id": "smith2020",
                "class": "monograph",
                "type": "book",
                "title": "Sample Work",
                "author": [{"family": "Smith", "given": "John"}],
                "issued": "2020"
            }
        }));

        let mut annotations = HashMap::new();
        annotations.insert(
            "smith2020".to_string(),
            "Foundational work on the topic.".to_string(),
        );

        let request = FormatDocumentRequest {
            style: StyleInput::Path(apa_style_path()),
            style_overrides: None,
            locale: None,
            output_format: OutputFormatKind::Plain,
            refs,
            citations: vec![make_citation_occ("cite-1", "smith2020", None)],
            bibliography_blocks: Vec::new(),
            document_options: Some(DocumentOptions {
                annotations: Some(annotations),
                ..Default::default()
            }),
            nocite: vec![],
        };

        let result = format_document(request).expect("format_document should succeed");

        assert!(
            result
                .bibliography
                .content
                .contains("Foundational work on the topic."),
            "annotation text should appear in bibliography output: {:?}",
            result.bibliography.content
        );
    }
}
