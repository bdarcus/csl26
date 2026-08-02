/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "bin")]

mod assembly;
mod bib_postprocess;
mod citation_validate;
mod cli;
mod output_plan;
mod runtime;

use assembly::{
    StandaloneAssembly, TemplateSourceSelection, apply_measured_bibliography_selection,
    apply_measured_citation_selection,
};
use citation_validate::validate_and_normalize_inferred_citations;
use clap::Parser;
use cli::{Args, FamilyCandidateMode};
use output_plan::{
    EvidenceDetails, apply_family_candidate_routing, count_yaml_lines, log_migration_output_plan,
    write_optional_evidence,
};
use runtime::{
    log_template_sources, output_style_and_debug, resolve_style_name_and_templates,
    workspace_root_for_style_path,
};

use citum_migrate::{
    OptionsExtractor, compilation,
    evidence::MeasuredSelectionEvidence,
    lineage::StyleLineage,
    options_extractor::{MigrationOptions, processing::detect_processing_mode},
    passes::sqi_refinement,
    provenance::ProvenanceTracker,
};
use csl_legacy::parser::parse_style;
use roxmltree::Document;
use std::{fs, path::Path};

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "dhat-heap")]
fn heap_profiling_requested(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = heap_profiling_requested(std::env::var_os("CITUM_DHAT_HEAP").as_deref())
        .then(dhat::Profiler::new_heap);

    let cli = Args::parse();
    let path = &cli.path;
    let family_candidate = FamilyCandidateMode::from_arg(cli.family_candidate.as_deref());

    let enable_provenance = cli.debug_variable.is_some();
    let tracker = ProvenanceTracker::new(enable_provenance);
    let workspace_root = workspace_root_for_style_path(path);

    let text = fs::read_to_string(path)?;
    let doc = Document::parse(&text)?;
    let legacy_style = parse_style(doc.root_element())?;

    let detected_regime = detect_processing_mode(&legacy_style);
    let mut lineage = StyleLineage::resolve(path, &workspace_root, &legacy_style.info.links)?
        .apply_regime_guard(detected_regime.as_ref());
    let routing =
        apply_family_candidate_routing(&mut lineage, &workspace_root, &family_candidate, path)?;

    tracing::debug!("Migrating {path} to Citum...");
    tracing::debug!(
        "Resolved lineage: semantic={:?}, form={:?}, parent={}",
        lineage.semantic_class,
        lineage.implementation_form,
        lineage.parent_style_id.as_deref().unwrap_or("none")
    );
    log_migration_output_plan(&lineage);

    let MigrationOptions {
        mut options,
        bibliography_options,
        citation_contributor_overrides,
        bibliography_contributor_overrides,
        citation_has_scope_shorten,
    } = OptionsExtractor::extract_migration_options(&legacy_style);

    let (style_name, mut resolved) = resolve_style_name_and_templates(path, &cli);

    validate_and_normalize_inferred_citations(
        &mut resolved,
        &options,
        &legacy_style,
        style_name.as_str(),
        citation_has_scope_shorten,
    );

    let xml_fallback = Some(compilation::compile_from_xml(
        &legacy_style,
        &mut options,
        enable_provenance,
        &tracker,
    ));

    warn_for_unsupported_layouts(xml_fallback.as_ref(), &legacy_style.info.id);

    log_template_sources(&resolved);

    let assembly = StandaloneAssembly {
        legacy_style: &legacy_style,
        resolved: &resolved,
        xml_fallback: &xml_fallback,
        options: &options,
        bibliography_options: &bibliography_options,
        citation_contributor_overrides: &citation_contributor_overrides,
        bibliography_contributor_overrides: &bibliography_contributor_overrides,
    };
    let (standalone_style, measured_selection) =
        apply_measured_selection_pipeline(&assembly, &style_name, &text, &workspace_root)?;
    let standalone_style = sqi_refinement::refine_style(standalone_style);
    // Measure the standalone form first so the evidence record can report
    // the compression delta without re-running the pipeline. Cheap: one YAML
    // serialization of an in-memory value.
    let standalone_lines = count_yaml_lines(&standalone_style)?;

    let style = if cli.minimize_wrapper {
        lineage.apply_to_migrated_style_minimized(standalone_style, true)?
    } else {
        lineage.apply_to_migrated_style(standalone_style)?
    };
    let emitted_lines = count_yaml_lines(&style)?;
    let migration_diagnostics = migration_diagnostics(
        cli.emit_evidence.is_some(),
        &workspace_root,
        xml_fallback.as_ref(),
        &legacy_style.info.id,
    )?;

    write_optional_evidence(
        &cli,
        &lineage,
        standalone_lines,
        emitted_lines,
        cli.minimize_wrapper,
        routing.audit,
        EvidenceDetails {
            measured_selection,
            diagnostics: migration_diagnostics,
        },
    )?;

    output_style_and_debug(&style, cli.debug_variable.as_deref(), &tracker)?;
    Ok(())
}

