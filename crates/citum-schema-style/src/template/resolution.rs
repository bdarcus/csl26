/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Template variant resolution logic.

use std::collections::HashSet;

use crate::template::{
    Template, TemplateComponent, TemplateComponentSelector, TemplateVariant, TemplateVariantDiff,
    TemplateVariants, TypeSelector,
};
use crate::{BibliographySpec, CitationSpec, ResolutionError, Style};

pub(crate) struct StyleVariantContext {
    citation: Option<CitationVariantContext>,
    bibliography: Option<SectionVariantContext>,
}

#[derive(Clone, Default)]
struct SectionVariantContext {
    fallback: Option<Template>,
    owns_fallback: bool,
    type_variants: Option<TemplateVariants>,
}

#[derive(Clone, Default)]
pub(crate) struct CitationVariantContext {
    fallback: Option<Template>,
    owns_fallback: bool,
    type_variants: Option<TemplateVariants>,
    integral: Option<Box<CitationVariantContext>>,
    non_integral: Option<Box<CitationVariantContext>>,
    subsequent: Option<Box<CitationVariantContext>>,
    ibid: Option<Box<CitationVariantContext>>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum TemplateOverlayState {
    #[default]
    Absent,
    Null,
    Present,
}

#[derive(Clone, Default)]
struct FallbackOverlayContext {
    template: TemplateOverlayState,
    template_ref: bool,
}

#[derive(Clone, Default)]
struct CitationOverlayContext {
    fallback: FallbackOverlayContext,
    integral: Option<Box<CitationOverlayContext>>,
    non_integral: Option<Box<CitationOverlayContext>>,
    subsequent: Option<Box<CitationOverlayContext>>,
    ibid: Option<Box<CitationOverlayContext>>,
}

#[derive(Clone, Default)]
struct StyleOverlayContext {
    citation: Option<CitationOverlayContext>,
    bibliography: Option<FallbackOverlayContext>,
}

struct FallbackResolutionContext<'a> {
    referenced_template: Option<Template>,
    inherited_fallback: Option<&'a [TemplateComponent]>,
    inherited_owns_fallback: bool,
    outer_fallback: Option<&'a [TemplateComponent]>,
    overlay: Option<&'a FallbackOverlayContext>,
    location: &'a str,
}

pub(crate) fn inherited_variant_context(style: &Style) -> Option<StyleVariantContext> {
    let context = StyleVariantContext {
        citation: style
            .citation
            .as_ref()
            .map(|citation| citation_variant_context(citation, None)),
        bibliography: style
            .bibliography
            .as_ref()
            .map(bibliography_variant_context),
    };
    (context.citation.is_some() || context.bibliography.is_some()).then_some(context)
}

fn bibliography_variant_context(spec: &BibliographySpec) -> SectionVariantContext {
    SectionVariantContext {
        fallback: spec.resolve_template(),
        owns_fallback: spec.template.is_some() || spec.template_ref.is_some(),
        type_variants: spec.type_variants.clone(),
    }
}

fn citation_variant_context(
    spec: &CitationSpec,
    outer_fallback: Option<&[TemplateComponent]>,
) -> CitationVariantContext {
    let own_fallback = spec.resolve_template();
    let owns_fallback = spec.template.is_some() || spec.template_ref.is_some();
    let fallback = own_fallback.or_else(|| {
        if owns_fallback {
            None
        } else {
            outer_fallback.map(<[_]>::to_vec)
        }
    });
    CitationVariantContext {
        fallback: fallback.clone(),
        owns_fallback,
        type_variants: spec.type_variants.clone(),
        integral: spec
            .integral
            .as_deref()
            .map(|child| citation_variant_context(child, fallback.as_deref()))
            .map(Box::new),
        non_integral: spec
            .non_integral
            .as_deref()
            .map(|child| citation_variant_context(child, fallback.as_deref()))
            .map(Box::new),
        subsequent: spec
            .subsequent
            .as_deref()
            .map(|child| citation_variant_context(child, fallback.as_deref()))
            .map(Box::new),
        ibid: spec
            .ibid
            .as_deref()
            .map(|child| citation_variant_context(child, fallback.as_deref()))
            .map(Box::new),
    }
}

