/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Rendering logic for contributors (authors, editors, translators).
//!
//! This module handles contributor rendering with support for name ordering,
//! role labels, et-al formatting, and multilingual name resolution.

pub(crate) mod labels;
pub(crate) mod merged;
pub mod names;
pub(crate) mod substitute;

use crate::reference::Reference;
use crate::values::{ComponentValues, ProcHints, ProcValues, RenderContext, RenderOptions};
use citum_schema::options::SubsequentNameForm;
use citum_schema::template::{
    ContributorForm, ContributorRole, TemplateComponent, TemplateContributor,
};

#[cfg(test)]
pub(crate) use names::{NameFormatContext, format_single_name};
pub use names::{NamesOverrides, format_contributors_short, format_names};

/// Resolve a contributor payload for a template contributor role.
///
/// This preserves the legacy `editor()` / `translator()` accessors for
/// reference shapes that still store those roles outside the generic
/// contributor-entry list.
pub(super) fn contributor_for_role(
    reference: &Reference,
    role: &ContributorRole,
) -> Option<citum_schema::reference::Contributor> {
    match role {
        ContributorRole::Author => reference.author(),
        ContributorRole::Editor => reference.editor(),
        ContributorRole::Translator => reference.translator(),
        _ => contributor_role_to_reference_role(role).and_then(|role| reference.contributor(role)),
    }
}

/// Map a template contributor role to the corresponding reference contributor role.
pub(crate) fn contributor_role_to_reference_role(
    role: &ContributorRole,
) -> Option<citum_schema::reference::ContributorRole> {
    match role {
        ContributorRole::Author => Some(citum_schema::reference::ContributorRole::Author),
        ContributorRole::Editor => Some(citum_schema::reference::ContributorRole::Editor),
        ContributorRole::Translator => Some(citum_schema::reference::ContributorRole::Translator),
        ContributorRole::Annotator => Some(citum_schema::reference::ContributorRole::Annotator),
        ContributorRole::Commentator => Some(citum_schema::reference::ContributorRole::Commentator),
        ContributorRole::ForewordAuthor => {
            Some(citum_schema::reference::ContributorRole::ForewordAuthor)
        }
        ContributorRole::IntroductionAuthor => {
            Some(citum_schema::reference::ContributorRole::IntroductionAuthor)
        }
        ContributorRole::AfterwordAuthor => {
            Some(citum_schema::reference::ContributorRole::AfterwordAuthor)
        }
        ContributorRole::Recipient => Some(citum_schema::reference::ContributorRole::Recipient),
        ContributorRole::Chair => Some(citum_schema::reference::ContributorRole::Unknown(
            "chair".to_string(),
        )),
        ContributorRole::Interviewer => Some(citum_schema::reference::ContributorRole::Interviewer),
        ContributorRole::Guest => Some(citum_schema::reference::ContributorRole::Guest),
        ContributorRole::Performer => Some(citum_schema::reference::ContributorRole::Performer),
        ContributorRole::Director => Some(citum_schema::reference::ContributorRole::Director),
        ContributorRole::Composer => Some(citum_schema::reference::ContributorRole::Composer),
        ContributorRole::Writer => Some(citum_schema::reference::ContributorRole::Writer),
        ContributorRole::Producer => Some(citum_schema::reference::ContributorRole::Producer),
        ContributorRole::Illustrator => Some(citum_schema::reference::ContributorRole::Illustrator),
        ContributorRole::Narrator => Some(citum_schema::reference::ContributorRole::Narrator),
        ContributorRole::Inventor => Some(citum_schema::reference::ContributorRole::Unknown(
            "inventor".to_string(),
        )),
        ContributorRole::Counsel => Some(citum_schema::reference::ContributorRole::Unknown(
            "counsel".to_string(),
        )),
        ContributorRole::CollectionEditor => Some(
            citum_schema::reference::ContributorRole::Unknown("collection-editor".to_string()),
        ),
        ContributorRole::ContainerAuthor => Some(
            citum_schema::reference::ContributorRole::Unknown("container-author".to_string()),
        ),
        ContributorRole::EditorialDirector => Some(
            citum_schema::reference::ContributorRole::Unknown("editorial-director".to_string()),
        ),
        ContributorRole::TextualEditor => Some(citum_schema::reference::ContributorRole::Unknown(
            "textual-editor".to_string(),
        )),
        ContributorRole::OriginalAuthor => Some(citum_schema::reference::ContributorRole::Unknown(
            "original-author".to_string(),
        )),
        ContributorRole::ReviewedAuthor => Some(citum_schema::reference::ContributorRole::Unknown(
            "reviewed-author".to_string(),
        )),
        ContributorRole::Unknown(role) => Some(match role.as_str() {
            "compiler" => citum_schema::reference::ContributorRole::Compiler,
            "performer" => citum_schema::reference::ContributorRole::Performer,
            "narrator" => citum_schema::reference::ContributorRole::Narrator,
            "host" => citum_schema::reference::ContributorRole::Host,
            "producer" | "executive-producer" => citum_schema::reference::ContributorRole::Producer,
            "writer" => citum_schema::reference::ContributorRole::Writer,
            _ => citum_schema::reference::ContributorRole::Unknown(role.clone()),
        }),
        ContributorRole::Interviewee | ContributorRole::Publisher => None,
        _ => None,
    }
}

