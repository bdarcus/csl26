/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Value extraction for template components.
//!
//! This module provides the logic to extract formatted values from references
//! based on template component specifications.

use crate::reference::{EdtfString, Reference};
use csln_core::locale::{Locale, TermForm};
use csln_core::options::{
    AndOptions, Config, DemoteNonDroppingParticle, DisplayAsSort, EditorLabelFormat,
    ShortenListOptions, SubstituteKey,
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
    /// Optional URL for hyperlinking.
    pub url: Option<String>,
    /// Variable key that was substituted (e.g., "title:Primary" when title replaces author).
    /// Used to prevent duplicate rendering per CSL variable-once rule.
    pub substituted_key: Option<String>,
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
    /// Citation number for numeric citation styles (1-based).
    pub citation_number: Option<usize>,
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
            ContributorRole::Author => reference.author(),
            ContributorRole::Editor => reference.editor(),
            ContributorRole::Translator => reference.translator(),
            _ => None,
        };

        // Handle substitution if author is empty
        if names.is_none() && matches!(self.contributor, ContributorRole::Author) {
            // Use explicit substitute config, or fall back to default (editor → title → translator)
            let default_substitute = csln_core::options::SubstituteConfig::default();
            let substitute_config = options
                .config
                .substitute
                .as_ref()
                .unwrap_or(&default_substitute);
            let substitute = substitute_config.resolve();

            for key in &substitute.template {
                match key {
                    SubstituteKey::Editor => {
                        if let Some(editors) = reference.editor() {
                            let names_vec = editors.to_names_vec();
                            if !names_vec.is_empty() {
                                // Substituted editors use the contributor's name_order and and
                                let effective_name_order = self.name_order.as_ref().or_else(|| {
                                    options
                                        .config
                                        .contributors
                                        .as_ref()?
                                        .role
                                        .as_ref()?
                                        .roles
                                        .as_ref()?
                                        .get(self.contributor.as_str())?
                                        .name_order
                                        .as_ref()
                                });

                                let formatted = format_names(
                                    &names_vec,
                                    &self.form,
                                    options,
                                    effective_name_order,
                                    self.and.as_ref(),
                                    hints,
                                );
                                // Add role suffix if configured, but ONLY in bibliography context.
                                // In citations, substituted editors should look identical to authors.
                                let suffix = if options.context == RenderContext::Bibliography {
                                    substitute.contributor_role_form.as_ref().and_then(|form| {
                                        let plural = names_vec.len() > 1;
                                        let term_form = match form.as_str() {
                                            "short" => TermForm::Short,
                                            "verb" => TermForm::Verb,
                                            "verb-short" => TermForm::VerbShort,
                                            _ => TermForm::Short, // Default to short
                                        };
                                        // Look up editor term from locale
                                        options
                                            .locale
                                            .role_term(&ContributorRole::Editor, plural, term_form)
                                            .map(|term| format!(" ({})", term))
                                    })
                                } else {
                                    None
                                };
                                return Some(ProcValues {
                                    value: formatted,
                                    prefix: None,
                                    suffix,
                                    url: None,
                                    // Mark editor as rendered to suppress explicit editor component
                                    // Use the same key format as get_variable_key() for consistency
                                    substituted_key: Some("contributor:Editor".to_string()),
                                });
                            }
                        }
                    }
                    SubstituteKey::Title => {
                        if let Some(title) = reference.title() {
                            let title_str = title.to_string();
                            // When title substitutes for author:
                            // - In CITATIONS: quote the title per CSL conventions
                            // - In BIBLIOGRAPHY: use title as-is (it will be styled normally)
                            let value = if options.context == RenderContext::Citation {
                                format!("\u{201C}{}\u{201D}", title_str) // Curly quotes
                            } else {
                                title_str
                            };
                            return Some(ProcValues {
                                value,
                                prefix: None,
                                suffix: None,
                                url: None,
                                substituted_key: Some("title:Primary".to_string()),
                            });
                        }
                    }
                    SubstituteKey::Translator => {
                        if let Some(translators) = reference.translator() {
                            let names_vec = translators.to_names_vec();
                            if !names_vec.is_empty() {
                                let formatted = format_names(
                                    &names_vec,
                                    &self.form,
                                    options,
                                    self.name_order.as_ref(),
                                    self.and.as_ref(),
                                    hints,
                                );
                                return Some(ProcValues {
                                    value: formatted,
                                    prefix: None,
                                    suffix: Some(" (Trans.)".to_string()),
                                    url: None,
                                    substituted_key: None,
                                });
                            }
                        }
                    }
                }
            }
            return None;
        }

        let names = names?;
        let names_vec = names.to_names_vec();
        if names_vec.is_empty() {
            return None;
        }

        // Use explicit name_order if provided on this contributor template,
        // otherwise check global config for this role.
        let effective_name_order = self.name_order.as_ref().or_else(|| {
            options
                .config
                .contributors
                .as_ref()?
                .role
                .as_ref()?
                .roles
                .as_ref()?
                .get(self.contributor.as_str())?
                .name_order
                .as_ref()
        });

        let formatted = format_names(
            &names_vec,
            &self.form,
            options,
            effective_name_order,
            self.and.as_ref(),
            hints,
        );

        // Add role term based on form:
        let editor_format = options
            .config
            .contributors
            .as_ref()
            .and_then(|c| c.editor_label_format);

        let (role_prefix, role_suffix) = if let Some(format) = editor_format {
            if matches!(
                self.contributor,
                ContributorRole::Editor | ContributorRole::Translator
            ) {
                let plural = names_vec.len() > 1;
                match format {
                    EditorLabelFormat::VerbPrefix => {
                        let term =
                            options
                                .locale
                                .role_term(&self.contributor, plural, TermForm::Verb);
                        (term.map(|t| format!("{} ", t)), None)
                    }
                    EditorLabelFormat::ShortSuffix => {
                        let term =
                            options
                                .locale
                                .role_term(&self.contributor, plural, TermForm::Short);
                        (None, term.map(|t| format!(" ({})", t)))
                    }
                    EditorLabelFormat::LongSuffix => {
                        let term =
                            options
                                .locale
                                .role_term(&self.contributor, plural, TermForm::Long);
                        (None, term.map(|t| format!(", {}", t)))
                    }
                }
            } else {
                (None, None)
            }
        } else {
            match (&self.form, &self.contributor) {
                (ContributorForm::Verb | ContributorForm::VerbShort, role) => {
                    let plural = names_vec.len() > 1;
                    let term_form = match self.form {
                        ContributorForm::VerbShort => TermForm::VerbShort,
                        _ => TermForm::Verb,
                    };
                    let term = options.locale.role_term(role, plural, term_form);
                    (term.map(|t| format!("{} ", t)), None)
                }
                (ContributorForm::Long, ContributorRole::Editor | ContributorRole::Translator) => {
                    let plural = names_vec.len() > 1;
                    let term = options
                        .locale
                        .role_term(&self.contributor, plural, TermForm::Short);
                    (None, term.map(|t| format!(" ({})", t)))
                }
                _ => (None, None),
            }
        };

        return Some(ProcValues {
            value: formatted,
            prefix: role_prefix,
            suffix: role_suffix,
            url: None,
            substituted_key: None,
        });
    }
}

