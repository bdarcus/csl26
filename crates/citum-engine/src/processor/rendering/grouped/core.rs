/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::super::{
    GroupRenderParams, Renderer, TemplateComponentTracker, TemplateRenderParams,
    TemplateRenderRequest, find_grouping_component, get_variable_key, has_contributor_component,
    leading_group_affix, remove_first_contributor_with_role, strip_author_component,
    strip_leading_group_affixes,
};
use super::component_predicates::{
    is_term_only_component, resolve_localized_type_variant, resolve_type_variant,
};
use super::group_citation_items_by_author;
use crate::error::ProcessorError;
use crate::reference::Reference;
use crate::render::{ProcTemplate, ProcTemplateComponent};
use crate::values::{ComponentValues, ProcHints, RenderContext, RenderOptions};
use citum_schema::template::{TemplateComponent, WrapConfig, WrapPunctuation};
use std::borrow::Cow;

struct GroupRenderState<'a> {
    first_item: &'a crate::reference::CitationItem,
    first_ref: &'a Reference,
    template: Cow<'a, [TemplateComponent]>,
}

struct ItemRenderState<'a> {
    item: &'a crate::reference::CitationItem,
    reference: &'a Reference,
    template: Cow<'a, [TemplateComponent]>,
}

struct GroupItemRenderRequest<'a> {
    item: &'a crate::reference::CitationItem,
    template: &'a [TemplateComponent],
    mode: &'a citum_schema::citation::CitationMode,
    suppress_author: bool,
    position: Option<&'a citum_schema::citation::Position>,
    note_start_text_case: Option<citum_schema::NoteStartTextCase>,
    delimiter: &'a str,
}

/// Resolved context for rendering a single template (or nested group)
/// component. Bundles parameters that would otherwise inflate
/// [`Renderer::render_template_component_with_format`] and
/// [`Renderer::render_group_component_with_format`] past the clippy
/// argument-count limit.
struct TemplateRenderContext<'a> {
    reference: &'a Reference,
    ref_type: &'a str,
    options: &'a RenderOptions<'a>,
    hint: &'a ProcHints,
    template_index: usize,
}

/// Inputs for [`Renderer::build_template_render_hint`]. Bundles the
/// per-citation state that would otherwise push the method past the clippy
/// argument-count limit.
struct HintInputs<'a> {
    reference: &'a Reference,
    context: RenderContext,
    citation_number: usize,
    position: Option<citum_schema::citation::Position>,
    integral_name_state: Option<citum_schema::citation::IntegralNameState>,
    org_abbreviation_state: Option<citum_schema::citation::IntegralNameState>,
    first_reference_note_number: Option<u32>,
}

