/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
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

use crate::grouping::{GroupSort, GroupSortKey, SortKey as GroupSortKey_};
use crate::options::multilingual::{
    MultilingualConfig, MultilingualMode, MultilingualSegment, MultilingualView, ScriptConfig,
    SegmentWrap,
};
use crate::options::{
    AndOptions, ContributorConfig, DateConfig, DelimiterPrecedesLast, DemoteNonDroppingParticle,
    DisplayAsSort, MonthFormat, NameForm, ShortenListOptions, Sort, SortKey, SortSpec, Substitute,
    SubstituteKey, TitleRendering, TitlesConfig,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Numeric all-authors: all family-first, no conjunction, compact initials,
    /// no list shortening, particle demotion disabled.
    /// Example: "Smith JD, Jones MK, Brown AB"
    NumericAllAuthors,
    /// Numeric given-first with period-only initials (no space), no conjunction,
    /// and comma delimiters.
    /// Example: "J.D. Smith, M.K. Jones, A.B. Brown"
    NumericGivenDot,
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
    /// All current variants, kept in sync manually since the enum is `#[non_exhaustive]`.
    ///
    /// Consumers that need to enumerate every named contributor preset (e.g. reverse-matching
    /// extracted configs in `citum-analyze`) should use this instead of hand-listing variants,
    /// so a forgotten update fails a compile-time-adjacent test rather than silently omitting
    /// a variant. See `tests` in this module for the coverage check.
    pub const ALL: &'static [ContributorPreset] = &[
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
        ContributorPreset::NumericAllAuthors,
        ContributorPreset::NumericGivenDot,
        ContributorPreset::AnnualReviews,
        ContributorPreset::MathPhys,
        ContributorPreset::SocSciFirst,
        ContributorPreset::PhysicsNumeric,
    ];

    /// Convert named-style presets to config.
    fn config_named_presets(&self) -> ContributorConfig {
        match self {
            ContributorPreset::Apa => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::Symbol),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(". ".to_string()),
                name_form: Some(NameForm::Initials),
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
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Contextual),
                ..Default::default()
            },
            ContributorPreset::Vancouver => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
                shorten: Some(ShortenListOptions {
                    min: 7,
                    use_first: 6,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::Ieee => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::None),
                and: Some(AndOptions::Text),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(". ".to_string()),
                name_form: Some(NameForm::Initials),
                ..Default::default()
            },
            ContributorPreset::Harvard => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::Text),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(".".to_string()),
                name_form: Some(NameForm::Initials),
                ..Default::default()
            },
            ContributorPreset::Springer => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
                sort_separator: Some(" ".to_string()),
                shorten: Some(ShortenListOptions {
                    min: 5,
                    use_first: 3,
                    ..Default::default()
                }),
                ..Default::default()
            },
            #[allow(clippy::unreachable, reason = "Subset of variants handled here")]
            _ => unreachable!(),
        }
    }

    /// Convert numeric-style presets to config.
    fn config_numeric_presets(&self) -> ContributorConfig {
        match self {
            ContributorPreset::NumericCompact => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
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
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
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
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
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
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                shorten: Some(ShortenListOptions {
                    min: 11,
                    use_first: 10,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ContributorPreset::NumericAllAuthors => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
                sort_separator: Some(" ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::Never),
                ..Default::default()
            },
            ContributorPreset::NumericGivenDot => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::None),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(".".to_string()),
                name_form: Some(NameForm::Initials),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
            #[allow(clippy::unreachable, reason = "Subset of variants handled here")]
            _ => unreachable!(),
        }
    }

    /// Convert specialty-style presets to config.
    fn config_specialty_presets(&self) -> ContributorConfig {
        match self {
            ContributorPreset::AnnualReviews => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::All),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Never),
                initialize_with: Some("".to_string()),
                name_form: Some(NameForm::Initials),
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
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(".".to_string()),
                name_form: Some(NameForm::Initials),
                sort_separator: Some(", ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
            ContributorPreset::SocSciFirst => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::First),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                delimiter_precedes_last: Some(DelimiterPrecedesLast::Always),
                initialize_with: Some(". ".to_string()),
                name_form: Some(NameForm::Initials),
                sort_separator: Some(", ".to_string()),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
            ContributorPreset::PhysicsNumeric => ContributorConfig {
                display_as_sort: Some(DisplayAsSort::None),
                and: Some(AndOptions::None),
                delimiter: Some(", ".into()),
                initialize_with: Some(". ".to_string()),
                name_form: Some(NameForm::Initials),
                demote_non_dropping_particle: Some(DemoteNonDroppingParticle::SortOnly),
                ..Default::default()
            },
            #[allow(clippy::unreachable, reason = "Subset of variants handled here")]
            _ => unreachable!(),
        }
    }

    /// Convert this preset to a concrete `ContributorConfig`.
    pub fn config(&self) -> ContributorConfig {
        match self {
            ContributorPreset::Apa
            | ContributorPreset::Chicago
            | ContributorPreset::Vancouver
            | ContributorPreset::Ieee
            | ContributorPreset::Harvard
            | ContributorPreset::Springer => self.config_named_presets(),
            ContributorPreset::NumericCompact
            | ContributorPreset::NumericMedium
            | ContributorPreset::NumericTight
            | ContributorPreset::NumericLarge
            | ContributorPreset::NumericAllAuthors
            | ContributorPreset::NumericGivenDot => self.config_numeric_presets(),
            ContributorPreset::AnnualReviews
            | ContributorPreset::MathPhys
            | ContributorPreset::SocSciFirst
            | ContributorPreset::PhysicsNumeric => self.config_specialty_presets(),
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
    /// All current variants. See [`ContributorPreset::ALL`] for why this exists.
    pub const ALL: &'static [DatePreset] = &[
        DatePreset::Long,
        DatePreset::Short,
        DatePreset::Numeric,
        DatePreset::Iso,
    ];

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
    /// All title categories italic, with no per-category overrides. Distinct from `humanities`
    /// (which leaves `component` plain): the corpus convention this covers italicizes articles
    /// too.
    /// Example: *Article title*. *Book Title*. *Journal Title*.
    EmphasisAll,
    /// All title categories in headline-style title case, with no per-category overrides.
    /// Example: Article Title. Book Title. Journal Title.
    TitleCase,
}

