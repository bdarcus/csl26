/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable, Rendering,
    TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber, TemplateTitle, TitleType,
    WrapPunctuation,
};

/// Embedded citation template for Vancouver (numeric) style.
///
/// Renders as: [1]
pub fn citation() -> Vec<TemplateComponent> {
    vec![TemplateComponent::Number(TemplateNumber {
        number: NumberVariable::CitationNumber,
        form: None,
        rendering: Rendering {
            wrap: Some(WrapPunctuation::Brackets),
            ..Default::default()
        },
        ..Default::default()
    })]
}

/// Embedded bibliography template for Vancouver style.
///
/// Renders as: 1. Author AA, Author BB. Title. Journal. Year;Volume(Issue):Pages.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // Citation number.
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::CitationNumber,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Author (Vancouver format - all initials, no periods)
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Title
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Journal
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year;
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(";".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        // (Issue)
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                ..Default::default()
            },
            ..Default::default()
        }),
        // :Pages
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                prefix: Some(":".to_string()),
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}
