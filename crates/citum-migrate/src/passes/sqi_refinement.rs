/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Post-assembly SQI refinement for migrated styles.

use crate::{
    passes::suppression::normalize_visible_suppress,
    template_diff::{TypeTemplateMap, build_type_variants},
};
use citum_schema::{
    BibliographySpec, CitationSpec, Style, TemplateVariant,
    options::{
        AndOptions, ContributorConfig, DisplayAsSort, TitleCategory, container_title_category,
        parent_serial_title_category, title_category,
    },
    template::{
        NameOrder, Rendering, TemplateComponent, TemplateContributor, TemplateTitle, TitleType,
        TypeSelector,
    },
};

/// Refine a fully assembled style for compact, maintainable serialization.
#[must_use]
pub fn refine_style(mut style: Style) -> Style {
    let candidates = HoistCandidates::from_style(&style);
    encode_bibliography_type_variants(&mut style);
    hoist_common_contributor_and(&mut style, candidates);
    prune_style_against_options(&mut style);
    normalize_serialized_suppress_markers(&mut style);
    style
}

#[derive(Debug, Clone, Default)]
struct HoistCandidates {
    citation_and: CommonAnd,
    bibliography_and: CommonAnd,
}

impl HoistCandidates {
    fn from_style(style: &Style) -> Self {
        let mut citation_templates = Vec::new();
        collect_citation_templates(style.citation.as_ref(), &mut citation_templates);

        let mut bibliography_templates = Vec::new();
        collect_bibliography_templates(style.bibliography.as_ref(), &mut bibliography_templates);

        Self {
            citation_and: common_contributor_and_in_templates(citation_templates),
            bibliography_and: common_contributor_and_in_templates(bibliography_templates),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
enum CommonAnd {
    #[default]
    NoContributors,
    Value(AndOptions),
    Ineligible,
}

impl CommonAnd {
    fn value(&self) -> Option<&AndOptions> {
        match self {
            Self::Value(value) => Some(value),
            Self::NoContributors | Self::Ineligible => None,
        }
    }
}

fn collect_citation_templates<'a>(
    spec: Option<&'a CitationSpec>,
    templates: &mut Vec<&'a [TemplateComponent]>,
) {
    let Some(spec) = spec else {
        return;
    };
    if let Some(template) = spec.template.as_ref() {
        templates.push(template);
    }
    if let Some(type_variants) = spec.type_variants.as_ref() {
        collect_full_variant_templates(type_variants, templates);
    }
    collect_citation_templates(spec.integral.as_deref(), templates);
    collect_citation_templates(spec.non_integral.as_deref(), templates);
    collect_citation_templates(spec.subsequent.as_deref(), templates);
    collect_citation_templates(spec.ibid.as_deref(), templates);
}

fn collect_bibliography_templates<'a>(
    spec: Option<&'a BibliographySpec>,
    templates: &mut Vec<&'a [TemplateComponent]>,
) {
    let Some(spec) = spec else {
        return;
    };
    if let Some(template) = spec.template.as_ref() {
        templates.push(template);
    }
    if let Some(type_variants) = spec.type_variants.as_ref() {
        collect_full_variant_templates(type_variants, templates);
    }
}

fn collect_full_variant_templates<'a>(
    type_variants: &'a indexmap::IndexMap<TypeSelector, TemplateVariant>,
    templates: &mut Vec<&'a [TemplateComponent]>,
) {
    for variant in type_variants.values() {
        if let TemplateVariant::Full(template) = variant {
            templates.push(template);
        }
    }
}

fn common_contributor_and_in_templates<'a>(
    templates: impl IntoIterator<Item = &'a [TemplateComponent]>,
) -> CommonAnd {
    let mut common = None;
    let mut saw_contributor = false;

    for template in templates {
        let mut contributors = Vec::new();
        collect_contributors(template, &mut contributors);
        for contributor in contributors {
            saw_contributor = true;
            let Some(and) = contributor.and.clone() else {
                return CommonAnd::Ineligible;
            };
            match common {
                None => common = Some(and),
                Some(ref current) if current == &and => {}
                Some(_) => return CommonAnd::Ineligible,
            }
        }
    }

    match (saw_contributor, common) {
        (true, Some(and)) => CommonAnd::Value(and),
        _ => CommonAnd::NoContributors,
    }
}

fn collect_contributors<'a>(
    template: &'a [TemplateComponent],
    contributors: &mut Vec<&'a TemplateContributor>,
) {
    for component in template {
        match component {
            TemplateComponent::Contributor(contributor) => contributors.push(contributor),
            TemplateComponent::Group(group) => collect_contributors(&group.group, contributors),
            _ => {}
        }
    }
}

