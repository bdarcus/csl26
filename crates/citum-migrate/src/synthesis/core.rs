/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Section-agnostic core of the synthesis loop.
//!
//! Holds the round/incumbent types, the seed picker, the bounded
//! propose/score/accept loop ([`run_rounds`]), and the held-out rejection
//! gate ([`heldout_gate`]). The citation and bibliography modules supply the
//! section-specific scoring and template accessors and call into this core.

use super::operators;
use crate::error::MigrateError;
use crate::measured_citation::{
    CandidateBudget, CandidateScore, HeldOutValidation, candidate_beats,
};
use citum_schema::Style;
use citum_schema::template::Template;

/// Maximum mutation rounds per style section before the loop stops.
///
/// Each round scores at most `CandidateBudget::max_total` candidates, so a
/// synthesis run renders a bounded number of candidates per section. The
/// loop also stops early on the first round that accepts no proposal.
pub const MAX_SYNTHESIS_ROUNDS: usize = 4;

/// One incumbent in the synthesis history: the seed-stage winner at index
/// zero, followed by each accepted mutation result in acceptance order.
pub(super) struct HistoryEntry {
    pub(super) name: String,
    pub(super) style: Style,
    pub(super) score: CandidateScore,
}

/// One accepted mutation, recorded for selection metadata and debug output.
pub(super) struct AcceptedMutation {
    pub(super) name: String,
    pub(super) family: &'static str,
}

/// Outcome of the bounded mutation rounds for one style section.
pub(super) struct RoundsOutcome {
    pub(super) history: Vec<HistoryEntry>,
    pub(super) accepted: Vec<AcceptedMutation>,
}

/// Shared parameters for one section's synthesis rounds.
pub(super) struct RoundsContext<'a> {
    pub(super) budget: CandidateBudget,
    pub(super) max_rounds: usize,
    pub(super) debug: bool,
    pub(super) style_name: &'a str,
    pub(super) section: &'static str,
}

/// Seed-stage scores: the inferred and XML baselines plus the seed winner.
pub(super) struct SeedScores {
    pub(super) inferred: CandidateScore,
    pub(super) xml: CandidateScore,
    pub(super) seed_index: usize,
    pub(super) seed_score: CandidateScore,
}

/// Pick the seed-stage winner and capture the baseline scores.
pub(super) fn pick_seed(
    scored: &[CandidateScore],
    section: &'static str,
) -> Result<SeedScores, MigrateError> {
    let Some(inferred) = scored.first().copied() else {
        return Err(MigrateError::Render(format!(
            "no {section} candidates were generated"
        )));
    };
    let Some(xml) = scored.get(1).copied() else {
        return Err(MigrateError::Render(format!(
            "XML {section} candidate was not generated"
        )));
    };
    let seed_index = best_candidate_index(scored);
    let Some(seed_score) = scored.get(seed_index).copied() else {
        return Err(MigrateError::Render(format!(
            "selected {section} candidate was not scored"
        )));
    };
    Ok(SeedScores {
        inferred,
        xml,
        seed_index,
        seed_score,
    })
}

/// Names of the mutations accepted up to the gate-selected incumbent.
pub(super) fn accepted_mutation_names(
    accepted: &[AcceptedMutation],
    selected_index: usize,
) -> Vec<String> {
    accepted
        .iter()
        .take(selected_index)
        .map(|mutation| mutation.name.clone())
        .collect()
}

/// Index of the best seed candidate under the Phase 1 acceptance rule, with
/// the first candidate as the incumbent default.
fn best_candidate_index(scored: &[CandidateScore]) -> usize {
    let Some(mut selected_score) = scored.first().copied() else {
        return 0;
    };
    let mut selected_index = 0;
    for (index, score) in scored.iter().enumerate().skip(1) {
        if candidate_beats(score, &selected_score) {
            selected_index = index;
            selected_score = *score;
        }
    }
    selected_index
}

/// Run bounded mutation rounds on the seed incumbent.
///
/// Each round enumerates one set of single-mutation proposals of the current
/// incumbent, scores them, and accepts the best proposal only under the
/// Phase 1 acceptance rule. The loop stops on the first round without an
/// accepted proposal or at `context.max_rounds`.
pub(super) fn run_rounds<ScoreFn, GetTemplate, WithTemplate>(
    seed: HistoryEntry,
    score_fn: &ScoreFn,
    get_template: &GetTemplate,
    with_template: &WithTemplate,
    context: &RoundsContext<'_>,
) -> RoundsOutcome
where
    ScoreFn: Fn(&Style) -> CandidateScore,
    GetTemplate: Fn(&Style) -> Option<Template>,
    WithTemplate: Fn(&Style, Template) -> Option<Style>,
{
    let mut history = vec![seed];
    let mut accepted = Vec::new();
    for round in 1..=context.max_rounds {
        let Some(incumbent) = history.last() else {
            break;
        };
        let incumbent_style = incumbent.style.clone();
        let incumbent_score = incumbent.score;
        let Some(template) = get_template(&incumbent_style) else {
            break;
        };
        let mut proposals = operators::enumerate_mutations(&template, context.budget);
        context.budget.truncate_mutations(&mut proposals, 1);

        let mut best: Option<HistoryEntry> = None;
        let mut best_family = "";
        for proposal in proposals {
            let Some(style) = with_template(&incumbent_style, proposal.template) else {
                continue;
            };
            let score = score_fn(&style);
            if !candidate_beats(&score, &incumbent_score) {
                continue;
            }
            if let Some(current) = best.as_ref()
                && !candidate_beats(&score, &current.score)
            {
                continue;
            }
            best_family = proposal.family.as_str();
            best = Some(HistoryEntry {
                name: proposal.name,
                style,
                score,
            });
        }
        let Some(winner) = best else {
            break;
        };
        if context.debug {
            eprintln!(
                "{} synthesis {} round {round}: accepted {} (+{} passes, {:+.3} similarity)",
                context.section,
                context.style_name,
                winner.name,
                winner.score.passes.saturating_sub(incumbent_score.passes),
                winner.score.similarity_sum - incumbent_score.similarity_sum,
            );
        }
        accepted.push(AcceptedMutation {
            name: winner.name.clone(),
            family: best_family,
        });
        history.push(winner);
    }
    RoundsOutcome { history, accepted }
}