#[cfg(all(test, feature = "dhat-heap"))]
mod tests {
    use super::heap_profiling_requested;
    use std::ffi::OsStr;

    #[test]
    fn heap_profiling_requires_explicit_environment_opt_in() {
        assert!(!heap_profiling_requested(None));
        assert!(heap_profiling_requested(Some(OsStr::new("1"))));
    }
}

fn migration_diagnostics(
    evidence_requested: bool,
    workspace_root: &Path,
    xml_fallback: Option<&compilation::XmlCompilationOutput>,
    style_id: &str,
) -> Result<Vec<citum_migrate::evidence::MigrationDiagnostic>, Box<dyn std::error::Error>> {
    if !evidence_requested {
        return Ok(Vec::new());
    }

    let mut diagnostics =
        citum_migrate::measured_citation::fixture_conversion_diagnostics(workspace_root)?;
    if xml_fallback.is_some_and(|fallback| fallback.unsupported_localized_layouts) {
        diagnostics.push(citum_migrate::evidence::MigrationDiagnostic {
            code: "unsupported-localized-layout-shape".to_string(),
            item_id: None,
            message: format!(
                "CSL-M localized layouts in {style_id} contain locale-specific wrapper, position, or type-variant shapes that Citum cannot preserve exactly"
            ),
        });
    }
    Ok(diagnostics)
}

fn warn_for_unsupported_layouts(
    xml_fallback: Option<&compilation::XmlCompilationOutput>,
    style_id: &str,
) {
    if xml_fallback.is_some_and(|fallback| fallback.unsupported_mixed_conditions) {
        tracing::debug!(
            "Warning: citation position branches could not be migrated cleanly for style {style_id}. Falling back to base citation template only."
        );
    }
    if xml_fallback.is_some_and(|fallback| fallback.unsupported_localized_layouts) {
        tracing::warn!(
            "CSL-M localized layouts in {style_id} contain locale-specific wrapper, position, or type-variant shapes that Citum cannot preserve exactly"
        );
    }
}

fn apply_measured_selection_pipeline(
    assembly: &StandaloneAssembly<'_>,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<
    (citum_schema::Style, Option<MeasuredSelectionEvidence>),
    citum_migrate::error::MigrateError,
> {
    let mut source_selection = TemplateSourceSelection::default();
    let standalone_style = assembly.assemble_with_selection(source_selection)?;
    let (standalone_style, use_xml_citation, citation_selection) =
        apply_measured_citation_selection(
            standalone_style,
            assembly,
            source_selection,
            style_name,
            style_xml,
            workspace_root,
        )?;
    source_selection.suppress_inferred_citation = use_xml_citation;
    let (standalone_style, _, bibliography_selection) = apply_measured_bibliography_selection(
        standalone_style,
        assembly,
        source_selection,
        style_name,
        style_xml,
        workspace_root,
    )?;
    let measured_selection = MeasuredSelectionEvidence {
        citation: citation_selection,
        bibliography: bibliography_selection,
    };
    Ok((
        standalone_style,
        (!measured_selection.is_empty()).then_some(measured_selection),
    ))
}

#[cfg(test)]
mod main_tests;
