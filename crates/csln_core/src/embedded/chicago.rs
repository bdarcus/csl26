/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::options::AndOptions;
use crate::{
    tc_contributor, tc_date, tc_number, tc_title, tc_variable,
    template::{
        ContributorForm, ContributorRole, TemplateComponent, TemplateContributor, WrapPunctuation,
    },
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
        tc_date!(Issued, Year, prefix = " "),
    ]
}

/// Embedded bibliography template for Chicago author-date style.
///
/// Renders the full bibliographic entry in Chicago format:
/// Author, First. Year. "Article Title." *Journal Title* Volume (Issue): Pages. https://doi.org/xxx
pub fn author_date_bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        tc_contributor!(Author, Long, suffix = ". "),
        // Year.
        tc_date!(Issued, Year, suffix = ". "),
        // "Title" - quoted for articles
        tc_title!(Primary, quote = true, suffix = " "),
        // Journal Title - italicized
        tc_title!(ParentSerial, emph = true, suffix = " "),
        // Volume
        tc_number!(Volume),
        // (Issue)
        tc_number!(Issue, wrap = WrapPunctuation::Parentheses),
        // : Pages
        tc_number!(Pages, prefix = ": ", suffix = ". "),
        // DOI
        tc_variable!(Doi, prefix = "https://doi.org/"),
    ]
}
