/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Style presets for common formatting patterns.
//!
//! Presets are named bundles of configuration that encode common patterns from major
//! citation styles. Instead of inheriting from a parent style, styles can compose
//! presets for different concerns (contributors, dates, titles).
//!
//! ## Usage
//!
//! Style authors can use preset names for defaults and override individual settings:
//!
//! ```yaml
//! options:
//!   contributors: apa
//!   dates: year-only
//!   titles: apa
//! ```
//!
//! ## Preset Expansion
//!
//! Each preset expands to concrete `Config` values. The style author can:
//! 1. Use a preset name for defaults
//! 2. Override individual fields as needed
//! 3. Skip presets entirely and specify everything explicitly

use crate::options::{
    AndOptions, ContributorConfig, DateConfig, DelimiterPrecedesLast, DisplayAsSort, MonthFormat,
    ShortenListOptions, Substitute, SubstituteKey, TitleRendering, TitlesConfig,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contributor formatting presets.
///
/// Each preset encodes the contributor formatting conventions for a major citation
/// style or style family. Use doc comments to describe the visual behavior so
/// style authors can choose the right preset without knowing style guide names.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ContributorPreset {
    /// First author family-first, "&" symbol, et al. after 20 authors.
    /// Example: "Smith, J. D., & Jones, M. K."
    Apa,
    /// First author family-first, "and" text, no serial comma.
    /// Example: "Smith, John D. and Mary K. Jones"
    Chicago,
    /// All authors family-first, no conjunction.
    /// Example: "Smith JD, Jones MK, Brown AB"
    Vancouver,
    /// All authors family-first, "and" before last.
    /// Example: "J. D. Smith, M. K. Jones, and A. B. Brown"
    Ieee,
    /// First author family-first, "and" text (Elsevier/Springer variant).
    /// Example: "Smith, J.D. and Jones, M.K."
    Harvard,
}

impl ContributorPreset {
    /// Convert this preset to a concrete `ContributorConfig`.
    pub fn config(&self) -> ContributorConfig {
        match self {
            ContributorPreset::Apa => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Symbol),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(". ".to_string()),
                shorten: Some(ShortenListOptions {
                    min: 21,
                    use_first: 19,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::Chicago => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Text),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Contextual),
                ..Default::default()
            },
            ContributorPreset::Vancouver => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                initialize_with: Some("".to_string()),
                shorten: Some(ShortenListOptions {
                    min: 7,
                    use_first: 6,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::Ieee => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::None), // Given-first format
                and: Some(AndOptions::Text),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(". ".to_string()),
                ..Default::default()
            },
            ContributorPreset::Harvard => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Text),
                delimiter: Some(", ".to_string()),
                initialize_with: Some(".".to_string()),
                ..Default::default()
            },
        }
    }
}

/// Date formatting presets.
///
/// Each preset defines how dates are displayed in citations and bibliographies.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DatePreset {
    /// Year only: (2024)
    YearOnly,
    /// Full date with locale-appropriate format: (January 15, 2024)
    Full,
    /// Month and year only: (January 2024)
    MonthYear,
    /// ISO 8601 format: (2024-01-15)
    Iso,
}

impl DatePreset {
    /// Convert this preset to a concrete `DateConfig`.
    pub fn config(&self) -> DateConfig {
        match self {
            DatePreset::YearOnly => DateConfig {
                month: MonthFormat::Long, // Not used when year-only, but kept for consistency
                _extra: HashMap::new(),
            },
            DatePreset::Full => DateConfig {
                month: MonthFormat::Long,
                _extra: HashMap::new(),
            },
            DatePreset::MonthYear => DateConfig {
                month: MonthFormat::Long,
                _extra: HashMap::new(),
            },
            DatePreset::Iso => DateConfig {
                month: MonthFormat::Numeric,
                _extra: HashMap::new(),
            },
        }
    }
}

/// Title formatting presets.
///
/// Each preset defines how different types of titles (articles, books, journals)
/// are formatted. Presets typically differ in whether titles are quoted, italicized,
/// or plain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TitlePreset {
    /// APA style: article titles plain, book/journal titles italic.
    /// Example: Article title. *Book Title*. *Journal Title*.
    Apa,
    /// Chicago style: article titles quoted, book/journal titles italic.
    /// Example: "Article Title." *Book Title*. *Journal Title*.
    Chicago,
    /// IEEE style: article titles quoted, book/journal titles italic.
    /// Example: "Article title," *Book Title*. *Journal Title*.
    Ieee,
    /// Scientific/Vancouver style: all titles plain (no formatting).
    /// Example: Article title. Book title. Journal title.
    Scientific,
}

