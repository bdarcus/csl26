/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::{
    tc_contributor, tc_date, tc_number, tc_title,
    template::{TemplateComponent, WrapPunctuation},
};

/// Embedded citation template for Vancouver (numeric) style.
///
/// Empty: the citation is its reference marker (`[1]`), declared with
/// `label-mode: numeric` and `label-wrap: brackets`.
/// See `docs/specs/REFERENCE_MARKERS.md`.
pub fn citation() -> Vec<TemplateComponent> {
    Vec::new()
}

/// Embedded bibliography template for Vancouver style.
///
/// Renders as: 1. Author AA, Author BB. Title. Journal. Year;Volume(Issue):Pages.
pub fn bibliography() -> Vec<TemplateComponent> {
    vec![
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
