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

/// Embedded citation template for IEEE style.
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

/// Embedded bibliography template for IEEE style.
///
/// Renders as: [1] A. B. Author and C. D. Author, "Title," *Journal*, vol. X, no. Y, pp. Z–W, Year.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // [Citation number]
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::CitationNumber,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Brackets),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            and: Some(AndOptions::Text),
            rendering: Rendering {
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // "Title,"
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                quote: Some(true),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // *Journal*,
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // vol. X,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering {
                prefix: Some("vol. ".to_string()),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // no. Y,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                prefix: Some("no. ".to_string()),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // pp. Z–W,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                prefix: Some("pp. ".to_string()),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year.
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}
