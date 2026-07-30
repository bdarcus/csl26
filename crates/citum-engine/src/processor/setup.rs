/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Processor construction and configuration helpers.
//!
//! This module owns setup-time concerns for [`Processor`]: constructor paths,
//! locale/config resolution, compound-set validation, and numeric citation
//! number initialization. It intentionally does not contain citation or
//! bibliography rendering logic.

use super::Processor;
use super::disambiguation::Disambiguator;
use super::run_state::RunState;
use crate::error::ProcessorError;
use crate::reference::{Bibliography, CitationItem, Reference};
use crate::values::ProcHints;
use citum_schema::Style;
use citum_schema::locale::Locale;
use citum_schema::options::{
    BibliographyPartitionKind, BibliographyPartitionMode, BibliographySortPartitioning, Config,
    PunctuationConfig, SortingMultilingualMode, bibliography::BibliographyConfig,
};
use indexmap::IndexMap;
use std::collections::HashMap;

impl Default for Processor {
    fn default() -> Self {
        let compound_sets = IndexMap::new();
        let (compound_set_by_ref, compound_member_index) =
            Self::build_compound_set_indexes(&compound_sets);
        Self {
            style: Style::default(),
            bibliography: Bibliography::default(),
            locale: Locale::en_us(),
            default_config: Config::default(),
            hints: HashMap::new(),
            compound_sets,
            compound_set_by_ref,
            compound_member_index,
            show_semantics: true,
            inject_ast_indices: false,
            abbreviation_map: None,
        }
    }
}

impl Processor {
    /// Fill unset style punctuation options from the active locale.
    fn resolve_punctuation_defaults(&self, config: &mut Config) {
        let punctuation = config
            .punctuation
            .get_or_insert_with(PunctuationConfig::default);
        punctuation
            .strong_terminal_comma_policy
            .get_or_insert(self.locale.grammar_options.strong_terminal_comma_policy);
        punctuation
            .delimiter_suppressing_terminal_marks
            .get_or_insert_with(|| {
                self.locale
                    .grammar_options
                    .delimiter_suppressing_terminal_marks
                    .clone()
            });
    }

    /// Return whether applying locale punctuation defaults would change `config`.
    fn punctuation_defaults_require_resolution(&self, config: &Config) -> bool {
        let punctuation = config.punctuation.as_ref();
        let policy_is_unset = punctuation
            .and_then(|options| options.strong_terminal_comma_policy)
            .is_none();
        let marks_are_unset = punctuation
            .and_then(|options| options.delimiter_suppressing_terminal_marks.as_ref())
            .is_none();

        (policy_is_unset
            && self.locale.grammar_options.strong_terminal_comma_policy
                != citum_schema::options::StrongTerminalCommaPolicy::default())
            || (marks_are_unset
                && self
                    .locale
                    .grammar_options
                    .delimiter_suppressing_terminal_marks
                    != "?!…")
    }

