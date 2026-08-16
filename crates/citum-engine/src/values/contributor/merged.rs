/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Cross-role contributor assembly and rendering.

use std::collections::HashSet;

use citum_schema::locale::TermForm;
use citum_schema::options::RoleLabelPreset;
use citum_schema::reference::Contributor;
use citum_schema::template::{
    ContributorForm, ContributorLabelMode, ContributorMerge, ContributorMergeOrder,
    ContributorRole, LabelPlacement, Rendering, RoleLabel, RoleLabelForm, TemplateContributor,
};

use super::names::{NameDecoration, NamesOverrides};
use super::{contributor_for_role, contributor_role_to_reference_role};
use crate::reference::{FlatName, Reference};
use crate::render::format::OutputFormat;
use crate::values::{ProcHints, ProcValues, RenderContext, RenderOptions};

#[derive(Debug, Clone)]
struct MergedName {
    name: FlatName,
    roles: Vec<ContributorRole>,
}

/// Render a list-form contributor component.
pub(super) fn values<F: OutputFormat<Output = String>>(
    component: &TemplateContributor,
    reference: &Reference,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    effective_rendering: &Rendering,
    fmt: &F,
) -> Option<ProcValues<F::Output>> {
    let roles = component.contributor.as_slice();
    let primary_role = roles.first()?;
    let merge = effective_merge(component, &options.config);
    let substitute = options.config.effective_substitute();
    let mut suppressed_roles = suppressed_roles(reference, roles, &options.config);
    suppressed_roles.extend(substitute_suppressed_roles(
        reference,
        roles,
        substitute.as_ref(),
    ));
    let entries = assemble_names(
        reference,
        roles,
        &merge,
        &suppressed_roles,
        &options.config,
        options.locale,
        options.suppress_author,
    );

    if entries.is_empty() {
        return resolve_empty_list::<F>(
            component,
            primary_role,
            reference,
            hints,
            options,
            effective_rendering,
            fmt,
        );
    }

    let names = entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    let name_overrides = NamesOverrides {
        name_order: component.name_order.as_ref().or_else(|| {
            options
                .config
                .contributors
                .as_ref()?
                .effective_role_name_order(primary_role)
        }),
        sort_separator: component.sort_separator.as_ref(),
        delimiter: component.delimiter.as_ref(),
        shorten: component
            .shorten
            .as_ref()
            .or_else(|| options.config.contributors.as_ref()?.shorten.as_ref()),
        and: component.and.as_ref(),
        initialize_with: effective_rendering.initialize_with.as_ref(),
        name_form: component.name_form.or(effective_rendering.name_form),
        strip_periods: effective_rendering.strip_periods,
        item_language: crate::values::effective_item_language(reference),
    };
    let selected_indices =
        super::names::selected_name_indices(names.len(), name_overrides.shorten, hints);
    let decorations = build_decorations::<F>(
        &entries,
        &selected_indices,
        &DecorationContext {
            component,
            reference,
            merge: &merge,
            effective_rendering,
            options,
            fmt,
        },
    );
    let formatted = super::names::format_names_decorated(
        &names,
        &component.form,
        options,
        &name_overrides,
        hints,
        &decorations,
    );
    let formatted = crate::values::apply_abbreviation(formatted, options.abbreviation_map);

    Some(ProcValues {
        value: fmt.text(&formatted),
        prefix: None,
        suffix: None,
        url: crate::values::resolve_effective_url(
            component.links.as_ref(),
            options.config.links.as_ref(),
            reference,
            citum_schema::options::LinkAnchor::Component,
        ),
        substituted_key: None,
        pre_formatted: true,
    })
}

