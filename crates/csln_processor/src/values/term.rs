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
        _reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let form = self.form.unwrap_or(TermForm::Long);
        let mut value = options
            .locale
            .general_term(&self.term, form)
            .unwrap_or("")
            .to_string();

        // Apply strip-periods if configured
        if crate::values::should_strip_periods(&self.rendering, options) {
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
