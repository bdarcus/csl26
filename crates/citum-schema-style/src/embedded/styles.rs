/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Embedded Citum style YAML files for priority citation styles.
//!
//! These are baked into the binary at compile time via `include_bytes!`,
//! allowing the CLI to load styles without a file path using `--builtin`.

use crate::Style;

/// Raw YAML bytes for an embedded style by name.
fn get_style_bytes(name: &str) -> Option<&'static [u8]> {
    match name {
        "elsevier-harvard-core" => Some(include_bytes!(
            "../../embedded/styles/elsevier-harvard-core.yaml"
        )),
        "elsevier-with-titles-core" => Some(include_bytes!(
            "../../embedded/styles/elsevier-with-titles-core.yaml"
        )),
        "elsevier-vancouver-core" => Some(include_bytes!(
            "../../embedded/styles/elsevier-vancouver-core.yaml"
        )),
        "springer-basic-author-date-core" => Some(include_bytes!(
            "../../embedded/styles/springer-basic-author-date-core.yaml"
        )),
        "springer-basic-brackets-core" => Some(include_bytes!(
            "../../embedded/styles/springer-basic-brackets-core.yaml"
        )),
        "springer-vancouver-brackets-core" => Some(include_bytes!(
            "../../embedded/styles/springer-vancouver-brackets-core.yaml"
        )),
        "taylor-and-francis-chicago-author-date-core" => Some(include_bytes!(
            "../../embedded/styles/taylor-and-francis-chicago-author-date-core.yaml"
        )),
        "taylor-and-francis-council-of-science-editors-author-date-core" => Some(include_bytes!(
            "../../embedded/styles/taylor-and-francis-council-of-science-editors-author-date-core.yaml"
        )),
        "taylor-and-francis-national-library-of-medicine-core" => Some(include_bytes!(
            "../../embedded/styles/taylor-and-francis-national-library-of-medicine-core.yaml"
        )),
        "chicago-shortened-notes-bibliography-core" => Some(include_bytes!(
            "../../embedded/styles/chicago-shortened-notes-bibliography-core.yaml"
        )),
        "chicago-18-base" => Some(include_bytes!("../../embedded/styles/chicago-18-base.yaml")),
        "gb-t-7714-2025-base" => Some(include_bytes!(
            "../../embedded/styles/gb-t-7714-2025-base.yaml"
        )),
        "preset-bases/apa-7th" => Some(include_bytes!("../../embedded/styles/apa-7th.yaml")),
        "preset-bases/chicago-author-date-18th" => Some(include_bytes!(
            "../../embedded/styles/chicago-author-date-18th.yaml"
        )),
        "preset-bases/chicago-notes-18th" => Some(include_bytes!(
            "../../embedded/styles/chicago-notes-18th.yaml"
        )),
        "chicago-notes-18th" => Some(include_bytes!(
            "../../embedded/styles/chicago-notes-18th.yaml"
        )),
        "chicago-author-date-18th" => Some(include_bytes!(
            "../../embedded/styles/chicago-author-date-18th.yaml"
        )),
        "apa-7th" => Some(include_bytes!("../../embedded/styles/apa-7th.yaml")),
        "elsevier-harvard" => Some(include_bytes!(
            "../../embedded/styles/elsevier-harvard.yaml"
        )),
        "elsevier-with-titles" => Some(include_bytes!(
            "../../embedded/styles/elsevier-with-titles.yaml"
        )),
        "elsevier-vancouver" => Some(include_bytes!(
            "../../embedded/styles/elsevier-vancouver.yaml"
        )),
        "springer-basic-author-date" => Some(include_bytes!(
            "../../embedded/styles/springer-basic-author-date.yaml"
        )),
        "springer-basic-brackets" => Some(include_bytes!(
            "../../embedded/styles/springer-basic-brackets.yaml"
        )),
        "springer-vancouver-brackets" => Some(include_bytes!(
            "../../embedded/styles/springer-vancouver-brackets.yaml"
        )),
        "american-medical-association" => Some(include_bytes!(
            "../../embedded/styles/american-medical-association.yaml"
        )),
        "ieee" => Some(include_bytes!("../../embedded/styles/ieee.yaml")),
        "gb-t-7714-2025-numeric" => Some(include_bytes!(
            "../../embedded/styles/gb-t-7714-2025-numeric.yaml"
        )),
        "gb-t-7714-2025-author-date" => Some(include_bytes!(
            "../../embedded/styles/gb-t-7714-2025-author-date.yaml"
        )),
        "gb-t-7714-2025-note" => Some(include_bytes!(
            "../../embedded/styles/gb-t-7714-2025-note.yaml"
        )),
        "taylor-and-francis-chicago-author-date" => Some(include_bytes!(
            "../../embedded/styles/taylor-and-francis-chicago-author-date.yaml"
        )),
        "taylor-and-francis-council-of-science-editors-author-date" => Some(include_bytes!(
            "../../embedded/styles/taylor-and-francis-council-of-science-editors-author-date.yaml"
        )),
        "taylor-and-francis-national-library-of-medicine" => Some(include_bytes!(
            "../../embedded/styles/taylor-and-francis-national-library-of-medicine.yaml"
        )),
        "chicago-shortened-notes-bibliography" => Some(include_bytes!(
            "../../embedded/styles/chicago-shortened-notes-bibliography.yaml"
        )),
        "modern-language-association" => Some(include_bytes!(
            "../../embedded/styles/modern-language-association.yaml"
        )),
        _ => None,
    }
}

