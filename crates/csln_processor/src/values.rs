/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Value extraction for template components.
//!
//! This module provides the logic to extract formatted values from references
//! based on template component specifications.

use crate::reference::{DateVariable, Name, Reference};
use csln_core::locale::{Locale, TermForm};
use csln_core::options::{AndOptions, Config, DisplayAsSort, ShortenListOptions, SubstituteKey};
use csln_core::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable as TemplateDateVar,
    NumberVariable, SimpleVariable, TemplateComponent, TemplateContributor,
    TemplateDate, TemplateNumber, TemplateTitle, TemplateVariable, TitleType,
};

/// Processed values ready for rendering.
#[derive(Debug, Clone, Default)]
pub struct ProcValues {
    /// The primary formatted value.
    pub value: String,
    /// Optional prefix to prepend.
    pub prefix: Option<String>,
    /// Optional suffix to append.
    pub suffix: Option<String>,
}

/// Processing hints for disambiguation and grouping.
#[derive(Debug, Clone, Default)]
pub struct ProcHints {
    /// Whether this reference needs disambiguation.
    pub disamb_condition: bool,
    /// Index within the disambiguation group (1-based).
    pub group_index: usize,
    /// Total references in the group.
    pub group_length: usize,
    /// The grouping key (author-year).
    pub group_key: String,
}

/// Context for rendering (citation vs bibliography).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderContext {
    #[default]
    Citation,
    Bibliography,
}

/// Options for rendering.
pub struct RenderOptions<'a> {
    pub config: &'a Config,
    pub locale: &'a Locale,
    pub context: RenderContext,
}

/// Trait for extracting values from template components.
pub trait ComponentValues {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues>;
}

impl ComponentValues for TemplateComponent {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        match self {
            TemplateComponent::Contributor(c) => c.values(reference, hints, options),
            TemplateComponent::Date(d) => d.values(reference, hints, options),
            TemplateComponent::Title(t) => t.values(reference, hints, options),
            TemplateComponent::Number(n) => n.values(reference, hints, options),
            TemplateComponent::Variable(v) => v.values(reference, hints, options),
            TemplateComponent::List(_) => None, // TODO: implement
            _ => None, // Handle future non-exhaustive variants
        }
    }
}

impl ComponentValues for TemplateContributor {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let names = match self.contributor {
            ContributorRole::Author => reference.author.as_ref(),
            ContributorRole::Editor => reference.editor.as_ref(),
            ContributorRole::Translator => reference.translator.as_ref(),
            _ => None,
        };