pub(crate) fn resolve_style_template_variants(
    style: &mut Style,
    inherited: Option<&StyleVariantContext>,
) -> Result<(), ResolutionError> {
    let overlay = style_overlay_context(style);
    resolve_style_template_variants_with_context(style, inherited, &overlay)
}

pub(crate) fn resolve_style_template_variants_with_overlay(
    style: &mut Style,
    inherited: Option<&StyleVariantContext>,
    overlay: &Style,
) -> Result<(), ResolutionError> {
    let overlay = style_overlay_context(overlay);
    resolve_style_template_variants_with_context(style, inherited, &overlay)
}

fn resolve_style_template_variants_with_context(
    style: &mut Style,
    inherited: Option<&StyleVariantContext>,
    overlay: &StyleOverlayContext,
) -> Result<(), ResolutionError> {
    let style_label = style
        .info
        .id
        .as_deref()
        .or(style.info.title.as_deref())
        .unwrap_or("<anonymous>")
        .to_string();
    if let Some(citation) = style.citation.as_mut() {
        resolve_citation_template_variants(
            citation,
            inherited.and_then(|context| context.citation.as_ref()),
            overlay.citation.as_ref(),
            &style_label,
            "citation",
            None,
        )?;
    }
    if let Some(bibliography) = style.bibliography.as_mut() {
        let inherited_bibliography = inherited.and_then(|context| context.bibliography.as_ref());
        let referenced_template = bibliography
            .template_ref
            .as_ref()
            .and_then(crate::template::TemplateReference::bibliography_template);
        let section_template = resolve_fallback_template_variant(
            &mut bibliography.template,
            &mut bibliography.template_ref,
            FallbackResolutionContext {
                referenced_template,
                inherited_fallback: inherited_bibliography
                    .and_then(|context| context.fallback.as_deref()),
                inherited_owns_fallback: inherited_bibliography
                    .is_some_and(|context| context.owns_fallback),
                outer_fallback: None,
                overlay: overlay.bibliography.as_ref(),
                location: &format!("{style_label}.bibliography.template"),
            },
        )?;
        resolve_template_variant_map(
            bibliography.type_variants.as_mut(),
            section_template.as_deref(),
            inherited_bibliography.and_then(|context| context.type_variants.as_ref()),
            "bibliography.type-variants",
        )?;
    }
    Ok(())
}

fn resolve_citation_template_variants(
    spec: &mut CitationSpec,
    inherited: Option<&CitationVariantContext>,
    overlay: Option<&CitationOverlayContext>,
    style_label: &str,
    location: &str,
    fallback_template: Option<&[TemplateComponent]>,
) -> Result<(), ResolutionError> {
    let referenced_template = spec
        .template_ref
        .as_ref()
        .and_then(crate::template::TemplateReference::citation_template);
    let section_template = resolve_fallback_template_variant(
        &mut spec.template,
        &mut spec.template_ref,
        FallbackResolutionContext {
            referenced_template,
            inherited_fallback: inherited.and_then(|context| context.fallback.as_deref()),
            inherited_owns_fallback: inherited.is_some_and(|context| context.owns_fallback),
            outer_fallback: fallback_template,
            overlay: overlay.map(|context| &context.fallback),
            location: &format!("{style_label}.{location}.template"),
        },
    )?;
    let effective_section_template = section_template.as_deref();
    resolve_template_variant_map(
        spec.type_variants.as_mut(),
        effective_section_template,
        inherited.and_then(|context| context.type_variants.as_ref()),
        &format!("{location}.type-variants"),
    )?;

    for (name, child, inherited_child, overlay_child) in [
        (
            "integral",
            spec.integral.as_deref_mut(),
            inherited.and_then(|context| context.integral.as_deref()),
            overlay.and_then(|context| context.integral.as_deref()),
        ),
        (
            "non-integral",
            spec.non_integral.as_deref_mut(),
            inherited.and_then(|context| context.non_integral.as_deref()),
            overlay.and_then(|context| context.non_integral.as_deref()),
        ),
        (
            "subsequent",
            spec.subsequent.as_deref_mut(),
            inherited.and_then(|context| context.subsequent.as_deref()),
            overlay.and_then(|context| context.subsequent.as_deref()),
        ),
        (
            "ibid",
            spec.ibid.as_deref_mut(),
            inherited.and_then(|context| context.ibid.as_deref()),
            overlay.and_then(|context| context.ibid.as_deref()),
        ),
    ] {
        if let Some(child) = child {
            resolve_citation_template_variants(
                child,
                inherited_child,
                overlay_child,
                style_label,
                &format!("{location}.{name}"),
                effective_section_template,
            )?;
        }
    }
    Ok(())
}

