/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

//! Detects when extracted configuration matches known presets.
//!
//! This module implements preset detection for Phase 3 of the style aliasing
//! design. When migrating CSL 1.0 styles, it compares extracted configuration
//! to known preset patterns and emits preset names instead of expanded configs.
//!
//! ## Detection Strategy
//!
//! For each configuration type, we check if the essential fields match a known
//! preset. We use "fuzzy matching" - not all fields need to match exactly, just
//! the characteristic ones that define the preset.
//!
//! See `.agent/design/STYLE_ALIASING.md` for design context.

use csln_core::options::{
    AndOptions, Config, ContributorConfig, DateConfig, DelimiterPrecedesLast,
    DemoteNonDroppingParticle, DisplayAsSort, TitlesConfig,
};
use csln_core::presets::{ContributorPreset, DatePreset, TitlePreset};

/// Holistic style presets that combine multiple configuration aspects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StylePreset {
    Apa,
    Chicago,
    Ieee,
    Elsevier,
    Vancouver,
    Harvard,
}

/// Detects a holistic style preset from a full configuration.
pub fn detect_style_preset(config: &Config) -> Option<StylePreset> {
    let cp = config
        .contributors
        .as_ref()
        .and_then(detect_contributor_preset);
    let tp = config.titles.as_ref().and_then(detect_title_preset);

    match (cp, tp) {
        (Some(ContributorPreset::Apa), Some(TitlePreset::Apa)) => Some(StylePreset::Apa),
        (Some(ContributorPreset::Chicago), Some(TitlePreset::Chicago)) => {
            Some(StylePreset::Chicago)
        }
        (Some(ContributorPreset::Ieee), Some(TitlePreset::Chicago)) => Some(StylePreset::Ieee),
        (Some(ContributorPreset::Vancouver), _) => Some(StylePreset::Vancouver),
        (Some(ContributorPreset::Harvard), _) => Some(StylePreset::Harvard),
        _ => None,
    }
}