fn assemble_names(
    reference: &Reference,
    roles: &[ContributorRole],
    merge: &ContributorMerge,
    suppressed_roles: &HashSet<ContributorRole>,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
    suppress_author: bool,
) -> Vec<MergedName> {
    let ordered_roles = match merge.order {
        ContributorMergeOrder::Document => None,
        ContributorMergeOrder::Role => Some(roles),
    };
    let mut result = Vec::new();

    if let Some(ordered_roles) = ordered_roles {
        for role in ordered_roles {
            if should_skip_role(role, suppressed_roles, suppress_author) {
                continue;
            }
            for entry in reference
                .all_contributor_entries()
                .iter()
                .filter(|entry| entry_has_template_role(entry, role))
            {
                let active_roles = matched_template_roles(entry, roles)
                    .into_iter()
                    .filter(|matched| !should_skip_role(matched, suppressed_roles, suppress_author))
                    .collect::<Vec<_>>();
                if merge.combine_same_person {
                    if active_roles.first() == Some(role) {
                        append_contributor(
                            &entry.contributor,
                            &active_roles,
                            config,
                            locale,
                            &mut result,
                        );
                    }
                } else {
                    append_contributor(
                        &entry.contributor,
                        std::slice::from_ref(role),
                        config,
                        locale,
                        &mut result,
                    );
                }
            }
        }
    } else {
        for entry in reference.all_contributor_entries() {
            let matched_roles = matched_template_roles(entry, roles);
            if matched_roles.is_empty() {
                continue;
            }
            let active_roles = matched_roles
                .into_iter()
                .filter(|role| !should_skip_role(role, suppressed_roles, suppress_author))
                .collect::<Vec<_>>();
            if active_roles.is_empty() {
                continue;
            }
            if merge.combine_same_person {
                append_contributor(
                    &entry.contributor,
                    &active_roles,
                    config,
                    locale,
                    &mut result,
                );
            } else {
                for role in active_roles {
                    append_contributor(
                        &entry.contributor,
                        std::slice::from_ref(&role),
                        config,
                        locale,
                        &mut result,
                    );
                }
            }
        }

        // Legacy reference shapes can expose author/editor/translator through
        // dedicated accessors without a unified entry. Add only roles absent
        // from the document-order vector.
        for role in roles {
            if reference.all_contributor_entries().iter().any(|entry| {
                contributor_role_to_reference_role(role)
                    .as_ref()
                    .is_some_and(|data_role| entry.roles.contains(data_role))
            }) || should_skip_role(role, suppressed_roles, suppress_author)
            {
                continue;
            }
            if let Some(contributor) = contributor_for_role(reference, role) {
                append_contributor(
                    &contributor,
                    std::slice::from_ref(role),
                    config,
                    locale,
                    &mut result,
                );
            }
        }
    }
    result
}

fn matched_template_roles(
    entry: &citum_schema::reference::ContributorEntry,
    declared_roles: &[ContributorRole],
) -> Vec<ContributorRole> {
    declared_roles
        .iter()
        .filter(|role| entry_has_template_role(entry, role))
        .cloned()
        .collect()
}

fn entry_has_template_role(
    entry: &citum_schema::reference::ContributorEntry,
    role: &ContributorRole,
) -> bool {
    contributor_role_to_reference_role(role)
        .as_ref()
        .is_some_and(|data_role| entry.roles.contains(data_role))
}

fn should_skip_role(
    role: &ContributorRole,
    suppressed_roles: &HashSet<ContributorRole>,
    suppress_author: bool,
) -> bool {
    suppressed_roles.contains(role) || (matches!(role, ContributorRole::Author) && suppress_author)
}

fn append_contributor(
    contributor: &Contributor,
    roles: &[ContributorRole],
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
    result: &mut Vec<MergedName>,
) {
    let resolved = semantic_contributor_names(contributor, config, locale);
    for name in resolved {
        result.push(MergedName {
            name,
            roles: roles.to_vec(),
        });
    }
}

/// Resolve one contributor into the multilingual display names used by semantic consumers.
pub(crate) fn semantic_contributor_names(
    contributor: &Contributor,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
) -> Vec<FlatName> {
    let multilingual = config.multilingual.as_ref();
    crate::values::resolve_multilingual_name(
        contributor,
        multilingual.and_then(|value| value.name_mode.as_ref()),
        multilingual.and_then(|value| value.preferred_transliteration.as_deref()),
        multilingual.and_then(|value| value.preferred_script.as_ref()),
        &locale.locale,
    )
}

