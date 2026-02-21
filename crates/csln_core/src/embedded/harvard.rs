/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::AndOptions;
use crate::{
    tc_contributor, tc_date, tc_number, tc_title,
    template::{
        ContributorForm, ContributorRole, TemplateComponent, TemplateContributor, WrapPunctuation,
    },
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
        tc_date!(Issued, Year, prefix = " "),
    ]
}

/// Embedded bibliography template for Harvard/Elsevier style.
///
/// Renders as: Author, A.B. (Year) Title. Journal Volume(Issue), Pages.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        tc_contributor!(Author, Long, suffix = " "),
        // (Year)
        tc_date!(
            Issued,
            Year,
            wrap = WrapPunctuation::Parentheses,
            suffix = " "
        ),
        // Title.
        tc_title!(Primary, suffix = ". "),
        // *Journal*
        tc_title!(ParentSerial, emph = true, suffix = " "),
        // Volume(Issue),
        tc_number!(Volume),
        tc_number!(Issue, wrap = WrapPunctuation::Parentheses, suffix = ", "),
        // Pages.
        tc_number!(Pages, suffix = "."),
    ]
}
