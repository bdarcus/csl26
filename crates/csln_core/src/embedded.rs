/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Embedded priority templates for common citation styles.
//!
//! This module provides pre-built templates for the most commonly used parent styles
//! in the CSL ecosystem. These templates are useful for:
//!
//! - **Migration tooling**: Faster migration of simple dependent styles
//! - **Reference implementations**: Battle-tested templates for testing
//! - **Fallback defaults**: When a style omits templates but specifies presets
//!
//! ## Priority Styles
//!
//! Based on analysis of dependent styles, these are the top parent styles:
//!
//! | Style | Dependents | Format |
//! |-------|------------|--------|
//! | APA | 783 | author-date |
//! | Elsevier with Titles | 672 | numeric |
//! | Elsevier Harvard | 665 | author-date |
//! | Elsevier Vancouver | 502 | numeric |
//! | Springer Vancouver Brackets | 472 | numeric |
//!
//! See `.agent/design/STYLE_ALIASING.md` for full analysis.

use crate::options::AndOptions;
use crate::template::{
    ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable, Rendering,
    SimpleVariable, TemplateComponent, TemplateContributor, TemplateDate, TemplateNumber,
    TemplateTitle, TemplateVariable, TitleType, WrapPunctuation,
};
use std::collections::HashMap;

/// Embedded citation template for APA style.
///
/// Renders as: (Author, Year)
/// Example: (Smith & Jones, 2024)
pub fn apa_citation() -> Vec<TemplateComponent> {
    vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            ..Default::default()
        }),
    ]
}