/// A mapping of short aliases to full embedded style names.
pub const EMBEDDED_STYLE_ALIASES: &[(&str, &str)] = &[
    ("apa", "apa-7th"),
    ("mla", "modern-language-association"),
    ("ieee", "ieee"),
    ("ama", "american-medical-association"),
    ("chicago", "chicago-shortened-notes-bibliography"),
    ("chicago-notes", "chicago-notes-18th"),
    ("chicago-author-date", "chicago-author-date-18th"),
    ("gb-t-7714-2025", "gb-t-7714-2025-numeric"),
    ("gb7714-2025", "gb-t-7714-2025-numeric"),
    ("vancouver", "elsevier-vancouver"),
    ("harvard", "elsevier-harvard"),
];

/// Resolve a style name or alias to the full embedded style name.
pub fn resolve_embedded_style_name(name: &str) -> Option<&'static str> {
    match name {
        "preset-bases/apa-7th" => return Some("preset-bases/apa-7th"),
        "preset-bases/chicago-author-date-18th" => {
            return Some("preset-bases/chicago-author-date-18th");
        }
        "preset-bases/chicago-notes-18th" => {
            return Some("preset-bases/chicago-notes-18th");
        }
        _ => {}
    }

    if let Some(n) = EMBEDDED_STYLE_NAMES.iter().find(|&&n| n == name) {
        return Some(*n);
    }
    EMBEDDED_STYLE_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, full)| *full)
}

/// Parse an embedded style by name or alias.
///
/// Returns `None` if `name` is not a known builtin or alias.
/// Returns `Some(Err(_))` only if the embedded YAML is malformed (should not
/// happen for styles that passed CI).
pub fn get_embedded_style(name: &str) -> Option<Result<Style, serde_yaml::Error>> {
    resolve_embedded_style_name(name)
        .and_then(get_style_bytes)
        .map(Style::from_yaml_bytes)
}