impl TitlePreset {
    /// All current variants. See [`ContributorPreset::ALL`] for why this exists.
    pub const ALL: &'static [TitlePreset] = &[
        TitlePreset::Apa,
        TitlePreset::Chicago,
        TitlePreset::Ieee,
        TitlePreset::Humanities,
        TitlePreset::JournalEmphasis,
        TitlePreset::Scientific,
        TitlePreset::EmphasisAll,
        TitlePreset::TitleCase,
    ];

    /// Convert this preset to a concrete `TitlesConfig`.
    pub fn config(&self) -> TitlesConfig {
        use crate::options::titles::TextCase;
        let emph_rendering = TitleRendering {
            emph: Some(true),
            ..Default::default()
        };
        match self {
            TitlePreset::Apa => TitlesConfig {
                component: Some(TitleRendering {
                    text_case: Some(TextCase::SentenceApa),
                    ..Default::default()
                }),
                monograph: Some(TitleRendering {
                    text_case: Some(TextCase::SentenceApa),
                    emph: Some(true),
                    ..Default::default()
                }),
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
                component: Some(TitleRendering {
                    text_case: Some(TextCase::SentenceNlm),
                    ..Default::default()
                }),
                monograph: Some(TitleRendering {
                    text_case: Some(TextCase::SentenceNlm),
                    ..Default::default()
                }),
                periodical: Some(TitleRendering::default()),
                ..Default::default()
            },
            TitlePreset::EmphasisAll => TitlesConfig {
                default: Some(emph_rendering),
                ..Default::default()
            },
            TitlePreset::TitleCase => TitlesConfig {
                default: Some(TitleRendering {
                    text_case: Some(TextCase::Title),
                    ..Default::default()
                }),
                ..Default::default()
            },
        }
    }
}