fn resolve_fallback_template_variant(
    template: &mut Option<TemplateVariant>,
    template_ref: &mut Option<crate::template::TemplateReference>,
    context: FallbackResolutionContext<'_>,
) -> Result<Option<Template>, ResolutionError> {
    let overlay_state = context
        .overlay
        .map_or(TemplateOverlayState::Absent, |overlay| overlay.template);
    let overlay_template_ref = context.overlay.is_some_and(|overlay| overlay.template_ref);

    if overlay_state == TemplateOverlayState::Null {
        *template = None;
        if !overlay_template_ref {
            *template_ref = None;
            return Ok(context.outer_fallback.map(<[_]>::to_vec));
        }
        return Ok(context.referenced_template);
    }

    if overlay_state == TemplateOverlayState::Absent && overlay_template_ref {
        *template = None;
        return Ok(context.referenced_template);
    }

    match template {
        Some(TemplateVariant::Full(template)) => Ok(Some(template.clone())),
        Some(TemplateVariant::Diff(diff)) => {
            if overlay_template_ref {
                return Err(invalid_fallback_diff(
                    context.location,
                    "template-ref and a template diff cannot be declared in the same section",
                ));
            }
            if diff.extends.is_some() {
                return Err(invalid_fallback_diff(
                    context.location,
                    "extends is not allowed because a fallback diff has exactly one inherited base",
                ));
            }

            let mut resolved = if context.inherited_owns_fallback {
                context.inherited_fallback.map(<[_]>::to_vec)
            } else {
                context
                    .outer_fallback
                    .map(<[_]>::to_vec)
                    .or_else(|| context.inherited_fallback.map(<[_]>::to_vec))
            }
            .ok_or_else(|| {
                invalid_fallback_diff(
                    context.location,
                    "no inherited fallback template is available",
                )
            })?;
            apply_template_variant_diff(&mut resolved, diff, context.location)?;
            *template = Some(TemplateVariant::Full(resolved.clone()));
            Ok(Some(resolved))
        }
        None => Ok(context
            .referenced_template
            .or_else(|| context.outer_fallback.map(<[_]>::to_vec))),
    }
}

fn invalid_fallback_diff(location: &str, reason: &str) -> ResolutionError {
    ResolutionError::InvalidFallbackTemplateDiff {
        location: location.to_string(),
        reason: reason.to_string(),
    }
}

fn style_overlay_context(style: &Style) -> StyleOverlayContext {
    let raw = style.raw_yaml.as_ref();
    StyleOverlayContext {
        citation: style.citation.as_ref().map(|citation| {
            citation_overlay_context(citation, raw.and_then(|raw| raw_child(raw, "citation")))
        }),
        bibliography: style.bibliography.as_ref().map(|bibliography| {
            fallback_overlay_context(
                bibliography.template.is_some(),
                bibliography.template_ref.is_some(),
                raw.and_then(|raw| raw_child(raw, "bibliography")),
            )
        }),
    }
}

