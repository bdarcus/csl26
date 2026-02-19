use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{SimpleVariable, TemplateVariable};

impl ComponentValues for TemplateVariable {
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let value = match self.variable {
            SimpleVariable::Doi => reference.doi(),
            SimpleVariable::Url => reference.url().map(|u| u.to_string()),
            SimpleVariable::Isbn => reference.isbn(),
            SimpleVariable::Issn => reference.issn(),
            SimpleVariable::Publisher => reference.publisher_str(),
            SimpleVariable::PublisherPlace => reference.publisher_place(),
            SimpleVariable::Genre => reference.genre(),
            SimpleVariable::Medium => reference.medium(),
            SimpleVariable::Abstract => reference.abstract_text(),
            SimpleVariable::Note => reference.note(),
            SimpleVariable::Authority => reference.authority(),
            SimpleVariable::Reporter => reference.reporter(),
            SimpleVariable::Page => reference.pages().map(|v| v.to_string()),
            SimpleVariable::Volume => reference.volume().map(|v| v.to_string()),
            SimpleVariable::Number => reference.number(),
            SimpleVariable::DocketNumber => match reference {
                Reference::Brief(r) => r.docket_number.clone(),
                _ => None,
            },
            SimpleVariable::PatentNumber => match reference {
                Reference::Patent(r) => Some(r.patent_number.clone()),
                _ => None,
            },
            SimpleVariable::StandardNumber => match reference {
                Reference::Standard(r) => Some(r.standard_number.clone()),
                _ => None,
            },
            SimpleVariable::ReportNumber => match reference {
                Reference::Monograph(r) => r.report_number.clone(),
                _ => None,
            },
            SimpleVariable::Version => reference.version(),
            SimpleVariable::Locator => {
                // If we have a locator value in options, use it
                options.locator.map(|loc| {
                    if let Some(label_type) = &options.locator_label {
                        // Page locators are typically rendered bare ("23") in note and
                        // author-date contexts; section/chapter/etc. keep labels ("sec. 2").
                        if matches!(label_type, csln_core::citation::LocatorType::Page) {
                            return loc.to_string();
                        }

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
            _ => None,
        };

        value.filter(|s: &String| !s.is_empty()).map(|value| {
            // Resolve effective rendering options
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
                pre_formatted: false,
            }
        })
    }
}
