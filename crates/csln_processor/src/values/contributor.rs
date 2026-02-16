use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderContext, RenderOptions};
use csln_core::locale::TermForm;
use csln_core::options::{
    AndOptions, DemoteNonDroppingParticle, DisplayAsSort, EditorLabelFormat, ShortenListOptions,
    SubstituteKey,
};
use csln_core::template::{ContributorForm, ContributorRole, TemplateContributor};

impl ComponentValues for TemplateContributor {
    fn values(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues> {
        // Apply visibility filter
        if matches!(
            options.visibility,
            csln_core::citation::ItemVisibility::SuppressAuthor
        ) {
            return None;
        }

        // Resolve effective rendering options (base merged with type-specific override)
        let mut effective_rendering = self.rendering.clone();
        if let Some(overrides) = &self.overrides {
            // Apply "all" wildcard override first
            if let Some(all_override) = overrides.get("all") {
                effective_rendering.merge(all_override);
            }
            // Then apply specific type override
            if let Some(type_override) = overrides.get(&reference.ref_type()) {
                effective_rendering.merge(type_override);
            }
        }

        let contributor = match self.contributor {
            ContributorRole::Author => reference.author(),
            ContributorRole::Editor => reference.editor(),
            ContributorRole::Translator => reference.translator(),
            _ => None,
        };

        // Resolve multilingual names if configured
        let names_vec = if let Some(contrib) = contributor {
            let mode = options
                .config
                .multilingual
                .as_ref()
                .and_then(|m| m.name_mode.as_ref());
            let preferred_script = options
                .config
                .multilingual
                .as_ref()
                .and_then(|m| m.preferred_script.as_ref());
            let locale_str = "en"; // TODO: get from options.locale

            crate::values::resolve_multilingual_name(&contrib, mode, preferred_script, locale_str)
        } else {
            Vec::new()
        };

        // Handle substitution if author is empty
        if names_vec.is_empty() && matches!(self.contributor, ContributorRole::Author) {
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
                            let mode = options
                                .config
                                .multilingual
                                .as_ref()
                                .and_then(|m| m.name_mode.as_ref());
                            let preferred_script = options
                                .config
                                .multilingual
                                .as_ref()
                                .and_then(|m| m.preferred_script.as_ref());
                            let locale_str = "en"; // TODO: get from options.locale

                            let names_vec = crate::values::resolve_multilingual_name(
                                &editors,
                                mode,
                                preferred_script,
                                locale_str,
                            );
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
                                    self.sort_separator.as_ref(),
                                    self.shorten.as_ref(),
                                    self.and.as_ref(),
                                    effective_rendering.initialize_with.as_ref(),
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
                                            .map(|term| {
                                                let term_str =
                                                    if crate::values::should_strip_periods(
                                                        &effective_rendering,
                                                        options,
                                                    ) {
                                                        crate::values::strip_trailing_periods(term)
                                                    } else {
                                                        term.to_string()
                                                    };
                                                format!(" ({})", term_str)
                                            })
                                    })
                                } else {
                                    None
                                };

                                let url = crate::values::resolve_effective_url(
                                    self.links.as_ref(),
                                    options.config.links.as_ref(),
                                    reference,
                                    csln_core::options::LinkAnchor::Component,
                                );

