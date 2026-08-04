/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for citation and bibliography output.
//!
//! This module handles template-based rendering of citations and bibliographies,
//! including handling of localization, numbering, formatting, and special modes
//! like integral (narrative) citations for numeric and label styles.

use crate::error::ProcessorError;
use crate::reference::{Bibliography, Reference};
use crate::values::{ProcHints, RenderContext, RenderOptions};
use citum_schema::citation::CitationLocator;
use citum_schema::locale::Locale;
use citum_schema::options::{
    CitationLabelMode, Config, LabelWrap, bibliography::BibliographyConfig,
};
use citum_schema::template::TemplateComponent;
use grouped::component_predicates::{resolve_localized_type_variant, resolve_type_variant};
use indexmap::IndexMap;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};

fn embedded_render_locales() -> &'static HashMap<String, Locale> {
    static LOCALES: OnceLock<HashMap<String, Locale>> = OnceLock::new();
    LOCALES.get_or_init(|| {
        let mut locales = HashMap::new();
        for id in citum_schema::embedded::EMBEDDED_LOCALE_IDS {
            let Some(locale) = citum_schema::embedded::get_locale(id) else {
                continue;
            };
            locales.insert(id.to_ascii_lowercase(), locale);
        }
        locales
    })
}

/// Look up a loaded embedded locale exactly matching `locale_id`, falling
/// back to another loaded locale sharing the same primary BCP 47 subtag
/// (e.g. `de-AT` falls back to a loaded `de-DE`). Returns `None` when
/// neither matches; the caller decides the ultimate fallback (the style
/// locale, per `docs/specs/PER_ITEM_TERM_LOCALE.md` §3 and
/// `MULTILINGUAL.md` §3.4).
pub(crate) fn lookup_embedded_locale(locale_id: &str) -> Option<&'static Locale> {
    let locales = embedded_render_locales();
    let key = locale_id.to_ascii_lowercase();
    locales.get(&key).or_else(|| {
        let primary = key.split(['-', '_']).next()?;
        locales.iter().find_map(|(candidate, locale)| {
            candidate
                .split(['-', '_'])
                .next()
                .is_some_and(|candidate_primary| candidate_primary == primary)
                .then_some(locale)
        })
    })
}

/// The renderer for citation and bibliography templates.
///
/// The `Renderer` is responsible for taking compiled templates and applying them
/// to bibliographic data, handling localization, numbering, and formatting.
pub struct Renderer<'a> {
    /// The style definition containing templates and options.
    pub style: &'a citum_schema::Style,
    /// The bibliography containing the reference data.
    pub bibliography: &'a Bibliography,
    /// The locale used for terms and formatting.
    pub locale: &'a Locale,
    /// The active configuration options.
    pub config: Arc<Config>,
    /// The active bibliography-only configuration.
    pub bibliography_config: Option<Arc<BibliographyConfig>>,
    /// Pre-calculated hints for optimization.
    pub hints: &'a HashMap<String, ProcHints>,
    /// Shared state for citation numbers (used in numeric styles).
    ///
    /// `RwLock`, not `RefCell`: bibliography entries render in parallel
    /// (behind the `parallel` feature) once above `PARALLEL_MIN_ENTRIES`,
    /// and each per-entry `Renderer` borrows this same run-scoped map.
    pub citation_numbers: &'a RwLock<HashMap<String, usize>>,
    /// Optional compound set membership indexed by reference id.
    pub compound_set_by_ref: &'a HashMap<String, String>,
    /// Optional 0-based member index within each compound set.
    pub compound_member_index: &'a HashMap<String, usize>,
    /// Compound sets keyed by set id.
    pub compound_sets: &'a IndexMap<String, Vec<String>>,
    /// Whether to output semantic markup (HTML spans, Djot attributes).
    pub show_semantics: bool,
    /// Whether to attach source template indices to rendered semantic wrappers.
    pub inject_ast_indices: bool,
    /// Mapping from filtered to original template indices (for grouped citations).
    pub filtered_to_original_index: RefCell<Option<Vec<usize>>>,
    /// Document-level abbreviation map for post-render substitution.
    pub abbreviation_map: Option<&'a crate::api::AbbreviationMap>,
    /// First note number per reference id (populated by normalize_note_context).
    pub first_note_by_id: Option<&'a RwLock<HashMap<String, u32>>>,
}

/// Borrowed compound-set context for rendering.
pub struct CompoundRenderData<'a> {
    /// Optional compound set membership indexed by reference id.
    pub set_by_ref: &'a HashMap<String, String>,
    /// Optional 0-based member index within each compound set.
    pub member_index: &'a HashMap<String, usize>,
    /// Compound sets keyed by set id.
    pub sets: &'a IndexMap<String, Vec<String>>,
}

mod collapse;
mod grouped;
mod grouped_fallback;
mod helpers;
mod marker;

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

