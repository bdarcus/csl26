/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::locale::TermForm;
use csln_core::template::TemplateTerm;

impl ComponentValues for TemplateTerm {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        // Resolve effective rendering options (base merged with type-specific override)
        let mut effective_rendering = self.rendering.clone();
        if let Some(overrides) = &self.overrides {
            use csln_core::template::ComponentOverride;
            let ref_type = reference.ref_type();
            let mut match_found = false;
            for (selector, ov) in overrides {
                if selector.matches(&ref_type) {
                    if let ComponentOverride::Rendering(r) = ov {
                        effective_rendering.merge(r);
                        match_found = true;
                    }
                }
            }
            if !match_found {
                for (selector, ov) in overrides {
                    if selector.matches("default") {
                        if let ComponentOverride::Rendering(r) = ov {
                            effective_rendering.merge(r);
                        }
                    }
                }
            }
        }

        let form = self.form.unwrap_or(TermForm::Long);
        let mut value = options
            .locale
            .general_term(&self.term, form)
            .unwrap_or("")
            .to_string();

        // Apply strip-periods if configured
        if crate::values::should_strip_periods(&effective_rendering, options) {
            value = crate::values::strip_trailing_periods(&value);
        }

        if value.is_empty() {
            None
        } else {
            Some(ProcValues {
                value,
                pre_formatted: false,
                ..Default::default()
            })
        }
    }
}
