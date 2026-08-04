/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for locale-authored message components.
//!
//! A message component resolves each named argument through the normal template
//! component renderer, then asks the active locale to evaluate the selected
//! message ID.

use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use citum_schema::locale::{MessageArgs, MessageEvaluator, Mf2MessageEvaluator};
use citum_schema::template::{MessageArgSource, TemplateMessage};
use std::collections::HashMap;

impl ComponentValues for TemplateMessage {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let mut named = HashMap::with_capacity(self.args.len());

        for (name, source) in &self.args {
            let value = render_message_arg::<F>(source, reference, hints, options)?;
            if value.trim().is_empty() {
                return None;
            }
            named.insert(name.clone(), value);
        }

        let args = MessageArgs {
            named,
            ..MessageArgs::default()
        };
        let mut value = if let Some(pattern) = options.config.messages.get(&self.message) {
            Mf2MessageEvaluator.evaluate(pattern, &args)?
        } else {
            options.locale.resolve_template_message(
                &self.message,
                &args,
                self.form.as_ref(),
                self.gender.clone(),
            )?
        };

        if crate::values::should_strip_periods(&self.rendering, options) {
            value = crate::values::strip_trailing_periods(&value);
        }

        if let Some(tc) = self.rendering.text_case {
            value = crate::values::text_case::apply_text_case_with_language(
                &value,
                tc,
                Some(options.locale.locale.as_str()),
            );
        }

        if value.trim().is_empty() {
            return None;
        }
        let term_backed = self.message.starts_with("term.");

        Some(ProcValues {
            value,
            pre_formatted: !term_backed,
            ..Default::default()
        })
    }
}

fn render_message_arg<F: crate::render::format::OutputFormat<Output = String>>(
    source: &MessageArgSource,
    reference: &Reference,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
) -> Option<String> {
    if let MessageArgSource::Literal { literal } = source {
        return Some(literal.clone());
    }
    if let MessageArgSource::ReferenceType { .. } = source {
        return Some(reference.ref_type());
    }
    if let MessageArgSource::Carrier { carrier } = source {
        return Some(reference.medium().unwrap_or_else(|| {
            let online = reference.url().is_some()
                || reference.doi().is_some()
                || reference.identifier("cstr").is_some();
            if online {
                carrier.online.clone()
            } else {
                carrier.absent.clone()
            }
        }));
    }

    let component = source.as_template_component()?;
    if component.rendering().suppress == Some(true) {
        return None;
    }

    let values = component.values::<F>(reference, hints, options)?;
    if values.value.trim().is_empty() {
        return None;
    }

    let fmt = F::default();
    let proc_item = crate::render::ProcTemplateComponent {
        template_component: component.clone(),
        template_index: options.current_template_index,
        value: values.value,
        prefix: values.prefix,
        suffix: values.suffix,
        url: values.url,
        ref_type: Some(reference.ref_type().clone()),
        config: Some(options.config.clone()),
        bibliography_config: options.bibliography_config.clone(),
        item_language: crate::values::effective_component_language(reference, &component),
        quote_marks: crate::render::format::QuoteMarks::from(options.locale),
        sentence_initial: false,
        pre_formatted: values.pre_formatted,
    };

    let rendered = crate::render::render_component_with_format_and_renderer::<F>(
        &proc_item,
        &fmt,
        options.show_semantics,
    );
    if rendered.trim().is_empty() {
        None
    } else {
        Some(rendered)
    }
}