pub use grouped_fallback::GroupRenderParams;
pub use grouped_fallback::TemplateRenderParams;
pub(super) use helpers::{
    find_grouping_component, has_contributor_component, leading_group_affix,
    remove_first_contributor_with_role, strip_author_component, strip_leading_group_affixes,
};

/// Internal render request used to keep template-processing call sites compact.
pub struct TemplateRenderRequest<'a> {
    /// The template to render.
    pub template: &'a [TemplateComponent],
    /// The rendering context (Citation or Bibliography).
    pub context: RenderContext,
    /// The citation mode (Integral or `NonIntegral`).
    pub mode: citum_schema::citation::CitationMode,
    /// Whether to suppress the author in output.
    pub suppress_author: bool,
    /// The raw citation locator if present (for new rendering logic).
    pub locator_raw: Option<&'a CitationLocator>,
    /// The citation number for numeric styles.
    pub citation_number: usize,
    /// The citation position (e.g., Ibid).
    pub position: Option<citum_schema::citation::Position>,
    /// Optional note-start text-case policy for note-style repeated-note output.
    pub note_start_text_case: Option<citum_schema::NoteStartTextCase>,
    /// Integral name state for name formatting.
    pub integral_name_state: Option<citum_schema::citation::IntegralNameState>,
    /// Org abbreviation state for org-name formatting.
    pub org_abbreviation_state: Option<citum_schema::citation::IntegralNameState>,
    /// First note number for this reference (note styles, subsequent position).
    pub first_reference_note_number: Option<u32>,
}

/// Per-item state resolved for rendering one ungrouped citation item.
struct UngroupedItemRenderState<'a> {
    reference: &'a Reference,
    template: Cow<'a, [TemplateComponent]>,
    delimiter: &'a str,
    /// The reference marker this item renders, if the style declares one.
    marker: Option<marker::CitationMarkerSpec>,
}

/// A resolved reference marker: its value plus how to present it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedMarker {
    /// The generated token.
    pub value: marker::MarkerValue,
    /// How the marker is placed and wrapped.
    pub spec: marker::CitationMarkerSpec,
}

/// A rendered citation item plus the metadata needed for semantic collapsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CitationChunk {
    /// Reference IDs represented by this chunk.
    pub ids: Vec<String>,
    /// Rendered chunk content before final numeric-label presentation.
    pub content: String,
    /// The reference marker, when this chunk is exactly its marker and so can
    /// still take part in numeric collapse. Presentation is realized after
    /// collapse; a chunk that has already been composed carries `None`.
    pub marker: Option<ResolvedMarker>,
}

/// Shared, citation-wide parameters threaded into each ungrouped item render.
#[derive(Clone, Copy)]
struct UngroupedItemRenderParams<'a> {
    mode: &'a citum_schema::citation::CitationMode,
    suppress_author: bool,
    position: Option<&'a citum_schema::citation::Position>,
    note_start_text_case: Option<citum_schema::NoteStartTextCase>,
}

#[derive(Clone, Default)]
struct TemplateComponentTracker {
    rendered_vars: HashSet<String>,
    substituted_bases: HashSet<String>,
}

impl TemplateComponentTracker {
    fn should_skip(&self, var_key: Option<&str>) -> bool {
        let Some(var_key) = var_key else {
            return false;
        };
        let base = key_base(var_key);
        self.rendered_vars.contains(var_key) || self.substituted_bases.contains(base.as_ref())
    }

    fn mark_rendered(&mut self, var_key: Option<String>, substituted_key: Option<&str>) {
        if let Some(var_key) = var_key {
            self.rendered_vars.insert(var_key);
        }
        if let Some(substituted_key) = substituted_key {
            self.rendered_vars.insert(substituted_key.to_string());
            self.substituted_bases
                .insert(key_base(substituted_key).into_owned());
        }
    }

    fn merge_from(&mut self, other: Self) {
        self.rendered_vars.extend(other.rendered_vars);
        self.substituted_bases.extend(other.substituted_bases);
    }
}

/// Core style resources borrowed by every [`Renderer`] instance.
///
/// Bundles the four immutable resolution inputs so that [`Renderer::new`] stays
/// within clippy's argument-count limit.
pub struct RendererResources<'a> {
    /// The style definition containing templates and options.
    pub style: &'a citum_schema::Style,
    /// The bibliography containing the reference data.
    pub bibliography: &'a Bibliography,
    /// The locale used for terms and formatting.
    pub locale: &'a Locale,
    /// The active configuration options.
    pub config: Arc<Config>,
    /// The active bibliography-only configuration.
    pub bibliography_config: Option<Arc<BibliographyConfig>>,
    /// First note number per reference id (note styles; `None` for bibliography rendering).
    pub first_note_by_id: Option<&'a RwLock<HashMap<String, u32>>>,
}