fn citation_overlay_context(
    spec: &CitationSpec,
    raw: Option<&serde_yaml::Value>,
) -> CitationOverlayContext {
    CitationOverlayContext {
        fallback: fallback_overlay_context(
            spec.template.is_some(),
            spec.template_ref.is_some(),
            raw,
        ),
        integral: spec
            .integral
            .as_deref()
            .map(|child| {
                citation_overlay_context(child, raw.and_then(|raw| raw_child(raw, "integral")))
            })
            .map(Box::new),
        non_integral: spec
            .non_integral
            .as_deref()
            .map(|child| {
                citation_overlay_context(child, raw.and_then(|raw| raw_child(raw, "non-integral")))
            })
            .map(Box::new),
        subsequent: spec
            .subsequent
            .as_deref()
            .map(|child| {
                citation_overlay_context(child, raw.and_then(|raw| raw_child(raw, "subsequent")))
            })
            .map(Box::new),
        ibid: spec
            .ibid
            .as_deref()
            .map(|child| {
                citation_overlay_context(child, raw.and_then(|raw| raw_child(raw, "ibid")))
            })
            .map(Box::new),
    }
}

fn fallback_overlay_context(
    has_template: bool,
    has_template_ref: bool,
    raw: Option<&serde_yaml::Value>,
) -> FallbackOverlayContext {
    let Some(mapping) = raw.and_then(serde_yaml::Value::as_mapping) else {
        return FallbackOverlayContext {
            template: if has_template {
                TemplateOverlayState::Present
            } else {
                TemplateOverlayState::Absent
            },
            template_ref: has_template_ref,
        };
    };
    let template_key = serde_yaml::Value::String("template".to_string());
    let template_ref_key = serde_yaml::Value::String("template-ref".to_string());
    FallbackOverlayContext {
        template: match mapping.get(&template_key) {
            Some(value) if value.is_null() => TemplateOverlayState::Null,
            Some(_) => TemplateOverlayState::Present,
            None => TemplateOverlayState::Absent,
        },
        template_ref: mapping
            .get(&template_ref_key)
            .is_some_and(|value| !value.is_null()),
    }
}

fn raw_child<'a>(value: &'a serde_yaml::Value, key: &str) -> Option<&'a serde_yaml::Value> {
    value
        .as_mapping()?
        .get(serde_yaml::Value::String(key.to_string()))
}

pub(crate) fn resolve_template_variant_map(
    variants: Option<&mut TemplateVariants>,
    section_template: Option<&[TemplateComponent]>,
    inherited: Option<&TemplateVariants>,
    location: &str,
) -> Result<(), ResolutionError> {
    let Some(variants) = variants else {
        return Ok(());
    };
    let original = variants.clone();
    let mut resolved = TemplateVariants::new();
    let mut visiting = HashSet::new();

    for selector in original.keys() {
        let template = resolve_template_variant(
            selector,
            &original,
            &mut resolved,
            inherited,
            section_template,
            location,
            &mut visiting,
        )?;
        resolved.insert(selector.clone(), TemplateVariant::Full(template));
    }

    *variants = resolved;
    Ok(())
}

pub(crate) fn resolve_template_variant(
    selector: &TypeSelector,
    original: &TemplateVariants,
    resolved: &mut TemplateVariants,
    inherited: Option<&TemplateVariants>,
    section_template: Option<&[TemplateComponent]>,
    location: &str,
    visiting: &mut HashSet<TypeSelector>,
) -> Result<Template, ResolutionError> {
    let variant_location = format!("{location}[{selector}]");
    if let Some(template) = resolved
        .get(selector)
        .and_then(TemplateVariant::as_template)
        .map(<[TemplateComponent]>::to_vec)
    {
        return Ok(template);
    }

    if !visiting.insert(selector.clone()) {
        return Err(ResolutionError::TemplateVariantCycle {
            location: variant_location,
            selector: selector.to_string(),
        });
    }

    let variant =
        original
            .get(selector)
            .ok_or_else(|| ResolutionError::MissingTemplateVariantParent {
                location: variant_location.clone(),
                selector: selector.to_string(),
            })?;

    let template = match variant {
        TemplateVariant::Full(template) => template.clone(),
        TemplateVariant::Diff(diff) => {
            let mut parent = resolve_variant_parent_template(
                selector,
                diff,
                original,
                resolved,
                inherited,
                section_template,
                &variant_location,
                visiting,
            )?;
            apply_template_variant_diff(&mut parent, diff, &variant_location)?;
            parent
        }
    };

    visiting.remove(selector);
    Ok(template)
}

