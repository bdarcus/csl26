/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Detects narrow formatting families for migration fixups.
//!
//! This module does not determine style taxonomy or wrapper lineage. It only
//! detects narrow formatting families used by migration-time fixups.
//!
//! ## Detection Strategy
//!
//! For each configuration type, we check if the essential fields match a known
//! preset. We use "fuzzy matching" - not all fields need to match exactly, just
//! the characteristic ones that define the preset.
//!
//! See `.agent/design/STYLE_ALIASING.md` for design context.

use citum_schema::options::{
    AndOptions, Config, ContributorConfig, DateConfig, DelimiterPrecedesLast,
    DemoteNonDroppingParticle, DisplayAsSort, Substitute, TitlesConfig,
};
use citum_schema::presets::{ContributorPreset, DatePreset, SubstitutePreset, TitlePreset};

/// Narrow formatting families used only by migration fixups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixupFamily {
    Apa,
    Chicago,
    Ieee,
    Vancouver,
    Harvard,
}

/// Detect the formatting family used by migration fixups from a full config.
pub fn detect_fixup_family(config: &Config) -> Option<FixupFamily> {
    let cp = config
        .contributors
        .as_ref()
        .and_then(detect_contributor_preset);
    let tp = config.titles.as_ref().and_then(detect_title_preset);

    match (cp, tp) {
        (Some(ContributorPreset::Apa), Some(TitlePreset::Apa)) => Some(FixupFamily::Apa),
        (Some(ContributorPreset::Chicago), Some(TitlePreset::Chicago)) => {
            Some(FixupFamily::Chicago)
        }
        (Some(ContributorPreset::Ieee), Some(TitlePreset::Chicago)) => Some(FixupFamily::Ieee),
        (Some(ContributorPreset::Vancouver), _) => Some(FixupFamily::Vancouver),
        (Some(ContributorPreset::Harvard), _) => Some(FixupFamily::Harvard),
        _ => None,
    }
}

/// Backward-compatible alias for the historical fixup family name.
#[deprecated(note = "use FixupFamily instead; this type models fixup families, not taxonomy")]
pub type StyleBase = FixupFamily;

/// Backward-compatible wrapper for the historical fixup family detector.
#[deprecated(note = "use detect_fixup_family instead")]
#[allow(
    deprecated,
    reason = "compatibility shim must preserve the historical signature"
)]
pub fn detect_style_base(config: &Config) -> Option<StyleBase> {
    detect_fixup_family(config)
}

/// Detects if a `ContributorConfig` matches a known preset.
///
/// Returns the matching preset if found, or `None` if the config is custom.
/// The detection is "fuzzy" - we check characteristic fields that define each
/// preset, not every single field.
#[must_use]
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
        && *dpl == citum_schema::options::DelimiterPrecedesLast::Contextual
    {
        return Some(ContributorPreset::Chicago);
    }

    None
}

/// Detects if a `TitlesConfig` matches a known preset.
///
/// Returns the matching preset if found, or `None` if the config is custom.
#[must_use]
pub fn detect_title_preset(config: &TitlesConfig) -> Option<TitlePreset> {
    let component_quoted = config
        .component
        .as_ref()
        .and_then(|c| c.quote)
        .unwrap_or(false);
    let component_case = config.component.as_ref().and_then(|c| c.text_case);
    let monograph_emph = config
        .monograph
        .as_ref()
        .and_then(|m| m.emph)
        .unwrap_or(false);
    let monograph_case = config.monograph.as_ref().and_then(|m| m.text_case);
    let periodical_emph = config
        .periodical
        .as_ref()
        .and_then(|p| p.emph)
        .unwrap_or(false);
    let periodical_case = config.periodical.as_ref().and_then(|p| p.text_case);
    let serial_emph = config.serial.as_ref().and_then(|s| s.emph).unwrap_or(false);

    // Scientific: sentence-case titles without default emphasis.
    if !component_quoted
        && matches!(
            component_case,
            Some(
                citum_schema::options::TextCase::Sentence
                    | citum_schema::options::TextCase::SentenceNlm
            )
        )
        && matches!(
            monograph_case,
            Some(
                citum_schema::options::TextCase::Sentence
                    | citum_schema::options::TextCase::SentenceNlm
            )
        )
        && !monograph_emph
        && !periodical_emph
        && periodical_case.is_none()
    {
        return Some(TitlePreset::Scientific);
    }

    // APA-family: sentence-case article/book titles, italic monographs/journals.
    if !component_quoted
        && matches!(
            component_case,
            Some(
                citum_schema::options::TextCase::Sentence
                    | citum_schema::options::TextCase::SentenceApa
            )
        )
        && matches!(
            monograph_case,
            Some(
                citum_schema::options::TextCase::Sentence
                    | citum_schema::options::TextCase::SentenceApa
            )
        )
        && monograph_emph
        && periodical_emph
        && periodical_case.is_none()
    {
        return Some(TitlePreset::Apa);
    }

    // Chicago/IEEE: component quoted, monograph/periodical italic
    if component_quoted && monograph_emph && periodical_emph {
        // Both Chicago and IEEE follow this pattern - default to Chicago
        return Some(TitlePreset::Chicago);
    }

    if !component_quoted && !monograph_emph && periodical_emph && serial_emph {
        return Some(TitlePreset::JournalEmphasis);
    }

    if !component_quoted && monograph_emph && periodical_emph && serial_emph {
        return Some(TitlePreset::Humanities);
    }

    None
}

