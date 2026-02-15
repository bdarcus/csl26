use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{TemplateTitle, TitleType};

impl ComponentValues for TemplateTitle {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        // Apply visibility filter
        if matches!(
            options.visibility,
            csln_core::citation::ItemVisibility::AuthorOnly
        ) {
            return None;
        }

        let binding = reference.ref_type();

        // Get the raw title based on type
        let raw_title = match self.title {
            TitleType::Primary => reference.title(),
            TitleType::ParentSerial => {
                if matches!(
                    binding.as_str(),
                    "article-journal"
                        | "article-magazine"
                        | "article-newspaper"
                        | "article"
                        | "paper-conference"
                ) {
                    reference.container_title()
                } else {
                    None
                }
            }
            TitleType::ParentMonograph => {
                if matches!(
                    binding.as_str(),
                    "chapter"
                        | "paper-conference"
                        | "entry"
                        | "entry-dictionary"
                        | "entry-encyclopedia"
                ) {
                    reference.container_title()
                } else {
                    None
                }
            }
            _ => None,
        };

        // Resolve multilingual title if configured
        let value = raw_title.map(|title| {
            use csln_core::reference::types::Title;

            match title {
                Title::Single(s) => s.clone(),
                Title::Multilingual(m) => {
                    let mode = options
                        .config
                        .multilingual
                        .as_ref()
                        .and_then(|ml| ml.title_mode.as_ref());
                    let preferred_script = options
                        .config
                        .multilingual
                        .as_ref()
                        .and_then(|ml| ml.preferred_script.as_ref());
                    let locale_str = "en"; // TODO: get from options.locale

                    let complex =
                        csln_core::reference::types::MultilingualString::Complex(m.clone());
                    crate::values::resolve_multilingual_string(
                        &complex,
                        mode,
                        preferred_script,
                        locale_str,
                    )
                }
                _ => title.to_string(),
            }
        });

        value.filter(|s| !s.is_empty()).map(|value| {
            use csln_core::options::LinkAnchor;
            let url = crate::values::resolve_effective_url(
                self.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                LinkAnchor::Title,
            );
            ProcValues {
                value,
                prefix: None,
                suffix: None,
                url,
                substituted_key: None,
            }
        })
    }
}