fn suppressed_roles(
    reference: &Reference,
    declared_roles: &[ContributorRole],
    config: &citum_schema::options::Config,
) -> HashSet<ContributorRole> {
    let Some(config) = config.contributors.as_ref() else {
        return HashSet::new();
    };
    config
        .suppress
        .iter()
        .filter(|rule| declared_roles.contains(&rule.role))
        .filter_map(|rule| {
            let subject = identity_set(reference, &rule.role);
            let comparison = identity_set(reference, &rule.when_identical_to);
            (!subject.is_empty() && subject == comparison).then(|| rule.role.clone())
        })
        .collect()
}

/// Return whether a configured exact-set rule suppresses `role` for `reference`.
pub(super) fn is_role_suppressed(
    reference: &Reference,
    role: &ContributorRole,
    config: &citum_schema::options::Config,
) -> bool {
    let Some(config) = config.contributors.as_ref() else {
        return false;
    };
    config.suppress.iter().any(|rule| {
        if &rule.role != role {
            return false;
        }
        let subject = identity_set(reference, role);
        let comparison = identity_set(reference, &rule.when_identical_to);
        !subject.is_empty() && subject == comparison
    })
}

/// Resolve the effective merged names used by sorting and disambiguation.
pub(crate) fn semantic_names(
    component: &TemplateContributor,
    reference: &Reference,
    config: &citum_schema::options::Config,
    locale: &citum_schema::locale::Locale,
) -> Vec<FlatName> {
    let roles = component.contributor.as_slice();
    if roles.len() < 2 {
        return Vec::new();
    }
    let merge = effective_merge(component, config);
    let substitute = config.effective_substitute();
    let mut suppressed = suppressed_roles(reference, roles, config);
    suppressed.extend(substitute_suppressed_roles(
        reference,
        roles,
        substitute.as_ref(),
    ));
    let entries = assemble_names(reference, roles, &merge, &suppressed, config, locale, false);
    entries.into_iter().map(|entry| entry.name).collect()
}

/// Return the declared roles excluded from merged assembly because a
/// primary contributor role elsewhere consumes them via `role-substitute`
/// fallback (see `docs/specs/ROLE_SUBSTITUTE_FALLBACK.md`).
fn substitute_suppressed_roles(
    reference: &Reference,
    declared_roles: &[ContributorRole],
    substitute: &citum_schema::options::Substitute,
) -> HashSet<ContributorRole> {
    declared_roles
        .iter()
        .filter(|role| {
            super::substitute::is_role_suppressed_by_substitute(role, substitute, reference)
        })
        .cloned()
        .collect()
}

/// Resolve the effective merge configuration, borrowing the component-level
/// or style-wide block when present and only owning the default otherwise.
fn effective_merge<'a>(
    component: &'a TemplateContributor,
    config: &'a citum_schema::options::Config,
) -> std::borrow::Cow<'a, ContributorMerge> {
    config.contributors.as_ref().map_or_else(
        || {
            component.merge.as_ref().map_or_else(
                || std::borrow::Cow::Owned(ContributorMerge::default()),
                std::borrow::Cow::Borrowed,
            )
        },
        |contributors| contributors.effective_merge(component.merge.as_ref()),
    )
}

fn identity_set(reference: &Reference, role: &ContributorRole) -> HashSet<usize> {
    let Some(data_role) = contributor_role_to_reference_role(role) else {
        return HashSet::new();
    };
    reference
        .all_contributor_entries()
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| entry.roles.contains(&data_role).then_some(index))
        .collect()
}

struct DecorationContext<'a, 'options, F: OutputFormat<Output = String>> {
    component: &'a TemplateContributor,
    reference: &'a Reference,
    merge: &'a ContributorMerge,
    effective_rendering: &'a Rendering,
    options: &'a RenderOptions<'options>,
    fmt: &'a F,
}

