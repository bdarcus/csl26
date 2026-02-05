/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::AndOptions;
use crate::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable, Rendering,
    TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber, TemplateTitle, TitleType,
    WrapPunctuation,
};

/// Embedded citation template for Harvard style.
///
/// Renders as: (Author Year)
/// Example: (Smith and Jones 2024)
pub fn citation() -> Vec<TemplateComponent> {
    vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            and: Some(AndOptions::Text),
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                prefix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded bibliography template for Harvard/Elsevier style.
///
/// Renders as: Author, A.B. (Year) Title. Journal Volume(Issue), Pages.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // (Year)
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Title.
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // *Journal*
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume(Issue),
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Pages.
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}
