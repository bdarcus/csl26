/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Role-label resolution for contributor rendering.

use super::contributor_role_to_reference_role;
use crate::reference::Reference;
use crate::render::format::OutputFormat;
use crate::values::{RenderContext, RenderOptions};
use citum_schema::locale::{GrammaticalGender, TermForm};
use citum_schema::options::RoleLabelPreset;
use citum_schema::reference::ContributorGender;
use citum_schema::template::{ContributorForm, ContributorRole, Rendering, TemplateContributor};

fn map_contributor_gender(gender: ContributorGender) -> GrammaticalGender {
    match gender {
        ContributorGender::Masculine => GrammaticalGender::Masculine,
        ContributorGender::Feminine => GrammaticalGender::Feminine,
        ContributorGender::Neuter => GrammaticalGender::Neuter,
        ContributorGender::Common => GrammaticalGender::Common,
    }
}

#[derive(Debug, Clone)]
pub(super) enum RoleGenderRequest {
    Specific(GrammaticalGender),
    Neutral,
}

/// How to resolve and transform a role-label term: the requested grammatical
/// gender and an optional `text-case` transform applied to the resolved term.
#[derive(Debug, Clone, Default)]
pub(super) struct RoleLabelTermOptions {
    pub gender: Option<RoleGenderRequest>,
    pub text_case: Option<citum_schema::options::titles::TextCase>,
}

/// Inputs shared by contributor role-label resolution paths.
pub(super) struct RoleLabelContext<'a, 'options, F: OutputFormat<Output = String>> {
    pub(super) component: &'a TemplateContributor,
    pub(super) role: &'a ContributorRole,
    pub(super) reference: &'a Reference,
    pub(super) names_count: usize,
    pub(super) effective_rendering: &'a Rendering,
    pub(super) options: &'a RenderOptions<'options>,
    pub(super) fmt: &'a F,
    pub(super) role_omitted: bool,
}

fn requested_role_gender(
    component: &TemplateContributor,
    role: &ContributorRole,
    reference: &Reference,
) -> Option<RoleGenderRequest> {
    if let Some(gender) = &component.gender {
        return Some(RoleGenderRequest::Specific(gender.clone()));
    }

    let data_role = contributor_role_to_reference_role(role)?;

    let entries = reference.contributor_entries(&data_role);
    let mut genders = entries
        .iter()
        .filter_map(|entry| entry.gender.map(map_contributor_gender));
    let first = genders.next()?;

    if genders.all(|gender| gender == first) {
        Some(RoleGenderRequest::Specific(first))
    } else {
        Some(RoleGenderRequest::Neutral)
    }
}

fn resolve_role_term_by_request(
    locale: &citum_schema::locale::Locale,
    role: &ContributorRole,
    plural: bool,
    term_form: TermForm,
    requested_gender: Option<RoleGenderRequest>,
) -> Option<String> {
    match requested_gender {
        Some(RoleGenderRequest::Specific(gender)) => {
            locale.resolved_role_term(role, plural, &term_form, Some(gender))
        }
        Some(RoleGenderRequest::Neutral) => {
            locale.resolved_role_term_neutral(role, plural, &term_form)
        }
        None => locale.resolved_role_term(role, plural, &term_form, None),
    }
}

