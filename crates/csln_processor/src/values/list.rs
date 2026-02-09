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
        let mut has_content = false;

        // Collect values from all items, applying their rendering
        let values: Vec<String> = self
            .items
            .iter()
            .filter_map(|item| {
                let v = item.values(reference, hints, options)?;
                if v.value.is_empty() {
                    return None;
                }

                // Track if we have any "meaningful" content (not just a term)
                if !is_term_based(item) {
                    has_content = true;
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

        if values.is_empty() || !has_content {
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
            prefix: None,
            suffix: None,
            url: None,
            substituted_key: None,
        })
    }
}

/// Check if a component is purely term-based or a list of such.
fn is_term_based(component: &csln_core::template::TemplateComponent) -> bool {
    use csln_core::template::TemplateComponent;
    match component {
        TemplateComponent::Term(_) => true,
        TemplateComponent::List(l) => l.items.iter().all(is_term_based),
        _ => false,
    }
}