fn build_decorations<F: OutputFormat<Output = String>>(
    entries: &[MergedName],
    selected_indices: &[usize],
    context: &DecorationContext<'_, '_, F>,
) -> Vec<NameDecoration> {
    let mut decorations = vec![NameDecoration::default(); entries.len()];
    let mut index = 0;
    while let Some(entry) = entries.get(index) {
        let mode = effective_label_mode(entry, context.merge);
        let run_end = if mode == ContributorLabelMode::Collective {
            entries
                .get(index + 1..)
                .unwrap_or_default()
                .iter()
                .position(|candidate| candidate.roles != entry.roles)
                .map_or(entries.len(), |offset| index + 1 + offset)
        } else {
            index + 1
        };
        let selected_run = selected_indices
            .iter()
            .copied()
            .filter(|selected| (index..run_end).contains(selected))
            .collect::<Vec<_>>();
        if mode != ContributorLabelMode::None && !selected_run.is_empty() {
            let plural = mode == ContributorLabelMode::Collective && run_end - index > 1;
            let (prefix, suffix) = resolve_entry_label::<F>(&entry.roles, plural, context);
            if let Some(decoration) = selected_run
                .first()
                .and_then(|first| decorations.get_mut(*first))
            {
                decoration.prefix = prefix.unwrap_or_default();
            }
            if let Some(decoration) = selected_run
                .last()
                .and_then(|last| decorations.get_mut(*last))
            {
                decoration.suffix = suffix.unwrap_or_default();
            }
        }
        index = run_end;
    }
    decorations
}

fn effective_label_mode(entry: &MergedName, merge: &ContributorMerge) -> ContributorLabelMode {
    entry
        .roles
        .first()
        .and_then(|role| merge.roles.get(role))
        .and_then(|role| role.labels)
        .unwrap_or(merge.labels)
}

#[derive(Debug, Clone)]
struct LabelPresentation {
    form: TermForm,
    placement: LabelPlacement,
    text_case: Option<citum_schema::options::titles::TextCase>,
    wrap: Option<citum_schema::template::WrapConfig>,
    before: String,
    after: String,
}

fn resolve_entry_label<F: OutputFormat<Output = String>>(
    roles: &[ContributorRole],
    plural: bool,
    context: &DecorationContext<'_, '_, F>,
) -> (Option<String>, Option<String>) {
    let Some(primary_role) = roles.first() else {
        return (None, None);
    };
    let explicit = context
        .merge
        .roles
        .get(primary_role)
        .and_then(|role| role.label.as_ref());

    if roles.len() == 1 {
        let mut single = context.component.clone();
        single.contributor = primary_role.clone().into();
        single.merge = None;
        single.label = explicit.cloned();
        return super::labels::resolve_role_labels::<F>(super::labels::RoleLabelContext {
            component: &single,
            role: primary_role,
            reference: context.reference,
            names_count: usize::from(plural) + 1,
            effective_rendering: context.effective_rendering,
            options: context.options,
            fmt: context.fmt,
            role_omitted: super::is_role_label_omitted(context.options, primary_role),
        });
    }

    let item_language = crate::values::effective_item_language(context.reference);
    let Some(presentation) = label_presentation(
        context.component,
        primary_role,
        explicit,
        context.options,
        item_language.as_deref(),
    ) else {
        return (None, None);
    };
    let term = resolve_combined_term(
        roles,
        plural,
        &presentation.form,
        context.merge,
        context.options,
    );
    let term = term.map(|term| {
        super::labels::apply_label_case(
            term,
            presentation.text_case,
            context.options.locale.locale.as_str(),
        )
    });
    place_term::<F>(
        term,
        &presentation,
        context.effective_rendering,
        context.options,
        context.fmt,
        item_language.as_deref(),
    )
}

