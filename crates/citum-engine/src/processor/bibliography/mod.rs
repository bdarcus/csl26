/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Bibliography processing and rendering.
//!
//! This module owns bibliography entry generation, grouped rendering,
//! subsequent-author substitution, and the document-facing facade methods used
//! by the document processor.

mod compound;
mod grouping;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests;

use super::matching::Matcher;
use super::rendering::{CompoundRenderData, Renderer, RendererResources};
use super::run_state::FinalizedRun;
use super::{ProcessedReferences, Processor};
use crate::api::AnnotationStyle;
use crate::reference::Reference;
use crate::render::format::OutputFormat;
use crate::render::{ProcEntry, ProcTemplate};
use crate::values::ProcHints;
use citum_schema::grouping::BibliographyGroup;
use citum_schema::options::{Config, bibliography::BibliographyConfig};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Rendered bibliography block data for document integration.
#[derive(Debug, Clone, Default)]
pub(crate) struct RenderedBibliographyGroup {
    /// The resolved group heading, if one exists.
    pub(crate) heading: Option<String>,
    /// The rendered bibliography body without any document-level heading wrapper.
    pub(crate) body: String,
    /// Individual entries rendered in this block.
    pub(crate) entries: Vec<crate::render::ProcEntry>,
}

/// Combined document bibliography rendering output.
///
/// Returned by [`Processor::render_document_bibliography`] — the unified facade
/// used by the batch, session, and document-string rendering paths. Both fields
/// are computed from the same eligible subset so subsequent-author substitution
/// stays consistent between the rendered string and the per-entry data.
#[derive(Debug, Clone, Default)]
pub(crate) struct DocumentBibliography {
    /// The full rendered bibliography string for the document.
    pub(crate) content: String,
    /// Flat per-entry data, one entry per eligible reference.
    pub(crate) entries: Vec<crate::render::ProcEntry>,
}

/// Bibliography entries render in parallel (behind the opt-in `parallel`
/// feature) once a bibliography reaches this many entries; below the
/// threshold, thread-pool dispatch overhead isn't worth paying and entries
/// render sequentially instead. Measurements on an 8-core desktop found the
/// parallel path performance-neutral at 10–400 entries (rendering is
/// allocation-bound, not compute-bound); see
/// `docs/specs/PARALLEL_BIBLIOGRAPHY_RENDERING.md` for the numbers.
#[cfg(feature = "parallel")]
pub(crate) const PARALLEL_MIN_ENTRIES: usize = 32;

/// Resolve `(reference, entry_number)` pairs for `sorted_refs`, in order.
///
/// Reads each reference's already-assigned citation number from `run` when
/// present — numeric styles pre-assign these at `begin_run`
/// (`initialize_numeric_bibliography_numbers`) — and falls back to its
/// 1-based position in `sorted_refs` otherwise. This is a sequential,
/// read-only pass over `run`'s shared `citation_numbers` map, done up front
/// so the render step that follows (parallel or not) is free of further
/// lock contention.
fn number_sorted_refs<'a>(
    sorted_refs: impl Iterator<Item = &'a Reference>,
    run: &FinalizedRun,
) -> Vec<(&'a Reference, usize)> {
    let numbers = run
        .state()
        .citation_numbers
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    sorted_refs
        .enumerate()
        .map(|(index, reference)| {
            let entry_number = numbers
                .get(reference.id().unwrap_or_default().as_str())
                .copied()
                .unwrap_or(index + 1);
            (reference, entry_number)
        })
        .collect()
}

