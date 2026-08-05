/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use citum_schema::template::{
    DateVariable, Rendering, SimpleVariable, TemplateComponent, TemplateVariable, TitleType,
    TypeSelector,
};
use csl_legacy::model::CslNode;

pub(super) fn selector_matches_any(selector: &TypeSelector, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| selector.matches(candidate))
}

struct LegalCaseFlags {
    has_issued: bool,
    has_parent_serial: bool,
    has_reporter: bool,
    has_page: bool,
    style_is_elsevier_harvard: bool,
    style_is_springer_socpsych: bool,
}

fn apply_legal_case_additions(template: &mut Vec<TemplateComponent>, flags: LegalCaseFlags) {
    if !flags.has_issued {
        let mut date_component = citum_schema::template::TemplateDate {
            date: DateVariable::Issued,
            ..Default::default()
        };
        if flags.style_is_springer_socpsych {
            date_component.form = citum_schema::template::DateForm::Full;
        }
        template.push(TemplateComponent::Date(date_component));
    }
    if !flags.has_parent_serial {
        template.push(TemplateComponent::Title(
            citum_schema::template::TemplateTitle {
                title: TitleType::ParentSerial,
                ..Default::default()
            },
        ));
    }
    if (flags.style_is_elsevier_harvard || flags.style_is_springer_socpsych) && !flags.has_reporter
    {
        template.push(TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Reporter,
            ..Default::default()
        }));
    }
    if flags.style_is_springer_socpsych && !flags.has_page {
        template.push(TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Page,
            ..Default::default()
        }));
    }
}

pub(super) fn normalize_legal_case_type_template(
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
) {
    let Some(map) = type_templates.as_mut() else {
        return;
    };
    let style_id = legacy_style.info.id.to_lowercase();
    let style_is_elsevier_harvard = style_id.contains("elsevier-harvard");
    let style_is_springer_socpsych = style_id.contains("springer-socpsych-author-date");

    for (selector, template) in map.iter_mut() {
        if !selector.matches("legal_case") && !selector.matches("legal-case") {
            continue;
        }

        let mut seen_locator = false;
        let mut seen_volume_number = false;
        let mut has_issued = false;
        let mut has_parent_serial = false;
        let mut has_reporter = false;
        let mut has_page = false;
        template.retain_mut(|component| {
            if let TemplateComponent::Term(term) = component
                && (matches!(term.term, citum_schema::locale::GeneralTerm::Circa)
                    || matches!(term.term, citum_schema::locale::GeneralTerm::At)
                    || matches!(term.term, citum_schema::locale::GeneralTerm::NoDate)
                    || matches!(term.term, citum_schema::locale::GeneralTerm::Section)
                    || matches!(term.term, citum_schema::locale::GeneralTerm::Accessed)
                    || matches!(term.term, citum_schema::locale::GeneralTerm::In))
            {
                return false;
            }

            if let TemplateComponent::Date(date_component) = component {
                if date_component.date == DateVariable::Issued {
                    has_issued = true;
                    date_component.rendering.suppress = Some(false);
                    if style_is_springer_socpsych {
                        date_component.form = citum_schema::template::DateForm::Full;
                    }
                } else {
                    return false;
                }
            }

            if let TemplateComponent::Title(title_component) = component {
                // Legal case names are not quoted; strip quote flags from primary title.
                if title_component.title == TitleType::Primary {
                    title_component.rendering.quote = None;
                    title_component.rendering.wrap = None;
                }
                // Book-chapter parent-monograph pattern does not apply to legal cases.
                if title_component.title == TitleType::ParentMonograph {
                    return false;
                }
                if matches!(
                    title_component.title,
                    TitleType::ContainerTitle | TitleType::ParentSerial
                ) {
                    has_parent_serial = true;
                }
            }

            if let TemplateComponent::Variable(variable) = component
                && (variable.variable == SimpleVariable::Locator
                    || variable.variable == SimpleVariable::Url)
            {
                if variable.variable == SimpleVariable::Locator {
                    if seen_locator {
                        return false;
                    }
                    seen_locator = true;
                } else {
                    return false;
                }
            }
            if let TemplateComponent::Variable(variable) = component {
                if variable.variable == SimpleVariable::Reporter {
                    has_reporter = true;
                }
                if variable.variable == SimpleVariable::Page {
                    has_page = true;
                }
                // Authority is not rendered in legal case entries for any style.
                if variable.variable == SimpleVariable::Authority {
                    return false;
                }
            }

            if let TemplateComponent::Number(number_component) = component
                && number_component.number == citum_schema::template::NumberVariable::Volume
            {
                if seen_volume_number {
                    // Remove duplicate volume from book-chapter in-container pattern.
                    return false;
                }
                seen_volume_number = true;
                number_component.rendering.suppress = Some(false);
            }

            true
        });

        apply_legal_case_additions(
            template,
            LegalCaseFlags {
                has_issued,
                has_parent_serial,
                has_reporter,
                has_page,
                style_is_elsevier_harvard,
                style_is_springer_socpsych,
            },
        );
    }
}

