/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Pure predicates over `TemplateComponent` and `Reference` used by grouped
//! citation rendering and template-policy logic.

use crate::reference::Reference;
use citum_schema::{
    ResolvedLocalizedTemplate, TemplateVariants,
    reference::NumOrStr,
    template::{DateVariable, NumberVariable, SimpleVariable, Template, TemplateComponent},
};

/// Returns the first type-variant template whose selector matches `ref_type`,
/// or `None` if there are no variants or none match.
pub(in crate::processor::rendering) fn resolve_type_variant<'a>(
    type_variants: Option<
        &'a indexmap::IndexMap<citum_schema::template::TypeSelector, citum_schema::TemplateVariant>,
    >,
    ref_type: &str,
) -> Option<&'a [TemplateComponent]> {
    let selector_candidates = crate::values::type_class::type_selector_aliases(ref_type);
    type_variants?.iter().find_map(|(selector, variant)| {
        if selector_candidates
            .iter()
            .any(|candidate| selector.matches(candidate))
        {
            variant.as_template()
        } else {
            None
        }
    })
}

/// Resolve locale-owned variants before section-level variants, retaining reference aliases.
pub(in crate::processor::rendering) fn resolve_localized_type_variant(
    resolved: ResolvedLocalizedTemplate,
    type_variants: Option<&TemplateVariants>,
    ref_type: &str,
) -> Template {
    let selector_candidates = crate::values::type_class::type_selector_aliases(ref_type);
    resolved
        .type_variants
        .as_ref()
        .and_then(|variants| {
            variants.iter().find_map(|(selector, template)| {
                selector_candidates
                    .iter()
                    .any(|candidate| selector.matches(candidate))
                    .then(|| template.clone())
            })
        })
        .or_else(|| {
            resolve_type_variant(type_variants, ref_type).map(<[TemplateComponent]>::to_vec)
        })
        .unwrap_or(resolved.template)
}

pub(super) fn is_term_only_component(component: &TemplateComponent) -> bool {
    match component {
        TemplateComponent::Term(_) => true,
        TemplateComponent::Message(message) if message.message.starts_with("term.") => true,
        TemplateComponent::Group(group) => group.group.iter().all(is_term_only_component),
        _ => false,
    }
}

pub(super) fn is_primary_title_component(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Title(title)
            if title.title == citum_schema::template::TitleType::Primary
    )
}

pub(super) fn is_parent_container_title_component(component: &TemplateComponent) -> bool {
    component_or_message_arg_contains(component, is_parent_container_title_component_direct)
}

fn is_parent_container_title_component_direct(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Title(title)
            if matches!(
                title.title,
                citum_schema::template::TitleType::ContainerTitle
                    | citum_schema::template::TitleType::ParentSerial
                    | citum_schema::template::TitleType::ParentMonograph
                    | citum_schema::template::TitleType::CollectionTitle
            )
    )
}

pub(super) fn is_parent_monograph_title_component(component: &TemplateComponent) -> bool {
    component_or_message_arg_contains(component, is_parent_monograph_title_component_direct)
}

fn is_parent_monograph_title_component_direct(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Title(title)
            if title.title == citum_schema::template::TitleType::ParentMonograph
    )
}

fn component_or_message_arg_contains(
    component: &TemplateComponent,
    direct: fn(&TemplateComponent) -> bool,
) -> bool {
    if direct(component) {
        return true;
    }

    match component {
        TemplateComponent::Group(group) => group
            .group
            .iter()
            .any(|child| component_or_message_arg_contains(child, direct)),
        TemplateComponent::Message(message) => message
            .args
            .values()
            .filter_map(|source| source.as_template_component())
            .any(|child| component_or_message_arg_contains(&child, direct)),
        _ => false,
    }
}

pub(super) fn is_issued_date_component(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Date(date) if date.date == DateVariable::Issued
    )
}

pub(super) fn is_volume_component(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Number(number) if number.number == NumberVariable::Volume
    )
}

pub(super) fn is_url_component(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Variable(variable) if variable.variable == SimpleVariable::Url
    )
}

pub(super) fn is_doi_component(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Variable(variable) if variable.variable == SimpleVariable::Doi
    )
}

pub(super) fn is_article_detail_component(component: &TemplateComponent) -> bool {
    matches!(
        component,
        TemplateComponent::Date(date) if date.date == DateVariable::Issued
    ) || matches!(
        component,
        TemplateComponent::Number(number)
            if matches!(
                number.number,
                NumberVariable::Volume | NumberVariable::Issue | NumberVariable::Pages
            )
    )
}

pub(super) fn reference_has_pages(reference: &Reference) -> bool {
    match reference.pages() {
        Some(NumOrStr::Str(pages)) => !pages.trim().is_empty(),
        Some(NumOrStr::Number(_)) => true,
        None => false,
    }
}

pub(super) fn reference_has_doi(reference: &Reference) -> bool {
    reference.doi().is_some_and(|doi| !doi.trim().is_empty())
}

pub(super) fn reference_has_url(reference: &Reference) -> bool {
    reference.url().is_some()
}

pub(super) fn reference_has_online_access(reference: &Reference) -> bool {
    reference_has_doi(reference) || reference_has_url(reference)
}