fn encode_bibliography_type_variants(style: &mut Style) {
    let Some(bibliography) = style.bibliography.as_mut() else {
        return;
    };
    let Some(base_template) = bibliography.template.as_ref() else {
        return;
    };
    let Some(type_variants) = bibliography.type_variants.as_ref() else {
        return;
    };
    let all_variants_full = type_variants
        .values()
        .all(|variant| matches!(variant, TemplateVariant::Full(_)));
    if !all_variants_full {
        debug_assert!(
            all_variants_full,
            "SQI refinement expects post-assembly bibliography variants to enter as Full templates"
        );
        return;
    }

    let Some(type_variants) = bibliography.type_variants.take() else {
        return;
    };
    let mut type_templates = TypeTemplateMap::new();
    for (selector, variant) in type_variants {
        let TemplateVariant::Full(template) = variant else {
            continue;
        };
        if template.as_slice() != base_template.as_slice() {
            type_templates.insert(selector, template);
        }
    }

    bibliography.type_variants =
        (!type_templates.is_empty()).then(|| build_type_variants(base_template, type_templates));
}

fn hoist_common_contributor_and(style: &mut Style, candidates: HoistCandidates) {
    let global_and = style
        .options
        .as_ref()
        .and_then(|options| options.contributors.as_ref())
        .and_then(|contributors| contributors.and.as_ref());

    if global_and.is_none()
        && let (Some(citation), Some(bibliography)) = (
            candidates.citation_and.value(),
            candidates.bibliography_and.value(),
        )
        && citation == bibliography
    {
        set_global_contributor_and(style, citation.clone());
        return;
    }

    if let Some(and) = candidates.citation_and.value()
        && Some(and) != global_and
        && let Some(citation) = style.citation.as_mut()
    {
        set_citation_contributor_and(citation, and.clone());
    }

    if let Some(and) = candidates.bibliography_and.value()
        && Some(and) != global_and
        && let Some(bibliography) = style.bibliography.as_mut()
    {
        set_bibliography_contributor_and(bibliography, and.clone());
    }
}

fn set_global_contributor_and(style: &mut Style, and: AndOptions) {
    style
        .options
        .get_or_insert_with(Default::default)
        .contributors
        .get_or_insert_with(Default::default)
        .and = Some(and);
}

fn set_citation_contributor_and(citation: &mut CitationSpec, and: AndOptions) {
    citation
        .options
        .get_or_insert_with(Default::default)
        .contributors
        .get_or_insert_with(Default::default)
        .and = Some(and);
}

fn set_bibliography_contributor_and(bibliography: &mut BibliographySpec, and: AndOptions) {
    bibliography
        .options
        .get_or_insert_with(Default::default)
        .contributors
        .get_or_insert_with(Default::default)
        .and = Some(and);
}

fn prune_style_against_options(style: &mut Style) {
    let global_options = style.options.clone().unwrap_or_default();
    if let Some(citation) = style.citation.as_mut() {
        prune_citation_spec(citation, &global_options);
    }
    if let Some(bibliography) = style.bibliography.as_mut() {
        let effective_options = bibliography.options.as_ref().map_or_else(
            || global_options.clone(),
            |options| options.merged_with(&global_options),
        );
        if let Some(template) = bibliography.template.as_mut() {
            normalize_templates_against_options(
                template,
                TitleDefaultContext::Default,
                &effective_options,
            );
        }
        prune_type_variants(bibliography.type_variants.as_mut(), &effective_options);
    }
}

fn prune_citation_spec(spec: &mut CitationSpec, inherited_options: &citum_schema::options::Config) {
    let effective_options = spec.options.as_ref().map_or_else(
        || inherited_options.clone(),
        |options| options.merged_with(inherited_options),
    );
    if let Some(template) = spec.template.as_mut() {
        normalize_templates_against_options(
            template,
            TitleDefaultContext::Default,
            &effective_options,
        );
    }
    prune_type_variants(spec.type_variants.as_mut(), &effective_options);
    if let Some(integral) = spec.integral.as_mut() {
        prune_citation_spec(integral, &effective_options);
    }
    if let Some(non_integral) = spec.non_integral.as_mut() {
        prune_citation_spec(non_integral, &effective_options);
    }
    if let Some(subsequent) = spec.subsequent.as_mut() {
        prune_citation_spec(subsequent, &effective_options);
    }
    if let Some(ibid) = spec.ibid.as_mut() {
        prune_citation_spec(ibid, &effective_options);
    }
}

fn prune_type_variants(
    type_variants: Option<&mut indexmap::IndexMap<TypeSelector, TemplateVariant>>,
    options: &citum_schema::options::Config,
) {
    let Some(type_variants) = type_variants else {
        return;
    };
    for (selector, variant) in type_variants {
        let title_context = selector_title_context(selector);
        match variant {
            TemplateVariant::Full(template) => {
                normalize_templates_against_options(template, title_context, options);
            }
            TemplateVariant::Diff(diff) => {
                for add in &mut diff.add {
                    normalize_component_against_options(&mut add.component, title_context, options);
                }
                // `modify.rendering` payloads are deliberately left explicit:
                // a `suppress: false` modify can un-suppress a component that is
                // suppressed in the base template, so pruning needs base-aware
                // semantics rather than the component-local checks used here.
            }
        }
    }
}

#[derive(Clone, Copy)]
enum TitleDefaultContext<'a> {
    Default,
    RefType(&'a str),
    Unknown,
}

