/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Embedded CSLN style YAML files for priority citation styles.
//!
//! These are baked into the binary at compile time via `include_bytes!`,
//! allowing the CLI to load styles without a file path using `--builtin`.

use crate::Style;

/// Raw YAML bytes for an embedded style by name.
fn get_style_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "apa-7th" => Some(include_bytes!("../../../../styles/apa-7th.yaml")),
        "elsevier-harvard" => Some(include_bytes!("../../../../styles/elsevier-harvard.yaml")),
        "elsevier-with-titles" => Some(include_bytes!(
            "../../../../styles/elsevier-with-titles.yaml"
        )),
        "elsevier-vancouver" => Some(include_bytes!("../../../../styles/elsevier-vancouver.yaml")),
        "springer-basic-author-date" => Some(include_bytes!(
            "../../../../styles/springer-basic-author-date.yaml"
        )),
        "springer-basic-brackets" => Some(include_bytes!(
            "../../../../styles/springer-basic-brackets.yaml"
        )),
        "springer-vancouver-brackets" => Some(include_bytes!(
            "../../../../styles/springer-vancouver-brackets.yaml"
        )),
        "american-medical-association" => Some(include_bytes!(
            "../../../../styles/american-medical-association.yaml"
        )),
        "ieee" => Some(include_bytes!("../../../../styles/ieee.yaml")),
        "taylor-and-francis-chicago-author-date" => Some(include_bytes!(
            "../../../../styles/taylor-and-francis-chicago-author-date.yaml"
        )),
        "chicago-shortened-notes-bibliography" => Some(include_bytes!(
            "../../../../styles/chicago-shortened-notes-bibliography.yaml"
        )),
        "modern-language-association" => Some(include_bytes!(
            "../../../../styles/modern-language-association.yaml"
        )),
        _ => None,
    }
}

/// Parse an embedded style by name.
///
/// Returns `None` if `name` is not a known builtin.
/// Returns `Some(Err(_))` only if the embedded YAML is malformed (should not
/// happen for styles that passed CI).
pub fn get_embedded_style(name: &str) -> Option<Result<Style, serde_yaml::Error>> {
    get_style_bytes(name).map(serde_yaml::from_slice)
}

/// All available embedded (builtin) style names, ordered by corpus impact
/// (dependent-style count descending).
pub const EMBEDDED_STYLE_NAMES: &[&str] = &[
    "apa-7th",
    "elsevier-harvard",
    "elsevier-with-titles",
    "elsevier-vancouver",
    "springer-basic-author-date",
    "springer-vancouver-brackets",
    "springer-basic-brackets",
    "american-medical-association",
    "ieee",
    "taylor-and-francis-chicago-author-date",
    "chicago-shortened-notes-bibliography",
    "modern-language-association",
];