/// Resolve a configured role-label preset to `(prefix, suffix)`.
pub(super) fn resolve_role_label_preset<F: OutputFormat<Output = String>>(
    role: &ContributorRole,
    preset: RoleLabelPreset,
    names_count: usize,
    term_opts: RoleLabelTermOptions,
    effective_rendering: &Rendering,
    options: &RenderOptions<'_>,
    fmt: &F,
) -> (Option<String>, Option<String>) {
    let plural = names_count > 1;
    let language = options.locale.locale.as_str();
    let RoleLabelTermOptions {
        gender: requested_gender,
        text_case,
    } = term_opts;
    match preset {
        RoleLabelPreset::None => (None, None),
        RoleLabelPreset::VerbPrefix => {
            let term = options
                .locale
                .resolved_role_term(role, plural, &TermForm::Verb, None);
            (
                term.map(|t| {
                    super::format_role_term::<F>(&t, fmt, effective_rendering, options, "", " ")
                }),
                None,
            )
        }
        RoleLabelPreset::VerbShortPrefix => {
            let term = options
                .locale
                .resolved_role_term(role, plural, &TermForm::VerbShort, None);
            (
                term.map(|t| {
                    super::format_role_term::<F>(&t, fmt, effective_rendering, options, "", " ")
                }),
                None,
            )
        }
        RoleLabelPreset::ShortSuffix => {
            let term = resolve_role_term_by_request(
                options.locale,
                role,
                plural,
                TermForm::Short,
                requested_gender,
            )
            .map(|t| apply_label_case(t, text_case, language));
            (
                None,
                term.map(|t| {
                    super::format_role_term::<F>(&t, fmt, effective_rendering, options, " (", ")")
                }),
            )
        }
        RoleLabelPreset::ShortSuffixComma => {
            let term = resolve_role_term_by_request(
                options.locale,
                role,
                plural,
                TermForm::Short,
                requested_gender,
            )
            .map(|t| apply_label_case(t, text_case, language));
            (
                None,
                term.map(|t| {
                    super::format_role_term::<F>(&t, fmt, effective_rendering, options, ", ", "")
                }),
            )
        }
        RoleLabelPreset::LongSuffix => {
            let term = resolve_role_term_by_request(
                options.locale,
                role,
                plural,
                TermForm::Long,
                requested_gender,
            )
            .map(|t| apply_label_case(t, text_case, language));
            (
                None,
                term.map(|t| {
                    super::format_role_term::<F>(&t, fmt, effective_rendering, options, ", ", "")
                }),
            )
        }
    }
}

/// Apply an optional `text-case` transform to a resolved role-label term.
///
/// Used so a style can render, e.g., "Eds." from the locale's "eds." (IEEE).
/// `language` gates English-only transforms via [`resolve_text_case`].
pub(super) fn apply_label_case(
    term: String,
    text_case: Option<citum_schema::options::titles::TextCase>,
    language: &str,
) -> String {
    match text_case {
        Some(case) => {
            let resolved = crate::values::text_case::resolve_text_case(case, Some(language));
            crate::values::text_case::apply_text_case_with_language(&term, resolved, Some(language))
        }
        None => term,
    }
}

/// Resolve a contributor's explicit `label` config to `(prefix, suffix)`.
///
/// Honours the term key, short/long form, optional `text-case`, and placement.
/// `label.term` keys recognized by [`resolve_explicit_label`]. Shared with
/// the style-load-time warning scan (`api::warnings`) so the two can't drift.
pub(crate) const RECOGNIZED_LABEL_TERMS: &[&str] = &["chair", "director", "editor", "translator"];

fn resolve_explicit_label<F: OutputFormat<Output = String>>(
    label_config: &citum_schema::template::RoleLabel,
    context: &RoleLabelContext<'_, '_, F>,
) -> (Option<String>, Option<String>) {
    use citum_schema::template::RoleLabelForm;

    let plural = context.names_count > 1;
    let term_form = match label_config.form {
        RoleLabelForm::Short => TermForm::Short,
        RoleLabelForm::Long => TermForm::Long,
    };

    let role = match label_config.term.as_str() {
        "chair" => Some(ContributorRole::Chair),
        "director" => Some(ContributorRole::Director),
        "editor" => Some(ContributorRole::Editor),
        "translator" => Some(ContributorRole::Translator),
        _ => Some(context.role.clone()),
    };

    let requested_gender =
        requested_role_gender(context.component, context.role, context.reference);
    let term_text = role
        .and_then(|r| {
            resolve_role_term_by_request(
                context.options.locale,
                &r,
                plural,
                term_form,
                requested_gender,
            )
        })
        .map(|t| {
            apply_label_case(
                t,
                label_config.text_case,
                context.options.locale.locale.as_str(),
            )
        });

    place_explicit_term::<F>(
        label_config,
        term_text,
        context.effective_rendering,
        context.options,
        context.fmt,
        crate::values::effective_item_language(context.reference).as_deref(),
    )
}

