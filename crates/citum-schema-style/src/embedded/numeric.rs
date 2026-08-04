/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

use crate::template::TemplateComponent;

/// Embedded citation template for plain numeric citation styles.
///
/// Empty: the citation is its reference marker, which the style declares with
/// `label-mode: numeric` rather than authoring. Wrapping is style-controlled —
/// `1`, `(1)`, or `[1]` depending on the parent citation options.
/// See `docs/specs/REFERENCE_MARKERS.md`.
pub fn citation() -> Vec<TemplateComponent> {
    Vec::new()
}