impl Renderer<'_> {
    fn strip_redundant_leading_group_punctuation<'a>(
        &self,
        value: &'a str,
        delimiter: &str,
    ) -> &'a str {
        let Some(delimiter_char) = delimiter.chars().find(|ch| !ch.is_whitespace()) else {
            return value;
        };

        let trimmed = value.trim_start();
        if !trimmed.starts_with(delimiter_char) {
            return value;
        }

        #[allow(clippy::string_slice, reason = "delimiter found at start")]
        trimmed[delimiter_char.len_utf8()..].trim_start()
    }

    fn join_integral_group_item_parts(&self, item_parts: &[String], delimiter: &str) -> String {
        let repeated_item_delimiter = if delimiter.trim().is_empty() {
            ", "
        } else {
            delimiter
        };

        let mut joined = String::new();
        for (index, part) in item_parts.iter().enumerate() {
            if index > 0 {
                joined.push_str(repeated_item_delimiter);
            }

            let normalized = if index == 0 {
                part.as_str()
            } else {
                self.strip_redundant_leading_group_punctuation(part, repeated_item_delimiter)
            };
            joined.push_str(normalized);
        }

        joined
    }

    /// Render citation items with author grouping, using plain text format.
    ///
    /// # Errors
    ///
    /// Returns an error when a referenced item is missing or grouped rendering fails.
    pub fn render_grouped_citation(
        &self,
        items: &[crate::reference::CitationItem],
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
        intra_delimiter: &str,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
    ) -> Result<Vec<String>, ProcessorError> {
        self.render_grouped_citation_with_format::<crate::render::plain::PlainText>(
            items,
            &GroupRenderParams {
                spec,
                mode,
                intra_delimiter,
                suppress_author,
                position,
                note_start_text_case: spec.note_start_text_case,
            },
        )
    }

    /// Render a group of items that must not be author-collapsed (legal cases,
    /// personal communications). Returns the rendered citation strings.
    fn render_special_type_items<F>(
        &self,
        group: &[&crate::reference::CitationItem],
        params: &GroupRenderParams<'_>,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let mut rendered_items = Vec::new();
        for item in group {
            let state = self.resolve_item_render_state(item, params.spec)?;
            if let Some(item_str) = self.render_group_item_from_template_with_format::<F>(
                state.reference,
                GroupItemRenderRequest {
                    item: state.item,
                    template: &state.template,
                    mode: params.mode,
                    suppress_author: params.suppress_author,
                    position: params.position,
                    note_start_text_case: params.note_start_text_case,
                    delimiter: params.intra_delimiter,
                },
            ) && let Some(chunk) = self.build_citation_chunk(
                &fmt,
                vec![item.id.clone()],
                item_str,
                item.prefix.as_deref(),
                item.suffix.as_deref(),
                None,
                "",
            ) {
                rendered_items.push(fmt.citation(chunk.ids, chunk.content));
            }
        }
        Ok(rendered_items)
    }

    /// Render one citation group using the explicit integral template.
    ///
    /// Returns `Ok(Some(citation))` if the group rendered (caller should push and `continue`),
    /// or `Ok(None)` if no items produced output (caller should fall through to other branches).
    fn render_integral_explicit_group<F>(
        &self,
        group: &[&crate::reference::CitationItem],
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
    ) -> Result<Option<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let component_delimiter = spec.delimiter.as_deref().unwrap_or(" ");
        let item_join_delim = spec.multi_cite_delimiter.as_deref().unwrap_or(", ");
        let mut group_items_str = Vec::new();
        let mut all_ids = Vec::new();

        for item in group {
            let state = self.resolve_item_render_state(item, spec)?;
            if let Some(item_str) = self.render_group_item_from_template_with_format::<F>(
                state.reference,
                GroupItemRenderRequest {
                    item: state.item,
                    template: &state.template,
                    mode,
                    suppress_author,
                    position,
                    note_start_text_case: spec.note_start_text_case,
                    delimiter: component_delimiter,
                },
            ) && !item_str.is_empty()
            {
                group_items_str.push(self.affix_content(
                    &fmt,
                    item_str,
                    item.prefix.as_deref(),
                    item.suffix.as_deref(),
                    Some(item.id.as_str()),
                ));
                all_ids.push(item.id.clone());
            }
        }

        if group_items_str.is_empty() {
            return Ok(None);
        }

        let combined_str = group_items_str.join(item_join_delim);
        Ok(Some(fmt.citation(all_ids, combined_str)))
    }

    /// This preserves per-item output when grouping rules require items to stay
    /// separate, and otherwise applies the requested renderer format to the
    /// grouped citation output.
    ///
    /// # Errors
    ///
    /// Returns an error when a referenced item is missing or grouped rendering
    /// fails.
    pub fn render_grouped_citation_with_format<F>(
        &self,
        items: &[crate::reference::CitationItem],
        params: &GroupRenderParams<'_>,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let groups = group_citation_items_by_author(self, items);
        let mut rendered_groups = Vec::new();
        for (_author_key, group) in groups {
            rendered_groups
                .extend(self.render_grouped_citation_group_with_format::<F>(&group, params)?);
        }

        Ok(rendered_groups)
    }

    fn render_grouped_citation_group_with_format<F>(
        &self,
        group: &[&crate::reference::CitationItem],
        params: &GroupRenderParams<'_>,
    ) -> Result<Vec<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let state = self.resolve_group_render_state(group, params.spec)?;

        // Multi-item same-author groups always collapse: one author name, all years
        // joined, in both integral and non-integral modes. The year-group wrap is
        // captured from the template and applied once around all years.
        // Single-item groups use the per-item explicit integral path when available.
        if group.len() == 1
            && let Some(citation) = self.try_render_integral_group_with_format::<F>(
                group,
                params.spec,
                params.mode,
                params.suppress_author,
                params.position,
            )?
        {
            return Ok(vec![citation]);
        }

        if self.requires_full_group_item_rendering(params.mode, state.first_ref) {
            return self.render_special_type_items::<F>(group, params);
        }

        Ok(self
            .render_fallback_grouped_citation_with_format::<F>(
                group,
                state.first_ref,
                state.first_item,
                &state.template,
                params,
            )?
            .into_iter()
            .collect())
    }

    fn render_fallback_grouped_citation_with_format<F>(
        &self,
        group: &[&crate::reference::CitationItem],
        first_ref: &Reference,
        first_item: &crate::reference::CitationItem,
        template: &[TemplateComponent],
        params: &GroupRenderParams<'_>,
    ) -> Result<Option<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let fmt = F::default();
        let author_part = self.render_author_for_grouping_with_format::<F>(
            first_ref,
            first_item,
            template,
            params.mode,
            params.suppress_author,
            params.position,
        );
        let (item_parts, group_delimiter, captured_year_wrap) =
            self.render_group_item_parts_with_format::<F>(&fmt, group, params)?;
        // CMOS 15.30 / citeproc's `after-collapse-delimiter`: once any item in a
        // same-author collapsed group carries a locator, the intra-group join
        // escalates from the ordinary group delimiter to `multi_cite_delimiter`
        // (default "; ") so the locator doesn't read as a bare extra year, e.g.
        // "Sutinen 1969; 1976, 257; 1981" rather than "Sutinen 1969, 1976, 257, 1981".
        // See docs/specs/CITATION_CLUSTER_RENDERING.md "Same-author collapse with
        // locators".
        let group_has_locator = group.iter().any(|item| item.locator.is_some());
        // Script/realization context, shared below by the escalated delimiter and
        // the integral wrap punctuation. Both must route through the same
        // script-aware realization table (e.g. GB/T's `{ mark: semicolon }`
        // resolving to a full-width `；`) rather than DelimiterPunctuation's
        // `Deref`, which only ever exposes the Latin default and would silently
        // emit ASCII punctuation in CJK output.
        let (script, realization) = crate::values::punctuation_realization_context(
            crate::values::effective_item_language(first_ref).as_deref(),
            self.config.multilingual.as_ref(),
            self.locale.punctuation_realization.as_ref(),
        );
        let escalated_delimiter = group_has_locator.then(|| {
            params
                .spec
                .multi_cite_delimiter
                .as_ref()
                .map(|punctuation| {
                    crate::render::format::realize_punctuation(
                        punctuation,
                        script,
                        realization.as_deref(),
                        crate::render::format::PunctuationPosition::Separator,
                    )
                    .into_owned()
                })
                .unwrap_or_else(|| "; ".to_string())
        });
        // Pre-compute a format-aware wrapped years string for integral collapsed groups.
        // Using fmt.inner_affix + fmt.wrap_punctuation honours output-format-specific
        // punctuation (e.g. LaTeX ``…'') and preserves WrapConfig.inner_prefix/suffix.
        // Non-integral groups leave pre_wrapped_years as None and rely on the
        // per-item template path in build_grouped_citation_content.
        let pre_wrapped_years =
            if matches!(params.mode, citum_schema::citation::CitationMode::Integral)
                && !item_parts.is_empty()
            {
                let delimiter = if let Some(escalated) = escalated_delimiter.as_deref() {
                    escalated
                } else {
                    group_delimiter.as_deref().unwrap_or(params.intra_delimiter)
                };
                let joined = self.join_integral_group_item_parts(&item_parts, delimiter);
                let wrap_punct = captured_year_wrap
                    .as_ref()
                    .map(|w| &w.punctuation)
                    .unwrap_or(&WrapPunctuation::Parentheses);
                let inner_prefix = captured_year_wrap
                    .as_ref()
                    .and_then(|w| w.inner_prefix.as_deref())
                    .unwrap_or("");
                let inner_suffix = captured_year_wrap
                    .as_ref()
                    .and_then(|w| w.inner_suffix.as_deref())
                    .unwrap_or("");
                let inner = fmt.inner_affix(inner_prefix, joined, inner_suffix);
                let marks = crate::render::format::QuoteMarks::from(&self.locale.grammar_options);
                Some(fmt.wrap_punctuation(
                    wrap_punct,
                    inner,
                    &marks,
                    script,
                    realization.as_deref(),
                ))
            } else {
                None
            };
        let Some(content) = self.build_grouped_citation_content::<F>(
            &author_part,
            &item_parts,
            params,
            group_delimiter.as_deref(),
            pre_wrapped_years.as_deref(),
            escalated_delimiter.as_deref(),
        ) else {
            return Ok(None);
        };
        let group_ids = group.iter().map(|item| item.id.clone()).collect();
        let prefix = first_item.prefix.as_deref().unwrap_or("");
        // Suffix is embedded in item_parts by render_group_item_parts_with_format when
        // item_parts is non-empty. Apply it here only when item_parts was empty (author-only output).
        let suffix = if item_parts.is_empty() {
            first_item.suffix.as_deref()
        } else {
            None
        };

        Ok(Some(fmt.citation(
            group_ids,
            self.affix_content(
                &fmt,
                content,
                Some(prefix),
                suffix,
                Some(first_item.id.as_str()),
            ),
        )))
    }

    fn build_grouped_citation_content<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        author_part: &str,
        item_parts: &[String],
        params: &GroupRenderParams<'_>,
        group_delimiter: Option<&str>,
        pre_wrapped_years: Option<&str>,
        escalated_delimiter: Option<&str>,
    ) -> Option<String> {
        if !author_part.is_empty() && !item_parts.is_empty() {
            let author_item_delimiter = group_delimiter.unwrap_or(params.intra_delimiter);
            return Some(match params.mode {
                citum_schema::citation::CitationMode::Integral => {
                    // pre_wrapped_years is Some for collapsed multi-item integral groups
                    // (format-aware wrap applied upstream, already locator-escalated by
                    // the caller). For single-item groups this path is not reached (they
                    // use the explicit integral path instead).
                    let wrapped = pre_wrapped_years.map(str::to_string).unwrap_or_else(|| {
                        self.join_integral_group_item_parts(item_parts, author_item_delimiter)
                    });
                    self.format_integral_grouped_items(
                        author_part,
                        &wrapped,
                        params.suppress_author,
                    )
                }
                citum_schema::citation::CitationMode::NonIntegral => {
                    // See the escalated_delimiter comment in
                    // render_fallback_grouped_citation_with_format: same escalation,
                    // non-integral side, already script-realized by the caller.
                    // author_item_delimiter (author -> first year) is untouched --
                    // only the join between repeated items escalates.
                    let repeated_item_delimiter = if let Some(escalated) = escalated_delimiter {
                        escalated
                    } else if author_item_delimiter.trim().is_empty() {
                        ", "
                    } else {
                        author_item_delimiter
                    };
                    let joined_items = item_parts.join(repeated_item_delimiter);
                    self.format_non_integral_grouped_items::<F>(
                        author_part,
                        author_item_delimiter,
                        &joined_items,
                        params.suppress_author,
                    )
                }
            });
        }

        if !author_part.is_empty() {
            return Some(author_part.to_string());
        }

        if !item_parts.is_empty() {
            return Some(item_parts.join(params.intra_delimiter));
        }

        None
    }

    fn format_integral_grouped_items(
        &self,
        author_part: &str,
        wrapped_content: &str,
        suppress_author: bool,
    ) -> String {
        if suppress_author {
            wrapped_content.to_string()
        } else {
            format!("{author_part} {wrapped_content}")
        }
    }

    fn format_non_integral_grouped_items<
        F: crate::render::format::OutputFormat<Output = String>,
    >(
        &self,
        author_part: &str,
        author_item_delimiter: &str,
        joined_items: &str,
        suppress_author: bool,
    ) -> String {
        if suppress_author {
            return joined_items.to_string();
        }

        if let Some(adjusted) =
            self.adjust_grouped_author_quote_punctuation::<F>(author_part, author_item_delimiter)
        {
            return format!("{adjusted}{joined_items}");
        }

        format!("{author_part}{author_item_delimiter}{joined_items}")
    }

    fn adjust_grouped_author_quote_punctuation<
        F: crate::render::format::OutputFormat<Output = String>,
    >(
        &self,
        author_part: &str,
        author_item_delimiter: &str,
    ) -> Option<String> {
        if !self.config.punctuation_in_quote || !author_item_delimiter.starts_with(',') {
            return None;
        }

        let close_quote = crate::render::format::QuoteMarks::from(self.locale).close;
        let mut adjusted = author_part.to_string();
        if !crate::render::punctuation::move_punctuation_into_quote::<F>(
            &mut adjusted,
            ',',
            &close_quote,
        ) {
            return None;
        }
        #[allow(clippy::string_slice, reason = "delimiter checked to start with ','")]
        Some(format!("{adjusted}{}", &author_item_delimiter[1..]))
    }

    fn render_group_item_parts_with_format<F>(
        &self,
        fmt: &F,
        group: &[&crate::reference::CitationItem],
        params: &GroupRenderParams<'_>,
    ) -> Result<(Vec<String>, Option<String>, Option<WrapConfig>), ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let mut item_parts = Vec::new();
        let mut group_delimiter: Option<String> = None;
        // For integral multi-item same-author groups, capture the full WrapConfig
        // (punctuation + inner_prefix/inner_suffix) from the first item's filtered
        // template and strip the wrap from all items. The caller applies it once,
        // format-aware, around the joined year string.
        // Non-integral groups preserve per-item wraps (they may be the primary
        // wrapping when no cluster-level wrap exists, e.g. author-date disambiguation).
        let mut captured_year_wrap: Option<WrapConfig> = None;
        let collapse_group = group.len() > 1
            && matches!(params.mode, citum_schema::citation::CitationMode::Integral);
        for (index, item) in group.iter().enumerate() {
            let state = self.resolve_item_render_state(item, params.spec)?;
            let (script, realization) = crate::values::punctuation_realization_context(
                crate::values::effective_item_language(state.reference).as_deref(),
                self.config.multilingual.as_ref(),
                self.locale.punctuation_realization.as_ref(),
            );
            let (mut filtered_template, leading_affix, strip_item_delimiter) =
                filter_author_from_template::<F>(
                    &state.template,
                    script,
                    realization.as_deref(),
                    fmt,
                );
            if collapse_group {
                if index == 0 {
                    // Capture the full WrapConfig from the first remaining component
                    // (typically the date or date-group). Preserves inner_prefix and
                    // inner_suffix alongside punctuation so the caller can apply the
                    // wrap format-aware via fmt.inner_affix + fmt.wrap_punctuation.
                    captured_year_wrap = filtered_template
                        .first_mut()
                        .and_then(|c| c.rendering_mut().wrap.take());
                } else {
                    // Strip the wrap on subsequent items to match the first item.
                    if let Some(first) = filtered_template.first_mut() {
                        first.rendering_mut().wrap = None;
                    }
                }
            }
            if group_delimiter.is_none() {
                group_delimiter = leading_affix
                    .as_ref()
                    .filter(|value| !value.is_empty())
                    .cloned();
            }
            let item_delimiter = if strip_item_delimiter {
                ""
            } else {
                params.intra_delimiter
            };
            if let Some(item_str) = self.render_group_item_from_template_with_format::<F>(
                state.reference,
                GroupItemRenderRequest {
                    item: state.item,
                    template: &filtered_template,
                    mode: params.mode,
                    suppress_author: params.suppress_author,
                    position: params.position,
                    note_start_text_case: params.note_start_text_case,
                    delimiter: item_delimiter,
                },
            ) && !item_str.is_empty()
            {
                let prefix = (index > 0).then_some(item.prefix.as_deref()).flatten();
                item_parts.push(self.affix_content(
                    fmt,
                    item_str,
                    prefix,
                    item.suffix.as_deref(),
                    Some(item.id.as_str()),
                ));
            }
        }
        Ok((item_parts, group_delimiter, captured_year_wrap))
    }

    fn resolve_group_render_state<'b>(
        &'b self,
        group: &'b [&'b crate::reference::CitationItem],
        spec: &'b citum_schema::CitationSpec,
    ) -> Result<GroupRenderState<'b>, ProcessorError> {
        #[allow(clippy::indexing_slicing, reason = "groups are non-empty")]
        let first_item = group[0];
        let first_ref = self
            .bibliography
            .get(&first_item.id)
            .ok_or_else(|| ProcessorError::ReferenceNotFound(first_item.id.clone()))?;
        let first_language = crate::values::effective_item_language(first_ref);
        let ref_type = first_ref.ref_type();
        let localized = spec.resolve_localized_template(first_language.as_deref());
        let first_template = localized
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
            .or_else(|| localized.map(|resolved| Cow::Owned(resolved.template)));

        Ok(GroupRenderState {
            first_item,
            first_ref,
            template: first_template.unwrap_or(Cow::Borrowed(&[])),
        })
    }

    fn resolve_item_render_state<'b>(
        &'b self,
        item: &'b crate::reference::CitationItem,
        spec: &'b citum_schema::CitationSpec,
    ) -> Result<ItemRenderState<'b>, ProcessorError> {
        let reference = self
            .bibliography
            .get(&item.id)
            .ok_or_else(|| ProcessorError::ReferenceNotFound(item.id.clone()))?;
        let item_language = crate::values::effective_item_language(reference);
        let ref_type = reference.ref_type();
        let localized = spec.resolve_localized_template(item_language.as_deref());
        let item_template = localized
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
            .or_else(|| localized.map(|resolved| Cow::Owned(resolved.template)));

        Ok(ItemRenderState {
            item,
            reference,
            template: item_template.unwrap_or(Cow::Borrowed(&[])),
        })
    }

    fn try_render_integral_group_with_format<F>(
        &self,
        group: &[&crate::reference::CitationItem],
        spec: &citum_schema::CitationSpec,
        mode: &citum_schema::citation::CitationMode,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
    ) -> Result<Option<String>, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        if !matches!(mode, citum_schema::citation::CitationMode::Integral)
            || !self.has_explicit_integral_template()
        {
            return Ok(None);
        }

        self.render_integral_explicit_group::<F>(group, spec, mode, suppress_author, position)
    }

    /// Returns true for non-integral citation types that must render as a single
    /// unit via [`render_special_type_items`] rather than the split author+items
    /// path used for standard author-date groups.
    ///
    /// Title-first types (`legal-case`, `treaty`, `hearing`) need this because
    /// their type-variant template leads with a title component, not a
    /// contributor. The grouped path strips only `Contributor::Author`, so the
    /// title would render twice (plain in the author slot, emph in the item
    /// slot). `personal-communication` is included because its per-item date
    /// and term must stay together and not be collapsed across items.
    fn requires_full_group_item_rendering(
        &self,
        mode: &citum_schema::citation::CitationMode,
        reference: &Reference,
    ) -> bool {
        matches!(mode, citum_schema::citation::CitationMode::NonIntegral)
            && matches!(
                reference.ref_type().as_str(),
                "legal-case" | "treaty" | "hearing" | "personal-communication"
            )
    }

    /// Render just the author part for citation grouping.
    pub(crate) fn render_author_for_grouping_with_format<F>(
        &self,
        reference: &Reference,
        item: &crate::reference::CitationItem,
        template: &[TemplateComponent],
        mode: &citum_schema::citation::CitationMode,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
    ) -> String
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let is_note_processing = self.config.processing.as_ref().is_some_and(|processing| {
            matches!(processing, citum_schema::options::Processing::Note)
        });
        if is_note_processing
            && matches!(
                position,
                Some(
                    citum_schema::citation::Position::Ibid
                        | citum_schema::citation::Position::IbidWithLocator
                )
            )
            && !template.iter().any(has_contributor_component)
        {
            return String::new();
        }

        let locale = self.locale_for_reference(reference, RenderContext::Citation);
        let options = self.citation_render_options(
            locale.as_ref(),
            mode.clone(),
            suppress_author,
            None,
            None,
        );

        // Try to use the first semantically relevant component (including nested lists)
        // so disambiguation hints and component-specific formatting are preserved.
        // This ensures substitution, shortening, and mode-dependent conjunctions are respected.
        if let Some(comp) = template.first().and_then(find_grouping_component) {
            let base_hints = self
                .hints
                .get(reference.id().as_deref().unwrap_or_default())
                .cloned()
                .unwrap_or_default();
            // Inject citation position so subsequent et-al thresholds are applied.
            let hints = ProcHints {
                position: position.cloned(),
                integral_name_state: item.integral_name_state,
                ..base_hints
            };
            if let Some(vals) = comp.values::<F>(reference, &hints, &options)
                && !vals.value.is_empty()
            {
                return vals.value;
            }
        }

        // Fallback for cases where first component isn't suitable or returned empty
        if let Some(authors) = reference.author() {
            let names_vec = self.resolve_contributor_names(&authors);
            F::default().text(&crate::values::format_contributors_short(
                &names_vec, &options,
            ))
        } else {
            String::new()
        }
    }

    /// Render the prose anchor for an integral citation without any trailing note text.
    pub(crate) fn render_integral_anchor_with_format<F>(
        &self,
        items: &[crate::reference::CitationItem],
        spec: &citum_schema::CitationSpec,
        inter_delimiter: &str,
        suppress_author: bool,
        position: Option<&citum_schema::citation::Position>,
    ) -> Result<String, ProcessorError>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let groups = group_citation_items_by_author(self, items);

        let mut rendered_groups = Vec::new();
        let fmt = F::default();
        for (_author_key, group) in groups {
            #[allow(
                clippy::indexing_slicing,
                reason = "group is non-empty by construction"
            )]
            let first_item = group[0];
            let reference = self
                .bibliography
                .get(&first_item.id)
                .ok_or_else(|| ProcessorError::ReferenceNotFound(first_item.id.clone()))?;
            let item_language = crate::values::effective_item_language(reference);
            let template = spec.resolve_template_for_language(item_language.as_deref());
            let effective_template = template.as_deref().unwrap_or(&[]);
            let author_part = self.render_author_for_grouping_with_format::<F>(
                reference,
                first_item,
                effective_template,
                &citum_schema::citation::CitationMode::Integral,
                suppress_author,
                position,
            );
            if !author_part.is_empty() {
                rendered_groups.push(author_part);
            }
        }

        Ok(fmt.join(rendered_groups, inter_delimiter))
    }

    /// Get the citation number for a reference, assigning one if not yet cited.
    #[must_use]
    pub fn get_or_assign_citation_number(&self, ref_id: &str) -> usize {
        let mut numbers = self
            .citation_numbers
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let next_num = numbers.len() + 1;
        *numbers.entry(ref_id.to_string()).or_insert(next_num)
    }

    /// Process a bibliography entry.
    #[must_use]
    pub fn process_bibliography_entry(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate> {
        self.process_bibliography_entry_with_format::<crate::render::plain::PlainText>(
            reference,
            entry_number,
        )
    }

    /// Render the reference marker one bibliography entry leads with, if the
    /// style declares one.
    ///
    /// The marker is not a template component, so it never reaches the template
    /// pipeline; entry assembly writes it ahead of the body.
    /// See `docs/specs/REFERENCE_MARKERS.md`.
    #[must_use]
    pub(crate) fn bibliography_marker_with_format<F>(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<String>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let spec =
            super::super::marker::resolve_bibliography_marker(self.bibliography_config.as_deref())?;
        let ref_id = reference.id().unwrap_or_default().to_string();
        let value = super::super::marker::marker_value(
            spec.kind,
            &self.config,
            reference,
            Some(entry_number),
            None,
            self.hints.get(&ref_id),
        )?;
        let fmt = F::default();
        Some(self.present_bibliography_marker_with_format(&fmt, &spec, &value, Some(&ref_id)))
    }

    /// Process a bibliography entry with specific format.
    #[must_use]
    pub fn process_bibliography_entry_with_format<F>(
        &self,
        reference: &Reference,
        entry_number: usize,
    ) -> Option<ProcTemplate>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let bib_spec = self.style.bibliography.as_ref()?;

        let item_language = crate::values::effective_item_language(reference);
        let ref_type = reference.ref_type();
        let localized = bib_spec.resolve_localized_template(item_language.as_deref());
        let template = localized
            .as_ref()
            .filter(|resolved| resolved.type_variants.is_some())
            .cloned()
            .map(|resolved| {
                Cow::Owned(resolve_localized_type_variant(
                    resolved,
                    bib_spec.type_variants.as_ref(),
                    &ref_type,
                ))
            })
            .or_else(|| {
                resolve_type_variant(bib_spec.type_variants.as_ref(), &ref_type).map(Cow::Borrowed)
            })
            .or_else(|| localized.map(|resolved| Cow::Owned(resolved.template)))?;

        let template = self.apply_anonymous_entry_bibliography_policy(reference, template)?;
        let template = self.apply_article_journal_bibliography_policy(reference, template);

        self.process_template_request_with_format::<F>(
            reference,
            TemplateRenderRequest {
                template: template.as_ref(),
                context: RenderContext::Bibliography,
                mode: citum_schema::citation::CitationMode::NonIntegral,
                suppress_author: false,
                locator_raw: None,
                citation_number: entry_number,
                position: None,
                note_start_text_case: None,
                integral_name_state: None,
                org_abbreviation_state: None,
                first_reference_note_number: None,
            },
        )
    }

    /// Process a template for a reference using plain text format.
    ///
    /// Accepts a [`TemplateRenderParams`] bundle rather than individual arguments
    /// to keep the call site readable and avoid argument-count lint issues.
    #[must_use]
    pub fn process_template_with_number(
        &self,
        reference: &Reference,
        params: TemplateRenderParams<'_>,
    ) -> Option<ProcTemplate> {
        self.process_template_with_number_with_format::<crate::render::plain::PlainText>(
            reference, params,
        )
    }

    /// Process a template for a reference with a specific output format.
    ///
    /// Accepts a [`TemplateRenderParams`] bundle rather than individual arguments
    /// to keep the call site readable and avoid argument-count lint issues.
    pub fn process_template_with_number_with_format<F>(
        &self,
        reference: &Reference,
        params: TemplateRenderParams<'_>,
    ) -> Option<ProcTemplate>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        self.process_template_request_with_format::<F>(
            reference,
            TemplateRenderRequest {
                template: params.template,
                context: params.context,
                mode: params.mode,
                suppress_author: params.suppress_author,
                locator_raw: params.locator_raw,
                citation_number: params.citation_number,
                position: params.position.cloned(),
                note_start_text_case: params.note_start_text_case,
                integral_name_state: params.integral_name_state,
                org_abbreviation_state: params.org_abbreviation_state,
                first_reference_note_number: None,
            },
        )
    }

    /// Process a template request with a specific output format.
    #[must_use]
    pub fn process_template_request_with_format<F>(
        &self,
        reference: &Reference,
        request: TemplateRenderRequest<'_>,
    ) -> Option<ProcTemplate>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let TemplateRenderRequest {
            template,
            context,
            mode,
            suppress_author,
            locator_raw,
            citation_number,
            position,
            note_start_text_case,
            integral_name_state,
            org_abbreviation_state,
            first_reference_note_number,
        } = request;
        let ref_type = reference.ref_type();
        let locale = self.locale_for_reference(reference, context);
        let options = RenderOptions {
            config: self.config.clone(),
            bibliography_config: self.bibliography_config.clone(),
            locale: locale.as_ref(),
            context,
            mode,
            suppress_author,
            locator_raw,
            ref_type: Some(ref_type.clone()),
            show_semantics: self.show_semantics,
            current_template_index: None,
            abbreviation_map: self.abbreviation_map,
        };
        // Only carry the first-reference note number (and its suppression side-effect)
        // when the template actually renders it.  Suppressing a `disambiguate-only`
        // title without emitting the note number as a replacement identifier would
        // silently reintroduce ambiguity for colliding works.
        let effective_first_ref_note = if template_uses_first_ref_note_number(template) {
            first_reference_note_number
        } else {
            None
        };
        let hint = self.build_template_render_hint(HintInputs {
            reference,
            context: options.context,
            citation_number,
            position,
            integral_name_state,
            org_abbreviation_state,
            first_reference_note_number: effective_first_ref_note,
        });
        let mut components =
            self.render_template_components::<F>(reference, &ref_type, &options, &hint, template);

        self.apply_sentence_initial_context::<F>(&mut components, context, note_start_text_case);

        (!components.is_empty()).then_some(components)
    }

    /// Render each top-level template component for `reference`, threading a
    /// fresh `TemplateRenderContext` per index so the source position is
    /// preserved in AST-injection mode.
    fn render_template_components<F>(
        &self,
        reference: &Reference,
        ref_type: &str,
        options: &RenderOptions<'_>,
        hint: &ProcHints,
        template: &[TemplateComponent],
    ) -> Vec<ProcTemplateComponent>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let mut tracker = TemplateComponentTracker::default();
        let mut components = Vec::with_capacity(template.len());
        let mut component_options = options.clone();
        for (template_index, component) in template.iter().enumerate() {
            component_options.current_template_index =
                self.inject_ast_indices.then_some(template_index);
            let ctx = TemplateRenderContext {
                reference,
                ref_type,
                options: &component_options,
                hint,
                template_index,
            };
            if let Some(component) =
                self.render_template_component_with_format::<F>(&ctx, component, &mut tracker)
            {
                components.push(component);
            }
        }
        components
    }

    fn build_template_render_hint(&self, inputs: HintInputs<'_>) -> ProcHints {
        let HintInputs {
            reference,
            context,
            citation_number,
            position,
            integral_name_state,
            org_abbreviation_state,
            first_reference_note_number,
        } = inputs;
        let default_hint = ProcHints::default();
        let base_hint = self
            .hints
            .get(reference.id().as_deref().unwrap_or_default())
            .unwrap_or(&default_hint);
        let is_subsequent = matches!(position, Some(citum_schema::citation::Position::Subsequent));
        ProcHints {
            citation_number: (citation_number > 0).then_some(citation_number),
            citation_sub_label: if context == RenderContext::Citation {
                reference
                    .id()
                    .as_deref()
                    .and_then(|id| self.citation_sub_label_for_ref(id))
            } else {
                None
            },
            position,
            integral_name_state,
            org_abbreviation_state,
            first_reference_note_number: if is_subsequent {
                first_reference_note_number
            } else {
                None
            },
            suppress_disambiguation_title: is_subsequent && first_reference_note_number.is_some(),
            ..base_hint.clone()
        }
    }

    fn render_template_component_with_format<F>(
        &self,
        ctx: &TemplateRenderContext<'_>,
        component: &TemplateComponent,
        tracker: &mut TemplateComponentTracker,
    ) -> Option<ProcTemplateComponent>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        if let TemplateComponent::Group(group) = component {
            return self.render_group_component_with_format::<F>(ctx, group, tracker);
        }

        let resolved_component = component;
        let mut component_hint = ctx.hint.clone();
        if matches!(
            resolved_component,
            TemplateComponent::Date(citum_schema::template::TemplateDate {
                date: citum_schema::template::DateVariable::Issued,
                ..
            })
        ) {
            component_hint.date_fallback_first_issued = Some(tracker.next_issued_is_first());
        }
        if resolved_component.rendering().suppress == Some(true) {
            return None;
        }

        let var_key = get_variable_key(resolved_component);
        if tracker.should_skip(var_key.as_deref()) {
            return None;
        }

        let mut values =
            resolved_component.values::<F>(ctx.reference, &component_hint, ctx.options)?;
        // Suppress affixes when a component resolves to no meaningful content.
        // A whitespace-only value carries no data, so its prefix/suffix must
        // not leak into output (e.g. a ". In " prefix on an empty editor list).
        if values.value.trim().is_empty() {
            return None;
        }
        self.apply_entry_link_fallback(ctx.reference, ctx.options, &mut values);

        let item_language =
            crate::values::effective_component_language(ctx.reference, resolved_component);
        tracker.mark_rendered(var_key, values.substituted_key.as_deref());

        Some(ProcTemplateComponent {
            template_component: resolved_component.clone(),
            template_index: self.inject_ast_indices.then_some(ctx.template_index),
            value: values.value,
            prefix: values.prefix,
            suffix: values.suffix,
            url: values.url,
            ref_type: Some(ctx.ref_type.to_string()),
            config: Some(ctx.options.config.clone()),
            bibliography_config: ctx.options.bibliography_config.clone(),
            item_language,
            quote_marks: crate::render::format::QuoteMarks::from(ctx.options.locale),
            sentence_initial: false,
            pre_formatted: values.pre_formatted,
        })
    }

    fn render_group_component_with_format<F>(
        &self,
        ctx: &TemplateRenderContext<'_>,
        group: &citum_schema::template::TemplateGroup,
        tracker: &mut TemplateComponentTracker,
    ) -> Option<ProcTemplateComponent>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        if group.rendering.suppress == Some(true) {
            return None;
        }
        if group.render_when.as_ref().is_some_and(|condition| {
            !crate::values::group_condition_matches(ctx.reference, condition)
        }) {
            return None;
        }

        let fmt = F::default();
        let mut group_tracker = tracker.clone();
        let values = self.render_group_child_values(&fmt, ctx, group, &mut group_tracker);
        tracker.merge_from(group_tracker);
        let values = values?;
        let default_delimiter = citum_schema::template::DelimiterPunctuation::Comma;
        let punctuation = group.delimiter.as_ref().unwrap_or(&default_delimiter);
        let (script, realization) = crate::values::punctuation_realization_context(
            crate::values::effective_item_language(ctx.reference).as_deref(),
            ctx.options.config.multilingual.as_ref(),
            ctx.options.locale.punctuation_realization.as_ref(),
        );
        let delimiter = crate::render::format::realize_punctuation(
            punctuation,
            script,
            realization.as_deref(),
            crate::render::format::PunctuationPosition::Separator,
        );
        let delimiter = if punctuation.is_semantic() {
            fmt.text(&delimiter)
        } else {
            delimiter.into_owned()
        };
        // Joining two empty strings surfaces any format-specific escaping
        // `fmt.join` would apply to the delimiter itself (e.g. LaTeX special
        // characters), so the boundary-aware join below sees the same
        // delimiter text `fmt.join` would have inserted.
        let escaped_delimiter = fmt.join(vec![String::new(), String::new()], &delimiter);
        let escaped_delimiter =
            crate::render::format::RealizedPunctuation::new(escaped_delimiter.into());
        let close_quote = crate::render::format::QuoteMarks::from(ctx.options.locale).close;
        let joined_value = crate::render::punctuation::join_with_quote_movement::<F>(
            values,
            &escaped_delimiter,
            ctx.options.config.punctuation_in_quote,
            &close_quote,
        );
        let group_component = TemplateComponent::Group(group.clone());
        Some(ProcTemplateComponent {
            template_component: group_component.clone(),
            template_index: self.inject_ast_indices.then_some(ctx.template_index),
            value: joined_value,
            prefix: None,
            suffix: None,
            url: None,
            ref_type: Some(ctx.ref_type.to_string()),
            config: Some(ctx.options.config.clone()),
            bibliography_config: ctx.options.bibliography_config.clone(),
            item_language: crate::values::effective_component_language(
                ctx.reference,
                &group_component,
            ),
            quote_marks: crate::render::format::QuoteMarks::from(ctx.options.locale),
            sentence_initial: false,
            pre_formatted: true,
        })
    }

    /// Render the children of a template group into rendered strings, dropping
    /// empty values. Returns `None` when no child carries meaningful content
    /// (i.e. only term-only siblings produced output) — except for the
    /// bibliography numeric-label pattern (the renderer's synthetic
    /// `[label, following]` group), where the label alone is still real
    /// content that must render even when `following` is empty (e.g. no
    /// author); the second element of the returned tuple flags that case so
    /// the caller can mark the resulting component `label_only` instead of
    /// treating it as ordinary preceding content for separator purposes.
    /// Borrows the parent `fmt` so a stateful `OutputFormat` sees a single
    /// instance for both child rendering and the final `join` in the caller.
    fn render_group_child_values<F>(
        &self,
        fmt: &F,
        ctx: &TemplateRenderContext<'_>,
        group: &citum_schema::template::TemplateGroup,
        tracker: &mut TemplateComponentTracker,
    ) -> Option<Vec<crate::render::component::RenderedComponent>>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let mut has_meaningful_content = false;
        let mut values = Vec::with_capacity(group.group.len());

        for item in &group.group {
            let Some(rendered) =
                self.render_template_component_with_format::<F>(ctx, item, tracker)
            else {
                continue;
            };
            let rendered_detailed =
                crate::render::component::render_component_detailed_with_format_and_renderer::<F>(
                    &rendered,
                    fmt,
                    ctx.options.show_semantics,
                );
            if rendered_detailed.text.trim().is_empty() {
                continue;
            }
            if !is_term_only_component(item) {
                has_meaningful_content = true;
            }
            values.push(rendered_detailed);
        }

        if values.is_empty() || !has_meaningful_content {
            return None;
        }
        Some(values)
    }

    fn apply_entry_link_fallback(
        &self,
        reference: &Reference,
        options: &RenderOptions<'_>,
        values: &mut crate::values::ProcValues<String>,
    ) {
        if values.url.is_some() {
            return;
        }

        let Some(links) = &options.config.links else {
            return;
        };
        use citum_schema::options::LinkAnchor;
        if matches!(links.anchor, Some(LinkAnchor::Entry)) {
            values.url = crate::values::resolve_url(links, reference);
        }
    }

    /// Apply the substitution string to the primary contributor component.
    pub fn apply_author_substitution(&self, proc: &mut ProcTemplate, substitute: &str) {
        self.apply_author_substitution_with_format::<crate::render::plain::PlainText>(
            proc, substitute,
        );
    }

    /// Apply the substitution string to the primary contributor component with specific format.
    pub fn apply_author_substitution_with_format<F>(
        &self,
        proc: &mut ProcTemplate,
        substitute: &str,
    ) where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        if let Some(component) = proc
            .iter_mut()
            .find(|c| matches!(c.template_component, TemplateComponent::Contributor(_)))
        {
            let fmt = F::default();
            component.value = fmt.text(substitute);
        }
    }

    fn render_group_item_from_template_with_format<F>(
        &self,
        reference: &Reference,
        item_request: GroupItemRenderRequest<'_>,
    ) -> Option<String>
    where
        F: crate::render::format::OutputFormat<Output = String>,
    {
        let request = self.citation_render_request(
            item_request.item,
            item_request.template,
            item_request.mode,
            item_request.suppress_author,
            item_request.position,
            item_request.note_start_text_case,
        );
        self.render_item_from_template_with_format::<F>(reference, request, item_request.delimiter)
    }
}