fn selector_title_context(selector: &TypeSelector) -> TitleDefaultContext<'_> {
    match selector {
        TypeSelector::Single(ref_type) => TitleDefaultContext::RefType(ref_type),
        TypeSelector::Multiple(_) => TitleDefaultContext::Unknown,
    }
}

fn normalize_templates_against_options(
    template: &mut [TemplateComponent],
    title_context: TitleDefaultContext<'_>,
    options: &citum_schema::options::Config,
) {
    for component in template {
        normalize_component_against_options(component, title_context, options);
    }
}

fn normalize_component_against_options(
    component: &mut TemplateComponent,
    title_context: TitleDefaultContext<'_>,
    options: &citum_schema::options::Config,
) {
    match component {
        TemplateComponent::Contributor(contributor) => {
            if let Some(defaults) = options.contributors.as_ref() {
                compact_inherited_contributor(contributor, defaults);
            }
        }
        TemplateComponent::Title(title) => {
            compact_inherited_title(title, title_context, options);
        }
        TemplateComponent::Group(group) => {
            normalize_templates_against_options(&mut group.group, title_context, options);
        }
        _ => {}
    }
}

fn compact_inherited_title(
    title: &mut TemplateTitle,
    title_context: TitleDefaultContext<'_>,
    options: &citum_schema::options::Config,
) {
    let Some(defaults) = effective_title_rendering(&title.title, title_context, options) else {
        return;
    };

    clear_rendering_field(&mut title.rendering.text_case, defaults.text_case);
    clear_rendering_field(&mut title.rendering.emph, defaults.emph);
    clear_rendering_field(&mut title.rendering.quote, defaults.quote);
    clear_rendering_field(&mut title.rendering.strong, defaults.strong);
    clear_rendering_field(&mut title.rendering.small_caps, defaults.small_caps);
    clear_rendering_field(&mut title.rendering.prefix, defaults.prefix);
    clear_rendering_field(&mut title.rendering.suffix, defaults.suffix);
}

fn clear_rendering_field<T: PartialEq>(field: &mut Option<T>, inherited: Option<T>) {
    if field.as_ref() == inherited.as_ref() {
        *field = None;
    }
}

fn effective_title_rendering(
    title_type: &TitleType,
    title_context: TitleDefaultContext<'_>,
    options: &citum_schema::options::Config,
) -> Option<Rendering> {
    let ref_type = match title_context {
        TitleDefaultContext::Default => None,
        TitleDefaultContext::RefType(ref_type) => Some(ref_type),
        TitleDefaultContext::Unknown => return None,
    };
    let titles = options.titles.as_ref()?;
    let mapped_category = ref_type.and_then(|rt| {
        titles
            .type_mapping
            .as_ref()
            .and_then(|mapping| mapping.get(rt))
    });

    let rendering = match title_type {
        TitleType::ContainerTitle => {
            if let Some(category) = mapped_category {
                match category.as_str() {
                    "periodical" => titles.periodical.as_ref(),
                    "serial" => titles.serial.as_ref(),
                    "monograph" | "collection" => titles
                        .container_monograph
                        .as_ref()
                        .or(titles.monograph.as_ref()),
                    _ => titles.default.as_ref(),
                }
            } else if let Some(ref_type) = ref_type {
                match container_title_category(ref_type) {
                    TitleCategory::Periodical => titles.periodical.as_ref(),
                    TitleCategory::ContainerMonograph => titles
                        .container_monograph
                        .as_ref()
                        .or(titles.monograph.as_ref()),
                    _ => titles.default.as_ref(),
                }
            } else {
                titles.default.as_ref()
            }
        }
        TitleType::ParentSerial => {
            if let Some(category) = mapped_category {
                match category.as_str() {
                    "periodical" => titles.periodical.as_ref(),
                    "serial" => titles.serial.as_ref(),
                    _ => titles.periodical.as_ref(),
                }
            } else if let Some(ref_type) = ref_type {
                match parent_serial_title_category(ref_type) {
                    TitleCategory::Periodical => titles.periodical.as_ref(),
                    _ => titles.serial.as_ref(),
                }
            } else {
                titles.periodical.as_ref()
            }
        }
        TitleType::ParentMonograph => titles
            .container_monograph
            .as_ref()
            .or(titles.monograph.as_ref()),
        TitleType::CollectionTitle => titles
            .container_monograph
            .as_ref()
            .or(titles.monograph.as_ref())
            .or(titles.default.as_ref()),
        TitleType::Primary => {
            if let Some(category) = mapped_category {
                match category.as_str() {
                    "component" => titles.component.as_ref(),
                    "monograph" => titles.monograph.as_ref(),
                    _ => titles.default.as_ref(),
                }
            } else if let Some(ref_type) = ref_type {
                match title_category(ref_type) {
                    TitleCategory::Component => titles.component.as_ref(),
                    TitleCategory::Monograph => titles.monograph.as_ref(),
                    _ => titles.default.as_ref(),
                }
            } else {
                titles.default.as_ref()
            }
        }
        _ => None,
    };

    Some(rendering.or(titles.default.as_ref())?.to_rendering())
}

