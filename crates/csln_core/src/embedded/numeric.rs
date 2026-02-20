/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

use crate::template::{NumberVariable, TemplateComponent, TemplateNumber};

/// Embedded citation template for plain numeric citation styles.
///
/// Renders as the citation number itself (wrapping is style-controlled):
/// `1`, `(1)`, or `[1]` depending on the parent citation options.
pub fn citation() -> Vec<TemplateComponent> {
    vec![TemplateComponent::Number(TemplateNumber {
        number: NumberVariable::CitationNumber,
        ..Default::default()
    })]
}
