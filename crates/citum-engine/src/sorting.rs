/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Reference sorting for bibliographies, groups, and citations.
//!
//! `ReferenceSorter` is the engine's single sorting stack: bibliography sorts
//! (explicit `bibliography.sort`, processing presets, and config-level
//! `sort:` mapped via `Sort::group_sort()`), per-group sorting, citation-item
//! ordering, and disambiguation all route through it. Sort keys are compiled
//! once and cached per reference (Schwartzian transform), so collation and
//! article stripping are not re-derived on every comparison. Supports:
//! - Type-order sorting (explicit sequence like [legal-case, statute, treaty])
//! - Name-order sorting (family-given vs given-family for multilingual bibliographies)
//! - Standard sort keys (author, title, issued) and an opt-in reference-ID tiebreak

use std::collections::HashMap;

use citum_schema::grouping::{GroupSort, GroupSortKey, NameSortOrder, SortKey as GroupSortKeyType};
use citum_schema::locale::Locale;
use citum_schema::options::Config;
#[cfg(test)]
use citum_schema::reference::ClassExtension;

use crate::reference::Reference;
use crate::sort_support::{
    SortKeyOptions, TextCollator, collator_locale_id, flat_names_sort_key, normalize_sort_text,
    title_sort_key_with_options,
};

enum PrimaryContributorSpec<'a> {
    Citation(&'a citum_schema::CitationSpec),
    Bibliography(&'a citum_schema::BibliographySpec),
}

/// Compares two optional values, with `None` sorting after every `Some`.
/// `Option::cmp`'s default (`None` first) is not equivalent and must not be
/// substituted for this. Shared by full-date `Issued` comparisons
/// (`ReferenceSorter::compare_by_issued`, `cache_sort_value`) and reference-id
/// comparisons (`ReferenceSorter::compare_cached_ids`,
/// `Disambiguator::compare_reference_ids` in `processor/disambiguation.rs`)
/// so the tiebreak semantics can't drift between the two — csl26-m8la,
/// PR #1150 review.
pub(crate) fn compare_none_last<T: Ord>(a: Option<T>, b: Option<T>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// Sorts grouped bibliography entries using group-specific sort rules.
pub struct ReferenceSorter<'a> {
    locale: &'a Locale,
    text_collator: TextCollator,
    sort_key_options: SortKeyOptions,
    config: Option<&'a Config>,
    primary_contributor_spec: Option<PrimaryContributorSpec<'a>>,
    primary_contributor_may_be_list: bool,
}

struct CachedReference<'a> {
    reference: &'a Reference,
    sort_values: Vec<CachedSortValue>,
    /// Reference ID, cached once per reference so the ID tiebreak does not
    /// re-derive it on every pairwise comparison. Populated only when sorting
    /// via [`ReferenceSorter::sort_references_with_id_tiebreak`]; `None` otherwise
    /// to keep ID-less sorts allocation-free.
    id: Option<String>,
}

enum CachedSortValue {
    RefType { name: String, rank: Option<usize> },
    OptionalText(Option<String>),
    Text(String),
    Issued(Option<(i32, u32, u32)>),
}

enum CompiledSortKey<'a> {
    RefType {
        ascending: bool,
        rank_by_type: Option<HashMap<String, usize>>,
    },
    Author {
        ascending: bool,
        name_order: NameSortOrder,
    },
    Title {
        ascending: bool,
    },
    Issued {
        ascending: bool,
    },
    Field {
        ascending: bool,
        field_name: &'a str,
    },
}

impl<'a> ReferenceSorter<'a> {
    /// Create a sorter that uses `locale` for locale-sensitive comparisons.
    #[must_use]
    pub fn new(locale: &'a Locale) -> Self {
        Self {
            locale,
            text_collator: TextCollator::new(locale),
            sort_key_options: SortKeyOptions::uniform(),
            config: None,
            primary_contributor_spec: None,
            primary_contributor_may_be_list: false,
        }
    }

    /// Create a bibliography sorter using the effective bibliography config.
    #[must_use]
    pub fn with_bibliography_config(locale: &'a Locale, config: &'a Config) -> Self {
        Self {
            locale,
            text_collator: TextCollator::new_for_locale_id(collator_locale_id(locale, config)),
            sort_key_options: SortKeyOptions::from_config(config),
            config: Some(config),
            primary_contributor_spec: None,
            primary_contributor_may_be_list: false,
        }
    }

    /// Use the effective bibliography template to resolve list-form author keys.
    #[must_use]
    pub fn with_bibliography_spec(mut self, spec: &'a citum_schema::BibliographySpec) -> Self {
        self.primary_contributor_spec = Some(PrimaryContributorSpec::Bibliography(spec));
        self.primary_contributor_may_be_list = bibliography_may_have_list_primary(spec);
        self
    }

    /// Use the effective citation template to resolve list-form author keys.
    #[must_use]
    pub fn with_citation_spec(mut self, spec: &'a citum_schema::CitationSpec) -> Self {
        self.primary_contributor_spec = Some(PrimaryContributorSpec::Citation(spec));
        self.primary_contributor_may_be_list = citation_may_have_list_primary(spec);
        self
    }