/// Format a list of names according to style options.
fn format_names(
    names: &[crate::reference::FlatName],
    form: &ContributorForm,
    options: &RenderOptions<'_>,
    name_order: Option<&csln_core::template::NameOrder>,
    and_override: Option<&AndOptions>,
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
    let (first_names, use_et_al, last_names) = if let Some(opts) = shorten {
        let use_first = hints.min_names_to_show.unwrap_or(opts.use_first as usize);
        if names.len() >= opts.min as usize
            || (hints.min_names_to_show.is_some() && names.len() > 1)
        {
            if use_first >= names.len() {
                (names.iter().collect::<Vec<_>>(), false, Vec::new())
            } else {
                let first: Vec<&crate::reference::FlatName> =
                    names.iter().take(use_first).collect();
                let last: Vec<&crate::reference::FlatName> = if let Some(ul) = opts.use_last {
                    // Show ul last names. Ensure no overlap with first names.
                    let take_last = ul as usize;
                    let skip = std::cmp::max(use_first, names.len().saturating_sub(take_last));
                    names.iter().skip(skip).collect()
                } else {
                    Vec::new()
                };
                (first, true, last)
            }
        } else {
            (names.iter().collect::<Vec<_>>(), false, Vec::new())
        }
    } else {
        (names.iter().collect::<Vec<_>>(), false, Vec::new())
    };

    // Format each name
    // Use explicit name_order if provided, otherwise use global display_as_sort
    let display_as_sort = config.and_then(|c| c.display_as_sort.clone());
    let initialize_with = config.and_then(|c| c.initialize_with.as_ref());
    let initialize_with_hyphen = config.and_then(|c| c.initialize_with_hyphen);
    let demote_ndp = config.and_then(|c| c.demote_non_dropping_particle.as_ref());
    let delimiter = config.and_then(|c| c.delimiter.as_deref()).unwrap_or(", ");

    let formatted_first: Vec<String> = first_names
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
                initialize_with_hyphen,
                demote_ndp,
                hints.expand_given_names,
            )
        })
        .collect();

    let formatted_last: Vec<String> = last_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let original_idx = names.len() - last_names.len() + i;
            format_single_name(
                name,
                form,
                original_idx,
                &display_as_sort,
                name_order,
                initialize_with,
                initialize_with_hyphen,
                demote_ndp,
                hints.expand_given_names,
            )
        })
        .collect();

    // Determine "and" setting: use override if provided, else global config
    let and_option = and_override.or_else(|| config.and_then(|c| c.and.as_ref()));
    // Determine conjunction between last two names
    // Default (None or no config) means no conjunction, matching CSL behavior
    let and_str = match and_option {
        Some(AndOptions::Text) => Some(locale.and_term(false)),
        Some(AndOptions::Symbol) => Some(locale.and_term(true)),
        Some(AndOptions::None) | None => None, // No conjunction
        _ => None,                             // Future variants: default to none
    };

    // Check if delimiter should precede last name (Oxford comma)
    use csln_core::options::DelimiterPrecedesLast;
    let delimiter_precedes_last = config.and_then(|c| c.delimiter_precedes_last.as_ref());

    let result = if formatted_first.len() == 1 {
        formatted_first[0].clone()
    } else if and_str.is_none() {
        // No conjunction - just join all with delimiter
        formatted_first.join(delimiter)
    } else if formatted_first.len() == 2 {
        let and_str = and_str.unwrap();
        // For two names: citations don't use delimiter before conjunction,
        // but bibliographies do (contextual Oxford comma).
        let use_delimiter = if options.context == RenderContext::Bibliography {
            // In bibliography, check delimiter-precedes-last setting
            match delimiter_precedes_last {
                Some(DelimiterPrecedesLast::Always) => true,
                Some(DelimiterPrecedesLast::Never) => false,
                Some(DelimiterPrecedesLast::Contextual) | None => true, // Default: use comma in bibliography
                Some(DelimiterPrecedesLast::AfterInvertedName) => display_as_sort
                    .as_ref()
                    .is_some_and(|das| matches!(das, DisplayAsSort::All | DisplayAsSort::First)),
            }
        } else {
            // In citations, never use delimiter before conjunction for 2 names
            false
        };

        if use_delimiter {
            format!(
                "{}{}{} {}",
                formatted_first[0], delimiter, and_str, formatted_first[1]
            )
        } else {
            format!("{} {} {}", formatted_first[0], and_str, formatted_first[1])
        }
    } else {
        let and_str = and_str.unwrap();
        let last = formatted_first.last().unwrap();
        let rest = &formatted_first[..formatted_first.len() - 1];
        // Check if delimiter should precede "and" (Oxford comma)
        let use_delimiter = match delimiter_precedes_last {
            Some(DelimiterPrecedesLast::Always) => true,
            Some(DelimiterPrecedesLast::Never) => false,
            Some(DelimiterPrecedesLast::Contextual) | None => true, // Default: comma for 3+ names
            Some(DelimiterPrecedesLast::AfterInvertedName) => {
                display_as_sort.as_ref().is_some_and(|das| {
                    matches!(das, DisplayAsSort::All)
                        || (matches!(das, DisplayAsSort::First) && first_names.len() == 1)
                })
            }
        };
        if use_delimiter {
            format!("{}{}{} {}", rest.join(delimiter), delimiter, and_str, last)
        } else {
            format!("{} {} {}", rest.join(delimiter), and_str, last)
        }
    };

    if use_et_al {
        if !formatted_last.is_empty() {
            // et-al-use-last: result + ellipsis + last names
            // CSL typically uses an ellipsis (...) for this.
            format!("{} … {}", result, formatted_last.join(delimiter))
        } else {
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
                            || (matches!(das, DisplayAsSort::First) && first_names.len() == 1)
                    })
                }
                Some(DelimiterPrecedesLast::Contextual) | None => {
                    // Default: use delimiter only if more than one name displayed
                    first_names.len() > 1
                }
            };

            if use_delimiter {
                format!("{}, {}", result, locale.et_al())
            } else {
                format!("{} {}", result, locale.et_al())
            }
        }
    } else {
        result
    }
}