/// Return `true` when `template` (or any nested group) contains a
/// `number: first-reference-note-number` component.
///
/// Used to gate `suppress_disambiguation_title`: if the style's template does
/// not render the note-number identifier, there is nothing to replace the
/// suppressed title and ambiguity would silently be reintroduced.
pub(super) fn template_uses_first_ref_note_number(template: &[TemplateComponent]) -> bool {
    template.iter().any(|c| match c {
        TemplateComponent::Number(n) => {
            n.number == citum_schema::template::NumberVariable::FirstReferenceNoteNumber
        }
        TemplateComponent::Group(g) => template_uses_first_ref_note_number(&g.group),
        _ => false,
    })
}

pub(super) fn filter_author_from_template<F>(
    template: &[TemplateComponent],
    script: crate::values::ScriptClass,
    realization: Option<&citum_schema::options::PunctuationRealization>,
    fmt: &F,
) -> (Vec<TemplateComponent>, Option<String>, bool)
where
    F: crate::render::format::OutputFormat<Output = String>,
{
    // The author part rendered by `render_author_for_grouping_with_format`
    // is the first grouping component of the leading template component —
    // any contributor role, not just author. Strip that exact contributor
    // from the item parts too, or a template leading with e.g. a translator
    // renders its names twice (once as author part, once in the item part).
    let grouping_role = template
        .first()
        .and_then(find_grouping_component)
        .and_then(|component| match component {
            TemplateComponent::Contributor(contributor)
                if contributor.contributor != citum_schema::template::ContributorRole::Author =>
            {
                Some(contributor.contributor.clone())
            }
            _ => None,
        });
    let mut filtered: Vec<TemplateComponent> =
        template.iter().filter_map(strip_author_component).collect();
    if let Some(role) = grouping_role
        && !filtered.is_empty()
    {
        let first = filtered.remove(0);
        if let (Some(remaining), _) = remove_first_contributor_with_role(first, &role) {
            filtered.insert(0, remaining);
        }
    }
    let stripped_leading_affix = filtered
        .first()
        .and_then(|first| leading_group_affix(first, script, realization, fmt));
    let leading_affix = stripped_leading_affix.clone().or_else(|| {
        filtered.first().and_then(|_| {
            template
                .first()
                .and_then(|first| author_group_delimiter_affix(first, script, realization, fmt))
        })
    });
    if let Some(first) = filtered.first_mut() {
        strip_leading_group_affixes(first);
    }
    (filtered, leading_affix, stripped_leading_affix.is_some())
}

