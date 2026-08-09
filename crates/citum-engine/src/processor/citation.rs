/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Citation rendering orchestration.
//!
//! This module resolves the effective citation spec for each citation, prepares
//! renderer delimiters and affixes. Template-level rendering, including
//! sentence-initial note-start handling, lives in `rendering`.
//!
//! Registration (`&mut RunState`) and rendering (`&RunState`) are interleaved
//! per citation, in document order: each citation's disambiguation,
//! position, and dynamic grouping depend on the cumulative state left by
//! every citation processed before it, and citation numbers may be assigned
//! lazily during one citation's render that a later citation's registration
//! then reads. This is why citation processing takes `&mut RunState`
//! end-to-end rather than the `&FinalizedRun` used by bibliography
//! rendering, which requires the complete, final state from all citations.
//! See `docs/specs/EXPLICIT_RENDER_RUN_STATE.md`.

use super::Processor;
use super::disambiguation::Disambiguator;
use super::rendering::{CompoundRenderData, GroupRenderParams, Renderer, RendererResources};
use super::run_state::RunState;
use crate::error::ProcessorError;
use crate::reference::Citation;
use crate::values::ProcHints;
use citum_schema::NoteStartTextCase;
use citum_schema::locale::{GeneralTerm, Locale, TermForm};
use citum_schema::options::{Config, GivennameRule};
use indexmap::IndexMap;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

/// Join rendered integral (narrative) groups with localized conjunctions.
///
/// Uses the locale's "and" term to join groups according to document grammar
/// rules (e.g., "A and B" or "A, B, and C" with optional serial comma).
fn join_integral_groups(rendered_groups: Vec<String>, locale: &Locale) -> String {
    match rendered_groups.len() {
        0 => String::new(),
        1 => rendered_groups.into_iter().next().unwrap_or_default(),
        2 => {
            let conjunction = locale
                .resolved_general_term(&GeneralTerm::And, &TermForm::Long, None)
                .unwrap_or_else(|| locale.and_term(false).to_string());
            rendered_groups.join(&format!(" {} ", conjunction.trim()))
        }
        _ => {
            let conjunction = locale
                .resolved_general_term(&GeneralTerm::And, &TermForm::Long, None)
                .unwrap_or_else(|| locale.and_term(false).to_string());
            let final_delimiter = if locale.grammar_options.serial_comma {
                format!(", {} ", conjunction.trim())
            } else {
                format!(" {} ", conjunction.trim())
            };

            let mut rendered_groups = rendered_groups;
            let last = rendered_groups.pop().unwrap_or_default();
            format!("{}{}{}", rendered_groups.join(", "), final_delimiter, last)
        }
    }
}

impl Processor {
    /// Determine the text-case policy for a citation at the start of a note.
    ///
    /// Only applies for note-based styles when a repeated-citation position (Ibid)
    /// is at the start of the note and has no user-supplied or spec-defined prefix.
    fn sentence_initial_note_start_text_case(
        &self,
        citation: &Citation,
        effective_spec: &citum_schema::CitationSpec,
    ) -> Option<NoteStartTextCase> {
        let spec_prefix = effective_spec.prefix.as_deref().unwrap_or("");
        if self.is_note_style()
            && matches!(
                citation.position,
                Some(
                    citum_schema::citation::Position::Ibid
                        | citum_schema::citation::Position::IbidWithLocator
                )
            )
            && matches!(
                citation.mode,
                citum_schema::citation::CitationMode::NonIntegral
            )
            && citation.prefix.as_deref().unwrap_or("").is_empty()
            && spec_prefix.is_empty()
        {
            effective_spec.note_start_text_case
        } else {
            None
        }
    }