fn compact_inherited_contributor(
    contributor: &mut TemplateContributor,
    defaults: &ContributorConfig,
) {
    if contributor.and.as_ref() == defaults.and.as_ref() {
        contributor.and = None;
    }
    if contributor.shorten.as_ref() == defaults.shorten.as_ref() {
        contributor.shorten = None;
    }
    if contributor.sort_separator.as_ref() == defaults.sort_separator.as_ref() {
        contributor.sort_separator = None;
    }
    if contributor.name_form.as_ref() == defaults.name_form.as_ref() {
        contributor.name_form = None;
    }
    if contributor
        .name_order
        .as_ref()
        .is_some_and(|name_order| inherited_name_order_matches(contributor, defaults, name_order))
    {
        contributor.name_order = None;
    }
}

fn inherited_name_order_matches(
    contributor: &TemplateContributor,
    defaults: &ContributorConfig,
    name_order: &NameOrder,
) -> bool {
    // Scalar components compact when their single role's effective default
    // matches; list-form components (e.g. merged [editor, translator]) only
    // compact when EVERY declared role's effective default agrees, since a
    // single explicit override can't be pushed onto a role whose own
    // default differs.
    let roles = contributor.contributor.as_slice();
    if !roles.is_empty()
        && roles
            .iter()
            .all(|role| defaults.effective_role_name_order(role) == Some(name_order))
    {
        return true;
    }

    matches!(
        (defaults.display_as_sort.as_ref(), name_order),
        (
            Some(DisplayAsSort::All | DisplayAsSort::First),
            NameOrder::FamilyFirst
        ) | (Some(DisplayAsSort::First), NameOrder::FamilyFirstOnly)
    )
}

fn normalize_serialized_suppress_markers(style: &mut Style) {
    if let Some(citation) = style.citation.as_mut() {
        normalize_citation_suppress(citation);
    }
    if let Some(bibliography) = style.bibliography.as_mut() {
        if let Some(template) = bibliography.template.as_mut() {
            normalize_visible_suppress(template);
        }
        normalize_variant_suppress(bibliography.type_variants.as_mut());
    }
}

fn normalize_citation_suppress(spec: &mut CitationSpec) {
    if let Some(template) = spec.template.as_mut() {
        normalize_visible_suppress(template);
    }
    normalize_variant_suppress(spec.type_variants.as_mut());
    if let Some(integral) = spec.integral.as_mut() {
        normalize_citation_suppress(integral);
    }
    if let Some(non_integral) = spec.non_integral.as_mut() {
        normalize_citation_suppress(non_integral);
    }
    if let Some(subsequent) = spec.subsequent.as_mut() {
        normalize_citation_suppress(subsequent);
    }
    if let Some(ibid) = spec.ibid.as_mut() {
        normalize_citation_suppress(ibid);
    }
}

fn normalize_variant_suppress(
    type_variants: Option<&mut indexmap::IndexMap<TypeSelector, TemplateVariant>>,
) {
    let Some(type_variants) = type_variants else {
        return;
    };
    for variant in type_variants.values_mut() {
        match variant {
            TemplateVariant::Full(template) => normalize_visible_suppress(template),
            TemplateVariant::Diff(diff) => {
                for add in &mut diff.add {
                    normalize_component_suppress(&mut add.component);
                }
                // Do not normalize `modify.rendering.suppress`: `false` can be
                // the meaningful diff that re-enables a base-suppressed anchor.
            }
        }
    }
}