/// Detects if a `ContributorConfig` matches a known preset.
///
/// Returns the matching preset if found, or `None` if the config is custom.
/// The detection is "fuzzy" - we check characteristic fields that define each
/// preset, not every single field.
pub fn detect_contributor_preset(config: &ContributorConfig) -> Option<ContributorPreset> {
    fn shorten_matches(config: &ContributorConfig, min: u8, use_first: u8) -> bool {
        config
            .shorten
            .as_ref()
            .is_some_and(|shorten| shorten.min == min && shorten.use_first == use_first)
    }

    let and_none = config.and == Some(AndOptions::None) || config.and.is_none();

    // APA: symbol "and", first author inverted, high et-al threshold
    if config.and == Some(AndOptions::Symbol)
        && config.display_as_sort == Some(DisplayAsSort::First)
    {
        // Check for APA's characteristic et-al threshold (21 min, 19 use-first)
        if let Some(ref shorten) = config.shorten
            && shorten.min >= 20
        {
            return Some(ContributorPreset::Apa);
        }
        // Even without high threshold, symbol "and" + first-sort is APA-like
        return Some(ContributorPreset::Apa);
    }

    // All family-first, no conjunction (Vancouver/Springer/numeric variants)
    if config.display_as_sort == Some(DisplayAsSort::All) && and_none {
        if config.demote_non_dropping_particle == Some(DemoteNonDroppingParticle::Never)
            && config.sort_separator.as_deref() == Some(" ")
            && config.initialize_with.as_deref() == Some("")
            && config.shorten.is_none()
        {
            return Some(ContributorPreset::NumericAllAuthors);
        }

        if config.demote_non_dropping_particle == Some(DemoteNonDroppingParticle::Never)
            && config.sort_separator.as_deref() == Some(" ")
            && shorten_matches(config, 7, 5)
        {
            return Some(ContributorPreset::AnnualReviews);
        }

        if config.demote_non_dropping_particle == Some(DemoteNonDroppingParticle::SortOnly)
            && config.sort_separator.as_deref() == Some(" ")
            && config.initialize_with.as_deref() == Some("")
        {
            if shorten_matches(config, 7, 6) {
                return Some(ContributorPreset::NumericCompact);
            }
            if shorten_matches(config, 4, 3) {
                return Some(ContributorPreset::NumericMedium);
            }
            if shorten_matches(config, 7, 3) {
                return Some(ContributorPreset::NumericTight);
            }
            if shorten_matches(config, 11, 10) {
                return Some(ContributorPreset::NumericLarge);
            }
        }

        if config.demote_non_dropping_particle == Some(DemoteNonDroppingParticle::SortOnly)
            && config.sort_separator.as_deref() == Some(", ")
            && config.initialize_with.as_deref() == Some(".")
            && config.delimiter_precedes_last == Some(DelimiterPrecedesLast::Always)
            && config.shorten.is_none()
        {
            return Some(ContributorPreset::MathPhys);
        }

        if config.sort_separator.as_deref() == Some(" ") {
            return Some(ContributorPreset::Springer);
        }
        return Some(ContributorPreset::Vancouver);
    }

    // Given-first, no conjunction numeric variants.
    if config.display_as_sort == Some(DisplayAsSort::None) && and_none {
        if config.initialize_with.as_deref() == Some(".")
            && config.delimiter.as_deref() == Some(", ")
            && config.delimiter_precedes_last == Some(DelimiterPrecedesLast::Always)
            && config.demote_non_dropping_particle == Some(DemoteNonDroppingParticle::SortOnly)
        {
            return Some(ContributorPreset::NumericGivenDot);
        }

        if config.initialize_with.as_deref() == Some(". ")
            && config.demote_non_dropping_particle == Some(DemoteNonDroppingParticle::SortOnly)
        {
            return Some(ContributorPreset::PhysicsNumeric);
        }
    }

    // IEEE: given-first format, text "and"
    if config.display_as_sort == Some(DisplayAsSort::None) && config.and == Some(AndOptions::Text) {
        return Some(ContributorPreset::Ieee);
    }

    // Harvard: all inverted, text "and"
    if config.display_as_sort == Some(DisplayAsSort::All) && config.and == Some(AndOptions::Text) {
        return Some(ContributorPreset::Harvard);
    }

    // Chicago: first inverted, text "and", contextual comma
    if config.display_as_sort == Some(DisplayAsSort::First)
        && config.and == Some(AndOptions::Text)
        && let Some(dpl) = &config.delimiter_precedes_last
        && *dpl == csln_core::options::DelimiterPrecedesLast::Contextual
    {
        return Some(ContributorPreset::Chicago);
    }

    None
}

/// Detects if a `TitlesConfig` matches a known preset.
///
/// Returns the matching preset if found, or `None` if the config is custom.
pub fn detect_title_preset(config: &TitlesConfig) -> Option<TitlePreset> {
    let component_quoted = config
        .component
        .as_ref()
        .and_then(|c| c.quote)
        .unwrap_or(false);
    let monograph_emph = config
        .monograph
        .as_ref()
        .and_then(|m| m.emph)
        .unwrap_or(false);
    let periodical_emph = config
        .periodical
        .as_ref()
        .and_then(|p| p.emph)
        .unwrap_or(false);

    // Scientific: all plain (no formatting)
    if !component_quoted && !monograph_emph && !periodical_emph {
        return Some(TitlePreset::Scientific);
    }

    // APA-family: component plain, monograph/periodical italic
    if !component_quoted && monograph_emph && periodical_emph {
        return Some(TitlePreset::Apa);
    }

    // Chicago/IEEE: component quoted, monograph/periodical italic
    if component_quoted && monograph_emph && periodical_emph {
        // Both Chicago and IEEE follow this pattern - default to Chicago
        return Some(TitlePreset::Chicago);
    }

    None
}