fn structural_label_presentation(
    form: RoleLabelForm,
    placement: LabelPlacement,
    text_case: Option<citum_schema::options::titles::TextCase>,
    wrap: Option<&citum_schema::template::WrapConfig>,
    prefix: Option<&str>,
    suffix: Option<&str>,
) -> LabelPresentation {
    let (before, after) = match (&placement, wrap) {
        (LabelPlacement::Suffix, Some(_)) => (prefix.unwrap_or(" "), suffix.unwrap_or_default()),
        (LabelPlacement::Prefix, _) => (prefix.unwrap_or_default(), suffix.unwrap_or(" ")),
        (LabelPlacement::Suffix, None) => (prefix.unwrap_or(", "), suffix.unwrap_or_default()),
    };
    LabelPresentation {
        form: match form {
            RoleLabelForm::Short => TermForm::Short,
            RoleLabelForm::Long => TermForm::Long,
        },
        placement,
        text_case,
        wrap: wrap.cloned(),
        before: before.to_string(),
        after: after.to_string(),
    }
}

fn label_presentation(
    component: &TemplateContributor,
    role: &ContributorRole,
    explicit: Option<&RoleLabel>,
    options: &RenderOptions<'_>,
    item_language: Option<&str>,
) -> Option<LabelPresentation> {
    if let Some(label) = explicit {
        let (default_before, default_after) = match (&label.placement, &label.wrap) {
            (LabelPlacement::Suffix, Some(_)) => (" ", ""),
            (LabelPlacement::Prefix, _) => ("", " "),
            (LabelPlacement::Suffix, None) => (", ", ""),
        };
        let before = super::labels::realize_label_affix(
            label.prefix.as_ref(),
            default_before,
            options,
            item_language,
            crate::render::format::PunctuationPosition::Prefix,
        );
        let after = super::labels::realize_label_affix(
            label.suffix.as_ref(),
            default_after,
            options,
            item_language,
            crate::render::format::PunctuationPosition::Suffix,
        );
        return Some(structural_label_presentation(
            label.form.clone(),
            label.placement.clone(),
            label.text_case,
            label.wrap.as_deref(),
            Some(&before),
            Some(&after),
        ));
    }

    // `role.omit` suppresses configured style-wide structural labels (an
    // explicit `role_label_presentation` override or a `role.defaults`
    // preset bundle) below, mirroring the scalar path
    // (`labels::resolve_role_labels`), subject to the same verb-form
    // exception: a `form: verb`/`form: verb-short` component's label is
    // structural, not decorative, and is never omitted.
    let role_omitted = super::is_role_label_omitted(options, role);
    let verb_form = matches!(
        component.form,
        ContributorForm::Verb | ContributorForm::VerbShort
    );
    let role_omitted_hides_structural_label = role_omitted && !verb_form;

    if !role_omitted_hides_structural_label
        && options.context == RenderContext::Bibliography
        && let Some(label) = options
            .config
            .contributors
            .as_ref()
            .and_then(|contributors| contributors.role_label_presentation(role))
    {
        return Some(structural_label_presentation(
            label.form.clone(),
            label.placement.clone(),
            label.text_case,
            label.wrap.as_deref(),
            label.prefix.as_deref(),
            label.suffix.as_deref(),
        ));
    }

    if !role_omitted_hides_structural_label
        && options.context == RenderContext::Bibliography
        && let Some(label) = options
            .config
            .contributors
            .as_ref()
            .and_then(|contributors| contributors.role.as_ref())
            .and_then(|role_options| role_options.defaults)
            .and_then(|defaults| defaults.presentation_for(role))
    {
        return Some(structural_label_presentation(
            label.form,
            label.placement,
            label.text_case,
            label.wrap.as_deref(),
            label.prefix.as_deref(),
            label.suffix.as_deref(),
        ));
    }

    let preset = (options.context == RenderContext::Bibliography)
        .then(|| {
            options
                .config
                .contributors
                .as_ref()
                .and_then(|contributors| contributors.effective_role_label_preset(role))
        })
        .flatten()
        .or_else(|| {
            (options.context == RenderContext::Bibliography).then_some(())?;
            options
                .config
                .contributors
                .as_ref()?
                .default_role_label_preset(role)
        });
    if let Some(preset) = preset {
        return presentation_for_preset(preset, role_omitted, &component.form);
    }
    match component.form {
        ContributorForm::Verb => {
            presentation_for_preset(RoleLabelPreset::VerbPrefix, false, &component.form)
        }
        ContributorForm::VerbShort => {
            presentation_for_preset(RoleLabelPreset::VerbShortPrefix, false, &component.form)
        }
        _ => None,
    }
}

