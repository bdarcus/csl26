use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{SimpleVariable, TemplateVariable};

impl ComponentValues for TemplateVariable {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        // Apply visibility filter: suppress only if not in integral mode
        if matches!(
            options.visibility,
            csln_core::citation::ItemVisibility::AuthorOnly
        ) && options.mode != csln_core::citation::CitationMode::Integral
        {
            return None;
        }

        let value = match self.variable {
            SimpleVariable::Doi => reference.doi(),
            SimpleVariable::Url => reference.url().map(|u| u.to_string()),
            SimpleVariable::Isbn => reference.isbn(),
            SimpleVariable::Issn => reference.issn(),
            SimpleVariable::Publisher => reference.publisher_str(),
            SimpleVariable::PublisherPlace => reference.publisher_place(),
            SimpleVariable::Genre => reference.genre(),
            SimpleVariable::Abstract => reference.abstract_text(),
            SimpleVariable::Locator => {
                // If we have a locator value in options, use it
                options.locator.map(|loc| {
                    if let Some(label_type) = &options.locator_label {
                        // Check if value is plural (contains hyphen, comma, or space)
                        let is_plural = loc.contains('-') || loc.contains(',') || loc.contains(' ');

                        // Look up term from locale
                        if let Some(term) = options.locale.locator_term(
                            label_type,
                            is_plural,
                            csln_core::locale::TermForm::Short,
                        ) {
                            format!("{} {}", term, loc)
                        } else {
                            loc.to_string()
                        }
                    } else {
                        loc.to_string()
                    }
                })
            }
            SimpleVariable::Infix => options.infix.map(|s| s.to_string()),
            _ => None,
        };

        value.filter(|s: &String| !s.is_empty()).map(|value| {
            use csln_core::options::{LinkAnchor, LinkTarget};
            let component_anchor = match self.variable {
                SimpleVariable::Url => LinkAnchor::Url,
                SimpleVariable::Doi => LinkAnchor::Doi,
                _ => LinkAnchor::Component,
            };

            let mut url = crate::values::resolve_effective_url(
                self.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                component_anchor,
            );

            // Fallback for simple legacy config
            if url.is_none() {
                if let Some(links) = &self.links {
                    if self.variable == SimpleVariable::Url
                        && (links.url == Some(true)
                            || matches!(links.target, Some(LinkTarget::Url | LinkTarget::UrlOrDoi)))
                    {
                        url = reference.url().map(|u| u.to_string());
                    } else if self.variable == SimpleVariable::Doi
                        && (links.doi == Some(true)
                            || matches!(links.target, Some(LinkTarget::Doi | LinkTarget::UrlOrDoi)))
                    {
                        url = reference.doi().map(|d| format!("https://doi.org/{}", d));
                    }
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