/// Checks if a contributor role label should be omitted for a given reference.
///
/// Returns true if the role appears in the configuration's role.omit list.
pub(super) fn is_role_label_omitted(options: &RenderOptions<'_>, role: &ContributorRole) -> bool {
    options
        .config
        .contributors
        .as_ref()
        .and_then(|c| c.role.as_ref())
        .is_some_and(|role_opts| {
            role_opts
                .omit
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(role.as_str()))
        })
}

/// Format a role term with period stripping if configured.
///
/// Handles the repeated pattern of checking `should_strip_periods` and formatting
/// the result with a given prefix and suffix pattern.
pub(super) fn format_role_term<F: crate::render::format::OutputFormat<Output = String>>(
    term: &str,
    fmt: &F,
    effective_rendering: &citum_schema::template::Rendering,
    options: &RenderOptions<'_>,
    prefix: &str,
    suffix: &str,
) -> String {
    let term_str = normalized_role_term(term, effective_rendering, options);
    fmt.text(&format!("{prefix}{term_str}{suffix}"))
}

fn normalized_role_term(
    term: &str,
    effective_rendering: &citum_schema::template::Rendering,
    options: &RenderOptions<'_>,
) -> String {
    let term_str = if crate::values::should_strip_periods(effective_rendering, options) {
        crate::values::strip_trailing_periods(term)
    } else {
        term.to_string()
    };
    // Locale role terms are stored lowercase (e.g. "translated by") since
    // they usually sit mid-sentence. A `form: verb` component is marked
    // `pre_formatted`, which skips the generic title/value text-case pass,
    // so a style that positions the verb label as its own clause (e.g.
    // after a `". "` prefix) must opt in here explicitly.
    match effective_rendering.text_case {
        Some(citum_schema::options::titles::TextCase::CapitalizeFirst) => {
            crate::values::text_case::apply_text_case_with_language(
                &term_str,
                citum_schema::options::titles::TextCase::CapitalizeFirst,
                Some(options.locale.locale.as_str()),
            )
        }
        _ => term_str,
    }
}