impl<'a> Renderer<'a> {
    /// Creates a new `Renderer` instance.
    pub fn new(
        resources: RendererResources<'a>,
        hints: &'a HashMap<String, ProcHints>,
        citation_numbers: &'a RwLock<HashMap<String, usize>>,
        compound: CompoundRenderData<'a>,
        show_semantics: bool,
        inject_ast_indices: bool,
        abbreviation_map: Option<&'a crate::api::AbbreviationMap>,
    ) -> Self {
        Self {
            style: resources.style,
            bibliography: resources.bibliography,
            locale: resources.locale,
            config: resources.config,
            bibliography_config: resources.bibliography_config,
            hints,
            citation_numbers,
            compound_set_by_ref: compound.set_by_ref,
            compound_member_index: compound.member_index,
            compound_sets: compound.sets,
            show_semantics,
            inject_ast_indices,
            filtered_to_original_index: RefCell::new(None),
            abbreviation_map,
            first_note_by_id: resources.first_note_by_id,
        }
    }

    /// Select the rendering locale for one reference.
    ///
    /// A matched `citation.locales[]`/`bibliography.locales[]` branch is
    /// authoritative and returns its embedded locale unchanged (structure
    /// and rendering locale, including typography, both come from the
    /// branch). Otherwise, under `options.multilingual.term-locale: item`,
    /// returns a hybrid locale that speaks the item's terms/dates inside the
    /// style's typography (see `docs/specs/PER_ITEM_TERM_LOCALE.md`); an
    /// item language with no loaded locale falls back to the style locale
    /// silently here — [`crate::api::warnings::term_locale_fallback_warnings`]
    /// surfaces that case as a diagnostic. Otherwise returns the style
    /// locale, today's default behavior byte for byte.
    fn locale_for_reference(
        &self,
        reference: &Reference,
        context: RenderContext,
    ) -> Cow<'a, Locale> {
        let language = crate::values::effective_item_language(reference);
        let selected = match context {
            RenderContext::Citation => self
                .style
                .citation
                .as_ref()
                .and_then(|spec| spec.resolve_localized_template(language.as_deref())),
            RenderContext::Bibliography => self
                .style
                .bibliography
                .as_ref()
                .and_then(|spec| spec.resolve_localized_template(language.as_deref())),
        };

        if let Some(locale_id) = selected.and_then(|resolved| resolved.locale) {
            return Cow::Borrowed(lookup_embedded_locale(&locale_id).unwrap_or(self.locale));
        }

        let term_locale_is_item = self
            .config
            .multilingual
            .as_ref()
            .is_some_and(|ml| ml.term_locale == citum_schema::options::TermLocale::Item);

        if term_locale_is_item
            && let Some(item_locale) = language.as_deref().and_then(lookup_embedded_locale)
        {
            return Cow::Owned(self.locale.with_term_surfaces_from(item_locale));
        }