pub(super) fn ensure_inferred_media_type_templates(
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
    bibliography_template: &[TemplateComponent],
) {
    let map = type_templates.get_or_insert_with(indexmap::IndexMap::new);
    let enable_interview_detail =
        legacy_style_uses_contributor_variable(legacy_style, "interviewer");
    let enable_motion_picture_detail = legacy_style_mentions_motion_picture_term(legacy_style)
        || legacy_style_uses_contributor_variable(legacy_style, "director");

    if enable_motion_picture_detail
        && !map
            .keys()
            .any(|selector| selector.matches("motion_picture"))
    {
        let mut template = base_media_template_from_bibliography(bibliography_template);
        template.push(TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Genre,
            ..Default::default()
        }));
        template.push(TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Medium,
            ..Default::default()
        }));
        template.push(TemplateComponent::Contributor(
            citum_schema::template::TemplateContributor {
                contributor: citum_schema::template::ContributorRole::Director.into(),
                form: citum_schema::template::ContributorForm::Long,
                rendering: Rendering {
                    prefix: Some("Directed by ".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
        ));
        if template.len() >= 3 {
            map.insert(TypeSelector::Single("motion-picture".to_string()), template);
        }
    }

    if enable_interview_detail && !map.keys().any(|selector| selector.matches("interview")) {
        let mut template = base_media_template_from_bibliography(bibliography_template);
        template.push(TemplateComponent::Contributor(
            citum_schema::template::TemplateContributor {
                contributor: citum_schema::template::ContributorRole::Interviewer.into(),
                form: citum_schema::template::ContributorForm::Long,
                ..Default::default()
            },
        ));
        template.push(TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Medium,
            ..Default::default()
        }));
        template.push(TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Url,
            ..Default::default()
        }));
        if template.len() >= 3 {
            map.insert(TypeSelector::Single("interview".to_string()), template);
        }
    }
}

fn base_media_template_from_bibliography(
    bibliography_template: &[TemplateComponent],
) -> Vec<TemplateComponent> {
    let mut template = Vec::new();
    if let Some(author_component) = bibliography_template.iter().find_map(|component| {
        if let TemplateComponent::Contributor(contributor) = component
            && contributor.contributor == citum_schema::template::ContributorRole::Author
        {
            return Some(component.clone());
        }
        None
    }) {
        template.push(author_component);
    }
    // Title precedes date in patent/media bibliography entries (author → title → year).
    if let Some(title_component) = bibliography_template.iter().find_map(|component| {
        if let TemplateComponent::Title(title_component) = component
            && title_component.title == TitleType::Primary
        {
            return Some(component.clone());
        }
        None
    }) {
        template.push(title_component);
    }
    if let Some(issued_component) = bibliography_template.iter().find_map(|component| {
        if let TemplateComponent::Date(date_component) = component
            && date_component.date == DateVariable::Issued
        {
            return Some(component.clone());
        }
        None
    }) {
        template.push(issued_component);
    }
    template
}

