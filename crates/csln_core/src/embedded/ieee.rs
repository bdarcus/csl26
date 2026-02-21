/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::AndOptions;
use crate::{
    tc_date, tc_number, tc_title,
    template::{
        ContributorForm, ContributorRole, TemplateComponent, TemplateContributor, WrapPunctuation,
    },
};

/// Embedded citation template for IEEE style.
///
/// Renders as: [1]
pub fn citation() -> Vec<TemplateComponent> {
    vec![tc_number!(CitationNumber, wrap = WrapPunctuation::Brackets)]
}

/// Embedded bibliography template for IEEE style.
///
/// Renders as: [1] A. B. Author and C. D. Author, "Title," *Journal*, vol. X, no. Y, pp. Z–W, Year.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // [Citation number]
        tc_number!(
            CitationNumber,
            wrap = WrapPunctuation::Brackets,
            suffix = " "
        ),
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            and: Some(AndOptions::Text),
            rendering: crate::template::Rendering {
                suffix: Some(", ".to_string()),
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
        // pp. Z–W,
        tc_number!(Pages, prefix = "pp. ", suffix = ", "),
        // Year.
        tc_date!(Issued, Year, suffix = "."),
    ]
}