/// Select the final incumbent under the held-out rejection gate.
///
/// The baseline is the seed-stage winner's held-out pass count. When the
/// final incumbent regresses it, walk the history backwards to the most
/// recent non-regressing incumbent, falling back to the seed itself. When
/// held-out validation is unavailable the final incumbent is kept, matching
/// the Phase 1 reporting-only behavior.
pub(super) fn heldout_gate<HeldOut>(
    history: &[HistoryEntry],
    heldout_of: &mut HeldOut,
    context: &RoundsContext<'_>,
) -> (usize, Option<HeldOutValidation>)
where
    HeldOut: FnMut(&Style) -> Option<HeldOutValidation>,
{
    let last_index = history.len().saturating_sub(1);
    let Some(final_entry) = history.last() else {
        return (0, None);
    };
    let final_validation = heldout_of(&final_entry.style);
    if last_index == 0 {
        return (0, final_validation);
    }
    let Some(baseline_entry) = history.first() else {
        return (last_index, final_validation);
    };
    let Some(baseline) = heldout_of(&baseline_entry.style) else {
        return (last_index, final_validation);
    };
    let Some(final_passes) = final_validation.map(|validation| validation.passes) else {
        return (last_index, None);
    };
    if final_passes >= baseline.passes {
        return (last_index, final_validation);
    }
    for index in (1..last_index).rev() {
        let Some(entry) = history.get(index) else {
            continue;
        };
        let Some(validation) = heldout_of(&entry.style) else {
            continue;
        };
        if validation.passes >= baseline.passes {
            if context.debug {
                eprintln!(
                    "{} synthesis {}: held-out regression; falling back to {}",
                    context.section, context.style_name, entry.name
                );
            }
            return (index, Some(validation));
        }
    }
    if context.debug {
        eprintln!(
            "{} synthesis {}: held-out regression; falling back to seed {}",
            context.section, context.style_name, baseline_entry.name
        );
    }
    (0, Some(baseline))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking and direct indexing are acceptable in tests."
)]
mod tests {
    use super::{HeldOutValidation, HistoryEntry, RoundsContext, heldout_gate, run_rounds};
    use crate::measured_citation::{CandidateBudget, CandidateScore};
    use citum_schema::Style;
    use citum_schema::template::{
        SimpleVariable, Template, TemplateComponent, TemplateGroup, TemplateVariable,
    };

    fn variable(variable: SimpleVariable) -> TemplateComponent {
        TemplateComponent::Variable(TemplateVariable {
            variable,
            ..Default::default()
        })
    }

