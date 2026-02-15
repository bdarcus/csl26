use crate::reference::{EdtfString, Reference};
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderOptions};
use csln_core::template::{DateForm, DateVariable as TemplateDateVar, TemplateDate};

impl ComponentValues for TemplateDate {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        // Apply visibility filter
        if matches!(
            options.visibility,
            csln_core::citation::ItemVisibility::AuthorOnly
        ) {
            return None;
        }

        let date_opt: Option<EdtfString> = match self.date {
            TemplateDateVar::Issued => reference.issued(),
            TemplateDateVar::Accessed => reference.accessed(),
            _ => None,
        };

        if date_opt.is_none() || date_opt.as_ref().unwrap().0.is_empty() {
            // Handle fallback if date is missing
            if let Some(fallbacks) = &self.fallback {
                for component in fallbacks {
                    if let Some(values) = component.values(reference, hints, options) {
                        return Some(values);
                    }
                }
            }
            return None;
        }

        let date = date_opt.unwrap();
        let locale = options.locale;
        let date_config = options.config.dates.as_ref();

        let formatted = if date.is_range() {
            // Handle date ranges
            let start = match self.form {
                DateForm::Year => date.year(),
                DateForm::YearMonth => {
                    let month = date.month(&locale.dates.months.long);
                    let year = date.year();
                    if month.is_empty() {
                        year
                    } else {
                        format!("{} {}", month, year)
                    }
                }
                DateForm::MonthDay => {
                    let month = date.month(&locale.dates.months.long);
                    let day = date.day();
                    match day {
                        Some(d) => format!("{} {}", month, d),
                        None => month,
                    }
                }
                DateForm::Full => {
                    let year = date.year();
                    let month = date.month(&locale.dates.months.long);
                    let day = date.day();
                    match (month.is_empty(), day) {
                        (true, _) => year,
                        (false, None) => format!("{} {}", month, year),
                        (false, Some(d)) => format!("{} {}, {}", month, d, year),
                    }
                }
            };

            if date.is_open_range() {
                // Open-ended range (e.g., "1990/..")
                if let Some(end_marker) = date_config
                    .and_then(|c| c.open_range_marker.as_deref())
                    .or(locale.dates.open_ended_term.as_deref())
                {
                    // U+2013 en-dash is the Unicode standard range delimiter (not language-specific)
                    let delimiter = date_config
                        .map(|c| c.range_delimiter.as_str())
                        .unwrap_or("–");
                    Some(format!("{}{}{}", start, delimiter, end_marker))
                } else {
                    // No open-ended term available - return start date only
                    Some(start)
                }
            } else if let Some(end) = date.range_end(&locale.dates.months.long) {
                // Closed range with end date
                // U+2013 en-dash is the Unicode standard range delimiter (not language-specific)
                let delimiter = date_config
                    .map(|c| c.range_delimiter.as_str())
                    .unwrap_or("–");
                Some(format!("{}{}{}", start, delimiter, end))
            } else {
                Some(start)
            }
        } else {
            // Single date (not a range)
            match self.form {
                DateForm::Year => {
                    let year = date.year();
                    if year.is_empty() {
                        None
                    } else {
                        Some(year)
                    }
                }
                DateForm::YearMonth => {
                    let year = date.year();
                    if year.is_empty() {
                        return None;
                    }
                    let month = date.month(&locale.dates.months.long);
                    if month.is_empty() {
                        Some(year)
                    } else {
                        Some(format!("{} {}", month, year))
                    }
                }
                DateForm::MonthDay => {
                    let month = date.month(&locale.dates.months.long);
                    if month.is_empty() {
                        return None;
                    }
                    let day = date.day();
                    match day {
                        Some(d) => Some(format!("{} {}", month, d)),
                        None => Some(month),
                    }
                }
                DateForm::Full => {
                    let year = date.year();
                    if year.is_empty() {
                        return None;
                    }
                    let month = date.month(&locale.dates.months.long);
                    let day = date.day();
                    match (month.is_empty(), day) {
                        (true, _) => Some(year),
                        (false, None) => Some(format!("{} {}", month, year)),
                        (false, Some(d)) => Some(format!("{} {}, {}", month, d, year)),
                    }
                }
            }
        };

        // Apply uncertainty and approximation markers
        let formatted = formatted.map(|mut value| {
            if date.is_approximate() {
                if let Some(marker) = date_config.and_then(|c| c.approximation_marker.as_ref()) {
                    value = format!("{}{}", marker, value);
                }
            }
            if date.is_uncertain() {
                if let Some(marker) = date_config.and_then(|c| c.uncertainty_marker.as_ref()) {
                    value = format!("{}{}", value, marker);
                }
            }
            value
        });

        // Handle disambiguation suffix (a, b, c...)
        let suffix = if hints.disamb_condition
            && formatted.as_ref().map(|s| s.len() == 4).unwrap_or(false)
        {
            // Check if year suffix is enabled
            let use_suffix = options
                .config
                .processing
                .as_ref()
                .map(|p| {
                    p.config()
                        .disambiguate
                        .as_ref()
                        .map(|d| d.year_suffix)
                        .unwrap_or(false)
                })
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
            url: crate::values::resolve_effective_url(
                self.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                csln_core::options::LinkAnchor::Component,
            ),
            substituted_key: None,
        })
    }
}

pub fn int_to_letter(n: u32) -> Option<String> {
    if n == 0 {
        return None;
    }
    char::from_u32(n + 96).map(|c| c.to_string())
}
