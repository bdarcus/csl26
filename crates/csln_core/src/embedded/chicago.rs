/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::AndOptions;
use crate::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable, Rendering,
    SimpleVariable, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
    TemplateTitle, TemplateVariable, TitleType, WrapPunctuation,
};

/// Embedded citation template for Chicago author-date style.
///
/// Renders as: (Author Year)
/// Example: (Smith and Jones 2024)
pub fn author_date_citation() -> Vec<TemplateComponent> {
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

/// Embedded bibliography template for Chicago author-date style.
///
/// Renders the full bibliographic entry in Chicago format:
/// Author, First. Year. "Article Title." *Journal Title* Volume (Issue): Pages. https://doi.org/xxx
pub fn author_date_bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year.
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // "Title" - quoted for articles
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
        // Journal Title - italicized
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
        // : Pages
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                prefix: Some(": ".to_string()),
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