pub(super) fn ensure_personal_communication_omitted(
    legacy_style: &csl_legacy::model::Style,
    citation_template: &[TemplateComponent],
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
) {
    if !citation_template_suppresses_personal_communication(citation_template)
        && !legacy_style_omits_personal_communication_in_bibliography(legacy_style)
    {
        return;
    }
    if !legacy_style_mentions_personal_communication(legacy_style) {
        return;
    }
    let map = type_templates.get_or_insert_with(indexmap::IndexMap::new);
    map.insert(
        TypeSelector::Single("personal-communication".to_string()),
        Vec::new(),
    );
    map.insert(
        TypeSelector::Single("personal-communication".to_string()),
        Vec::new(),
    );
}

fn citation_template_suppresses_personal_communication(template: &[TemplateComponent]) -> bool {
    template
        .iter()
        .any(component_suppresses_personal_communication)
}

fn component_suppresses_personal_communication(_component: &TemplateComponent) -> bool {
    false
}

fn legacy_style_mentions_personal_communication(style: &csl_legacy::model::Style) -> bool {
    fn node_mentions_personal_communication(node: &CslNode) -> bool {
        match node {
            CslNode::Choose(choose) => {
                let branch_mentions = |branch: &csl_legacy::model::ChooseBranch| {
                    branch.type_.as_ref().is_some_and(|types| {
                        types
                            .split_whitespace()
                            .any(|t| t == "personal_communication")
                    }) || branch
                        .children
                        .iter()
                        .any(node_mentions_personal_communication)
                };
                branch_mentions(&choose.if_branch)
                    || choose.else_if_branches.iter().any(branch_mentions)
                    || choose.else_branch.as_ref().is_some_and(|children| {
                        children.iter().any(node_mentions_personal_communication)
                    })
            }
            CslNode::Group(group) => group
                .children
                .iter()
                .any(node_mentions_personal_communication),
            _ => false,
        }
    }

    style.bibliography.as_ref().is_some_and(|bibliography| {
        bibliography
            .layout
            .children
            .iter()
            .any(node_mentions_personal_communication)
    })
}

fn legacy_style_omits_personal_communication_in_bibliography(
    style: &csl_legacy::model::Style,
) -> bool {
    fn node_has_omit_branch(node: &CslNode) -> bool {
        match node {
            CslNode::Choose(choose) => {
                let branch_is_omit = |branch: &csl_legacy::model::ChooseBranch| {
                    branch.type_.as_ref().is_some_and(|types| {
                        types
                            .split_whitespace()
                            .any(|t| t == "personal_communication")
                    }) && branch.children.is_empty()
                        && branch.variable.is_none()
                        && branch.is_numeric.is_none()
                        && branch.is_uncertain_date.is_none()
                        && branch.locator.is_none()
                        && branch.position.is_none()
                };

                branch_is_omit(&choose.if_branch)
                    || choose.else_if_branches.iter().any(branch_is_omit)
                    || choose.if_branch.children.iter().any(node_has_omit_branch)
                    || choose
                        .else_if_branches
                        .iter()
                        .any(|branch| branch.children.iter().any(node_has_omit_branch))
                    || choose
                        .else_branch
                        .as_ref()
                        .is_some_and(|children| children.iter().any(node_has_omit_branch))
            }
            CslNode::Group(group) => group.children.iter().any(node_has_omit_branch),
            _ => false,
        }
    }

    style.bibliography.as_ref().is_some_and(|bibliography| {
        bibliography
            .layout
            .children
            .iter()
            .any(node_has_omit_branch)
    })
}

fn legacy_style_uses_contributor_variable(
    style: &csl_legacy::model::Style,
    variable_name: &str,
) -> bool {
    fn node_uses_contributor_variable(node: &CslNode, variable_name: &str) -> bool {
        match node {
            CslNode::Names(names) => names
                .variable
                .split_whitespace()
                .any(|candidate| candidate == variable_name),
            CslNode::Group(group) => group
                .children
                .iter()
                .any(|child| node_uses_contributor_variable(child, variable_name)),
            CslNode::Choose(choose) => {
                choose
                    .if_branch
                    .children
                    .iter()
                    .any(|child| node_uses_contributor_variable(child, variable_name))
                    || choose.else_if_branches.iter().any(|branch| {
                        branch
                            .children
                            .iter()
                            .any(|child| node_uses_contributor_variable(child, variable_name))
                    })
                    || choose.else_branch.as_ref().is_some_and(|children| {
                        children
                            .iter()
                            .any(|child| node_uses_contributor_variable(child, variable_name))
                    })
            }
            _ => false,
        }
    }

    style.macros.iter().any(|macro_def| {
        macro_def
            .children
            .iter()
            .any(|node| node_uses_contributor_variable(node, variable_name))
    }) || style
        .citation
        .layout
        .children
        .iter()
        .any(|node| node_uses_contributor_variable(node, variable_name))
        || style.bibliography.as_ref().is_some_and(|bibliography| {
            bibliography
                .layout
                .children
                .iter()
                .any(|node| node_uses_contributor_variable(node, variable_name))
        })
}

