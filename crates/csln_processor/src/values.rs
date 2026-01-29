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
use csln_core::options::{
    AndOptions, Config, DemoteNonDroppingParticle, DisplayAsSort, ShortenListOptions, SubstituteKey,
};
use csln_core::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable as TemplateDateVar,
    DelimiterPunctuation, NumberVariable, SimpleVariable, TemplateComponent, TemplateContributor,
    TemplateDate, TemplateList, TemplateNumber, TemplateTitle, TemplateVariable, TitleType,
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

#[derive(Debug, Clone, Default)]
pub struct ProcHints {
    /// Whether disambiguation is active (triggers year-suffix).
    pub disamb_condition: bool,
    /// Index in the disambiguation group (1-based).
    pub group_index: usize,
    /// Total size of the disambiguation group.
    pub group_length: usize,
    /// The grouping key used.
    pub group_key: String,
    /// Whether to expand given names for disambiguation.
    pub expand_given_names: bool,
    /// Minimum number of names to show to resolve ambiguity (overrides et-al-use-first).
    pub min_names_to_show: Option<usize>,
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
            TemplateComponent::List(l) => l.values(reference, hints, options),
            _ => None, // Handle future non-exhaustive variants
        }
    }
}

impl ComponentValues for TemplateContributor {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let names = match self.contributor {
            ContributorRole::Author => reference.author.as_ref(),
            ContributorRole::Editor => reference.editor.as_ref(),
            ContributorRole::Translator => reference.translator.as_ref(),
            _ => None,
        };

