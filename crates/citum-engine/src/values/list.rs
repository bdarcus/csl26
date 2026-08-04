/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for list components with configurable delimiters.
//!
//! This module handles rendering of lists of template items, with support for
//! different delimiters between items (commas, semicolons, etc.) and rendering modes.

use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use citum_schema::template::{DelimiterPunctuation, TemplateGroup};

impl ComponentValues for TemplateGroup {
    /// Returns the resolved list of component values for rendering.
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let mut has_content = false;
        let fmt = F::default();

        // Collect values from all items, applying their rendering
        let mut values = Vec::with_capacity(self.group.len());
        for item in &self.group {
            let Some(v) = item.values::<F>(reference, hints, options) else {
                continue;
            };
            if v.value.is_empty() {
                continue;
            }

            // Track if we have any "meaningful" content (not just a term)
            if !is_term_based(item) {
                has_content = true;
            }

            // Use the central rendering logic to apply global config, local settings, and overrides
            let proc_item = crate::render::ProcTemplateComponent {
                template_component: item.clone(),
                template_index: options.current_template_index,
                value: v.value,
                prefix: v.prefix,
                suffix: v.suffix,
                url: v.url,
                ref_type: Some(reference.ref_type().clone()),
                config: Some(options.config.clone()),
                bibliography_config: options.bibliography_config.clone(),
                item_language: crate::values::effective_component_language(reference, item),
                quote_marks: crate::render::format::QuoteMarks::from(options.locale),
                sentence_initial: false,
                pre_formatted: v.pre_formatted,
            };

            let rendered =
                crate::render::component::render_component_detailed_with_format_and_renderer::<F>(
                    &proc_item,
                    &fmt,
                    options.show_semantics,
                );
            if !rendered.text.is_empty() {
                values.push(rendered);
            }
        }

        if values.is_empty() || !has_content {
            return None;
        }

        // Join with delimiter
        let default_delimiter = DelimiterPunctuation::Comma;
        let punctuation = self.delimiter.as_ref().unwrap_or(&default_delimiter);
        let (script, realization) = crate::values::punctuation_realization_context(
            crate::values::effective_item_language(reference).as_deref(),
            options.config.multilingual.as_ref(),
            options.locale.punctuation_realization.as_ref(),
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
        // `fmt.join` may apply format-specific escaping to the delimiter itself
        // (e.g. LaTeX special characters); joining two empty strings surfaces
        // that transform on `delimiter` alone, so the boundary-aware join below
        // sees the same delimiter text `fmt.join` would have inserted.
        let escaped_delimiter = fmt.join(vec![String::new(), String::new()], &delimiter);
        let escaped_delimiter =
            crate::render::format::RealizedPunctuation::new(escaped_delimiter.into());

        let close_quote = crate::render::format::QuoteMarks::from(options.locale).close;
        let joined = crate::render::punctuation::join_with_quote_movement::<F>(
            values,
            &escaped_delimiter,
            options.config.punctuation_in_quote,
            &close_quote,
        );

        Some(ProcValues {
            value: joined,
            prefix: None,
            suffix: None,
            url: None,
            substituted_key: None,
            pre_formatted: true,
        })
    }
}

/// Returns true if the component value is derived from a locale term rather than a data field.
fn is_term_based(component: &citum_schema::template::TemplateComponent) -> bool {
    use citum_schema::template::TemplateComponent;
    match component {
        TemplateComponent::Term(_) => true,
        TemplateComponent::Message(message) if message.message.starts_with("term.") => true,
        TemplateComponent::Group(l) => l.group.iter().all(is_term_based),
        _ => false,
    }
}