/// All available embedded (builtin) style names, ordered by corpus impact
/// (dependent-style count descending).
pub const EMBEDDED_STYLE_NAMES: &[&str] = &[
    "elsevier-harvard-core",
    "elsevier-with-titles-core",
    "elsevier-vancouver-core",
    "springer-basic-author-date-core",
    "springer-vancouver-brackets-core",
    "springer-basic-brackets-core",
    "taylor-and-francis-chicago-author-date-core",
    "taylor-and-francis-council-of-science-editors-author-date-core",
    "taylor-and-francis-national-library-of-medicine-core",
    "chicago-shortened-notes-bibliography-core",
    "chicago-18-base",
    "gb-t-7714-2025-base",
    "apa-7th",
    "elsevier-harvard",
    "elsevier-with-titles",
    "elsevier-vancouver",
    "springer-basic-author-date",
    "springer-vancouver-brackets",
    "springer-basic-brackets",
    "american-medical-association",
    "ieee",
    "gb-t-7714-2025-numeric",
    "gb-t-7714-2025-author-date",
    "gb-t-7714-2025-note",
    "taylor-and-francis-chicago-author-date",
    "taylor-and-francis-council-of-science-editors-author-date",
    "taylor-and-francis-national-library-of-medicine",
    "chicago-shortened-notes-bibliography",
    "chicago-notes-18th",
    "chicago-author-date-18th",
    "modern-language-association",
];

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use crate::style_base::StyleReference;

    const GB_T_7714_HEADS: [&str; 3] = [
        "gb-t-7714-2025-numeric",
        "gb-t-7714-2025-author-date",
        "gb-t-7714-2025-note",
    ];

    #[test]
    fn gb_t_7714_heads_inherit_the_hidden_bibliography_base() {
        let base = get_embedded_style("gb-t-7714-2025-base")
            .expect("hidden GB/T 7714 base should be embedded")
            .expect("hidden GB/T 7714 base should parse");
        assert!(
            base.citation.is_none(),
            "the hidden family base should not define a citation grammar"
        );
        assert!(
            base.bibliography.is_some(),
            "the hidden family base should define the shared bibliography grammar"
        );

        for name in GB_T_7714_HEADS {
            let head = get_embedded_style(name)
                .unwrap_or_else(|| panic!("{name} should be embedded"))
                .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
            assert_eq!(
                head.extends.as_ref().map(StyleReference::key),
                Some("gb-t-7714-2025-base"),
                "{name} should inherit the hidden family base"
            );

            let resolved = head.into_resolved();
            assert!(
                resolved.citation.is_some(),
                "{name} should provide its own citation grammar"
            );
            assert!(
                resolved.bibliography.is_some(),
                "{name} should inherit the shared bibliography grammar"
            );
        }
    }

    #[test]
    fn gb_t_7714_family_aliases_resolve_to_the_numeric_head() {
        for alias in ["gb-t-7714-2025", "gb7714-2025"] {
            assert_eq!(
                resolve_embedded_style_name(alias),
                Some("gb-t-7714-2025-numeric")
            );
        }
    }

    #[test]
    fn chicago_notes_family_adds_a_bibliography_only_in_the_descendant() {
        let authored = get_embedded_style("chicago-notes-18th")
            .expect("Chicago notes should be embedded")
            .expect("Chicago notes should parse");
        assert!(
            authored
                .raw_yaml
                .as_ref()
                .and_then(|raw| raw.get("bibliography"))
                .is_none(),
            "Chicago notes should omit the optional bibliography section"
        );

        let resolved = authored.into_resolved();
        assert!(
            resolved.citation.is_some(),
            "Chicago notes should retain its citation grammar"
        );
        assert!(
            resolved.bibliography.is_none(),
            "Chicago notes should remain bibliography-free after resolution"
        );

        let with_bibliography = get_embedded_style("chicago-shortened-notes-bibliography-core")
            .expect("the Chicago notes-with-bibliography core should be embedded")
            .expect("the Chicago notes-with-bibliography core should parse");
        assert_eq!(
            with_bibliography.extends.as_ref().map(StyleReference::key),
            Some("chicago-notes-18th"),
            "the bibliography-bearing variant should inherit Chicago notes"
        );
        assert!(
            with_bibliography.bibliography.is_some(),
            "the descendant should author its bibliography mapping"
        );
        assert!(
            with_bibliography.into_resolved().bibliography.is_some(),
            "the descendant should retain its bibliography after resolution"
        );
    }
}
