/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Grouped bibliography rendering with configurable selectors and sorting.

use super::RenderedBibliographyGroup;
use crate::api::AnnotationStyle;
use crate::grouping::SelectorEvaluator;
use crate::processor::FinalizedRun;
use crate::processor::Processor;
use crate::processor::disambiguation::Disambiguator;
use crate::reference::{Bibliography, Reference};
use crate::render::ProcEntry;
use crate::render::format::{OutputFormat, ProcEntryMetadata};
use crate::sorting::ReferenceSorter;
use crate::values::{
    ProcHints, RenderContext, RenderOptions, format_contributors_short, resolve_multilingual_name,
    resolve_multilingual_string,
};
use citum_schema::grouping::{BibliographyGroup, DisambiguationScope, GroupHeading};
use citum_schema::options::{BibliographyPartitionHeading, BibliographySortPartitioning};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::EntryRenderContext;

/// Products requested from the shared flat bibliography render pass.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FlatBibliographyOutput {
    /// Produce only the formatted bibliography string.
    ContentOnly,
    /// Produce the formatted string and flat per-reference entry data.
    ContentAndEntries,
}

impl Processor {
    /// Resolve a localized or literal group heading.
    pub(super) fn resolve_group_heading(&self, heading: &GroupHeading) -> Option<String> {
        match heading {
            GroupHeading::Literal { literal } => Some(literal.clone()),
            GroupHeading::Term { term, form } => self.locale.resolved_general_term(
                term,
                &form.clone().unwrap_or(citum_schema::locale::TermForm::Long),
                None,
            ),
            GroupHeading::Localized { localized } => self.resolve_localized_heading(localized),
        }
    }