    /// Apply locale punctuation defaults only when they change the effective config.
    fn with_punctuation_defaults<'a>(
        &self,
        config: std::borrow::Cow<'a, Config>,
    ) -> std::borrow::Cow<'a, Config> {
        if !self.punctuation_defaults_require_resolution(&config) {
            return config;
        }

        let mut config = config.into_owned();
        self.resolve_punctuation_defaults(&mut config);
        std::borrow::Cow::Owned(config)
    }

    /// Core internal constructor path.
    ///
    /// Resolves the style presets before initializing the processor.
    fn build_processor(
        style: Style,
        bibliography: Bibliography,
        locale: Locale,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Self {
        let style = style.into_resolved();
        Self::build_processor_pre_resolved(style, bibliography, locale, compound_sets)
    }

    /// Build a processor from an already-resolved style, skipping preset resolution.
    ///
    /// Use this when the style was cloned from a processor that has already
    /// called `into_resolved()`, to avoid a second resolution pass that would
    /// re-apply preset defaults and overwrite null-cleared fields.
    pub(super) fn build_processor_pre_resolved(
        style: Style,
        bibliography: Bibliography,
        locale: Locale,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Self {
        let (compound_set_by_ref, compound_member_index) =
            Self::build_compound_set_indexes(&compound_sets);
        let mut processor = Processor {
            style,
            bibliography,
            locale,
            default_config: Config::default(),
            hints: HashMap::new(),
            compound_sets,
            compound_set_by_ref,
            compound_member_index,
            show_semantics: true,
            inject_ast_indices: false,
            abbreviation_map: None,
        };

        // Pre-calculate hints for disambiguation.
        processor.hints = processor.calculate_hints();
        processor
    }

    /// Validate compound sets against the bibliography.
    fn try_validate_compound_sets(
        bibliography: &Bibliography,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Result<IndexMap<String, Vec<String>>, ProcessorError> {
        super::validate_compound_sets(Some(compound_sets), bibliography)
            .map(Option::unwrap_or_default)
    }

    /// Validate compound sets, falling back to an empty map on error.
    fn validate_compound_sets_or_default(
        bibliography: &Bibliography,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> IndexMap<String, Vec<String>> {
        Self::try_validate_compound_sets(bibliography, compound_sets).unwrap_or_default()
    }

    /// Build flat reverse-lookup maps for compound sets.
    ///
    /// Maps reference IDs to their parent set ID and their 0-based position
    /// within that set.
    fn build_compound_set_indexes(
        sets: &IndexMap<String, Vec<String>>,
    ) -> (HashMap<String, String>, HashMap<String, usize>) {
        let mut by_ref = HashMap::new();
        let mut member_index = HashMap::new();
        for (set_id, members) in sets {
            for (idx, member) in members.iter().enumerate() {
                by_ref.insert(member.clone(), set_id.clone());
                member_index.insert(member.clone(), idx);
            }
        }
        (by_ref, member_index)
    }

    /// Check whether the style uses note-based citations (footnotes/endnotes).
    pub(crate) fn is_note_style(&self) -> bool {
        self.get_config()
            .processing
            .as_ref()
            .is_some_and(|processing| matches!(processing, citum_schema::options::Processing::Note))
    }

    /// Check whether the style uses numeric citation rendering.
    fn is_numeric_style(&self) -> bool {
        self.get_config()
            .processing
            .as_ref()
            .is_some_and(|processing| {
                matches!(processing, citum_schema::options::Processing::Numeric)
            })
    }

    /// Check whether the style uses numeric bibliography rendering.
    fn is_numeric_bibliography_style(&self) -> bool {
        self.get_bibliography_config()
            .processing
            .as_ref()
            .is_some_and(|processing| {
                matches!(processing, citum_schema::options::Processing::Numeric)
            })
    }

    /// Resolve the effective bibliography sort specification.
    ///
    /// Accounts for style overrides, processing-family preset defaults, and
    /// (when neither applies) an explicit config-level `sort:`. The returned
    /// `bool` is `true` only when the sort originates from that last,
    /// explicit config-level step; the caller then applies the deterministic
    /// entry-ID tiebreaker so config-driven sorts (`Processing::Numeric` /
    /// `Custom` with an explicit `sort:`) stay oracle-fidelity-stable.
    /// Styles with a processing-family preset default (`AuthorDate*`, `Note`,
    /// `Label`) never reach the config-level step; those without any
    /// `processing:` fall back to the default family (`AuthorDate`), whose
    /// config carries the author-date sort preset.
    fn resolved_bibliography_sort(&self) -> Option<(citum_schema::grouping::GroupSort, bool)> {
        if let Some(sort_spec) = self
            .style
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.sort.as_ref())
        {
            return Some((sort_spec.resolve(), false));
        }

        let bibliography_config = self.get_bibliography_config();

        if let Some(preset) = bibliography_config
            .processing
            .as_ref()
            .and_then(citum_schema::options::Processing::default_bibliography_sort)
        {
            return Some((preset.group_sort(), false));
        }

        bibliography_config
            .processing
            .clone()
            .unwrap_or_default()
            .config()
            .sort
            .map(|sort_entry| (sort_entry.resolve().group_sort(), true))
    }

    /// Initialize numeric citation numbers from bibliography insertion order.
    ///
    /// citeproc-js registers all bibliography items before citation rendering in
    /// the oracle workflow, so numeric labels are stable by reference registry
    /// order rather than first-citation order.
    ///
    /// When the style declares an explicit bibliography sort, or the
    /// processing family provides a bibliography default, citation numbers
    /// must follow that resolved bibliography order.
    pub(crate) fn initialize_numeric_citation_numbers(&self, run: &mut RunState) {
        if !self.is_numeric_style() {
            return;
        }

        self.initialize_numeric_numbers(run, self.sort_citation_number_order());
    }

    /// Initialize numeric bibliography numbers from resolved bibliography order.
    pub(crate) fn initialize_numeric_bibliography_numbers(&self, run: &mut RunState) {
        if !self.is_numeric_bibliography_style() {
            return;
        }

        self.initialize_numeric_numbers(run, self.sort_bibliography_number_order());
    }

    /// Initialize citation numbers if the map is currently empty.
    fn initialize_numeric_numbers(&self, run: &mut RunState, ordered_ids: Vec<String>) {
        if !run
            .citation_numbers
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
        {
            return;
        }

        self.initialize_numeric_citation_numbers_from_ordered_ids(run, ordered_ids);
    }

    /// Calculate the document-wide reference order for citation numbering.
    fn sort_citation_number_order(&self) -> Vec<String> {
        self.sort_references(self.bibliography.values().collect())
            .into_iter()
            .filter_map(citum_schema::reference::InputReference::id)
            .map(String::from)
            .collect()
    }

    /// Calculate the reference order for bibliography numbering.
    fn sort_bibliography_number_order(&self) -> Vec<String> {
        self.sort_references(self.bibliography.values().collect())
            .into_iter()
            .filter_map(citum_schema::reference::InputReference::id)
            .map(String::from)
            .collect()
    }

    /// Assign stable numeric labels to references based on a document order.
    ///
    /// Also populates compound groups for numeric styles that enable compound
    /// numbering.
    fn initialize_numeric_citation_numbers_from_ordered_ids(
        &self,
        run: &mut RunState,
        ordered_ids: Vec<String>,
    ) {
        let mut numbers = run
            .citation_numbers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !numbers.is_empty() {
            return;
        }

        let compound_config = self.get_bibliography_options().compound_numeric.clone();

        if compound_config.is_some() {
            let mut set_first_seen: IndexMap<String, usize> = IndexMap::new();
            let mut current_number = 1usize;
            run.compound_groups.clear();

            for ref_id in &ordered_ids {
                if let Some(set_id) = self.compound_set_by_ref.get(ref_id) {
                    if let Some(&number) = set_first_seen.get(set_id) {
                        numbers.insert(ref_id.clone(), number);
                    } else {
                        set_first_seen.insert(set_id.clone(), current_number);
                        if let Some(members) = self.compound_sets.get(set_id) {
                            let present_members: Vec<String> = members
                                .iter()
                                .filter(|id| self.bibliography.contains_key(*id))
                                .cloned()
                                .collect();
                            for member in &present_members {
                                numbers.insert(member.clone(), current_number);
                            }
                            if present_members.len() > 1 {
                                run.compound_groups.insert(current_number, present_members);
                            }
                        } else {
                            numbers.insert(ref_id.clone(), current_number);
                        }
                        current_number += 1;
                    }
                } else if !numbers.contains_key(ref_id) {
                    numbers.insert(ref_id.clone(), current_number);
                    current_number += 1;
                }
            }
        } else {
            for (index, ref_id) in ordered_ids.into_iter().enumerate() {
                numbers.insert(ref_id, index + 1);
            }
        }
    }

    /// Begin a new render run.
    ///
    /// Allocates fresh per-run state (citation numbers, cite-order tracking,
    /// dynamic compound groups, first-note tracking) and performs numeric
    /// citation-number pre-initialization from the processor's immutable
    /// bibliography/style data. Registration methods (`&self, &mut RunState`)
    /// populate the returned `RunState` in citation-processing order; call
    /// [`RunState::finalize`] before rendering. See
    /// `docs/specs/EXPLICIT_RENDER_RUN_STATE.md`.
    #[must_use]
    pub fn begin_run(&self) -> RunState {
        let mut run = RunState::default();
        self.initialize_numeric_citation_numbers(&mut run);
        self.initialize_numeric_bibliography_numbers(&mut run);
        run
    }

    /// Create a new processor with default English locale (en-US).
    #[must_use]
    pub fn new(style: Style, bibliography: Bibliography) -> Self {
        Self::with_compound_sets(style, bibliography, IndexMap::new())
    }

    /// Create a new processor with explicit compound sets, returning an error for invalid sets.
    ///
    /// # Errors
    ///
    /// Returns an error when any compound set references unknown bibliography
    /// entries or reuses the same member more than once.
    pub fn try_with_compound_sets(
        style: Style,
        bibliography: Bibliography,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Result<Self, ProcessorError> {
        Self::try_with_locale_and_compound_sets(style, bibliography, Locale::en_us(), compound_sets)
    }

    /// Create a new processor with explicit compound sets.
    ///
    /// If `compound_sets` is invalid, this constructor ignores the supplied sets
    /// and falls back to a processor without compound sets.
    #[must_use]
    pub fn with_compound_sets(
        style: Style,
        bibliography: Bibliography,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Self {
        let validated_sets = Self::validate_compound_sets_or_default(&bibliography, compound_sets);
        Self::build_processor(style, bibliography, Locale::en_us(), validated_sets)
    }

    /// Create a new processor with a specified locale.
    ///
    /// The locale determines term translations and locale-specific formatting behavior.
    #[must_use]
    pub fn with_locale(style: Style, bibliography: Bibliography, locale: Locale) -> Self {
        Self::with_locale_and_compound_sets(style, bibliography, locale, IndexMap::new())
    }

    /// Create a new processor with explicit locale and compound sets, returning
    /// an error for invalid sets.
    ///
    /// # Errors
    ///
    /// Returns an error when any compound set references unknown bibliography
    /// entries or reuses the same member more than once.
    pub fn try_with_locale_and_compound_sets(
        style: Style,
        bibliography: Bibliography,
        locale: Locale,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Result<Self, ProcessorError> {
        let validated_sets = Self::try_validate_compound_sets(&bibliography, compound_sets)?;
        Ok(Self::build_processor(
            style,
            bibliography,
            locale,
            validated_sets,
        ))
    }

    /// Create a new processor with a specified locale and explicit compound sets.
    ///
    /// The locale determines term translations and locale-specific formatting behavior.
    ///
    /// If `compound_sets` is invalid, this constructor ignores the supplied sets
    /// and falls back to a processor without compound sets.
    #[must_use]
    pub fn with_locale_and_compound_sets(
        style: Style,
        bibliography: Bibliography,
        locale: Locale,
        compound_sets: IndexMap<String, Vec<String>>,
    ) -> Self {
        let validated_sets = Self::validate_compound_sets_or_default(&bibliography, compound_sets);
        Self::build_processor(style, bibliography, locale, validated_sets)
    }

    /// Create a new processor, loading the locale from disk.
    ///
    /// Loads the locale specified in the style's `default_locale` field from the given directory,
    /// falling back to en-US if not found or not specified.
    #[must_use]
    pub fn with_style_locale(
        style: Style,
        bibliography: Bibliography,
        locales_dir: &std::path::Path,
    ) -> Self {
        let style = style.into_resolved();
        let locale = if let Some(ref locale_id) = style.info.default_locale {
            Locale::load(locale_id, locales_dir)
        } else {
            Locale::en_us()
        };
        Self::with_locale_and_compound_sets(style, bibliography, locale, IndexMap::new())
    }

    /// Return a copy of the processor that injects source template indices into semantic HTML.
    #[must_use]
    pub fn with_inject_ast_indices(mut self, inject_ast_indices: bool) -> Self {
        self.inject_ast_indices = inject_ast_indices;
        self
    }

    /// Enable or disable source template index injection for semantic HTML output.
    pub fn set_inject_ast_indices(&mut self, inject_ast_indices: bool) {
        self.inject_ast_indices = inject_ast_indices;
    }

    /// Return the global style configuration.
    pub fn get_config(&self) -> &Config {
        self.style.options.as_ref().unwrap_or(&self.default_config)
    }

    /// Return merged config for citation rendering.
    ///
    /// Combines global style options with citation-specific overrides, borrowing
    /// the global configuration when no merge or locale resolution is required.
    pub fn get_citation_config(&self) -> std::borrow::Cow<'_, Config> {
        let base = self.get_config();
        let config = match self
            .style
            .citation
            .as_ref()
            .and_then(|citation| citation.options.as_ref())
        {
            Some(citation_options) => std::borrow::Cow::Owned(
                citation_options
                    .merged_with_raw(base, self.style.scoped_raw_options.citation.as_ref()),
            ),
            None => std::borrow::Cow::Borrowed(base),
        };
        self.with_punctuation_defaults(config)
    }

    /// Return merged shared config for bibliography rendering.
    ///
    /// Combines global shared style options with bibliography-local shared overrides,
    /// borrowing the global configuration when no merge or locale resolution is required.
    pub fn get_bibliography_config(&self) -> std::borrow::Cow<'_, Config> {
        let base = self.get_config();
        let config = match self
            .style
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.options.as_ref())
        {
            Some(bibliography_options) => std::borrow::Cow::Owned(
                bibliography_options
                    .merged_with_raw(base, self.style.scoped_raw_options.bibliography.as_ref()),
            ),
            None => std::borrow::Cow::Borrowed(base),
        };
        self.with_punctuation_defaults(config)
    }

    /// Return effective bibliography-only configuration.
    pub fn get_bibliography_options(&self) -> std::borrow::Cow<'_, BibliographyConfig> {
        match self
            .style
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.options.as_ref())
        {
            Some(bibliography_options) => {
                std::borrow::Cow::Owned(bibliography_options.to_bibliography_config())
            }
            None => std::borrow::Cow::Owned(BibliographyConfig::default()),
        }
    }

    /// Sort references according to the style's bibliography sort specification.
    ///
    /// Uses style-specified sort keys (author, title, issued, etc.) and sort order.
    pub fn sort_references<'a>(&self, references: Vec<&'a Reference>) -> Vec<&'a Reference> {
        let bibliography_config = self.get_bibliography_config();
        let mut sorted_refs = match self.resolved_bibliography_sort() {
            Some((sort_spec, true)) => {
                let mut sorter = crate::sorting::ReferenceSorter::with_bibliography_config(
                    &self.locale,
                    &bibliography_config,
                );
                if let Some(spec) = self.style.bibliography.as_ref() {
                    sorter = sorter.with_bibliography_spec(spec);
                }
                sorter.sort_references_with_id_tiebreak(references, &sort_spec)
            }
            Some((sort_spec, false)) => {
                let mut sorter = crate::sorting::ReferenceSorter::with_bibliography_config(
                    &self.locale,
                    &bibliography_config,
                );
                if let Some(spec) = self.style.bibliography.as_ref() {
                    sorter = sorter.with_bibliography_spec(spec);
                }
                sorter.sort_references(references, &sort_spec)
            }
            None => references,
        };

        let bibliography_options = self.get_bibliography_options();
        if let Some(partitioning) =
            effective_sort_partitioning(&bibliography_options, &bibliography_config).as_ref()
            && crate::sort_partitioning::should_sort_flat(partitioning)
        {
            crate::sort_partitioning::sort_by_partition(
                sorted_refs.as_mut_slice(),
                &self.locale,
                partitioning,
            );
        }

        sorted_refs
    }

    /// Sort citation items according to the style's citation sort specification.
    pub fn sort_citation_items(
        &self,
        items: Vec<CitationItem>,
        spec: &citum_schema::CitationSpec,
    ) -> Vec<CitationItem> {
        if let Some(sort_spec) = &spec.sort {
            let items_with_refs: Vec<(CitationItem, Option<&Reference>)> = items
                .into_iter()
                .map(|item| {
                    let reference = self.bibliography.get(&item.id);
                    (item, reference)
                })
                .collect();

            let resolved_sort = sort_spec.resolve();
            let citation_config = self.get_citation_config();
            let sorter = crate::sorting::ReferenceSorter::with_bibliography_config(
                &self.locale,
                &citation_config,
            )
            .with_citation_spec(spec);
            let sorted =
                sorter.sort_by_keys(items_with_refs, &resolved_sort.template, |item| item.1);

            return sorted.into_iter().map(|(item, _reference)| item).collect();
        }

        items
    }

    /// Calculate disambiguation hints needed for the style.
    ///
    /// Analyzes the bibliography to determine which items need disambiguation
    /// (year suffixes, etc.) and calculates hints for efficient rendering.
    pub fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let citation_config = self.get_citation_config();
        let config = citation_config.as_ref();
        let bibliography_config = self.get_bibliography_config();
        let bibliography_sort = self.resolved_bibliography_sort();

        let mut disambiguator = if let Some((resolved_sort, _id_tiebreak)) = &bibliography_sort {
            Disambiguator::with_group_sort(
                &self.bibliography,
                config,
                &bibliography_config,
                &self.locale,
                resolved_sort,
            )
        } else {
            Disambiguator::new(
                &self.bibliography,
                config,
                &bibliography_config,
                &self.locale,
            )
        };

        if let Some(citation_spec) = self.style.citation.as_ref() {
            disambiguator = disambiguator.with_citation_spec(citation_spec);
        }
        if let Some(bibliography_spec) = self.style.bibliography.as_ref() {
            disambiguator = disambiguator.with_bibliography_spec(bibliography_spec);
        }

        disambiguator.calculate_hints()
    }
}

fn effective_sort_partitioning(
    bibliography_options: &BibliographyConfig,
    bibliography_config: &Config,
) -> Option<BibliographySortPartitioning> {
    if let Some(partitioning) = &bibliography_options.sort_partitioning {
        return Some(partitioning.clone());
    }

    bibliography_config
        .sorting
        .as_ref()
        .is_some_and(|sorting| {
            sorting.effective_multilingual() == SortingMultilingualMode::PerScript
        })
        .then(|| BibliographySortPartitioning {
            by: BibliographyPartitionKind::Script,
            mode: BibliographyPartitionMode::SortOnly,
            order: Vec::new(),
            headings: HashMap::new(),
            unknown_fields: Default::default(),
        })
}