fn author_group_delimiter_affix<F>(
    component: &TemplateComponent,
    script: crate::values::ScriptClass,
    realization: Option<&citum_schema::options::PunctuationRealization>,
    fmt: &F,
) -> Option<String>
where
    F: crate::render::format::OutputFormat<Output = String>,
{
    let TemplateComponent::Group(group) = component else {
        return None;
    };
    group
        .group
        .first()
        .is_some_and(component_starts_with_author)
        .then_some(group.delimiter.as_ref())
        .flatten()
        .map(|punctuation| {
            let realized = crate::render::format::realize_punctuation(
                punctuation,
                script,
                realization,
                crate::render::format::PunctuationPosition::Separator,
            );
            if punctuation.is_semantic() {
                fmt.text(&realized)
            } else {
                realized.into_owned()
            }
        })
        .filter(|delimiter| !delimiter.is_empty())
}

fn component_starts_with_author(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Contributor(contributor) => contributor
            .contributor
            .contains(&citum_schema::template::ContributorRole::Author),
        TemplateComponent::Group(group) => group
            .group
            .first()
            .is_some_and(component_starts_with_author),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citum_schema::template::{
        ContributorRole, DelimiterPunctuation, TemplateContributor, TemplateGroup,
    };

    #[test]
    fn author_group_delimiter_affix_recognizes_merged_leading_author_component() {
        // given a group whose leading component is a merged [author, editor]
        // contributor list rather than a scalar author component
        let group = TemplateComponent::Group(TemplateGroup {
            group: vec![TemplateComponent::Contributor(TemplateContributor {
                contributor: vec![ContributorRole::Author, ContributorRole::Editor].into(),
                ..Default::default()
            })],
            delimiter: Some(DelimiterPunctuation::Comma),
            ..Default::default()
        });

        // when resolving the leading author-group delimiter affix
        let affix = author_group_delimiter_affix(
            &group,
            crate::values::ScriptClass::Latin,
            None,
            &crate::render::plain::PlainText,
        );

        // then the merged component is recognized as starting with author
        assert_eq!(affix, Some(", ".to_string()));
    }

    #[test]
    fn author_group_delimiter_affix_ignores_merged_component_without_author() {
        // given a group whose leading component is a merged [editor,
        // translator] contributor list that never declares author
        let group = TemplateComponent::Group(TemplateGroup {
            group: vec![TemplateComponent::Contributor(TemplateContributor {
                contributor: vec![ContributorRole::Editor, ContributorRole::Translator].into(),
                ..Default::default()
            })],
            delimiter: Some(DelimiterPunctuation::Comma),
            ..Default::default()
        });

        // when resolving the leading author-group delimiter affix
        let affix = author_group_delimiter_affix(
            &group,
            crate::values::ScriptClass::Latin,
            None,
            &crate::render::plain::PlainText,
        );

        // then no affix is produced since the group does not start with author
        assert_eq!(affix, None);
    }
}
