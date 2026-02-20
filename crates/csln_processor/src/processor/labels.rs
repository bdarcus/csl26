/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Label generation for alphanumeric citation styles.
//!
//! Generates citation labels like `[AHU74]`, `[Knu84]`, `[Ban+92]`
//! from author family names and publication year.
//!
//! ## Algorithm
//!
//! - 1 author:  up to `single_author_chars` chars from family name + year
//! - 2+ authors (below et_al_min): `multi_author_chars` chars each + year
//! - >= et_al_min authors: first 3 authors' initials + et_al_marker + year

use crate::reference::Reference;
use csln_core::options::LabelParams;

/// Generate a base citation label for a reference (without disambiguation suffix).
///
/// Uses the first letter(s) of author family names combined with the
/// publication year according to the resolved LabelParams.
pub fn generate_base_label(reference: &Reference, params: &LabelParams) -> String {
    let name_part = generate_name_part(reference, params);
    let year_part = generate_year_part(reference, params.year_digits);
    format!("{}{}", name_part, year_part)
}

fn generate_name_part(reference: &Reference, params: &LabelParams) -> String {
    let Some(contributor) = reference.author().or_else(|| reference.editor()) else {
        // No author/editor: use first 3 chars of title
        return reference
            .title()
            .map(|t| {
                let s = t.to_string();
                s.chars()
                    .filter(|c| c.is_alphabetic())
                    .take(params.single_author_chars as usize)
                    .collect()
            })
            .unwrap_or_default();
    };

    let names = contributor.to_names_vec();
    let count = names.len();

    if count == 1 {
        // Single author: up to single_author_chars from family name
        let family = names[0].family_or_literal();
        family
            .chars()
            .filter(|c| c.is_alphabetic())
            .take(params.single_author_chars as usize)
            .collect()
    } else if count < params.et_al_min as usize {
        // 2 to et_al_min-1 authors: multi_author_chars chars each
        names
            .iter()
            .map(|n| {
                n.family_or_literal()
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .take(params.multi_author_chars as usize)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        // et_al_min or more authors: first 3 initials + et_al_marker
        let initials: String = names
            .iter()
            .take(3)
            .map(|n| {
                n.family_or_literal()
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .take(params.multi_author_chars as usize)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("");
        format!("{}{}", initials, params.et_al_marker)
    }
}

fn generate_year_part(reference: &Reference, year_digits: u8) -> String {
    reference
        .issued()
        .and_then(|d| d.year().parse::<i32>().ok())
        .map(|y| {
            let y_str = y.to_string();
            if year_digits == 2 && y_str.len() >= 2 {
                y_str[y_str.len() - 2..].to_string()
            } else {
                y_str
            }
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use csl_legacy::csl_json::{DateVariable, Name, Reference as LegacyReference};
    use csln_core::options::{LabelConfig, LabelPreset};

    fn alpha_params() -> LabelParams {
        LabelConfig::default().effective_params()
    }

    fn din_params() -> LabelParams {
        LabelConfig {
            preset: LabelPreset::Din,
            ..Default::default()
        }
        .effective_params()
    }

    fn params_4digit_year() -> LabelParams {
        LabelParams {
            year_digits: 4,
            ..alpha_params()
        }
    }

    fn make_ref(authors: Vec<Name>, year: i32) -> Reference {
        Reference::from(LegacyReference {
            id: "t".to_string(),
            ref_type: "book".to_string(),
            author: Some(authors),
            issued: Some(DateVariable::year(year)),
            ..Default::default()
        })
    }

    #[test]
    fn test_single_author_alpha() {
        // Kuhn 1962 → "Kuh62" (3 chars from family name + 2-digit year)
        let r = make_ref(vec![Name::new("Kuhn", "Thomas S.")], 1962);
        assert_eq!(generate_base_label(&r, &alpha_params()), "Kuh62");
    }

    #[test]
    fn test_single_author_short_family() {
        // "Li" only has 2 chars; no padding expected
        let r = make_ref(vec![Name::new("Li", "Wei")], 2010);
        assert_eq!(generate_base_label(&r, &alpha_params()), "Li10");
    }

    #[test]
    fn test_single_author_din() {
        // DIN uses up to 4 chars for single-author labels
        let r = make_ref(vec![Name::new("Kuhn", "Thomas S.")], 1962);
        assert_eq!(generate_base_label(&r, &din_params()), "Kuhn62");
    }

    #[test]
    fn test_two_authors_alpha() {
        // 2 < et_al_min=4 → multi-author case: 1 char each
        // Weinberg + Freedman 1971 → "WF71"
        let r = make_ref(
            vec![
                Name::new("Weinberg", "Gerald M."),
                Name::new("Freedman", "Daniel P."),
            ],
            1971,
        );
        assert_eq!(generate_base_label(&r, &alpha_params()), "WF71");
    }

    #[test]
    fn test_three_authors_alpha() {
        // 3 < et_al_min=4 → multi-author case: 1 char each
        // LeCun + Bengio + Hinton 2015 → "LBH15"
        let r = make_ref(
            vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ],
            2015,
        );
        assert_eq!(generate_base_label(&r, &alpha_params()), "LBH15");
    }

    #[test]
    fn test_et_al_alpha() {
        // 8 >= et_al_min=4 → first 3 initials + "+" marker
        // Vaswani, Shazeer, Parmar, ... 2017 → "VSP+17"
        let r = make_ref(
            vec![
                Name::new("Vaswani", "Ashish"),
                Name::new("Shazeer", "Noam"),
                Name::new("Parmar", "Niki"),
                Name::new("Uszkoreit", "Jakob"),
                Name::new("Jones", "Llion"),
                Name::new("Gomez", "Aidan N."),
                Name::new("Kaiser", "Lukasz"),
                Name::new("Polosukhin", "Illia"),
            ],
            2017,
        );
        assert_eq!(generate_base_label(&r, &alpha_params()), "VSP+17");
    }

    #[test]
    fn test_three_authors_din_triggers_et_al() {
        // DIN et_al_min=3: count=3 is NOT < 3, so et_al case (no marker)
        // LeCun + Bengio + Hinton 2015 → "LBH15"
        let r = make_ref(
            vec![
                Name::new("LeCun", "Yann"),
                Name::new("Bengio", "Yoshua"),
                Name::new("Hinton", "Geoffrey"),
            ],
            2015,
        );
        assert_eq!(generate_base_label(&r, &din_params()), "LBH15");
    }

    #[test]
    fn test_et_al_din_no_marker() {
        // DIN et_al case has no "+" → "VSP17" not "VSP+17"
        let r = make_ref(
            vec![
                Name::new("Vaswani", "Ashish"),
                Name::new("Shazeer", "Noam"),
                Name::new("Parmar", "Niki"),
                Name::new("Uszkoreit", "Jakob"),
            ],
            2017,
        );
        assert_eq!(generate_base_label(&r, &din_params()), "VSP17");
    }

    #[test]
    fn test_four_digit_year() {
        let r = make_ref(vec![Name::new("Kuhn", "Thomas S.")], 1962);
        assert_eq!(generate_base_label(&r, &params_4digit_year()), "Kuh1962");
    }

    #[test]
    fn test_literal_org_author() {
        // Literal name: take single_author_chars=3 from "World Bank"
        let r = Reference::from(LegacyReference {
            id: "t".to_string(),
            ref_type: "report".to_string(),
            author: Some(vec![Name::literal("World Bank")]),
            issued: Some(DateVariable::year(2023)),
            ..Default::default()
        });
        assert_eq!(generate_base_label(&r, &alpha_params()), "Wor23");
    }

    #[test]
    fn test_no_author_falls_back_to_title() {
        // No author or editor: use first 3 alpha chars of title
        let r = Reference::from(LegacyReference {
            id: "t".to_string(),
            ref_type: "book".to_string(),
            title: Some("Deep Learning".to_string()),
            issued: Some(DateVariable::year(2016)),
            ..Default::default()
        });
        assert_eq!(generate_base_label(&r, &alpha_params()), "Dee16");
    }

    #[test]
    fn test_no_date_gives_empty_year() {
        // Missing issued date → year part is empty
        let r = Reference::from(LegacyReference {
            id: "t".to_string(),
            ref_type: "book".to_string(),
            author: Some(vec![Name::new("Knuth", "Donald E.")]),
            ..Default::default()
        });
        assert_eq!(generate_base_label(&r, &alpha_params()), "Knu");
    }
}