        Cow::Borrowed(self.locale)
    }

    /// Resolve multilingual contributor names using the style's config.
    fn resolve_contributor_names(
        &self,
        contributor: &citum_schema::reference::contributor::Contributor,
    ) -> Vec<crate::reference::FlatName> {
        let ml = self.config.multilingual.as_ref();
        crate::values::resolve_multilingual_name(
            contributor,
            ml.and_then(|m| m.name_mode.as_ref()),
            ml.and_then(|m| m.preferred_transliteration.as_deref()),
            ml.and_then(|m| m.preferred_script.as_ref()),
            &self.locale.locale,
        )
    }

    /// Generate an alphabetic or numeric sub-label (e.g., "a", "1") for a
    /// reference member of a compound set.
    fn citation_sub_label_for_ref(&self, ref_id: &str) -> Option<String> {
        let compound = self
            .bibliography_config
            .as_ref()
            .and_then(|b| b.compound_numeric.as_ref())?;
        let set_id = self.compound_set_by_ref.get(ref_id)?;
        let members = self.compound_sets.get(set_id)?;
        if members.len() <= 1 {
            return None;
        }
        if !compound.subentry {
            return None;
        }
        let idx = *self.compound_member_index.get(ref_id)?;
        match compound.sub_label {
            citum_schema::options::bibliography::SubLabelStyle::Alphabetic => {
                crate::values::int_to_letter((idx + 1) as u32)
            }
            citum_schema::options::bibliography::SubLabelStyle::Numeric => {
                Some((idx + 1).to_string())
            }
        }
    }

    /// Resolve the effective declarative citation label mode for one citation spec.
    fn citation_label_mode(&self, spec: &citum_schema::CitationSpec) -> Option<CitationLabelMode> {
        marker::citation_label_mode(&self.config, spec)
    }

    /// Apply a citation label wrapper after semantic numeric collapse.
    fn wrap_citation_label_with_format<F>(
        &self,
        fmt: &F,
        content: String,
        wrap: Option<LabelWrap>,
        ref_id: Option<&str>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let Some(wrap) = wrap else {
            return content;
        };
        if wrap == LabelWrap::None {
            return content;
        }
        if wrap == LabelWrap::Superscript {
            return fmt.superscript(content);
        }
        let language = ref_id
            .and_then(|id| self.bibliography.get(id))
            .and_then(crate::values::effective_item_language);
        let (script, realization) = crate::values::punctuation_realization_context(
            language.as_deref(),
            self.config.multilingual.as_ref(),
            self.locale.punctuation_realization.as_ref(),
        );
        let marks = crate::render::format::QuoteMarks::from(&self.locale.grammar_options);
        let Some(config) = wrap.as_wrap_config() else {
            return content;
        };
        fmt.wrap_punctuation(
            &config.punctuation,
            content,
            &marks,
            script,
            realization.as_deref(),
        )
    }

    /// Determines if the processor should render author-plus-number text for a numeric style
    /// when in "integral" (narrative) citation mode.
    ///
    /// This happens when the style is numeric and the user requests a narrative
    /// citation (e.g., "Smith [1]"), but hasn't provided an explicit narrative template.
    fn should_render_author_number_for_numeric_integral(
        &self,
        mode: &citum_schema::citation::CitationMode,
    ) -> bool {
        matches!(mode, citum_schema::citation::CitationMode::Integral)
            && self.config.processing.as_ref().is_some_and(|processing| {
                matches!(processing, citum_schema::options::Processing::Numeric)
            })
            && !self.has_explicit_integral_template()
    }

    /// Whether the style provides an explicit integral (narrative) template.
    fn has_explicit_integral_template(&self) -> bool {
        self.style.citation.as_ref().is_some_and(|c| {
            c.integral.as_ref().is_some_and(|i| {
                i.template.is_some() || i.template_ref.is_some() || i.locales.is_some()
            })
        })
    }

    /// Determine if compound subentries should be collapsed for this citation.
    fn should_collapse_compound_subentries(
        &self,
        mode: &citum_schema::citation::CitationMode,
    ) -> bool {
        if !matches!(mode, citum_schema::citation::CitationMode::NonIntegral) {
            return false;
        }

        self.bibliography_config
            .as_ref()
            .and_then(|b| b.compound_numeric.as_ref())
            .is_some_and(|c| c.subentry && c.collapse_subentries)
    }

    /// Determine if citation numbers should be collapsed into ranges.
    fn should_collapse_citation_numbers(
        &self,
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
    ) -> bool {
        if !matches!(mode, citum_schema::citation::CitationMode::NonIntegral) {
            return false;
        }

        let is_numeric = self
            .config
            .processing
            .as_ref()
            .is_some_and(|p| matches!(p, citum_schema::options::Processing::Numeric));

        is_numeric
            && matches!(
                spec.collapse,
                Some(citum_schema::CitationCollapse::CitationNumber)
            )
    }

    /// Heuristic for ensuring proper spacing after a citation prefix.
    fn normalize_prefix_spacing(prefix: &str) -> String {
        if !prefix.is_empty() && !prefix.ends_with(char::is_whitespace) {
            format!("{prefix} ")
        } else {
            prefix.to_string()
        }
    }

    /// Ensure suffix has proper spacing (add space if suffix doesn't start with
    /// punctuation and isn't empty).
    fn ensure_suffix_spacing(suffix: &str) -> String {
        if suffix.is_empty() {
            String::new()
        } else if suffix.starts_with(char::is_whitespace)
            || suffix.starts_with(',')
            || suffix.starts_with(';')
            || suffix.starts_with('.')
        {
            // Already has leading space or punctuation
            suffix.to_string()
        } else {
            // Add space before suffix to separate from content
            format!(" {suffix}")
        }
    }

    /// Whether `options.multilingual.scripts.latin.punctuation: latin` applies to the
    /// reference behind `ref_id`.
    ///
    /// Citation-cluster-level `prefix`/`suffix`/`delimiter` (e.g. GB/T author-date's
    /// full-width `（ ）` wrap) are applied in [`Self::affix_content`], outside each
    /// component's own rendering — component-internal punctuation is already remapped
    /// by `render::component::wants_latin_punctuation`. This mirrors that check using
    /// the citation item's resolved reference.
    fn wants_latin_punctuation_for_id(&self, ref_id: &str) -> bool {
        let configured = self.config.multilingual.as_ref().is_some_and(|ml| {
            ml.scripts.get("latin").is_some_and(|script| {
                script.punctuation == Some(citum_schema::options::PunctuationStyle::Latin)
            })
        });

        configured
            && self.bibliography.get(ref_id).is_some_and(|reference| {
                crate::values::is_latin_script_language(
                    crate::values::effective_item_language(reference).as_deref(),
                )
            })
    }

    /// Apply prefix and suffix spacing heuristics to a rendered string.
    ///
    /// `ref_id` identifies the reference this content belongs to, so a
    /// script-aware punctuation remap (see [`Self::wants_latin_punctuation_for_id`])
    /// can be applied to affixes assembled outside component rendering. Pass
    /// `None` when no single reference applies (e.g. author-only content).
    fn affix_content<F>(
        &self,
        fmt: &F,
        content: String,
        prefix: Option<&str>,
        suffix: Option<&str>,
        ref_id: Option<&str>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let prefix = prefix.unwrap_or("");
        let suffix = suffix.unwrap_or("");
        let affixed = if prefix.is_empty() && suffix.is_empty() {
            content
        } else {
            fmt.affix(
                &Self::normalize_prefix_spacing(prefix),
                content,
                &Self::ensure_suffix_spacing(suffix),
            )
        };

        if ref_id.is_some_and(|id| self.wants_latin_punctuation_for_id(id)) {
            crate::render::component::remap_to_latin_punctuation(affixed)
        } else {
            affixed
        }
    }

    /// Pair rendered content with associated reference IDs to form a semantic chunk.
    /// Present a bibliography marker: its wrap, any wrap-implied suffix, then
    /// the `label-separator` that joins it to the entry body.
    fn present_bibliography_marker_with_format<F>(
        &self,
        fmt: &F,
        spec: &marker::BibliographyMarkerSpec,
        value: &marker::MarkerValue,
        ref_id: Option<&str>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let text = fmt.text(&value.as_localized_text(&self.locale.number_formats.digit_system));
        let wrapped = match spec.wrap {
            Some(wrap) => self.wrap_bibliography_label_with_format(fmt, text, wrap, ref_id),
            None => text,
        };
        let suffix = spec
            .wrap
            .and_then(citum_schema::options::BibliographyLabelWrap::as_suffix)
            .unwrap_or_default();
        format!("{wrapped}{suffix}{}", spec.separator)
    }

    /// Apply a bibliography label wrap, reusing the citation wrap machinery so
    /// punctuation realization and quote marks resolve identically.
    fn wrap_bibliography_label_with_format<F>(
        &self,
        fmt: &F,
        content: String,
        wrap: citum_schema::options::BibliographyLabelWrap,
        ref_id: Option<&str>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let Some(config) = wrap.as_wrap_config() else {
            return content;
        };
        let language = ref_id
            .and_then(|id| self.bibliography.get(id))
            .and_then(crate::values::effective_item_language);
        let (script, realization) = crate::values::punctuation_realization_context(
            language.as_deref(),
            self.config.multilingual.as_ref(),
            self.locale.punctuation_realization.as_ref(),
        );
        let marks = crate::render::format::QuoteMarks::from(&self.locale.grammar_options);
        fmt.wrap_punctuation(
            &config.punctuation,
            content,
            &marks,
            script,
            realization.as_deref(),
        )
    }

    /// Render a resolved marker to text, with its own `label-wrap` applied.
    fn present_marker_with_format<F>(
        &self,
        fmt: &F,
        resolved: &ResolvedMarker,
        ref_id: Option<&str>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let text = fmt.text(
            &resolved
                .value
                .as_localized_text(&self.locale.number_formats.digit_system),
        );
        self.wrap_citation_label_with_format(fmt, text, resolved.spec.label_wrap, ref_id)
    }

    /// Join a marker to the item body it belongs to, honouring placement.
    fn compose_marker_with_body(
        marker: String,
        body: String,
        placement: marker::MarkerPlacement,
        delimiter: &str,
    ) -> String {
        if body.is_empty() {
            return marker;
        }
        match placement {
            marker::MarkerPlacement::Leading => format!("{marker}{delimiter}{body}"),
            marker::MarkerPlacement::Trailing => format!("{body}{delimiter}{marker}"),
        }
    }

    /// Pair rendered content with associated reference IDs to form a semantic chunk.
    ///
    /// A chunk whose body is empty keeps its marker unrendered so numeric
    /// collapse can still merge it; every other chunk is composed here, because
    /// the marker and any `item-wrap` sit *inside* the cite's own prefix and
    /// suffix ("see also [2]", not "[see also 2]").
    #[allow(
        clippy::too_many_arguments,
        reason = "chunk assembly keeps rendering affixes and marker metadata together"
    )]
    fn build_citation_chunk<F>(
        &self,
        fmt: &F,
        ids: Vec<String>,
        body: String,
        prefix: Option<&str>,
        suffix: Option<&str>,
        resolved: Option<ResolvedMarker>,
        delimiter: &str,
    ) -> Option<CitationChunk>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let ref_id = ids.first().cloned();
        let ref_id = ref_id.as_deref();
        let collapsible = body.is_empty()
            && prefix.is_none()
            && suffix.is_none()
            && resolved
                .as_ref()
                .is_some_and(|resolved| resolved.value.is_numeric());

        if collapsible {
            return Some(CitationChunk {
                ids,
                content: String::new(),
                marker: resolved,
            });
        }

        let content = match &resolved {
            Some(resolved) => {
                let marker_text = self.present_marker_with_format(fmt, resolved, ref_id);
                let composed = Self::compose_marker_with_body(
                    marker_text,
                    body,
                    resolved.spec.placement,
                    delimiter,
                );
                self.wrap_citation_label_with_format(fmt, composed, resolved.spec.item_wrap, ref_id)
            }
            None => body,
        };
        if content.is_empty() {
            return None;
        }
        Some(CitationChunk {
            ids,
            content: self.affix_content(fmt, content, prefix, suffix, ref_id),
            marker: None,
        })
    }

    /// Build a citation chunk for a single item from its rendered body.
    fn build_item_chunk<F>(
        &self,
        fmt: &F,
        item: &crate::reference::CitationItem,
        reference: &Reference,
        body: String,
        spec: Option<marker::CitationMarkerSpec>,
        delimiter: &str,
    ) -> Option<CitationChunk>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let resolved = spec.and_then(|spec| {
            marker::marker_value(
                spec.kind,
                &self.config,
                reference,
                Some(self.get_or_assign_citation_number(&item.id)),
                self.citation_sub_label_for_ref(&item.id),
                self.hints.get(&item.id),
            )
            .map(|value| ResolvedMarker { value, spec })
        });
        self.build_citation_chunk(
            fmt,
            vec![item.id.clone()],
            body,
            item.prefix.as_deref(),
            item.suffix.as_deref(),
            resolved,
            delimiter,
        )
    }

    /// Create a template render request for a single citation item.
    fn citation_render_request<'b>(
        &self,
        item: &'b crate::reference::CitationItem,
        template: &'b [TemplateComponent],
        mode: &citum_schema::citation::CitationMode,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
        note_start_text_case: Option<citum_schema::NoteStartTextCase>,
    ) -> TemplateRenderRequest<'b> {
        TemplateRenderRequest {
            template,
            context: RenderContext::Citation,
            mode: mode.clone(),
            suppress_author,
            locator_raw: item.locator.as_ref(),
            citation_number: self.get_or_assign_citation_number(&item.id),
            position: position.cloned(),
            note_start_text_case,
            integral_name_state: item.integral_name_state,
            org_abbreviation_state: item.org_abbreviation_state,
            first_reference_note_number: self.first_note_by_id.as_ref().and_then(|m| {
                m.read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&item.id)
                    .copied()
            }),
        }
    }

    /// Render a single item to a formatted string using a template.
    fn render_item_from_template_with_format<F>(
        &self,
        reference: &Reference,
        request: TemplateRenderRequest<'_>,
        delimiter: &str,
    ) -> Option<String>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        self.process_template_request_with_format::<F>(reference, request)
            .map(|proc| {
                crate::render::citation::citation_to_string_with_format::<F>(
                    &proc,
                    None,
                    None,
                    None,
                    Some(delimiter),
                )
            })
    }

    /// Resolve the reference, template, and delimiter needed to render one
    /// ungrouped citation item, applying type-variant and language fallbacks.
    fn resolve_ungrouped_item_render_state<'b>(
        &'b self,
        item: &'b crate::reference::CitationItem,
        spec: &'b citum_schema::CitationSpec,
        mode: &'b citum_schema::citation::CitationMode,
        intra_delimiter: &'b str,
    ) -> Result<UngroupedItemRenderState<'b>, ProcessorError> {
        let reference = self
            .bibliography
            .get(&item.id)
            .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;
        let ref_type = reference.ref_type();
        let item_language = crate::values::effective_item_language(reference);
        let localized = spec.resolve_localized_template(item_language.as_deref());
        let template = localized
            .as_ref()
            .filter(|resolved| resolved.type_variants.is_some())
            .cloned()
            .map(|resolved| {
                Cow::Owned(resolve_localized_type_variant(
                    resolved,
                    spec.type_variants.as_ref(),
                    &ref_type,
                ))
            })
            .or_else(|| {
                resolve_type_variant(spec.type_variants.as_ref(), &ref_type).map(Cow::Borrowed)
            })
            .or_else(|| localized.map(|resolved| Cow::Owned(resolved.template)))
            .unwrap_or(Cow::Borrowed(&[] as &[TemplateComponent]));

        Ok(UngroupedItemRenderState {
            reference,
            template,
            delimiter: intra_delimiter,
            marker: marker::resolve_citation_marker(&self.config, spec, mode),
        })
    }

    /// Initialize render options for a citation.
    ///
    /// `locale` is resolved by the caller via [`Self::locale_for_reference`]
    /// so the `Cow` it may own outlives this borrow of `RenderOptions`.
    fn citation_render_options<'b>(
        &'b self,
        locale: &'b Locale,
        mode: citum_schema::citation::CitationMode,
        suppress_author: bool,
        locator_raw: Option<&'b CitationLocator>,
        ref_type: Option<String>,
    ) -> RenderOptions<'b> {
        RenderOptions {
            config: self.config.clone(),
            bibliography_config: self.bibliography_config.clone(),
            locale,
            context: RenderContext::Citation,
            mode,
            suppress_author,
            locator_raw,
            ref_type,
            show_semantics: self.show_semantics,
            current_template_index: None,
            abbreviation_map: self.abbreviation_map,
        }
    }

    /// Render author + citation number for numeric integral citations.
    ///
    /// Default implementation for narrative citations in numeric styles (e.g., "Smith [1]").
    fn render_author_number_for_numeric_integral_with_format<F>(
        &self,
        fmt: &F,
        reference: &Reference,
        item: &crate::reference::CitationItem,
        citation_number: usize,
        label_wrap: Option<LabelWrap>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let locale = self.locale_for_reference(reference, RenderContext::Citation);
        let options = self.citation_render_options(
            locale.as_ref(),
            citum_schema::citation::CitationMode::Integral,
            false,
            item.locator.as_ref(),
            Some(reference.ref_type()),
        );

        // Render author in short form
        let author_part = if let Some(authors) = reference.author() {
            let names_vec = self.resolve_contributor_names(&authors);
            fmt.text(&crate::values::format_contributors_short(
                &names_vec, &options,
            ))
        } else {
            String::new()
        };

        // Include compound sub-label (e.g. "a", "b") when applicable.
        let ref_id = reference.id().unwrap_or_default().to_string();
        let sub_label = self.citation_sub_label_for_ref(&ref_id).unwrap_or_default();

        let raw_label = format!("{citation_number}{sub_label}");
        let label = match label_wrap {
            Some(wrap) => self.wrap_citation_label_with_format::<F>(
                fmt,
                raw_label,
                Some(wrap),
                Some(&item.id),
            ),
            None => format!("[{raw_label}]"),
        };

        // Format: "Author [Na]" by default, with an explicit label-wrap override.
        if author_part.is_empty() {
            // Fallback: just citation number if no author.
            label
        } else {
            format!("{author_part} {label}")
        }
    }

    /// Render one item as author + citation number for numeric integral cites.
    fn render_numeric_integral_item_chunk_with_format<F>(
        &self,
        fmt: &F,
        item: &crate::reference::CitationItem,
        label_wrap: Option<LabelWrap>,
    ) -> Result<Option<CitationChunk>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let reference = self
            .bibliography
            .get(&item.id)
            .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;
        let citation_number = self.get_or_assign_citation_number(&item.id);
        let item_str = self.render_author_number_for_numeric_integral_with_format::<F>(
            fmt,
            reference,
            item,
            citation_number,
            label_wrap,
        );
        Ok(self.build_item_chunk(fmt, item, reference, item_str, None, ""))
    }

    /// Render one ungrouped item from its resolved template state.
    fn render_template_item_chunk_with_format<F>(
        &self,
        fmt: &F,
        item: &crate::reference::CitationItem,
        state: UngroupedItemRenderState<'_>,
        params: UngroupedItemRenderParams<'_>,
    ) -> Option<CitationChunk>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let request = self.citation_render_request(
            item,
            &state.template,
            params.mode,
            params.suppress_author,
            params.position,
            params.note_start_text_case,
        );
        // A marker-only style has an empty body template, so the body render
        // yields nothing; the marker still has to produce a chunk.
        let body = self
            .render_item_from_template_with_format::<F>(state.reference, request, state.delimiter)
            .unwrap_or_default();
        if body.is_empty() && state.marker.is_none() {
            return None;
        }
        self.build_item_chunk(
            fmt,
            item,
            state.reference,
            body,
            state.marker,
            state.delimiter,
        )
    }

    /// Render citation items without grouping, using plain text format.
    ///
    /// # Errors
    ///
    /// Returns an error when a referenced item is missing or item rendering
    /// fails.
    pub fn render_ungrouped_citation(
        &self,
        items: &[crate::reference::CitationItem],
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
        intra_delimiter: &str,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
    ) -> Result<Vec<String>, ProcessorError> {
        self.render_ungrouped_citation_with_format::<crate::render::plain::PlainText>(
            items,
            spec,
            mode,
            intra_delimiter,
            suppress_author,
            position,
            spec.note_start_text_case,
        )
    }

    /// Render citation items without grouping, generic over the output format.
    ///
    /// This is the core logic for iterating over citation items, looking up references,
    /// and applying the appropriate template or fallback logic.
    ///
    /// # Errors
    ///
    /// Returns an error when a referenced item is missing or item rendering
    /// fails.
    #[allow(
        clippy::too_many_arguments,
        reason = "Ungrouped citation rendering now needs explicit note-start context."
    )]
    pub fn render_ungrouped_citation_with_format<F>(
        &self,
        items: &[crate::reference::CitationItem],
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
        intra_delimiter: &str,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
        note_start_text_case: Option<citum_schema::NoteStartTextCase>,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let mut chunks: Vec<CitationChunk> = Vec::new();

        // For numeric styles with integral mode, render author + citation number instead.
        let use_author_number = self.should_render_author_number_for_numeric_integral(mode)
            && self.citation_label_mode(spec) != Some(CitationLabelMode::None);
        let params = UngroupedItemRenderParams {
            mode,
            suppress_author,
            position,
            note_start_text_case,
        };

        for item in items {
            let chunk = if use_author_number {
                self.render_numeric_integral_item_chunk_with_format::<F>(
                    &fmt,
                    item,
                    spec.options.as_ref().and_then(|options| options.label_wrap),
                )?
            } else {
                let state =
                    self.resolve_ungrouped_item_render_state(item, spec, mode, intra_delimiter)?;
                self.render_template_item_chunk_with_format::<F>(&fmt, item, state, params)
            };

            if let Some(chunk) = chunk {
                chunks.push(chunk);
            }
        }

        if self.should_collapse_compound_subentries(mode) {
            chunks = self.collapse_compound_citation_chunks(chunks);
        }
        if self.should_collapse_citation_numbers(spec, mode) {
            chunks = self.collapse_numeric_citation_chunks(chunks);
        }

        Ok(chunks
            .into_iter()
            .map(|chunk| {
                let ref_id = chunk.ids.first().map(String::as_str);
                // A chunk that still carries a marker was held back for
                // collapse; its presentation is realized here, once, with the
                // label wrap enclosing the marker and the item wrap enclosing
                // the whole item.
                let content = match &chunk.marker {
                    Some(resolved) => {
                        let marker_text = self.present_marker_with_format(&fmt, resolved, ref_id);
                        self.wrap_citation_label_with_format(
                            &fmt,
                            marker_text,
                            resolved.spec.item_wrap,
                            ref_id,
                        )
                    }
                    None => chunk.content,
                };
                fmt.citation(chunk.ids, content)
            })
            .collect())
    }
}

