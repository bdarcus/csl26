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
    AndOptions, ContributorConfig, DateConfig, DelimiterPrecedesLast, DemoteNonDroppingParticle,
    DisplayAsSort, MonthFormat, ShortenListOptions, Substitute, SubstituteKey, TitleRendering,
    TitlesConfig,
};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contributor formatting presets.
///
/// Each preset encodes the contributor formatting conventions for a major citation
/// style or style family. Use doc comments to describe the visual behavior so
/// style authors can choose the right preset without knowing style guide names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ContributorPreset {
    /// First author family-first, "&" symbol, et al. after 20 authors,
    /// initials with period-space, comma before "&".
    /// Example: "Smith, J. D., & Jones, M. K."
    Apa,
    /// First author family-first, "and" text, contextual serial comma,
    /// full given names (no initials).
    /// Example: "Smith, John D., and Mary K. Jones"
    Chicago,
    /// All authors family-first, no conjunction, compact initials (no
    /// period/space), et al. after 6 of 7+.
    /// Example: "Smith JD, Jones MK, Brown AB"
    Vancouver,
    /// Given-first format, "and" text, initials with period-space,
    /// comma before "and".
    /// Example: "J. D. Smith, M. K. Jones, and A. B. Brown"
    Ieee,
    /// All authors family-first, "and" text, compact initials (period,
    /// no space), comma before "and".
    /// Example: "Smith, J.D., Jones, M.K., and Brown, A.B."
    Harvard,
    /// All authors family-first, no conjunction, compact initials (no
    /// period/space), space sort-separator, et al. after 3 of 5+.
    /// Example: "Smith JD, Jones MK, Brown AB"
    Springer,
    /// Numeric compact author list for journal-heavy corpora:
    /// all family-first, no conjunction, sort-only particle demotion,
    /// space sort-separator, et al. after 6 of 7+.
    NumericCompact,
    /// Numeric medium author list variant:
    /// same as `numeric-compact`, but et al. after 3 of 4+.
    NumericMedium,
    /// Numeric tight: all family-first, no initials, et al. after 3 of 7+.
    /// Tighter than `numeric-compact` (use-first: 3 vs 6).
    /// Example: "Smith J, Jones M, Brown A, et al."
    NumericTight,
    /// Numeric large: all family-first, no initials, et al. after 10 of 11+.
    /// For biomedical journals that show nearly all authors.
    /// Example: "Smith J, Jones M, Brown A, [10 authors], et al."
    NumericLarge,
    /// Annual Reviews style: all family-first, no initials, et al. after 5 of 7+,
    /// particle demotion never. Distinguishable by "never" demote policy.
    /// Example: "van der Berg J, Smith M, Jones A, Brown B, White C, et al."
    AnnualReviews,
    /// Math/physics author-date: all family-first, period initial (no trailing
    /// space), comma sort-separator, no conjunction. Used across Springer
    /// math/physics and related author-date journals.
    /// Example: "Smith, J., Jones, M., Brown, A."
    MathPhys,
    /// Social science first-author inversion: first author family-first,
    /// remaining authors given-first, period-space initials, comma
    /// sort-separator, no conjunction. Common in sociology and civil engineering.
    /// Example: "Smith, J. D., M. K. Jones, A. B. Brown"
    SocSciFirst,
    /// Physics numeric given-first: no author inversion, period-space initial,
    /// no conjunction, sort-only particle demotion. Used by numeric physics
    /// journals like APS.
    /// Example: "J. Smith, M. Jones, A. Brown"
    PhysicsNumeric,
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
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::Text),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(".".to_string()),
                ..Default::default()
            },
            ContributorPreset::Springer => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                sort_separator: Some(" ".to_string()),
                shorten: Some(ShortenListOptions {
                    min: 5,
                    use_first: 3,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::NumericCompact => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                shorten: Some(ShortenListOptions {
                    min: 7,
                    use_first: 6,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::NumericMedium => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                shorten: Some(ShortenListOptions {
                    min: 4,
                    use_first: 3,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::NumericTight => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                shorten: Some(ShortenListOptions {
                    min: 7,
                    use_first: 3,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::NumericLarge => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                shorten: Some(ShortenListOptions {
                    min: 11,
                    use_first: 10,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::AnnualReviews => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Never),
                initialize_with: Some("".to_string()),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::Never),
                shorten: Some(ShortenListOptions {
                    min: 7,
                    use_first: 5,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::MathPhys => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(".".to_string()),
                sort_separator: Some(", ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
            ContributorPreset::SocSciFirst => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(". ".to_string()),
                sort_separator: Some(", ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
            ContributorPreset::PhysicsNumeric => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::None),
                and: Some(AndOptions::None),
                delimiter: Some(", ".to_string()),
                initialize_with: Some(". ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
        }
    }
}

/// Date formatting presets.
///
/// Each preset defines how dates are displayed in citations and bibliographies,
/// including month format, EDTF uncertainty/approximation markers, and range
/// delimiters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum DatePreset {
    /// Long month names, EDTF markers, en-dash ranges.
    /// Example: "January 15, 2024", "ca. 2024", "2024?"
    Long,
    /// Short month names, EDTF markers, en-dash ranges.
    /// Example: "Jan 15, 2024"
    Short,
    /// Numeric months, EDTF markers, en-dash ranges.
    /// Example: "1/15/2024"
    Numeric,
    /// ISO 8601 numeric format, no EDTF markers.
    /// Example: "2024-01-15"
    Iso,
}

impl DatePreset {
    /// Convert this preset to a concrete `DateConfig`.
    pub fn config(&self) -> DateConfig {
        match self {
            DatePreset::Long => DateConfig {
                month: MonthFormat::Long,
                ..Default::default()
            },
            DatePreset::Short => DateConfig {
                month: MonthFormat::Short,
                ..Default::default()
            },
            DatePreset::Numeric => DateConfig {
                month: MonthFormat::Numeric,
                ..Default::default()
            },
            DatePreset::Iso => DateConfig {
                month: MonthFormat::Numeric,
                uncertainty_marker: None,
                approximation_marker: None,
                ..Default::default()
            },
        }
    }
}

/// Title formatting presets.
///
/// Each preset defines how different types of titles (articles, books, journals)
/// are formatted. Presets typically differ in whether titles are quoted, italicized,
/// or plain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
    /// Humanities style: monographs, periodicals, and serials all italic,
    /// articles plain. Common in geography, history, and social sciences.
    /// Example: Article title. *Book Title*. *Journal Title*. *Series Title*.
    Humanities,
    /// Journal-focused emphasis: periodicals and serials italic,
    /// monographs plain.
    JournalEmphasis,
    /// Scientific/Vancouver style: all titles plain (no formatting).
    /// Example: Article title. Book title. Journal title.
    Scientific,
}

impl TitlePreset {
    /// Convert this preset to a concrete `TitlesConfig`.
    pub fn config(&self) -> TitlesConfig {
        let emph_rendering = TitleRendering {
            emph: Some(true),
            ..Default::default()
        };
        match self {
            TitlePreset::Apa => TitlesConfig {
                component: Some(TitleRendering::default()),
                monograph: Some(emph_rendering.clone()),
                periodical: Some(emph_rendering),
                ..Default::default()
            },
            TitlePreset::Chicago | TitlePreset::Ieee => TitlesConfig {
                component: Some(TitleRendering {
                    quote: Some(true),
                    ..Default::default()
                }),
                monograph: Some(emph_rendering.clone()),
                periodical: Some(emph_rendering),
                ..Default::default()
            },
            TitlePreset::Humanities => TitlesConfig {
                component: Some(TitleRendering::default()),
                monograph: Some(emph_rendering.clone()),
                periodical: Some(emph_rendering.clone()),
                serial: Some(emph_rendering),
                ..Default::default()
            },
            TitlePreset::JournalEmphasis => TitlesConfig {
                component: Some(TitleRendering::default()),
                periodical: Some(emph_rendering.clone()),
                serial: Some(emph_rendering),
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
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
    /// Editor only with short role labels.
    EditorShort,
    /// Editor only with long role labels.
    EditorLong,
    /// Editor then translator with short role labels.
    EditorTranslatorShort,
    /// Editor then translator with long role labels.
    EditorTranslatorLong,
    /// Editor then title with short role labels.
    EditorTitleShort,
    /// Editor then title with long role labels.
    EditorTitleLong,
    /// Editor then translator then title with short role labels.
    EditorTranslatorTitleShort,
    /// Editor then translator then title with long role labels.
    EditorTranslatorTitleLong,
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
            SubstitutePreset::EditorShort => Substitute {
                contributor_role_form: Some("short".to_string()),
                template: vec![SubstituteKey::Editor],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorLong => Substitute {
                contributor_role_form: Some("long".to_string()),
                template: vec![SubstituteKey::Editor],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorTranslatorShort => Substitute {
                contributor_role_form: Some("short".to_string()),
                template: vec![SubstituteKey::Editor, SubstituteKey::Translator],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorTranslatorLong => Substitute {
                contributor_role_form: Some("long".to_string()),
                template: vec![SubstituteKey::Editor, SubstituteKey::Translator],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorTitleShort => Substitute {
                contributor_role_form: Some("short".to_string()),
                template: vec![SubstituteKey::Editor, SubstituteKey::Title],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorTitleLong => Substitute {
                contributor_role_form: Some("long".to_string()),
                template: vec![SubstituteKey::Editor, SubstituteKey::Title],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorTranslatorTitleShort => Substitute {
                contributor_role_form: Some("short".to_string()),
                template: vec![
                    SubstituteKey::Editor,
                    SubstituteKey::Translator,
                    SubstituteKey::Title,
                ],
                overrides: HashMap::new(),
            },
            SubstitutePreset::EditorTranslatorTitleLong => Substitute {
                contributor_role_form: Some("long".to_string()),
                template: vec![
                    SubstituteKey::Editor,
                    SubstituteKey::Translator,
                    SubstituteKey::Title,
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
    fn test_contributor_preset_springer() {
        let config = ContributorPreset::Springer.config();
        assert_eq!(config.and, Some(AndOptions::None));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::All));
        assert_eq!(config.sort_separator, Some(" ".to_string()));
        let shorten = config.shorten.unwrap();
        assert_eq!(shorten.min, 5);
        assert_eq!(shorten.use_first, 3);
    }

    #[test]
    fn test_contributor_preset_numeric_compact() {
        let config = ContributorPreset::NumericCompact.config();
        assert_eq!(config.and, Some(AndOptions::None));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::All));
        assert_eq!(config.sort_separator, Some(" ".to_string()));
        assert_eq!(
            config.demote_non_dropping_particle,
            Some(DemoteNonDroppingParticle::SortOnly)
        );
        let shorten = config.shorten.unwrap();
        assert_eq!(shorten.min, 7);
        assert_eq!(shorten.use_first, 6);
    }

    #[test]
    fn test_date_preset_long() {
        let config = DatePreset::Long.config();
        assert_eq!(config.month, MonthFormat::Long);
        assert!(config.uncertainty_marker.is_some());
    }

    #[test]
    fn test_date_preset_iso() {
        let config = DatePreset::Iso.config();
        assert_eq!(config.month, MonthFormat::Numeric);
        // ISO preset suppresses EDTF markers
        assert!(config.uncertainty_marker.is_none());
        assert!(config.approximation_marker.is_none());
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
            ContributorPreset::Springer,
            ContributorPreset::NumericCompact,
            ContributorPreset::NumericMedium,
            ContributorPreset::NumericTight,
            ContributorPreset::NumericLarge,
            ContributorPreset::AnnualReviews,
            ContributorPreset::MathPhys,
            ContributorPreset::SocSciFirst,
            ContributorPreset::PhysicsNumeric,
        ];
        for preset in contributor_presets {
            let yaml = serde_yaml::to_string(&preset).unwrap();
            let _: ContributorPreset = serde_yaml::from_str(&yaml).unwrap();
        }

        let date_presets = vec![
            DatePreset::Long,
            DatePreset::Short,
            DatePreset::Numeric,
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
            TitlePreset::Humanities,
            TitlePreset::JournalEmphasis,
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
            SubstitutePreset::EditorShort,
            SubstitutePreset::EditorLong,
            SubstitutePreset::EditorTranslatorShort,
            SubstitutePreset::EditorTranslatorLong,
            SubstitutePreset::EditorTitleShort,
            SubstitutePreset::EditorTitleLong,
            SubstitutePreset::EditorTranslatorTitleShort,
            SubstitutePreset::EditorTranslatorTitleLong,
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
