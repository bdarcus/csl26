/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Bibliography-section synthesis entry points.
//!
//! Assembles the bibliography candidate seeds (inferred, XML-compiled, and
//! the Phase 1 typed patch candidates), runs the shared synthesis loop over
//! the bibliography template, and reports the gate-selected candidate with its
//! patch/mutation provenance.

use super::core::{
    HistoryEntry, MAX_SYNTHESIS_ROUNDS, RoundsContext, RoundsOutcome, accepted_mutation_names,
    heldout_gate, pick_seed, run_rounds,
};
use crate::error::MigrateError;
use crate::js_runtime::{EmbeddedTemplateRuntime, FixtureSet};
use crate::measured_citation::{
    CandidateBudget, CandidateScore, CandidateStyle, MeasuredBibliographySelection,
    bibliography_mutation_candidates, fixture_bibliography, heldout_bibliography_validation,
    heldout_debug_suffix, score_bibliography_candidate,
};
use citum_schema::Style;
use citum_schema::template::Template;
use std::collections::BTreeMap;
use std::path::Path;

/// Synthesize the bibliography template for a migrated style by measured
/// search.
///
/// Seeds the candidate pool with the inferred and XML-compiled styles plus
/// the Phase 1 typed patch candidates, then runs up to
/// [`MAX_SYNTHESIS_ROUNDS`] bounded mutation rounds on the winner and
/// validates the result on the held-out fixture set.
///
/// # Errors
///
/// Returns an error when the embedded runtime, fixture data, or reference
/// rendering is unavailable; callers should treat that as "keep the
/// inferred candidate".
pub fn synthesize_bibliography(
    inferred_style: &Style,
    xml_style: &Style,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
) -> Result<MeasuredBibliographySelection, MigrateError> {
    synthesize_bibliography_rounds(
        inferred_style,
        xml_style,
        style_name,
        style_xml,
        workspace_root,
        MAX_SYNTHESIS_ROUNDS,
    )
}

/// Bibliography synthesis with an explicit round cap; zero rounds reproduces
/// the Phase 1 bounded selector.
pub(crate) fn synthesize_bibliography_rounds(
    inferred_style: &Style,
    xml_style: &Style,
    style_name: &str,
    style_xml: &str,
    workspace_root: &Path,
    max_rounds: usize,
) -> Result<MeasuredBibliographySelection, MigrateError> {
    let mut runtime = EmbeddedTemplateRuntime::new(workspace_root)?;
    let reference_json =
        runtime.render_bibliography_strings(style_name, style_xml, FixtureSet::Selection)?;
    let references: BTreeMap<String, Option<String>> = serde_json::from_str(&reference_json)
        .map_err(|err| {
            MigrateError::Parse(format!(
                "failed to parse citeproc bibliography references: {err}"
            ))
        })?;
    let bibliography = fixture_bibliography(workspace_root)?;
    let budget = CandidateBudget::default();
    let debug = std::env::var_os("CITUM_MIGRATE_DEBUG_BIB_SELECTION").is_some();

    let mut candidates = vec![
        CandidateStyle::baseline("inferred", inferred_style.clone()),
        CandidateStyle::source_xml(xml_style.clone()),
    ];
    let mut mutations = bibliography_mutation_candidates(inferred_style, budget);
    budget.truncate_mutations(&mut mutations, candidates.len());
    candidates.extend(mutations);

    let score_fn = |style: &Style| score_bibliography_candidate(style, &bibliography, &references);
    let scored: Vec<CandidateScore> = candidates
        .iter()
        .map(|candidate| score_fn(&candidate.style))
        .collect();
    if debug {
        debug_bibliography_candidates(&candidates, &scored, style_name);
    }
    let seed = pick_seed(&scored, "bibliography")?;
    let Some(seed_candidate) = candidates.get(seed.seed_index) else {
        return Err(MigrateError::Render(
            "selected bibliography candidate was not generated".to_string(),
        ));
    };

    let context = RoundsContext {
        budget,
        max_rounds,
        debug,
        style_name,
        section: "bibliography",
    };
    let outcome = run_rounds(
        HistoryEntry {
            name: seed_candidate.name.clone(),
            style: seed_candidate.style.clone(),
            score: seed.seed_score,
        },
        &score_fn,
        &full_bibliography_template,
        &|style: &Style, template: Template| {
            let mut mutated = style.clone();
            mutated.bibliography.as_mut()?.template = Some(template.into());
            Some(mutated)
        },
        &context,
    );
    let mut heldout_of = |style: &Style| {
        heldout_bibliography_validation(&mut runtime, style_name, style_xml, style, workspace_root)
    };
    let (selected_index, heldout) = heldout_gate(&outcome.history, &mut heldout_of, &context);
    let Some(selected) = outcome.history.get(selected_index) else {
        return Err(MigrateError::Render(
            "selected bibliography candidate was not generated".to_string(),
        ));
    };
    let accepted_mutations = accepted_mutation_names(&outcome.accepted, selected_index);
    let (selected_family, selected_section, selected_affected_types) =
        bibliography_selection_metadata(seed_candidate, &outcome, selected_index);
    if debug {
        eprintln!(
            "bibliography selected {style_name} {} [{} {} {:?}]: +{} passes, {:+.3} similarity{}",
            selected.name,
            selected_family.as_deref().unwrap_or("baseline"),
            selected_section.as_deref().unwrap_or("none"),
            selected_affected_types,
            selected.score.passes.saturating_sub(seed.inferred.passes),
            selected.score.similarity_sum - seed.inferred.similarity_sum,
            heldout_debug_suffix(heldout)
        );
    }

    Ok(MeasuredBibliographySelection {
        selected_style: selected.style.clone(),
        selected_candidate: selected.name.clone(),
        selected_family,
        selected_section,
        selected_affected_types,
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

fn full_bibliography_template(style: &Style) -> Option<Template> {
    style
        .bibliography
        .as_ref()
        .and_then(|bibliography| bibliography.template.as_ref())
        .and_then(citum_schema::TemplateVariant::as_template)
        .map(<[_]>::to_vec)
}

/// Print per-candidate seed-stage scores for bibliography selection debug.
fn debug_bibliography_candidates(
    candidates: &[CandidateStyle],
    scored: &[CandidateScore],
    style_name: &str,
) {
    for (candidate, score) in candidates.iter().zip(scored) {
        eprintln!(
            "bibliography candidate {style_name} {} [{} {} {:?}]: {} passes, {:.3} similarity over {} items",
            candidate.name,
            candidate.family_label(),
            candidate.section_label(),
            candidate.affected_types,
            score.passes,
            score.similarity_sum,
            score.items
        );
    }
}

/// Selection metadata for the gate-selected bibliography incumbent: the seed
/// candidate's patch metadata at index zero, or the accepted mutation's
/// family for synthesized incumbents.
fn bibliography_selection_metadata(
    seed_candidate: &CandidateStyle,
    outcome: &RoundsOutcome,
    selected_index: usize,
) -> (Option<String>, Option<String>, Vec<String>) {
    if selected_index == 0 {
        return (
            seed_candidate
                .family
                .map(|family| family.as_str().to_string()),
            seed_candidate
                .affected_section
                .map(|section| section.as_str().to_string()),
            seed_candidate.affected_types.clone(),
        );
    }
    let family = outcome
        .accepted
        .get(selected_index.saturating_sub(1))
        .map(|mutation| mutation.family.to_string());
    (family, Some("bibliography".to_string()), Vec::new())
}
