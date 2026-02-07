use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{DelimiterPunctuation, TemplateList};

impl ComponentValues for TemplateList {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        // Collect values from all items, applying their rendering
        let values: Vec<String> = self
            .items
            .iter()
            .filter_map(|item| {
                let v = item.values(reference, hints, options)?;
                if v.value.is_empty() {
                    return None;
                }

                // Use the central rendering logic to apply global config, local settings, and overrides
                let proc_item = crate::render::ProcTemplateComponent {
                    template_component: item.clone(),
                    value: v.value,
                    prefix: v.prefix,
                    suffix: v.suffix,
                    url: v.url,
                    ref_type: Some(reference.ref_type()),
                    config: Some(options.config.clone()),
                };

                let rendered = crate::render::render_component(&proc_item);
                if rendered.is_empty() {
                    None
                } else {
                    Some(rendered)
                }
            })
            .collect();

        if values.is_empty() {
            return None;
        }

        // Join with delimiter
        let delimiter = self
            .delimiter
            .as_ref()
            .unwrap_or(&DelimiterPunctuation::Comma)
            .to_string_with_space();

        Some(ProcValues {
            value: values.join(&delimiter),
            prefix: self.rendering.prefix.clone(),
            suffix: self.rendering.suffix.clone(),
            url: None,
            substituted_key: None,
        })
    }
}