                                return Some(ProcValues {
                                    value: formatted,
                                    prefix: None,
                                    suffix,
                                    url,
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

                            // Check if links should be applied to substituted title
                            let url = crate::values::resolve_effective_url(
                                self.links.as_ref(),
                                options.config.links.as_ref(),
                                reference,
                                csln_core::options::LinkAnchor::Title,
                            );

                            return Some(ProcValues {
                                value,
                                prefix: None,
                                suffix: None,
                                url,
                                substituted_key: Some("title:Primary".to_string()),
                            });
                        }
                    }
                    SubstituteKey::Translator => {
                        if let Some(translators) = reference.translator() {
                            let mode = options
                                .config
                                .multilingual
                                .as_ref()
                                .and_then(|m| m.name_mode.as_ref());
                            let preferred_script = options
                                .config
                                .multilingual
                                .as_ref()
                                .and_then(|m| m.preferred_script.as_ref());
                            let locale_str = "en"; // TODO: get from options.locale

                            let names_vec = crate::values::resolve_multilingual_name(
                                &translators,
                                mode,
                                preferred_script,
                                locale_str,
                            );
                            if !names_vec.is_empty() {
                                let formatted = format_names(
                                    &names_vec,
                                    &self.form,
                                    options,
                                    self.name_order.as_ref(),
                                    self.sort_separator.as_ref(),
                                    self.shorten.as_ref(),
                                    self.and.as_ref(),
                                    effective_rendering.initialize_with.as_ref(),
                                    hints,
                                );

                                let url = crate::values::resolve_effective_url(
                                    self.links.as_ref(),
                                    options.config.links.as_ref(),
                                    reference,
                                    csln_core::options::LinkAnchor::Component,
                                );

                                return Some(ProcValues {
                                    value: formatted,
                                    prefix: None,
                                    suffix: Some(" (Trans.)".to_string()),
                                    url,
                                    substituted_key: None,
                                });
                            }
                        }
                    }
                }
            }
            return None;
        }

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

        // Resolve effective rendering options (base merged with type-specific override)
        let mut effective_rendering = self.rendering.clone();
        if let Some(overrides) = &self.overrides {
            // Apply "all" wildcard override first
            if let Some(all_override) = overrides.get("all") {
                effective_rendering.merge(all_override);
            }
            // Then apply specific type override
            if let Some(type_override) = overrides.get(&reference.ref_type()) {
                effective_rendering.merge(type_override);
            }
        }

        let formatted = format_names(
            &names_vec,
            &self.form,
            options,
            effective_name_order,
            self.sort_separator.as_ref(),
            self.shorten.as_ref(),
            self.and.as_ref(),
            effective_rendering.initialize_with.as_ref(),
            hints,
        );

        // Check for explicit label configuration first
        let (role_prefix, role_suffix) = if let Some(label_config) = &self.label {
            use csln_core::template::{LabelPlacement, RoleLabelForm};

            // Determine if plural based on contributor count
            let plural = names_vec.len() > 1;

            // Map label form to term form
            let term_form = match label_config.form {
                RoleLabelForm::Short => TermForm::Short,
                RoleLabelForm::Long => TermForm::Long,
            };

            // Parse the role from term string (e.g., "editor" -> ContributorRole::Editor)
            let role = match label_config.term.as_str() {
                "editor" => Some(ContributorRole::Editor),
                "translator" => Some(ContributorRole::Translator),
                _ => Some(self.contributor.clone()), // Fall back to current role
            };

            // Look up term from locale
            let term_text = role.and_then(|r| options.locale.role_term(&r, plural, term_form));

            // Apply placement
            match label_config.placement {
                LabelPlacement::Prefix => (term_text.map(|t| format!("{} ", t)), None),
                LabelPlacement::Suffix => (None, term_text.map(|t| format!(" {}", t))),
            }
        } else {
            // Fall back to global editor_label_format configuration
            let editor_format = options
                .config
                .contributors
                .as_ref()
                .and_then(|c| c.editor_label_format);

            if let Some(format) = editor_format {
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
                            (
                                term.map(|t| {
                                    let term_str = if crate::values::should_strip_periods(
                                        &effective_rendering,
                                        options,
                                    ) {
                                        crate::values::strip_trailing_periods(t)
                                    } else {
                                        t.to_string()
                                    };
                                    format!("{} ", term_str)
                                }),
                                None,
                            )
                        }
                        EditorLabelFormat::ShortSuffix => {
                            let term = options.locale.role_term(
                                &self.contributor,
                                plural,
                                TermForm::Short,
                            );
                            (
                                None,
                                term.map(|t| {
                                    let term_str = if crate::values::should_strip_periods(
                                        &effective_rendering,
                                        options,
                                    ) {
                                        crate::values::strip_trailing_periods(t)
                                    } else {
                                        t.to_string()
                                    };
                                    format!(" ({})", term_str)
                                }),
                            )
                        }
                        EditorLabelFormat::LongSuffix => {
                            let term =
                                options
                                    .locale
                                    .role_term(&self.contributor, plural, TermForm::Long);
                            (
                                None,
                                term.map(|t| {
                                    let term_str = if crate::values::should_strip_periods(
                                        &effective_rendering,
                                        options,
                                    ) {
                                        crate::values::strip_trailing_periods(t)
                                    } else {
                                        t.to_string()
                                    };
                                    format!(", {}", term_str)
                                }),
                            )
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
                        (
                            term.map(|t| {
                                let term_str = if crate::values::should_strip_periods(
                                    &effective_rendering,
                                    options,
                                ) {
                                    crate::values::strip_trailing_periods(t)
                                } else {
                                    t.to_string()
                                };
                                format!("{} ", term_str)
                            }),
                            None,
                        )
                    }
                    (
                        ContributorForm::Long,
                        ContributorRole::Editor | ContributorRole::Translator,
                    ) => {
                        let plural = names_vec.len() > 1;
                        let term =
                            options
                                .locale
                                .role_term(&self.contributor, plural, TermForm::Short);
                        (
                            None,
                            term.map(|t| {
                                let term_str = if crate::values::should_strip_periods(
                                    &effective_rendering,
                                    options,
                                ) {
                                    crate::values::strip_trailing_periods(t)
                                } else {
                                    t.to_string()
                                };
                                format!(" ({})", term_str)
                            }),
                        )
                    }
                    _ => (None, None),
                }
            }
        };

        Some(ProcValues {
            value: formatted,
            prefix: role_prefix,
            suffix: role_suffix,
            url: crate::values::resolve_effective_url(
                self.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                csln_core::options::LinkAnchor::Component, // Contributors only link if explicit or whole-component
            ),
            substituted_key: None,
        })
    }
}