    fn style_with_citation_template(template: Template) -> Style {
        Style {
            citation: Some(citum_schema::CitationSpec {
                template: Some(template.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn citation_template(style: &Style) -> Option<Template> {
        style
            .citation
            .as_ref()
            .and_then(|citation| citation.template.as_ref())
            .and_then(citum_schema::TemplateVariant::as_template)
            .map(<[_]>::to_vec)
    }

    fn with_citation_template(style: &Style, template: Template) -> Option<Style> {
        let mut mutated = style.clone();
        mutated.citation.as_mut()?.template = Some(template.into());
        Some(mutated)
    }

    fn score_by_component_count(style: &Style) -> CandidateScore {
        let passes = citation_template(style).map_or(0, |template| template.len());
        CandidateScore {
            passes,
            similarity_sum: 0.0,
            items: 10,
        }
    }

    fn context(max_rounds: usize) -> RoundsContext<'static> {
        RoundsContext {
            budget: CandidateBudget::default(),
            max_rounds,
            debug: false,
            style_name: "test-style",
            section: "citation",
        }
    }

    fn seed_entry(style: Style, score: CandidateScore) -> HistoryEntry {
        HistoryEntry {
            name: "inferred".to_string(),
            style,
            score,
        }
    }

    #[test]
    fn run_rounds_accepts_improving_mutation_then_stops_without_progress() {
        let template = vec![TemplateComponent::Group(TemplateGroup {
            group: vec![variable(SimpleVariable::Doi), variable(SimpleVariable::Url)],
            ..Default::default()
        })];
        let style = style_with_citation_template(template);
        let seed_score = score_by_component_count(&style);

        let outcome = run_rounds(
            seed_entry(style, seed_score),
            &score_by_component_count,
            &citation_template,
            &with_citation_template,
            &context(4),
        );

        let accepted: Vec<&str> = outcome
            .accepted
            .iter()
            .map(|mutation| mutation.name.as_str())
            .collect();
        assert_eq!(accepted, vec!["group-flatten-0"]);
        assert_eq!(outcome.history.len(), 2);
        assert_eq!(outcome.history[1].score.passes, 2);
    }

    #[test]
    fn run_rounds_respects_the_round_cap() {
        fn score_by_wrap_count(style: &Style) -> CandidateScore {
            let passes = citation_template(style).map_or(0, |template| {
                template
                    .iter()
                    .filter(|component| component.rendering().wrap.is_some())
                    .count()
            });
            CandidateScore {
                passes,
                similarity_sum: 0.0,
                items: 10,
            }
        }
        let template = vec![variable(SimpleVariable::Doi), variable(SimpleVariable::Url)];
        let style = style_with_citation_template(template);
        let seed_score = score_by_wrap_count(&style);

        let outcome = run_rounds(
            seed_entry(style, seed_score),
            &score_by_wrap_count,
            &citation_template,
            &with_citation_template,
            &context(1),
        );

        let accepted: Vec<&str> = outcome
            .accepted
            .iter()
            .map(|mutation| mutation.name.as_str())
            .collect();
        assert_eq!(accepted, vec!["affix-wrap-parentheses-0"]);
        assert_eq!(outcome.history.len(), 2);
    }

    #[test]
    fn run_rounds_without_a_template_keeps_the_seed() {
        let style = Style::default();
        let seed_score = CandidateScore {
            passes: 3,
            similarity_sum: 0.0,
            items: 10,
        };

        let outcome = run_rounds(
            seed_entry(style, seed_score),
            &score_by_component_count,
            &citation_template,
            &with_citation_template,
            &context(4),
        );

        assert!(outcome.accepted.is_empty());
        assert_eq!(outcome.history.len(), 1);
    }

    fn history_with_component_counts(counts: &[usize]) -> Vec<HistoryEntry> {
        counts
            .iter()
            .enumerate()
            .map(|(index, count)| {
                let template: Template =
                    (0..*count).map(|_| variable(SimpleVariable::Doi)).collect();
                HistoryEntry {
                    name: format!("entry-{index}"),
                    style: style_with_citation_template(template),
                    score: CandidateScore {
                        passes: *count,
                        similarity_sum: 0.0,
                        items: 10,
                    },
                }
            })
            .collect()
    }

    fn heldout_by_component_count(
        passes_by_count: &'static [(usize, usize)],
    ) -> impl FnMut(&Style) -> Option<HeldOutValidation> {
        move |style: &Style| {
            let count = citation_template(style).map_or(0, |template| template.len());
            passes_by_count
                .iter()
                .find(|(component_count, _)| *component_count == count)
                .map(|(_, passes)| HeldOutValidation {
                    passes: *passes,
                    items: 10,
                })
        }
    }

    #[test]
    fn heldout_gate_keeps_the_final_incumbent_without_regression() {
        let history = history_with_component_counts(&[1, 2, 3]);
        let mut heldout_of = heldout_by_component_count(&[(1, 4), (2, 4), (3, 6)]);

        let (selected_index, validation) = heldout_gate(&history, &mut heldout_of, &context(4));

        assert_eq!(selected_index, 2);
        assert_eq!(validation.map(|validation| validation.passes), Some(6));
    }

    #[test]
    fn heldout_gate_falls_back_to_the_best_non_regressing_incumbent() {
        let history = history_with_component_counts(&[1, 2, 3]);
        let mut heldout_of = heldout_by_component_count(&[(1, 5), (2, 5), (3, 3)]);

        let (selected_index, validation) = heldout_gate(&history, &mut heldout_of, &context(4));

        assert_eq!(selected_index, 1);
        assert_eq!(validation.map(|validation| validation.passes), Some(5));
    }

    #[test]
    fn heldout_gate_falls_back_to_the_seed_when_all_mutations_regress() {
        let history = history_with_component_counts(&[1, 2, 3]);
        let mut heldout_of = heldout_by_component_count(&[(1, 5), (2, 2), (3, 3)]);

        let (selected_index, validation) = heldout_gate(&history, &mut heldout_of, &context(4));

        assert_eq!(selected_index, 0);
        assert_eq!(validation.map(|validation| validation.passes), Some(5));
    }

    #[test]
    fn heldout_gate_keeps_the_final_incumbent_when_validation_is_unavailable() {
        let history = history_with_component_counts(&[1, 2]);
        let mut heldout_of = |_: &Style| None;

        let (selected_index, validation) = heldout_gate(&history, &mut heldout_of, &context(4));

        assert_eq!(selected_index, 1);
        assert!(validation.is_none());
    }
}