fn key_base(key: &str) -> Cow<'_, str> {
    let mut parts = key.splitn(3, ':');
    match (parts.next(), parts.next()) {
        (Some(kind), Some(var)) => Cow::Owned(format!("{kind}:{var}")),
        _ => Cow::Borrowed(key),
    }
}

/// Get a unique key for a template component's variable, for
/// [`TemplateComponentTracker`] dedup/substitution tracking.
///
/// Contributor/Variable/Number/Identifier key by variable + rendering context
/// (prefix/suffix); Title also keys by form. `Date` components are exempt —
/// see the `Date` arm below.
#[must_use]
pub fn get_variable_key(component: &TemplateComponent) -> Option<String> {
    use citum_schema::template::Rendering;
    use std::fmt::Write;

    fn push_context_suffix(key: &mut String, rendering: &Rendering) {
        match (&rendering.prefix, &rendering.suffix) {
            (Some(prefix), Some(suffix)) => {
                key.push(':');
                key.push_str(prefix);
                key.push('_');
                key.push_str(suffix);
            }
            (Some(prefix), None) => {
                key.push(':');
                key.push_str(prefix);
            }
            (None, Some(suffix)) => {
                key.push(':');
                key.push_str(suffix);
            }
            (None, None) => {}
        }
    }

    fn make_key(kind: &str, value: impl std::fmt::Debug, rendering: &Rendering) -> Option<String> {
        let mut key = String::new();
        write!(&mut key, "{kind}:{value:?}").ok()?;
        push_context_suffix(&mut key, rendering);
        Some(key)
    }

    match component {
        TemplateComponent::Contributor(c) => c.contributor.as_single().map_or_else(
            || make_key("contributor", &c.contributor, &c.rendering),
            |role| make_key("contributor", role, &c.rendering),
        ),
        // Dates are never auto-suppressed for reappearing in a template — CSL
        // restricts variable-consumption tracking to cs:substitute (names
        // only). A style that writes `date: issued` twice (e.g. a short
        // citation year up front, a full precise date later) means for both
        // to render regardless of matching form or rendering context.
        TemplateComponent::Date(_) => None,
        TemplateComponent::Variable(v) => make_key("variable", &v.variable, &v.rendering),
        TemplateComponent::Title(t) => {
            let mut key = format!("title:{:?}", t.title);
            if let Some(form) = &t.form {
                write!(&mut key, ":{form:?}").ok()?;
            }
            push_context_suffix(&mut key, &t.rendering);
            Some(key)
        }
        TemplateComponent::Number(n) => make_key("number", &n.number, &n.rendering),
        TemplateComponent::Identifier(i) => make_key("identifier", &i.identifier, &i.rendering),
        TemplateComponent::Group(_) => None,
        _ => None,
    }
}