    /// Sort references according to a group sort specification.
    ///
    /// Applies sort keys in order, with later keys acting as tiebreakers.
    ///
    /// # Arguments
    ///
    /// * `references` - References to sort
    /// * `sort_spec` - Group sort specification
    #[must_use]
    pub fn sort_references<'b>(
        &self,
        references: Vec<&'b Reference>,
        sort_spec: &GroupSort,
    ) -> Vec<&'b Reference> {
        self.sort_references_impl(references, sort_spec, false)
    }

    /// Sort references according to a group sort specification, breaking ties
    /// between references whose keys compare equal by comparing reference IDs.
    ///
    /// References without an ID sort after references with one. The tiebreak
    /// makes config-driven bibliography sorts fully deterministic, and is
    /// opt-in because most `ReferenceSorter` call sites (grouping/render paths)
    /// rely on stable-sort/registry order for equal keys instead.
    ///
    /// An empty sort template is a no-op: with no keys to compare, references
    /// keep registry order rather than being reordered by ID alone.
    #[must_use]
    pub fn sort_references_with_id_tiebreak<'b>(
        &self,
        references: Vec<&'b Reference>,
        sort_spec: &GroupSort,
    ) -> Vec<&'b Reference> {
        self.sort_references_impl(references, sort_spec, true)
    }

    fn sort_references_impl<'b>(
        &self,
        mut references: Vec<&'b Reference>,
        sort_spec: &GroupSort,
        id_tiebreak: bool,
    ) -> Vec<&'b Reference> {
        let compiled_keys = self.compile_sort_keys(&sort_spec.template);
        if compiled_keys.is_empty() {
            return references;
        }

        let mut cached_references = references
            .drain(..)
            .map(|reference| CachedReference {
                reference,
                sort_values: compiled_keys
                    .iter()
                    .map(|sort_key| self.cache_sort_value(reference, sort_key))
                    .collect(),
                id: id_tiebreak.then(|| reference.id().map(|id| id.0)).flatten(),
            })
            .collect::<Vec<_>>();

        cached_references.sort_by(|a, b| {
            let cmp = self.compare_cached_references(a, b, &compiled_keys);
            if cmp != std::cmp::Ordering::Equal || !id_tiebreak {
                return cmp;
            }
            Self::compare_cached_ids(a, b)
        });
        cached_references
            .into_iter()
            .map(|entry| entry.reference)
            .collect()
    }

    /// Deterministic tiebreaker: compare cached reference IDs as `&str`,
    /// with a missing ID sorting last (see [`compare_none_last`]).
    fn compare_cached_ids(a: &CachedReference<'_>, b: &CachedReference<'_>) -> std::cmp::Ordering {
        compare_none_last(a.id.as_deref(), b.id.as_deref())
    }

    /// Compare two references by a single sort key.
    #[must_use]
    pub fn compare_by_key(
        &self,
        a: &Reference,
        b: &Reference,
        sort_key: &GroupSortKey,
    ) -> std::cmp::Ordering {
        self.compare_by_key_with_context(a, b, sort_key)
    }

    /// Stably sort arbitrary items by a `GroupSort` template, precomputing
    /// each item's sort key set once instead of re-deriving it (author,
    /// title, issued resolution) on every pairwise comparison.
    ///
    /// This generalizes the Schwartzian-transform caching
    /// [`Self::sort_references_impl`] applies to `&Reference` slices to any
    /// item type, via a reference-extraction closure — letting comparator
    /// call sites that don't sort bare `&Reference` (year-suffix ordering,
    /// citation-item ordering) share the same cached comparison instead of
    /// calling [`Self::compare_by_key`] from scratch on every pair.
    ///
    /// Items whose extractor returns `None` sort after every item with a
    /// resolved reference; ties (including `None` vs `None`) preserve their
    /// relative order (`sort_by` is stable).
    pub(crate) fn sort_by_keys<T>(
        &self,
        mut items: Vec<T>,
        template: &[GroupSortKey],
        reference_of: impl Fn(&T) -> Option<&Reference>,
    ) -> Vec<T> {
        let compiled_keys = self.compile_sort_keys(template);
        let mut decorated = items
            .drain(..)
            .map(|item| {
                let sort_values = reference_of(&item).map(|reference| {
                    compiled_keys
                        .iter()
                        .map(|sort_key| self.cache_sort_value(reference, sort_key))
                        .collect::<Vec<_>>()
                });
                (item, sort_values)
            })
            .collect::<Vec<_>>();

        decorated.sort_by(|(_, a), (_, b)| match (a, b) {
            (Some(a), Some(b)) => self.compare_cached_values(a, b, &compiled_keys),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        decorated.into_iter().map(|(item, _)| item).collect()
    }

    fn compare_by_key_with_context(
        &self,
        a: &Reference,
        b: &Reference,
        sort_key: &GroupSortKey,
    ) -> std::cmp::Ordering {
        let cmp = match &sort_key.key {
            GroupSortKeyType::RefType => sort_key.order.as_ref().map_or_else(
                || a.ref_type().cmp(&b.ref_type()),
                |order| Self::compare_by_type_order(a, b, order),
            ),
            GroupSortKeyType::Author => sort_key.sort_order.as_ref().map_or_else(
                || self.compare_by_author_with_order(a, b, NameSortOrder::FamilyGiven),
                |name_order| self.compare_by_author_with_order(a, b, *name_order),
            ),
            GroupSortKeyType::Title => self.compare_by_title(a, b),
            GroupSortKeyType::Issued => Self::compare_by_issued(a, b),
            GroupSortKeyType::Field(field_name) => Self::compare_by_field(a, b, field_name),
        };

        if sort_key.ascending {
            cmp
        } else {
            cmp.reverse()
        }
    }

    /// Compare by type using explicit order sequence.
    ///
    /// Types appear in the order specified, regardless of alphabetical content.
    /// Types not in the order list sort after those in the list, alphabetically.
    fn compare_by_type_order(a: &Reference, b: &Reference, order: &[String]) -> std::cmp::Ordering {
        let a_type = a.ref_type();
        let b_type = b.ref_type();

        let a_pos = order.iter().position(|t| t == &a_type);
        let b_pos = order.iter().position(|t| t == &b_type);

        match (a_pos, b_pos) {
            (Some(a_idx), Some(b_idx)) => a_idx.cmp(&b_idx),
            (Some(_), None) => std::cmp::Ordering::Less, // a in order, b not
            (None, Some(_)) => std::cmp::Ordering::Greater, // b in order, a not
            (None, None) => a_type.cmp(&b_type),         // both not in order, alphabetical
        }
    }

    fn compile_sort_keys<'b>(&self, template: &'b [GroupSortKey]) -> Vec<CompiledSortKey<'b>> {
        template
            .iter()
            .map(|sort_key| match &sort_key.key {
                GroupSortKeyType::RefType => CompiledSortKey::RefType {
                    ascending: sort_key.ascending,
                    rank_by_type: sort_key.order.as_ref().map(|order| {
                        order
                            .iter()
                            .enumerate()
                            .map(|(index, ref_type)| (ref_type.clone(), index))
                            .collect()
                    }),
                },
                GroupSortKeyType::Author => CompiledSortKey::Author {
                    ascending: sort_key.ascending,
                    name_order: sort_key.sort_order.unwrap_or(NameSortOrder::FamilyGiven),
                },
                GroupSortKeyType::Title => CompiledSortKey::Title {
                    ascending: sort_key.ascending,
                },
                GroupSortKeyType::Issued => CompiledSortKey::Issued {
                    ascending: sort_key.ascending,
                },
                GroupSortKeyType::Field(field_name) => CompiledSortKey::Field {
                    ascending: sort_key.ascending,
                    field_name,
                },
            })
            .collect()
    }

    fn cache_sort_value(
        &self,
        reference: &Reference,
        sort_key: &CompiledSortKey<'_>,
    ) -> CachedSortValue {
        match sort_key {
            CompiledSortKey::RefType { rank_by_type, .. } => {
                let ref_type = reference.ref_type();
                CachedSortValue::RefType {
                    name: ref_type.clone(),
                    rank: rank_by_type
                        .as_ref()
                        .and_then(|ranks| ranks.get(&ref_type).copied()),
                }
            }
            CompiledSortKey::Author { name_order, .. } => CachedSortValue::OptionalText(
                self.extract_author_sort_key_opt(reference, *name_order),
            ),
            CompiledSortKey::Title { .. } => CachedSortValue::Text(self.title_sort_key(reference)),
            CompiledSortKey::Issued { .. } => {
                CachedSortValue::Issued(Self::issued_date_parts(reference))
            }
            CompiledSortKey::Field { field_name, .. } => {
                CachedSortValue::Text(Self::field_sort_value(reference, field_name))
            }
        }
    }

    fn compare_cached_references(
        &self,
        a: &CachedReference<'_>,
        b: &CachedReference<'_>,
        compiled_keys: &[CompiledSortKey<'_>],
    ) -> std::cmp::Ordering {
        self.compare_cached_values(&a.sort_values, &b.sort_values, compiled_keys)
    }

    /// Compare two precomputed sort-value sequences key by key, honoring
    /// each key's ascending/descending direction and stopping at the first
    /// non-equal comparison. Shared by [`Self::compare_cached_references`]
    /// (bibliography sorts) and [`Self::sort_by_keys`] (generic item sorts).
    fn compare_cached_values(
        &self,
        a: &[CachedSortValue],
        b: &[CachedSortValue],
        compiled_keys: &[CompiledSortKey<'_>],
    ) -> std::cmp::Ordering {
        for (index, sort_key) in compiled_keys.iter().enumerate() {
            #[allow(
                clippy::indexing_slicing,
                reason = "index is derived from compiled_keys"
            )]
            let cmp = self.compare_cached_value(&a[index], &b[index]);
            let cmp = if Self::is_ascending(sort_key) {
                cmp
            } else {
                cmp.reverse()
            };

            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }

        std::cmp::Ordering::Equal
    }

    fn compare_cached_value(&self, a: &CachedSortValue, b: &CachedSortValue) -> std::cmp::Ordering {
        match (a, b) {
            (
                CachedSortValue::RefType {
                    name: a_name,
                    rank: a_rank,
                },
                CachedSortValue::RefType {
                    name: b_name,
                    rank: b_rank,
                },
            ) => match (a_rank, b_rank) {
                (Some(a_idx), Some(b_idx)) => a_idx.cmp(b_idx),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a_name.cmp(b_name),
            },
            (CachedSortValue::OptionalText(a_text), CachedSortValue::OptionalText(b_text)) => {
                match (a_text, b_text) {
                    (Some(a_value), Some(b_value)) => self.text_collator.compare(a_value, b_value),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            }
            (CachedSortValue::Text(a_text), CachedSortValue::Text(b_text)) => {
                self.text_collator.compare(a_text, b_text)
            }
            (CachedSortValue::Issued(a_date), CachedSortValue::Issued(b_date)) => {
                compare_none_last(*a_date, *b_date)
            }
            _ => std::cmp::Ordering::Equal,
        }
    }

    fn is_ascending(sort_key: &CompiledSortKey<'_>) -> bool {
        match sort_key {
            CompiledSortKey::RefType { ascending, .. }
            | CompiledSortKey::Author { ascending, .. }
            | CompiledSortKey::Title { ascending }
            | CompiledSortKey::Issued { ascending }
            | CompiledSortKey::Field { ascending, .. } => *ascending,
        }
    }

    /// Compare by author with culturally appropriate name ordering.
    fn compare_by_author_with_order(
        &self,
        a: &Reference,
        b: &Reference,
        name_order: NameSortOrder,
    ) -> std::cmp::Ordering {
        let a_key = self.extract_author_sort_key_opt(a, name_order);
        let b_key = self.extract_author_sort_key_opt(b, name_order);
        match (a_key, b_key) {
            (Some(a), Some(b)) => self.text_collator.compare(&a, &b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    }

    /// Extract author sort key with specified name ordering.
    ///
    /// Unlike generic bibliography sorting, author-key sorting follows CSL
    /// semantics for name keys: items without author/editor names are treated
    /// as missing-name entries and sort after named entries.
    fn extract_author_sort_key_opt(
        &self,
        reference: &Reference,
        name_order: NameSortOrder,
    ) -> Option<String> {
        let default_config = Config::default();
        let config = self.config.unwrap_or(&default_config);

        if let Some(component) = self.primary_contributor_component(reference)
            && component.contributor.is_multiple()
        {
            let names = crate::values::contributor::merged::semantic_names(
                &component,
                reference,
                config,
                self.locale,
            );
            if let Some(key) = flat_names_sort_key(&names, name_order) {
                return Some(key);
            }
            // An empty merged template component (e.g. a type variant's
            // `[writer, director]` primary with neither present) falls
            // through to the effective-primary resolver below instead of
            // jumping straight to the title key, so sorting can still walk
            // the substitute chain the render path uses
            // (`merged.rs::resolve_empty_list`) and land on, say, an editor.
        }
        let substitute =
            citum_schema::options::SubstituteConfig::resolve_or_default(config.substitute.as_ref());
        let primary_key = match crate::values::contributor::substitute::effective_primary(
            reference,
            substitute.as_ref(),
            config,
            self.locale,
        ) {
            Some(crate::values::contributor::substitute::EffectivePrimary::Contributor {
                contributor,
                ..
            }) => crate::sort_support::contributor_sort_key(
                &contributor,
                name_order,
                &self.sort_key_options,
            ),
            Some(crate::values::contributor::substitute::EffectivePrimary::Merged(roles)) => {
                let names = crate::values::contributor::merged::semantic_names(
                    &citum_schema::template::TemplateContributor {
                        contributor: roles,
                        ..Default::default()
                    },
                    reference,
                    config,
                    self.locale,
                );
                flat_names_sort_key(&names, name_order)
            }
            Some(crate::values::contributor::substitute::EffectivePrimary::Title { .. }) | None => {
                None
            }
        };
        primary_key.or_else(|| {
            Some(title_sort_key_with_options(
                reference,
                self.locale,
                &self.sort_key_options,
            ))
            .filter(|key| !key.is_empty())
        })
    }

    fn primary_contributor_component(
        &self,
        reference: &Reference,
    ) -> Option<citum_schema::template::TemplateContributor> {
        if !self.primary_contributor_may_be_list {
            return None;
        }
        let language = reference.language().map(|language| language.to_string());
        match self.primary_contributor_spec.as_ref()? {
            PrimaryContributorSpec::Citation(spec) => {
                primary_contributor_for_citation(spec, reference)
            }
            PrimaryContributorSpec::Bibliography(spec) => {
                primary_contributor_for_bibliography(spec, reference, language.as_deref())
            }
        }
    }

    /// Public helper retained for tests/debugging.
    #[must_use]
    pub fn extract_author_sort_key(
        &self,
        reference: &Reference,
        name_order: NameSortOrder,
    ) -> String {
        self.extract_author_sort_key_opt(reference, name_order)
            .unwrap_or_default()
    }

    /// Compare by title (with article stripping).
    fn compare_by_title(&self, a: &Reference, b: &Reference) -> std::cmp::Ordering {
        let a_title = self.title_sort_key(a);
        let b_title = self.title_sort_key(b);
        self.text_collator.compare(&a_title, &b_title)
    }

    /// Compare by full issued date (year, month, day).
    fn compare_by_issued(a: &Reference, b: &Reference) -> std::cmp::Ordering {
        compare_none_last(Self::issued_date_parts(a), Self::issued_date_parts(b))
    }

    /// Compare by custom field.
    fn compare_by_field(a: &Reference, b: &Reference, field_name: &str) -> std::cmp::Ordering {
        Self::field_sort_value(a, field_name).cmp(&Self::field_sort_value(b, field_name))
    }

    fn title_sort_key(&self, reference: &Reference) -> String {
        title_sort_key_with_options(reference, self.locale, &self.sort_key_options)
    }

    /// Full (year, month, day) for sort comparison. A missing month or day
    /// defaults to `0`, sorting before any real value - the less precise a
    /// date is, the earlier it's treated, matching EDTF's own
    /// could-be-anywhere-in-range convention for reduced precision.
    fn issued_date_parts(reference: &Reference) -> Option<(i32, u32, u32)> {
        let date = reference.effective_issued_date()?;
        let (year, month, day) = date.date_parts()?;
        let year = i32::try_from(year).ok().filter(|year| *year != 0)?;
        Some((year, month.unwrap_or(0), day.unwrap_or(0)))
    }

    fn field_sort_value(reference: &Reference, field_name: &str) -> String {
        match field_name {
            "language" => normalize_sort_text(reference.language().unwrap_or_default().as_ref()),
            // Future: support for keywords, custom metadata
            _ => String::new(),
        }
    }
}

fn first_contributor_component_ref(
    template: &[citum_schema::template::TemplateComponent],
) -> Option<&citum_schema::template::TemplateContributor> {
    for component in template {
        match component {
            citum_schema::template::TemplateComponent::Contributor(contributor) => {
                return Some(contributor);
            }
            citum_schema::template::TemplateComponent::Group(group) => {
                if let Some(contributor) = first_contributor_component_ref(&group.group) {
                    return Some(contributor);
                }
            }
            _ => {}
        }
    }
    None
}

fn first_contributor_component(
    template: &[citum_schema::template::TemplateComponent],
) -> Option<citum_schema::template::TemplateContributor> {
    first_contributor_component_ref(template).cloned()
}

fn template_has_list_primary(template: &[citum_schema::template::TemplateComponent]) -> bool {
    first_contributor_component_ref(template)
        .is_some_and(|component| component.contributor.is_multiple())
}

fn variants_may_have_list_primary(variants: &citum_schema::template::TemplateVariants) -> bool {
    variants
        .values()
        .any(|variant| variant.as_template().is_none_or(template_has_list_primary))
}

/// Return whether any template reachable from this citation spec (base,
/// locale overrides, type variants, or integral/non-integral/subsequent/ibid
/// forms) declares a merged-list primary contributor component.
pub(crate) fn citation_may_have_list_primary(spec: &citum_schema::CitationSpec) -> bool {
    spec.template
        .as_ref()
        .and_then(citum_schema::template::TemplateVariant::as_template)
        .is_some_and(template_has_list_primary)
        || spec
            .template_ref
            .as_ref()
            .and_then(citum_schema::template::TemplateReference::citation_template)
            .as_deref()
            .is_some_and(template_has_list_primary)
        || spec.locales.as_ref().is_some_and(|locales| {
            locales
                .iter()
                .any(|localized| template_has_list_primary(&localized.template))
        })
        || spec
            .type_variants
            .as_ref()
            .is_some_and(variants_may_have_list_primary)
        || spec
            .integral
            .as_deref()
            .is_some_and(citation_may_have_list_primary)
        || spec
            .non_integral
            .as_deref()
            .is_some_and(citation_may_have_list_primary)
        || spec
            .subsequent
            .as_deref()
            .is_some_and(citation_may_have_list_primary)
        || spec
            .ibid
            .as_deref()
            .is_some_and(citation_may_have_list_primary)
}

fn bibliography_may_have_list_primary(spec: &citum_schema::BibliographySpec) -> bool {
    spec.template
        .as_ref()
        .and_then(citum_schema::template::TemplateVariant::as_template)
        .is_some_and(template_has_list_primary)
        || spec
            .template_ref
            .as_ref()
            .and_then(citum_schema::template::TemplateReference::bibliography_template)
            .as_deref()
            .is_some_and(template_has_list_primary)
        || spec.locales.as_ref().is_some_and(|locales| {
            locales
                .iter()
                .any(|localized| template_has_list_primary(&localized.template))
        })
        || spec
            .type_variants
            .as_ref()
            .is_some_and(variants_may_have_list_primary)
}

/// Resolve the first contributor component from a reference's effective citation template.
pub(crate) fn primary_contributor_for_citation(
    spec: &citum_schema::CitationSpec,
    reference: &Reference,
) -> Option<citum_schema::template::TemplateContributor> {
    let language = reference.language().map(|language| language.to_string());
    let template = spec.resolve_template_for_type(&reference.ref_type(), language.as_deref())?;
    first_contributor_component(&template)
}

fn primary_contributor_for_bibliography(
    spec: &citum_schema::BibliographySpec,
    reference: &Reference,
    language: Option<&str>,
) -> Option<citum_schema::template::TemplateContributor> {
    let template = spec.resolve_template_for_type(&reference.ref_type(), language)?;
    first_contributor_component(&template)
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
    use citum_schema::grouping::GroupSortKey;
    use citum_schema::options::{MultilingualConfig, SortingConfig, SortingMultilingualMode};
    use citum_schema::reference::contributor::MultilingualName;
    use citum_schema::reference::types::MultilingualComplex;
    use citum_schema::reference::{
        Contributor, ContributorList, DateValue, Monograph, MonographType, MultilingualString,
        StructuredName, Title,
    };
    use std::collections::HashMap;

    fn make_locale() -> Locale {
        Locale::en_us()
    }

    fn make_reference(
        id: &str,
        ref_type: &str,
        author_family: &str,
        title: &str,
        year: i32,
    ) -> Reference {
        let json = serde_json::json!({
            "id": id,
            "type": ref_type,
            "author": [{"family": author_family, "given": "Test"}],
            "issued": {"date-parts": [[year]]},
            "title": title,
            "container-title": "Test Container",
        });
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(json).unwrap();
        legacy.into()
    }

    fn make_reference_no_author(id: &str, ref_type: &str, title: &str, year: i32) -> Reference {
        let json = serde_json::json!({
            "id": id,
            "type": ref_type,
            "issued": {"date-parts": [[year]]},
            "title": title,
            "container-title": "Test Container",
        });
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_value(json).unwrap();
        legacy.into()
    }

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

    fn multilingual_author_reference(
        id: &str,
        original_family: MultilingualString,
        sort_as: Option<&str>,
        transliteration_family: Option<&str>,
        title: Title,
    ) -> Reference {
        let transliterations = transliteration_family.map_or_else(HashMap::new, |family| {
            HashMap::from([(
                "ru-Latn-alalc97".to_string(),
                StructuredName {
                    family: family.into(),
                    given: "Lev".into(),
                    ..Default::default()
                },
            )])
        });

        Reference::Monograph(Box::new(Monograph {
            id: Some(id.into()),
            r#type: MonographType::Book,
            title: Some(title),
            author: Some(Contributor::ContributorList(ContributorList(vec![
                Contributor::Multilingual(MultilingualName {
                    original: StructuredName {
                        family: original_family,
                        given: "Лев".into(),
                        ..Default::default()
                    },
                    lang: Some("ru".into()),
                    sort_as: sort_as.map(str::to_string),
                    transliterations,
                    translations: HashMap::new(),
                }),
            ]))),
            issued: DateValue::new("1869".to_string()),
            ..Default::default()
        }))
    }

    fn title_sort(sorter: &ReferenceSorter<'_>, references: Vec<&Reference>) -> Vec<String> {
        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Title,
                ascending: true,
                order: None,
                sort_order: None,
            }],
        };

        sorter
            .sort_references(references, &sort_spec)
            .into_iter()
            .map(|reference| reference.id().expect("test reference id").to_string())
            .collect()
    }

    #[test]
    fn test_type_order_sorting() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        // Use standard CSL JSON types for testing
        let journal = make_reference("r1", "article-journal", "Smith", "Title J", 1990);
        let magazine = make_reference("r2", "article-magazine", "Jones", "Title M", 2000);
        let newspaper = make_reference("r3", "article-newspaper", "Brown", "Title N", 1985);
        let book = make_reference("r4", "book", "Davis", "Title B", 1995);

        let mut refs = vec![&book, &newspaper, &journal, &magazine];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::RefType,
                ascending: true,
                order: Some(vec![
                    "article-journal".to_string(),
                    "article-magazine".to_string(),
                    "article-newspaper".to_string(),
                ]),
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be: article-journal, article-magazine, article-newspaper, then book (alphabetically after)
        assert_eq!(refs[0].id().unwrap(), "r1"); // article-journal
        assert_eq!(refs[1].id().unwrap(), "r2"); // article-magazine
        assert_eq!(refs[2].id().unwrap(), "r3"); // article-newspaper
        assert_eq!(refs[3].id().unwrap(), "r4"); // book
    }

    #[test]
    fn test_author_family_given_order() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let smith = make_reference("r1", "book", "Smith", "Title", 2000);
        let jones = make_reference("r2", "book", "Jones", "Title", 2000);
        let brown = make_reference("r3", "book", "Brown", "Title", 2000);

        let mut refs = vec![&smith, &jones, &brown];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Author,
                ascending: true,
                order: None,
                sort_order: Some(NameSortOrder::FamilyGiven),
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be alphabetical by family name
        assert_eq!(refs[0].id().unwrap(), "r3"); // Brown
        assert_eq!(refs[1].id().unwrap(), "r2"); // Jones
        assert_eq!(refs[2].id().unwrap(), "r1"); // Smith
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_author_sort_uses_unicode_collation_for_accented_names() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let celik = make_reference("r1", "book", "Çelik", "Title", 2000);
        let zimring = make_reference("r2", "book", "Zimring", "Title", 2000);
        let o_tuathail = make_reference("r3", "book", "Ó Tuathail", "Title", 2000);

        let mut refs = vec![&o_tuathail, &zimring, &celik];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Author,
                ascending: true,
                order: None,
                sort_order: Some(NameSortOrder::FamilyGiven),
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        assert_eq!(refs[0].id().unwrap(), "r1");
        assert_eq!(refs[1].id().unwrap(), "r3");
        assert_eq!(refs[2].id().unwrap(), "r2");
    }

    #[test]
    #[cfg(feature = "icu")]
    fn test_title_sort_uses_unicode_collation_for_accented_titles() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let accent = make_reference_no_author("r1", "book", "Órbitas del sur", 2000);
        let plain = make_reference_no_author("r2", "book", "Origins of Theory", 2000);
        let zeta = make_reference_no_author("r3", "book", "Zebra Studies", 2000);

        let mut refs = vec![&zeta, &plain, &accent];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Title,
                ascending: true,
                order: None,
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        assert_eq!(refs[0].id().unwrap(), "r1");
        assert_eq!(refs[1].id().unwrap(), "r2");
        assert_eq!(refs[2].id().unwrap(), "r3");
    }

    #[test]
    fn test_issued_descending() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let old = make_reference("r1", "book", "Smith", "Title", 1990);
        let new = make_reference("r2", "book", "Jones", "Title", 2020);
        let mid = make_reference("r3", "book", "Brown", "Title", 2005);

        let mut refs = vec![&old, &new, &mid];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Issued,
                ascending: false, // Descending
                order: None,
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be newest first
        assert_eq!(refs[0].id().unwrap(), "r2"); // 2020
        assert_eq!(refs[1].id().unwrap(), "r3"); // 2005
        assert_eq!(refs[2].id().unwrap(), "r1"); // 1990
    }

    #[test]
    fn test_issued_ascending_places_undated_last() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let dated_early = make_reference("r1", "book", "Smith", "Book D", 1999);
        let dated_late = make_reference("r2", "book", "Jones", "Book B", 2000);
        let mut undated = make_reference("r3", "book", "Brown", "Book A", 2000);
        if let ClassExtension::Monograph(monograph) = undated.extension_mut() {
            monograph.issued = citum_schema::reference::DateValue::new(String::new());
        }

        let mut refs = vec![&undated, &dated_late, &dated_early];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Issued,
                ascending: true,
                order: None,
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        assert_eq!(refs[0].id().unwrap(), "r1");
        assert_eq!(refs[1].id().unwrap(), "r2");
        assert_eq!(refs[2].id().unwrap(), "r3");
    }

    #[test]
    fn test_issued_sort_uses_created_when_issued_is_missing() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let dated = make_reference("r1", "book", "Smith", "Book D", 1999);
        let mut created_only = make_reference("r2", "book", "Jones", "Book C", 2000);
        if let ClassExtension::Monograph(monograph) = created_only.extension_mut() {
            monograph.created = citum_schema::reference::DateValue::new("1985".to_string());
            monograph.issued = citum_schema::reference::DateValue::new(String::new());
        }

        let mut refs = vec![&dated, &created_only];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Issued,
                ascending: true,
                order: None,
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        assert_eq!(refs[0].id().unwrap(), "r2");
        assert_eq!(refs[1].id().unwrap(), "r1");
    }

    #[test]
    fn test_composite_sort() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let smith2020 = make_reference("r1", "book", "Smith", "Title", 2020);
        let smith2010 = make_reference("r2", "book", "Smith", "Title", 2010);
        let jones2020 = make_reference("r3", "book", "Jones", "Title", 2020);

        let mut refs = vec![&smith2020, &jones2020, &smith2010];

        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: GroupSortKeyType::Author,
                    ascending: true,
                    order: None,
                    sort_order: Some(NameSortOrder::FamilyGiven),
                },
                GroupSortKey {
                    key: GroupSortKeyType::Issued,
                    ascending: false, // Descending within author
                    order: None,
                    sort_order: None,
                },
            ],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Should be: Jones 2020, then Smith 2020, then Smith 2010
        assert_eq!(refs[0].id().unwrap(), "r3"); // Jones 2020
        assert_eq!(refs[1].id().unwrap(), "r1"); // Smith 2020
        assert_eq!(refs[2].id().unwrap(), "r2"); // Smith 2010
    }

    #[test]
    fn test_author_sort_falls_back_to_title_for_missing_names() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let no_author = make_reference_no_author("r1", "legal-case", "Brown v. Board", 1954);
        let brown = make_reference("r2", "book", "Brown", "Title", 2000);
        let smith = make_reference("r3", "book", "Smith", "Title", 2000);

        let mut refs = vec![&no_author, &smith, &brown];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Author,
                ascending: true,
                order: None,
                sort_order: Some(NameSortOrder::FamilyGiven),
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        assert_eq!(refs[0].id().unwrap(), "r2"); // Brown
        assert_eq!(refs[1].id().unwrap(), "r1"); // Brown v. Board
        assert_eq!(refs[2].id().unwrap(), "r3"); // Smith
    }

    #[test]
    fn test_legal_citation_sort() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let case_a = make_reference("r1", "legal-case", "", "Doe v. Smith", 1990);
        let case_b = make_reference("r2", "legal-case", "", "Brown v. Board", 1954);

        let mut refs = vec![&case_a, &case_b];

        let sort_spec = GroupSort {
            template: vec![
                GroupSortKey {
                    key: GroupSortKeyType::Title, // Case name
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: GroupSortKeyType::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
        };

        refs = sorter.sort_references(refs, &sort_spec);
        assert_eq!(refs[0].id().unwrap(), "r2"); // Brown v. Board
    }

    #[test]
    fn test_legal_hierarchy_sort() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let statute = make_reference("r1", "statute", "", "Clean Air Act", 1970);
        let case = make_reference("r2", "legal-case", "", "Roe v. Wade", 1973);
        let treaty = make_reference("r3", "treaty", "", "Paris Agreement", 2015);

        let mut refs = vec![&treaty, &case, &statute];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::RefType,
                ascending: true,
                order: Some(vec![
                    "legal-case".to_string(),
                    "statute".to_string(),
                    "treaty".to_string(),
                ]),
                sort_order: None,
            }],
        };

        refs = sorter.sort_references(refs, &sort_spec);

        // Hierarchy: case, statute, treaty
        assert_eq!(refs[0].id().unwrap(), "r2");
        assert_eq!(refs[1].id().unwrap(), "r1");
        assert_eq!(refs[2].id().unwrap(), "r3");
    }

    /// Given references whose sort keys are all equal, when sorted with the
    /// ID tiebreak, then they come out in ID order.
    #[test]
    fn test_id_tiebreak_orders_equal_keys_by_id() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let c = make_reference("r-c", "book", "Smith", "Same Title", 2000);
        let a = make_reference("r-a", "book", "Smith", "Same Title", 2000);
        let b = make_reference("r-b", "book", "Smith", "Same Title", 2000);

        let refs = vec![&c, &a, &b];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Author,
                ascending: true,
                order: None,
                sort_order: Some(NameSortOrder::FamilyGiven),
            }],
        };

        let sorted = sorter.sort_references_with_id_tiebreak(refs, &sort_spec);

        assert_eq!(sorted[0].id().unwrap(), "r-a");
        assert_eq!(sorted[1].id().unwrap(), "r-b");
        assert_eq!(sorted[2].id().unwrap(), "r-c");
    }

    /// Given one reference with no ID and one with equal sort keys, when
    /// sorted with the ID tiebreak, then the ID-less reference sorts last.
    #[test]
    fn test_id_tiebreak_places_missing_id_last() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let with_id = make_reference("r1", "book", "Smith", "Same Title", 2000);
        let mut no_id = make_reference("r2", "book", "Smith", "Same Title", 2000);
        if let ClassExtension::Monograph(monograph) = no_id.extension_mut() {
            monograph.id = None;
        }

        let refs = vec![&with_id, &no_id];

        let sort_spec = GroupSort {
            template: vec![GroupSortKey {
                key: GroupSortKeyType::Author,
                ascending: true,
                order: None,
                sort_order: Some(NameSortOrder::FamilyGiven),
            }],
        };

        let sorted = sorter.sort_references_with_id_tiebreak(refs, &sort_spec);

        assert_eq!(sorted[0].id().unwrap(), "r1");
        assert!(sorted[1].id().is_none());
    }

    /// Given an empty sort template, when sorted with the ID tiebreak, then
    /// references keep registry order instead of being reordered by ID alone.
    #[test]
    fn test_id_tiebreak_with_empty_template_keeps_registry_order() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);

        let c = make_reference("r-c", "book", "Smith", "Title C", 2000);
        let a = make_reference("r-a", "book", "Jones", "Title A", 2001);

        let refs = vec![&c, &a];
        let sort_spec = GroupSort { template: vec![] };

        let sorted = sorter.sort_references_with_id_tiebreak(refs, &sort_spec);

        assert_eq!(sorted[0].id().unwrap(), "r-c");
        assert_eq!(sorted[1].id().unwrap(), "r-a");
    }

    #[test]
    fn romanized_author_sort_uses_hidden_sort_as() {
        let locale = make_locale();
        let config = romanized_config();
        let sorter = ReferenceSorter::with_bibliography_config(&locale, &config);
        let reference = multilingual_author_reference(
            "tolstoy",
            "Толстой".into(),
            Some("Tolstoy"),
            Some("Tolstoĭ"),
            Title::Single("War and Peace".to_string()),
        );

        assert_eq!(
            sorter.extract_author_sort_key(&reference, NameSortOrder::FamilyGiven),
            "Tolstoy"
        );
    }

    #[test]
    fn uniform_author_sort_ignores_hidden_sort_as() {
        let locale = make_locale();
        let sorter = ReferenceSorter::new(&locale);
        let reference = multilingual_author_reference(
            "tolstoy",
            "Толстой".into(),
            Some("Tolstoy"),
            Some("Tolstoĭ"),
            Title::Single("War and Peace".to_string()),
        );

        assert_eq!(
            sorter.extract_author_sort_key(&reference, NameSortOrder::FamilyGiven),
            "Толстой"
        );
    }

    #[test]
    fn romanized_author_sort_falls_back_to_matched_transliteration() {
        let locale = make_locale();
        let config = romanized_config();
        let sorter = ReferenceSorter::with_bibliography_config(&locale, &config);
        let reference = multilingual_author_reference(
            "tolstoy",
            "Толстой".into(),
            None,
            Some("Tolstoĭ"),
            Title::Single("War and Peace".to_string()),
        );

        assert_eq!(
            sorter.extract_author_sort_key(&reference, NameSortOrder::FamilyGiven),
            "Tolstoĭ"
        );
    }

    #[test]
    fn holistic_sort_as_wins_over_part_level_sort_as() {
        let locale = make_locale();
        let config = romanized_config();
        let sorter = ReferenceSorter::with_bibliography_config(&locale, &config);
        let family = MultilingualString::Complex(MultilingualComplex {
            original: "Толстой".to_string(),
            lang: Some("ru".into()),
            sort_as: Some("Part Level".to_string()),
            transliterations: HashMap::new(),
            translations: HashMap::new(),
        });
        let reference = multilingual_author_reference(
            "tolstoy",
            family,
            Some("Whole Name"),
            Some("Tolstoĭ"),
            Title::Single("War and Peace".to_string()),
        );

        assert_eq!(
            sorter.extract_author_sort_key(&reference, NameSortOrder::FamilyGiven),
            "Whole Name"
        );
    }

    #[test]
    fn title_sort_uses_hidden_sort_as_under_romanized_mode() {
        let locale = make_locale();
        let config = romanized_config();
        let sorter = ReferenceSorter::with_bibliography_config(&locale, &config);
        let cyrillic = multilingual_author_reference(
            "cyrillic-title",
            "Smith".into(),
            None,
            None,
            Title::Multilingual(MultilingualComplex {
                original: "Война и мир".to_string(),
                lang: Some("ru".into()),
                sort_as: Some("Academic War and Peace".to_string()),
                transliterations: HashMap::new(),
                translations: HashMap::new(),
            }),
        );
        let latin = make_reference_no_author("latin-title", "book", "Beta Studies", 2000);

        assert_eq!(
            title_sort(&sorter, vec![&latin, &cyrillic]),
            vec!["cyrillic-title".to_string(), "latin-title".to_string()]
        );
    }
}
