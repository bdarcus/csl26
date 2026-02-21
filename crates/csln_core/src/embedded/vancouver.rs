/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::{
    tc_contributor, tc_date, tc_number, tc_title,
    template::{TemplateComponent, WrapPunctuation},
};

/// Embedded citation template for Vancouver (numeric) style.
///
/// Renders as: [1]
pub fn citation() -> Vec<TemplateComponent> {
    vec![tc_number!(CitationNumber, wrap = WrapPunctuation::Brackets)]
}

/// Embedded bibliography template for Vancouver style.
///
/// Renders as: 1. Author AA, Author BB. Title. Journal. Year;Volume(Issue):Pages.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
        // Citation number.
        tc_number!(CitationNumber, suffix = ". "),
        // Author (Vancouver format - all initials, no periods)
        tc_contributor!(Author, Long, suffix = ". "),
        // Title
        tc_title!(Primary, suffix = ". "),
        // Journal
        tc_title!(ParentSerial, suffix = ". "),
        // Year;
        tc_date!(Issued, Year, suffix = ";"),
        // Volume
        tc_number!(Volume),
        // (Issue)
        tc_number!(Issue, wrap = WrapPunctuation::Parentheses),
        // :Pages
        tc_number!(Pages, prefix = ":", suffix = "."),
    ]
}
