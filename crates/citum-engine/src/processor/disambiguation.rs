/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::reference::{Bibliography, Reference};
use crate::values::ProcHints;
use citum_schema::options::{Config, GivennameRule};
use citum_schema::reference::Title;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::sorting::{ReferenceSorter, compare_none_last};
use citum_schema::grouping::GroupSort;
use citum_schema::locale::Locale;

/// Handles disambiguation logic for author-date citations.
///
/// Disambiguation resolves ambiguities when multiple references produce
/// identical rendered strings. The processor applies strategies in cascade:
///
/// 1. **Name expansion** (`disambiguate-add-names`): If et-al is triggered
///    in the base citation, try expanding the author list to differentiate
///    references with same first author and year.
///
/// 2. **Given name expansion** (`disambiguate-add-givenname`): Add initials
///    or full given names to author list to resolve remaining collisions
///    (e.g., "Smith, John" vs "Smith, Jane").
///
/// 3. **Combined expansion**: Try showing both more names AND given names
///    to maximize differentiation before falling back to year suffix.
///
/// 4. **Year suffix fallback** (`disambiguate-add-year-suffix`): If above
///    strategies fail, append letters (a, b, c, ..., z, aa, ab, ...) to
///    the year. Ordering follows the resolved per-group sort when one is
///    configured, otherwise lowercase reference title order.
///
/// ## Algorithm Overview
///
/// - References are grouped by their base collision key
///   (for example, `smith:2020` or a label key)
/// - For each group with 2+ collisions, strategies are applied in order
/// - Once a strategy resolves ambiguity, higher-priority strategies skip
/// - Year suffix assignment is deterministic from the resolved per-group sort
///
/// ## Output
///
/// Returns `ProcHints` for each reference containing:
/// - `group_index`: Position within collision group (1-indexed)
/// - `group_length`: Total references in collision group
/// - `group_key`: Author-year key used for grouping
/// - `disamb_condition`: Whether year suffix should be applied
/// - `expand_given_names`: Whether to show given names/initials
/// - `min_names_to_show`: Minimum author count for name expansion
pub struct Disambiguator<'a> {
    bibliography: &'a Bibliography,
    config: &'a Config,
    /// Effective bibliography config governing bibliography-owned date-slot
    /// grouping and multilingual/locale sort-key policy. Year-suffix grouping
    /// and ordering must use this — not `config` — when the bibliography
    /// supplies the corresponding template or sort behavior.
    sort_config: &'a Config,
    locale: &'a Locale,
    group_sort: Option<&'a GroupSort>,
    citation_spec: Option<&'a citum_schema::CitationSpec>,
    citation_primary_may_be_list: bool,
    bibliography_spec: Option<&'a citum_schema::BibliographySpec>,
    /// Whether the resolved bibliography sort breaks ties by reference id, as
    /// `ReferenceSorter::sort_references_with_id_tiebreak` does. Mirrors the
    /// renderer's tiebreak in `sort_group_for_year_suffix` so year-suffix
    /// order agrees with render order even when the sort keys alone don't
    /// fully determine it. csl26-m8la.
    id_tiebreak: bool,
}

#[derive(Clone, Copy, Default)]
struct DisambiguationFlags {
    add_names: bool,
    add_givenname: bool,
    year_suffix: bool,
    is_label_mode: bool,
    primary_givenname_only: bool,
    /// Whether given-name expansion may escalate all the way to the full
    /// given name. `false` for `all-names-with-initials` /
    /// `primary-name-with-initials`, which must stop at the initials level
    /// even when initials alone don't resolve a collision (csl26-h9jy).
    givenname_full_allowed: bool,
    /// Whether `GivennameRule::ByCite`'s per-position expansion ceiling is
    /// active: `disambiguate-add-givenname` is on and the resolved
    /// `givenname_rule` is `ByCite`. When set, `select_group_hint_action`
    /// routes the whole group through `select_by_cite_resolution` instead
    /// of the uniform name-partition/givenname-resolution cascade every
    /// other rule uses. See csl26-5753.
    by_cite_positional: bool,
}

/// How far a single author position's given name escalated within
/// `select_by_cite_resolution`'s search. Unlike the uniform
/// `expand_given_names_full` flag every other rule uses, `by-cite` tracks
/// this per position (csl26-5753): a position stops at `Initials` once
/// that's enough to distinguish it from its collision partners, and only
/// escalates to `Full` when initials alone still collide (e.g. "Brandon"
/// and "Biff" both reduce to "B.", csl26-h9jy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GivennameLevel {
    /// Initials (only reachable when `initials_available`, mirroring real
    /// CSL's `initialize-with`-presence gate).
    Initials,
    /// Full given name.
    Full,
}

/// One reference's resolved `by-cite` plan: how many names to show and
/// which shown positions needed to escalate all the way to the full given
/// name (every other shown position defaults to `Initials` when reachable,
/// otherwise `Full` -- see `positional_given_level`) to make it (and its
/// remaining collision partners, if any) unique. Produced by
/// `select_by_cite_resolution` and consumed by `apply_by_cite_plans`.
struct ByCitePlan<'a> {
    reference: &'a CachedReference<'a>,
    /// `None` means "no override, use the style's own default" -- reserved
    /// for a bucket where the search never made *any* progress at all (see
    /// `select_by_cite_resolution`'s `single_bucket` reset): retaining a
    /// no-op `Some(n)` there would still flip
    /// `grouping.rs::group_citation_items_by_author`'s
    /// `preserve_individual_citations` check, defeating same-author
    /// collapse for a collision that's genuinely unresolvable by name at
    /// all (e.g. the exact same person cited for two different works).
    /// Every other plan -- resolved or an unresolved residual next to a
    /// sibling that *did* split off -- carries a genuine `Some(n)`.
    min_names_to_show: Option<usize>,
    /// Whether any given-name work was needed at all. `false` when the
    /// bucket was already resolved by name-count alone (strategy 1), or
    /// reset alongside `min_names_to_show: None` above -- `full_positions`
    /// is meaningless in either case.
    expand_given_names: bool,
    /// Index-aligned to author position; `true` at an index means that
    /// position must escalate to the full given name. May be empty or
    /// all-`false` when the default depth already resolved everything.
    full_positions: Vec<bool>,
}

struct GroupDisambiguationContext<'a> {
    key: &'a str,
    group: &'a [&'a CachedReference<'a>],
    flags: DisambiguationFlags,
    author_group_lengths: &'a HashMap<String, usize>,
}

#[derive(Clone, Copy)]
struct HintPlan<'a> {
    key: &'a str,
    expand_given_names: bool,
    expand_given_names_full: bool,
    expand_given_names_primary_only: bool,
    min_names_to_show: Option<usize>,
    disamb_condition: bool,
}

#[derive(Clone, Copy)]
enum HintOrder {
    Encountered,
    GroupSorted,
}

enum GroupHintAction<'a> {
    Singleton(&'a CachedReference<'a>),
    LabelYearSuffix,
    NamePartitions {
        min_names_to_show: usize,
        partitions: HashMap<String, Vec<&'a CachedReference<'a>>>,
    },
    GivennameResolution(GivennameLevel),
    CombinedResolution {
        min_names_to_show: usize,
        level: GivennameLevel,
        primary_only_requires_suffix: bool,
    },
    /// `GivennameRule::ByCite`'s per-position resolution: replaces
    /// `NamePartitions`/`GivennameResolution`/`CombinedResolution` entirely
    /// for this rule. `unresolved` holds the last-attempted plan (name
    /// count, positional escalation) for any member the search couldn't
    /// separate even at the maximum author count -- these fall through to
    /// year-suffix, retaining that state rather than resetting it.
    ByCite {
        plans: Vec<ByCitePlan<'a>>,
        unresolved: Vec<ByCitePlan<'a>>,
    },
    FallbackYearSuffix,
}

type ReferenceCache<'a> = Vec<CachedReference<'a>>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ReferenceCacheKey {
    Id(String),
    Index(usize),
}

struct CachedReference<'a> {
    reference: &'a Reference,
    #[allow(dead_code, reason = "Cache key policy is asserted in unit tests.")]
    key: ReferenceCacheKey,
    data: CachedReferenceData,
}

struct CachedReferenceData {
    author_key: String,
    group_key: String,
    names: Vec<crate::reference::FlatName>,
    title_key: Option<String>,
    /// Position of this reference in the bibliography's registry
    /// (`IndexMap`) order — the same order `ReferenceSorter` falls back to
    /// when a resolved sort has no keys, or no keys, left to compare. Used by
    /// `sort_group_for_year_suffix` to mirror the renderer's tiebreak instead
    /// of an independently-computed title order. csl26-m8la.
    index: usize,
}

impl<'a> Disambiguator<'a> {
    /// Creates a disambiguator that uses the default title-based fallback order.
    ///
    /// `sort_config` is the effective bibliography config; pass the same
    /// config used to render and sort the final bibliography so bibliography-
    /// owned date grouping and year-suffix order agree with its output.
    #[must_use]
    pub fn new(
        bibliography: &'a Bibliography,
        config: &'a Config,
        sort_config: &'a Config,
        locale: &'a Locale,
    ) -> Self {
        Self {
            bibliography,
            config,
            sort_config,
            locale,
            group_sort: None,
            citation_spec: None,
            citation_primary_may_be_list: false,
            bibliography_spec: None,
            id_tiebreak: false,
        }
    }

    /// Creates a disambiguator with an explicit per-group sort specification.
    ///
    /// `sort_config` is the effective bibliography config; pass the same
    /// config used to render and sort the final bibliography so bibliography-
    /// owned date grouping and year-suffix order agree with its output.
    #[must_use]
    pub fn with_group_sort(
        bibliography: &'a Bibliography,
        config: &'a Config,
        sort_config: &'a Config,
        locale: &'a Locale,
        group_sort: &'a GroupSort,
    ) -> Self {
        Self {
            bibliography,
            config,
            sort_config,
            locale,
            group_sort: Some(group_sort),
            citation_spec: None,
            citation_primary_may_be_list: false,
            bibliography_spec: None,
            id_tiebreak: false,
        }
    }

    /// Resolve disambiguation names from the effective citation template.
    #[must_use]
    pub fn with_citation_spec(mut self, spec: &'a citum_schema::CitationSpec) -> Self {
        self.citation_spec = Some(spec);
        self.citation_primary_may_be_list = crate::sorting::citation_may_have_list_primary(spec);
        self
    }

    /// Resolve year-suffix sort keys from the effective bibliography template.
    #[must_use]
    pub fn with_bibliography_spec(mut self, spec: &'a citum_schema::BibliographySpec) -> Self {
        self.bibliography_spec = Some(spec);
        self
    }

    /// Mark that the resolved bibliography sort breaks ties by reference id.
    ///
    /// Pass the same flag `Processor::resolved_bibliography_sort` returns for
    /// the sort used to render the final bibliography
    /// (`ReferenceSorter::sort_references_with_id_tiebreak`), so year-suffix
    /// order agrees with render order.
    #[must_use]
    pub fn with_id_tiebreak(mut self, id_tiebreak: bool) -> Self {
        self.id_tiebreak = id_tiebreak;
        self
    }

    /// Calculate processing hints for disambiguation across all references.
    ///
    /// This is a single-pass algorithm that:
    /// 1. Groups references by their base collision key
    /// 2. For each group with multiple references, applies disambiguation
    ///    strategies in cascade order
    /// 3. Returns pre-calculated hints for the renderer
    ///
    /// ## Cascade Order
    ///
    /// For each collision group:
    /// - Try expanding author list (et-al → full names)
    /// - Try adding given names/initials
    /// - Try combined approach (more names + given names)
    /// - Fall back to year suffix (a, b, c, ...)
    ///
    /// ## Performance
    ///
    /// - O(n) for grouping, where n = number of references
    /// - O(g²) for collision detection within each group g
    /// - Total: O(n + Σ(g²)) where typical g << n
    ///
    /// ## Example
    ///
    /// Input bibliography:
    /// - Smith, John (2020) - "Article A"
    /// - Smith, Jane (2020) - "Article B"
    /// - Brown, Tom (2020) - "Article C"
    ///
    /// Output hints:
    /// - "item-1": { `group_key`: "smith:2020", `expand_given_names`: true, `group_length`: 2 }
    /// - "item-2": { `group_key`: "smith:2020", `expand_given_names`: true, `group_length`: 2 }
    /// - "item-3": { `group_key`: "brown:2020" } (no collision)
    #[must_use]
    pub fn calculate_hints(&self) -> HashMap<String, ProcHints> {
        let mut hints = HashMap::new();
        let refs: Vec<&Reference> = self.bibliography.values().collect();
        let flags = self.disambiguation_flags();
        // Always populate title_key when year-suffix disambiguation is active so that
        // sort_group_for_year_suffix can use it as a stable tie-breaker regardless of
        // whether a group_sort is configured.
        let needs_title_key = flags.year_suffix;
        let cache = self.build_reference_cache(&refs, needs_title_key);
        let grouped = self.group_references(&cache);
        let author_group_lengths = self.author_group_lengths(&cache);

        for (key, group) in grouped {
            self.apply_group_hints(
                &mut hints,
                GroupDisambiguationContext {
                    key: &key,
                    group: &group,
                    flags,
                    author_group_lengths: &author_group_lengths,
                },
            );
        }

        hints
    }

    /// Resolves disambiguation configuration from the processor config.
    fn disambiguation_flags(&self) -> DisambiguationFlags {
        let disamb_config = self.config.effective_processing().config().disambiguate;

        DisambiguationFlags {
            add_names: disamb_config.as_ref().is_some_and(|d| d.names),
            add_givenname: disamb_config.as_ref().is_some_and(|d| d.add_givenname),
            year_suffix: disamb_config.as_ref().is_some_and(|d| d.year_suffix),
            is_label_mode: self
                .config
                .processing
                .as_ref()
                .is_some_and(|p| matches!(p, citum_schema::options::Processing::Label(_))),
            primary_givenname_only: disamb_config.as_ref().is_some_and(|d| {
                matches!(
                    d.givenname_rule,
                    GivennameRule::PrimaryName | GivennameRule::PrimaryNameWithInitials
                )
            }),
            givenname_full_allowed: disamb_config.as_ref().is_none_or(|d| {
                !matches!(
                    d.givenname_rule,
                    GivennameRule::AllNamesWithInitials | GivennameRule::PrimaryNameWithInitials
                )
            }),
            by_cite_positional: disamb_config.as_ref().is_some_and(|d| {
                d.add_givenname && matches!(d.givenname_rule, GivennameRule::ByCite)
            }),
        }
    }

    /// The `initialize-with` separator configured for this scope, if any.
    fn initialize_with(&self) -> Option<&String> {
        self.config
            .contributors
            .as_ref()
            .and_then(|c| c.initialize_with.as_ref())
    }

    /// Whether the initials-level rung is reachable in the given-name escalation
    /// ladder. Mirrors real CSL's `initialize-with`-presence gate: available
    /// either because a separator is explicitly configured, or because the
    /// style's baseline form is already `name-form: initials` (which Citum
    /// defaults to a "." separator for when unset).
    fn initials_available(&self) -> bool {
        self.initialize_with().is_some()
            || self
                .config
                .contributors
                .as_ref()
                .and_then(|c| c.name_form)
                .is_some_and(|form| matches!(form, citum_schema::options::NameForm::Initials))
    }