#[allow(
    clippy::too_many_arguments,
    reason = "Template variant resolution needs explicit inherited and local context."
)]
pub(crate) fn resolve_variant_parent_template(
    selector: &TypeSelector,
    diff: &TemplateVariantDiff,
    original: &TemplateVariants,
    resolved: &mut TemplateVariants,
    inherited: Option<&TemplateVariants>,
    section_template: Option<&[TemplateComponent]>,
    location: &str,
    visiting: &mut HashSet<TypeSelector>,
) -> Result<Template, ResolutionError> {
    if let Some(parent_selector) = &diff.extends {
        if parent_selector != selector && original.contains_key(parent_selector) {
            return resolve_template_variant(
                parent_selector,
                original,
                resolved,
                inherited,
                section_template,
                location,
                visiting,
            );
        }
        return inherited
            .and_then(|variants| variants.get(parent_selector))
            .and_then(TemplateVariant::as_template)
            .map(<[TemplateComponent]>::to_vec)
            .ok_or_else(|| ResolutionError::MissingTemplateVariantParent {
                location: location.to_string(),
                selector: parent_selector.to_string(),
            });
    }

    inherited
        .and_then(|variants| variants.get(selector))
        .and_then(TemplateVariant::as_template)
        .map(<[TemplateComponent]>::to_vec)
        .or_else(|| section_template.map(<[TemplateComponent]>::to_vec))
        .ok_or_else(|| ResolutionError::MissingTemplateVariantParent {
            location: location.to_string(),
            selector: selector.to_string(),
        })
}

pub(crate) fn apply_template_variant_diff(
    template: &mut Template,
    diff: &TemplateVariantDiff,
    location: &str,
) -> Result<(), ResolutionError> {
    for op in &diff.modify {
        let index = find_required_anchor(template, &op.match_selector, location)?;
        if let Some(component) = template.get_mut(index) {
            if let Some(label_form) = op.label_form.clone()
                && let TemplateComponent::Number(number) = component
            {
                number.label_form = Some(label_form);
            }
            component.rendering_mut().merge(&op.rendering);
        }
    }
    for op in &diff.remove {
        let index = find_required_anchor(template, &op.match_selector, location)?;
        template.remove(index);
    }
    for op in &diff.add {
        let anchor = match (&op.before, &op.after) {
            (Some(selector), None) => Some((selector, false)),
            (None, Some(selector)) => Some((selector, true)),
            _ => {
                return Err(ResolutionError::InvalidTemplateVariantAdd {
                    location: location.to_string(),
                });
            }
        };
        let Some((selector, insert_after)) = anchor else {
            return Err(ResolutionError::InvalidTemplateVariantAdd {
                location: location.to_string(),
            });
        };
        let anchor_index = find_required_anchor(template, selector, location)?;
        let insert_at = if insert_after {
            anchor_index.saturating_add(1)
        } else {
            anchor_index
        };
        template.insert(insert_at, op.component.clone());
    }
    Ok(())
}

pub(crate) fn find_required_anchor(
    template: &[TemplateComponent],
    selector: &TemplateComponentSelector,
    location: &str,
) -> Result<usize, ResolutionError> {
    find_optional_anchor(template, selector, location)?.ok_or_else(|| {
        ResolutionError::TemplateVariantAnchorNotFound {
            location: location.to_string(),
        }
    })
}

pub(crate) fn find_optional_anchor(
    template: &[TemplateComponent],
    selector: &TemplateComponentSelector,
    location: &str,
) -> Result<Option<usize>, ResolutionError> {
    if selector.is_empty() {
        return Err(ResolutionError::TemplateVariantAmbiguousAnchor {
            location: location.to_string(),
        });
    }
    let mut matches = template
        .iter()
        .enumerate()
        .filter_map(|(index, component)| selector.matches(component).then_some(index));
    let first = matches.next();
    if matches.next().is_some() {
        return Err(ResolutionError::TemplateVariantAmbiguousAnchor {
            location: location.to_string(),
        });
    }
    Ok(first)
}