fn presentation_for_preset(
    preset: RoleLabelPreset,
    role_omitted: bool,
    form: &ContributorForm,
) -> Option<LabelPresentation> {
    if role_omitted && !matches!(form, ContributorForm::Verb | ContributorForm::VerbShort) {
        return None;
    }
    let (term_form, placement, before, after) = match preset {
        RoleLabelPreset::None => return None,
        RoleLabelPreset::VerbPrefix => (TermForm::Verb, LabelPlacement::Prefix, "", " "),
        RoleLabelPreset::VerbShortPrefix => (TermForm::VerbShort, LabelPlacement::Prefix, "", " "),
        RoleLabelPreset::ShortSuffix => (TermForm::Short, LabelPlacement::Suffix, " (", ")"),
        RoleLabelPreset::ShortSuffixComma => (TermForm::Short, LabelPlacement::Suffix, ", ", ""),
        RoleLabelPreset::LongSuffix => (TermForm::Long, LabelPlacement::Suffix, ", ", ""),
    };
    Some(LabelPresentation {
        form: term_form,
        placement,
        text_case: None,
        wrap: None,
        before: before.to_string(),
        after: after.to_string(),
    })
}

fn resolve_combined_term(
    roles: &[ContributorRole],
    plural: bool,
    form: &TermForm,
    merge: &ContributorMerge,
    options: &RenderOptions<'_>,
) -> Option<String> {
    options
        .locale
        .resolved_role_combination_term(roles, plural, form, None)
        .or_else(|| {
            let connector = merge
                .role_conjunction
                .as_deref()
                .unwrap_or_else(|| options.locale.role_conjunction());
            let terms = roles
                .iter()
                .map(|role| options.locale.resolved_role_term(role, plural, form, None))
                .collect::<Option<Vec<_>>>()?;
            Some(terms.join(connector))
        })
}

fn place_term<F: OutputFormat<Output = String>>(
    term: Option<String>,
    presentation: &LabelPresentation,
    effective_rendering: &Rendering,
    options: &RenderOptions<'_>,
    fmt: &F,
    item_language: Option<&str>,
) -> (Option<String>, Option<String>) {
    let placed = term.map(|term| {
        super::format_wrapped_role_term::<F>(
            &term,
            fmt,
            effective_rendering,
            options,
            (&presentation.before, &presentation.after),
            presentation.wrap.as_ref(),
            item_language,
        )
    });
    match presentation.placement {
        LabelPlacement::Prefix => (placed, None),
        LabelPlacement::Suffix => (None, placed),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Empty-list substitution shares the contributor rendering context."
)]
fn resolve_empty_list<F: OutputFormat<Output = String>>(
    component: &TemplateContributor,
    primary_role: &ContributorRole,
    reference: &Reference,
    hints: &ProcHints,
    options: &RenderOptions<'_>,
    effective_rendering: &Rendering,
    fmt: &F,
) -> Option<ProcValues<F::Output>> {
    let substitute = options.config.effective_substitute();
    let mut scalar = component.clone();
    scalar.contributor = primary_role.clone().into();
    scalar.merge = None;
    if matches!(primary_role, ContributorRole::Author) {
        super::substitute::resolve_author_substitute::<F>(
            &scalar,
            hints,
            options,
            reference,
            effective_rendering,
            fmt,
            substitute.as_ref(),
        )
    } else {
        super::substitute::resolve_role_substitute::<F>(
            primary_role,
            &scalar,
            hints,
            options,
            reference,
            effective_rendering,
            fmt,
            substitute.as_ref(),
        )
    }
}