impl TitlePreset {
    /// Convert this preset to a concrete `TitlesConfig`.
    pub fn config(&self) -> TitlesConfig {
        match self {
            TitlePreset::Apa => TitlesConfig {
                component: Some(TitleRendering::default()), // Plain
                monograph: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                periodical: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            TitlePreset::Chicago => TitlesConfig {
                component: Some(TitleRendering {
                    quote: Some(true),
                    ..Default::default()
                }),
                monograph: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                periodical: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            TitlePreset::Ieee => TitlesConfig {
                component: Some(TitleRendering {
                    quote: Some(true),
                    ..Default::default()
                }),
                monograph: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                periodical: Some(TitleRendering {
                    emph: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            TitlePreset::Scientific => TitlesConfig {
                component: Some(TitleRendering::default()),
                monograph: Some(TitleRendering::default()),
                periodical: Some(TitleRendering::default()),
                ..Default::default()
            },
        }
    }
}

/// Substitute presets for author substitution fallback logic.
///
/// These presets define the order in which fields are tried when the primary
/// author is missing. Most styles follow the standard order, but some have
/// variations.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SubstitutePreset {
    /// Standard substitution order: Editor → Title → Translator.
    /// Used by most citation styles (APA, Chicago, etc.).
    Standard,
    /// Editor-first: Editor → Translator → Title.
    /// Prioritizes contributors over title.
    EditorFirst,
    /// Title-first: Title → Editor → Translator.
    /// Used when anonymous works should show title prominently.
    TitleFirst,
}

impl SubstitutePreset {
    /// Convert this preset to a concrete `Substitute`.
    pub fn config(&self) -> Substitute {
        match self {
            SubstitutePreset::Standard => Substitute {
                contributor_role_form: None,
                template: vec![
                    SubstituteKey::Editor,
                    SubstituteKey::Title,
                    SubstituteKey::Translator,
                ],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorFirst => Substitute {
                contributor_role_form: None,
                template: vec![
                    SubstituteKey::Editor,
                    SubstituteKey::Translator,
                    SubstituteKey::Title,
                ],
                overrides: HashMap::new(),
            },
            SubstitutePreset::TitleFirst => Substitute {
                contributor_role_form: None,
                template: vec![
                    SubstituteKey::Title,
                    SubstituteKey::Editor,
                    SubstituteKey::Translator,
                ],
                overrides: HashMap::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contributor_preset_apa() {
        let config = ContributorPreset::Apa.config();
        assert_eq!(config.and, Some(AndOptions::Symbol));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::First));
        let shorten = config.shorten.unwrap();
        assert_eq!(shorten.min, 21);
        assert_eq!(shorten.use_first, 19);
    }

    #[test]
    fn test_contributor_preset_chicago() {
        let config = ContributorPreset::Chicago.config();
        assert_eq!(config.and, Some(AndOptions::Text));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::First));
    }

    #[test]
    fn test_contributor_preset_vancouver() {
        let config = ContributorPreset::Vancouver.config();
        assert_eq!(config.and, Some(AndOptions::None));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::All));
    }

    #[test]
    fn test_date_preset_year_only() {
        let config = DatePreset::YearOnly.config();
        assert_eq!(config.month, MonthFormat::Long);
    }

    #[test]
    fn test_date_preset_iso() {
        let config = DatePreset::Iso.config();
        assert_eq!(config.month, MonthFormat::Numeric);
    }

    #[test]
    fn test_title_preset_apa() {
        let config = TitlePreset::Apa.config();
        // Component titles should be plain (no formatting)
        let component = config.component.unwrap();
        assert!(component.quote.is_none() || component.quote == Some(false));
        // Monograph titles should be italic
        let monograph = config.monograph.unwrap();
        assert_eq!(monograph.emph, Some(true));
    }

    #[test]
    fn test_title_preset_chicago() {
        let config = TitlePreset::Chicago.config();
        // Component titles should be quoted
        let component = config.component.unwrap();
        assert_eq!(component.quote, Some(true));
        // Monograph titles should be italic
        let monograph = config.monograph.unwrap();
        assert_eq!(monograph.emph, Some(true));
    }

    #[test]
    fn test_preset_yaml_roundtrip() {
        let yaml = r#"apa"#;
        let preset: ContributorPreset = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(preset, ContributorPreset::Apa);

        let serialized = serde_yaml::to_string(&preset).unwrap();
        assert!(serialized.contains("apa"));
    }

    #[test]
    fn test_all_presets_serialize() {
        // Ensure all presets can be serialized/deserialized
        let contributor_presets = vec![
            ContributorPreset::Apa,
            ContributorPreset::Chicago,
            ContributorPreset::Vancouver,
            ContributorPreset::Ieee,
            ContributorPreset::Harvard,
        ];
        for preset in contributor_presets {
            let yaml = serde_yaml::to_string(&preset).unwrap();
            let _: ContributorPreset = serde_yaml::from_str(&yaml).unwrap();
        }

        let date_presets = vec![
            DatePreset::YearOnly,
            DatePreset::Full,
            DatePreset::MonthYear,
            DatePreset::Iso,
        ];
        for preset in date_presets {
            let yaml = serde_yaml::to_string(&preset).unwrap();
            let _: DatePreset = serde_yaml::from_str(&yaml).unwrap();
        }

        let title_presets = vec![
            TitlePreset::Apa,
            TitlePreset::Chicago,
            TitlePreset::Ieee,
            TitlePreset::Scientific,
        ];
        for preset in title_presets {
            let yaml = serde_yaml::to_string(&preset).unwrap();
            let _: TitlePreset = serde_yaml::from_str(&yaml).unwrap();
        }

        let substitute_presets = vec![
            SubstitutePreset::Standard,
            SubstitutePreset::EditorFirst,
            SubstitutePreset::TitleFirst,
        ];
        for preset in substitute_presets {
            let yaml = serde_yaml::to_string(&preset).unwrap();
            let _: SubstitutePreset = serde_yaml::from_str(&yaml).unwrap();
        }
    }

    #[test]
    fn test_substitute_preset_standard() {
        let config = SubstitutePreset::Standard.config();
        assert_eq!(
            config.template,
            vec![
                SubstituteKey::Editor,
                SubstituteKey::Title,
                SubstituteKey::Translator,
            ]
        );
    }

    #[test]
    fn test_substitute_preset_title_first() {
        let config = SubstitutePreset::TitleFirst.config();
        assert_eq!(config.template[0], SubstituteKey::Title);
    }
}