/// Format a role term with optional structural wrapping.
///
/// The unwrapped path delegates to [`format_role_term`] to preserve its exact
/// escaping and affix behavior. Wrapped labels apply inner affixes, wrapping
/// punctuation, and finally the outer label affixes.
pub(super) fn format_wrapped_role_term<F: crate::render::format::OutputFormat<Output = String>>(
    term: &str,
    fmt: &F,
    effective_rendering: &citum_schema::template::Rendering,
    options: &RenderOptions<'_>,
    affixes: (&str, &str),
    wrap: Option<&citum_schema::template::WrapConfig>,
    item_language: Option<&str>,
) -> String {
    let (prefix, suffix) = affixes;
    let Some(wrap) = wrap else {
        return format_role_term(term, fmt, effective_rendering, options, prefix, suffix);
    };
    let term = normalized_role_term(term, effective_rendering, options);
    let content = fmt.text(&term);
    let content = fmt.inner_affix(
        wrap.inner_prefix.as_deref().unwrap_or_default(),
        content,
        wrap.inner_suffix.as_deref().unwrap_or_default(),
    );
    let marks = crate::render::format::QuoteMarks::from(&options.locale.grammar_options);
    let (script, realization) = crate::values::punctuation_realization_context(
        item_language,
        options.config.multilingual.as_ref(),
        options.locale.punctuation_realization.as_ref(),
    );
    let content = fmt.wrap_punctuation(
        &wrap.punctuation,
        content,
        &marks,
        script,
        realization.as_deref(),
    );
    format!("{}{content}{}", fmt.text(prefix), fmt.text(suffix))
}

/// Apply the integral-citation subsequent-form rewrite to a contributor on a
/// `Subsequent` mention. No-op unless the style configures `integral-name-memory`.
fn apply_integral_subsequent_form(
    component: &mut TemplateContributor,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
) {
    if options.context != RenderContext::Citation {
        return;
    }
    if !matches!(options.mode, citum_schema::citation::CitationMode::Integral) {
        return;
    }
    if !component.contributor.contains(&ContributorRole::Author) {
        return;
    }
    if !matches!(
        hints.integral_name_state,
        Some(citum_schema::citation::IntegralNameState::Subsequent)
    ) {
        return;
    }
    let Some(memory) = options.config.integral_name_memory.as_ref() else {
        return;
    };
    component.form = match memory.resolve().subsequent_form {
        SubsequentNameForm::Short => ContributorForm::Short,
        SubsequentNameForm::FamilyOnly => ContributorForm::FamilyOnly,
    };
}

/// Build name overrides and format all names for a contributor component.
fn format_contributor_names(
    component: &TemplateContributor,
    role: &ContributorRole,
    names_vec: &[crate::reference::FlatName],
    reference: &Reference,
    effective_rendering: &citum_schema::template::Rendering,
    options: &RenderOptions<'_>,
    hints: &ProcHints,
) -> String {
    let effective_name_order = component.name_order.as_ref().or_else(|| {
        options
            .config
            .contributors
            .as_ref()?
            .effective_role_name_order(role)
    });
    let effective_shorten = component
        .shorten
        .as_ref()
        .or_else(|| options.config.contributors.as_ref()?.shorten.as_ref());

    // Priority chain for name_form:
    // 1. component.name_form (TemplateContributor-level override - highest priority)
    // 2. effective_rendering.name_form (from overrides, second priority)
    // 3. config (options-level fallback)
    let effective_name_form = component.name_form.or(effective_rendering.name_form);

    let name_overrides = names::NamesOverrides {
        name_order: effective_name_order,
        sort_separator: component.sort_separator.as_ref(),
        delimiter: component.delimiter.as_ref(),
        shorten: effective_shorten,
        and: component.and.as_ref(),
        initialize_with: effective_rendering.initialize_with.as_ref(),
        name_form: effective_name_form,
        strip_periods: effective_rendering.strip_periods,
        item_language: crate::values::effective_item_language(reference),
    };
    names::format_names(names_vec, &component.form, options, &name_overrides, hints)
}

/// Render the options-level terminal message after author candidates are exhausted.
fn resolve_author_fallback<F: crate::render::format::OutputFormat<Output = String>>(
    reference: &Reference,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    fmt: &F,
    substitute: &citum_schema::options::Substitute,
) -> Option<ProcValues<F::Output>> {
    let message = substitute.otherwise_message()?;
    let fallback = TemplateComponent::Message(message.to_template_message());
    let values = fallback.values::<F>(reference, hints, options)?;
    let substituted_key = values.substituted_key.clone();
    let output =
        crate::values::date::render_fallback_component(fmt, &fallback, values, reference, options);
    (!output.trim().is_empty()).then_some(ProcValues {
        value: output,
        prefix: None,
        suffix: None,
        url: None,
        substituted_key,
        pre_formatted: true,
    })
}