/// Format a list of names according to style options.
#[allow(clippy::too_many_arguments)]
pub fn format_names(
    names: &[crate::reference::FlatName],
    form: &ContributorForm,
    options: &RenderOptions<'_>,
    name_order: Option<&csln_core::template::NameOrder>,
    sort_separator_override: Option<&String>,
    shorten_override: Option<&ShortenListOptions>,
    and_override: Option<&AndOptions>,
    initialize_with_override: Option<&String>,
    hints: &ProcHints,
) -> String {
    if names.is_empty() {
        return String::new();
    }

    let config = options.config.contributors.as_ref();
    let locale = options.locale;

    // Determine shortening options:
    // 1. Use explicit override from template (e.g. bibliography et-al)
    // 2. Else use global config
    let shorten = shorten_override.or_else(|| config.and_then(|c| c.shorten.as_ref()));

    let (first_names, use_et_al, last_names) = if let Some(opts) = shorten {
        // Phase 3: Et-al Disambiguation Logic
        // When min_names_to_show is set (name expansion disambiguation),
        // determine effective threshold for et-al application.
        let effective_min = if let Some(expanded) = hints.min_names_to_show {
            // Name expansion disambiguation: show at least 'expanded' names.
            // If normal et-al threshold is met, apply et-al but show 'expanded' names.
            expanded.max(opts.use_first as usize)
        } else {
            // Normal mode: use standard et-al threshold
            opts.use_first as usize
        };

        // Apply et-al only if the list exceeds the minimum threshold
        if names.len() >= opts.min as usize {
            if effective_min >= names.len() {
                // Show all names (no et-al)
                (names.iter().collect::<Vec<_>>(), false, Vec::new())
            } else {
                // Apply et-al with effective minimum shown
                let first: Vec<&crate::reference::FlatName> =
                    names.iter().take(effective_min).collect();
                let last: Vec<&crate::reference::FlatName> = if let Some(ul) = opts.use_last {
                    // Show ul last names. Ensure no overlap with first names.
                    let take_last = ul as usize;
                    let skip = std::cmp::max(effective_min, names.len().saturating_sub(take_last));
                    names.iter().skip(skip).collect()
                } else {
                    Vec::new()
                };
                (first, true, last)
            }
        } else {
            // Below et-al threshold: show all names
            (names.iter().collect::<Vec<_>>(), false, Vec::new())
        }
    } else {
        (names.iter().collect::<Vec<_>>(), false, Vec::new())
    };

    // Format each name
    // Use explicit name_order if provided, otherwise use global display_as_sort
    let display_as_sort = config.and_then(|c| c.display_as_sort);
    let initialize_with =
        initialize_with_override.or_else(|| config.and_then(|c| c.initialize_with.as_ref()));
    let initialize_with_hyphen = config.and_then(|c| c.initialize_with_hyphen);
    let demote_ndp = config.and_then(|c| c.demote_non_dropping_particle.as_ref());
    let sort_separator =
        sort_separator_override.or_else(|| config.and_then(|c| c.sort_separator.as_ref()));
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
                sort_separator,
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
                sort_separator,
                hints.expand_given_names,
            )
        })
        .collect();

    // Determine "and" setting: use override if provided, else global config
    let mut and_option = and_override.or_else(|| config.and_then(|c| c.and.as_ref()));

    // Resolve mode-dependent "and" if necessary
    while let Some(AndOptions::ModeDependent {
        integral,
        non_integral,
    }) = and_option
    {
        if options.context == RenderContext::Citation {
            and_option = if options.mode == csln_core::citation::CitationMode::Integral {
                Some(integral)
            } else {
                Some(non_integral)
            };
        } else {
            // In bibliography, always use the non-integral (parenthetical) conjunction style
            // for APA (which uses & in bib but 'and' in narrative citations)
            and_option = Some(non_integral);
        }
    }

    // Determine conjunction between last two names
    // Default (None or no config) means no conjunction, matching CSL behavior
    let and_str = match and_option {
        Some(AndOptions::Text) => Some(locale.and_term(false)),
        Some(AndOptions::Symbol) => Some(locale.and_term(true)),
        Some(AndOptions::None) | None => None, // No conjunction
        _ => None,                             // Already resolved ModeDependent
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
        let conjunction = and_str.as_ref().unwrap();
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
                formatted_first[0], delimiter, conjunction, formatted_first[1]
            )
        } else {
            format!(
                "{} {} {}",
                formatted_first[0], conjunction, formatted_first[1]
            )
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
pub fn format_single_name(
    name: &crate::reference::FlatName,
    form: &ContributorForm,
    index: usize,
    display_as_sort: &Option<DisplayAsSort>,
    name_order: Option<&csln_core::template::NameOrder>,
    initialize_with: Option<&String>,
    initialize_with_hyphen: Option<bool>,
    demote_ndp: Option<&DemoteNonDroppingParticle>,
    sort_separator: Option<&String>,
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
                // Family Part + sort_separator + Given Part + Particle Part + Suffix
                let sep = sort_separator.map(|s| s.as_str()).unwrap_or(", ");
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
                    format!("{}{}{}", family_part, sep, suffix_part)
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

/// Format contributors in short form for citation grouping.
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
        None,
        None,
        None,
        &ProcHints::default(),
    )
}