pub(super) fn realize_label_affix(
    affix: Option<&citum_schema::template::DelimiterPunctuation>,
    fallback: &str,
    options: &RenderOptions<'_>,
    item_language: Option<&str>,
    position: crate::render::format::PunctuationPosition,
) -> String {
    let Some(affix) = affix else {
        return fallback.to_string();
    };
    let (script, realization) = crate::values::punctuation_realization_context(
        item_language,
        options.config.multilingual.as_ref(),
        options.locale.punctuation_realization.as_ref(),
    );
    crate::render::format::realize_punctuation(affix, script, realization.as_deref(), position)
        .into_owned()
}

/// Place an already-resolved term using an explicit role-label configuration.
pub(super) fn place_explicit_term<F: OutputFormat<Output = String>>(
    label_config: &citum_schema::template::RoleLabel,
    term_text: Option<String>,
    effective_rendering: &Rendering,
    options: &RenderOptions<'_>,
    fmt: &F,
    item_language: Option<&str>,
) -> (Option<String>, Option<String>) {
    use citum_schema::template::LabelPlacement;

    // Explicit label affixes override the placement-derived defaults,
    // mirroring CSL 1.0 `cs:label` prefix/suffix (e.g. `" ("`/`")"`).
    let (default_before, default_after) = match (&label_config.placement, &label_config.wrap) {
        (LabelPlacement::Suffix, Some(_)) => (" ", ""),
        (LabelPlacement::Prefix, _) => ("", " "),
        (LabelPlacement::Suffix, None) => (", ", ""),
    };
    let before = realize_label_affix(
        label_config.prefix.as_ref(),
        default_before,
        options,
        item_language,
        crate::render::format::PunctuationPosition::Prefix,
    );
    let after = realize_label_affix(
        label_config.suffix.as_ref(),
        default_after,
        options,
        item_language,
        crate::render::format::PunctuationPosition::Suffix,
    );

    match label_config.placement {
        LabelPlacement::Prefix => (
            term_text.map(|t| {
                super::format_wrapped_role_term::<F>(
                    &t,
                    fmt,
                    effective_rendering,
                    options,
                    (&before, &after),
                    label_config.wrap.as_deref(),
                    item_language,
                )
            }),
            None,
        ),
        LabelPlacement::Suffix => (
            None,
            term_text.map(|t| {
                super::format_wrapped_role_term::<F>(
                    &t,
                    fmt,
                    effective_rendering,
                    options,
                    (&before, &after),
                    label_config.wrap.as_deref(),
                    item_language,
                )
            }),
        ),
    }
}

fn configured_structural_label<F: OutputFormat<Output = String>>(
    context: &RoleLabelContext<'_, '_, F>,
) -> Option<citum_schema::template::RoleLabel> {
    if context.options.context != RenderContext::Bibliography {
        return None;
    }
    let contributors = context.options.config.contributors.as_ref()?;
    let build = |presentation: &citum_schema::options::RoleLabelPresentation| {
        citum_schema::template::RoleLabel {
            term: context.role.as_str().to_string(),
            form: presentation.form.clone(),
            placement: presentation.placement.clone(),
            text_case: presentation.text_case,
            wrap: presentation.wrap.clone(),
            prefix: presentation.prefix.clone().map(Into::into),
            suffix: presentation.suffix.clone().map(Into::into),
        }
    };

    if let Some(presentation) = contributors.role_label_presentation(context.role) {
        return Some(build(presentation));
    }
    contributors
        .role
        .as_ref()?
        .defaults?
        .presentation_for(context.role)
        .map(|presentation| build(&presentation))
}