/// Detects if a `DateConfig` matches a known preset.
///
/// Returns the matching preset if found, or `None` if the config is custom.
pub fn detect_date_preset(config: &DateConfig) -> Option<DatePreset> {
    use csln_core::options::MonthFormat;

    match config.month {
        MonthFormat::Numeric => Some(DatePreset::Numeric),
        MonthFormat::Short => Some(DatePreset::Short),
        MonthFormat::Long => Some(DatePreset::Long),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csln_core::options::{DelimiterPrecedesLast, ShortenListOptions, TitleRendering};

    #[test]
    fn test_detect_apa_contributor() {
        // APA has symbol "and", first author inverted, high et-al threshold
        let config = ContributorConfig {
            and: Some(AndOptions::Symbol),
            display_as_sort: Some(DisplayAsSort::First),
            shorten: Some(ShortenListOptions {
                min: 21,
                use_first: 19,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::Apa)
        );
    }

    #[test]
    fn test_detect_vancouver_contributor() {
        // Vancouver has all authors inverted, no "and"
        let config = ContributorConfig {
            and: Some(AndOptions::None),
            display_as_sort: Some(DisplayAsSort::All),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::Vancouver)
        );
    }

    #[test]
    fn test_detect_chicago_contributor() {
        // Chicago has first author inverted, text "and", contextual comma
        let config = ContributorConfig {
            and: Some(AndOptions::Text),
            display_as_sort: Some(DisplayAsSort::First),
            delimiter_precedes_last: Some(DelimiterPrecedesLast::Contextual),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::Chicago)
        );
    }

    #[test]
    fn test_detect_harvard_contributor() {
        // Harvard: all inverted, text "and"
        let config = ContributorConfig {
            and: Some(AndOptions::Text),
            display_as_sort: Some(DisplayAsSort::All),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::Harvard)
        );
    }

    #[test]
    fn test_detect_springer_contributor() {
        // Springer: all inverted, no "and", space sort-separator
        let config = ContributorConfig {
            and: Some(AndOptions::None),
            display_as_sort: Some(DisplayAsSort::All),
            sort_separator: Some(" ".to_string()),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::Springer)
        );
    }

    #[test]
    fn test_detect_ieee_contributor() {
        // IEEE has given-first format, text "and"
        let config = ContributorConfig {
            and: Some(AndOptions::Text),
            display_as_sort: Some(DisplayAsSort::None),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::Ieee)
        );
    }

    #[test]
    fn test_detect_numeric_all_authors_contributor() {
        let config = ContributorConfig {
            and: Some(AndOptions::None),
            display_as_sort: Some(DisplayAsSort::All),
            initialize_with: Some("".to_string()),
            sort_separator: Some(" ".to_string()),
            demote_non_dropping_particle: Some(DemoteNonDroppingParticle::Never),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::NumericAllAuthors)
        );
    }

    #[test]
    fn test_detect_numeric_medium_contributor() {
        let config = ContributorConfig {
            and: Some(AndOptions::None),
            display_as_sort: Some(DisplayAsSort::All),
            initialize_with: Some("".to_string()),
            sort_separator: Some(" ".to_string()),
            demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
            shorten: Some(ShortenListOptions {
                min: 4,
                use_first: 3,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::NumericMedium)
        );
    }

    #[test]
    fn test_detect_numeric_given_dot_contributor() {
        let config = ContributorConfig {
            and: Some(AndOptions::None),
            display_as_sort: Some(DisplayAsSort::None),
            initialize_with: Some(".".to_string()),
            delimiter: Some(", ".to_string()),
            delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
            demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
            ..Default::default()
        };
        assert_eq!(
            detect_contributor_preset(&config),
            Some(ContributorPreset::NumericGivenDot)
        );
    }

    #[test]
    fn test_detect_apa_title() {
        // APA: component plain, monograph/periodical italic
        let config = TitlesConfig {
            component: Some(TitleRendering::default()),
            monograph: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            periodical: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(detect_title_preset(&config), Some(TitlePreset::Apa));
    }

    #[test]
    fn test_detect_chicago_title() {
        // Chicago: component quoted, monograph/periodical italic
        let config = TitlesConfig {
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
        };
        assert_eq!(detect_title_preset(&config), Some(TitlePreset::Chicago));
    }

    #[test]
    fn test_detect_scientific_title() {
        // Scientific: all plain
        let config = TitlesConfig {
            component: Some(TitleRendering::default()),
            monograph: Some(TitleRendering::default()),
            periodical: Some(TitleRendering::default()),
            ..Default::default()
        };
        assert_eq!(detect_title_preset(&config), Some(TitlePreset::Scientific));
    }

    #[test]
    fn test_detect_numeric_date() {
        use csln_core::options::MonthFormat;

        let config = DateConfig {
            month: MonthFormat::Numeric,
            ..Default::default()
        };
        assert_eq!(detect_date_preset(&config), Some(DatePreset::Numeric));
    }
}