        // Handle substitution if author is empty
        if names.map(|n| n.is_empty()).unwrap_or(true)
            && matches!(self.contributor, ContributorRole::Author)
        {
            if let Some(substitute) = &options.config.substitute {
                for key in &substitute.template {
                    match key {
                        SubstituteKey::Editor => {
                            if let Some(editors) = &reference.editor {
                                if !editors.is_empty() {
                                    // Substituted editors use the contributor's name_order
                                    let formatted = format_names(
                                        editors,
                                        &self.form,
                                        options,
                                        self.name_order.as_ref(),
                                        hints,
                                    );
                                    // Add role suffix if configured
                                    let suffix = substitute
                                        .contributor_role_form
                                        .as_ref()
                                        .map(|_| " (Ed.)".to_string());
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
                                    let formatted = format_names(
                                        translators,
                                        &self.form,
                                        options,
                                        self.name_order.as_ref(),
                                        hints,
                                    );
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

        // Use explicit name_order if provided on this contributor template
        let formatted = format_names(names, &self.form, options, self.name_order.as_ref(), hints);

        // Add role label suffix for verb forms (e.g., "Name (Ed.)")
        let suffix = match (&self.form, &self.contributor) {
            (ContributorForm::Verb | ContributorForm::VerbShort, role) => {
                let plural = names.len() > 1;
                let form = match self.form {
                    ContributorForm::VerbShort => TermForm::Short,
                    _ => TermForm::Short, // Use short for label: (Ed.) not (editor)
                };
                options
                    .locale
                    .role_term(role, plural, form)
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
///
/// The `name_order` parameter overrides the global `display-as-sort` setting
/// for this specific rendering. Used when editors need "Given Family" format
/// even when the global setting is "Family, Given".
fn format_names(
    names: &[Name],
    form: &ContributorForm,
    options: &RenderOptions<'_>,
    name_order: Option<&csln_core::template::NameOrder>,
    hints: &ProcHints,
) -> String {
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
        let use_first = hints.min_names_to_show.unwrap_or(opts.use_first as usize);
        if names.len() >= opts.min as usize
            || (hints.min_names_to_show.is_some() && names.len() > 1)
        {
            if use_first >= names.len() {
                (names.iter().collect(), false)
            } else {
                let display: Vec<&Name> = names.iter().take(use_first).collect();
                (display, true)
            }
        } else {
            (names.iter().collect(), false)
        }
    } else {
        (names.iter().collect(), false)
    };

    // Format each name
    // Use explicit name_order if provided, otherwise use global display_as_sort
    let display_as_sort = config.and_then(|c| c.display_as_sort.clone());
    let initialize_with = config.and_then(|c| c.initialize_with.as_ref());
    let demote_ndp = config.and_then(|c| c.demote_non_dropping_particle.as_ref());

    let formatted: Vec<String> = display_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            format_single_name(
                name,
                form,
                i,
                &display_as_sort,
                name_order,
                initialize_with,
                demote_ndp,
                hints.expand_given_names,
            )
        })
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
        // Determine delimiter before "et al." based on delimiter_precedes_et_al option
        use csln_core::options::DelimiterPrecedesLast;
        let delimiter_precedes = config.and_then(|c| c.delimiter_precedes_et_al.as_ref());
        let use_delimiter = match delimiter_precedes {
            Some(DelimiterPrecedesLast::Always) => true,
            Some(DelimiterPrecedesLast::Never) => false,
            Some(DelimiterPrecedesLast::AfterInvertedName) => {
                // Use delimiter if last displayed name was inverted (family-first)
                display_as_sort.as_ref().is_some_and(|das| {
                    matches!(das, DisplayAsSort::All)
                        || (matches!(das, DisplayAsSort::First) && display_names.len() == 1)
                })
            }
            Some(DelimiterPrecedesLast::Contextual) | None => {
                // Default: use delimiter only if more than one name displayed
                display_names.len() > 1
            }
        };

        if use_delimiter {
            format!("{}, {}", result, locale.et_al())
        } else {
            format!("{} {}", result, locale.et_al())
        }
    } else {
        result
    }
}

/// Format a single name.
#[allow(clippy::too_many_arguments)]
fn format_single_name(
    name: &Name,
    form: &ContributorForm,
    index: usize,
    display_as_sort: &Option<DisplayAsSort>,
    name_order: Option<&csln_core::template::NameOrder>,
    initialize_with: Option<&String>,
    demote_ndp: Option<&DemoteNonDroppingParticle>,
    expand_given_names: bool,
) -> String {
    use csln_core::template::NameOrder;

    // Handle literal names (e.g., corporate authors)
    if let Some(literal) = &name.literal {
        return literal.clone();
    }

    let family = name.family.as_deref().unwrap_or("");
    let given = name.given.as_deref().unwrap_or("");
    let dp = name.dropping_particle.as_deref().unwrap_or("");
    let ndp = name.non_dropping_particle.as_deref().unwrap_or("");
    let suffix = name.suffix.as_deref().unwrap_or("");

    // Determine if we should invert (Family, Given)
    let inverted = match name_order {
        Some(NameOrder::GivenFirst) => false,
        Some(NameOrder::FamilyFirst) => true,
        None => match display_as_sort {
            Some(DisplayAsSort::All) => true,
            Some(DisplayAsSort::First) => index == 0,
            _ => false,
        },
    };

    // Determine effective form
    let effective_form = if expand_given_names && matches!(form, ContributorForm::Short) {
        &ContributorForm::Long
    } else {
        form
    };

    match effective_form {
        ContributorForm::Short => {
            // Short form usually just family name, but includes non-dropping particle
            // e.g. "van Beethoven" (unless demoted? CSL spec says demote only affects sorting/display of full names mostly?)
            // Spec: "demote-non-dropping-particle ... This attribute does not affect ... the short form"
            // So for short form, we keep ndp with family.
            let full_family = if !ndp.is_empty() {
                format!("{} {}", ndp, family)
            } else {
                family.to_string()
            };
            full_family
        }
        ContributorForm::Long | ContributorForm::Verb | ContributorForm::VerbShort => {
            // Determine parts based on demotion
            let demote = matches!(demote_ndp, Some(DemoteNonDroppingParticle::DisplayAndSort));

            let family_part = if !ndp.is_empty() && !demote {
                format!("{} {}", ndp, family)
            } else {
                family.to_string()
            };

            let given_part = if let Some(_init) = initialize_with {
                given
                    .split_whitespace()
                    .map(|w| {
                        w.chars()
                            .next()
                            .map(|c| format!("{}.", c))
                            .unwrap_or_default()
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                given.to_string()
            };

            // Construct particle part (dropping + demoted non-dropping)
            let mut particle_part = String::new();
            if !dp.is_empty() {
                particle_part.push_str(dp);
            }
            if demote && !ndp.is_empty() {
                if !particle_part.is_empty() {
                    particle_part.push(' ');
                }
                particle_part.push_str(ndp);
            }

            if inverted {
                // "Family, Given" format
                // Family Part + "," + Given Part + Particle Part + Suffix
                let mut suffix_part = String::new();
                if !given_part.is_empty() {
                    suffix_part.push_str(&given_part);
                }
                if !particle_part.is_empty() {
                    if !suffix_part.is_empty() {
                        suffix_part.push(' ');
                    }
                    suffix_part.push_str(&particle_part);
                }
                if !suffix.is_empty() {
                    if !suffix_part.is_empty() {
                        suffix_part.push(' ');
                    }
                    suffix_part.push_str(suffix);
                }

                if !suffix_part.is_empty() {
                    format!("{}, {}", family_part, suffix_part)
                } else {
                    family_part
                }
            } else {
                // "Given Family" format
                // Given Part + Particle Part + Family Part + Suffix
                let mut parts = Vec::new();
                if !given_part.is_empty() {
                    parts.push(given_part);
                }
                if !particle_part.is_empty() {
                    parts.push(particle_part);
                }
                if !family_part.is_empty() {
                    parts.push(family_part);
                }
                if !suffix.is_empty() {
                    parts.push(suffix.to_string());
                }

                parts.join(" ")
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
                    (Some(m), Some(d)) => Some(format!(
                        "{} {}, {}",
                        locale.month_name(m as u8, false),
                        d,
                        year
                    )),
                    (Some(m), None) => {
                        Some(format!("{} {}", locale.month_name(m as u8, false), year))
                    }
                    _ => Some(year.to_string()),
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
        })
    }
}

fn int_to_letter(n: u32) -> Option<String> {
    if n == 0 {
        return None;
    }
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
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let value = match self.number {
            NumberVariable::Volume => reference.volume.as_ref().map(|v| v.to_string()),
            NumberVariable::Issue => reference.issue.as_ref().map(|v| v.to_string()),
            NumberVariable::Pages => reference
                .page
                .clone()
                .map(|p| format_page_range(&p, options.config.page_range_format.as_ref())),
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

/// Format a page range according to the specified format.
///
/// Formats: expanded (default), minimal, minimal-two, chicago, chicago-16
fn format_page_range(pages: &str, format: Option<&csln_core::options::PageRangeFormat>) -> String {
    use csln_core::options::PageRangeFormat;

    // First, replace hyphen with en-dash
    let pages = pages.replace("-", "–");

    // If no range or no format specified, return as-is
    let format = match format {
        Some(f) => f,
        None => return pages, // Default: just convert to en-dash
    };

    // Check if this is a range (contains en-dash)
    let parts: Vec<&str> = pages.split('–').collect();
    if parts.len() != 2 {
        return pages; // Not a simple range
    }

    let start = parts[0].trim();
    let end = parts[1].trim();

    // Parse as numbers
    let start_num: Option<u32> = start.parse().ok();
    let end_num: Option<u32> = end.parse().ok();

    match (start_num, end_num) {
        (Some(s), Some(e)) if e > s => {
            let formatted_end = match format {
                PageRangeFormat::Expanded => end.to_string(),
                PageRangeFormat::Minimal => format_minimal(start, end, 1),
                PageRangeFormat::MinimalTwo => format_minimal(start, end, 2),
                PageRangeFormat::Chicago | PageRangeFormat::Chicago16 => format_chicago(s, e),
                _ => end.to_string(), // Future variants: default to expanded
            };
            format!("{}–{}", start, formatted_end)
        }
        _ => pages, // Can't parse or invalid range
    }
}

/// Minimal format: keep only differing digits, with minimum min_digits
fn format_minimal(start: &str, end: &str, min_digits: usize) -> String {
    let start_chars: Vec<char> = start.chars().collect();
    let end_chars: Vec<char> = end.chars().collect();

    if start_chars.len() != end_chars.len() {
        return end.to_string();
    }

    // Find first differing position
    let mut first_diff = 0;
    for (i, (s, e)) in start_chars.iter().zip(end_chars.iter()).enumerate() {
        if s != e {
            first_diff = i;
            break;
        }
    }

    // Keep at least min_digits from the end
    let keep_from = first_diff.min(end_chars.len().saturating_sub(min_digits));
    end_chars[keep_from..].iter().collect()
}

/// Chicago Manual of Style page range format
fn format_chicago(start: u32, end: u32) -> String {
    // Chicago rules (simplified from CMOS 17th):
    // - Under 100: use all digits (3–10, 71–72, 96–117)
    // - 100+, same hundreds: use changed part only for 2+ digits (107–8, 321–28, 1536–38)
    // - Different hundreds: use all digits (107–108, 321–328 if change of hundreds)

    if start < 100 || end < 100 {
        return end.to_string();
    }

    let start_str = start.to_string();
    let end_str = end.to_string();

    if start_str.len() != end_str.len() {
        return end_str;
    }

    // Check if same hundreds
    let start_prefix = start / 100;
    let end_prefix = end / 100;

    if start_prefix != end_prefix {
        return end_str; // Different hundreds, use full number
    }

    // Same hundreds: use minimal-two style
    format_minimal(&start_str, &end_str, 2)
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

impl ComponentValues for TemplateList {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        use csln_core::template::WrapPunctuation;

        // Collect values from all items, applying their rendering
        let values: Vec<String> = self
            .items
            .iter()
            .filter_map(|item| {
                let v = item.values(reference, hints, options)?;
                if v.value.is_empty() {
                    return None;
                }

                // Apply rendering from the item
                let rendering = item.rendering();
                let (wrap_open, wrap_close) =
                    match rendering.wrap.as_ref().unwrap_or(&WrapPunctuation::None) {
                        WrapPunctuation::Parentheses => ("(", ")"),
                        WrapPunctuation::Brackets => ("[", "]"),
                        WrapPunctuation::None => ("", ""),
                    };

                let prefix = rendering.prefix.as_deref().unwrap_or_default();
                let suffix = rendering.suffix.as_deref().unwrap_or_default();

                // Build the formatted value
                let mut s = String::new();
                s.push_str(wrap_open);
                s.push_str(prefix);
                if let Some(p) = &v.prefix {
                    s.push_str(p);
                }
                s.push_str(&v.value);
                if let Some(suf) = &v.suffix {
                    s.push_str(suf);
                }
                s.push_str(suffix);
                s.push_str(wrap_close);

                Some(s)
            })
            .collect();

        if values.is_empty() {
            return None;
        }

        // Join with delimiter
        let delimiter = match self
            .delimiter
            .as_ref()
            .unwrap_or(&DelimiterPunctuation::Comma)
        {
            DelimiterPunctuation::Comma => ", ",
            DelimiterPunctuation::Semicolon => "; ",
            DelimiterPunctuation::Period => ". ",
            DelimiterPunctuation::Colon => ": ",
            DelimiterPunctuation::Ampersand => " & ",
            DelimiterPunctuation::VerticalLine => " | ",
            DelimiterPunctuation::Slash => "/",
            DelimiterPunctuation::Hyphen => "-",
            DelimiterPunctuation::Space => " ",
            DelimiterPunctuation::None => "",
        };

        Some(ProcValues {
            value: values.join(delimiter),
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
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let reference = make_reference();
        let hints = ProcHints::default();

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            name_order: None,
            delimiter: None,
            rendering: Default::default(),
            _extra: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "Kuhn");
    }

    #[test]
    fn test_date_values() {
        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let reference = make_reference();
        let hints = ProcHints::default();

        let component = TemplateDate {
            date: TemplateDateVar::Issued,
            form: DateForm::Year,
            rendering: Default::default(),
            _extra: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "1962");
    }

    #[test]
    fn test_et_al() {
        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
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
            name_order: None,
            delimiter: None,
            rendering: Default::default(),
            _extra: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "LeCun et al.");
    }

    #[test]
    fn test_format_page_range_expanded() {
        use csln_core::options::PageRangeFormat;
        assert_eq!(
            format_page_range("321-328", Some(&PageRangeFormat::Expanded)),
            "321–328"
        );
        assert_eq!(
            format_page_range("42-45", Some(&PageRangeFormat::Expanded)),
            "42–45"
        );
    }

    #[test]
    fn test_format_page_range_minimal() {
        use csln_core::options::PageRangeFormat;
        // minimal: keep only differing digits
        assert_eq!(
            format_page_range("321-328", Some(&PageRangeFormat::Minimal)),
            "321–8"
        );
        assert_eq!(
            format_page_range("42-45", Some(&PageRangeFormat::Minimal)),
            "42–5"
        );
        assert_eq!(
            format_page_range("12-17", Some(&PageRangeFormat::Minimal)),
            "12–7"
        );
    }

    #[test]
    fn test_format_page_range_minimal_two() {
        use csln_core::options::PageRangeFormat;
        // minimal-two: at least 2 digits
        assert_eq!(
            format_page_range("321-328", Some(&PageRangeFormat::MinimalTwo)),
            "321–28"
        );
        assert_eq!(
            format_page_range("42-45", Some(&PageRangeFormat::MinimalTwo)),
            "42–45"
        );
    }

    #[test]
    fn test_format_page_range_chicago() {
        use csln_core::options::PageRangeFormat;
        // Chicago: special rules for under 100 and same hundreds
        assert_eq!(
            format_page_range("71-72", Some(&PageRangeFormat::Chicago)),
            "71–72"
        );
        assert_eq!(
            format_page_range("321-328", Some(&PageRangeFormat::Chicago)),
            "321–28"
        );
        assert_eq!(
            format_page_range("1536-1538", Some(&PageRangeFormat::Chicago)),
            "1536–38"
        );
    }

    #[test]
    fn test_format_page_range_no_format() {
        // No format specified: just convert hyphen to en-dash
        assert_eq!(format_page_range("321-328", None), "321–328");
    }

    #[test]
    fn test_et_al_delimiter_never() {
        use csln_core::options::DelimiterPrecedesLast;

        let mut config = make_config();
        if let Some(ref mut contributors) = config.contributors {
            contributors.shorten = Some(ShortenListOptions {
                min: 2,
                use_first: 1,
                ..Default::default()
            });
            contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Never);
        }

        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let hints = ProcHints::default();

        let reference = Reference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
            ..Default::default()
        };

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            name_order: None,
            delimiter: None,
            rendering: Default::default(),
            _extra: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        // With "never", no comma before et al.
        assert_eq!(values.value, "Smith et al.");
    }

    #[test]
    fn test_et_al_delimiter_always() {
        use csln_core::options::DelimiterPrecedesLast;

        let mut config = make_config();
        if let Some(ref mut contributors) = config.contributors {
            contributors.shorten = Some(ShortenListOptions {
                min: 2,
                use_first: 1,
                ..Default::default()
            });
            contributors.delimiter_precedes_et_al = Some(DelimiterPrecedesLast::Always);
        }

        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let hints = ProcHints::default();

        let reference = Reference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
            ..Default::default()
        };

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            name_order: None,
            delimiter: None,
            rendering: Default::default(),
            _extra: Default::default(),
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        // With "always", comma before et al.
        assert_eq!(values.value, "Smith, et al.");
    }

    #[test]
    fn test_demote_non_dropping_particle() {
        use csln_core::options::DemoteNonDroppingParticle;

        // Name: Ludwig van Beethoven
        let name = Name {
            family: Some("Beethoven".to_string()),
            given: Some("Ludwig".to_string()),
            non_dropping_particle: Some("van".to_string()),
            ..Default::default()
        };

        // Case 1: Never demote (default CSL behavior for display)
        // Inverted: "van Beethoven, Ludwig"
        let res_never = format_single_name(
            &name,
            &ContributorForm::Long,
            0,
            &Some(DisplayAsSort::All), // Force inverted
            None,
            None,
            Some(&DemoteNonDroppingParticle::Never),
            false,
        );
        assert_eq!(res_never, "van Beethoven, Ludwig");

        // Case 2: Display-and-sort (demote)
        // Inverted: "Beethoven, Ludwig van"
        let res_demote = format_single_name(
            &name,
            &ContributorForm::Long,
            0,
            &Some(DisplayAsSort::All), // Force inverted
            None,
            None,
            Some(&DemoteNonDroppingParticle::DisplayAndSort),
            false,
        );
        assert_eq!(res_demote, "Beethoven, Ludwig van");

        // Case 3: Sort-only (same as Never for display)
        // Inverted: "van Beethoven, Ludwig"
        let res_sort_only = format_single_name(
            &name,
            &ContributorForm::Long,
            0,
            &Some(DisplayAsSort::All), // Force inverted
            None,
            None,
            Some(&DemoteNonDroppingParticle::SortOnly),
            false,
        );
        assert_eq!(res_sort_only, "van Beethoven, Ludwig");

        // Case 4: Not inverted (should be same for all)
        // "Ludwig van Beethoven"
        let res_straight = format_single_name(
            &name,
            &ContributorForm::Long,
            0,
            &Some(DisplayAsSort::None), // Not inverted
            None,
            None,
            Some(&DemoteNonDroppingParticle::DisplayAndSort),
            false,
        );
        assert_eq!(res_straight, "Ludwig van Beethoven");
    }
}