    /// Resolve the citation specification based on the citation's document position.
    ///
    /// Delegates to the style's citation spec to handle ibid, subsequent, or first
    /// position overrides.
    fn resolve_positioned_citation_spec(
        &self,
        citation: &Citation,
    ) -> std::borrow::Cow<'_, citum_schema::CitationSpec> {
        self.style.citation.as_ref().map_or_else(
            || std::borrow::Cow::Owned(citum_schema::CitationSpec::default()),
            |spec| spec.resolve_for_position(citation.position.as_ref()),
        )
    }

    /// Register nocite reference IDs into the cited set.
    ///
    /// Nocite IDs are treated as cited for bibliography-selection purposes (they
    /// appear in `bibliography.entries` alongside normally cited refs and are
    /// matched by `CitedStatus::Visible` selectors), but no `formatted_citations`
    /// entry is produced for them. This matches standard citeproc / Pandoc `nocite`
    /// semantics.
    ///
    /// IDs that are absent from `self.bibliography` are silently ignored here;
    /// callers are responsible for emitting `nocite_missing_ref` warnings first.
    pub fn register_nocite_ids(&self, ids: impl IntoIterator<Item = String>, run: &mut RunState) {
        for id in ids {
            run.cited_ids.insert(id);
        }
    }

    /// Register cited reference IDs and ensure numeric labels are initialized.
    ///
    /// This maintains the set of all references cited in the document and ensures
    /// that numeric styles have a stable numbering map.
    fn track_cited_ids_and_init_numbers(&self, citation: &Citation, run: &mut RunState) {
        self.initialize_numeric_citation_numbers(run);
        for item in &citation.items {
            run.cited_ids.insert(item.id.clone());
        }
    }

    /// Resolve the final effective citation spec for a given mode and position.
    fn resolve_effective_citation_spec(&self, citation: &Citation) -> citum_schema::CitationSpec {
        self.resolve_positioned_citation_spec(citation)
            .into_owned()
            .resolve_for_mode(&citation.mode)
            .into_owned()
    }

    /// Resolve intra-item and inter-citation delimiters for a citation spec.
    fn resolve_citation_delimiters<'a>(
        &'a self,
        citation: &Citation,
        effective_spec: &'a citum_schema::CitationSpec,
    ) -> (Cow<'a, str>, Cow<'a, str>) {
        let (script, realization) = self.citation_punctuation_context(citation);
        let intra_delimiter = effective_spec
            .delimiter
            .as_ref()
            .map(|punctuation| {
                Cow::Owned(
                    crate::render::format::realize_punctuation(
                        punctuation,
                        script,
                        realization.as_deref(),
                        crate::render::format::PunctuationPosition::Separator,
                    )
                    .into_owned(),
                )
            })
            .unwrap_or(Cow::Borrowed(", "));
        let inter_delimiter = effective_spec
            .multi_cite_delimiter
            .as_ref()
            .map(|punctuation| {
                Cow::Owned(
                    crate::render::format::realize_punctuation(
                        punctuation,
                        script,
                        realization.as_deref(),
                        crate::render::format::PunctuationPosition::Separator,
                    )
                    .into_owned(),
                )
            })
            .unwrap_or(Cow::Borrowed("; "));

        (intra_delimiter, inter_delimiter)
    }

    /// Register a dynamic compound group for a `grouped` citation.
    ///
    /// The first item in `citation.items` is the head; subsequent items are tails.
    /// Skips silently when:
    /// - The style has no `compound-numeric` bibliography configuration (non-numeric style).
    /// - A static compound set already covers the head or any tail (static sets take precedence).
    /// - The head or any tail was previously cited in any context (first occurrence wins).
    ///
    /// This method must be called before `track_cited_ids_and_init_numbers` so that
    /// `cited_ids` reflects only references from prior citations, not the current one.
    fn resolve_dynamic_group(&self, citation: &Citation, run: &mut RunState) {
        if self.get_bibliography_options().compound_numeric.is_none() {
            return;
        }

        if citation.items.len() < 2 {
            return;
        }

        #[allow(clippy::indexing_slicing, reason = "citation.items.len() >= 2")]
        let head_id = &citation.items[0].id;
        #[allow(clippy::indexing_slicing, reason = "citation.items.len() >= 2")]
        let tail_ids: Vec<String> = citation.items[1..].iter().map(|i| i.id.clone()).collect();

        // Static sets take precedence — skip if head or any tail is in a static set.
        if self.compound_set_by_ref.contains_key(head_id) {
            return;
        }
        for tail in &tail_ids {
            if self.compound_set_by_ref.contains_key(tail.as_str()) {
                return;
            }
        }

        // First-occurrence wins: reject if the head or any tail was already cited in any
        // context — whether via a prior dynamic group or a previous ungrouped citation.
        // Because this method is called before cited_ids is updated for the current
        // citation, `cited_ids` contains only references from earlier citations.
        if run
            .dynamic_compound_set_by_ref
            .contains_key(head_id.as_str())
            || run.cited_ids.contains(head_id.as_str())
        {
            return;
        }
        for tail in &tail_ids {
            if run.dynamic_compound_set_by_ref.contains_key(tail.as_str())
                || run.cited_ids.contains(tail.as_str())
            {
                return;
            }
        }

        let head_number = {
            let numbers = run
                .citation_numbers
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(&n) = numbers.get(head_id.as_str()) else {
                return;
            };
            n
        };

        // Assign all tails the same citation number as the head.
        {
            let mut numbers = run
                .citation_numbers
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for tail in &tail_ids {
                numbers.insert(tail.clone(), head_number);
            }
        }

        // Build the ordered member list for this group.
        let all_members: Vec<String> = std::iter::once(head_id.clone())
            .chain(tail_ids.iter().cloned())
            .collect();

        // Populate dynamic index maps so the renderer can assign sub-labels.
        for (idx, member) in all_members.iter().enumerate() {
            run.dynamic_compound_set_by_ref
                .insert(member.clone(), head_id.clone());
            run.dynamic_compound_member_index
                .insert(member.clone(), idx);
        }

        // Inject into compound_groups for bibliography rendering.
        {
            let members = run
                .compound_groups
                .entry(head_number)
                .or_insert_with(|| vec![head_id.clone()]);
            for tail in &tail_ids {
                if !members.contains(tail) {
                    members.push(tail.clone());
                }
            }
        }

        // Register dynamic set so citation_sub_label_for_ref can find members.
        run.dynamic_compound_sets
            .insert(head_id.clone(), all_members);
    }

    /// Build a citation-local hint overlay for CSL `givenname-disambiguation-rule: by-cite`.
    ///
    /// Global hints remain authoritative for bibliography rendering, year-suffix ordering,
    /// numeric state, and note-position state. This overlay only recalculates name expansion
    /// fields for the references rendered by the current citation.
    fn citation_scoped_by_cite_hints(
        &self,
        items: &[crate::reference::CitationItem],
        config: &Config,
    ) -> Option<HashMap<String, ProcHints>> {
        if !Self::uses_by_cite_givenname(config) {
            return None;
        }

        let mut scoped_hints = HashMap::new();
        let mut scoped_bibliography = IndexMap::new();

        for item in items {
            let mut hint = self.hints.get(&item.id).cloned().unwrap_or_default();
            hint.expand_given_names = false;
            hint.expand_given_names_primary_only = false;
            hint.min_names_to_show = None;
            scoped_hints.insert(item.id.clone(), hint);

            if let Some(reference) = self.bibliography.get(&item.id) {
                scoped_bibliography.insert(item.id.clone(), reference.clone());
            }
        }

        if scoped_bibliography.len() < 2 {
            return Some(scoped_hints);
        }

        let bibliography_config = self.get_bibliography_config();
        let mut disambiguator = Disambiguator::new(
            &scoped_bibliography,
            config,
            &bibliography_config,
            &self.locale,
        );
        if let Some(spec) = self.style.citation.as_ref() {
            disambiguator = disambiguator.with_citation_spec(spec);
        }
        let local_hints = disambiguator.calculate_hints();

        for item in items {
            let Some(local) = local_hints.get(&item.id) else {
                continue;
            };
            let target = scoped_hints.entry(item.id.clone()).or_default();
            target.expand_given_names = local.expand_given_names;
            target.expand_given_names_primary_only = local.expand_given_names_primary_only;
            target.min_names_to_show = local.min_names_to_show;
        }

        Some(scoped_hints)
    }

    /// Return true when the active citation config requests CSL by-cite given-name expansion.
    fn uses_by_cite_givenname(config: &Config) -> bool {
        let disambiguate = config.effective_processing().config().disambiguate;

        disambiguate
            .as_ref()
            .is_some_and(|d| d.add_givenname && matches!(d.givenname_rule, GivennameRule::ByCite))
    }

    /// Build the merged static + dynamic compound lookup maps for the renderer.
    ///
    /// When no dynamic groups exist (the common case) the static maps are returned
    /// via references with no allocation. Owned merged maps are only constructed when
    /// at least one dynamic group is registered.
    fn merged_compound_data(
        &self,
        run: &RunState,
    ) -> (
        Option<HashMap<String, String>>,
        Option<HashMap<String, usize>>,
        Option<IndexMap<String, Vec<String>>>,
    ) {
        if run.dynamic_compound_set_by_ref.is_empty() {
            return (None, None, None);
        }
        let merged_set: HashMap<String, String> = self
            .compound_set_by_ref
            .iter()
            .chain(run.dynamic_compound_set_by_ref.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let merged_idx: HashMap<String, usize> = self
            .compound_member_index
            .iter()
            .chain(run.dynamic_compound_member_index.iter())
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        let merged_sets: IndexMap<String, Vec<String>> = self
            .compound_sets
            .iter()
            .chain(run.dynamic_compound_sets.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        (Some(merged_set), Some(merged_idx), Some(merged_sets))
    }

    /// Render the core content of a citation, handling sorting and grouping.
    ///
    /// This is the main orchestration point for template rendering, compound data
    /// resolution, and mode-specific (integral vs non-integral) formatting.
    fn render_citation_content<F>(
        &self,
        citation: &Citation,
        effective_spec: &citum_schema::CitationSpec,
        renderer_delimiter: &str,
        renderer_inter_delimiter: &str,
        note_start_text_case: Option<NoteStartTextCase>,
        run: &RunState,
    ) -> Result<(String, bool), ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        // Grouped citations preserve item order (dynamic grouping was already resolved
        // in process_citation_with_format before cited_ids was updated).
        let sorted_items = if citation.grouped {
            citation.items.clone()
        } else {
            self.sort_citation_items(citation.items.clone(), effective_spec)
        };

        // Build merged compound lookup maps (static + dynamic).
        // Return owned maps only when dynamic groups exist; otherwise use static maps directly.
        let (dyn_set_owned, dyn_idx_owned, dyn_sets_owned) = self.merged_compound_data(run);
        let effective_set_by_ref = dyn_set_owned.as_ref().unwrap_or(&self.compound_set_by_ref);
        let effective_member_index = dyn_idx_owned
            .as_ref()
            .unwrap_or(&self.compound_member_index);
        let effective_compound_sets = dyn_sets_owned.as_ref().unwrap_or(&self.compound_sets);

        let citation_config = self.get_citation_config();
        let citation_config = match effective_spec.options.as_ref() {
            Some(mode_options) => {
                let mut config = citation_config.into_owned();
                config.merge(&mode_options.to_config());
                std::borrow::Cow::Owned(config)
            }
            None => citation_config,
        };
        let scoped_hints = self.citation_scoped_by_cite_hints(&sorted_items, &citation_config);
        let renderer_hints = scoped_hints.as_ref().unwrap_or(&self.hints);
        let punctuation_in_quote = citation_config.punctuation_in_quote;
        let citation_config = Arc::new(citation_config.into_owned());
        let renderer = Renderer::new(
            RendererResources {
                style: &self.style,
                bibliography: &self.bibliography,
                locale: &self.locale,
                config: citation_config.clone(),
                bibliography_config: Some(Arc::new(self.get_bibliography_options().into_owned())),
                first_note_by_id: Some(&run.first_note_by_id),
            },
            renderer_hints,
            &run.citation_numbers,
            CompoundRenderData {
                set_by_ref: effective_set_by_ref,
                member_index: effective_member_index,
                sets: effective_compound_sets,
            },
            self.show_semantics,
            self.inject_ast_indices,
            self.abbreviation_map.as_ref(),
        );
        let processing = citation_config.processing.clone().unwrap_or_default();
        let has_explicit_integral_multi_cite_delimiter = matches!(
            citation.mode,
            citum_schema::citation::CitationMode::Integral
        ) && self
            .resolve_positioned_citation_spec(citation)
            .integral
            .as_ref()
            .and_then(|spec| spec.multi_cite_delimiter.as_ref())
            .is_some();
        let rendered_groups = if matches!(
            processing,
            citum_schema::options::Processing::Numeric
                | citum_schema::options::Processing::Label(_)
        ) {
            renderer.render_ungrouped_citation_with_format::<F>(
                &sorted_items,
                effective_spec,
                &citation.mode,
                renderer_delimiter,
                citation.suppress_author,
                citation.position.as_ref(),
                note_start_text_case,
            )?
        } else {
            renderer.render_grouped_citation_with_format::<F>(
                &sorted_items,
                &GroupRenderParams {
                    spec: effective_spec,
                    mode: &citation.mode,
                    intra_delimiter: renderer_delimiter,
                    suppress_author: citation.suppress_author,
                    position: citation.position.as_ref(),
                    note_start_text_case,
                },
            )?
        };

        let rendered = if matches!(
            citation.mode,
            citum_schema::citation::CitationMode::Integral
        ) && !has_explicit_integral_multi_cite_delimiter
        {
            join_integral_groups(rendered_groups, &self.locale)
        } else {
            F::default().join(rendered_groups, renderer_inter_delimiter)
        };
        Ok((rendered, punctuation_in_quote))
    }

    /// Apply user-supplied prefix and suffix from the citation input.
    ///
    /// Automatically adds a trailing space to the prefix and a leading space to
    /// the suffix if they are not already present and not empty.
    fn apply_citation_input_affixes<F>(
        &self,
        citation: &Citation,
        content: String,
        fmt: &F,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let citation_prefix = citation.prefix.as_deref().unwrap_or("");
        let citation_suffix = citation.suffix.as_deref().unwrap_or("");

        if citation_prefix.is_empty() && citation_suffix.is_empty() {
            return content;
        }

        let formatted_prefix =
            if !citation_prefix.is_empty() && !citation_prefix.ends_with(char::is_whitespace) {
                format!("{citation_prefix} ")
            } else {
                citation_prefix.to_string()
            };

        let formatted_suffix =
            if !citation_suffix.is_empty() && !citation_suffix.starts_with(char::is_whitespace) {
                format!(" {citation_suffix}")
            } else {
                citation_suffix.to_string()
            };

        fmt.affix(&formatted_prefix, content, &formatted_suffix)
    }

    /// Apply style-defined wrapping and affixes to the rendered citation output.
    ///
    /// Handles `wrap` logic (inner prefixes/suffixes and punctuation) based on
    /// the citation mode and position.
    fn apply_spec_wrap_and_affixes<F>(
        &self,
        citation: &Citation,
        effective_spec: &citum_schema::CitationSpec,
        output: String,
        punctuation_in_quote: bool,
        fmt: &F,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let (script, realization) = self.citation_punctuation_context(citation);
        let spec_prefix = effective_spec
            .prefix
            .as_ref()
            .map(|punctuation| {
                crate::render::format::realize_punctuation(
                    punctuation,
                    script,
                    realization.as_deref(),
                    crate::render::format::PunctuationPosition::Prefix,
                )
            })
            .unwrap_or(Cow::Borrowed(""));
        let spec_suffix = effective_spec
            .suffix
            .as_ref()
            .map(|punctuation| {
                crate::render::format::realize_punctuation(
                    punctuation,
                    script,
                    realization.as_deref(),
                    crate::render::format::PunctuationPosition::Suffix,
                )
            })
            .unwrap_or(Cow::Borrowed(""));
        let spec_suffix = crate::render::format::RealizedPunctuation::new(spec_suffix);
        let is_integral = matches!(
            citation.mode,
            citum_schema::citation::CitationMode::Integral
        );
        let suffix_applies = is_integral || effective_spec.wrap.is_none();
        let mut output = output;
        let suffix_core = spec_suffix.core();
        let suffix_moved_inside_quote = if suffix_applies
            && punctuation_in_quote
            && let Some(mark @ ('.' | ',')) = suffix_core
        {
            let quote_marks = crate::render::format::QuoteMarks::from(&self.locale.grammar_options);
            crate::render::punctuation::move_punctuation_into_quote::<F>(
                &mut output,
                mark,
                &quote_marks.close,
            )
        } else {
            false
        };
        let spec_suffix_text = if suffix_moved_inside_quote {
            spec_suffix.tail()
        } else {
            spec_suffix.text()
        };

        if is_integral {
            if !spec_prefix.is_empty() || !spec_suffix.is_empty() {
                crate::render::format::apply_punctuation_affixes(
                    fmt,
                    effective_spec
                        .prefix
                        .as_ref()
                        .map(|punctuation| (punctuation, spec_prefix.as_ref())),
                    output,
                    effective_spec
                        .suffix
                        .as_ref()
                        .map(|punctuation| (punctuation, spec_suffix_text)),
                )
            } else {
                output
            }
        } else if let Some(wrap) = effective_spec.wrap.as_ref() {
            let inner_prefix = wrap.inner_prefix.as_deref().unwrap_or("");
            let inner_suffix = wrap.inner_suffix.as_deref().unwrap_or("");
            let inner_wrapped = if !inner_prefix.is_empty() || !inner_suffix.is_empty() {
                fmt.inner_affix(inner_prefix, output, inner_suffix)
            } else {
                output
            };
            let marks = crate::render::format::QuoteMarks::from(&self.locale.grammar_options);
            fmt.wrap_punctuation(
                &wrap.punctuation,
                inner_wrapped,
                &marks,
                script,
                realization.as_deref(),
            )
        } else if !spec_prefix.is_empty() || !spec_suffix.is_empty() {
            crate::render::format::apply_punctuation_affixes(
                fmt,
                effective_spec
                    .prefix
                    .as_ref()
                    .map(|punctuation| (punctuation, spec_prefix.as_ref())),
                output,
                effective_spec
                    .suffix
                    .as_ref()
                    .map(|punctuation| (punctuation, spec_suffix_text)),
            )
        } else {
            output
        }
    }

    /// Whether `options.multilingual.scripts.latin.punctuation: latin` applies to a
    /// citation, based on its first item's effective language.
    ///
    /// The citation-spec-level `prefix`/`suffix`/`wrap` applied by
    /// [`Self::apply_spec_wrap_and_affixes`] sits outside all per-component and
    /// per-item rendering (which already remap their own full-width delimiters —
    /// see `render::component` and [`super::rendering::Renderer::affix_content`]),
    /// so a literal full-width wrap like GB/T author-date's `prefix: （ suffix: ）`
    /// needs the same remap applied to the fully-assembled citation. All items in
    /// one citation typically share a language; the first item's stands in for
    /// mixed compound citations as a reasonable approximation.
    fn wants_latin_punctuation_for_citation(&self, citation: &Citation) -> bool {
        let configured = self.get_config().multilingual.as_ref().is_some_and(|ml| {
            ml.scripts.get("latin").is_some_and(|script| {
                script.punctuation == Some(citum_schema::options::PunctuationStyle::Latin)
            })
        });

        configured
            && citation.items.first().is_some_and(|item| {
                self.bibliography.get(&item.id).is_some_and(|reference| {
                    crate::values::is_latin_script_language(
                        crate::values::effective_item_language(reference).as_deref(),
                    )
                })
            })
    }

    /// Resolve script selection and per-script realization overrides for a
    /// citation using its first item as the mixed-cluster approximation.
    fn citation_punctuation_context(
        &self,
        citation: &Citation,
    ) -> (
        crate::values::ScriptClass,
        Option<Cow<'_, citum_schema::options::PunctuationRealization>>,
    ) {
        let lang = citation.items.first().and_then(|item| {
            self.bibliography
                .get(&item.id)
                .and_then(crate::values::effective_item_language)
        });
        crate::values::punctuation_realization_context(
            lang.as_deref(),
            self.get_config().multilingual.as_ref(),
            self.locale.punctuation_realization.as_ref(),
        )
    }

    /// Render a single citation to plain text.
    ///
    /// This is a one-shot convenience wrapper: it begins a throwaway
    /// [`RunState`] internally, so it has no continuity with any other call.
    /// Use [`Processor::process_citations`] (or the run-threaded
    /// `_with_format` variants with an explicit, shared `RunState`) to render
    /// multiple citations from one document with correct cumulative
    /// numbering, cite-order tracking, and dynamic compound grouping.
    ///
    /// # Errors
    ///
    /// Returns an error when referenced items are missing or rendering fails.
    pub fn process_citation(&self, citation: &Citation) -> Result<String, ProcessorError> {
        let mut run = self.begin_run();
        self.process_citation_with_format::<crate::render::plain::PlainText>(citation, &mut run)
    }

    /// Render a citation to a string using a specific output format.
    ///
    /// This resolves the effective citation spec for the citation's mode and
    /// position, renders the citation body, and applies input and style
    /// affixes. `run` accumulates cite-order state (citation numbers, cited
    /// IDs, dynamic compound groups) across calls; pass the same `RunState`
    /// for every citation in one document to get correct cumulative
    /// behavior, or a fresh one (via [`Processor::begin_run`]) for an
    /// isolated, one-off render.
    ///
    /// # Errors
    ///
    /// Returns an error when referenced items are missing or rendering fails.
    pub fn process_citation_with_format<F>(
        &self,
        citation: &Citation,
        run: &mut RunState,
    ) -> Result<String, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();

        // For grouped citations, resolve the dynamic compound group BEFORE updating
        // cited_ids with the current citation's items. This ensures the first-occurrence
        // check in resolve_dynamic_group sees only references from prior citations.
        if citation.grouped {
            self.initialize_numeric_citation_numbers(run);
            self.resolve_dynamic_group(citation, run);
        }

        self.track_cited_ids_and_init_numbers(citation, run);

        let effective_spec = self.resolve_effective_citation_spec(citation);
        let note_start_text_case =
            self.sentence_initial_note_start_text_case(citation, &effective_spec);
        let (renderer_delimiter, renderer_inter_delimiter) =
            self.resolve_citation_delimiters(citation, &effective_spec);
        let renderer_delimiter = if effective_spec
            .delimiter
            .as_ref()
            .is_some_and(citum_schema::template::DelimiterPunctuation::is_semantic)
        {
            fmt.text(&renderer_delimiter)
        } else {
            renderer_delimiter.into_owned()
        };
        let renderer_inter_delimiter = if effective_spec
            .multi_cite_delimiter
            .as_ref()
            .is_some_and(citum_schema::template::DelimiterPunctuation::is_semantic)
        {
            fmt.text(&renderer_inter_delimiter)
        } else {
            renderer_inter_delimiter.into_owned()
        };
        let (content, punctuation_in_quote) = self.render_citation_content::<F>(
            citation,
            &effective_spec,
            &renderer_delimiter,
            &renderer_inter_delimiter,
            note_start_text_case,
            run,
        )?;
        let output = self.apply_citation_input_affixes(citation, content, &fmt);
        let wrapped = self.apply_spec_wrap_and_affixes(
            citation,
            &effective_spec,
            output,
            punctuation_in_quote,
            &fmt,
        );
        let wrapped = if self.wants_latin_punctuation_for_citation(citation) {
            crate::render::component::remap_to_latin_punctuation(wrapped)
        } else {
            wrapped
        };

        // If the host signals that this cluster opens a sentence, capitalize
        // the leading character of the composed output.  The markup-aware
        // variant skips leading punctuation (e.g. an opening parenthesis) so
        // only the first alphabetic character is affected.
        let finalized = if citation.sentence_start {
            let case = crate::values::text_case::resolve_text_case(
                citum_schema::options::titles::TextCase::CapitalizeFirst,
                Some(self.locale.locale.as_str()),
            );
            crate::values::text_case::apply_text_case_markup_aware_with_language(
                &wrapped,
                case,
                Some(self.locale.locale.as_str()),
            )
        } else {
            wrapped
        };

        Ok(fmt.finish(finalized))
    }

    /// Render multiple citations in document order.
    ///
    /// For note-based styles, normalizes context and assigns citation
    /// positions. This is a one-shot convenience wrapper: it begins a
    /// throwaway [`RunState`] internally, shared across all citations in
    /// `citations` (so cumulative numbering/grouping within this call is
    /// correct) but not shared with any other call.
    ///
    /// # Errors
    ///
    /// Returns an error when any citation in the sequence fails to render.
    pub fn process_citations(&self, citations: &[Citation]) -> Result<Vec<String>, ProcessorError> {
        let mut run = self.begin_run();
        self.process_citations_with_format::<crate::render::plain::PlainText>(citations, &mut run)
    }

    /// Render multiple citations with a custom output format.
    ///
    /// `run` is threaded through every citation in `citations`, in order, so
    /// numbering/cite-order/dynamic-grouping state accumulates correctly
    /// across the whole batch. Pass the same `run` on to bibliography
    /// rendering (after [`RunState::finalize`]) to render a consistent
    /// document.
    ///
    /// # Errors
    ///
    /// Returns an error when any citation in the sequence fails to render.
    pub fn process_citations_with_format<F>(
        &self,
        citations: &[Citation],
        run: &mut RunState,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let mut normalized = self.normalize_note_context(citations, run);
        self.annotate_positions(&mut normalized);
        normalized
            .iter()
            .map(|citation| self.process_citation_with_format::<F>(citation, run))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::plain::PlainText;
    use citum_schema::Style;
    use citum_schema::template::DelimiterPunctuation;

    fn apply_period_suffix(punctuation_in_quote: bool) -> String {
        let style = Style {
            options: Some(Config {
                punctuation_in_quote,
                ..Default::default()
            }),
            ..Default::default()
        };
        let processor = Processor::new(style, Default::default());
        let spec = citum_schema::CitationSpec {
            suffix: Some(DelimiterPunctuation::Custom(".".to_string())),
            ..Default::default()
        };

        processor.apply_spec_wrap_and_affixes(
            &Citation::default(),
            &spec,
            "“Title”".to_string(),
            punctuation_in_quote,
            &PlainText,
        )
    }

    #[test]
    fn citation_spec_suffix_respects_punctuation_in_quote() {
        assert_eq!(apply_period_suffix(true), "“Title.”");
        assert_eq!(apply_period_suffix(false), "“Title”.");
    }
}