impl ComponentValues for TemplateContributor {
    #[allow(
        clippy::too_many_lines,
        reason = "large match statement for contributor role dispatch"
    )]
    fn values<F: crate::render::format::OutputFormat<Output = String>>(
        &self,
        reference: &Reference,
        hints: &ProcHints,
        options: &RenderOptions<'_>,
    ) -> Option<ProcValues<F::Output>> {
        let fmt = F::default();

        let mut component = self.clone();
        let effective_rendering = self.rendering.clone();

        // Apply integral-citation subsequent-form (FullThenShort rule)
        apply_integral_subsequent_form(&mut component, hints, options);

        // Respect explicit suppression before either contributor rendering path.
        if effective_rendering.suppress == Some(true) {
            return None;
        }

        let Some(role) = component.contributor.as_single().cloned() else {
            return merged::values::<F>(
                &component,
                reference,
                hints,
                options,
                &effective_rendering,
                &fmt,
            );
        };

        if merged::is_role_suppressed(reference, &role, &options.config) {
            return None;
        }

        // Resolve substitute config once for all substitute/suppression checks below.
        let substitute = options.config.effective_substitute();

        // The author slot is resolved as one effective-primary value so
        // rendering, sorting, and disambiguation share type overrides and
        // semantic-author precedence.
        if matches!(role, ContributorRole::Author) {
            if options.suppress_author {
                return None;
            }
            if let Some(values) = substitute::resolve_author_substitute::<F>(
                &component,
                hints,
                options,
                reference,
                &effective_rendering,
                &fmt,
                substitute.as_ref(),
            ) {
                return Some(values);
            }
            return resolve_author_fallback::<F>(
                reference,
                hints,
                options,
                &fmt,
                substitute.as_ref(),
            );
        }

        let contributor = contributor_for_role(reference, &role);

        // Check if this secondary role is suppressed by role-substitute
        // configuration. Primary-slot overrides deliberately promote roles
        // that may also appear in these secondary fallback chains.
        if substitute::is_role_suppressed_by_substitute(&role, substitute.as_ref(), reference) {
            return None;
        }

        // Resolve multilingual names if configured
        let names_vec = if let Some(contrib) = contributor {
            substitute::resolve_multilingual_for_contrib(&contrib, options)
        } else {
            Vec::new()
        };

        // Handle role-substitute if this role is empty.
        if names_vec.is_empty() {
            return substitute::resolve_role_substitute::<F>(
                &role,
                &component,
                hints,
                options,
                reference,
                &effective_rendering,
                &fmt,
                substitute.as_ref(),
            );
        }

        let formatted = format_contributor_names(
            &component,
            &role,
            &names_vec,
            reference,
            &effective_rendering,
            options,
            hints,
        );

        let role_omitted = is_role_label_omitted(options, &role);
        let (role_prefix, role_suffix) =
            labels::resolve_role_labels::<F>(labels::RoleLabelContext {
                component: &component,
                role: &role,
                reference,
                names_count: names_vec.len(),
                effective_rendering: &effective_rendering,
                options,
                fmt: &fmt,
                role_omitted,
            });

        let is_pre_formatted = role_prefix.is_some() || role_suffix.is_some();
        let formatted = crate::values::apply_abbreviation(formatted, options.abbreviation_map);
        let final_value = if is_pre_formatted {
            fmt.text(&formatted)
        } else {
            formatted
        };

        Some(ProcValues {
            value: final_value,
            prefix: role_prefix,
            suffix: role_suffix,
            url: crate::values::resolve_effective_url(
                component.links.as_ref(),
                options.config.links.as_ref(),
                reference,
                citum_schema::options::LinkAnchor::Component,
            ),
            substituted_key: None,
            pre_formatted: is_pre_formatted,
        })
    }
}