    /// The number of names the base (non-escalated) citation already shows
    /// for a group, before any `disambiguate-add-names` growth:
    /// `et-al-use-first` when the style truncates author lists at all, or
    /// every author when it doesn't (no `shorten` configured, so there's no
    /// "hidden" position to begin with -- every position is already
    /// visible, just not yet given-name-escalated). `by-cite`'s per-position
    /// search starts here even when `disambiguate-add-names` is disabled:
    /// that flag only forbids *growing* past this count, not examining
    /// names the base citation already renders.
    ///
    /// Does not account for a reference whose own author count falls below
    /// `et-al-min` (which would show all its names regardless of
    /// `use_first`) -- the search still starts from the style-wide
    /// `use_first`, a residual approximation shared with the group-wide `n`
    /// concept the rest of this cascade already uses.
    fn base_visible_name_count(&self, max_authors: usize) -> usize {
        match self
            .config
            .contributors
            .as_ref()
            .and_then(|c| c.shorten.as_ref())
        {
            Some(opts) => usize::from(opts.use_first).clamp(1, max_authors.max(1)),
            None => max_authors,
        }
    }

    /// Builds an internal cache of reference data (author keys, group keys, titles)
    /// to avoid redundant string generation during disambiguation.
    fn build_reference_cache<'b>(
        &self,
        refs: &[&'b Reference],
        needs_title_key: bool,
    ) -> ReferenceCache<'b> {
        // Grouping must resolve the substitute chain from `sort_config` (the
        // effective bibliography config, per its doc comment), not `config`
        // (which may be citation-scoped). A style can override the
        // bibliography-scope substitute independently of the citation-scope
        // one (e.g. GB/T 7714 author-date's constant `佚名` anonymous-author
        // fallback, csl26-6eak) — if grouping used the citation substitute
        // instead, it would see a per-reference substituted title where the
        // bibliography renders the same constant text for every such
        // reference, so those references would each form a singleton group
        // instead of colliding on year like a real shared author.
        let substitute = self.sort_config.effective_substitute();
        refs.iter()
            .enumerate()
            .map(|(index, reference)| {
                let locale = self.effective_locale_for_reference(reference);
                let names = if self.citation_primary_may_be_list {
                    self.citation_spec
                        .and_then(|spec| {
                            crate::sorting::primary_contributor_for_citation(spec, reference)
                        })
                        .filter(|component| component.contributor.is_multiple())
                        .map_or_else(
                            || {
                                crate::values::contributor::substitute::effective_primary_names(
                                    reference,
                                    substitute.as_ref(),
                                    self.config,
                                    locale.as_ref(),
                                )
                            },
                            |component| {
                                crate::values::contributor::merged::semantic_names(
                                    &component,
                                    reference,
                                    self.config,
                                    locale.as_ref(),
                                )
                            },
                        )
                } else {
                    crate::values::contributor::substitute::effective_primary_names(
                        reference,
                        substitute.as_ref(),
                        self.config,
                        locale.as_ref(),
                    )
                };
                let author_key = self.build_author_slot_key(
                    reference,
                    &names,
                    substitute.as_ref(),
                    locale.as_ref(),
                );
                let group_key = self.build_group_key(index, reference, &author_key);
                // Year-suffix letters (a, b, c…) must follow the effective bibliography
                // sort order. Reuse the bibliography title sort key (leading-article
                // stripping + locale collation) so suffix assignment cannot diverge from
                // the rendered order — a raw lowercased title sorts "An Ecology" before
                // "Biology", producing `2019b` before `2019a` (DISAMBIGUATION.md §3).
                let title_key = needs_title_key.then(|| {
                    crate::sort_support::title_sort_key_with_options(
                        reference,
                        self.locale,
                        &crate::sort_support::SortKeyOptions::from_config(self.sort_config),
                    )
                });

                CachedReference {
                    reference,
                    key: Self::reference_cache_key(index, reference),
                    data: CachedReferenceData {
                        author_key,
                        group_key,
                        names,
                        title_key,
                        index,
                    },
                }
            })
            .collect()
    }

    fn build_author_slot_key(
        &self,
        reference: &Reference,
        author_names: &[crate::reference::FlatName],
        substitute: &citum_schema::options::Substitute,
        locale: &Locale,
    ) -> String {
        let author_key = self.build_author_key(author_names);
        if !author_key.is_empty() {
            return author_key;
        }

        match crate::values::contributor::substitute::effective_primary(
            reference,
            substitute,
            self.sort_config,
            locale,
        ) {
            Some(crate::values::contributor::substitute::EffectivePrimary::Title {
                title, ..
            }) => Self::title_substitute_key(title),
            // A terminal message is a shared rendered author identity, so
            // anonymous works collide consistently for year-suffix handling.
            None => substitute
                .otherwise_message()
                .and_then(|message| {
                    crate::values::date::fallback_message_discriminant(
                        &message.to_template_message(),
                        locale,
                        self.sort_config,
                    )
                })
                .unwrap_or_default(),
            Some(_) => String::new(),
        }
    }

    fn effective_locale_for_reference(&self, reference: &Reference) -> Cow<'a, Locale> {
        let language = crate::values::effective_item_language(reference);
        let localized_locale = if let Some(spec) = self.bibliography_spec {
            spec.resolve_localized_template(language.as_deref())
                .and_then(|resolved| resolved.locale)
        } else {
            self.citation_spec
                .and_then(|spec| spec.resolve_localized_template(language.as_deref()))
                .and_then(|resolved| resolved.locale)
        };

        crate::processor::rendering::effective_locale_for_reference(
            reference,
            localized_locale.as_deref(),
            self.sort_config,
            self.locale,
        )
    }

    /// Calculates how many references in `refs` share the same `author_key`.
    /// The returned map is keyed only by `author_key` and is later used when
    /// populating `ProcHints::group_length`, rather than representing the size
    /// of a per-`group_key` collision group.
    fn author_group_lengths(&self, refs: &ReferenceCache<'_>) -> HashMap<String, usize> {
        let mut author_group_lengths = HashMap::new();
        for reference in refs {
            let author_key = &reference.data.author_key;
            if !author_key.is_empty() {
                *author_group_lengths.entry(author_key.clone()).or_insert(0) += 1;
            }
        }
        author_group_lengths
    }

    /// Orchestrates the disambiguation cascade for a single collision group.
    /// It attempts strategies in increasing order of disruptiveness (expansion -> year suffix).
    fn apply_group_hints(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        context: GroupDisambiguationContext<'_>,
    ) {
        match self.select_group_hint_action(&context) {
            GroupHintAction::Singleton(reference) => {
                self.insert_hint(
                    hints,
                    reference,
                    context.author_group_lengths,
                    ProcHints::default(),
                );
            }
            GroupHintAction::LabelYearSuffix => {
                self.apply_year_suffix(hints, &context, false, false, None);
            }
            GroupHintAction::NamePartitions {
                min_names_to_show,
                partitions,
            } => self.apply_name_partitions(hints, &context, min_names_to_show, &partitions),
            GroupHintAction::GivennameResolution(level) => {
                self.apply_resolution(
                    hints,
                    context.group,
                    &context,
                    true,
                    level == GivennameLevel::Full,
                    None,
                );
            }
            GroupHintAction::CombinedResolution {
                min_names_to_show,
                level,
                primary_only_requires_suffix,
            } => {
                if primary_only_requires_suffix {
                    self.apply_year_suffix_for_group(
                        hints,
                        context.group,
                        &context,
                        true,
                        level == GivennameLevel::Full,
                        Some(min_names_to_show),
                    );
                } else {
                    self.apply_resolution(
                        hints,
                        context.group,
                        &context,
                        true,
                        level == GivennameLevel::Full,
                        Some(min_names_to_show),
                    );
                }
            }
            GroupHintAction::ByCite { plans, unresolved } => {
                self.apply_by_cite_plans(hints, &context, plans, unresolved);
            }
            GroupHintAction::FallbackYearSuffix => {
                self.apply_year_suffix(hints, &context, false, false, None);
            }
        }
    }

    /// Selects the first applicable disambiguation action without mutating hint state.
    fn select_group_hint_action<'b>(
        &self,
        context: &GroupDisambiguationContext<'b>,
    ) -> GroupHintAction<'b> {
        if let Some(reference) = self.select_singleton_hint(context) {
            return GroupHintAction::Singleton(reference);
        }

        if self.select_label_mode_year_suffix(context) {
            return GroupHintAction::LabelYearSuffix;
        }

        if context.flags.by_cite_positional {
            let (plans, unresolved) = self.select_by_cite_resolution(context);
            return GroupHintAction::ByCite { plans, unresolved };
        }

        if let Some((min_names_to_show, partitions)) = self.select_name_partitions(context) {
            return GroupHintAction::NamePartitions {
                min_names_to_show,
                partitions,
            };
        }

        if let Some(level) = self.select_givenname_resolution(context) {
            return GroupHintAction::GivennameResolution(level);
        }

        if let Some((min_names_to_show, level, primary_only_requires_suffix)) =
            self.select_combined_resolution(context)
        {
            return GroupHintAction::CombinedResolution {
                min_names_to_show,
                level,
                primary_only_requires_suffix,
            };
        }

        GroupHintAction::FallbackYearSuffix
    }

    /// Selects singleton handling for groups with only one reference (no collision).
    fn select_singleton_hint<'b>(
        &self,
        context: &GroupDisambiguationContext<'b>,
    ) -> Option<&'b CachedReference<'b>> {
        if context.group.len() == 1 {
            #[allow(clippy::indexing_slicing, reason = "context.group.len() == 1")]
            return Some(context.group[0]);
        }

        None
    }

    /// Selects year-suffix disambiguation specifically for label-based styles (e.g. [Knu84a]).
    fn select_label_mode_year_suffix(&self, context: &GroupDisambiguationContext<'_>) -> bool {
        context.flags.is_label_mode && context.flags.year_suffix
    }

    /// Selects partitions produced by expanding the number of names shown (et al. expansion).
    fn select_name_partitions<'b>(
        &self,
        context: &GroupDisambiguationContext<'b>,
    ) -> Option<(usize, HashMap<String, Vec<&'b CachedReference<'b>>>)> {
        context
            .flags
            .add_names
            .then(|| self.partition_by_name_expansion(context.group))
            .flatten()
    }

    /// Selects collision resolution by adding given names or initials.
    ///
    /// Tries the given-name escalation ladder (initials, then full — see
    /// `resolve_givenname_level`) and returns the level that resolves the
    /// collision, if any. (With n=1, the full and primary-only keys are
    /// equivalent — both inspect only the primary author — so no separate
    /// primary-only check is needed here.)
    fn select_givenname_resolution(
        &self,
        context: &GroupDisambiguationContext<'_>,
    ) -> Option<GivennameLevel> {
        if !context.flags.add_givenname {
            return None;
        }
        self.resolve_givenname_level(context.group, None, false, context.flags)
    }

    /// Entry point for `GivennameRule::ByCite`'s per-position resolution.
    ///
    /// Strategy 1 (name-count growth via `disambiguate-add-names`) is tried
    /// first, exactly as every other rule -- `by-cite` has no authority over
    /// that axis (`DISAMBIGUATION.md` §2.1.1). Any resulting family bucket
    /// that's still colliding is handed to `resolve_by_cite_positions`,
    /// which escalates given-name positions left to right (growing the
    /// shown name count further when every position at the current count
    /// has been tried without effect), splitting the bucket as soon as a
    /// position's value distinguishes some of its members.
    fn select_by_cite_resolution<'b>(
        &self,
        context: &GroupDisambiguationContext<'b>,
    ) -> (Vec<ByCitePlan<'b>>, Vec<ByCitePlan<'b>>) {
        let group = context.group;
        let flags = context.flags;
        let max_authors = group
            .iter()
            .map(|reference| reference.data.names.len())
            .max()
            .unwrap_or(0);
        let base_n = self.base_visible_name_count(max_authors);

        let buckets: Vec<(usize, Vec<&'b CachedReference<'b>>)> = if flags.add_names {
            match self.partition_by_name_expansion(group) {
                Some((n, partitions)) => {
                    partitions.into_values().map(|bucket| (n, bucket)).collect()
                }
                None => vec![(base_n, group.to_vec())],
            }
        } else {
            vec![(base_n, group.to_vec())]
        };

        // Whether `disambiguate-add-names` ever actually split the group by
        // family name into more than one bucket. `partition_by_name_expansion`
        // only ever returns `Some` when it found >1 partitions, so this is
        // exactly "did strategy 1 contribute any real distinguishing
        // signal at all" -- used below to decide whether an unresolved
        // residual's search state is worth retaining (see `ByCitePlan::min_names_to_show`).
        let single_bucket = buckets.len() <= 1;

        let mut plans = Vec::new();
        let mut unresolved = Vec::new();

        for (start_n, bucket) in buckets {
            if bucket.len() <= 1 {
                if let [reference] = bucket.as_slice() {
                    plans.push(ByCitePlan {
                        reference,
                        min_names_to_show: Some(start_n),
                        expand_given_names: false,
                        full_positions: Vec::new(),
                    });
                }
                continue;
            }
            if !flags.add_givenname {
                let min_names_to_show = if single_bucket { None } else { Some(start_n) };
                unresolved.extend(bucket.into_iter().map(|reference| ByCitePlan {
                    reference,
                    min_names_to_show,
                    expand_given_names: false,
                    full_positions: Vec::new(),
                }));
                continue;
            }
            let full_positions = vec![false; start_n];
            let plans_before = plans.len();
            let unresolved_before = unresolved.len();
            self.resolve_by_cite_positions(
                &bucket,
                start_n,
                0,
                &full_positions,
                max_authors,
                flags,
                &mut plans,
                &mut unresolved,
            );
            if single_bucket && plans.len() == plans_before {
                // Nothing distinguished any member of this bucket from any
                // other, at any name count or escalation depth tried -- a
                // genuinely unresolvable collision (e.g. the exact same
                // person cited for two different works). Retaining the
                // failed search's state would be a pure no-op for
                // disambiguation, yet would still render a gratuitous
                // given-name reveal and defeat
                // `grouping.rs::group_citation_items_by_author`'s
                // same-author collapse (which keys off these fields being
                // empty). Reset to the pre-search default.
                let reset_from = unresolved.get_mut(unresolved_before..).unwrap_or(&mut []);
                for plan in reset_from {
                    plan.min_names_to_show = None;
                    plan.expand_given_names = false;
                    plan.full_positions = Vec::new();
                }
            }
        }

        (plans, unresolved)
    }

    /// Maps a position's `by-cite` escalation bit to the level that
    /// actually renders there once its shown position is revealed at all:
    /// `true` always means `Full`; `false` means the default depth for a
    /// revealed position -- `Initials` when `initials_available` (real
    /// CSL's `initialize-with`-presence gate), otherwise `Full` directly
    /// (mirrors `resolve_givenname_level`'s own gating, so a style with no
    /// initials rung never gets offered one).
    fn positional_given_level(&self, full: bool) -> GivennameLevel {
        if full || !self.initials_available() {
            GivennameLevel::Full
        } else {
            GivennameLevel::Initials
        }
    }

    /// Recursively resolves a `by-cite` collision `bucket` (all members
    /// already tied at `n` shown names, positions `0..pos` already
    /// committed in `full_positions`) by escalating individual positions to
    /// the full given name, splitting the bucket at the first position
    /// whose escalated value distinguishes at least some members, and
    /// growing `n` (revealing one more author) once every position `0..n`
    /// has been tried without effect. Members still colliding once `n`
    /// reaches `max_authors` with no position left to try are pushed to
    /// `unresolved`.
    #[allow(clippy::too_many_arguments, reason = "recursive search state")]
    fn resolve_by_cite_positions<'b>(
        &self,
        bucket: &[&'b CachedReference<'b>],
        n: usize,
        pos: usize,
        full_positions: &[bool],
        max_authors: usize,
        flags: DisambiguationFlags,
        plans: &mut Vec<ByCitePlan<'b>>,
        unresolved: &mut Vec<ByCitePlan<'b>>,
    ) {
        if bucket.len() <= 1 {
            if let [reference] = bucket {
                plans.push(ByCitePlan {
                    reference,
                    min_names_to_show: Some(n),
                    expand_given_names: true,
                    full_positions: full_positions.to_vec(),
                });
            }
            return;
        }

        // The bucket may already be split by information already reflected
        // in `full_positions` at the current `n` -- a freshly revealed
        // position's own default-depth difference, or an earlier commit
        // interacting with a position not yet explicitly escalated. Check
        // before escalating further.
        let current =
            self.bucket_by_positional_key(bucket, n, flags.primary_givenname_only, full_positions);
        if current.len() > 1 {
            for sub in current.into_values() {
                self.resolve_by_cite_positions(
                    &sub,
                    n,
                    pos,
                    full_positions,
                    max_authors,
                    flags,
                    plans,
                    unresolved,
                );
            }
            return;
        }

        let max_pos = if flags.primary_givenname_only { 1 } else { n };
        if pos >= max_pos {
            // Revealing another author (growing `n`) is strategy 1
            // (`disambiguate-add-names`), which `by-cite` has no authority
            // over (§2.1.1) -- only attempt it when that strategy is
            // actually enabled. Otherwise stop here: escalating given
            // names within the names already shown is all `by-cite` is
            // allowed to do.
            if !flags.add_names || n >= max_authors {
                unresolved.extend(bucket.iter().map(|reference| ByCitePlan {
                    reference,
                    min_names_to_show: Some(n),
                    expand_given_names: true,
                    full_positions: full_positions.to_vec(),
                }));
            } else {
                let mut grown = full_positions.to_vec();
                grown.push(false);
                self.resolve_by_cite_positions(
                    bucket,
                    n + 1,
                    n,
                    &grown,
                    max_authors,
                    flags,
                    plans,
                    unresolved,
                );
            }
            return;
        }

        if flags.givenname_full_allowed {
            let mut trial = full_positions.to_vec();
            #[allow(
                clippy::indexing_slicing,
                reason = "pos < max_pos <= n == trial.len(), checked above"
            )]
            {
                trial[pos] = true;
            }
            let buckets =
                self.bucket_by_positional_key(bucket, n, flags.primary_givenname_only, &trial);
            if buckets.len() > 1 {
                for sub in buckets.into_values() {
                    self.resolve_by_cite_positions(
                        &sub,
                        n,
                        pos + 1,
                        &trial,
                        max_authors,
                        flags,
                        plans,
                        unresolved,
                    );
                }
                return;
            }
        }

        // Escalating this position to full doesn't distinguish any members
        // (or full escalation isn't allowed at all -- `all-names-with-
        // initials`/`primary-name-with-initials` never reach `by-cite`, but
        // stay defensive); leave it at default depth and move on.
        self.resolve_by_cite_positions(
            bucket,
            n,
            pos + 1,
            full_positions,
            max_authors,
            flags,
            plans,
            unresolved,
        );
    }

    /// Buckets `group` by the positional collision key (`n` names shown,
    /// `full_positions` giving each position's escalation state).
    fn bucket_by_positional_key<'b>(
        &self,
        group: &[&'b CachedReference<'b>],
        n: usize,
        primary_only: bool,
        full_positions: &[bool],
    ) -> HashMap<String, Vec<&'b CachedReference<'b>>> {
        let mut buckets: HashMap<String, Vec<&'b CachedReference<'b>>> = HashMap::new();
        let mut buf = String::new();
        for reference in group {
            buf.clear();
            self.append_givenname_resolution_key_positional(
                &mut buf,
                &reference.data.names,
                n,
                primary_only,
                full_positions,
            );
            buckets.entry(buf.clone()).or_default().push(*reference);
        }
        buckets
    }

    /// Derives the `(expand_given_names_full, expand_given_names_full_positions)`
    /// pair a `ByCitePlan` renders as. `expand_given_names_full_positions` is
    /// `None` in exactly three cases: no given-name work was needed at all
    /// (`!plan.expand_given_names`); initials aren't reachable at all
    /// (`!initials_available()`), so every escalated position renders as the
    /// full given name uniformly and there's no per-position depth choice to
    /// carry (see `positional_given_level`); or, trivially, an empty
    /// `full_positions` (the name-count-alone resolution path). A genuine
    /// per-position search whose result happens to be uniform (all `true` or
    /// all `false`) still returns `Some(...)` -- the field's presence means
    /// "a real search happened here," not "positions disagree."
    fn by_cite_positional_fields(&self, plan: &ByCitePlan<'_>) -> (bool, Option<Vec<bool>>) {
        let uniform_full = plan.expand_given_names && !self.initials_available();
        let full_positions = if uniform_full || !plan.expand_given_names {
            None
        } else {
            Some(plan.full_positions.clone())
        };
        (uniform_full, full_positions)
    }

    /// Finalizes `select_by_cite_resolution`'s output: inserts a hint per
    /// resolved plan, then falls through any still-colliding remainder to
    /// year-suffix via `apply_by_cite_unresolved_fallback`, which retains
    /// each plan's last-attempted name count and positional escalation
    /// instead of resetting it.
    fn apply_by_cite_plans(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        context: &GroupDisambiguationContext<'_>,
        plans: Vec<ByCitePlan<'_>>,
        unresolved: Vec<ByCitePlan<'_>>,
    ) {
        for (idx, plan) in plans.into_iter().enumerate() {
            let (uniform_full, full_positions) = self.by_cite_positional_fields(&plan);
            self.insert_hint(
                hints,
                plan.reference,
                context.author_group_lengths,
                ProcHints {
                    disamb_condition: false,
                    group_index: idx + 1,
                    group_key: context.key.to_string(),
                    expand_given_names: plan.expand_given_names,
                    expand_given_names_full: uniform_full,
                    expand_given_names_primary_only: context.flags.primary_givenname_only,
                    expand_given_names_full_positions: full_positions,
                    min_names_to_show: plan.min_names_to_show,
                    ..Default::default()
                },
            );
        }

        self.apply_by_cite_unresolved_fallback(hints, context, unresolved);
    }

    /// Falls unresolved `by-cite` plans through to year-suffix, retaining
    /// each plan's last-attempted `min_names_to_show`/positional escalation
    /// rather than resetting it to the style's default (unlike a blanket
    /// `apply_year_suffix_for_group` call, which has no per-plan state to
    /// carry). Order follows the same resolved bibliography sort every
    /// other year-suffix assignment uses (`sort_group_for_year_suffix`).
    fn apply_by_cite_unresolved_fallback(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        context: &GroupDisambiguationContext<'_>,
        unresolved: Vec<ByCitePlan<'_>>,
    ) {
        if unresolved.is_empty() {
            return;
        }

        let refs: Vec<&CachedReference<'_>> =
            unresolved.iter().map(|plan| plan.reference).collect();
        let sorted = self.sort_group_for_year_suffix(&refs);
        let plan_by_id: HashMap<String, &ByCitePlan<'_>> = unresolved
            .iter()
            .map(|plan| {
                (
                    plan.reference
                        .reference
                        .id()
                        .unwrap_or_default()
                        .to_string(),
                    plan,
                )
            })
            .collect();

        for (idx, reference) in sorted.into_iter().enumerate() {
            let id = reference.reference.id().unwrap_or_default().to_string();
            let Some(plan) = plan_by_id.get(&id) else {
                continue;
            };
            let (uniform_full, full_positions) = self.by_cite_positional_fields(plan);
            self.insert_hint(
                hints,
                reference,
                context.author_group_lengths,
                ProcHints {
                    disamb_condition: true,
                    group_index: idx + 1,
                    group_key: context.key.to_string(),
                    expand_given_names: plan.expand_given_names,
                    expand_given_names_full: uniform_full,
                    expand_given_names_primary_only: context.flags.primary_givenname_only,
                    expand_given_names_full_positions: full_positions,
                    min_names_to_show: plan.min_names_to_show,
                    ..Default::default()
                },
            );
        }
    }

    /// Selects collision resolution by using both more names AND given name expansion.
    ///
    /// When `primary_givenname_only` is active, the renderer only shows given names for
    /// the first author. `find_combined_resolution` finds the minimum name count and
    /// escalation level that would work in theory; this function then verifies whether
    /// that resolution also holds under the restricted primary-only rendering.
    fn select_combined_resolution(
        &self,
        context: &GroupDisambiguationContext<'_>,
    ) -> Option<(usize, GivennameLevel, bool)> {
        if !context.flags.add_names || !context.flags.add_givenname {
            return None;
        }

        let (min_names_to_show, level) = self.find_combined_resolution(context)?;
        let primary_only_requires_suffix = context.flags.primary_givenname_only
            && !self.check_givenname_resolution(
                context.group,
                Some(min_names_to_show),
                true,
                level,
            );

        Some((min_names_to_show, level, primary_only_requires_suffix))
    }

    /// Applies a name-expansion partition plan, suffixing any unresolved subgroups.
    fn apply_name_partitions(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        context: &GroupDisambiguationContext<'_>,
        min_names_to_show: usize,
        partitions: &HashMap<String, Vec<&CachedReference<'_>>>,
    ) {
        for subgroup in partitions.values() {
            if subgroup.len() == 1 {
                self.apply_resolution(
                    hints,
                    subgroup,
                    context,
                    false,
                    false,
                    Some(min_names_to_show),
                );
                continue;
            }

            let resolution = context
                .flags
                .add_givenname
                .then(|| {
                    self.resolve_givenname_level(
                        subgroup,
                        Some(min_names_to_show),
                        false,
                        context.flags,
                    )
                })
                .flatten();

            if let Some(level) = resolution {
                let expand_full = level == GivennameLevel::Full;
                // Under primary-name rules, secondary given names are not rendered.
                // If the full-expansion check passes but primary-only does not, the
                // subgroup must fall back to year-suffix (with expansion retained).
                if context.flags.primary_givenname_only
                    && !self.check_givenname_resolution(
                        subgroup,
                        Some(min_names_to_show),
                        true,
                        level,
                    )
                {
                    self.apply_year_suffix_for_group(
                        hints,
                        subgroup,
                        context,
                        true,
                        expand_full,
                        Some(min_names_to_show),
                    );
                } else {
                    self.apply_resolution(
                        hints,
                        subgroup,
                        context,
                        true,
                        expand_full,
                        Some(min_names_to_show),
                    );
                }
                continue;
            }

            self.apply_year_suffix_for_group(
                hints,
                subgroup,
                context,
                false,
                false,
                Some(min_names_to_show),
            );
        }
    }

    /// Searches for the minimum number of names that, when combined with given name expansion,
    /// resolves the collision group. Returns the name count together with the escalation
    /// level (initials or full) that resolves it, preferring the least-disruptive level at
    /// each candidate count.
    fn find_combined_resolution(
        &self,
        context: &GroupDisambiguationContext<'_>,
    ) -> Option<(usize, GivennameLevel)> {
        let group = context.group;
        let max_authors = group
            .iter()
            .map(|reference| reference.data.names.len())
            .max()
            .unwrap_or(0);

        // The caller is responsible for verifying the result under primary-only rendering
        // when primary_givenname_only is active.
        (2..=max_authors).find_map(|n| {
            self.resolve_givenname_level(group, Some(n), false, context.flags)
                .map(|level| (n, level))
        })
    }

    /// Tries the given-name escalation ladder (initials, then full given name) and returns
    /// the least-disruptive level that resolves the collision, if any.
    ///
    /// Initials are only tried when `initials_available` — mirroring real CSL's
    /// `initialize-with`-presence gate. Full-name escalation is only tried when
    /// `flags.givenname_full_allowed` — `all-names-with-initials` and
    /// `primary-name-with-initials` must stop at initials even if that doesn't
    /// resolve the collision (csl26-h9jy).
    fn resolve_givenname_level(
        &self,
        group: &[&CachedReference<'_>],
        min_names: Option<usize>,
        primary_only: bool,
        flags: DisambiguationFlags,
    ) -> Option<GivennameLevel> {
        if self.initials_available()
            && self.check_givenname_resolution(
                group,
                min_names,
                primary_only,
                GivennameLevel::Initials,
            )
        {
            return Some(GivennameLevel::Initials);
        }
        if flags.givenname_full_allowed
            && self.check_givenname_resolution(group, min_names, primary_only, GivennameLevel::Full)
        {
            return Some(GivennameLevel::Full);
        }
        None
    }

    /// Finalizes a successful disambiguation strategy by inserting the calculated hints into the map.
    fn apply_resolution(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        group: &[&CachedReference<'_>],
        context: &GroupDisambiguationContext<'_>,
        expand_given_names: bool,
        expand_given_names_full: bool,
        min_names_to_show: Option<usize>,
    ) {
        self.insert_group_hints(
            hints,
            group,
            context.author_group_lengths,
            HintPlan {
                key: context.key,
                expand_given_names,
                expand_given_names_full,
                expand_given_names_primary_only: context.flags.primary_givenname_only,
                min_names_to_show,
                disamb_condition: false,
            },
            HintOrder::Encountered,
        );
    }

    /// Inserts a single hint into the hints map, ensuring the author group length is correctly set.
    fn insert_hint(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        reference: &CachedReference<'_>,
        author_group_lengths: &HashMap<String, usize>,
        mut hint: ProcHints,
    ) {
        hint.group_length = self
            .author_group_length(reference, author_group_lengths)
            .unwrap_or(1);
        hints.insert(
            reference.reference.id().unwrap_or_default().to_string(),
            hint,
        );
    }

    /// Retrieves the number of references sharing the author key for a specific reference.
    fn author_group_length(
        &self,
        reference: &CachedReference<'_>,
        author_group_lengths: &HashMap<String, usize>,
    ) -> Option<usize> {
        let author_key = &reference.data.author_key;
        author_group_lengths.get(author_key).copied()
    }

    /// Applies year-suffix disambiguation to the entire group in the context.
    fn apply_year_suffix(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        context: &GroupDisambiguationContext<'_>,
        expand_given_names: bool,
        expand_given_names_full: bool,
        min_names_to_show: Option<usize>,
    ) {
        self.apply_year_suffix_for_group(
            hints,
            context.group,
            context,
            expand_given_names,
            expand_given_names_full,
            min_names_to_show,
        );
    }

    /// Applies year-suffix disambiguation to a specific (sub)group of references.
    fn apply_year_suffix_for_group(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        group: &[&CachedReference<'_>],
        context: &GroupDisambiguationContext<'_>,
        expand_given_names: bool,
        expand_given_names_full: bool,
        min_names_to_show: Option<usize>,
    ) {
        self.insert_group_hints(
            hints,
            group,
            context.author_group_lengths,
            HintPlan {
                key: context.key,
                expand_given_names,
                expand_given_names_full,
                expand_given_names_primary_only: context.flags.primary_givenname_only,
                min_names_to_show,
                disamb_condition: true,
            },
            HintOrder::GroupSorted,
        );
    }

    /// Iterates through a group of references and inserts hints according to the specified order.
    fn insert_group_hints(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        group: &[&CachedReference<'_>],
        author_group_lengths: &HashMap<String, usize>,
        plan: HintPlan<'_>,
        order: HintOrder,
    ) {
        match order {
            HintOrder::Encountered => {
                for (idx, reference) in group.iter().enumerate() {
                    self.insert_planned_hint(hints, reference, author_group_lengths, plan, idx + 1);
                }
            }
            HintOrder::GroupSorted => {
                for (idx, reference) in self.sort_group_for_year_suffix(group).iter().enumerate() {
                    self.insert_planned_hint(hints, reference, author_group_lengths, plan, idx + 1);
                }
            }
        }
    }

    /// Helper to insert a hint with common planned fields (key, expand flags, group index).
    fn insert_planned_hint(
        &self,
        hints: &mut HashMap<String, ProcHints>,
        reference: &CachedReference<'_>,
        author_group_lengths: &HashMap<String, usize>,
        plan: HintPlan<'_>,
        group_index: usize,
    ) {
        self.insert_hint(
            hints,
            reference,
            author_group_lengths,
            ProcHints {
                disamb_condition: plan.disamb_condition,
                group_index,
                group_key: plan.key.to_string(),
                expand_given_names: plan.expand_given_names,
                expand_given_names_full: plan.expand_given_names_full,
                expand_given_names_primary_only: plan.expand_given_names_primary_only,
                min_names_to_show: plan.min_names_to_show,
                ..Default::default()
            },
        );
    }

    /// Sorts a collision group to determine the deterministic order for year-suffix assignment.
    /// It uses the provided group sort specification or falls back to title-based sorting.
    fn sort_group_for_year_suffix<'b>(
        &self,
        group: &[&'b CachedReference<'b>],
    ) -> Vec<&'b CachedReference<'b>> {
        if let Some(sort_spec) = self.group_sort {
            let mut sorter =
                ReferenceSorter::with_bibliography_config(self.locale, self.sort_config);
            if let Some(spec) = self.bibliography_spec {
                sorter = sorter.with_bibliography_spec(spec);
            } else if let Some(spec) = self.citation_spec {
                sorter = sorter.with_citation_spec(spec);
            }
            // Pre-sort so entries that compare equal under the primary sort_spec keep a
            // stable, deterministic order — matching the renderer, not an independently
            // computed title order. `sort_by_keys` uses sort_by (stable), so the pre-sort
            // order survives for entries that tie under the primary key.
            //
            // The renderer (`ReferenceSorter::sort_references_impl`, sorting.rs) stable-
            // sorts the registry-ordered bibliography and, only when the resolved sort
            // opts into it, breaks ties by reference id afterward (`compare_cached_ids`).
            // An empty template is a renderer no-op — `sort_references_impl` early-
            // returns on `compiled_keys.is_empty()` — so neither the id nor the date
            // comparison below may run for it; registry order must survive untouched.
            // Only once the template is non-empty does the renderer's stable sort
            // actually engage those finer tiebreaks, so both steps are gated on that,
            // not just on `id_tiebreak`.
            //
            // No date comparison here: `sort_by_keys` below (shared with the real
            // bibliography renderer) already compares full issued dates — not just the
            // year — for a resolved `Issued` sort key
            // (`ReferenceSorter::compare_by_issued`/`issued_date_parts`, sorting.rs). A
            // same-year, no-title collision pair that used to tie under a year-only
            // Issued key (`chicago-author-date-18th`'s May/September Gourmet magazine
            // entries) now resolves through that one shared comparator instead of a
            // second, independently-maintained date comparison here — duplicating it
            // would only risk the two drifting apart again. Only registry `index` is
            // still needed as the final tiebreak, for entries that remain fully tied
            // after every key in the resolved template. Gated on a non-empty template:
            // an empty one is a renderer no-op (`sort_references_impl`'s early return),
            // so `sort_by_keys` never runs and `index` alone must decide order.
            let mut pre_sorted: Vec<&CachedReference<'_>> = group.to_vec();
            if sort_spec.template.is_empty() {
                pre_sorted.sort_by_key(|cached| cached.data.index);
            } else if self.id_tiebreak {
                pre_sorted.sort_by(|a, b| {
                    compare_none_last(
                        a.reference.id().map(|id| id.0),
                        b.reference.id().map(|id| id.0),
                    )
                    .then_with(|| a.data.index.cmp(&b.data.index))
                });
            } else {
                pre_sorted.sort_by_key(|cached| cached.data.index);
            }
            sorter.sort_by_keys(pre_sorted, &sort_spec.template, |cached| {
                Some(cached.reference)
            })
        } else {
            let mut sorted: Vec<&CachedReference<'_>> = group.to_vec();
            sorted.sort_by(|a, b| {
                let a_title = a.data.title_key.as_deref().unwrap_or_default();
                let b_title = b.data.title_key.as_deref().unwrap_or_default();
                a_title.cmp(b_title).then_with(|| {
                    year_suffix_date_key(a.reference).cmp(&year_suffix_date_key(b.reference))
                })
            });
            sorted
        }
    }

    /// Partition a collision group by showing more names, preserving `et al.`
    /// distinction when some references still have hidden trailing names.
    fn partition_by_name_expansion<'b>(
        &self,
        group: &[&'b CachedReference<'b>],
    ) -> Option<(usize, HashMap<String, Vec<&'b CachedReference<'b>>>)> {
        let max_authors = group
            .iter()
            .map(|reference| reference.data.names.len())
            .max()
            .unwrap_or(0);

        let mut buf = String::new();
        for n in 2..=max_authors {
            let mut partitions: HashMap<String, Vec<&'b CachedReference<'b>>> = HashMap::new();
            for reference in group {
                let names = &reference.data.names;
                buf.clear();
                self.append_name_expansion_key(&mut buf, names, n);
                if let Some(v) = partitions.get_mut(buf.as_str()) {
                    v.push(*reference);
                } else {
                    partitions.insert(buf.clone(), vec![*reference]);
                }
            }

            if partitions.len() > 1 {
                return Some((n, partitions));
            }
        }

        None
    }

    /// Check if expanding given names at the given escalation `level` resolves
    /// ambiguity in the group.
    ///
    /// If `min_names` is `Some(n)`, it checks resolution when showing `n` names.
    ///
    /// When `primary_only` is `true`, only the first author's given name is included
    /// in the resolution key — mirroring what `primary-name` and
    /// `primary-name-with-initials` actually render.  Use this to validate that a
    /// candidate expansion still works under restricted rendering before committing.
    fn check_givenname_resolution(
        &self,
        group: &[&CachedReference<'_>],
        min_names: Option<usize>,
        primary_only: bool,
        level: GivennameLevel,
    ) -> bool {
        let mut seen = HashSet::new();
        let mut buf = String::new();
        let n = min_names.unwrap_or(1);
        for reference in group {
            let names = &reference.data.names;
            buf.clear();
            self.append_givenname_resolution_key(&mut buf, names, n, primary_only, level);
            if !seen.insert(buf.clone()) {
                return false;
            }
        }
        true
    }

    /// Group references by their base collision key for disambiguation.
    fn group_references<'b>(
        &self,
        references: &'b ReferenceCache<'b>,
    ) -> HashMap<String, Vec<&'b CachedReference<'b>>> {
        let mut groups: HashMap<String, Vec<&'b CachedReference<'b>>> = HashMap::new();

        for reference in references {
            let key = reference.data.group_key.clone();
            groups.entry(key).or_default().push(reference);
        }

        groups
    }

    /// Generates a normalized author string used for grouping and et-al detection.
    fn build_author_key(&self, names: &[crate::reference::FlatName]) -> String {
        let shorten = self
            .config
            .contributors
            .as_ref()
            .and_then(|c| c.shorten.as_ref());

        if names.is_empty() {
            return String::new();
        }

        let mut key = String::new();
        if let Some(opts) = shorten
            && names.len() >= opts.min as usize
        {
            self.append_lowercased_families(&mut key, names, opts.use_first as usize, ',');
            if !key.is_empty() {
                key.push(',');
            }
            key.push_str("et-al");
            return key;
        }

        self.append_lowercased_families(&mut key, names, names.len(), ',');
        key
    }

    fn title_substitute_key(title: Title) -> String {
        let mut key = String::new();
        Self::push_lowercased(&mut key, title.to_string().trim());
        key
    }

    /// Create a grouping key for a reference based on its base citation form.
    fn build_group_key(&self, index: usize, reference: &Reference, author_key: &str) -> String {
        // In label mode, group by base label string rather than author-year.
        // This ensures disambiguation happens at the label level (Knu84a/Knu84b)
        // rather than the author-year level.
        if let Some(citum_schema::options::Processing::Label(config)) = &self.config.processing {
            let params = config.effective_params();
            return crate::processor::labels::generate_base_label(reference, &params);
        }

        // Anonymous entries (no author key) must not be grouped together for year-suffix
        // assignment. CSL year-suffix disambiguates entries with the same *author* —
        // anonymous entries are already distinguished by their title substitution.
        // Give each anonymous reference a unique key so it forms its own singleton group.
        if author_key.is_empty() {
            if let Some(ref_id) = reference.id().filter(|id| !id.is_empty()) {
                return format!("anon:{ref_id}");
            }
            return format!("anon:index:{index}");
        }

        let mut key = String::with_capacity(author_key.len() + 8);
        key.push_str(author_key);
        key.push(':');
        if let Some(year) = reference
            .effective_issued_date()
            .and_then(|d| d.year().parse::<i32>().ok())
        {
            let _ = write!(key, "{year}");
            return key;
        }

        // No issued year. Collision grouping must reflect what the style's
        // resolved date slot actually yields for this reference, not a
        // uniform "no date" assumption — a type-conditional date macro (e.g.
        // GB/T 7714's article-journal branch, which never reaches the
        // no-date term) renders different text for different reference
        // types, and those references are already visually distinguishable.
        // See csl26-huuz.
        key.push_str(&self.date_slot_discriminant(reference));
        key
    }

    /// Discriminant for the date half of the collision key when a reference
    /// has no issued year. Reads the reference's effective resolved
    /// template — the first effective, non-suppressed issued component under the author,
    /// or the first effective date when the template has no issued slot
    /// (`crate::sorting::first_date_component_for_bibliography`, preferring
    /// the bibliography spec when present, else the citation spec) — and
    /// returns text identifying what it actually renders:
    ///
    /// - the date variable resolves to a real, non-empty value → the text
    ///   it would actually render (`form`-restricted and marker-applied,
    ///   the same formatting `TemplateDate::values` uses — not the raw
    ///   stored value, which can carry more precision than `form` shows);
    /// - the variable is empty and the resolved fallback chain (explicit or
    ///   the implicit no-date-term branch) renders the locale's no-date
    ///   term → a discriminant for that term, scoped by the reference's
    ///   effective language (mirrors `build_author_slot_key`'s
    ///   `ANONYMOUS_FALLBACK_KEY` scoping — the rendered term itself varies
    ///   by language, "无日期" vs "n.d.");
    ///
    /// Access dates (`DateVariable::Accessed`), whether the slot's own
    /// primary variable or a fallback candidate, are never used as a
    /// discriminant even when present — an access date is retrieval
    /// metadata, not part of a work's identity, so two otherwise identical
    /// undated entries must not be distinguished by it.
    ///
    /// Bibliography-preferred, not citation-preferred: mirrors
    /// `sort_group_for_year_suffix`'s existing precedent (`csl26-m8la`) for
    /// the same reason — collision grouping is measured against the
    /// bibliography oracle, and a style's citation template is commonly a
    /// simpler, non-type-differentiated form of the same date logic (GB/T
    /// author-date's `citation:` section has one flat `date: issued` with no
    /// `type-variants:` at all, unlike its bibliography section). Preferring
    /// citation_spec here would let that undifferentiated template collapse
    /// every undated reference onto the same discriminant regardless of
    /// type, defeating the type-conditional split this function exists to
    /// make. Confirmed empirically: an in-tree literal `bibliography_spec`
    /// vs `citation_spec` preference swap was compared against the GB/T
    /// oracle before landing this order.
    ///
    /// Returns the empty string both when the effective date-fallback policy
    /// resolves blank and when there is no template to resolve —
    /// `build_group_key`'s existing undiscriminated key for that case.
    fn date_slot_discriminant(&self, reference: &Reference) -> String {
        let component_and_config = self
            .bibliography_spec
            .and_then(|spec| crate::sorting::first_date_component_for_bibliography(spec, reference))
            .map(|component| (component, self.sort_config))
            .or_else(|| {
                self.citation_spec
                    .and_then(|spec| {
                        crate::sorting::first_date_component_for_citation(spec, reference)
                    })
                    .map(|component| (component, self.config))
            });
        let Some((component, config)) = component_and_config else {
            return String::new();
        };
        let locale = self.effective_locale_for_reference(reference);
        Self::date_component_discriminant(
            &component,
            reference,
            locale.as_ref(),
            config,
            &reference.ref_type(),
        )
    }

    /// The literal locale message ID the implicit no-date branch in
    /// `TemplateDate::values` (`values/date.rs`) evaluates, and the same ID
    /// every GB/T-style explicit `message: term.no-date` fallback names.
    /// Both render identical text, so both must produce the same
    /// discriminant here.
    /// Classify what a resolved date component renders for a reference with
    /// no issued year. See `date_slot_discriminant`'s doc comment for the
    /// cases.
    ///
    /// A resolving candidate's *rendered* text is used, not the raw stored
    /// date value — `DateValue`'s `Display` is the unformatted EDTF/literal
    /// string, which can carry more precision than the component's `form`
    /// shows (e.g. a day-precision `copyright` date under `form: year`
    /// renders as a bare year, but its raw value still has the day). Reading
    /// the raw value here could split a collision group whose members
    /// render identical date-slot text, defeating the discriminant's whole
    /// purpose. See csl26-huuz, flagged in PR review.
    fn date_component_discriminant(
        component: &citum_schema::template::TemplateDate,
        reference: &Reference,
        locale: &citum_schema::locale::Locale,
        config: &citum_schema::options::Config,
        ref_type: &str,
    ) -> String {
        use citum_schema::template::{DateVariable, TemplateComponent};

        let date_config = config.dates.as_ref();

        if matches!(component.date, DateVariable::Accessed) {
            if crate::values::date::resolve_date_variable(&component.date, reference)
                .is_some_and(|value| !value.is_empty())
            {
                // An access date carries no identity even when present —
                // this is the terminal case for this slot, not a signal to
                // keep looking at a fallback chain.
                return String::new();
            }
        } else if let Some(value) =
            crate::values::date::resolve_date_variable(&component.date, reference)
                .filter(|value| !value.is_empty())
        {
            // The primary variable is present, so — mirroring
            // `TemplateDate::values`'s own normal-value branch, which never
            // consults `self.fallback` once the primary date resolves —
            // this is terminal even when the value fails to format (e.g. a
            // literal date under a numeric `form`, which the renderer also
            // shows as nothing). Reuses the same fallback-candidate helper as
            // the loop below: its visible rendering configuration is inert
            // here (constant across every reference resolving this same
            // component, so it can never split two references from each
            // other), but `date.note` is per-reference *data*, not
            // per-component config, so including it is defensive even though
            // a note-bearing value can't reach this branch under the current
            // data model (a note-bearing date is structured and parseable,
            // so `build_group_key`'s own year check already returns before
            // this function ever runs).
            return crate::values::date::fallback_candidate_discriminant(
                &value,
                &component.form,
                &component.rendering,
                component.suppress_note,
                locale,
                date_config,
            )
            .unwrap_or_default();
        }

        if !matches!(component.date, DateVariable::Issued) {
            return String::new();
        }
        let Some(fallbacks) =
            crate::values::date::effective_date_fallback_candidates(config, true, ref_type)
        else {
            return String::new();
        };

        for candidate in fallbacks.iter() {
            match candidate.to_template_component() {
                TemplateComponent::Message(message) => {
                    let Some(discriminant) = crate::values::date::fallback_message_discriminant(
                        &message, locale, config,
                    ) else {
                        // Rendering skips unresolved or suppressed messages and
                        // continues to the next fallback candidate.
                        continue;
                    };
                    return discriminant;
                }
                TemplateComponent::Date(inner) => {
                    let Some(value) =
                        crate::values::date::resolve_date_variable(&inner.date, reference)
                            .filter(|value| !value.is_empty())
                    else {
                        // This candidate doesn't resolve — not selected,
                        // keep scanning the chain (e.g. GB/T's accessed
                        // fallback when no accessed date exists at all).
                        continue;
                    };
                    let Some(discriminant) = crate::values::date::fallback_candidate_discriminant(
                        &value,
                        &inner.form,
                        &inner.rendering,
                        inner.suppress_note,
                        locale,
                        date_config,
                    ) else {
                        // Present but renders nothing (e.g. a literal date
                        // under a numeric form) — `render_date_fallback_chain`
                        // treats an unrendering candidate the same as an
                        // absent one and keeps scanning, so the discriminant
                        // must too.
                        continue;
                    };
                    // A resolving, rendering candidate is the terminal case
                    // for this slot: citeproc-js's underlying if/else-if
                    // branching (which this fallback chain mirrors) never
                    // falls through to a later branch once one is selected —
                    // even when, as with an access date, that branch's own
                    // content carries no identity and this discriminant is
                    // therefore empty rather than the date's text.
                    return if matches!(inner.date, DateVariable::Accessed) {
                        String::new()
                    } else {
                        discriminant
                    };
                }
                _ => {}
            }
        }

        String::new()
    }

    /// Appends a sequence of family names to the key buffer, lowercased.
    fn append_lowercased_families(
        &self,
        key: &mut String,
        names: &[crate::reference::FlatName],
        take: usize,
        separator: char,
    ) {
        for (idx, name) in names.iter().take(take).enumerate() {
            if idx > 0 {
                key.push(separator);
            }
            Self::push_lowercased(key, name.family_or_literal());
        }
    }

    /// Creates a key representing the citation form when n names are shown.
    fn append_name_expansion_key(
        &self,
        key: &mut String,
        names: &[crate::reference::FlatName],
        n: usize,
    ) {
        self.append_lowercased_families(key, names, n, '|');
        if names.len() > n {
            if !key.is_empty() {
                key.push('|');
            }
            key.push_str("et-al");
        }
    }

    /// Creates a key including full name parts (given names, particles) for exact resolution.
    ///
    /// When `primary_only` is `true`, only the first author (index 0) receives full
    /// given-name/particle parts; subsequent authors contribute only their family name.
    /// This mirrors what `primary-name` and `primary-name-with-initials` actually render,
    /// allowing resolution checks to validate against the real rendered surface form.
    fn append_givenname_resolution_key(
        &self,
        key: &mut String,
        names: &[crate::reference::FlatName],
        n: usize,
        primary_only: bool,
        level: GivennameLevel,
    ) {
        let initialize_with_hyphen = self
            .config
            .contributors
            .as_ref()
            .and_then(|c| c.initialize_with_hyphen);
        for (idx, name) in names.iter().take(n).enumerate() {
            if idx > 0 {
                key.push_str("||");
            }
            Self::append_optional_part(key, name.family.as_deref());
            if primary_only && idx > 0 {
                // Secondary authors: family name only under primary-name rules.
                continue;
            }
            key.push('|');
            match level {
                GivennameLevel::Full => {
                    Self::append_optional_part(key, name.given.as_deref());
                }
                GivennameLevel::Initials => {
                    let initials = name.given.as_deref().map(|given| {
                        crate::values::contributor::names::initialize_given_name(
                            given,
                            self.initialize_with(),
                            initialize_with_hyphen,
                        )
                    });
                    Self::append_optional_part(key, initials.as_deref());
                }
            }
            key.push('|');
            Self::append_optional_part(key, name.non_dropping_particle.as_deref());
            key.push('|');
            Self::append_optional_part(key, name.dropping_particle.as_deref());
        }
    }

    /// `by-cite` positional variant of `append_givenname_resolution_key`:
    /// instead of one uniform `level` applied to every shown position, each
    /// position gets its own `bool` from `full_positions` (`true` = full
    /// given name, `false` = the default depth for a revealed position),
    /// resolved to a `GivennameLevel` via `positional_given_level`.
    fn append_givenname_resolution_key_positional(
        &self,
        key: &mut String,
        names: &[crate::reference::FlatName],
        n: usize,
        primary_only: bool,
        full_positions: &[bool],
    ) {
        let initialize_with_hyphen = self
            .config
            .contributors
            .as_ref()
            .and_then(|c| c.initialize_with_hyphen);
        for (idx, name) in names.iter().take(n).enumerate() {
            if idx > 0 {
                key.push_str("||");
            }
            Self::append_optional_part(key, name.family.as_deref());
            if primary_only && idx > 0 {
                continue;
            }
            key.push('|');
            let full = full_positions.get(idx).copied().unwrap_or(false);
            match self.positional_given_level(full) {
                GivennameLevel::Full => {
                    Self::append_optional_part(key, name.given.as_deref());
                }
                GivennameLevel::Initials => {
                    let initials = name.given.as_deref().map(|given| {
                        crate::values::contributor::names::initialize_given_name(
                            given,
                            self.initialize_with(),
                            initialize_with_hyphen,
                        )
                    });
                    Self::append_optional_part(key, initials.as_deref());
                }
            }
            key.push('|');
            Self::append_optional_part(key, name.non_dropping_particle.as_deref());
            key.push('|');
            Self::append_optional_part(key, name.dropping_particle.as_deref());
        }
    }

    /// Serializes an optional name part into the key buffer with its length.
    fn append_optional_part(key: &mut String, value: Option<&str>) {
        match value {
            Some(value) => {
                let _ = write!(key, "{}:", value.len());
                key.push_str(value);
            }
            None => key.push('-'),
        }
    }

    /// Pushes a lowercased version of the string to the buffer, optimized for ASCII.
    fn push_lowercased(key: &mut String, value: &str) {
        if value.is_ascii() {
            key.reserve(value.len());
            for byte in value.bytes() {
                key.push((byte as char).to_ascii_lowercase());
            }
        } else {
            key.push_str(&value.to_lowercase());
        }
    }

    /// Returns the stable per-run cache key used for disambiguation metadata.
    fn reference_cache_key(index: usize, reference: &Reference) -> ReferenceCacheKey {
        reference
            .id()
            .map_or(ReferenceCacheKey::Index(index), |id| {
                ReferenceCacheKey::Id(id.to_string())
            })
    }
}

fn year_suffix_date_key(reference: &Reference) -> String {
    reference
        .effective_issued_date()
        .map(|date| date.to_string())
        .unwrap_or_default()
}

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
mod tests {
    use super::*;
    use crate::Processor;
    use citum_schema::citation::Citation;
    use citum_schema::grouping::{GroupSort, GroupSortKey, SortKey};
    use citum_schema::options::dates::DateConfig;
    use citum_schema::options::{
        Config, ContributorConfig, DisplayAsSort, MultilingualConfig, NameForm, SortingConfig,
        SortingMultilingualMode,
    };
    use citum_schema::reference::types::MultilingualComplex;
    use citum_schema::reference::{
        Contributor, DateValue, InputReference as Reference, Monograph, MonographType,
        MultilingualString, StructuredName, Title,
    };
    use citum_schema::template::{
        DateForm, DateVariable, Rendering, TemplateComponent, TemplateDate, WrapConfig,
        WrapPunctuation,
    };
    use citum_schema::{BibliographySpec, CitationSpec, Style, StyleInfo};
    use rstest::rstest;

    fn make_ref(id: &str, family: &str, given: &str, year: i32) -> Reference {
        let title = format!("Title {id}");
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(title.clone())),
            short_title: None,
            container: None,
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            editor: None,
            translator: None,
            issued: DateValue::new(year.to_string()),
            ..Default::default()
        }))
    }

    /// Like `make_ref`, but with an independently chosen title — `make_ref`'s
    /// title is derived from `id`, which makes id order and title order
    /// coincide and so cannot distinguish which one a sort actually used.
    fn make_ref_with_title(
        id: &str,
        family: &str,
        given: &str,
        year: i32,
        title: &str,
    ) -> Reference {
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(title.to_string())),
            short_title: None,
            container: None,
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            editor: None,
            translator: None,
            issued: DateValue::new(year.to_string()),
            ..Default::default()
        }))
    }

    fn make_ref_without_id(title_suffix: &str, family: &str, given: &str, year: i32) -> Reference {
        let title = format!("Title {title_suffix}");
        Reference::Monograph(Box::new(Monograph {
            id: None,
            r#type: MonographType::Book,
            title: Some(Title::Single(title)),
            short_title: None,
            container: None,
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(given.to_string()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            editor: None,
            translator: None,
            issued: DateValue::new(year.to_string()),
            ..Default::default()
        }))
    }

    fn make_multi_author_ref(id: &str, authors: &[(&str, &str)], year: i32) -> Reference {
        let title = format!("Title {id}");
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(title)),
            short_title: None,
            container: None,
            author: Some(Contributor::ContributorList(
                citum_schema::reference::ContributorList(
                    authors
                        .iter()
                        .map(|(family, given)| {
                            Contributor::StructuredName(StructuredName {
                                family: MultilingualString::Simple((*family).to_string()),
                                given: MultilingualString::Simple((*given).to_string()),
                                suffix: None,
                                dropping_particle: None,
                                non_dropping_particle: None,
                            })
                        })
                        .collect(),
                ),
            )),
            editor: None,
            translator: None,
            issued: DateValue::new(year.to_string()),
            ..Default::default()
        }))
    }

    fn make_author_date_style(config: Config, bibliography_sort: Option<GroupSort>) -> Style {
        Style {
            info: StyleInfo {
                title: Some("Disambiguation Test".to_string()),
                id: Some("disambiguation-test".into()),
                ..Default::default()
            },
            options: Some(config),
            citation: Some(CitationSpec {
                template: Some(
                    vec![
                        citum_schema::tc_contributor!(Author, Short),
                        citum_schema::tc_date!(Issued, Year, prefix = ", "),
                    ]
                    .into(),
                ),
                wrap: Some(WrapPunctuation::Parentheses.into()),
                ..Default::default()
            }),
            bibliography: Some(BibliographySpec {
                sort: bibliography_sort.map(citum_schema::grouping::GroupSortEntry::Explicit),
                template: Some(
                    vec![TemplateComponent::Title(
                        citum_schema::template::TemplateTitle {
                            title: citum_schema::template::TitleType::Primary,
                            ..Default::default()
                        },
                    )]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_group_aware_year_suffix_sort() {
        use citum_schema::options::{Disambiguation, Processing, ProcessingCustom};

        let r1 = make_ref("r1", "Smith", "Same", 2020);
        let r2 = make_ref("r2", "Smith", "Same", 2020);

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), r1);
        bib.insert("r2".to_string(), r2);

        let config = Config::default();
        let locale = Locale::en_us();

        // 1. Default sorting (by title): r1 should be 'a', r2 should be 'b'.
        // Title r1 < Title r2 alphabetically, so r1 gets group_index 1.
        let disamb_default = Disambiguator::new(&bib, &config, &config, &locale);
        let hints_default = disamb_default.calculate_hints();

        assert_eq!(hints_default.get("r1").unwrap().group_index, 1);
        assert_eq!(hints_default.get("r2").unwrap().group_index, 2);

        // 2. Custom group sort: Sort by title descending -> r2 should be 'a', r1 should be 'b'
        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: SortKey::Title,
                ascending: false,
                order: None,
                sort_order: None,
            }],
        };

        let disamb_custom =
            Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec);
        let hints_custom = disamb_custom.calculate_hints();

        assert_eq!(hints_custom.get("r2").unwrap().group_index, 1);
        assert_eq!(hints_custom.get("r1").unwrap().group_index, 2);

        let style = make_author_date_style(
            Config {
                processing: Some(Processing::Custom(ProcessingCustom {
                    base: None,
                    disambiguate: Some(Disambiguation {
                        names: false,
                        add_givenname: false,
                        givenname_rule: GivennameRule::default(),
                        year_suffix: true,
                    }),
                    ..Default::default()
                })),
                contributors: Some(ContributorConfig {
                    display_as_sort: Some(DisplayAsSort::First),
                    ..Default::default()
                }),
                ..Default::default()
            },
            Some(sort_spec),
        );
        let processor = Processor::new(style, bib);

        let rendered_r1 = processor.process_citation(&Citation::simple("r1")).unwrap();
        let rendered_r2 = processor.process_citation(&Citation::simple("r2")).unwrap();

        assert!(
            rendered_r1.contains("2020b"),
            "expected r1 to sort second: {rendered_r1}"
        );
        assert!(
            rendered_r2.contains("2020a"),
            "expected r2 to sort first: {rendered_r2}"
        );
    }

    /// Two same-author/same-year references whose registration order
    /// disagrees with both title order ("Alpha Report" < "Beta Report") and id order
    /// ("a-ref" < "b-ref"), so each ordering is independently distinguishable
    /// in the tests below. csl26-m8la.
    fn build_diverging_order_group() -> Bibliography {
        let mut bib = Bibliography::new();
        bib.insert(
            "b-ref".to_string(),
            make_ref_with_title("b-ref", "Smith", "Same", 2020, "Beta Report"),
        );
        bib.insert(
            "a-ref".to_string(),
            make_ref_with_title("a-ref", "Smith", "Same", 2020, "Alpha Report"),
        );
        bib
    }

    #[test]
    fn test_empty_group_sort_template_follows_registration_order() {
        let bib = build_diverging_order_group();
        let config = Config::default();
        let locale = Locale::en_us();
        let sort_spec = GroupSort { template: vec![] };

        let disamb = Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec);
        let hints = disamb.calculate_hints();

        // Registered first, not title-first ("Alpha Report" would win under the old
        // title-alphabetical pre-sort) or id-first ("a-ref" < "b-ref").
        assert_eq!(hints.get("b-ref").unwrap().group_index, 1);
        assert_eq!(hints.get("a-ref").unwrap().group_index, 2);
    }

    #[test]
    fn test_empty_group_sort_template_ignores_id_tiebreak() {
        let bib = build_diverging_order_group();
        let config = Config::default();
        let locale = Locale::en_us();
        let sort_spec = GroupSort { template: vec![] };

        // An empty template is a renderer no-op regardless of id_tiebreak
        // (ReferenceSorter::sort_references_impl's early return) — the flag
        // must not reorder by id here.
        let disamb = Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec)
            .with_id_tiebreak(true);
        let hints = disamb.calculate_hints();

        assert_eq!(hints.get("b-ref").unwrap().group_index, 1);
        assert_eq!(hints.get("a-ref").unwrap().group_index, 2);
    }

    #[test]
    fn test_non_empty_template_with_equal_keys_follows_registration_order_without_id_tiebreak() {
        let bib = build_diverging_order_group();
        let config = Config::default();
        let locale = Locale::en_us();
        // Author and issued are equal across the group, so this template
        // doesn't itself resolve the tie.
        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: SortKey::Author,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: SortKey::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
        };

        let disamb = Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec);
        let hints = disamb.calculate_hints();

        assert_eq!(hints.get("b-ref").unwrap().group_index, 1);
        assert_eq!(hints.get("a-ref").unwrap().group_index, 2);
    }

    #[test]
    fn test_non_empty_template_with_equal_keys_follows_id_order_with_id_tiebreak() {
        let bib = build_diverging_order_group();
        let config = Config::default();
        let locale = Locale::en_us();
        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: SortKey::Author,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: SortKey::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
        };

        let disamb = Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec)
            .with_id_tiebreak(true);
        let hints = disamb.calculate_hints();

        // "a-ref" < "b-ref" — id order, not the "b-ref" first registration order.
        assert_eq!(hints.get("a-ref").unwrap().group_index, 1);
        assert_eq!(hints.get("b-ref").unwrap().group_index, 2);
    }

    #[test]
    fn test_id_tiebreak_sorts_missing_id_last() {
        let mut bib = Bibliography::new();
        // Registered first but has no id: must still sort *after* the
        // reference that does, mirroring ReferenceSorter::compare_cached_ids.
        bib.insert(
            "missing".to_string(),
            make_ref_without_id("missing", "Smith", "Same", 2020),
        );
        bib.insert(
            "with-id".to_string(),
            make_ref("with-id", "Smith", "Same", 2020),
        );

        let config = Config::default();
        let locale = Locale::en_us();
        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: SortKey::Author,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: SortKey::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
        };

        let disamb = Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec)
            .with_id_tiebreak(true);
        let hints = disamb.calculate_hints();

        // Missing-id references key the hints map under the empty string
        // (`insert_hint`'s `id().unwrap_or_default()`).
        assert_eq!(hints.get("with-id").unwrap().group_index, 1);
        assert_eq!(hints.get("").unwrap().group_index, 2);
    }

    #[test]
    fn test_non_empty_template_title_key_still_governs_over_registration_and_id_order() {
        let bib = build_diverging_order_group();
        let config = Config::default();
        let locale = Locale::en_us();
        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: SortKey::Author,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: SortKey::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: SortKey::Title,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
        };

        let disamb = Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec)
            .with_id_tiebreak(true);
        let hints = disamb.calculate_hints();

        // A real Title sort key resolves the tie itself: "Alpha Report" < "Beta Report",
        // overriding both registration order ("b-ref" first) and id order
        // ("a-ref" < "b-ref" would coincidentally agree here, but the point is
        // this is driven by the template's own Title key, not our tiebreak).
        assert_eq!(hints.get("a-ref").unwrap().group_index, 1);
        assert_eq!(hints.get("b-ref").unwrap().group_index, 2);
    }

    #[test]
    fn test_author_date_default_uses_year_suffix_without_name_expansion() {
        use citum_schema::options::Processing;

        let r1 = make_ref("r1", "Smith", "John", 2020);
        let r2 = make_ref("r2", "Smith", "Alice", 2020);

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), r1);
        bib.insert("r2".to_string(), r2);

        let config = Config {
            processing: Some(Processing::AuthorDate),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let disamb = Disambiguator::new(&bib, &config, &config, &locale);
        let hints = disamb.calculate_hints();
        let r1_hints = hints.get("r1").unwrap();
        let r2_hints = hints.get("r2").unwrap();

        assert!(r1_hints.disamb_condition);
        assert!(r2_hints.disamb_condition);
        assert!(!r1_hints.expand_given_names);
        assert!(!r2_hints.expand_given_names);
        assert_eq!(r1_hints.min_names_to_show, None);
        assert_eq!(r2_hints.min_names_to_show, None);

        let style = make_author_date_style(config, None);
        let processor = Processor::new(style, bib);

        let rendered_r1 = processor.process_citation(&Citation::simple("r1")).unwrap();
        let rendered_r2 = processor.process_citation(&Citation::simple("r2")).unwrap();

        assert!(
            rendered_r1.contains("2020a") || rendered_r1.contains("2020b"),
            "expected r1 to receive a year suffix: {rendered_r1}"
        );
        assert!(
            rendered_r2.contains("2020a") || rendered_r2.contains("2020b"),
            "expected r2 to receive a year suffix: {rendered_r2}"
        );
        assert!(
            !rendered_r1.contains("John") && !rendered_r1.contains("J."),
            "expected r1 to avoid given-name expansion: {rendered_r1}"
        );
        assert!(
            !rendered_r2.contains("Alice") && !rendered_r2.contains("A."),
            "expected r2 to avoid given-name expansion: {rendered_r2}"
        );
    }

    #[test]
    fn test_disambiguate_given_names() {
        use citum_schema::options::{Disambiguation, Processing, ProcessingCustom};

        // Use different given names to test if expansion resolves the collision
        let r1 = make_ref("r1", "Smith", "John", 2020);
        let r2 = make_ref("r2", "Smith", "Alice", 2020);

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), r1);
        bib.insert("r2".to_string(), r2);

        let config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: false,
                    add_givenname: true,
                    givenname_rule: GivennameRule::AllNames,
                    year_suffix: false,
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let disamb = Disambiguator::new(&bib, &config, &config, &locale);
        let hints = disamb.calculate_hints();

        // Both should have expand_given_names set to true to resolve the Smith (2020) collision
        assert!(hints.get("r1").unwrap().expand_given_names);
        assert!(hints.get("r2").unwrap().expand_given_names);

        // Should NOT have year suffix since it's disabled in config (and given names resolve it)
        assert!(!hints.get("r1").unwrap().disamb_condition);
        assert!(!hints.get("r2").unwrap().disamb_condition);

        // Collision resolved: entries occupy distinct positions
        assert_ne!(
            hints.get("r1").unwrap().group_index,
            hints.get("r2").unwrap().group_index
        );

        let style = make_author_date_style(
            Config {
                processing: Some(Processing::Custom(ProcessingCustom {
                    base: None,
                    disambiguate: Some(Disambiguation {
                        names: false,
                        add_givenname: true,
                        givenname_rule: GivennameRule::AllNames,
                        year_suffix: false,
                    }),
                    ..Default::default()
                })),
                contributors: Some(ContributorConfig {
                    initialize_with: Some(". ".to_string()),
                    name_form: Some(NameForm::Initials),
                    ..Default::default()
                }),
                ..Default::default()
            },
            None,
        );
        let processor = Processor::new(style, bib);

        let rendered_r1 = processor.process_citation(&Citation::simple("r1")).unwrap();
        let rendered_r2 = processor.process_citation(&Citation::simple("r2")).unwrap();

        assert!(
            rendered_r1.contains("J. Smith"),
            "expected expanded given name for r1: {rendered_r1}"
        );
        assert!(
            rendered_r2.contains("A. Smith"),
            "expected expanded given name for r2: {rendered_r2}"
        );
    }

    /// When `primary-name` is active and expanding the first author's given name does
    /// not resolve the collision (both works share an identical primary author), the
    /// disambiguator must fall back to year-suffix while retaining the et-al expansion
    /// that was found.  Concretely: hints must have `expand_given_names: true`,
    /// `expand_given_names_primary_only: true`, `min_names_to_show: Some(2)`, and
    /// `disamb_condition: true` (year-suffix), with distinct `group_index` values.
    #[test]
    fn test_primary_name_identical_primary_falls_back_to_year_suffix() {
        use citum_schema::options::{
            Disambiguation, Processing, ProcessingCustom, ShortenListOptions,
        };

        // Primary author ("Asthma/Albert") is identical; secondary authors differ only
        // in given name ("Brandon" vs "Edward") — identical families.
        let r1 = make_multi_author_ref(
            "r1",
            &[
                ("Asthma", "Albert"),
                ("Bronchitis", "Brandon"),
                ("Cold", "Crispin"),
            ],
            1990,
        );
        let r2 = make_multi_author_ref(
            "r2",
            &[
                ("Asthma", "Albert"),
                ("Bronchitis", "Edward"),
                ("Cold", "Crispin"),
            ],
            1990,
        );

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), r1);
        bib.insert("r2".to_string(), r2);

        let config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: true,
                    givenname_rule: GivennameRule::PrimaryName,
                    year_suffix: true,
                }),
                ..Default::default()
            })),
            contributors: Some(ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 3,
                    use_first: 1,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let hints = Disambiguator::new(&bib, &config, &config, &locale).calculate_hints();

        let h1 = hints.get("r1").expect("r1 must have a hint");
        let h2 = hints.get("r2").expect("r2 must have a hint");

        // Et-al expansion to two names must be retained.
        assert_eq!(
            h1.min_names_to_show,
            Some(2),
            "r1: expected min_names_to_show=2"
        );
        assert_eq!(
            h2.min_names_to_show,
            Some(2),
            "r2: expected min_names_to_show=2"
        );

        // Given-name expansion must be active (primary author initial shown).
        assert!(h1.expand_given_names, "r1: expected expand_given_names");
        assert!(h2.expand_given_names, "r2: expected expand_given_names");

        // Primary-only flag must be propagated.
        assert!(
            h1.expand_given_names_primary_only,
            "r1: expected primary-only"
        );
        assert!(
            h2.expand_given_names_primary_only,
            "r2: expected primary-only"
        );

        // Year-suffix must be assigned (disamb_condition true, distinct indices).
        assert!(
            h1.disamb_condition,
            "r1: expected disamb_condition (year-suffix)"
        );
        assert!(
            h2.disamb_condition,
            "r2: expected disamb_condition (year-suffix)"
        );
        assert_ne!(
            h1.group_index, h2.group_index,
            "r1 and r2 must receive distinct year-suffix positions"
        );
    }

    #[test]
    fn test_build_reference_cache_populates_title_keys_when_year_suffix_is_active() {
        // title_key must be populated whenever year-suffix is on (regardless of group_sort)
        // so that sort_group_for_year_suffix can use it as a stable tie-breaker.
        use citum_schema::options::{Disambiguation, Processing, ProcessingCustom};

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), make_ref("r1", "Smith", "John", 2020));
        let refs: Vec<&Reference> = bib.values().collect();
        let locale = Locale::en_us();

        let disabled_config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: false,
                    add_givenname: true,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: false,
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        let disabled = Disambiguator::new(&bib, &disabled_config, &disabled_config, &locale);
        let disabled_flags = disabled.disambiguation_flags();
        // year_suffix=false → title_key must be None
        let disabled_cache = disabled.build_reference_cache(&refs, disabled_flags.year_suffix);
        assert!(
            disabled_cache
                .iter()
                .all(|reference| reference.data.title_key.is_none())
        );

        let enabled_config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: false,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: true,
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        let enabled = Disambiguator::new(&bib, &enabled_config, &enabled_config, &locale);
        let enabled_flags = enabled.disambiguation_flags();
        // year_suffix=true → title_key must be Some regardless of group_sort
        let enabled_cache = enabled.build_reference_cache(&refs, enabled_flags.year_suffix);
        assert!(
            enabled_cache
                .iter()
                .all(|reference| reference.data.title_key.is_some())
        );
    }

    #[test]
    fn test_reference_cache_key_uses_reference_id_or_index_fallback() {
        let with_id = make_ref("r1", "Smith", "John", 2020);
        let without_id = make_ref_without_id("missing-id", "Smith", "Jane", 2020);

        assert_eq!(
            Disambiguator::reference_cache_key(7, &with_id),
            ReferenceCacheKey::Id("r1".to_string())
        );
        assert_eq!(
            Disambiguator::reference_cache_key(7, &without_id),
            ReferenceCacheKey::Index(7)
        );

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), with_id);
        bib.insert("missing".to_string(), without_id);
        let refs: Vec<&Reference> = bib.values().collect();
        let cache = Disambiguator::new(
            &bib,
            &Config::default(),
            &Config::default(),
            &Locale::en_us(),
        )
        .build_reference_cache(&refs, false);

        assert_eq!(cache[0].key, ReferenceCacheKey::Id("r1".to_string()));
        assert_eq!(cache[1].key, ReferenceCacheKey::Index(1));
    }

    #[test]
    fn test_anonymous_refs_do_not_receive_year_suffix() {
        // Anonymous entries (no author) sharing the same year must each be placed in
        // their own singleton group, even when an embedded reference id is empty or missing.
        use citum_schema::options::{Disambiguation, Processing, ProcessingCustom};

        let mut bib = Bibliography::new();
        bib.insert("a1".to_string(), make_ref("a1", "", "", 2020));
        bib.insert("a2".to_string(), make_ref("a2", "", "", 2020));
        bib.insert("a3".to_string(), make_ref("", "", "", 2020));
        bib.insert(
            "a4".to_string(),
            make_ref_without_id("missing-id", "", "", 2020),
        );
        let locale = Locale::en_us();
        let config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: true,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: true,
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        let disambiguator = Disambiguator::new(&bib, &config, &config, &locale);
        let refs: Vec<&Reference> = bib.values().collect();
        let cache = disambiguator.build_reference_cache(&refs, false);
        let grouped = disambiguator.group_references(&cache);

        assert_eq!(grouped.len(), 4);
        assert!(!grouped.contains_key("anon:"));
        assert!(grouped.values().all(|group| group.len() == 1));
    }

    #[test]
    fn terminal_author_messages_use_each_references_effective_locale() {
        fn anonymous_ref(id: &str, language: &str) -> Reference {
            Reference::Monograph(Box::new(Monograph {
                id: Some(id.into()),
                r#type: MonographType::Book,
                title: Some(Title::Single(format!("Title {id}"))),
                language: Some(language.parse().expect("valid language tag")),
                issued: DateValue::new("2020"),
                ..Default::default()
            }))
        }

        let mut bibliography = Bibliography::new();
        bibliography.insert("english".to_string(), anonymous_ref("english", "en-US"));
        bibliography.insert("chinese".to_string(), anonymous_ref("chinese", "zh-CN"));
        let config: Config = serde_yaml::from_str(
            r#"
multilingual:
  term-locale: item
substitute:
  candidates: none
  otherwise:
    message: term.anonymous
    form: short
"#,
        )
        .expect("localized terminal substitute should parse");
        let locale = citum_schema::embedded::get_locale("zh-CN").expect("embedded zh-CN locale");
        let refs: Vec<&Reference> = bibliography.values().collect();

        let cache = Disambiguator::new(&bibliography, &config, &config, &locale)
            .build_reference_cache(&refs, false);
        let english_key = &cache
            .iter()
            .find(|reference| reference.key == ReferenceCacheKey::Id("english".to_string()))
            .expect("English cache entry")
            .data
            .author_key;
        let chinese_key = &cache
            .iter()
            .find(|reference| reference.key == ReferenceCacheKey::Id("chinese".to_string()))
            .expect("Chinese cache entry")
            .data
            .author_key;

        assert_eq!(english_key, "Anon|None|None|None|None|None|None|None|None");
        assert_eq!(chinese_key, "佚名|None|None|None|None|None|None|None|None");
        assert_ne!(english_key, chinese_key);
    }

    #[test]
    fn test_push_lowercased_matches_str_lowercase_for_non_ascii() {
        let mut key = String::new();
        let value = "ΟΣ";

        Disambiguator::push_lowercased(&mut key, value);

        assert_eq!(key, value.to_lowercase());
    }

    #[test]
    fn test_partitioned_name_expansion_keeps_unique_items_and_suffixes_remainders() {
        use citum_schema::options::{
            ContributorConfig, Disambiguation, Processing, ProcessingCustom, ShortenListOptions,
        };

        let mut bib = Bibliography::new();
        bib.insert(
            "r1".to_string(),
            make_multi_author_ref("r1", &[("Smith", "John"), ("Jones", "Peter")], 2020),
        );
        bib.insert(
            "r2".to_string(),
            make_multi_author_ref("r2", &[("Smith", "John"), ("Brown", "Alice")], 2020),
        );
        bib.insert(
            "r3".to_string(),
            make_multi_author_ref("r3", &[("Smith", "John"), ("Brown", "Adam")], 2020),
        );

        let config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: true,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: true,
                }),
                ..Default::default()
            })),
            contributors: Some(ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 2,
                    use_first: 1,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let hints = Disambiguator::new(&bib, &config, &config, &locale).calculate_hints();

        let unique = hints.get("r1").unwrap();
        assert!(!unique.disamb_condition);
        assert_eq!(unique.group_index, 1);
        assert_eq!(unique.min_names_to_show, Some(2));
        assert_eq!(unique.group_length, 3);

        let remaining_a = hints.get("r2").unwrap();
        let remaining_b = hints.get("r3").unwrap();
        assert!(remaining_a.disamb_condition);
        assert!(remaining_b.disamb_condition);
        assert_eq!(remaining_a.min_names_to_show, Some(2));
        assert_eq!(remaining_b.min_names_to_show, Some(2));
        assert_eq!(remaining_a.group_length, 3);
        assert_eq!(remaining_b.group_length, 3);
        assert_ne!(remaining_a.group_index, remaining_b.group_index);
    }

    #[test]
    fn test_label_mode_skips_name_strategies_and_suffixes_by_label_group() {
        use citum_schema::options::{LabelConfig, LabelPreset, Processing};

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), make_ref("r1", "Kuhn", "Thomas", 1962));
        bib.insert("r2".to_string(), make_ref("r2", "Kuhn", "Thomas", 1962));

        let config = Config {
            processing: Some(Processing::Label(LabelConfig {
                preset: LabelPreset::Din,
                ..Default::default()
            })),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let hints = Disambiguator::new(&bib, &config, &config, &locale).calculate_hints();
        let first = hints.get("r1").unwrap();
        let second = hints.get("r2").unwrap();

        assert!(first.disamb_condition);
        assert!(second.disamb_condition);
        assert!(!first.expand_given_names);
        assert!(!second.expand_given_names);
        assert_eq!(first.min_names_to_show, None);
        assert_eq!(second.min_names_to_show, None);
        assert_eq!(first.group_key, second.group_key);
        assert!(!first.group_key.contains(':'));
        assert_ne!(first.group_index, second.group_index);
    }

    /// Build a reference whose author is `Contributor::Multilingual` with distinct
    /// `original` but a shared `transliterations` entry keyed by `translit_tag`.
    fn make_multilingual_ref(
        id: &str,
        original_family: &str,
        translit_family: &str,
        translit_tag: &str,
        year: i32,
    ) -> Reference {
        use citum_schema::reference::contributor::MultilingualName;
        use std::collections::HashMap;

        let mut transliterations = HashMap::new();
        transliterations.insert(
            translit_tag.to_string(),
            StructuredName {
                family: MultilingualString::Simple(translit_family.to_string()),
                given: MultilingualString::Simple("A.".to_string()),
                ..Default::default()
            },
        );
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(format!("Title {id}"))),
            author: Some(Contributor::Multilingual(MultilingualName {
                original: StructuredName {
                    family: MultilingualString::Simple(original_family.to_string()),
                    given: MultilingualString::Simple("A.".to_string()),
                    ..Default::default()
                },
                lang: Some("ja".into()),
                sort_as: None,
                transliterations,
                translations: HashMap::new(),
            })),
            issued: DateValue::new(year.to_string()),
            ..Default::default()
        }))
    }

    /// DISAMBIGUATION.md §4: when display mode is `Transliterated`, two references
    /// whose transliterations collide must produce the same author key (→ one
    /// collision group). When mode is `Primary` (distinct originals), keys must differ.
    #[test]
    fn test_multilingual_key_generation_respects_display_mode() {
        use citum_schema::options::MultilingualConfig;
        use citum_schema::options::MultilingualMode;

        // Two distinct Japanese authors that share the same romanisation.
        // Original families differ ("田中" vs "谷中"), but transliteration is "Tanaka".
        let r1 = make_multilingual_ref("r1", "田中", "Tanaka", "ja-Latn", 2020);
        let r2 = make_multilingual_ref("r2", "谷中", "Tanaka", "ja-Latn", 2020);

        let mut bib = Bibliography::new();
        bib.insert("r1".to_string(), r1);
        bib.insert("r2".to_string(), r2);

        let locale = Locale::en_us();

        // --- case 1: Transliterated mode → same key (collision) ---
        let config_translit = Config {
            multilingual: Some(MultilingualConfig {
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_transliteration: Some(vec!["ja-Latn".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let cache_translit = Disambiguator::new(&bib, &config_translit, &config_translit, &locale)
            .build_reference_cache(&[bib.get("r1").unwrap(), bib.get("r2").unwrap()], false);

        let ck_r1 = ReferenceCacheKey::Id("r1".to_string());
        let ck_r2 = ReferenceCacheKey::Id("r2".to_string());
        let ak_r1 = &cache_translit
            .iter()
            .find(|reference| reference.key == ck_r1)
            .expect("r1 cache entry")
            .data
            .author_key;
        let ak_r2 = &cache_translit
            .iter()
            .find(|reference| reference.key == ck_r2)
            .expect("r2 cache entry")
            .data
            .author_key;

        assert_eq!(
            ak_r1, ak_r2,
            "transliterated mode: colliding transliterations must produce the same author key"
        );
        assert_eq!(
            ak_r1, "tanaka",
            "key should be the lowercased transliteration"
        );

        // --- case 2: Primary mode → distinct keys (no collision) ---
        let config_primary = Config::default(); // multilingual: None → falls through to original

        let cache_primary = Disambiguator::new(&bib, &config_primary, &config_primary, &locale)
            .build_reference_cache(&[bib.get("r1").unwrap(), bib.get("r2").unwrap()], false);

        let ak_r1_primary = &cache_primary
            .iter()
            .find(|reference| reference.key == ck_r1)
            .expect("r1 cache entry")
            .data
            .author_key;
        let ak_r2_primary = &cache_primary
            .iter()
            .find(|reference| reference.key == ck_r2)
            .expect("r2 cache entry")
            .data
            .author_key;

        assert_ne!(
            ak_r1_primary, ak_r2_primary,
            "primary mode: distinct originals must produce different author keys"
        );
    }

    /// Effective bibliography config for `sorting.multilingual: romanized`, with
    /// a preferred transliteration — mirrors `sorting.rs`'s `romanized_config`.
    fn romanized_config() -> Config {
        Config {
            sorting: Some(SortingConfig {
                multilingual: Some(SortingMultilingualMode::Romanized),
                ..Default::default()
            }),
            multilingual: Some(MultilingualConfig {
                preferred_transliteration: Some(vec!["ru-Latn-alalc97".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Two same-author, same-year references whose original (Cyrillic) title
    /// order is the *opposite* of their `sort-as` (romanized) title order.
    ///
    /// - `r-alpha`: original starts with "Я" (sorts last in Cyrillic collation),
    ///   `sort-as` starts with "Apple" (sorts first once romanized).
    /// - `r-beta`: original starts with "А" (sorts first in Cyrillic collation),
    ///   `sort-as` starts with "Zebra" (sorts last once romanized).
    ///
    /// So uniform sorting orders them beta-then-alpha, while romanized sorting
    /// orders them alpha-then-beta — the two policies deliberately disagree.
    fn multilingual_year_suffix_pair() -> (Reference, Reference) {
        let make = |id: &str, original: &str, sort_as: &str| {
            Reference::Monograph(Box::new(Monograph {
                id: Some(id.into()),
                r#type: MonographType::Book,
                title: Some(Title::Multilingual(MultilingualComplex {
                    original: original.to_string(),
                    lang: Some("ru".into()),
                    sort_as: Some(sort_as.to_string()),
                    transliterations: HashMap::new(),
                    translations: HashMap::new(),
                })),
                short_title: None,
                container: None,
                author: Some(Contributor::StructuredName(StructuredName {
                    family: MultilingualString::Simple("Smith".to_string()),
                    given: MultilingualString::Simple("Jordan".to_string()),
                    suffix: None,
                    dropping_particle: None,
                    non_dropping_particle: None,
                })),
                editor: None,
                translator: None,
                issued: DateValue::new("2020".to_string()),
                ..Default::default()
            }))
        };

        (
            make("r-alpha", "Яблоко", "Apple Studies"),
            make("r-beta", "Абрикос", "Zebra Studies"),
        )
    }

    fn title_group_sort() -> GroupSort {
        GroupSort {
            template: vec![GroupSortKey {
                key: SortKey::Title,
                ascending: true,
                order: None,
                sort_order: None,
            }],
        }
    }

    #[test]
    fn year_suffix_order_follows_romanized_bibliography_sort_policy() {
        let (r_alpha, r_beta) = multilingual_year_suffix_pair();
        let mut bib = Bibliography::new();
        bib.insert("r-alpha".to_string(), r_alpha);
        bib.insert("r-beta".to_string(), r_beta);

        let locale = Locale::en_us();
        let config = Config::default();
        let sort_spec = title_group_sort();

        // Uniform policy (the historical bug): year-suffix order follows the
        // original Cyrillic title order, independent of any bibliography
        // multilingual/locale sort configuration.
        let uniform_disamb =
            Disambiguator::with_group_sort(&bib, &config, &config, &locale, &sort_spec);
        let uniform_hints = uniform_disamb.calculate_hints();
        assert_eq!(
            uniform_hints.get("r-beta").unwrap().group_index,
            1,
            "uniform policy: original-text order puts r-beta ('Абрикос') first"
        );
        assert_eq!(
            uniform_hints.get("r-alpha").unwrap().group_index,
            2,
            "uniform policy: original-text order puts r-alpha ('Яблоко') second"
        );

        // Romanized policy: year-suffix order must follow the same sort-key
        // policy the final bibliography uses (sort-as / romanized order),
        // which is the opposite of the uniform original-text order here.
        let romanized_sort_config = romanized_config();
        let romanized_disamb = Disambiguator::with_group_sort(
            &bib,
            &config,
            &romanized_sort_config,
            &locale,
            &sort_spec,
        );
        let romanized_hints = romanized_disamb.calculate_hints();
        assert_eq!(
            romanized_hints.get("r-alpha").unwrap().group_index,
            1,
            "romanized policy: sort-as order puts r-alpha ('Apple Studies') first"
        );
        assert_eq!(
            romanized_hints.get("r-beta").unwrap().group_index,
            2,
            "romanized policy: sort-as order puts r-beta ('Zebra Studies') second"
        );

        // Tie the assertion explicitly to the final bibliography's own sorter,
        // so this test fails if the two ever diverge again.
        let refs: Vec<&Reference> = vec![bib.get("r-alpha").unwrap(), bib.get("r-beta").unwrap()];
        let sorter = ReferenceSorter::with_bibliography_config(&locale, &romanized_sort_config);
        let sorted_ids: Vec<String> = sorter
            .sort_references(refs, &sort_spec)
            .into_iter()
            .map(|reference| reference.id().expect("id").to_string())
            .collect();
        assert_eq!(
            sorted_ids,
            vec!["r-alpha".to_string(), "r-beta".to_string()],
            "final bibliography order must match the year-suffix group order"
        );
    }

    /// Reproduces the group-local disambiguation call site
    /// (`processor/bibliography/grouping.rs::build_group_local_hints`), which
    /// passes the same effective bibliography config as both `config` and
    /// `sort_config`. Confirms the fix applies to the group-local path too,
    /// not just the global `calculate_hints` path in `processor/setup.rs`.
    #[test]
    fn group_local_disambiguation_also_follows_romanized_bibliography_sort_policy() {
        let (r_alpha, r_beta) = multilingual_year_suffix_pair();
        let mut group_bibliography = Bibliography::new();
        group_bibliography.insert("r-alpha".to_string(), r_alpha);
        group_bibliography.insert("r-beta".to_string(), r_beta);

        let locale = Locale::en_us();
        let bibliography_config = romanized_config();
        let sort_spec = title_group_sort();

        let disambiguator = Disambiguator::with_group_sort(
            &group_bibliography,
            &bibliography_config,
            &bibliography_config,
            &locale,
            &sort_spec,
        );
        let hints = disambiguator.calculate_hints();

        assert_eq!(
            hints.get("r-alpha").unwrap().group_index,
            1,
            "group-local path: romanized sort-as order puts r-alpha first"
        );
        assert_eq!(
            hints.get("r-beta").unwrap().group_index,
            2,
            "group-local path: romanized sort-as order puts r-beta second"
        );
    }

    // csl26-huuz: collision grouping must reflect what the resolved date
    // slot actually renders for an undated reference, not a uniform "no
    // date" assumption.

    /// A reference with an explicit issued/accessed pair and a real shared
    /// author. `date_component_discriminant` never reads the author; the
    /// integration test below shares one author across every case so only
    /// the date half of the collision key varies.
    fn make_dated_ref(id: &str, family: &str, issued: &str, accessed: Option<&str>) -> Reference {
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(format!("Title {id}"))),
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(String::new()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new(issued),
            accessed: accessed.map(DateValue::new),
            ..Default::default()
        }))
    }

    /// The GB/T-shaped date component under test: primary `issued`, falling
    /// back to an access year (never a discriminant, per csl26-huuz) and
    /// then to the locale's no-date term.
    fn issued_date_component() -> TemplateDate {
        TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            ..Default::default()
        }
    }

    #[test]
    fn date_slot_uses_the_first_effective_issued_component() {
        use citum_schema::template::{
            TemplateConditionField, TemplateGroup, TemplateGroupCondition,
        };

        let reference = make_dated_ref("issued-slot", "Smith", "", None);
        let mut bibliography = Bibliography::new();
        bibliography.insert("issued-slot".to_string(), reference);
        let config: Config =
            serde_yaml::from_str("date-fallback: standard").expect("standard policy should parse");
        let locale = Locale::en_us();
        let bibliography_spec = BibliographySpec {
            template: Some(
                vec![
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![TemplateComponent::Date(TemplateDate {
                            date: DateVariable::OriginalPublished,
                            form: DateForm::Year,
                            ..Default::default()
                        })],
                        render_when: Some(TemplateGroupCondition {
                            field_present: Some(TemplateConditionField::OriginalPublished),
                            field_absent: None,
                        }),
                        ..Default::default()
                    }),
                    TemplateComponent::Date(issued_date_component()),
                ]
                .into(),
            ),
            ..Default::default()
        };
        let reference = bibliography.get("issued-slot").expect("reference");

        let discriminant = Disambiguator::new(&bibliography, &config, &config, &locale)
            .with_bibliography_spec(&bibliography_spec)
            .date_slot_discriminant(reference);

        assert_eq!(discriminant, "n.d.|None|None|None|None|None|None|None|None");
    }

    #[rstest]
    #[case::real_issued_value_wins_without_consulting_fallback(
        "2020",
        None,
        "2020||None|None|None|None|None|None|None|None"
    )]
    #[case::present_accessed_stops_the_chain_with_no_identity("", Some("2020"), "")]
    #[case::absent_accessed_falls_through_to_the_no_date_term(
        "",
        None,
        "no date|None|None|None|None|None|None|None|None"
    )]
    #[case::empty_string_accessed_is_treated_as_absent(
        "",
        Some(""),
        "no date|None|None|None|None|None|None|None|None"
    )]
    fn given_an_issued_with_accessed_fallback_when_computing_the_collision_discriminant_then_it_matches(
        #[case] issued: &str,
        #[case] accessed: Option<&str>,
        #[case] expected: &str,
    ) {
        let reference = make_dated_ref("d1", "Smith", issued, accessed);
        let component = issued_date_component();
        let config: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    default:
    - date: accessed
      form: year
    - message: term.no-date
"#,
        )
        .expect("date fallback should parse");

        let locale = Locale::en_us();
        let discriminant = Disambiguator::date_component_discriminant(
            &component,
            &reference,
            &locale,
            &config,
            &reference.ref_type(),
        );

        assert_eq!(discriminant, expected);
    }

    #[test]
    fn standard_and_explicit_short_no_date_rules_share_the_same_discriminant() {
        let reference = make_dated_ref("d2", "Smith", "", None);
        let component = issued_date_component();
        let standard: Config =
            serde_yaml::from_str("date-fallback: standard").expect("preset should parse");
        let explicit: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    default:
    - message: term.no-date
      form: short
"#,
        )
        .expect("explicit policy should parse");

        let locale = Locale::en_us();
        let discriminants = [&standard, &explicit].map(|config| {
            Disambiguator::date_component_discriminant(
                &component,
                &reference,
                &locale,
                config,
                &reference.ref_type(),
            )
        });

        assert_eq!(discriminants[0], discriminants[1]);
        assert_eq!(
            discriminants[0],
            "n.d.|None|None|None|None|None|None|None|None"
        );
    }

    #[test]
    fn unresolved_message_fallback_continues_to_the_next_candidate() {
        let reference = make_dated_ref("d2", "Smith", "", None);
        let component = issued_date_component();
        let config: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    default:
    - message: term.does-not-exist
    - message: term.no-date
"#,
        )
        .expect("date fallback should parse");

        let locale = Locale::en_us();
        let discriminant = Disambiguator::date_component_discriminant(
            &component,
            &reference,
            &locale,
            &config,
            &reference.ref_type(),
        );

        assert_eq!(
            discriminant,
            "no date|None|None|None|None|None|None|None|None"
        );
    }

    #[test]
    fn explicit_none_rule_yields_empty_discriminant() {
        let reference = make_dated_ref("d3", "Smith", "", None);
        let component = issued_date_component();
        let config: Config =
            serde_yaml::from_str("date-fallback:\n  first-issued:\n    default: none")
                .expect("none rule should parse");

        let locale = Locale::en_us();
        let discriminant = Disambiguator::date_component_discriminant(
            &component,
            &reference,
            &locale,
            &config,
            &reference.ref_type(),
        );

        assert_eq!(discriminant, "");
    }

    #[test]
    fn fallback_candidate_note_obeys_the_candidate_suppress_note_flag() {
        let date = DateValue {
            value: "1947".to_string(),
            note: Some("民国36年".to_string()),
        };
        let rendering = Rendering {
            prefix: Some("c".into()),
            ..Default::default()
        };
        let date_config = DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        };
        let locale = Locale::en_us();

        let visible = crate::values::date::fallback_candidate_discriminant(
            &date,
            &DateForm::Year,
            &rendering,
            None,
            &locale,
            Some(&date_config),
        );
        let suppressed = crate::values::date::fallback_candidate_discriminant(
            &date,
            &DateForm::Year,
            &rendering,
            Some(true),
            &locale,
            Some(&date_config),
        );

        assert_eq!(
            visible.as_deref(),
            Some("1947|民国36年|None|None|None|None|None|Some(Custom(\"c\"))|None|None")
        );
        assert_eq!(
            suppressed.as_deref(),
            Some("1947||None|None|None|None|None|Some(Custom(\"c\"))|None|None")
        );
    }

    #[test]
    fn identity_date_slot_uses_the_configuration_from_its_effective_scope() {
        let reference = Reference::Monograph(Box::new(Monograph {
            id: Some("scope-date".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Scoped date".to_string())),
            issued: DateValue::new(""),
            copyright: Some(DateValue {
                value: "1995".to_string(),
                note: Some("source calendar".to_string()),
            }),
            ..Default::default()
        }));
        let date_component = TemplateDate {
            date: DateVariable::Copyright,
            form: DateForm::Year,
            ..Default::default()
        };
        let bibliography_spec = BibliographySpec {
            template: Some(vec![TemplateComponent::Date(date_component.clone())].into()),
            ..Default::default()
        };
        let citation_spec = CitationSpec {
            template: Some(vec![TemplateComponent::Date(date_component)].into()),
            ..Default::default()
        };
        let note_dates = DateConfig {
            note_wrap: Some(WrapConfig {
                punctuation: WrapPunctuation::Parentheses,
                inner_prefix: None,
                inner_suffix: None,
            }),
            ..Default::default()
        };
        let with_note = Config {
            dates: Some(note_dates),
            ..Default::default()
        };
        let without_note = Config::default();
        let bibliography = Bibliography::new();
        let locale = Locale::en_us();

        let bibliography_owned =
            Disambiguator::new(&bibliography, &without_note, &with_note, &locale)
                .with_citation_spec(&citation_spec)
                .with_bibliography_spec(&bibliography_spec)
                .date_slot_discriminant(&reference);
        let citation_owned = Disambiguator::new(&bibliography, &with_note, &without_note, &locale)
            .with_citation_spec(&citation_spec)
            .date_slot_discriminant(&reference);

        assert_eq!(
            bibliography_owned, "1995|source calendar|None|None|None|None|None|None|None|None",
            "a bibliography-selected slot must use effective bibliography date options"
        );
        assert_eq!(
            citation_owned, "1995|source calendar|None|None|None|None|None|None|None|None",
            "the citation fallback slot must use effective citation date options"
        );
    }

    /// A reference whose only date is a `copyright` fallback candidate with
    /// day precision — mirrors GB/T 7714's `book,thesis,map` fallback chain
    /// (`date: copyright, form: year`).
    fn make_ref_with_copyright(id: &str, family: &str, copyright: &str) -> Reference {
        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(Title::Single(format!("Title {id}"))),
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple(family.to_string()),
                given: MultilingualString::Simple(String::new()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new(""),
            copyright: Some(DateValue::new(copyright)),
            ..Default::default()
        }))
    }

    #[rstest]
    #[case::day_precision_early_in_the_year("1995-03-01")]
    #[case::day_precision_late_in_the_year("1995-11-20")]
    fn given_a_copyright_fallback_date_with_day_precision_when_the_form_is_year_then_the_discriminant_is_the_bare_year(
        #[case] copyright: &str,
    ) {
        // `DateValue`'s `Display` is the raw stored EDTF string. Reading it
        // directly here would give "1995-03-01" and "1995-11-20" — two
        // different discriminants for two references that both render as
        // the bare year "1995" under `form: year`, wrongly treating them as
        // already distinguishable. Flagged in PR review for csl26-huuz.
        let component = issued_date_component();
        let config: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    default:
    - date: copyright
      form: year
"#,
        )
        .expect("copyright fallback should parse");
        let reference = make_ref_with_copyright("r1", "Smith", copyright);
        let locale = Locale::en_us();

        let discriminant = Disambiguator::date_component_discriminant(
            &component,
            &reference,
            &locale,
            &config,
            &reference.ref_type(),
        );

        assert_eq!(
            discriminant,
            "1995||None|None|None|None|None|None|None|None"
        );
    }

    /// GB/T 7714's real `book,thesis,map` fallback chain: `copyright`
    /// prefixed with `c`, `printing` suffixed with `印刷`. A reference
    /// resolving one and a reference resolving the other can share a bare
    /// formatted year (`"1995"`) while rendering visibly different text
    /// (`c1995` vs `1995印刷`) — the discriminant must not collapse them.
    /// Flagged in PR review for csl26-huuz.
    #[test]
    fn copyright_and_printing_fallbacks_with_the_same_year_do_not_collide() {
        let component = issued_date_component();
        let config: Config = serde_yaml::from_str(
            r#"
date-fallback:
  first-issued:
    default:
    - date: copyright
      form: year
      prefix: c
    - date: printing
      form: year
      suffix: 印刷
"#,
        )
        .expect("publication fallback should parse");
        let copyright_ref = Reference::Monograph(Box::new(Monograph {
            id: Some("copyright-ref".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Title copyright-ref".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple("Smith".to_string()),
                given: MultilingualString::Simple(String::new()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new(""),
            copyright: Some(DateValue::new("1995")),
            ..Default::default()
        }));
        let printing_ref = Reference::Monograph(Box::new(Monograph {
            id: Some("printing-ref".into()),
            r#type: MonographType::Book,
            title: Some(Title::Single("Title printing-ref".to_string())),
            author: Some(Contributor::StructuredName(StructuredName {
                family: MultilingualString::Simple("Smith".to_string()),
                given: MultilingualString::Simple(String::new()),
                suffix: None,
                dropping_particle: None,
                non_dropping_particle: None,
            })),
            issued: DateValue::new(""),
            copyright: None,
            printing: Some(DateValue::new("1995")),
            ..Default::default()
        }));
        let locale = Locale::en_us();

        let copyright_discriminant = Disambiguator::date_component_discriminant(
            &component,
            &copyright_ref,
            &locale,
            &config,
            &copyright_ref.ref_type(),
        );
        let printing_discriminant = Disambiguator::date_component_discriminant(
            &component,
            &printing_ref,
            &locale,
            &config,
            &printing_ref.ref_type(),
        );

        assert_ne!(
            copyright_discriminant, printing_discriminant,
            "c1995 and 1995印刷 render visibly different text and must not collide"
        );
    }

    #[test]
    fn differing_access_dates_still_collide_via_shared_disambiguator() {
        // Mirrors GB/T 7714's webpage type-variant: an access date never
        // carries identity, so two references with *different* access years
        // must still collide (form one suffixed group), while a reference
        // with no access date at all — reaching the no-date term instead —
        // must land in a *separate* group. Both groups share the same
        // author, so only the date-slot discriminant can be responsible for
        // the split. See csl26-huuz.
        use citum_schema::options::{Disambiguation, Processing, ProcessingCustom};

        let mut bib = Bibliography::new();
        bib.insert(
            "b1".to_string(),
            make_dated_ref("b1", "Smith", "", Some("2020")),
        );
        bib.insert(
            "b2".to_string(),
            make_dated_ref("b2", "Smith", "", Some("2019")),
        );
        bib.insert("b3".to_string(), make_dated_ref("b3", "Smith", "", None));
        bib.insert("b4".to_string(), make_dated_ref("b4", "Smith", "", None));

        let locale = Locale::en_us();
        let mut config = Config {
            processing: Some(Processing::Custom(ProcessingCustom {
                base: None,
                disambiguate: Some(Disambiguation {
                    names: false,
                    add_givenname: false,
                    givenname_rule: GivennameRule::default(),
                    year_suffix: true,
                }),
                ..Default::default()
            })),
            ..Default::default()
        };
        config.date_fallback = Some(
            serde_yaml::from_str::<citum_schema::options::DateFallbackEntry>(
                r#"
first-issued:
  default:
  - date: accessed
    form: year
  - message: term.no-date
"#,
            )
            .expect("date fallback should parse")
            .resolve(),
        );
        let bibliography_spec = BibliographySpec {
            template: Some(
                vec![
                    citum_schema::tc_contributor!(Author, Long),
                    TemplateComponent::Date(issued_date_component()),
                ]
                .into(),
            ),
            ..Default::default()
        };
        let disambiguator = Disambiguator::new(&bib, &config, &config, &locale)
            .with_bibliography_spec(&bibliography_spec);
        let hints = disambiguator.calculate_hints();

        let accessed_group_key = hints.get("b1").unwrap().group_key.clone();
        let no_date_group_key = hints.get("b3").unwrap().group_key.clone();

        assert_eq!(
            hints.get("b2").unwrap().group_key,
            accessed_group_key,
            "different access years still share one group — the value never discriminates"
        );
        assert_eq!(
            hints.get("b4").unwrap().group_key,
            no_date_group_key,
            "both no-access-date references share the other group"
        );
        assert_ne!(
            accessed_group_key, no_date_group_key,
            "an access-date group must not merge with the no-date-term group"
        );

        // `group_length` reports same-*author* count (documented on
        // `author_group_lengths`), not collision-group size — all four
        // share author "Smith", so it's 4 for every entry here regardless
        // of which of the two date-discriminant groups they're actually in.
        // `group_index` is the field that reflects actual collision-group
        // membership: each 2-member group gets index 1 and 2.
        let b1_index = hints.get("b1").unwrap().group_index;
        let b2_index = hints.get("b2").unwrap().group_index;
        let b3_index = hints.get("b3").unwrap().group_index;
        let b4_index = hints.get("b4").unwrap().group_index;
        assert_eq!(
            [b1_index, b2_index]
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            [1, 2].into_iter().collect(),
            "the access-date pair is indexed 1 and 2 within its own group"
        );
        assert_eq!(
            [b3_index, b4_index]
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            [1, 2].into_iter().collect(),
            "the no-date pair is indexed 1 and 2 within its own, separate group"
        );

        for id in ["b1", "b2", "b3", "b4"] {
            assert!(
                hints.get(id).unwrap().disamb_condition,
                "{id}: both groups need a suffix"
            );
        }
    }
}