/// Sort order presets for bibliography entries.
///
/// Each preset encodes the sort key sequence for a citation style family.
/// Use for the `bibliography.sort` field to avoid repeating boilerplate key lists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SortPreset {
    /// Author → year → title. Standard for author-date styles (APA, Chicago, Harvard).
    AuthorDateTitle,
    /// Author → title → year. Used in some footnote and note styles.
    AuthorTitleDate,
    /// Citation number only. Used in numeric styles (Vancouver, IEEE).
    CitationNumber,
}

impl SortPreset {
    /// Convert this preset to a concrete `Sort`.
    pub fn sort(&self) -> Sort {
        match self {
            SortPreset::AuthorDateTitle => Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![
                    SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Year,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Title,
                        ascending: true,
                    },
                ],
            },
            SortPreset::AuthorTitleDate => Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![
                    SortSpec {
                        key: SortKey::Author,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Title,
                        ascending: true,
                    },
                    SortSpec {
                        key: SortKey::Year,
                        ascending: true,
                    },
                ],
            },
            SortPreset::CitationNumber => Sort {
                shorten_names: false,
                render_substitutions: false,
                template: vec![SortSpec {
                    key: SortKey::CitationNumber,
                    ascending: true,
                }],
            },
        }
    }

    /// Convert this preset to a `GroupSort` for use in citation sorting.
    pub fn group_sort(&self) -> GroupSort {
        let keys: Vec<GroupSortKey> = match self {
            SortPreset::AuthorDateTitle => vec![
                GroupSortKey {
                    key: GroupSortKey_::Author,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: GroupSortKey_::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: GroupSortKey_::Title,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
            SortPreset::AuthorTitleDate => vec![
                GroupSortKey {
                    key: GroupSortKey_::Author,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: GroupSortKey_::Title,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
                GroupSortKey {
                    key: GroupSortKey_::Issued,
                    ascending: true,
                    order: None,
                    sort_order: None,
                },
            ],
            SortPreset::CitationNumber => vec![],
        };
        GroupSort { template: keys }
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
        let editor = SubstituteKey::Editor;
        let title = SubstituteKey::Title;
        let translator = SubstituteKey::Translator;

        // Build a substitute from an optional role-label form and a template
        // order; all presets share empty overrides/role-substitute/unknowns.
        let build = |form: Option<&str>, template: Vec<SubstituteKey>| Substitute {
            contributor_role_form: form.map(str::to_string),
            template,
            ..Substitute::default()
        };

        match self {
            SubstitutePreset::Standard => build(None, vec![editor, title, translator]),
            SubstitutePreset::EditorFirst => build(None, vec![editor, translator, title]),
            SubstitutePreset::TitleFirst => build(None, vec![title, editor, translator]),
            SubstitutePreset::EditorShort => build(Some("short"), vec![editor]),
            SubstitutePreset::EditorLong => build(Some("long"), vec![editor]),
            SubstitutePreset::EditorTranslatorShort => {
                build(Some("short"), vec![editor, translator])
            }
            SubstitutePreset::EditorTranslatorLong => build(Some("long"), vec![editor, translator]),
            SubstitutePreset::EditorTitleShort => build(Some("short"), vec![editor, title]),
            SubstitutePreset::EditorTitleLong => build(Some("long"), vec![editor, title]),
            SubstitutePreset::EditorTranslatorTitleShort => {
                build(Some("short"), vec![editor, translator, title])
            }
            SubstitutePreset::EditorTranslatorTitleLong => {
                build(Some("long"), vec![editor, translator, title])
            }
        }
    }
}

/// Multilingual rendering policy presets.
///
/// Each preset encodes the romanization and translation conventions for a major
/// citation style or style family when rendering references in an English-language
/// context.  Style YAML authors can write a bare preset name:
///
/// ```yaml
/// options:
///   multilingual: romanized-translated
/// ```
///
/// and Citum resolves it to the full [`MultilingualConfig`] at load time.
///
/// Preset names describe the **rendering behavior**, not a specific style family.
/// For CJK-inclusive 3-way views (e.g. `romanized original [translated]`) use an
/// explicit `pattern:` block instead — that display is something these styles
/// *allow*, not something they mandate by default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum MultilingualPreset {
    /// Romanize titles and append an English translation in brackets.
    ///
    /// Title pattern: `romanized [translated]` (= `Combined` mode).
    /// Names: romanized.  Script: `Latn`.
    ///
    /// Appropriate for APA, Chicago, MLA, Harvard, Vancouver, AMA, NLM, CSE,
    /// and most other English-language styles.  All of these styles require
    /// romanized names and romanized titles; translation is recommended or
    /// required for non-English titles; original-script display is a house
    /// option that styles *allow* but do not mandate.
    RomanizedTranslated,
    /// Romanize titles and names only — no translation appended.
    ///
    /// Title pattern: romanized form only (= `Transliterated` mode).
    /// Names: romanized.  Script: `Latn`.
    ///
    /// Appropriate for IEEE and numeric styles where the translation bracket
    /// is typically omitted.
    RomanizedOnly,
    /// Three-part view: romanized form, original script, bracketed translation.
    ///
    /// Title pattern: `romanized original-script [translated]`.
    /// Names: `romanized original-script` (no bracket).
    /// Script: `Latn`.  Han and Hangul names rendered family-first with no inter-part space.
    ///
    /// This is the rendering pattern used by some area-studies and East Asian studies
    /// house styles (e.g. CJKR, JAAS).  It is **not** a mandated disciplinary standard —
    /// it is a display convention that some publishers allow or require.
    ///
    /// When CJK names render in original script the `use-native-ordering` flag ensures
    /// family-first ordering.  For Latin-script (romanized) names, ordering follows the
    /// normal template `name-order`.
    RomanizedScriptTranslated,
}

impl MultilingualPreset {
    /// Resolve this preset into a concrete [`MultilingualConfig`].
    pub fn config(self) -> MultilingualConfig {
        match self {
            MultilingualPreset::RomanizedTranslated => MultilingualConfig {
                title_mode: Some(MultilingualMode::Combined),
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            },
            MultilingualPreset::RomanizedOnly => MultilingualConfig {
                title_mode: Some(MultilingualMode::Transliterated),
                name_mode: Some(MultilingualMode::Transliterated),
                preferred_script: Some("Latn".to_string()),
                ..Default::default()
            },
            MultilingualPreset::RomanizedScriptTranslated => {
                let mut scripts = HashMap::new();
                scripts.insert(
                    "Han".to_string(),
                    ScriptConfig {
                        use_native_ordering: true,
                        ..Default::default()
                    },
                );
                scripts.insert(
                    "Hangul".to_string(),
                    ScriptConfig {
                        use_native_ordering: true,
                        ..Default::default()
                    },
                );
                MultilingualConfig {
                    title_mode: Some(MultilingualMode::Pattern(vec![
                        MultilingualSegment {
                            view: MultilingualView::Transliterated,
                            wrap: SegmentWrap::None,
                        },
                        MultilingualSegment {
                            view: MultilingualView::OriginalScript,
                            wrap: SegmentWrap::None,
                        },
                        MultilingualSegment {
                            view: MultilingualView::Translated,
                            wrap: SegmentWrap::Brackets,
                        },
                    ])),
                    name_mode: Some(MultilingualMode::Pattern(vec![
                        MultilingualSegment {
                            view: MultilingualView::Transliterated,
                            wrap: SegmentWrap::None,
                        },
                        MultilingualSegment {
                            view: MultilingualView::OriginalScript,
                            wrap: SegmentWrap::None,
                        },
                    ])),
                    preferred_script: Some("Latn".to_string()),
                    scripts,
                    ..Default::default()
                }
            }
        }
    }
}

/// Entry for `options.multilingual`: either a preset name or an explicit config block.
///
/// Style YAML can use a short preset name:
/// ```yaml
/// options:
///   multilingual: romanized-translated
/// ```
/// or a full explicit block:
/// ```yaml
/// options:
///   multilingual:
///     title-mode: combined
///     name-mode: transliterated
///     preferred-script: Latn
/// ```
#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[serde(untagged)]
pub enum MultilingualConfigEntry {
    /// A named preset (`"romanized-translated"` or `"romanized-only"`).
    Preset(MultilingualPreset),
    /// An explicit multilingual configuration block.
    Explicit(Box<MultilingualConfig>),
}

impl MultilingualConfigEntry {
    /// Resolve this entry into a concrete [`MultilingualConfig`].
    pub fn resolve(self) -> MultilingualConfig {
        match self {
            MultilingualConfigEntry::Preset(p) => p.config(),
            MultilingualConfigEntry::Explicit(c) => *c,
        }
    }
}

/// Custom deserializer for [`MultilingualConfigEntry`].
///
/// Accepts either a bare preset name (`"romanized-translated"`, `"romanized-only"`) or a full
/// `MultilingualConfig` map block. A hand-written visitor is used instead of
/// `#[serde(untagged)]` because the untagged mechanism wraps inner
/// deserializers in a content-based buffer that converts externally-tagged
/// YAML maps (like `{pattern: [...]}`) into serde enum inputs — and those
/// inputs cannot be re-dispatched through `deserialize_any`, causing a
/// *"untagged and internally tagged enums do not support enum input"* error
/// on serialization roundtrips.
impl<'de> serde::Deserialize<'de> for MultilingualConfigEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct EntryVisitor;

        impl<'de> serde::de::Visitor<'de> for EntryVisitor {
            type Value = MultilingualConfigEntry;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a multilingual preset name or an explicit config block")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                let preset =
                    MultilingualPreset::deserialize(serde::de::value::StrDeserializer::new(v))?;
                Ok(MultilingualConfigEntry::Preset(preset))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                let config = MultilingualConfig::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;
                Ok(MultilingualConfigEntry::Explicit(Box::new(config)))
            }
        }

        deserializer.deserialize_any(EntryVisitor)
    }
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
    fn test_contributor_preset_numeric_all_authors() {
        let config = ContributorPreset::NumericAllAuthors.config();
        assert_eq!(config.and, Some(AndOptions::None));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::All));
        assert_eq!(config.sort_separator, Some(" ".to_string()));
        assert_eq!(config.initialize_with, Some("".to_string()));
        assert_eq!(
            config.demote_non_dropping_particle,
            Some(DemoteNonDroppingParticle::Never)
        );
        assert!(config.shorten.is_none());
    }

    #[test]
    fn test_contributor_preset_numeric_given_dot() {
        let config = ContributorPreset::NumericGivenDot.config();
        assert_eq!(config.and, Some(AndOptions::None));
        assert_eq!(config.display_as_sort, Some(DisplayAsSort::None));
        assert_eq!(config.initialize_with, Some(".".to_string()));
        assert_eq!(
            config.demote_non_dropping_particle,
            Some(DemoteNonDroppingParticle::SortOnly)
        );
        assert_eq!(
            config.delimiter_precedes_last,
            Some(DelimiterPrecedesLast::Always)
        );
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
    fn test_title_preset_emphasis_all() {
        let config = TitlePreset::EmphasisAll.config();
        let default = config.default.unwrap();
        assert_eq!(default.emph, Some(true));
        assert!(config.component.is_none());
        assert!(config.monograph.is_none());
        assert!(config.periodical.is_none());
        assert!(config.serial.is_none());
    }

    #[test]
    fn test_title_preset_title_case() {
        let config = TitlePreset::TitleCase.config();
        let default = config.default.unwrap();
        assert_eq!(
            default.text_case,
            Some(crate::options::titles::TextCase::Title)
        );
        assert!(default.emph.is_none() || default.emph == Some(false));
    }

    #[test]
    fn test_all_preset_const_variants_resolve_to_config() {
        // `.config()` for ContributorPreset dispatches through per-family helpers with
        // `unreachable!()` arms; a variant present in `ALL` but missing from a dispatch match
        // must panic here rather than at analyzer runtime.
        for preset in ContributorPreset::ALL {
            let _ = preset.config();
        }
        for preset in DatePreset::ALL {
            let _ = preset.config();
        }
        for preset in TitlePreset::ALL {
            let _ = preset.config();
        }
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
            ContributorPreset::NumericAllAuthors,
            ContributorPreset::NumericGivenDot,
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
            TitlePreset::EmphasisAll,
            TitlePreset::TitleCase,
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