/// Format a single name.
#[allow(clippy::too_many_arguments)]
fn format_single_name(
    name: &crate::reference::FlatName,
    form: &ContributorForm,
    index: usize,
    display_as_sort: &Option<DisplayAsSort>,
    name_order: Option<&csln_core::template::NameOrder>,
    initialize_with: Option<&String>,
    initialize_with_hyphen: Option<bool>,
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

            let given_part = if let Some(init) = initialize_with {
                let separators = if initialize_with_hyphen == Some(false) {
                    vec![' ', '\u{00A0}'] // Non-breaking space too
                } else {
                    vec![' ', '-', '\u{00A0}']
                };

                let mut result = String::new();
                let mut current_part = String::new();

                for c in given.chars() {
                    if separators.contains(&c) {
                        if !current_part.is_empty() {
                            if let Some(first) = current_part.chars().next() {
                                result.push(first);
                                result.push_str(init);
                            }
                            current_part.clear();
                        }
                        // Push separator if: it's not whitespace (e.g., hyphen for J.-P.),
                        // or if init already has whitespace (so we don't double-space)
                        if !c.is_whitespace() || init.chars().any(|ic| ic.is_whitespace()) {
                            result.push(c);
                        }
                    } else {
                        current_part.push(c);
                    }
                }

                if !current_part.is_empty() {
                    if let Some(first) = current_part.chars().next() {
                        result.push(first);
                        result.push_str(init);
                    }
                }
                result.trim().to_string()
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

/// Format contributors in short form for citation grouping.
///
/// Used when collapsing same-author citations to render "Author" part separately.
/// Format a list of names for citations (short form).
///
/// Used when collapsing same-author citations to render "Author" part separately.
pub fn format_contributors_short(
    names: &[crate::reference::FlatName],
    options: &RenderOptions<'_>,
) -> String {
    format_names(
        names,
        &ContributorForm::Short,
        options,
        None,
        None,
        &ProcHints::default(),
    )
}

impl ComponentValues for TemplateTitle {
    fn values(
        &self,
        reference: &Reference,
        _hints: &ProcHints,
        _options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        let binding = reference.ref_type();
        let _ref_type = binding.as_str();

        let value = match self.title {
            TitleType::Primary => reference.title().map(|t| t.to_string()),
            TitleType::ParentSerial => {
                if matches!(
                    binding.as_str(),
                    "article-journal" | "article-magazine" | "article-newspaper"
                ) {
                    reference.container_title().map(|t| t.to_string())
                } else {
                    None
                }
            }
            TitleType::ParentMonograph => {
                if binding.as_str() == "chapter" {
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

impl ComponentValues for TemplateNumber {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        use csln_core::template::LabelForm;

        let value = match self.number {
            NumberVariable::Volume => reference.volume().map(|v| v.to_string()),
            NumberVariable::Issue => reference.issue().map(|v| v.to_string()),
            NumberVariable::Pages => reference.pages().map(|p| {
                format_page_range(&p.to_string(), options.config.page_range_format.as_ref())
            }),
            NumberVariable::Edition => reference.edition(),
            NumberVariable::CitationNumber => hints.citation_number.map(|n| n.to_string()),
            _ => None,
        };

        value.filter(|s| !s.is_empty()).map(|value| {
            // Handle label if label_form is specified
            let prefix = if let Some(label_form) = &self.label_form {
                if let Some(locator_type) = number_var_to_locator_type(&self.number) {
                    // Check pluralization
                    let plural = check_plural(&value, &locator_type);

                    let term_form = match label_form {
                        LabelForm::Long => TermForm::Long,
                        LabelForm::Short => TermForm::Short,
                        LabelForm::Symbol => TermForm::Symbol,
                    };

                    options
                        .locale
                        .locator_term(&locator_type, plural, term_form)
                        .map(|t| format!("{} ", t))
                } else {
                    None
                }
            } else {
                None
            };

            ProcValues {
                value,
                prefix,
                suffix: None,
                url: None,
                substituted_key: None,
            }
        })
    }
}

fn number_var_to_locator_type(var: &NumberVariable) -> Option<csln_core::citation::LocatorType> {
    use csln_core::citation::LocatorType;
    match var {
        NumberVariable::Volume => Some(LocatorType::Volume),
        NumberVariable::Pages => Some(LocatorType::Page),
        NumberVariable::ChapterNumber => Some(LocatorType::Chapter),
        NumberVariable::NumberOfPages => Some(LocatorType::Page),
        NumberVariable::NumberOfVolumes => Some(LocatorType::Volume),
        NumberVariable::Issue => Some(LocatorType::Issue),
        _ => None,
    }
}

fn check_plural(value: &str, _locator_type: &csln_core::citation::LocatorType) -> bool {
    // Simple heuristic: if contains ranges or separators, it's plural.
    // "1-10", "1, 3", "1 & 3"
    value.contains('–') || value.contains('-') || value.contains(',') || value.contains('&')
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
            SimpleVariable::Doi => reference.doi(),
            SimpleVariable::Url => reference.url().map(|u| u.to_string()),
            SimpleVariable::Isbn => reference.isbn(),
            SimpleVariable::Issn => reference.issn(),
            SimpleVariable::Publisher => reference.publisher_str(),
            SimpleVariable::PublisherPlace => reference.publisher_place(),
            SimpleVariable::Genre => reference.genre(),
            SimpleVariable::Abstract => reference.abstract_text(),
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

impl ComponentValues for TemplateList {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        // Collect values from all items, applying their rendering
        let values: Vec<String> = self
            .items
            .iter()
            .filter_map(|item| {
                let v = item.values(reference, hints, options)?;
                if v.value.is_empty() {
                    return None;
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

                Some(crate::render::render_component(&proc_item))
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
            prefix: self.rendering.prefix.clone(),
            suffix: self.rendering.suffix.clone(),
            url: None,
            substituted_key: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference, StringOrNumber};
    use csln_core::locale::Locale;
    use csln_core::reference::FlatName;

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
        Reference::from(LegacyReference {
            id: "kuhn1962".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Kuhn", "Thomas S.")]),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            issued: Some(DateVariable::year(1962)),
            publisher: Some("University of Chicago Press".to_string()),
            ..Default::default()
        })
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
            and: None,
            rendering: Default::default(),
            overrides: None,
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
            overrides: None,
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

        let reference = Reference::from(LegacyReference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ]),
            ..Default::default()
        });

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            name_order: None,
            delimiter: None,
            and: None,
            rendering: Default::default(),
            overrides: None,
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

        let reference = Reference::from(LegacyReference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
            ..Default::default()
        });

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            name_order: None,
            delimiter: None,
            and: None,
            rendering: Default::default(),
            overrides: None,
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

        let reference = Reference::from(LegacyReference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![Name::new("Smith", "John"), Name::new("Jones", "Jane")]),
            ..Default::default()
        });

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            name_order: None,
            delimiter: None,
            and: None,
            rendering: Default::default(),
            overrides: None,
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
        let name = FlatName {
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
            None, // initialize_with_hyphen
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
            None, // initialize_with_hyphen
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
            None, // initialize_with_hyphen
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
            None, // initialize_with_hyphen
            Some(&DemoteNonDroppingParticle::DisplayAndSort),
            false,
        );
        assert_eq!(res_straight, "Ludwig van Beethoven");
    }

    #[test]
    fn test_template_list_suppression() {
        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let reference = Reference::from(LegacyReference {
            id: "multi".to_string(),
            ..Default::default()
        });
        let hints = ProcHints::default();

        let component = TemplateList {
            items: vec![
                TemplateComponent::Variable(TemplateVariable {
                    variable: SimpleVariable::Doi,
                    ..Default::default()
                }),
                TemplateComponent::Variable(TemplateVariable {
                    variable: SimpleVariable::Url,
                    ..Default::default()
                }),
            ],
            delimiter: Some(DelimiterPunctuation::Comma),
            ..Default::default()
        };

        let values = component.values(&reference, &hints, &options);
        assert!(values.is_none());
    }

    #[test]
    fn test_et_al_use_last() {
        let mut config = make_config();
        if let Some(ref mut contributors) = config.contributors {
            contributors.shorten = Some(ShortenListOptions {
                min: 3,
                use_first: 1,
                use_last: Some(1),
                ..Default::default()
            });
        }

        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let hints = ProcHints::default();

        let reference = Reference::from(LegacyReference {
            id: "multi".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ]),
            ..Default::default()
        });

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            ..Default::default()
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        // first name (LeCun) + ellipsis + last name (Hinton)
        assert_eq!(values.value, "LeCun … Hinton");
    }

    #[test]
    fn test_et_al_use_last_overlap() {
        // Edge case: use_first + use_last >= names.len() should show all names
        let mut config = make_config();
        if let Some(ref mut contributors) = config.contributors {
            contributors.shorten = Some(ShortenListOptions {
                min: 3,
                use_first: 2,
                use_last: Some(2),
                ..Default::default()
            });
        }

        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let hints = ProcHints::default();

        let reference = Reference::from(LegacyReference {
            id: "overlap".to_string(),
            ref_type: "article-journal".to_string(),
            author: Some(vec![
                Name::new("Alpha", "A."),
                Name::new("Beta", "B."),
                Name::new("Gamma", "C."),
            ]),
            ..Default::default()
        });

        let component = TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            ..Default::default()
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        // use_first(2) + use_last(2) = 4 >= 3 names, so show first 2 + ellipsis + last 1
        // Alpha & Beta … Gamma (skip=max(2, 3-2)=2, so last 1 name)
        assert_eq!(values.value, "Alpha & Beta … Gamma");
    }

    #[test]
    fn test_title_hyperlink() {
        use csln_core::options::LinksConfig;

        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let hints = ProcHints::default();

        let reference = Reference::from(LegacyReference {
            id: "kuhn1962".to_string(),
            title: Some("The Structure of Scientific Revolutions".to_string()),
            doi: Some("10.1001/example".to_string()),
            ..Default::default()
        });

        let component = TemplateTitle {
            title: TitleType::Primary,
            links: Some(LinksConfig {
                doi: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(
            values.url,
            Some("https://doi.org/10.1001/example".to_string())
        );
    }

    #[test]
    fn test_title_hyperlink_url_fallback() {
        use csln_core::options::LinksConfig;

        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Citation,
        };
        let hints = ProcHints::default();

        // Reference with URL but no DOI
        let reference = Reference::from(LegacyReference {
            id: "web2024".to_string(),
            title: Some("A Web Resource".to_string()),
            url: Some("https://example.com/resource".to_string()),
            ..Default::default()
        });

        let component = TemplateTitle {
            title: TitleType::Primary,
            links: Some(LinksConfig {
                doi: Some(true),
                url: Some(true),
            }),
            ..Default::default()
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        // Falls back to URL when DOI is absent
        assert_eq!(values.url, Some("https://example.com/resource".to_string()));
    }

    #[test]
    fn test_variable_hyperlink() {
        use csln_core::options::LinksConfig;

        let config = make_config();
        let locale = make_locale();
        let options = RenderOptions {
            config: &config,
            locale: &locale,
            context: RenderContext::Bibliography,
        };
        let hints = ProcHints::default();

        let reference = Reference::from(LegacyReference {
            id: "pub2024".to_string(),
            publisher: Some("MIT Press".to_string()),
            doi: Some("10.1234/pub".to_string()),
            ..Default::default()
        });

        let component = TemplateVariable {
            variable: SimpleVariable::Publisher,
            links: Some(LinksConfig {
                doi: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let values = component.values(&reference, &hints, &options).unwrap();
        assert_eq!(values.value, "MIT Press");
        assert_eq!(values.url, Some("https://doi.org/10.1234/pub".to_string()));
    }

    #[test]
    fn test_editor_label_format() {
        let mut config = make_config();
        let locale = make_locale();
        let hints = ProcHints::default();

        let reference = Reference::from(LegacyReference {
            id: "editor-test".to_string(),
            ref_type: "book".to_string(),
            editor: Some(vec![Name::new("Doe", "John")]),
            ..Default::default()
        });

        let component = TemplateContributor {
            contributor: ContributorRole::Editor,
            form: ContributorForm::Long,
            ..Default::default()
        };

        // Test VerbPrefix
        if let Some(ref mut contributors) = config.contributors {
            contributors.editor_label_format = Some(EditorLabelFormat::VerbPrefix);
        }
        {
            let options = RenderOptions {
                config: &config,
                locale: &locale,
                context: RenderContext::Bibliography,
            };
            let values = component.values(&reference, &hints, &options).unwrap();
            // Assuming locale for "editor" verb is "edited by"
            assert_eq!(values.prefix, Some("edited by ".to_string()));
        }

        // Test ShortSuffix
        if let Some(ref mut contributors) = config.contributors {
            contributors.editor_label_format = Some(EditorLabelFormat::ShortSuffix);
        }
        {
            let options = RenderOptions {
                config: &config,
                locale: &locale,
                context: RenderContext::Bibliography,
            };
            let values = component.values(&reference, &hints, &options).unwrap();
            // Assuming locale for "editor" short is "Ed."
            assert_eq!(values.suffix, Some(" (Ed.)".to_string()));
        }

        // Test LongSuffix
        if let Some(ref mut contributors) = config.contributors {
            contributors.editor_label_format = Some(EditorLabelFormat::LongSuffix);
        }
        {
            let options = RenderOptions {
                config: &config,
                locale: &locale,
                context: RenderContext::Bibliography,
            };
            let values = component.values(&reference, &hints, &options).unwrap();
            // Assuming locale for "editor" long is "editor"
            assert_eq!(values.suffix, Some(", editor".to_string()));
        }
    }
}