/// Embedded bibliography template for APA style.
///
/// Renders the full bibliographic entry in APA format:
/// Author, A. A., & Author, B. B. (Year). Title of work. *Journal Title*, *Volume*(Issue), Pages. https://doi.org/xxx
pub fn apa_bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // (Year).
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Title (primary) - italicized for monographs, plain for articles
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Container title (journal) - italicized
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume - italicized
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }),
        // (Issue)
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Pages
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // DOI
        TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Doi,
            rendering: Rendering {
                prefix: Some("https://doi.org/".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded citation template for Chicago author-date style.
///
/// Renders as: (Author Year)
/// Example: (Smith and Jones 2024)
pub fn chicago_author_date_citation() -> Vec<TemplateComponent> {
    vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            and: Some(AndOptions::Text),
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                prefix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded bibliography template for Chicago author-date style.
///
/// Renders the full bibliographic entry in Chicago format:
/// Author, First. Year. "Article Title." *Journal Title* Volume (Issue): Pages. https://doi.org/xxx
pub fn chicago_author_date_bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year.
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // "Title" - quoted for articles
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                quote: Some(true),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Journal Title - italicized
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        // (Issue)
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                ..Default::default()
            },
            ..Default::default()
        }),
        // : Pages
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                prefix: Some(": ".to_string()),
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // DOI
        TemplateComponent::Variable(TemplateVariable {
            variable: SimpleVariable::Doi,
            rendering: Rendering {
                prefix: Some("https://doi.org/".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded citation template for Vancouver (numeric) style.
///
/// Renders as: [1]
pub fn vancouver_citation() -> Vec<TemplateComponent> {
    vec![TemplateComponent::Number(TemplateNumber {
        number: NumberVariable::CitationNumber,
        form: None,
        rendering: Rendering {
            wrap: Some(WrapPunctuation::Brackets),
            ..Default::default()
        },
        ..Default::default()
    })]
}

/// Embedded bibliography template for Vancouver style.
///
/// Renders as: 1. Author AA, Author BB. Title. Journal. Year;Volume(Issue):Pages.
pub fn vancouver_bibliography() -> Vec<TemplateComponent> {
    vec![
        // Citation number.
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::CitationNumber,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Author (Vancouver format - all initials, no periods)
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Title
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Journal
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year;
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(";".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        // (Issue)
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                ..Default::default()
            },
            ..Default::default()
        }),
        // :Pages
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                prefix: Some(":".to_string()),
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded citation template for IEEE style.
///
/// Renders as: [1]
pub fn ieee_citation() -> Vec<TemplateComponent> {
    vec![TemplateComponent::Number(TemplateNumber {
        number: NumberVariable::CitationNumber,
        form: None,
        rendering: Rendering {
            wrap: Some(WrapPunctuation::Brackets),
            ..Default::default()
        },
        ..Default::default()
    })]
}

/// Embedded bibliography template for IEEE style.
///
/// Renders as: [1] A. B. Author and C. D. Author, "Title," *Journal*, vol. X, no. Y, pp. Z–W, Year.
pub fn ieee_bibliography() -> Vec<TemplateComponent> {
    vec![
        // [Citation number]
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::CitationNumber,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Brackets),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            and: Some(AndOptions::Text),
            rendering: Rendering {
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // "Title,"
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                quote: Some(true),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // *Journal*,
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // vol. X,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering {
                prefix: Some("vol. ".to_string()),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // no. Y,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                prefix: Some("no. ".to_string()),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // pp. Z–W,
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                prefix: Some("pp. ".to_string()),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Year.
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded citation template for Harvard style.
///
/// Renders as: (Author Year)
/// Example: (Smith and Jones 2024)
pub fn harvard_citation() -> Vec<TemplateComponent> {
    vec![
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Short,
            and: Some(AndOptions::Text),
            ..Default::default()
        }),
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                prefix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Embedded bibliography template for Harvard/Elsevier style.
///
/// Renders as: Author, A.B. (Year) Title. Journal Volume(Issue), Pages.
pub fn harvard_bibliography() -> Vec<TemplateComponent> {
    vec![
        // Author
        TemplateComponent::Contributor(TemplateContributor {
            contributor: ContributorRole::Author,
            form: ContributorForm::Long,
            rendering: Rendering {
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // (Year)
        TemplateComponent::Date(TemplateDate {
            date: DateVariable::Issued,
            form: DateForm::Year,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Title.
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::Primary,
            form: None,
            rendering: Rendering {
                suffix: Some(". ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // *Journal*
        TemplateComponent::Title(TemplateTitle {
            title: TitleType::ParentSerial,
            form: None,
            rendering: Rendering {
                emph: Some(true),
                suffix: Some(" ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Volume(Issue),
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Volume,
            form: None,
            rendering: Rendering::default(),
            ..Default::default()
        }),
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Issue,
            form: None,
            rendering: Rendering {
                wrap: Some(WrapPunctuation::Parentheses),
                suffix: Some(", ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
        // Pages.
        TemplateComponent::Number(TemplateNumber {
            number: NumberVariable::Pages,
            form: None,
            rendering: Rendering {
                suffix: Some(".".to_string()),
                ..Default::default()
            },
            ..Default::default()
        }),
    ]
}

/// Get all available embedded citation templates.
///
/// Returns a map of style names to their citation templates.
pub fn citation_templates() -> HashMap<&'static str, Vec<TemplateComponent>> {
    let mut map = HashMap::new();
    map.insert("apa", apa_citation());
    map.insert("chicago-author-date", chicago_author_date_citation());
    map.insert("vancouver", vancouver_citation());
    map.insert("ieee", ieee_citation());
    map.insert("harvard", harvard_citation());
    map
}

/// Get all available embedded bibliography templates.
///
/// Returns a map of style names to their bibliography templates.
pub fn bibliography_templates() -> HashMap<&'static str, Vec<TemplateComponent>> {
    let mut map = HashMap::new();
    map.insert("apa", apa_bibliography());
    map.insert("chicago-author-date", chicago_author_date_bibliography());
    map.insert("vancouver", vancouver_bibliography());
    map.insert("ieee", ieee_bibliography());
    map.insert("harvard", harvard_bibliography());
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apa_citation_structure() {
        let template = apa_citation();
        assert_eq!(template.len(), 2);

        match &template[0] {
            TemplateComponent::Contributor(c) => {
                assert_eq!(c.contributor, ContributorRole::Author);
                assert_eq!(c.form, ContributorForm::Short);
            }
            _ => panic!("Expected Contributor"),
        }

        match &template[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(d.date, DateVariable::Issued);
                assert_eq!(d.form, DateForm::Year);
            }
            _ => panic!("Expected Date"),
        }
    }

    #[test]
    fn test_apa_bibliography_structure() {
        let template = apa_bibliography();
        assert!(
            template.len() >= 6,
            "APA bibliography should have multiple components"
        );

        // Check first component is author
        match &template[0] {
            TemplateComponent::Contributor(c) => {
                assert_eq!(c.contributor, ContributorRole::Author);
            }
            _ => panic!("First component should be Contributor"),
        }

        // Check second component is date with parentheses
        match &template[1] {
            TemplateComponent::Date(d) => {
                assert_eq!(d.rendering.wrap, Some(WrapPunctuation::Parentheses));
            }
            _ => panic!("Second component should be Date"),
        }
    }

    #[test]
    fn test_vancouver_citation_is_numeric() {
        let template = vancouver_citation();
        assert_eq!(template.len(), 1);

        match &template[0] {
            TemplateComponent::Number(n) => {
                assert_eq!(n.number, NumberVariable::CitationNumber);
                assert_eq!(n.rendering.wrap, Some(WrapPunctuation::Brackets));
            }
            _ => panic!("Vancouver citation should be a Number component"),
        }
    }

    #[test]
    fn test_chicago_uses_text_and() {
        let template = chicago_author_date_citation();

        match &template[0] {
            TemplateComponent::Contributor(c) => {
                assert_eq!(c.and, Some(AndOptions::Text));
            }
            _ => panic!("Expected Contributor"),
        }
    }

    #[test]
    fn test_citation_templates_map() {
        let templates = citation_templates();
        assert!(templates.contains_key("apa"));
        assert!(templates.contains_key("chicago-author-date"));
        assert!(templates.contains_key("vancouver"));
        assert!(templates.contains_key("ieee"));
        assert!(templates.contains_key("harvard"));
    }

    #[test]
    fn test_bibliography_templates_map() {
        let templates = bibliography_templates();
        assert!(templates.contains_key("apa"));
        assert!(templates.contains_key("chicago-author-date"));
        assert!(templates.contains_key("vancouver"));
        assert!(templates.contains_key("ieee"));
        assert!(templates.contains_key("harvard"));
    }

    #[test]
    fn test_ieee_bibliography_has_labels() {
        let template = ieee_bibliography();

        // Find volume component and check it has "vol." prefix
        let volume = template.iter().find(
            |c| matches!(c, TemplateComponent::Number(n) if n.number == NumberVariable::Volume),
        );
        assert!(volume.is_some());

        match volume.unwrap() {
            TemplateComponent::Number(n) => {
                assert_eq!(n.rendering.prefix, Some("vol. ".to_string()));
            }
            _ => unreachable!(),
        }
    }
}
