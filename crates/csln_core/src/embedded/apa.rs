/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::{
    tc_contributor, tc_date, tc_number, tc_title, tc_variable,
    template::{TemplateComponent, WrapPunctuation},
};

/// Embedded citation template for APA style.
///
/// Renders as: (Author, Year)
/// Example: (Smith & Jones, 2024)
pub fn citation() -> Vec<TemplateComponent> {
    vec![tc_contributor!(Author, Short), tc_date!(Issued, Year)]
}

/// Embedded bibliography template for APA style.
///
/// Renders the full bibliographic entry in APA format:
/// Author, A. A., & Author, B. B. (Year). Title of work. *Journal Title*, *Volume*(Issue), Pages. https://doi.org/xxx
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        tc_contributor!(Author, Long, suffix = " "),
        // (Year).
        tc_date!(
            Issued,
            Year,
            wrap = WrapPunctuation::Parentheses,
            suffix = ". "
        ),
        // Title (primary) - italicized for monographs, plain for articles
        tc_title!(Primary, suffix = ". "),
        // Container title (journal) - italicized
        tc_title!(ParentSerial, emph = true, suffix = ", "),
        // Container title (book) - italicized, with "In " prefix
        tc_title!(ParentMonograph, prefix = "In ", emph = true, suffix = ", "),
        // Volume - italicized
        tc_number!(Volume, emph = true),
        // (Issue)
        tc_number!(Issue, wrap = WrapPunctuation::Parentheses, suffix = ", "),
        // Pages
        tc_number!(Pages, suffix = ". "),
        // Publisher
        tc_variable!(Publisher, suffix = ". "),
        // DOI
        tc_variable!(Doi, prefix = "https://doi.org/"),
    ]
}
