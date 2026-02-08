use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{SimpleVariable, TemplateVariable};

impl ComponentValues for TemplateVariable {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        _options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let value = match self.variable {
            SimpleVariable::Doi => reference.doi(),
            SimpleVariable::Url => reference.url().map(|u| u.to_string()),
            SimpleVariable::Isbn => reference.isbn(),
            SimpleVariable::Issn => reference.issn(),
            SimpleVariable::Publisher => reference.publisher_str(),
            SimpleVariable::PublisherPlace => reference.publisher_place(),
            SimpleVariable::Genre => reference.genre(),
            SimpleVariable::Abstract => reference.abstract_text(),
            SimpleVariable::Locator => None, // PIN support handled separately in process_citation
            _ => None,
        };

        value.filter(|s: &String| !s.is_empty()).map(|value| {
            let mut url = None;
            if let Some(links) = &self.links {
                if links.doi == Some(true) {
                    url = reference
                        .doi()
                        .as_ref()
                        .map(|d| format!("https://doi.org/{}", d));
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
