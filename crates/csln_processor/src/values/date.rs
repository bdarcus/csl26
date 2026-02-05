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
        let date: EdtfString = match self.date {
            TemplateDateVar::Issued => reference.issued()?,
            TemplateDateVar::Accessed => reference.accessed().unwrap_or(EdtfString(String::new())), // accessed might be None
            _ => return None,
        };
        if date.0.is_empty() {
            return None;
        }

        let locale = options.locale;

        let formatted = match self.form {
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
        };

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
            url: None,
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