/// Resources needed to build a per-entry [`Renderer`] for one bibliography
/// render pass (flat or grouped), resolved and `Arc`-wrapped once per pass.
///
/// Hoisting the config merge and `Arc` construction out of the per-entry
/// loop matters twice over: it removes an O(entries) deep-clone cost from
/// the sequential path (the follow-up deferred in bean `csl26-qi7l`), and
/// it keeps the parallel path (see
/// [`render_numbered_refs_parallel`](Processor::render_numbered_refs_parallel))
/// from hammering the allocator with per-entry config clones across
/// threads. Each parallel task clones only the `Arc`s into a fresh
/// `Renderer`, never sharing one across threads — `Renderer` holds a
/// per-render scratch `RefCell` (`filtered_to_original_index`) that is
/// intentionally not `Sync`.
struct EntryRenderContext<'a> {
    /// The style to render with (group-overridden for grouped passes).
    style: &'a citum_schema::Style,
    /// Pre-calculated processing hints, group-scoped when applicable.
    hints: &'a HashMap<String, ProcHints>,
    /// The effective shared configuration, merged once per pass.
    config: Arc<Config>,
    /// The effective bibliography-only configuration, merged once per pass.
    bibliography_config: Arc<BibliographyConfig>,
    /// The finalized run providing citation numbers and note-order state.
    run: &'a FinalizedRun,
}

impl Processor {
    /// Return the manual bibliography groups that are currently enabled.
    ///
    /// Keeping the `groups_enabled` gate here ensures every bibliography
    /// rendering surface interprets a retained but disabled `groups:` block
    /// identically.
    fn effective_custom_groups(&self) -> Option<&[BibliographyGroup]> {
        self.style
            .bibliography
            .as_ref()
            .filter(|bibliography| bibliography.groups_enabled)
            .and_then(|bibliography| bibliography.groups.as_deref())
    }

    /// Build the [`EntryRenderContext`] for a flat (ungrouped) bibliography
    /// pass: processor-level style, hints, and merged configs.
    fn flat_render_context<'a>(&'a self, run: &'a FinalizedRun) -> EntryRenderContext<'a> {
        EntryRenderContext {
            style: &self.style,
            hints: &self.hints,
            config: Arc::new(self.get_bibliography_config().into_owned()),
            bibliography_config: Arc::new(self.get_bibliography_options().into_owned()),
            run,
        }
    }

    /// Build the `Renderer` used for one bibliography entry.
    fn entry_renderer<'a>(&'a self, ctx: &EntryRenderContext<'a>) -> Renderer<'a> {
        Renderer::new(
            RendererResources {
                style: ctx.style,
                bibliography: &self.bibliography,
                locale: &self.locale,
                config: ctx.config.clone(),
                bibliography_config: Some(ctx.bibliography_config.clone()),
                first_note_by_id: None,
            },
            ctx.hints,
            &ctx.run.state().citation_numbers,
            CompoundRenderData {
                set_by_ref: &self.compound_set_by_ref,
                member_index: &self.compound_member_index,
                sets: &self.compound_sets,
            },
            self.show_semantics,
            self.inject_ast_indices,
            self.abbreviation_map.as_ref(),
        )
    }

