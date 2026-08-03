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
    BibliographyLabelMode, BibliographyLabelWrap, CitationLabelMode, Config, LabelWrap,
    bibliography::BibliographyConfig,
};
use citum_schema::template::{NumberVariable, TemplateComponent};
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

/// A citation template with its declarative label materialized.
struct MaterializedCitationTemplate<'a> {
    /// The effective template, cloned only when a label was inserted or rewritten.
    template: Cow<'a, [TemplateComponent]>,
    /// The label mode whose generated label is the template's only visible content.
    label_only: Option<CitationLabelMode>,
    /// Label presentation held back until numeric collapse has run.
    deferred_wrap: Option<LabelWrap>,
}

/// Per-item state resolved for rendering one ungrouped citation item.
struct UngroupedItemRenderState<'a> {
    reference: &'a Reference,
    template: Cow<'a, [TemplateComponent]>,
    delimiter: &'a str,
    label_only: Option<CitationLabelMode>,
    label_wrap: Option<LabelWrap>,
}

/// Semantic metadata carried by a citation chunk while numeric presentation is deferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NumericCitationLabel {
    /// Processor-assigned base citation number.
    pub number: usize,
    /// Optional compound-entry sub-label, such as `a` in `1a`.
    pub sub_label: Option<String>,
}