fn legacy_style_mentions_motion_picture_term(style: &csl_legacy::model::Style) -> bool {
    fn node_mentions_motion_picture(node: &CslNode) -> bool {
        match node {
            CslNode::Text(text) => text
                .term
                .as_ref()
                .is_some_and(|term| term == "motion_picture"),
            CslNode::Group(group) => group.children.iter().any(node_mentions_motion_picture),
            CslNode::Choose(choose) => {
                choose
                    .if_branch
                    .children
                    .iter()
                    .any(node_mentions_motion_picture)
                    || choose
                        .else_if_branches
                        .iter()
                        .any(|branch| branch.children.iter().any(node_mentions_motion_picture))
                    || choose
                        .else_branch
                        .as_ref()
                        .is_some_and(|children| children.iter().any(node_mentions_motion_picture))
            }
            _ => false,
        }
    }

    style
        .macros
        .iter()
        .any(|macro_def| macro_def.children.iter().any(node_mentions_motion_picture))
        || style
            .citation
            .layout
            .children
            .iter()
            .any(node_mentions_motion_picture)
        || style.bibliography.as_ref().is_some_and(|bibliography| {
            bibliography
                .layout
                .children
                .iter()
                .any(node_mentions_motion_picture)
        })
}

fn legacy_style_uses_number_variable(style: &csl_legacy::model::Style) -> bool {
    fn node_uses_number_variable(node: &CslNode) -> bool {
        match node {
            CslNode::Number(number) => number.variable == "number",
            CslNode::Text(text) => text.variable.as_ref().is_some_and(|var| var == "number"),
            CslNode::Group(group) => group.children.iter().any(node_uses_number_variable),
            CslNode::Choose(choose) => {
                choose
                    .if_branch
                    .children
                    .iter()
                    .any(node_uses_number_variable)
                    || choose
                        .else_if_branches
                        .iter()
                        .any(|branch| branch.children.iter().any(node_uses_number_variable))
                    || choose
                        .else_branch
                        .as_ref()
                        .is_some_and(|children| children.iter().any(node_uses_number_variable))
            }
            _ => false,
        }
    }

    style
        .macros
        .iter()
        .any(|macro_def| macro_def.children.iter().any(node_uses_number_variable))
        || style
            .citation
            .layout
            .children
            .iter()
            .any(node_uses_number_variable)
        || style.bibliography.as_ref().is_some_and(|bibliography| {
            bibliography
                .layout
                .children
                .iter()
                .any(node_uses_number_variable)
        })
}

pub(super) fn ensure_inferred_patent_type_template(
    legacy_style: &csl_legacy::model::Style,
    type_templates: &mut Option<indexmap::IndexMap<TypeSelector, Vec<TemplateComponent>>>,
    bibliography_template: &[TemplateComponent],
) {
    let map = type_templates.get_or_insert_with(indexmap::IndexMap::new);
    if map.keys().any(|selector| selector.matches("patent")) {
        return;
    }

    let mut patent_template = base_media_template_from_bibliography(bibliography_template);

    if legacy_style_uses_number_variable(legacy_style) {
        patent_template.push(TemplateComponent::Number(
            citum_schema::template::TemplateNumber {
                number: citum_schema::template::NumberVariable::Number,
                ..Default::default()
            },
        ));
    }

    if patent_template.len() >= 2 {
        map.insert(TypeSelector::Single("patent".to_string()), patent_template);
    }
}