    /// Choose the sequential or parallel render path for `numbered_refs`
    /// and apply it.
    ///
    /// Parallel rendering requires both the `parallel` feature and
    /// `numbered_refs.len() >= PARALLEL_MIN_ENTRIES`; otherwise this falls
    /// back to the single-shared-`Renderer` sequential path.
    fn render_numbered_refs<'a, F>(
        &self,
        numbered_refs: &[(&'a Reference, usize)],
        ctx: &EntryRenderContext<'_>,
    ) -> Vec<(&'a Reference, Option<ProcTemplate>, Option<String>)>
    where
        F: OutputFormat<Output = String>,
    {
        #[cfg(feature = "parallel")]
        if numbered_refs.len() >= PARALLEL_MIN_ENTRIES {
            return self.render_numbered_refs_parallel::<F>(numbered_refs, ctx);
        }
        self.render_numbered_refs_sequential::<F>(numbered_refs, ctx)
    }

    /// Render numbered references through one shared `Renderer`, preserving
    /// input order.
    fn render_numbered_refs_sequential<'a, F>(
        &self,
        numbered_refs: &[(&'a Reference, usize)],
        ctx: &EntryRenderContext<'_>,
    ) -> Vec<(&'a Reference, Option<ProcTemplate>, Option<String>)>
    where
        F: OutputFormat<Output = String>,
    {
        let renderer = self.entry_renderer(ctx);
        numbered_refs
            .iter()
            .map(|&(reference, entry_number)| {
                (
                    reference,
                    renderer.process_bibliography_entry_with_format::<F>(reference, entry_number),
                    renderer.bibliography_marker_with_format::<F>(reference, entry_number),
                )
            })
            .collect()
    }

    /// Render numbered references across the rayon thread pool, preserving
    /// input order.
    ///
    /// Builds a fresh `Renderer` per task from `ctx`'s `Arc`s (cheap; see
    /// [`EntryRenderContext`] for why one `Renderer` cannot be shared across
    /// threads). `par_iter` over a slice is order-preserving under
    /// `collect`, so the subsequent-author-substitution post-pass sees the
    /// same sequence it would under
    /// [`render_numbered_refs_sequential`](Self::render_numbered_refs_sequential).
    #[cfg(feature = "parallel")]
    fn render_numbered_refs_parallel<'a, F>(
        &self,
        numbered_refs: &[(&'a Reference, usize)],
        ctx: &EntryRenderContext<'_>,
    ) -> Vec<(&'a Reference, Option<ProcTemplate>, Option<String>)>
    where
        F: OutputFormat<Output = String>,
    {
        use rayon::prelude::*;
        numbered_refs
            .par_iter()
            .map(|&(reference, entry_number)| {
                let renderer = self.entry_renderer(ctx);
                (
                    reference,
                    renderer.process_bibliography_entry_with_format::<F>(reference, entry_number),
                    renderer.bibliography_marker_with_format::<F>(reference, entry_number),
                )
            })
            .collect()
    }

    /// Create a bibliography renderer with effective shared and bibliography-only config.
    fn with_bibliography_renderer<T>(
        &self,
        run: &FinalizedRun,
        render: impl FnOnce(Renderer<'_>) -> T,
    ) -> T {
        let bibliography_shared_config = self.get_bibliography_config();
        let bibliography_config = self.get_bibliography_options().into_owned();
        let renderer = Renderer::new(
            RendererResources {
                style: &self.style,
                bibliography: &self.bibliography,
                locale: &self.locale,
                config: Arc::new(bibliography_shared_config.into_owned()),
                bibliography_config: Some(Arc::new(bibliography_config)),
                first_note_by_id: None,
            },
            &self.hints,
            &run.state().citation_numbers,
            CompoundRenderData {
                set_by_ref: &self.compound_set_by_ref,
                member_index: &self.compound_member_index,
                sets: &self.compound_sets,
            },
            self.show_semantics,
            self.inject_ast_indices,
            self.abbreviation_map.as_ref(),
        );

        render(renderer)
    }

    /// Process sorted references and apply subsequent-author substitution.
    ///
    /// Returns bibliography entries with optional author substitution applied.
    ///
    /// This is the core iterator for flat bibliography rendering. Entry
    /// numbers are resolved sequentially first (a single pass over `run`'s
    /// shared `citation_numbers` map), then entries render via
    /// [`render_numbered_refs`](Self::render_numbered_refs) — sequentially,
    /// or across the rayon thread pool once the bibliography is large enough
    /// (see [`PARALLEL_MIN_ENTRIES`]) — and finally
    /// [`apply_substitution_post_pass`](Self::apply_substitution_post_pass)
    /// walks the (order-preserved) results sequentially to apply
    /// subsequent-author substitution, which depends on cite-order.
    fn process_sorted_refs<'a, I, F>(&self, sorted_refs: I, run: &FinalizedRun) -> Vec<ProcEntry>
    where
        I: Iterator<Item = &'a Reference>,
        F: OutputFormat<Output = String>,
    {
        let ctx = self.flat_render_context(run);
        let numbered_refs = number_sorted_refs(sorted_refs, run);
        let rendered = self.render_numbered_refs::<F>(&numbered_refs, &ctx);

        let substitute = ctx
            .bibliography_config
            .subsequent_author_substitute
            .as_ref();
        self.apply_substitution_post_pass::<F>(rendered, substitute, &ctx)
    }

    /// Apply subsequent-author substitution to already-rendered entries and
    /// assemble [`ProcEntry`]s, in order.
    ///
    /// This is the sequential part of bibliography rendering: substitution
    /// depends on the *previous successfully rendered* reference, so it
    /// cannot itself run in parallel. `rendered` must already be in final
    /// bibliography order (as produced by
    /// [`render_numbered_refs`](Self::render_numbered_refs), parallel or
    /// not); entries whose render produced `None` are skipped entirely and
    /// do not advance the "previous reference" used for contributor
    /// matching.
    fn apply_substitution_post_pass<F>(
        &self,
        rendered: Vec<(&Reference, Option<ProcTemplate>, Option<String>)>,
        substitute: Option<&String>,
        ctx: &EntryRenderContext<'_>,
    ) -> Vec<ProcEntry>
    where
        F: OutputFormat<Output = String>,
    {
        let renderer = substitute.map(|_| self.entry_renderer(ctx));
        let mut bibliography = Vec::with_capacity(rendered.len());
        let mut previous_reference: Option<&Reference> = None;

        for (reference, processed, marker) in rendered {
            let Some(mut processed) = processed else {
                continue;
            };

            if let Some(substitute_string) = substitute
                && let Some(renderer) = renderer.as_ref()
                && let Some(previous) = previous_reference
                && self.contributors_match(previous, reference)
            {
                renderer
                    .apply_author_substitution_with_format::<F>(&mut processed, substitute_string);
            }

            let ref_id = reference.id().unwrap_or_default().to_string();
            bibliography.push(ProcEntry {
                id: ref_id,
                marker,
                template: processed,
                metadata: self.extract_metadata(reference, ctx),
            });
            previous_reference = Some(reference);
        }

        bibliography
    }

    /// Process all bibliography references and render them.
    ///
    /// This is a one-shot convenience wrapper: it begins a throwaway
    /// [`super::run_state::RunState`] internally, so it has no continuity
    /// with any citations processed elsewhere. Use
    /// [`Processor::process_references_with_format`] with an explicit,
    /// shared `FinalizedRun` to render a bibliography that reflects prior
    /// citation registration in the same document.
    pub fn process_references(&self) -> ProcessedReferences {
        let run = self.begin_run().finalize();
        self.process_references_with_format::<crate::render::plain::PlainText>(&run)
    }

    /// Process all bibliography references using the requested output format.
    ///
    /// This preserves format-specific inline markup in per-entry API output.
    /// `run` should reflect all citations already processed for this
    /// document (or be a fresh [`Processor::begin_run`] for a standalone
    /// bibliography with no citations); see
    /// [`Processor::process_references_with_format_standalone`] for a
    /// one-shot convenience.
    pub fn process_references_with_format<F>(&self, run: &FinalizedRun) -> ProcessedReferences
    where
        F: OutputFormat<Output = String>,
    {
        if self.style.bibliography.is_none() {
            return ProcessedReferences::default();
        }
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        let bibliography = self.process_sorted_refs::<_, F>(sorted_refs.iter().copied(), run);
        ProcessedReferences {
            bibliography,
            citations: None,
        }
    }

    /// Process only the selected bibliography entries, in bibliography sort order.
    ///
    /// Mirrors the flat path inside
    /// [`render_selected_bibliography_with_format_and_annotations`] so that
    /// per-entry `text` and subsequent-author substitution are computed against
    /// the same subset that produced `content` — not the full loaded
    /// bibliography. This matters for subsequent-author substitution: an uncited
    /// predecessor must not cause the first cited entry to receive `———`.
    pub(crate) fn process_selected_references_with_format<F, I>(
        &self,
        item_ids: I,
        run: &FinalizedRun,
    ) -> ProcessedReferences
    where
        F: OutputFormat<Output = String>,
        I: IntoIterator<Item = String>,
    {
        if self.style.bibliography.is_none() {
            return ProcessedReferences::default();
        }
        let selected: HashSet<String> = item_ids.into_iter().collect();
        let sorted_refs = self.sort_references(self.bibliography.values().collect());
        let bibliography = self.process_sorted_refs::<_, F>(
            sorted_refs
                .iter()
                .filter(|r| r.id().as_deref().is_some_and(|id| selected.contains(id)))
                .copied(),
            run,
        );
        ProcessedReferences {
            bibliography,
            citations: None,
        }
    }

    /// Process and render a bibliography entry.
    pub fn process_bibliography_entry(
        &self,
        reference: &Reference,
        entry_number: usize,
        run: &FinalizedRun,
    ) -> Option<ProcTemplate> {
        self.style.bibliography.as_ref()?;
        self.with_bibliography_renderer(run, |renderer| {
            renderer.process_bibliography_entry(reference, entry_number)
        })
    }

    /// Process a bibliography entry with specific format.
    pub fn process_bibliography_entry_with_format<F>(
        &self,
        reference: &Reference,
        entry_number: usize,
        run: &FinalizedRun,
    ) -> Option<ProcTemplate>
    where
        F: OutputFormat<Output = String>,
    {
        self.style.bibliography.as_ref()?;
        self.with_bibliography_renderer(run, |renderer| {
            renderer.process_bibliography_entry_with_format::<F>(reference, entry_number)
        })
    }

    /// Check whether primary contributors match between two references.
    ///
    /// Used for subsequent author substitution in bibliographies.
    pub fn contributors_match(&self, prev: &Reference, current: &Reference) -> bool {
        let config = self.style.options.as_ref().unwrap_or(&self.default_config);
        let matcher = Matcher::new(&self.style, config, &self.locale);
        matcher.contributors_match(prev, current)
    }

    /// Render the bibliography to a string using a specific format.
    pub fn render_bibliography_with_format<F>(&self, run: &FinalizedRun) -> String
    where
        F: OutputFormat<Output = String>,
    {
        self.render_bibliography_with_format_and_annotations::<F>(None, None, run)
    }

    /// Render the bibliography to a string with annotations.
    pub fn render_bibliography_with_format_and_annotations<F>(
        &self,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        self.render_selected_bibliography_with_format_and_annotations::<F, _>(
            self.bibliography.keys().cloned().collect::<Vec<_>>(),
            annotations,
            annotation_style,
            run,
        )
    }

    /// Render a selected bibliography subset to a string using a specific format.
    pub fn render_selected_bibliography_with_format<F, I>(
        &self,
        item_ids: I,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
        I: IntoIterator<Item = String>,
    {
        self.render_selected_bibliography_with_format_and_annotations::<F, _>(
            item_ids, None, None, run,
        )
    }

    /// Render a selected bibliography subset to a string with annotations.
    ///
    /// Orchestrates the choice between:
    /// 1. Custom bibliography groups (selectors and headings).
    /// 2. Automatic sort partitioning with sections (headings only).
    /// 3. Standard flat rendering.
    pub fn render_selected_bibliography_with_format_and_annotations<F, I>(
        &self,
        item_ids: I,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
        I: IntoIterator<Item = String>,
    {
        if self.style.bibliography.is_none() {
            return String::new();
        }
        let selected: HashSet<String> = item_ids.into_iter().collect();

        // 1. Check for custom bibliography groups
        if let Some(groups) = self.effective_custom_groups() {
            let all_entries = self.sorted_id_stubs();
            return self.render_with_custom_groups_filtered::<F>(
                &all_entries,
                groups,
                &selected,
                annotations,
                annotation_style,
                run,
            );
        }

        // 2. Check for automatic sort partitioning with sections
        let bibliography_options = self.get_bibliography_options();
        if let Some(partitioning) = bibliography_options.sort_partitioning.as_ref()
            && crate::sort_partitioning::should_render_sections(partitioning)
        {
            let all_sorted = self.sort_references(self.bibliography.values().collect());
            let selected_sorted: Vec<&Reference> = all_sorted
                .into_iter()
                .filter(|r| r.id().as_deref().is_some_and(|id| selected.contains(id)))
                .collect();
            return self.render_with_partition_sections::<F>(
                selected_sorted,
                partitioning,
                annotations,
                annotation_style,
                run,
            );
        }

        // 3. Fallback to flat rendering
        let sorted_refs = self.sort_references(self.bibliography.values().collect());

        let bibliography = self.process_sorted_refs::<_, F>(
            sorted_refs
                .iter()
                .filter(|r| r.id().as_deref().is_some_and(|id| selected.contains(id)))
                .copied(),
            run,
        );

        let bibliography = self.merge_compound_entries::<F>(bibliography, run);
        crate::render::refs_to_string_with_format::<F>(bibliography, annotations, annotation_style)
    }

    /// Render the entire bibliography to a formatted string.
    ///
    /// One-shot convenience wrapper: begins a throwaway run internally, so
    /// it has no continuity with any citations processed elsewhere. Use
    /// [`Processor::render_bibliography_with_format`] with an explicit,
    /// shared `FinalizedRun` for a bibliography that reflects prior citation
    /// registration.
    pub fn render_bibliography(&self) -> String {
        let run = self.begin_run().finalize();
        self.render_bibliography_with_format::<crate::render::plain::PlainText>(&run)
    }

    /// One-shot convenience for [`Processor::process_references_with_format`]:
    /// begins a throwaway run internally.
    pub fn process_references_with_format_standalone<F>(&self) -> ProcessedReferences
    where
        F: OutputFormat<Output = String>,
    {
        let run = self.begin_run().finalize();
        self.process_references_with_format::<F>(&run)
    }

    /// One-shot convenience for [`Processor::process_bibliography_entry`]:
    /// begins a throwaway run internally.
    pub fn process_bibliography_entry_standalone(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate> {
        let run = self.begin_run().finalize();
        self.process_bibliography_entry(reference, entry_number, &run)
    }

    /// One-shot convenience for [`Processor::process_bibliography_entry_with_format`]:
    /// begins a throwaway run internally.
    pub fn process_bibliography_entry_with_format_standalone<F>(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate>
    where
        F: OutputFormat<Output = String>,
    {
        let run = self.begin_run().finalize();
        self.process_bibliography_entry_with_format::<F>(reference, entry_number, &run)
    }

    /// One-shot convenience for [`Processor::render_bibliography_with_format`]:
    /// begins a throwaway run internally.
    pub fn render_bibliography_with_format_standalone<F>(&self) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let run = self.begin_run().finalize();
        self.render_bibliography_with_format::<F>(&run)
    }

    /// One-shot convenience for
    /// [`Processor::render_bibliography_with_format_and_annotations`]:
    /// begins a throwaway run internally.
    pub fn render_bibliography_with_format_and_annotations_standalone<F>(
        &self,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let run = self.begin_run().finalize();
        self.render_bibliography_with_format_and_annotations::<F>(
            annotations,
            annotation_style,
            &run,
        )
    }

    /// One-shot convenience for [`Processor::render_selected_bibliography_with_format`]:
    /// begins a throwaway run internally.
    pub fn render_selected_bibliography_with_format_standalone<F, I>(&self, item_ids: I) -> String
    where
        F: OutputFormat<Output = String>,
        I: IntoIterator<Item = String>,
    {
        let run = self.begin_run().finalize();
        self.render_selected_bibliography_with_format::<F, I>(item_ids, &run)
    }

    /// One-shot convenience for
    /// [`Processor::render_selected_bibliography_with_format_and_annotations`]:
    /// begins a throwaway run internally.
    pub fn render_selected_bibliography_with_format_and_annotations_standalone<F, I>(
        &self,
        item_ids: I,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
    ) -> String
    where
        F: OutputFormat<Output = String>,
        I: IntoIterator<Item = String>,
    {
        let run = self.begin_run().finalize();
        self.render_selected_bibliography_with_format_and_annotations::<F, I>(
            item_ids,
            annotations,
            annotation_style,
            &run,
        )
    }
}