        // Handle substitution if author is empty
        if names.map(|n| n.is_empty()).unwrap_or(true) && matches!(self.contributor, ContributorRole::Author) {
            if let Some(substitute) = &options.config.substitute {
                for key in &substitute.template {
                    match key {
                        SubstituteKey::Editor => {
                            if let Some(editors) = &reference.editor {
                                if !editors.is_empty() {
                                    let formatted = format_names(editors, &self.form, options);
                                    // Add role suffix if configured
                                    let suffix = substitute.contributor_role_form.as_ref().map(|_| " (Ed.)".to_string());
                                    return Some(ProcValues {
                                        value: formatted,
                                        prefix: None,
                                        suffix,
                                    });
                                }
                            }
                        }
                        SubstituteKey::Title => {
                            if let Some(title) = &reference.title {
                                return Some(ProcValues {
                                    value: title.clone(),
                                    prefix: None,
                                    suffix: None,
                                });
                            }
                        }
                        SubstituteKey::Translator => {
                            if let Some(translators) = &reference.translator {
                                if !translators.is_empty() {
                                    let formatted = format_names(translators, &self.form, options);
                                    return Some(ProcValues {
                                        value: formatted,
                                        prefix: None,
                                        suffix: Some(" (Trans.)".to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            return None;
        }

        let names = names?;
        if names.is_empty() {
            return None;
        }

        let formatted = format_names(names, &self.form, options);
        
        // Add role label suffix for verb forms (e.g., "Name (Ed.)")
        let suffix = match (&self.form, &self.contributor) {
            (ContributorForm::Verb | ContributorForm::VerbShort, role) => {
                let plural = names.len() > 1;
                let form = match self.form {
                    ContributorForm::VerbShort => TermForm::Short,
                    _ => TermForm::Short, // Use short for label: (Ed.) not (editor)
                };
                options.locale.role_term(role, plural, form)
                    .map(|t| format!(" ({})", t))
            }
            _ => None,
        };

        Some(ProcValues {
            value: formatted,
            prefix: None,
            suffix,
        })
    }
}

/// Format a list of names according to style options.
fn format_names(names: &[Name], form: &ContributorForm, options: &RenderOptions<'_>) -> String {
    if names.is_empty() {
        return String::new();
    }

    let config = options.config.contributors.as_ref();
    let locale = options.locale;

    // Only apply et al. truncation in citations, not bibliographies
    let shorten: Option<&ShortenListOptions> = if options.context == RenderContext::Citation {
        config.and_then(|c| c.shorten.as_ref())
    } else {
        None
    };
    let (display_names, use_et_al) = if let Some(opts) = shorten {
        if names.len() >= opts.min as usize {
            let display: Vec<&Name> = names.iter().take(opts.use_first as usize).collect();
            (display, true)
        } else {
            (names.iter().collect(), false)
        }
    } else {
        (names.iter().collect(), false)
    };

    // Format each name
    let display_as_sort = config.and_then(|c| c.display_as_sort.clone());
    let formatted: Vec<String> = display_names
        .iter()
        .enumerate()
        .map(|(i, name)| format_single_name(name, form, i, &display_as_sort))
        .collect();

    // Join with appropriate delimiter and "and" from locale
    let use_symbol = matches!(config.and_then(|c| c.and.clone()), Some(AndOptions::Symbol));
    let and_str = format!(" {} ", locale.and_term(use_symbol));

    let result = if formatted.len() == 1 {
        formatted[0].clone()
    } else if formatted.len() == 2 {
        format!("{}{}{}", formatted[0], and_str, formatted[1])
    } else {
        let last = formatted.last().unwrap();
        let rest = &formatted[..formatted.len() - 1];
        format!("{},{} {}", rest.join(", "), and_str.trim_end(), last)
    };

    if use_et_al {
        format!("{} {}", result, locale.et_al())
    } else {
        result
    }
}

/// Format a single name.
fn format_single_name(
    name: &Name,
    form: &ContributorForm,
    index: usize,
    display_as_sort: &Option<DisplayAsSort>,
) -> String {
    // Handle literal names
    if let Some(literal) = &name.literal {
        return literal.clone();
    }

    let family = name.family.as_deref().unwrap_or("");
    let given = name.given.as_deref().unwrap_or("");

    // Determine if we should invert (Family, Given)
    let inverted = match display_as_sort {
        Some(DisplayAsSort::All) => true,
        Some(DisplayAsSort::First) => index == 0,
        _ => false,
    };

    match form {
        ContributorForm::Short => family.to_string(),
        ContributorForm::Long | ContributorForm::Verb | ContributorForm::VerbShort => {
            if inverted {
                // Initialize given name(s)
                let initials: String = given
                    .split_whitespace()
                    .map(|w| {
                        w.chars()
                            .next()
                            .map(|c| format!("{}.", c))
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if initials.is_empty() {
                    family.to_string()
                } else {
                    format!("{}, {}", family, initials)
                }
            } else {
                // Given Family format
                if given.is_empty() {
                    family.to_string()
                } else {
                    let initial = given.chars().next().map(|c| format!("{}.", c)).unwrap_or_default();
                    format!("{} {}", initial, family)
                }
            }
        }
    }
}

impl ComponentValues for TemplateDate {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let date: &DateVariable = match self.date {
            TemplateDateVar::Issued => reference.issued.as_ref()?,
            TemplateDateVar::Accessed => reference.accessed.as_ref()?,
            _ => return None,
        };

        let locale = options.locale;

        let formatted = match self.form {
            DateForm::Year => date.year_value().map(|y| y.to_string()),
            DateForm::YearMonth => {
                let year = date.year_value()?;
                let month = date.month_value();
                match month {
                    Some(m) => Some(format!("{} {}", locale.month_name(m as u8, false), year)),
                    None => Some(year.to_string()),
                }
            }
            DateForm::MonthDay => {
                // Only output month-day if present; return None if only year
                let month = date.month_value()?;
                let day = date.day_value();
                match day {
                    Some(d) => Some(format!("{} {}", locale.month_name(month as u8, false), d)),
                    None => Some(locale.month_name(month as u8, false).to_string()),
                }
            }
            DateForm::Full => {
                let year = date.year_value()?;
                let month = date.month_value();
                let day = date.day_value();
                match (month, day) {
                    (Some(m), Some(d)) => Some(format!("{} {}, {}", locale.month_name(m as u8, false), d, year)),
                    (Some(m), None) => Some(format!("{} {}", locale.month_name(m as u8, false), year)),
                    _ => Some(year.to_string()),
                }
            }
        };

        // Handle disambiguation suffix (a, b, c...)
        let suffix = if hints.disamb_condition && formatted.as_ref().map(|s| s.len() == 4).unwrap_or(false) {
            // Check if year suffix is enabled
            let use_suffix = options.config.processing
                .as_ref()
                .map(|p| p.config().disambiguate.as_ref().map(|d| d.year_suffix).unwrap_or(false))
                .unwrap_or(false);
            
            if use_suffix {
                int_to_letter((hints.group_index % 26) as u32)
            } else {
                None
            }
        } else {
            None
        };

        formatted.map(|value| ProcValues {
            value,
            prefix: None,
            suffix,
        })
    }
}

fn int_to_letter(n: u32) -> Option<String> {
    if n == 0 { return None; }
    char::from_u32(n + 96).map(|c| c.to_string())
}

impl ComponentValues for TemplateTitle {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        _options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let value = match self.title {
            TitleType::Primary => reference.title.clone(),
            TitleType::ParentSerial => reference.container_title.clone(),
            TitleType::ParentMonograph => reference.collection_title.clone(),
            _ => None, // Handle future non-exhaustive variants
        };

        value.filter(|s| !s.is_empty()).map(|value| ProcValues {
            value,
            prefix: None,
            suffix: None,
        })
    }
}

impl ComponentValues for TemplateNumber {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        _options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let value = match self.number {
            NumberVariable::Volume => reference.volume.as_ref().map(|v| v.to_string()),
            NumberVariable::Issue => reference.issue.as_ref().map(|v| v.to_string()),
            NumberVariable::Pages => reference.page.clone(),
            NumberVariable::Edition => reference.edition.as_ref().map(|v| v.to_string()),
            _ => None,
        };

        value.filter(|s| !s.is_empty()).map(|value| ProcValues {
            value,
            prefix: None,
            suffix: None,
        })
    }
}

impl ComponentValues for TemplateVariable {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        _options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let value = match self.variable {
            SimpleVariable::Doi => reference.doi.clone(),
            SimpleVariable::Url => reference.url.clone(),
            SimpleVariable::Isbn => reference.isbn.clone(),
            SimpleVariable::Issn => reference.issn.clone(),
            SimpleVariable::Publisher => reference.publisher.clone(),
            SimpleVariable::PublisherPlace => reference.publisher_place.clone(),
            SimpleVariable::Genre => reference.genre.clone(),
            SimpleVariable::Abstract => reference.abstract_text.clone(),
            _ => None,
        };

        value.filter(|s| !s.is_empty()).map(|value| ProcValues {
            value,
            prefix: None,
            suffix: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::locale::Locale;

    fn make_config() -> Config {
        Config {
            processing: Some(csln_core::options::Processing::AuthorDate),
            contributors: Some(csln_core::options::ContributorConfig {
                shorten: Some(ShortenListOptions {
                    min: 3,
                    use_first: 1,
                    ..Default::default()
                }),
                and: Some(AndOptions::Symbol),
                display_as_sort: Some(DisplayAsSort::First),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_locale() -> Locale {
        Locale::en_us()
    }

    fn make_reference() -> Reference {
        Reference {
            id: "kuhn1962".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            issued: Some(DateVariable::year(1962)),
            publisher: Some("University of Chicago Press".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_contributor_values() {
        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions { config: &config, locale: &locale, context: RenderContext::Citation };
        let reference = make_reference();
        let hints = ProcHints::default();

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            delimiter: None,
            rendering: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "Kuhn");
    }

    #[test]
    fn test_date_values() {
        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions { config: &config, locale: &locale, context: RenderContext::Citation };
        let reference = make_reference();
        let hints = ProcHints::default();

        let component = TemplateDate {
            date: TemplateDateVar::Issued,
            form: DateForm::Year,
            rendering: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "1962");
    }

    #[test]
    fn test_et_al() {
        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions { config: &config, locale: &locale, context: RenderContext::Citation };
        let hints = ProcHints::default();

        let reference = Reference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ]),
            ..Default::default()
        };

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            delimiter: None,
            rendering: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "LeCun et al.");
    }
}
