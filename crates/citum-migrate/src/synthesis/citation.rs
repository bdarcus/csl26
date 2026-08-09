/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citation-section synthesis entry points.
//!
//! Assembles the citation candidate seeds (inferred, XML-compiled, and the
//! Phase 1 citation-local mutations), runs the shared synthesis loop over the
//! citation template, and reports the gate-selected candidate.

use super::core::{
    HistoryEntry, MAX_SYNTHESIS_ROUNDS, RoundsContext, accepted_mutation_names, heldout_gate,
    pick_seed, run_rounds,
};
use crate::error::MigrateError;
use crate::js_runtime::{EmbeddedTemplateRuntime, FixtureSet};
use crate::measured_citation::{
    CandidateBudget, CandidateScore, MeasuredCitationSelection, citation_mutation_candidates,
    fixture_bibliography, heldout_citation_validation, heldout_debug_suffix, score_candidate,
};
use citum_schema::Style;
use citum_schema::template::Template;
use std::collections::BTreeMap;
use std::path::Path;

/// Synthesize the citation templates for a migrated style by measured search.
///
/// Seeds the candidate pool with the inferred and XML-compiled styles, then
/// runs up to [`MAX_SYNTHESIS_ROUNDS`] bounded mutation rounds on the winner
/// and validates the result on the held-out fixture set.
///
/// # Errors
///
/// Returns an error when the embedded runtime, fixture data, or reference
/// rendering is unavailable; callers should treat that as "keep the
/// inferred candidate".
pub fn synthesize_citation(
    inferred_style: &Style,
    xml_style: &Style,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<MeasuredCitationSelection, MigrateError> {
    synthesize_citation_rounds(
        inferred_style,
        xml_style,
        style_name,
        style_xml,
        workspace_root,
        MAX_SYNTHESIS_ROUNDS,
    )
}

/// Citation synthesis with an explicit round cap; zero rounds reproduces the
/// Phase 1 bounded selector.
pub(crate) fn synthesize_citation_rounds(
    inferred_style: &Style,
    xml_style: &Style,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
    max_rounds: usize,
) -> Result<MeasuredCitationSelection, MigrateError> {
    let mut runtime = EmbeddedTemplateRuntime::new(workspace_root)?;
    let reference_json =
        runtime.render_citation_strings(style_name, style_xml, FixtureSet::Selection)?;
    let references: BTreeMap<String, Vec<Option<String>>> = serde_json::from_str(&reference_json)
        .map_err(|err| {
        MigrateError::Parse(format!(
            "failed to parse citeproc citation references: {err}"
        ))
    })?;
    let bibliography = fixture_bibliography(workspace_root)?;
    let budget = CandidateBudget::default();
    let debug = std::env::var_os("CITUM_MIGRATE_DEBUG_CITATION_SELECTION").is_some();

    let mut candidates = vec![
        ("inferred".to_string(), inferred_style.clone()),
        ("xml".to_string(), xml_style.clone()),
    ];
    let mut mutations = citation_mutation_candidates(inferred_style);
    budget.truncate_mutations(&mut mutations, candidates.len());
    candidates.extend(mutations);

    let score_fn = |style: &Style| score_candidate(style, &bibliography, &references);
    let scored: Vec<CandidateScore> = candidates
        .iter()
        .map(|(_, style)| score_fn(style))
        .collect();
    if debug {
        for ((name, _), score) in candidates.iter().zip(&scored) {
            eprintln!(
                "citation candidate {style_name} {name}: {} passes, {:.3} similarity over {} items",
                score.passes, score.similarity_sum, score.items
            );
        }
    }
    let seed = pick_seed(&scored, "citation")?;
    let Some((seed_name, seed_style)) = candidates.get(seed.seed_index) else {
        return Err(MigrateError::Render(
            "selected citation candidate was not generated".to_string(),
        ));
    };

    let context = RoundsContext {
        budget,
        max_rounds,
        debug,
        style_name,
        section: "citation",
    };
    let outcome = run_rounds(
        HistoryEntry {
            name: seed_name.clone(),
            style: seed_style.clone(),
            score: seed.seed_score,
        },
        &score_fn,
        &|style: &Style| {
            style
                .citation
                .as_ref()
                .and_then(|citation| citation.template.as_ref())
                .and_then(citum_schema::TemplateVariant::as_template)
                .map(<[_]>::to_vec)
        },
        &|style: &Style, template: Template| {
            let mut mutated = style.clone();
            mutated.citation.as_mut()?.template = Some(template.into());
            Some(mutated)
        },
        &context,
    );
    let mut heldout_of = |style: &Style| {
        heldout_citation_validation(&mut runtime, style_name, style_xml, style, workspace_root)
    };
    let (selected_index, heldout) = heldout_gate(&outcome.history, &mut heldout_of, &context);
    let Some(selected) = outcome.history.get(selected_index) else {
        return Err(MigrateError::Render(
            "selected citation candidate was not generated".to_string(),
        ));
    };
    let accepted_mutations = accepted_mutation_names(&outcome.accepted, selected_index);
    if debug {
        eprintln!(
            "citation selected {style_name} {}: +{} passes, {:+.3} similarity{}",
            selected.name,
            selected.score.passes.saturating_sub(seed.inferred.passes),
            selected.score.similarity_sum - seed.inferred.similarity_sum,
            heldout_debug_suffix(heldout)
        );
    }

    Ok(MeasuredCitationSelection {
        selected_style: selected.style.clone(),
        selected_candidate: selected.name.clone(),
        use_xml: selected.name == "xml",
        selected_passes: selected.score.passes,
        inferred_passes: seed.inferred.passes,
        xml_passes: seed.xml.passes,
        items: seed.inferred.items,
        heldout,
        synthesis_rounds: accepted_mutations.len(),
        accepted_mutations,
    })
}