fn normalize_component_suppress(component: &mut TemplateComponent) {
    let rendering = component.rendering_mut();
    if rendering.suppress == Some(false) {
        rendering.suppress = None;
    }
    if let TemplateComponent::Group(group) = component {
        for child in &mut group.group {
            normalize_component_suppress(child);
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use citum_engine::{Processor, reference::Bibliography};
    use citum_schema::{
        options::{
            NameForm, RoleOptions, RoleRendering, TextCase, TitleRendering, TitlesConfig,
            classified_ref_types,
        },
        template::{
            ContributorForm, ContributorRole, TemplateGroup, TemplateTitle, TemplateVariable,
        },
    };
    use rstest::rstest;

    fn author(and: Option<AndOptions>) -> TemplateComponent {
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author.into(),
            form: ContributorForm::Long,
            and,
            ..TemplateContributor::default()
        })
    }

    fn author_with_name_options() -> TemplateComponent {
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author.into(),
            form: ContributorForm::Long,
            name_order: Some(NameOrder::FamilyFirstOnly),
            name_form: Some(NameForm::Initials),
            and: Some(AndOptions::Text),
            ..TemplateContributor::default()
        })
    }

    fn primary_title(text_case: Option<TextCase>) -> TemplateComponent {
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            rendering: Rendering {
                text_case,
                ..Rendering::default()
            },
            ..TemplateTitle::default()
        })
    }

    fn container_title(text_case: Option<TextCase>) -> TemplateComponent {
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ContainerTitle,
            rendering: Rendering {
                text_case,
                ..Rendering::default()
            },
            ..TemplateTitle::default()
        })
    }

    fn url() -> TemplateComponent {
        TemplateComponent::Variable(TemplateVariable {
            variable: citum_schema::template::SimpleVariable::Url,
            ..TemplateVariable::default()
        })
    }

    fn style_options() -> citum_schema::options::Config {
        citum_schema::options::Config {
            contributors: Some(ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Text),
                name_form: Some(NameForm::Initials),
                ..ContributorConfig::default()
            }),
            titles: Some(TitlesConfig {
                default: Some(TitleRendering {
                    text_case: Some(TextCase::SentenceApa),
                    ..TitleRendering::default()
                }),
                ..TitlesConfig::default()
            }),
            ..citum_schema::options::Config::default()
        }
    }

    #[test]
    fn common_contributor_and_is_hoisted_to_global_options() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(vec![author(Some(AndOptions::Text))]),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![author(Some(AndOptions::Text))]),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);

        assert_eq!(
            refined
                .options
                .as_ref()
                .and_then(|options| options.contributors.as_ref())
                .and_then(|contributors| contributors.and.as_ref()),
            Some(&AndOptions::Text)
        );
        let citation_template = refined
            .citation
            .as_ref()
            .and_then(|citation| citation.template.as_ref())
            .expect("citation template should exist");
        let TemplateComponent::Contributor(author) = &citation_template[0] else {
            panic!("citation component should be author");
        };
        assert_eq!(author.and, None);
    }

    #[test]
    fn scope_contributor_and_is_hoisted_without_global_match() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(vec![author(Some(AndOptions::Symbol))]),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![primary_title(None)]),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);

        assert_eq!(
            refined
                .citation
                .as_ref()
                .and_then(|citation| citation.options.as_ref())
                .and_then(|options| options.contributors.as_ref())
                .and_then(|contributors| contributors.and.as_ref()),
            Some(&AndOptions::Symbol)
        );
        assert!(
            refined
                .options
                .as_ref()
                .and_then(|options| options.contributors.as_ref())
                .and_then(|contributors| contributors.and.as_ref())
                .is_none()
        );
    }

    #[test]
    fn preexisting_global_contributor_and_prunes_without_scope_hoist() {
        let style = Style {
            options: Some(citum_schema::options::Config {
                contributors: Some(ContributorConfig {
                    and: Some(AndOptions::Text),
                    ..ContributorConfig::default()
                }),
                ..citum_schema::options::Config::default()
            }),
            citation: Some(CitationSpec {
                template: Some(vec![author(Some(AndOptions::Text))]),
                ..CitationSpec::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![author(Some(AndOptions::Text))]),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);

        assert!(
            refined
                .citation
                .as_ref()
                .and_then(|citation| citation.options.as_ref())
                .and_then(|options| options.contributors.as_ref())
                .and_then(|contributors| contributors.and.as_ref())
                .is_none()
        );
        assert!(
            refined
                .bibliography
                .as_ref()
                .and_then(|bibliography| bibliography.options.as_ref())
                .and_then(|options| options.contributors.as_ref())
                .and_then(|contributors| contributors.and.as_ref())
                .is_none()
        );
        let citation_author = refined
            .citation
            .as_ref()
            .and_then(|citation| citation.template.as_ref())
            .and_then(|template| template.first())
            .expect("citation author exists");
        let TemplateComponent::Contributor(citation_author) = citation_author else {
            panic!("citation component should be contributor");
        };
        assert_eq!(citation_author.and, None);
    }

    #[test]
    fn citation_base_subsequent_and_ibid_are_pruned_against_options() {
        let style = Style {
            options: Some(style_options()),
            citation: Some(CitationSpec {
                template: Some(vec![
                    author_with_name_options(),
                    primary_title(Some(TextCase::SentenceApa)),
                ]),
                subsequent: Some(Box::new(CitationSpec {
                    template: Some(vec![author_with_name_options()]),
                    ..CitationSpec::default()
                })),
                ibid: Some(Box::new(CitationSpec {
                    template: Some(vec![author(Some(AndOptions::Text))]),
                    ..CitationSpec::default()
                })),
                ..CitationSpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let citation = refined.citation.as_ref().expect("citation should exist");
        let template = citation.template.as_ref().expect("template should exist");
        let TemplateComponent::Contributor(author) = &template[0] else {
            panic!("first component should be contributor");
        };
        assert_eq!(author.and, None);
        assert_eq!(author.name_form, None);
        assert_eq!(author.name_order, None);
        let TemplateComponent::Title(title) = &template[1] else {
            panic!("second component should be title");
        };
        assert_eq!(title.rendering.text_case, None);

        let subsequent = citation
            .subsequent
            .as_ref()
            .and_then(|spec| spec.template.as_ref())
            .expect("subsequent template should exist");
        let TemplateComponent::Contributor(subsequent_author) = &subsequent[0] else {
            panic!("subsequent component should be contributor");
        };
        assert_eq!(subsequent_author.and, None);
        assert_eq!(subsequent_author.name_form, None);

        let ibid = citation
            .ibid
            .as_ref()
            .and_then(|spec| spec.template.as_ref())
            .expect("ibid template should exist");
        let TemplateComponent::Contributor(ibid_author) = &ibid[0] else {
            panic!("ibid component should be contributor");
        };
        assert_eq!(ibid_author.and, None);
    }

    #[test]
    fn list_form_contributor_name_order_compacts_only_when_every_role_matches() {
        // A merged [editor, translator] component's explicit name-order
        // should compact only when it agrees with EVERY declared role's
        // effective default, not just the first one.
        let mut roles = std::collections::HashMap::new();
        roles.insert(
            "editor".to_string(),
            RoleRendering {
                name_order: Some(NameOrder::FamilyFirstOnly),
                ..RoleRendering::default()
            },
        );
        roles.insert(
            "translator".to_string(),
            RoleRendering {
                name_order: Some(NameOrder::FamilyFirstOnly),
                ..RoleRendering::default()
            },
        );
        let options = citum_schema::options::Config {
            contributors: Some(ContributorConfig {
                role: Some(RoleOptions {
                    roles: Some(roles),
                    ..RoleOptions::default()
                }),
                ..ContributorConfig::default()
            }),
            ..citum_schema::options::Config::default()
        };

        let merged = TemplateComponent::Contributor(TemplateContributor {
            contributor: vec![ContributorRole::Editor, ContributorRole::Translator].into(),
            form: ContributorForm::Long,
            name_order: Some(NameOrder::FamilyFirstOnly),
            ..TemplateContributor::default()
        });
        let style = Style {
            options: Some(options),
            bibliography: Some(BibliographySpec {
                template: Some(vec![merged]),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let template = refined
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.template.as_ref())
            .expect("bibliography template should exist");
        let TemplateComponent::Contributor(contributor) = &template[0] else {
            panic!("component should be contributor");
        };
        assert_eq!(
            contributor.name_order, None,
            "explicit name-order matching every declared role's default should compact"
        );
    }

    #[test]
    fn bibliography_full_variants_are_diffed_before_pruning() {
        let mut type_variants = indexmap::IndexMap::new();
        type_variants.insert(
            TypeSelector::Single("book".to_string()),
            TemplateVariant::Full(vec![TemplateComponent::Contributor(TemplateContributor {
                contributor: ContributorRole::Author.into(),
                form: ContributorForm::Long,
                and: None,
                ..TemplateContributor::default()
            })]),
        );
        let style = Style {
            options: Some(citum_schema::options::Config {
                contributors: Some(ContributorConfig {
                    and: Some(AndOptions::Text),
                    ..ContributorConfig::default()
                }),
                ..citum_schema::options::Config::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![author(Some(AndOptions::Text))]),
                type_variants: Some(type_variants),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let variant = refined
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.type_variants.as_ref())
            .and_then(|variants| variants.get(&TypeSelector::Single("book".to_string())))
            .expect("book variant should exist");

        let TemplateVariant::Full(template) = variant else {
            panic!("non-rendering inherited-field difference should remain Full after diff phase");
        };
        let TemplateComponent::Contributor(author) = &template[0] else {
            panic!("variant component should be contributor");
        };
        assert_eq!(author.and, None);
    }

    #[test]
    fn category_specific_type_mapping_title_defaults_are_compacted() {
        let mut type_variants = indexmap::IndexMap::new();
        type_variants.insert(
            TypeSelector::Single("article-journal".to_string()),
            TemplateVariant::Full(vec![container_title(Some(TextCase::SentenceApa))]),
        );
        let mut type_mapping = std::collections::HashMap::new();
        type_mapping.insert("article-journal".to_string(), "periodical".to_string());
        let style = Style {
            options: Some(citum_schema::options::Config {
                titles: Some(TitlesConfig {
                    type_mapping: Some(type_mapping),
                    periodical: Some(TitleRendering {
                        text_case: Some(TextCase::SentenceApa),
                        ..TitleRendering::default()
                    }),
                    ..TitlesConfig::default()
                }),
                ..citum_schema::options::Config::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![primary_title(None)]),
                type_variants: Some(type_variants),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let variant = refined
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.type_variants.as_ref())
            .and_then(|variants| variants.get(&TypeSelector::Single("article-journal".to_string())))
            .expect("article-journal variant should exist");
        let TemplateVariant::Full(template) = variant else {
            panic!("non-rendering title-kind difference should remain Full");
        };
        let TemplateComponent::Title(title) = &template[0] else {
            panic!("variant component should be title");
        };
        assert_eq!(title.rendering.text_case, None);
    }

    #[test]
    fn diff_add_payloads_are_pruned_after_diff_generation() {
        let mut type_variants = indexmap::IndexMap::new();
        type_variants.insert(
            TypeSelector::Single("webpage".to_string()),
            TemplateVariant::Full(vec![primary_title(None), author(Some(AndOptions::Text))]),
        );
        let style = Style {
            options: Some(citum_schema::options::Config {
                contributors: Some(ContributorConfig {
                    and: Some(AndOptions::Text),
                    ..ContributorConfig::default()
                }),
                ..citum_schema::options::Config::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![primary_title(None)]),
                type_variants: Some(type_variants),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let variant = refined
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.type_variants.as_ref())
            .and_then(|variants| variants.get(&TypeSelector::Single("webpage".to_string())))
            .expect("webpage variant should exist");

        let TemplateVariant::Diff(diff) = variant else {
            panic!("webpage variant should be encoded as a diff");
        };
        let TemplateComponent::Contributor(author) = &diff.add[0].component else {
            panic!("added component should be contributor");
        };
        assert_eq!(author.and, None);
    }

    #[test]
    fn suppress_false_is_normalized_on_full_templates_and_diff_adds() {
        let mut visible_author = author(Some(AndOptions::Text));
        visible_author.rendering_mut().suppress = Some(false);
        let mut added_url = url();
        added_url.rendering_mut().suppress = Some(false);
        let mut type_variants = indexmap::IndexMap::new();
        type_variants.insert(
            TypeSelector::Single("webpage".to_string()),
            TemplateVariant::Full(vec![visible_author.clone(), added_url]),
        );
        let style = Style {
            bibliography: Some(BibliographySpec {
                template: Some(vec![visible_author]),
                type_variants: Some(type_variants),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let bibliography = refined.bibliography.as_ref().expect("bibliography exists");
        assert_eq!(
            bibliography
                .template
                .as_ref()
                .and_then(|template| template[0].rendering().suppress),
            None
        );

        let variant = bibliography
            .type_variants
            .as_ref()
            .and_then(|variants| variants.get(&TypeSelector::Single("webpage".to_string())))
            .expect("webpage variant should exist");
        let TemplateVariant::Diff(diff) = variant else {
            panic!("webpage variant should be a diff");
        };
        assert_eq!(diff.add[0].component.rendering().suppress, None);
    }

    #[test]
    fn diff_modify_suppress_false_remains_explicit() {
        let mut base_title = primary_title(None);
        base_title.rendering_mut().suppress = Some(true);
        let mut unsuppressed_title = primary_title(None);
        unsuppressed_title.rendering_mut().suppress = Some(false);
        let mut type_variants = indexmap::IndexMap::new();
        type_variants.insert(
            TypeSelector::Single("book".to_string()),
            TemplateVariant::Full(vec![unsuppressed_title]),
        );
        let style = Style {
            bibliography: Some(BibliographySpec {
                template: Some(vec![base_title]),
                type_variants: Some(type_variants),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let variant = refined
            .bibliography
            .as_ref()
            .and_then(|bibliography| bibliography.type_variants.as_ref())
            .and_then(|variants| variants.get(&TypeSelector::Single("book".to_string())))
            .expect("book variant should exist");
        let TemplateVariant::Diff(diff) = variant else {
            panic!("rendering-only suppress change should be a diff");
        };
        assert_eq!(diff.modify[0].rendering.suppress, Some(false));
    }

    #[test]
    fn non_integral_suppress_false_is_normalized() {
        let mut visible_author = author(Some(AndOptions::Text));
        visible_author.rendering_mut().suppress = Some(false);
        let style = Style {
            citation: Some(CitationSpec {
                non_integral: Some(Box::new(CitationSpec {
                    template: Some(vec![visible_author]),
                    ..CitationSpec::default()
                })),
                ..CitationSpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);
        let non_integral = refined
            .citation
            .as_ref()
            .and_then(|citation| citation.non_integral.as_ref())
            .and_then(|spec| spec.template.as_ref())
            .expect("non-integral template exists");
        assert_eq!(non_integral[0].rendering().suppress, None);
    }

    #[test]
    fn refinement_preserves_engine_rendering() {
        let style = Style {
            options: Some(citum_schema::options::Config {
                contributors: Some(ContributorConfig {
                    and: Some(AndOptions::Text),
                    name_form: Some(NameForm::Initials),
                    ..ContributorConfig::default()
                }),
                titles: Some(TitlesConfig {
                    default: Some(TitleRendering {
                        text_case: Some(TextCase::SentenceApa),
                        ..TitleRendering::default()
                    }),
                    ..TitlesConfig::default()
                }),
                ..citum_schema::options::Config::default()
            }),
            bibliography: Some(BibliographySpec {
                template: Some(vec![
                    author_with_name_options(),
                    primary_title(Some(TextCase::SentenceApa)),
                ]),
                ..BibliographySpec::default()
            }),
            ..Style::default()
        };
        let bibliography = one_book_bibliography();

        let before = Processor::new(style.clone(), bibliography.clone()).render_bibliography();
        let after = Processor::new(refine_style(style), bibliography).render_bibliography();

        assert_eq!(after, before);
    }

    fn one_book_bibliography() -> Bibliography {
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_str(
            r#"{
                "id": "item-1",
                "type": "book",
                "author": [{"family": "Kuhn", "given": "Thomas S."}],
                "title": "The Structure of Scientific Revolutions",
                "issued": {"date-parts": [[1962]]},
                "publisher": "University of Chicago Press"
            }"#,
        )
        .expect("fixture should parse");
        let mut bibliography = Bibliography::new();
        bibliography.insert("item-1".to_string(), legacy.into());
        bibliography
    }

    fn reference_with_type(ref_type: &str) -> Bibliography {
        let legacy: csl_legacy::csl_json::Reference = serde_json::from_str(&format!(
            r#"{{
                "id": "item-1",
                "type": "{ref_type}",
                "author": [{{"family": "Kuhn", "given": "Thomas S."}}],
                "title": "The Structure of Scientific Revolutions",
                "container-title": "Journal of the History of Ideas",
                "issued": {{"date-parts": [[1962]]}},
                "publisher": "University of Chicago Press"
            }}"#
        ))
        .expect("fixture should parse");
        let mut bibliography = Bibliography::new();
        bibliography.insert("item-1".to_string(), legacy.into());
        bibliography
    }

    /// Mirrors `effective_title_rendering`'s fallback classification, using
    /// the same shared `citum_schema::options` functions, to build a
    /// template whose explicit rendering equals what SQI pruning would
    /// consider redundant with the engine's default.
    fn category_rendering(
        title_type: &TitleType,
        ref_type: &str,
        titles: &TitlesConfig,
    ) -> Option<TitleRendering> {
        match title_type {
            TitleType::Primary => match title_category(ref_type) {
                TitleCategory::Component => titles.component.clone(),
                TitleCategory::Monograph => titles.monograph.clone(),
                _ => titles.default.clone(),
            },
            TitleType::ContainerTitle => match container_title_category(ref_type) {
                TitleCategory::Periodical => titles.periodical.clone(),
                TitleCategory::ContainerMonograph => titles
                    .container_monograph
                    .clone()
                    .or_else(|| titles.monograph.clone()),
                _ => titles.default.clone(),
            },
            TitleType::ParentSerial => match parent_serial_title_category(ref_type) {
                TitleCategory::Periodical => titles.periodical.clone(),
                _ => titles.serial.clone(),
            },
            _ => None,
        }
    }

    #[rstest]
    #[case::primary(TitleType::Primary)]
    #[case::container_title(TitleType::ContainerTitle)]
    #[case::parent_serial(TitleType::ParentSerial)]
    fn sqi_pruning_matches_engine_rendering_for_every_classified_type(
        #[case] title_type: TitleType,
    ) {
        let titles = TitlesConfig {
            component: Some(TitleRendering {
                text_case: Some(TextCase::Title),
                ..TitleRendering::default()
            }),
            monograph: Some(TitleRendering {
                text_case: Some(TextCase::SentenceApa),
                ..TitleRendering::default()
            }),
            container_monograph: Some(TitleRendering {
                text_case: Some(TextCase::SentenceNlm),
                ..TitleRendering::default()
            }),
            periodical: Some(TitleRendering {
                text_case: Some(TextCase::Uppercase),
                ..TitleRendering::default()
            }),
            serial: Some(TitleRendering {
                text_case: Some(TextCase::Lowercase),
                ..TitleRendering::default()
            }),
            default: Some(TitleRendering {
                text_case: Some(TextCase::CapitalizeFirst),
                ..TitleRendering::default()
            }),
            ..TitlesConfig::default()
        };

        for ref_type in classified_ref_types() {
            let rendering = category_rendering(&title_type, ref_type, &titles)
                .expect("every classified ref_type resolves to a configured category");

            let style = Style {
                options: Some(citum_schema::options::Config {
                    titles: Some(titles.clone()),
                    ..citum_schema::options::Config::default()
                }),
                bibliography: Some(BibliographySpec {
                    template: Some(vec![TemplateComponent::Title(TemplateTitle {
                        title: title_type.clone(),
                        rendering: rendering.to_rendering(),
                        ..TemplateTitle::default()
                    })]),
                    ..BibliographySpec::default()
                }),
                ..Style::default()
            };
            let bibliography = reference_with_type(ref_type);

            let before = Processor::new(style.clone(), bibliography.clone()).render_bibliography();
            let after = Processor::new(refine_style(style), bibliography).render_bibliography();

            assert_eq!(
                after, before,
                "ref_type={ref_type} title_type={title_type:?}"
            );
        }
    }

    #[test]
    fn contributor_without_explicit_and_blocks_hoist() {
        let style = Style {
            citation: Some(CitationSpec {
                template: Some(vec![
                    author(Some(AndOptions::Text)),
                    TemplateComponent::Group(TemplateGroup {
                        group: vec![author(None)],
                        ..TemplateGroup::default()
                    }),
                ]),
                ..CitationSpec::default()
            }),
            ..Style::default()
        };

        let refined = refine_style(style);

        assert!(
            refined
                .citation
                .as_ref()
                .and_then(|citation| citation.options.as_ref())
                .and_then(|options| options.contributors.as_ref())
                .and_then(|contributors| contributors.and.as_ref())
                .is_none()
        );
        let template = refined
            .citation
            .as_ref()
            .and_then(|citation| citation.template.as_ref())
            .expect("citation template exists");
        let TemplateComponent::Contributor(author) = &template[0] else {
            panic!("first component should be contributor");
        };
        assert_eq!(author.and, Some(AndOptions::Text));
    }
}
