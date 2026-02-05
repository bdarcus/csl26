/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable, Rendering,
    SimpleVariable, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
    TemplateTitle, TemplateVariable, TitleType, WrapPunctuation,
};

/// Embedded citation template for APA style.
///
/// Renders as: (Author, Year)
/// Example: (Smith & Jones, 2024)
pub fn citation() -> Vec<TemplateComponent> {
    vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            ..Default::default()
        }),
    ]
}

/// Embedded bibliography template for APA style.
///
/// Renders the full bibliographic entry in APA format:
/// Author, A. A., & Author, B. B. (Year). Title of work. *Journal Title*, *Volume*(Issue), Pages. https://doi.org/xxx
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
        // (Year).
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Title (primary) - italicized for monographs, plain for articles
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Container title (journal) - italicized
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
        // Container title (book) - italicized, with "In " prefix
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentMonograph,
            form: None,
            rendering: Rendering {
                prefix: Some("In ".to_string()),
                emph: Some(true),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume - italicized
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }),
        // (Issue)
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
        // Pages
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Publisher
        TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Publisher,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // DOI
        TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Doi,
            rendering: Rendering {
                prefix: Some("https://doi.org/".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}
