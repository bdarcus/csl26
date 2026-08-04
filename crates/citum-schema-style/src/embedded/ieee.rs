/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::options::AndOptions;
use crate::{
    tc_date, tc_number, tc_title,
    template::{
        ContributorForm, ContributorRole, LabelForm, NumberVariable, TemplateComponent,
        TemplateContributor, TemplateNumber,
    },
};

/// Embedded citation template for IEEE style.
///
/// Empty: the citation is its reference marker (`[1]`), declared with
/// `label-mode: numeric` and `item-wrap: brackets`.
/// See `docs/specs/REFERENCE_MARKERS.md`.
pub fn citation() -> Vec<TemplateComponent> {
    Vec::new()
}

/// Embedded bibliography template for IEEE style.
///
/// Renders as: \[1\] A. B. Author and C. D. Author, "Title," *Journal*, vol. X, no. Y, localized pages, Year.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author.into(),
            form: ContributorForm::Long,
            and: Some(AndOptions::Text),
            rendering: crate::template::Rendering {
                suffix: Some(", ".into()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // "Title,"
        tc_title!(Primary, quote = true, suffix = " "),
        // *Journal*,
        tc_title!(ParentSerial, emph = true, suffix = ", "),
        // vol. X,
        tc_number!(Volume, prefix = "vol. ", suffix = ", "),
        // no. Y,
        tc_number!(Issue, prefix = "no. ", suffix = ", "),
        // Localized page label and page range,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            label_form: Some(LabelForm::Short),
            rendering: crate::template::Rendering {
                suffix: Some(", ".into()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year.
        tc_date!(Issued, Year, suffix = "."),
    ]
}