/// A rendered citation item plus the metadata needed for semantic collapsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CitationChunk {
    /// Reference IDs represented by this chunk.
    pub ids: Vec<String>,
    /// Rendered chunk content before final numeric-label presentation.
    pub content: String,
    /// Numeric label identity, when this chunk is a label-only citation item.
    pub numeric_label: Option<NumericCitationLabel>,
    /// Scoped label presentation to apply after numeric collapsing.
    pub label_wrap: Option<LabelWrap>,
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
    ///
    /// An omitted mode is inferred from the processing preset: numeric processing
    /// implies numeric labels and label processing implies alphabetic ones, which
    /// is a no-op for styles that still author their label component explicitly.
    fn citation_label_mode(&self, spec: &citum_schema::CitationSpec) -> Option<CitationLabelMode> {
        spec.options
            .as_ref()
            .and_then(|options| options.label_mode)
            .or_else(|| match self.config.effective_processing() {
                citum_schema::options::Processing::Numeric => Some(CitationLabelMode::Numeric),
                citum_schema::options::Processing::Label(_) => Some(CitationLabelMode::Alphabetic),
                _ => None,
            })
    }

    /// Whether a template contains an authored label component for `variable`.
    fn template_has_label(components: &[TemplateComponent], variable: &NumberVariable) -> bool {
        components.iter().any(|component| match component {
            TemplateComponent::Number(number) => number.number == *variable,
            TemplateComponent::Group(group) => Self::template_has_label(&group.group, variable),
            _ => false,
        })
    }

    /// Remove foreign label components from a cloned template without mutating
    /// the style.
    ///
    /// `keep` names the label variable the declared mode generates; every other
    /// label variable is removed, so a mode change also clears an inherited
    /// label of the kind the style itself does not generate. `None` removes both.
    fn strip_other_labels(components: &mut Vec<TemplateComponent>, keep: Option<&NumberVariable>) {
        components.retain_mut(|component| match component {
            TemplateComponent::Number(number)
                if is_citation_label_variable(&number.number)
                    && keep.is_none_or(|keep| number.number != *keep) =>
            {
                false
            }
            TemplateComponent::Group(group) => {
                Self::strip_other_labels(&mut group.group, keep);
                !group.group.is_empty()
            }
            _ => true,
        });
    }

    /// Whether a template carries a label component `keep` does not name.
    ///
    /// `keep` is the variable the declared label mode generates; `None` (no
    /// mode, or `label-mode: none`) makes every label component foreign.
    fn template_has_other_label(
        components: &[TemplateComponent],
        keep: Option<&NumberVariable>,
    ) -> bool {
        components.iter().any(|component| match component {
            TemplateComponent::Number(number) => {
                is_citation_label_variable(&number.number)
                    && keep.is_none_or(|keep| number.number != *keep)
            }
            TemplateComponent::Group(group) => Self::template_has_other_label(&group.group, keep),
            _ => false,
        })
    }

    /// Apply a citation label presentation to explicit or generated label nodes.
    fn apply_citation_label_wrap(
        components: &mut [TemplateComponent],
        variable: &NumberVariable,
        wrap: LabelWrap,
    ) {
        for component in components {
            match component {
                TemplateComponent::Number(number) if number.number == *variable => {
                    number.rendering.wrap = wrap.as_wrap_config();
                    number.rendering.vertical_align = (wrap == LabelWrap::Superscript)
                        .then_some(citum_schema::VerticalAlign::Superscript);
                    number.rendering.suffix = None;
                }
                TemplateComponent::Group(group) => {
                    Self::apply_citation_label_wrap(&mut group.group, variable, wrap);
                }
                _ => {}
            }
        }
    }

    /// Whether the visible template is only a citation label for `variable`.
    fn template_is_label_only(components: &[TemplateComponent], variable: &NumberVariable) -> bool {
        let Some(component) = components.first() else {
            return false;
        };
        if components.len() != 1 {
            return false;
        }
        match component {
            TemplateComponent::Number(number) => {
                number.number == *variable
                    && number.rendering.suppress != Some(true)
                    && number.rendering.prefix.is_none()
                    && number.rendering.suffix.is_none()
                    && number.rendering.emph.is_none()
                    && number.rendering.strong.is_none()
                    && number.rendering.small_caps.is_none()
                    && number.rendering.quote.is_none()
            }
            TemplateComponent::Group(group) => {
                group.render_when.is_none()
                    && group.rendering == citum_schema::template::Rendering::default()
                    && group.custom.is_none()
                    && group
                        .delimiter
                        .as_ref()
                        .is_none_or(|delimiter| match delimiter {
                            citum_schema::template::DelimiterPunctuation::None => true,
                            citum_schema::template::DelimiterPunctuation::Custom(text) => {
                                text.is_empty()
                            }
                            _ => false,
                        })
                    && Self::template_is_label_only(&group.group, variable)
            }
            _ => false,
        }
    }

    /// Whether a label-only template carries authored presentation that should be retained.
    fn template_has_label_presentation(components: &[TemplateComponent]) -> bool {
        let Some(component) = components.first() else {
            return false;
        };
        if components.len() != 1 {
            return false;
        }
        match component {
            TemplateComponent::Number(number) => {
                number.rendering.wrap.is_some() || number.rendering.vertical_align.is_some()
            }
            TemplateComponent::Group(group) => Self::template_has_label_presentation(&group.group),
            _ => false,
        }
    }

    /// Materialize declarative citation labels after the effective template is selected.
    fn materialize_citation_template<'b>(
        &self,
        template: Cow<'b, [TemplateComponent]>,
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
    ) -> MaterializedCitationTemplate<'b> {
        let label_mode = self.citation_label_mode(spec);
        let label_wrap = spec.options.as_ref().and_then(|options| options.label_wrap);
        let variable = label_mode.and_then(CitationLabelMode::label_variable);
        let has_label = variable
            .as_ref()
            .is_some_and(|variable| Self::template_has_label(template.as_ref(), variable));
        let needs_label = variable.is_some() && !has_label;
        // A declared mode names the label variable: a label of any other kind is
        // a leftover from an inherited template and is always removed, whether
        // or not the declared label is also present. No declared mode at all
        // leaves the authored template exactly as written.
        let suppress_label = label_mode.is_some()
            && Self::template_has_other_label(template.as_ref(), variable.as_ref());
        let needs_wrap = variable.is_some() && label_wrap.is_some() && has_label;

        if !needs_label && !suppress_label && !needs_wrap {
            let is_label_only = variable.as_ref().is_some_and(|variable| {
                Self::template_is_label_only(template.as_ref(), variable)
                    && !Self::template_has_label_presentation(template.as_ref())
            });
            return MaterializedCitationTemplate {
                template,
                label_only: is_label_only.then_some(label_mode).flatten(),
                deferred_wrap: None,
            };
        }

        let mut owned = template.into_owned();
        if suppress_label {
            Self::strip_other_labels(&mut owned, variable.as_ref());
        }
        if let Some(variable) = variable.clone().filter(|_| needs_label) {
            let label = TemplateComponent::Number(citum_schema::TemplateNumber {
                number: variable,
                ..Default::default()
            });
            if matches!(mode, citum_schema::citation::CitationMode::Integral) {
                owned.push(label);
            } else {
                owned.insert(0, label);
            }
        }

        let is_label_only = variable
            .as_ref()
            .is_some_and(|variable| Self::template_is_label_only(&owned, variable));
        let label_only = is_label_only.then_some(label_mode).flatten();
        // Only numeric labels are collapsed, so only they need their presentation
        // held back; alphabetic labels take their wrapping in the template.
        let defer_wrap = label_only == Some(CitationLabelMode::Numeric) && label_wrap.is_some();
        if let (Some(variable), Some(wrap)) = (variable.as_ref(), label_wrap) {
            Self::apply_citation_label_wrap(&mut owned, variable, wrap);
        }
        if defer_wrap {
            // Keep the semantic number bare until collapse has run; wrapping the
            // finished range is equivalent to wrapping each source label but keeps
            // the collapse predicate independent of presentation punctuation.
            Self::clear_citation_label_presentation(&mut owned);
        }

        MaterializedCitationTemplate {
            template: Cow::Owned(owned),
            label_only,
            deferred_wrap: defer_wrap.then_some(label_wrap).flatten(),
        }
    }

    /// Clear only presentation fields that would obscure a semantic numeric label.
    fn clear_citation_label_presentation(components: &mut [TemplateComponent]) {
        for component in components {
            match component {
                TemplateComponent::Number(number)
                    if number.number == NumberVariable::CitationNumber =>
                {
                    number.rendering.wrap = None;
                    number.rendering.vertical_align = None;
                    number.rendering.suffix = None;
                }
                TemplateComponent::Group(group) => {
                    Self::clear_citation_label_presentation(&mut group.group);
                }
                _ => {}
            }
        }
    }

    /// Materialize runtime bibliography label presentation without changing the style AST.
    pub(super) fn materialize_bibliography_template<'b>(
        &self,
        template: Cow<'b, [TemplateComponent]>,
    ) -> Cow<'b, [TemplateComponent]> {
        let Some(config) = self.bibliography_config.as_ref() else {
            return template;
        };
        let mode = config.label_mode;
        let wrap = config.label_wrap;
        let separator = config.label_separator.clone();
        let variable = mode.and_then(BibliographyLabelMode::label_variable);
        let has_label = variable
            .as_ref()
            .is_some_and(|variable| Self::template_has_label(&template, variable));
        // A declared mode names the label variable: a label of any other kind is
        // a leftover from an inherited template and is always removed, whether
        // or not the declared label is also present. `author-date` and `none`
        // name no variable, so for them every label component is foreign. No
        // declared mode at all leaves the authored template exactly as written.
        let needs_strip =
            mode.is_some() && Self::template_has_other_label(&template, variable.as_ref());
        let needs_insert = variable.is_some() && !has_label;
        // A separator is part of the label's presentation just as the wrap is,
        // so an authored label picks it up too — otherwise the same option would
        // mean one thing for a generated label and another for an authored one.
        let needs_wrap = (wrap.is_some() || separator.is_some()) && has_label;
        if !needs_strip && !needs_insert && !needs_wrap {
            return template;
        }

        let mut owned = template.into_owned();
        if needs_strip {
            Self::strip_other_labels(&mut owned, variable.as_ref());
        }
        if let Some(variable) = variable.filter(|_| needs_insert) {
            let mut label = citum_schema::TemplateNumber {
                number: variable,
                ..Default::default()
            };
            if let Some(wrap) = wrap {
                label.rendering.wrap = wrap.as_wrap_config();
                label.rendering.suffix = wrap.as_suffix().map(Into::into);
            }
            let label = TemplateComponent::Number(label);
            if owned.is_empty() {
                owned.push(label);
            } else {
                let following = owned.remove(0);
                owned.insert(
                    0,
                    TemplateComponent::Group(citum_schema::TemplateGroup {
                        group: vec![label, following],
                        delimiter: Some(separator.unwrap_or(
                            citum_schema::template::DelimiterPunctuation::Custom(String::new()),
                        )),
                        ..Default::default()
                    }),
                );
            }
        } else if needs_wrap {
            Self::apply_bibliography_label_presentation(&mut owned, wrap, separator.as_ref());
        }
        Cow::Owned(owned)
    }

    /// Apply runtime label presentation to legacy explicit label components.
    ///
    /// `separator` supplies the label suffix when the wrap style contributes
    /// none of its own, so an authored label keeps the spacing the declarative
    /// insertion path would have given it.
    fn apply_bibliography_label_presentation(
        components: &mut [TemplateComponent],
        wrap: Option<BibliographyLabelWrap>,
        separator: Option<&citum_schema::template::DelimiterPunctuation>,
    ) {
        for component in components {
            match component {
                TemplateComponent::Number(number) if is_citation_label_variable(&number.number) => {
                    if let Some(wrap) = wrap {
                        number.rendering.wrap = wrap.as_wrap_config();
                    }
                    number.rendering.suffix = wrap
                        .and_then(BibliographyLabelWrap::as_suffix)
                        .map(Into::into)
                        .or_else(|| separator.cloned());
                }
                TemplateComponent::Group(group) => {
                    Self::apply_bibliography_label_presentation(&mut group.group, wrap, separator);
                }
                _ => {}
            }
        }
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
    #[allow(
        clippy::too_many_arguments,
        reason = "chunk assembly keeps rendering affixes and semantic label metadata together"
    )]
    fn build_citation_chunk<F>(
        &self,
        fmt: &F,
        ids: Vec<String>,
        content: String,
        prefix: Option<&str>,
        suffix: Option<&str>,
        numeric_label: Option<NumericCitationLabel>,
        label_wrap: Option<LabelWrap>,
    ) -> Option<CitationChunk>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        if content.is_empty() {
            None
        } else {
            let affixed = self.affix_content(
                fmt,
                content,
                prefix,
                suffix,
                ids.first().map(String::as_str),
            );
            Some(CitationChunk {
                ids,
                content: affixed,
                numeric_label,
                label_wrap,
            })
        }
    }

    /// Build a citation chunk for a single item from its rendered content.
    fn build_item_chunk<F>(
        &self,
        fmt: &F,
        item: &crate::reference::CitationItem,
        content: String,
        label_only: Option<CitationLabelMode>,
        label_wrap: Option<LabelWrap>,
    ) -> Option<CitationChunk>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        // Only numeric label-only chunks carry collapse identity; alphabetic
        // labels have no numeric ordering to collapse into ranges.
        let numeric_label = (label_only == Some(CitationLabelMode::Numeric)
            && item.prefix.is_none()
            && item.suffix.is_none())
        .then(|| NumericCitationLabel {
            number: self.get_or_assign_citation_number(&item.id),
            sub_label: self.citation_sub_label_for_ref(&item.id),
        });
        self.build_citation_chunk(
            fmt,
            vec![item.id.clone()],
            content,
            item.prefix.as_deref(),
            item.suffix.as_deref(),
            numeric_label,
            label_wrap,
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

        let materialized = self.materialize_citation_template(template, spec, mode);

        Ok(UngroupedItemRenderState {
            reference,
            template: materialized.template,
            delimiter: intra_delimiter,
            label_only: materialized.label_only,
            label_wrap: materialized.deferred_wrap,
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
        Ok(self.build_item_chunk(fmt, item, item_str, None, None))
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
        self.render_item_from_template_with_format::<F>(state.reference, request, state.delimiter)
            .and_then(|item_str| {
                self.build_item_chunk(fmt, item, item_str, state.label_only, state.label_wrap)
            })
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
                let content = if chunk.numeric_label.is_some() {
                    self.wrap_citation_label_with_format(
                        &fmt,
                        chunk.content,
                        chunk.label_wrap,
                        chunk.ids.first().map(String::as_str),
                    )
                } else {
                    chunk.content
                };
                fmt.citation(chunk.ids, content)
            })
            .collect())
    }
}

/// Whether a number variable is one of the processor-generated citation labels.
fn is_citation_label_variable(variable: &NumberVariable) -> bool {
    matches!(
        variable,
        NumberVariable::CitationNumber | NumberVariable::CitationLabel
    )
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
