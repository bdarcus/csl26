use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{TemplateTitle, TitleType};

impl ComponentValues for TemplateTitle {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        _options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let binding = reference.ref_type();

        let value = match self.title {
            TitleType::Primary => reference.title().map(|t| t.to_string()),
            TitleType::ParentSerial => {
                if matches!(
                    binding.as_str(),
                    "article-journal"
                        | "article-magazine"
                        | "article-newspaper"
                        | "article"
                        | "paper-conference"
                ) {
                    reference.container_title().map(|t| t.to_string())
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
                    reference.container_title().map(|t| t.to_string())
                } else {
                    None
                }
            }
            _ => None,
        };

        value.filter(|s| !s.is_empty()).map(|value| {
            let mut url = None;
            if let Some(links) = &self.links {
                if links.doi == Some(true) {
                    url = reference.doi().map(|d| format!("https://doi.org/{}", d));
                }
                if url.is_none() && links.url == Some(true) {
                    url = reference.url().map(|u| u.to_string());
                }
            }
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
