/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Embedded priority templates for common citation styles.

pub mod apa;
pub mod chicago;
pub mod harvard;
pub mod ieee;
pub mod locales;
pub mod numeric;
pub mod styles;
pub mod vancouver;

use crate::template::TemplateComponent;
use std::collections::HashMap;

// Re-export for original API compatibility
pub use apa::bibliography as apa_bibliography;
pub use apa::citation as apa_citation;
pub use chicago::author_date_bibliography as chicago_author_date_bibliography;
pub use chicago::author_date_citation as chicago_author_date_citation;
pub use harvard::bibliography as harvard_bibliography;
pub use harvard::citation as harvard_citation;
pub use ieee::bibliography as ieee_bibliography;
pub use ieee::citation as ieee_citation;
pub use locales::{EMBEDDED_LOCALE_IDS, get_locale_bytes};
pub use numeric::citation as numeric_citation;
pub use styles::{EMBEDDED_STYLE_NAMES, get_embedded_style};
pub use vancouver::bibliography as vancouver_bibliography;
pub use vancouver::citation as vancouver_citation;

/// Get all available embedded citation templates.
pub fn citation_templates() -> HashMap<&'static str, Vec<TemplateComponent>> {
    let mut map = HashMap::new();
    map.insert("apa", apa_citation());
    map.insert("chicago-author-date", chicago_author_date_citation());
    map.insert("vancouver", vancouver_citation());
    map.insert("ieee", ieee_citation());
    map.insert("harvard", harvard_citation());
    map.insert("numeric-citation", numeric_citation());
    map
}

/// Get all available embedded bibliography templates.
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
    use crate::options::AndOptions;
    use crate::template::{
        ContributorForm, ContributorRole, DateForm, DateVariable, NumberVariable,
        TemplateComponent, WrapPunctuation,
    };

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
        assert!(templates.contains_key("numeric-citation"));
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