/// Detects if a `DateConfig` matches a known preset.
///
/// Returns the matching preset if found, or `None` if the config is custom.
#[must_use]
pub fn detect_date_preset(config: &DateConfig) -> Option<DatePreset> {
    use citum_schema::options::MonthFormat;

    match config.month {
        MonthFormat::Numeric => Some(DatePreset::Numeric),
        MonthFormat::Short => Some(DatePreset::Short),
        MonthFormat::Long => Some(DatePreset::Long),
    }
}

/// Detect whether a substitute config exactly matches a named preset.
///
/// Unlike contributor/date/title detection, this is intentionally strict:
/// substitute fallback chains can carry type overrides and role substitution
/// policy, so only a byte-for-byte resolved preset match is folded.
#[must_use]
pub fn detect_substitute_preset(config: &Substitute) -> Option<SubstitutePreset> {
    [
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
    ]
    .into_iter()
    .find(|preset| preset.config() == *config)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in tests."
)]
mod tests {
    use super::*;
    use citum_schema::options::{
        DelimiterPrecedesLast, ShortenListOptions, SubstituteCandidates, SubstituteKey,
        TitleRendering,
    };
    use std::collections::HashMap;

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
            initialize_with: Some(String::new()),
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
            initialize_with: Some(String::new()),
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
            delimiter: Some(", ".into()),
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
        // APA: sentence-case article/book titles, italic monographs/journals.
        let config = TitlesConfig {
            component: Some(TitleRendering {
                text_case: Some(citum_schema::options::TextCase::Sentence),
                ..Default::default()
            }),
            monograph: Some(TitleRendering {
                text_case: Some(citum_schema::options::TextCase::Sentence),
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
        // Scientific: sentence-case titles with plain monographs/journals.
        let config = TitlesConfig {
            component: Some(TitleRendering {
                text_case: Some(citum_schema::options::TextCase::Sentence),
                ..Default::default()
            }),
            monograph: Some(TitleRendering {
                text_case: Some(citum_schema::options::TextCase::Sentence),
                ..Default::default()
            }),
            periodical: Some(TitleRendering::default()),
            ..Default::default()
        };
        assert_eq!(detect_title_preset(&config), Some(TitlePreset::Scientific));
    }

    #[test]
    fn detects_exact_standard_substitute_preset() {
        let config = SubstitutePreset::Standard.config();

        assert_eq!(
            detect_substitute_preset(&config),
            Some(SubstitutePreset::Standard)
        );
    }

    #[test]
    fn does_not_fold_substitute_with_role_substitute_chain() {
        let mut config = SubstitutePreset::Standard.config();
        config.role_substitute =
            HashMap::from([("container-author".to_string(), vec!["editor".to_string()])]);

        assert_eq!(detect_substitute_preset(&config), None);
    }

    #[test]
    fn does_not_fold_non_preset_substitute_ordering() {
        let config = Substitute {
            candidates: Some(SubstituteCandidates::Candidates(vec![
                SubstituteKey::Editor,
                SubstituteKey::Translator,
            ])),
            ..Default::default()
        };

        assert_eq!(detect_substitute_preset(&config), None);
    }

    #[test]
    fn test_detect_humanities_title() {
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
            serial: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(detect_title_preset(&config), Some(TitlePreset::Humanities));
    }

    #[test]
    fn test_detect_journal_emphasis_title() {
        let config = TitlesConfig {
            component: Some(TitleRendering::default()),
            periodical: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            serial: Some(TitleRendering {
                emph: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            detect_title_preset(&config),
            Some(TitlePreset::JournalEmphasis)
        );
    }

    #[test]
    fn test_detect_numeric_date() {
        use citum_schema::options::MonthFormat;

        let config = DateConfig {
            month: MonthFormat::Numeric,
            ..Default::default()
        };
        assert_eq!(detect_date_preset(&config), Some(DatePreset::Numeric));
    }
}