    /// Resolve a localized heading map based on the processor locale.
    ///
    /// Matches in order:
    /// 1. Exact locale (e.g., "en-GB")
    /// 2. Primary language (e.g., "en")
    /// 3. Style default locale
    /// 4. en-US fallback
    /// 5. First alphabetically defined key
    fn resolve_localized_heading(&self, localized: &HashMap<String, String>) -> Option<String> {
        fn language_tag(locale: &str) -> &str {
            locale.split('-').next().unwrap_or(locale)
        }

        let mut candidates = Vec::new();
        let mut push_candidate = |locale: &str| {
            let candidate = locale.to_string();
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        push_candidate(&self.locale.locale);
        push_candidate(language_tag(&self.locale.locale));

        if let Some(default_locale) = self.style.info.default_locale.as_deref() {
            push_candidate(default_locale);
            push_candidate(language_tag(default_locale));
        }

        push_candidate("en-US");
        push_candidate("en");

        for locale in candidates {
            if let Some(value) = localized.get(&locale) {
                return Some(value.clone());
            }
        }

        localized
            .iter()
            .min_by(|left, right| left.0.cmp(right.0))
            .map(|(_locale, value)| value.clone())
    }

    /// Resolve a bibliography partition heading.
    fn resolve_partition_heading(&self, heading: &BibliographyPartitionHeading) -> Option<String> {
        match heading {
            BibliographyPartitionHeading::Literal { literal } => Some(literal.clone()),
            BibliographyPartitionHeading::Term { term, form } => self.locale.resolved_general_term(
                term,
                &form.clone().unwrap_or(citum_schema::locale::TermForm::Long),
                None,
            ),
            BibliographyPartitionHeading::Localized { localized } => {
                self.resolve_localized_heading(localized)
            }
        }
    }

    /// Find unassigned bibliography entries that match a group's selector.
    fn collect_matching_group_refs<'a>(
        &'a self,
        bibliography: &'a [ProcEntry],
        assigned: &HashSet<String>,
        evaluator: &SelectorEvaluator<'_>,
        group: &BibliographyGroup,
    ) -> Vec<&'a Reference> {
        bibliography
            .iter()
            .filter(|entry| !assigned.contains(&entry.id))
            .filter_map(|entry| {
                self.bibliography
                    .get(&entry.id)
                    .filter(|reference| evaluator.matches(reference, &group.selector))
            })
            .collect()
    }

    /// Returns `ProcEntry` stubs with only `id` populated, in sort order.
    ///
    /// Used for grouping paths that only need IDs for selector matching — avoids
    /// the full PlainText render pass that `process_references` performs.
    /// `pub(crate)` so callers that render multiple bibliography blocks in one
    /// pass (or exercise that path in tests) can compute this once and share
    /// it, instead of each block independently re-sorting the bibliography.
    pub(crate) fn sorted_id_stubs(&self) -> Vec<ProcEntry> {
        // Numeric citation numbers are already populated by `Processor::begin_run`,
        // which every `FinalizedRun` this module consumes was produced from.
        self.sort_references(self.bibliography.values().collect())
            .into_iter()
            .filter_map(|r| {
                r.id().map(|id| ProcEntry {
                    id: id.to_string(),
                    marker: None,
                    template: vec![],
                    metadata: ProcEntryMetadata::default(),
                })
            })
            .collect()
    }

    /// Mark references as assigned to a bibliography group.
    fn mark_group_members_assigned(assigned: &mut HashSet<String>, references: &[&Reference]) {
        for reference in references {
            if let Some(id) = reference.id() {
                assigned.insert(id.to_string());
            }
        }
    }

    /// Calculate disambiguation hints locally within a bibliography group.
    ///
    /// Only calculates hints if the group specifies local disambiguation scope.
    fn build_group_local_hints(
        &self,
        sorted_refs: &[&Reference],
        group: &BibliographyGroup,
    ) -> Option<HashMap<String, ProcHints>> {
        if !matches!(group.disambiguate, Some(DisambiguationScope::Locally)) {
            return None;
        }

        let mut group_bibliography = Bibliography::new();
        for reference in sorted_refs {
            group_bibliography.insert(
                reference.id().unwrap_or_default().to_string(),
                (*reference).clone(),
            );
        }

        let resolved_sort = group
            .sort
            .as_ref()
            .map(citum_schema::GroupSortEntry::resolve);
        let bibliography_config = self.get_bibliography_config();
        let mut disambiguator = if let Some(sort) = resolved_sort.as_ref() {
            Disambiguator::with_group_sort(
                &group_bibliography,
                &bibliography_config,
                &bibliography_config,
                &self.locale,
                sort,
            )
        } else {
            Disambiguator::new(
                &group_bibliography,
                &bibliography_config,
                &bibliography_config,
                &self.locale,
            )
        };

        if let Some(spec) = self.style.citation.as_ref() {
            disambiguator = disambiguator.with_citation_spec(spec);
        }
        if let Some(spec) = self.style.bibliography.as_ref() {
            disambiguator = disambiguator.with_bibliography_spec(spec);
        }

        Some(disambiguator.calculate_hints())
    }

    /// Resolve the effective style to use for a bibliography group.
    fn effective_group_style<'a>(
        &'a self,
        group: &'a BibliographyGroup,
    ) -> Cow<'a, citum_schema::Style> {
        if let Some(group_template) = &group.template {
            let mut local_style = self.style.clone();
            if let Some(bibliography) = local_style.bibliography.as_mut() {
                bibliography.template = Some(group_template.clone().into());
            }
            Cow::Owned(local_style)
        } else {
            Cow::Borrowed(&self.style)
        }
    }

    /// Render bibliography entries for a specific group.
    ///
    /// Entry numbers are resolved sequentially first, then entries render
    /// via [`render_numbered_refs`](Self::render_numbered_refs) —
    /// sequentially through one shared `Renderer`, or across the rayon
    /// thread pool (one fresh `Renderer` per task; see
    /// [`EntryRenderContext`]) once the group is large enough — and finally
    /// [`apply_substitution_post_pass`](super::Processor::apply_substitution_post_pass)
    /// applies subsequent-author substitution sequentially over the
    /// (order-preserved) results.
    fn render_group_entries<F>(
        &self,
        _bibliography: &[ProcEntry],
        sorted_refs: Vec<&Reference>,
        group: &BibliographyGroup,
        local_hints: Option<&HashMap<String, ProcHints>>,
        run: &FinalizedRun,
    ) -> Vec<ProcEntry>
    where
        F: OutputFormat<Output = String>,
    {
        // Always process entries with format F so that group components (pre_formatted=true)
        // contain markup in the target format rather than PlainText (_..._).
        let effective_style = self.effective_group_style(group);
        let ctx = EntryRenderContext {
            style: &effective_style,
            hints: local_hints.unwrap_or(&self.hints),
            config: Arc::new(self.get_bibliography_config().into_owned()),
            bibliography_config: Arc::new(self.get_bibliography_options().into_owned()),
            run,
        };

        let numbered_refs = super::number_sorted_refs(sorted_refs.into_iter(), run);
        let rendered = self.render_numbered_refs::<F>(&numbered_refs, &ctx);

        let substitute = ctx
            .bibliography_config
            .subsequent_author_substitute
            .as_ref();
        self.apply_substitution_post_pass::<F>(rendered, substitute, &ctx)
    }

    /// Append a rendered bibliography group to the output string.
    fn append_rendered_group<F>(
        &self,
        result: &mut String,
        group: &BibliographyGroup,
        entries: Vec<ProcEntry>,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        suppress_heading: bool,
    ) where
        F: OutputFormat<Output = String>,
    {
        if !result.is_empty() {
            result.push_str("\n\n");
        }

        if !suppress_heading
            && let Some(heading) = group
                .heading
                .as_ref()
                .and_then(|group_heading| self.resolve_group_heading(group_heading))
        {
            result.push_str(&self.render_group_heading::<F>(&heading));
        }

        result.push_str(&crate::render::refs_to_string_with_format::<F>(
            entries,
            annotations,
            annotation_style,
        ));
    }

    /// Append a rendered bibliography partition to the output string.
    fn append_rendered_partition<F>(
        &self,
        result: &mut String,
        heading: Option<&BibliographyPartitionHeading>,
        entries: Vec<ProcEntry>,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
    ) where
        F: OutputFormat<Output = String>,
    {
        if !result.is_empty() {
            result.push_str("\n\n");
        }

        if let Some(heading) =
            heading.and_then(|group_heading| self.resolve_partition_heading(group_heading))
        {
            result.push_str(&self.render_group_heading::<F>(&heading));
        }

        result.push_str(&crate::render::refs_to_string_with_format::<F>(
            entries,
            annotations,
            annotation_style,
        ));
    }

    /// Orchestrate the rendering of automatic bibliography partitions with headings.
    pub(super) fn render_with_partition_sections<F>(
        &self,
        sorted_refs: Vec<&Reference>,
        partitioning: &BibliographySortPartitioning,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let mut result = String::new();

        for (partition_key, references) in
            crate::sort_partitioning::partition_references(sorted_refs, &self.locale, partitioning)
        {
            let heading = partition_key
                .as_ref()
                .and_then(|key| partitioning.headings.get(key));
            let entries = self.merge_compound_entries::<F>(
                self.process_sorted_refs::<_, F>(references.into_iter(), run),
                run,
            );
            self.append_rendered_partition::<F>(
                &mut result,
                heading,
                entries,
                annotations,
                annotation_style,
            );
        }

        fmt.finish(result)
    }

    /// Render a filtered subset of entries using custom bibliography grouping.
    ///
    /// This uses a two-pass grouping strategy:
    /// 1. Collect and render all populated groups.
    /// 2. Determine if heading suppression applies (only one group populated).
    /// 3. Append groups and any remaining unassigned entries.
    pub(super) fn render_with_custom_groups_filtered<F>(
        &self,
        all_entries: &[ProcEntry],
        groups: &[BibliographyGroup],
        selected: &HashSet<String>,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let cited_ids = &run.state().cited_ids;
        let evaluator = SelectorEvaluator::new(cited_ids);
        let bibliography_config = self.get_bibliography_config();
        let mut sorter =
            ReferenceSorter::with_bibliography_config(&self.locale, &bibliography_config);
        if let Some(spec) = self.style.bibliography.as_ref() {
            sorter = sorter.with_bibliography_spec(spec);
        }

        let mut assigned = HashSet::new();
        let mut result = String::new();

        // First pass: collect all populated groups with their rendered entries
        let mut populated_groups: Vec<(&BibliographyGroup, Vec<ProcEntry>)> = Vec::new();

        for group in groups {
            let matching_refs =
                self.collect_matching_group_refs(all_entries, &assigned, &evaluator, group);

            let matching_refs: Vec<&Reference> = matching_refs
                .into_iter()
                .filter(|r| r.id().as_deref().is_some_and(|id| selected.contains(id)))
                .collect();

            if matching_refs.is_empty() {
                continue;
            }

            Self::mark_group_members_assigned(&mut assigned, &matching_refs);

            let sorted_refs = if let Some(sort_spec) = &group.sort {
                sorter.sort_references(matching_refs, &sort_spec.resolve())
            } else {
                matching_refs
            };
            let local_hints = self.build_group_local_hints(&sorted_refs, group);
            let entries = self.merge_compound_entries::<F>(
                self.render_group_entries::<F>(
                    all_entries,
                    sorted_refs,
                    group,
                    local_hints.as_ref(),
                    run,
                ),
                run,
            );

            populated_groups.push((group, entries));
        }

        // Compute unassigned entries to determine if heading suppression applies
        let unassigned_refs: Vec<&Reference> = all_entries
            .iter()
            .filter(|entry| !assigned.contains(&entry.id) && selected.contains(&entry.id))
            .filter_map(|entry| self.bibliography.get(&entry.id))
            .collect();

        let suppress_heading = populated_groups.len() == 1 && unassigned_refs.is_empty();

        // Second pass: render populated groups with optional heading suppression
        for (group, entries) in populated_groups {
            self.append_rendered_group::<F>(
                &mut result,
                group,
                entries,
                annotations,
                annotation_style,
                suppress_heading,
            );
        }

        self.append_unassigned_entries_filtered::<F>(
            &mut result,
            all_entries,
            &assigned,
            selected,
            annotations,
            annotation_style,
            run,
        );
        fmt.finish(result)
    }

    /// Append unassigned bibliography entries to the output string.
    #[allow(
        clippy::too_many_arguments,
        reason = "internal helper, all params load-bearing"
    )]
    fn append_unassigned_entries_filtered<F>(
        &self,
        result: &mut String,
        bibliography: &[ProcEntry],
        assigned: &HashSet<String>,
        selected: &HashSet<String>,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) where
        F: OutputFormat<Output = String>,
    {
        let unassigned_refs: Vec<&Reference> = bibliography
            .iter()
            .filter(|entry| !assigned.contains(&entry.id) && selected.contains(&entry.id))
            .filter_map(|entry| self.bibliography.get(&entry.id))
            .collect();

        if unassigned_refs.is_empty() {
            return;
        }

        // Re-process references to ensure correct author substitution and disambiguation
        // within the unassigned subset.
        let unassigned = self.merge_compound_entries::<F>(
            self.process_sorted_refs::<_, F>(unassigned_refs.into_iter(), run),
            run,
        );

        if !result.is_empty() {
            result.push_str("\n\n");
        }

        result.push_str(&crate::render::refs_to_string_with_format::<F>(
            unassigned,
            annotations,
            annotation_style,
        ));
    }

    /// Render already-merged compound entries as a flat bibliography.
    ///
    /// The compound path renders every configured member before merging. A
    /// compound set is a single bibliographic unit: a cited-only document
    /// retains a merged row if *any* configured member of that set is cited,
    /// showing the full row (leader plus every tail) rather than hiding
    /// non-cited members. Rows outside any compound set still filter by
    /// their own cited status. All-references callers render every merged
    /// row.
    fn render_flat_compound_entries<F>(
        &self,
        bibliography: &[ProcEntry],
        restrict_to_cited: bool,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let selected = if restrict_to_cited {
            let cited_ids = &run.state().cited_ids;
            let compound_groups = &run.state().compound_groups;
            let ref_to_group = Self::build_compound_group_lookup(compound_groups);
            let row_is_cited = |id: &str| match ref_to_group.get(id) {
                Some(group_number) => compound_groups
                    .get(group_number)
                    .is_some_and(|members| members.iter().any(|member| cited_ids.contains(member))),
                None => cited_ids.contains(id),
            };
            Cow::Owned(
                bibliography
                    .iter()
                    .filter(|entry| row_is_cited(&entry.id))
                    .cloned()
                    .collect(),
            )
        } else {
            Cow::Borrowed(bibliography)
        };

        let result = if selected.is_empty() {
            String::new()
        } else {
            crate::render::refs_to_string_slice_with_format::<F>(
                selected.as_ref(),
                annotations,
                annotation_style,
            )
        };

        fmt.finish(result)
    }

    /// Render all loaded references with the style's effective grouping policy.
    ///
    /// Enabled manual groups take precedence over automatic partition sections,
    /// which in turn take precedence over flat rendering. Group selectors apply
    /// to individual references before compound numeric rows are merged, so each
    /// rendered group only includes the members that matched its selector.
    pub fn render_grouped_bibliography_with_format<F>(&self, run: &FinalizedRun) -> String
    where
        F: OutputFormat<Output = String>,
    {
        self.render_grouped_bibliography_with_format_and_annotations::<F>(None, None, run)
    }

    /// Render the bibliography with grouping and annotations.
    pub fn render_grouped_bibliography_with_format_and_annotations<F>(
        &self,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        if self.style.bibliography.is_none() {
            return String::new();
        }
        self.render_grouped_bibliography_inner::<F>(false, annotations, annotation_style, run)
    }

    /// One-shot convenience for [`Processor::render_grouped_bibliography_with_format`]:
    /// begins a throwaway run internally.
    pub fn render_grouped_bibliography_with_format_standalone<F>(&self) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let run = self.begin_run().finalize();
        self.render_grouped_bibliography_with_format::<F>(&run)
    }

    /// One-shot convenience for
    /// [`Processor::render_grouped_bibliography_with_format_and_annotations`]:
    /// begins a throwaway run internally.
    pub fn render_grouped_bibliography_with_format_and_annotations_standalone<F>(
        &self,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let run = self.begin_run().finalize();
        self.render_grouped_bibliography_with_format_and_annotations::<F>(
            annotations,
            annotation_style,
            &run,
        )
    }

    /// Unified document bibliography facade — returns content and per-entry data together.
    ///
    /// This is the single entry point for all document-context bibliography rendering:
    /// batch (`format_document`), interactive session (`DocumentSession`), and the
    /// document-string (`process_document`) path all funnel through here.
    ///
    /// When `restrict_to_cited` is `true` (the document case), only references present
    /// in `run`'s `cited_ids` — cited in-text or registered via `nocite` — are included.
    /// When `false`, all loaded references are eligible; this hook is reserved for the
    /// `allrefs` escape hatch (csl26-f9ri) and is not yet exposed publicly.
    ///
    /// Both `content` and `entries` are computed from the same eligible subset so
    /// subsequent-author substitution stays consistent across both outputs.
    ///
    /// The flat and sort-partitioned-sections cases render each eligible
    /// reference's template exactly once — see
    /// [`render_flat_bibliography`](Self::render_flat_bibliography) — instead
    /// of once for `content` and again for `entries`. Custom
    /// bibliography groups (`style.bibliography.groups`) need group-local
    /// disambiguation and per-group templates that a flat entry list can't
    /// carry. Compound-numeric merging needs to see every configured group
    /// member whether cited or not (see
    /// [`merge_compound_entries`](Self::merge_compound_entries)), so it keeps
    /// its historical full-member content pass.
    pub(crate) fn render_document_bibliography<F>(
        &self,
        restrict_to_cited: bool,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> super::DocumentBibliography
    where
        F: OutputFormat<Output = String>,
    {
        if self.style.bibliography.is_none() {
            return super::DocumentBibliography::default();
        }
        let has_custom_groups = self.effective_custom_groups().is_some();

        if has_custom_groups || !run.state().compound_groups.is_empty() {
            let content = self.render_grouped_bibliography_inner::<F>(
                restrict_to_cited,
                annotations,
                annotation_style,
                run,
            );
            let cited_ids: Vec<String> = run.state().cited_ids.iter().cloned().collect();
            let entries = if restrict_to_cited {
                self.process_selected_references_with_format::<F, _>(cited_ids, run)
                    .bibliography
            } else {
                self.process_references_with_format::<F>(run).bibliography
            };
            return super::DocumentBibliography { content, entries };
        }

        let bibliography_options = self.get_bibliography_options();
        let partitioning = bibliography_options
            .sort_partitioning
            .as_ref()
            .filter(|partitioning| crate::sort_partitioning::should_render_sections(partitioning));

        self.render_flat_bibliography::<F>(
            restrict_to_cited,
            partitioning,
            FlatBibliographyOutput::ContentAndEntries,
            annotations,
            annotation_style,
            run,
        )
    }

    /// Collect the eligible references — all loaded, or only the cited/`nocite`
    /// subset when `restrict_to_cited` is `true` — in bibliography sort order.
    fn sorted_eligible_refs(&self, restrict_to_cited: bool, run: &FinalizedRun) -> Vec<&Reference> {
        let mut refs: Vec<&Reference> = self.bibliography.values().collect();
        if restrict_to_cited {
            let cited = &run.state().cited_ids;
            refs.retain(|reference| {
                reference
                    .id()
                    .as_deref()
                    .is_some_and(|id| cited.contains(id))
            });
        }
        self.sort_references(refs)
    }

    /// Render a flat or automatically partitioned bibliography in one pass.
    ///
    /// Shared by document and grouped bibliography surfaces once the caller
    /// has established there are no custom or compound-numeric groups active.
    /// When `restrict_to_cited` is `true`, only cited and `nocite` references
    /// are eligible; otherwise every loaded reference is eligible. Renders each
    /// eligible reference's template exactly once (`render_numbered_refs` — the
    /// expensive step, resolving names/dates/titles through the full template)
    /// and reuses that render for both outputs:
    ///
    /// - `entries`: one continuous subsequent-author-substitution pass over the
    ///   flat, globally sorted eligible set — matches the historical
    ///   `process_selected_references_with_format` contract, including its
    ///   ordering.
    /// - `content`: without partitioning, the same flat pass rendered to a
    ///   string. With `partitioning` requesting visible sections, an
    ///   independent substitution pass runs *per section* — substitution
    ///   state must reset at each section boundary to match historical
    ///   [`render_with_partition_sections`](Self::render_with_partition_sections)
    ///   output — reusing the one template render already produced above.
    ///   Only this lightweight linear post-pass reruns per section; the
    ///   expensive template render does not.
    ///
    /// Entry numbering (`number_sorted_refs`) is resolved once, in flat sorted
    /// order, for both outputs. Numeric bibliography styles pre-assign
    /// citation numbers document-wide during `begin_run` and look them up from
    /// that shared map regardless of section membership, so this is a no-op
    /// difference for them. Only the position-based fallback used by
    /// non-numeric styles could differ from a per-section index — but
    /// non-numeric styles do not render a citation-number variable, so that
    /// fallback value is never observable in output.
    fn render_flat_bibliography<F>(
        &self,
        restrict_to_cited: bool,
        partitioning: Option<&BibliographySortPartitioning>,
        output: FlatBibliographyOutput,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> super::DocumentBibliography
    where
        F: OutputFormat<Output = String>,
    {
        let sorted_refs = self.sorted_eligible_refs(restrict_to_cited, run);

        if output == FlatBibliographyOutput::ContentOnly
            && let Some(partitioning) = partitioning
        {
            let content = self.render_with_partition_sections::<F>(
                sorted_refs,
                partitioning,
                annotations,
                annotation_style,
                run,
            );
            return super::DocumentBibliography {
                content,
                entries: Vec::new(),
            };
        }

        let ctx = self.flat_render_context(run);
        let numbered_refs = super::number_sorted_refs(sorted_refs.iter().copied(), run);
        let rendered = self.render_numbered_refs::<F>(&numbered_refs, &ctx);

        let substitute = ctx
            .bibliography_config
            .subsequent_author_substitute
            .as_ref();

        if let Some(partitioning) = partitioning {
            // Only ContentAndEntries reaches this branch: ContentOnly with
            // partitioning returned through `render_with_partition_sections`
            // above, so `entries` is always wanted here.
            let entries =
                self.apply_substitution_post_pass::<F>(rendered.clone(), substitute, &ctx);
            let mut result = String::new();
            for (partition_key, refs_in_section) in crate::sort_partitioning::partition_references(
                sorted_refs,
                &self.locale,
                partitioning,
            ) {
                let heading = partition_key
                    .as_ref()
                    .and_then(|key| partitioning.headings.get(key));
                let section_ids: HashSet<String> = refs_in_section
                    .iter()
                    .filter_map(|reference| reference.id().map(|id| id.to_string()))
                    .collect();
                let section_rendered = rendered
                    .iter()
                    .filter(|(reference, _, _)| {
                        reference
                            .id()
                            .as_deref()
                            .is_some_and(|id| section_ids.contains(id))
                    })
                    .cloned()
                    .collect();
                let section_entries =
                    self.apply_substitution_post_pass::<F>(section_rendered, substitute, &ctx);
                self.append_rendered_partition::<F>(
                    &mut result,
                    heading,
                    section_entries,
                    annotations,
                    annotation_style,
                );
            }
            return super::DocumentBibliography {
                content: F::default().finish(result),
                entries,
            };
        }

        let rendered_entries = self.apply_substitution_post_pass::<F>(rendered, substitute, &ctx);
        let content = if rendered_entries.is_empty() {
            String::new()
        } else {
            crate::render::refs_to_string_slice_with_format::<F>(
                &rendered_entries,
                annotations,
                annotation_style,
            )
        };
        let entries = match output {
            FlatBibliographyOutput::ContentAndEntries => rendered_entries,
            FlatBibliographyOutput::ContentOnly => Vec::new(),
        };

        super::DocumentBibliography { content, entries }
    }

    /// Shared implementation for grouped bibliography rendering.
    ///
    /// When `restrict_to_cited` is `true`, each branch limits its candidate
    /// set to references present in `run`'s `cited_ids`. When `false`, all
    /// loaded references are eligible (the all-references behaviour used by
    /// the standalone grouped API, FFI, and tests).
    fn render_grouped_bibliography_inner<F>(
        &self,
        restrict_to_cited: bool,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> String
    where
        F: OutputFormat<Output = String>,
    {
        if let Some(groups) = self.effective_custom_groups() {
            let id_stubs = self.sorted_id_stubs();
            let selected = if restrict_to_cited {
                let cited = &run.state().cited_ids;
                id_stubs
                    .iter()
                    .filter(|e| cited.contains(&e.id))
                    .map(|e| e.id.clone())
                    .collect::<HashSet<_>>()
            } else {
                id_stubs
                    .iter()
                    .map(|e| e.id.clone())
                    .collect::<HashSet<_>>()
            };
            return self.render_with_custom_groups_filtered::<F>(
                &id_stubs,
                groups,
                &selected,
                annotations,
                annotation_style,
                run,
            );
        }

        let bibliography_options = self.get_bibliography_options();
        let partitioning = bibliography_options
            .sort_partitioning
            .as_ref()
            .filter(|partitioning| crate::sort_partitioning::should_render_sections(partitioning));

        if !run.state().compound_groups.is_empty() {
            if let Some(partitioning) = partitioning {
                let sorted_refs = self.sorted_eligible_refs(restrict_to_cited, run);
                return self.render_with_partition_sections::<F>(
                    sorted_refs,
                    partitioning,
                    annotations,
                    annotation_style,
                    run,
                );
            }

            let all_entries = self.process_references_with_format::<F>(run).bibliography;
            let merged = self.merge_compound_entries::<F>(all_entries, run);
            return self.render_flat_compound_entries::<F>(
                &merged,
                restrict_to_cited,
                annotations,
                annotation_style,
                run,
            );
        }

        self.render_flat_bibliography::<F>(
            restrict_to_cited,
            partitioning,
            FlatBibliographyOutput::ContentOnly,
            annotations,
            annotation_style,
            run,
        )
        .content
    }

    /// Extract and render entries for a bibliography group.
    ///
    /// Returns the individual processed entries for the group, threading
    /// the `assigned` dedup set to ensure each reference appears in only one
    /// group. `spine` is the document-wide sorted ID stub list (see
    /// [`sorted_id_stubs`](Self::sorted_id_stubs)); callers that render
    /// multiple groups in one pass (e.g.
    /// [`render_document_bibliography_blocks`](Self::render_document_bibliography_blocks))
    /// compute it once and pass the same slice to every group instead of
    /// re-sorting the whole bibliography per group.
    fn entries_for_bibliography_group<F>(
        &self,
        spine: &[ProcEntry],
        group: &BibliographyGroup,
        assigned: &mut HashSet<String>,
        run: &FinalizedRun,
    ) -> Vec<crate::render::ProcEntry>
    where
        F: OutputFormat<Output = String>,
    {
        let cited_ids = &run.state().cited_ids;
        let evaluator = SelectorEvaluator::new(cited_ids);
        let bibliography_config = self.get_bibliography_config();
        let mut sorter =
            ReferenceSorter::with_bibliography_config(&self.locale, &bibliography_config);
        if let Some(spec) = self.style.bibliography.as_ref() {
            sorter = sorter.with_bibliography_spec(spec);
        }

        let matching_refs = self.collect_matching_group_refs(spine, assigned, &evaluator, group);
        Self::mark_group_members_assigned(assigned, &matching_refs);

        if matching_refs.is_empty() {
            return Vec::new();
        }

        let sorted_refs = if let Some(sort_spec) = &group.sort {
            sorter.sort_references(matching_refs, &sort_spec.resolve())
        } else {
            matching_refs
        };

        let local_hints = self.build_group_local_hints(&sorted_refs, group);
        self.merge_compound_entries::<F>(
            self.render_group_entries::<F>(spine, sorted_refs, group, local_hints.as_ref(), run),
            run,
        )
    }

    /// Render one bibliography block for document output.
    ///
    /// Returns heading and body separately so callers can insert headings
    /// in their own output format.
    ///
    /// `spine` is the document-wide sorted ID spine (see
    /// [`sorted_id_stubs`](Self::sorted_id_stubs)). Standalone callers pass
    /// their own `self.sorted_id_stubs()`;
    /// [`render_document_bibliography_blocks`](Self::render_document_bibliography_blocks)
    /// computes it once and shares the same slice across every block,
    /// avoiding a full bibliography re-sort per block.
    #[allow(
        clippy::too_many_arguments,
        reason = "internal helper, all params load-bearing"
    )]
    pub(crate) fn render_document_bibliography_block<F>(
        &self,
        spine: &[ProcEntry],
        group: &BibliographyGroup,
        assigned: &mut HashSet<String>,
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> RenderedBibliographyGroup
    where
        F: OutputFormat<Output = String>,
    {
        let mut headingless = group.clone();
        let heading = headingless
            .heading
            .take()
            .and_then(|group_heading| self.resolve_group_heading(&group_heading));

        let entries = self.entries_for_bibliography_group::<F>(spine, &headingless, assigned, run);
        let body = crate::render::refs_to_string_slice_with_format::<F>(
            &entries,
            annotations,
            annotation_style,
        );

        RenderedBibliographyGroup {
            heading,
            body,
            entries,
        }
    }

    /// Render an ordered sequence of sectional bibliography blocks.
    ///
    /// Threads a single `assigned` dedup set so each reference appears in
    /// only one block. Returns rendered groups with heading, body, and
    /// entries. Computes the sorted ID spine once for the whole call (see
    /// [`sorted_id_stubs`](Self::sorted_id_stubs)) and shares it across every
    /// block, instead of each block independently re-sorting the full
    /// bibliography.
    pub(crate) fn render_document_bibliography_blocks<F>(
        &self,
        groups: &[BibliographyGroup],
        annotations: Option<&HashMap<String, String>>,
        annotation_style: Option<&AnnotationStyle>,
        run: &FinalizedRun,
    ) -> Vec<RenderedBibliographyGroup>
    where
        F: OutputFormat<Output = String>,
    {
        if self.style.bibliography.is_none() {
            return Vec::new();
        }
        let spine = self.sorted_id_stubs();
        let mut assigned = std::collections::HashSet::new();
        groups
            .iter()
            .map(|group| {
                self.render_document_bibliography_block::<F>(
                    &spine,
                    group,
                    &mut assigned,
                    annotations,
                    annotation_style,
                    run,
                )
            })
            .collect()
    }

    pub(super) fn extract_metadata(
        &self,
        reference: &Reference,
        ctx: &EntryRenderContext<'_>,
    ) -> ProcEntryMetadata {
        let bibliography_config = &ctx.config;
        let options = RenderOptions {
            config: bibliography_config.clone(),
            bibliography_config: Some(ctx.bibliography_config.clone()),
            locale: &self.locale,
            context: RenderContext::Bibliography,
            mode: citum_schema::citation::CitationMode::NonIntegral,
            suppress_author: false,
            locator_raw: None,
            ref_type: None,
            show_semantics: self.show_semantics,
            current_template_index: None,
            abbreviation_map: self.abbreviation_map.as_ref(),
        };

        let ml = bibliography_config.multilingual.as_ref();
        let preferred_transliteration = ml.and_then(|m| m.preferred_transliteration.as_deref());
        let preferred_script = ml.and_then(|m| m.preferred_script.as_ref());

        ProcEntryMetadata {
            author: reference.author().map(|author| {
                let names = resolve_multilingual_name(
                    &author,
                    ml.and_then(|m| m.name_mode.as_ref()),
                    preferred_transliteration,
                    preferred_script,
                    &self.locale.locale,
                );
                format_contributors_short(&names, &options)
            }),
            year: reference
                .effective_issued_date()
                .map(|issued| issued.year().clone()),
            title: reference.title().map(|title| {
                use citum_schema::reference::types::{MultilingualString, Title};
                match &title {
                    Title::Multilingual(m) => resolve_multilingual_string(
                        &MultilingualString::Complex(m.clone()),
                        ml.and_then(|ml| ml.title_mode.as_ref()),
                        preferred_transliteration,
                        preferred_script,
                        &self.locale.locale,
                    ),
                    _ => title.to_string(),
                }
            }),
        }
    }

    fn render_group_heading<F>(&self, heading: &str) -> String
    where
        F: OutputFormat<Output = String>,
    {
        let fmt = F::default();
        fmt.finish(fmt.unnumbered_heading(2, fmt.text(heading)))
    }
}