/// Resolve the role-label prefix and suffix for a formatted contributor.
///
/// Returns `(prefix, suffix)` strings to wrap the formatted name list.
/// Precedence: explicit `label` config > configured role presets > form-based defaults.
pub(super) fn resolve_role_labels<F: OutputFormat<Output = String>>(
    context: RoleLabelContext<'_, '_, F>,
) -> (Option<String>, Option<String>) {
    if let Some(label_config) = &context.component.label {
        return resolve_explicit_label(label_config, &context);
    }
    // `role.omit` also suppresses a configured style-wide structural label
    // (an explicit `role_label_presentation` override or a `role.defaults`
    // preset bundle), subject to the same verb-form exception applied below
    // to the remaining preset tiers: a `form: verb`/`form: verb-short`
    // component's label is structural, not decorative, and is never omitted.
    let verb_form = matches!(
        context.component.form,
        ContributorForm::Verb | ContributorForm::VerbShort
    );
    if (!context.role_omitted || verb_form)
        && let Some(label) = configured_structural_label(&context)
    {
        return resolve_explicit_label(&label, &context);
    }
    let RoleLabelContext {
        component,
        role,
        reference,
        names_count,
        effective_rendering,
        options,
        fmt,
        role_omitted,
    } = context;

    // `role.omit` suppresses the *decorative* default/preset label (e.g. a
    // trailing "(Trans.)"), not a `form: verb`/`form: verb-short` label: the
    // verb phrase ("Translated by X") is structural to that form, not an
    // optional suffix, so a style that both requests verb form and omits the
    // role's default label (to avoid double-labeling under a different form)
    // must still get its verb label.
    if role_omitted
        && !matches!(
            component.form,
            ContributorForm::Verb | ContributorForm::VerbShort
        )
    {
        return (None, None);
    }

    if let Some(preset) = options
        .config
        .contributors
        .as_ref()
        .and_then(|contributors| contributors.effective_role_label_preset(role))
    {
        let requested_gender = requested_role_gender(component, role, reference);
        return resolve_role_label_preset(
            role,
            preset,
            names_count,
            RoleLabelTermOptions {
                gender: requested_gender,
                text_case: None,
            },
            effective_rendering,
            options,
            fmt,
        );
    }

    // Style-declared default bundle (`contributors.role.defaults`). Role
    // labels are a bibliography-only convention in every examined style
    // guide (div-012), so the bundle never fires in citation context.
    if options.context == RenderContext::Bibliography
        && !matches!(
            component.form,
            ContributorForm::Verb | ContributorForm::VerbShort
        )
        && let Some(preset) = options
            .config
            .contributors
            .as_ref()
            .and_then(|contributors| contributors.default_role_label_preset(role))
    {
        let requested_gender = requested_role_gender(component, role, reference);
        return resolve_role_label_preset(
            role,
            preset,
            names_count,
            RoleLabelTermOptions {
                gender: requested_gender,
                text_case: None,
            },
            effective_rendering,
            options,
            fmt,
        );
    }

    // Form-based defaults: verb forms carry their structural verb phrase;
    // no other form receives an automatic label.
    match &component.form {
        ContributorForm::Verb | ContributorForm::VerbShort => {
            let plural = names_count > 1;
            let term_form = match component.form {
                ContributorForm::VerbShort => TermForm::VerbShort,
                _ => TermForm::Verb,
            };
            let term = options
                .locale
                .resolved_role_term(role, plural, &term_form, None);

            (
                term.map(|t| {
                    super::format_role_term::<F>(&t, fmt, effective_rendering, options, "", " ")
                }),
                None,
            )
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::apply_label_case;
    use citum_schema::options::titles::TextCase;

    #[test]
    fn capitalize_first_uppercases_label_initial() {
        // given the locale's lowercase short editor term and capitalize-first
        let out = apply_label_case("eds.".to_string(), Some(TextCase::CapitalizeFirst), "en-US");
        // then the IEEE-style capitalized form is produced
        assert_eq!(out, "Eds.");
    }

    #[test]
    fn no_text_case_leaves_term_unchanged() {
        // given no text-case transform
        let out = apply_label_case("eds.".to_string(), None, "en-US");
        // then the term is returned verbatim
        assert_eq!(out, "eds.");
    }
}
