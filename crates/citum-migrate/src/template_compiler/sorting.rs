/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use super::{
    ContributorRole, DateVariable, TemplateCompiler, TemplateComponent, TemplateTitle, TitleType,
};

impl TemplateCompiler {
    pub(super) fn sort_citation_components(&self, components: &mut [TemplateComponent]) {
        components.sort_by_key(|c| match c {
            TemplateComponent::Contributor(c) if c.contributor == ContributorRole::Author => 0,
            TemplateComponent::Contributor(_) => 1,
            TemplateComponent::Date(d) if d.date == DateVariable::Issued => 2,
            TemplateComponent::Date(_) => 3,
            TemplateComponent::Title(_) => 4,
            _ => 5,
        });
    }

    /// Sort components for bibliography: citation-number first (for numeric styles),
    /// then author, date, title, then rest.
    #[allow(dead_code, reason = "helper functions")]
    pub(super) fn sort_bibliography_components(
        &self,
        components: &mut [TemplateComponent],
        is_numeric: bool,
    ) {
        components.sort_by_key(|c| match c {
            TemplateComponent::Contributor(c) if c.contributor == ContributorRole::Author => 1,
            TemplateComponent::Date(d) if d.date == DateVariable::Issued => {
                if is_numeric {
                    20
                } else {
                    2
                }
            }
            TemplateComponent::Title(t) if t.title == TitleType::Primary => 3,
            TemplateComponent::Title(t)
                if matches!(
                    t.title,
                    TitleType::ContainerTitle
                        | TitleType::ParentSerial
                        | TitleType::CollectionTitle
                ) =>
            {
                4
            }
            TemplateComponent::Title(t) if t.title == TitleType::ParentMonograph => 5,
            TemplateComponent::Number(_) => 6,
            TemplateComponent::Variable(_) => 7,
            TemplateComponent::Contributor(_) => 8,
            TemplateComponent::Date(_) => 9,
            TemplateComponent::Title(_) => 10,
            TemplateComponent::Group(l) => {
                if self.has_variable_recursive(
                    &l.group,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::Primary,
                        ..Default::default()
                    }),
                ) {
                    3
                } else if self.has_variable_recursive(
                    &l.group,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ContainerTitle,
                        ..Default::default()
                    }),
                ) || self.has_variable_recursive(
                    &l.group,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentSerial,
                        ..Default::default()
                    }),
                ) || self.has_variable_recursive(
                    &l.group,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::CollectionTitle,
                        ..Default::default()
                    }),
                ) {
                    4
                } else if self.has_variable_recursive(
                    &l.group,
                    &TemplateComponent::Title(TemplateTitle {
                        title: TitleType::ParentMonograph,
                        ..Default::default()
                    }),
                ) {
                    5
                } else {
                    11
                }
            }
            _ => 99,
        });
    }
}
